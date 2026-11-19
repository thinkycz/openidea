//! Owned server shutdown must finish queued protocol messages and close stdin.
#![cfg(unix)]

use ide_core::{DocId, LanguageId, Pos};
use ide_jsonrpc::{Handler, Peer};
use ide_language::{LanguageRegistry, LanguageServerConfig};
use ide_lsp::{LaunchError, Launcher, LspManager, ProcessLauncher, types::GotoKind};
use parking_lot::Mutex;
use std::{path::Path, sync::Arc, time::Duration};

#[derive(Default)]
struct NativeLauncher(Mutex<Option<Peer>>);

impl Launcher for NativeLauncher {
    fn launch(
        &self,
        config: &LanguageServerConfig,
        root: &Path,
        handler: Arc<dyn Handler>,
        stderr: Box<dyn Fn(String) + Send + Sync>,
    ) -> Result<Peer, LaunchError> {
        let peer = ProcessLauncher.launch(config, root, handler, stderr)?;
        *self.0.lock() = Some(peer.clone());
        Ok(peer)
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn owned_shutdown_delivers_exit_and_eof_without_killing_the_server() {
    assert_eq!(shutdown_probe("cooperative").await, ("exit and eof received".into(), Some(0)));
}

#[tokio::test(flavor = "multi_thread")]
async fn owned_shutdown_still_reaps_a_server_that_ignores_eof() {
    assert_eq!(shutdown_probe("ignore-eof").await, ("exit and eof received".into(), None));
}

#[tokio::test(flavor = "multi_thread")]
async fn owned_shutdown_still_reaps_a_server_that_never_answers_shutdown() {
    assert_eq!(shutdown_probe("ignore-shutdown").await, ("ready".into(), None));
}

async fn shutdown_probe(mode: &str) -> (String, Option<i32>) {
    let dir = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(dir.path()).unwrap();
    let script = root.join("server.py");
    let witness = root.join("shutdown.txt");
    std::fs::write(
        &script,
        r#"import json, pathlib, sys, time

pathlib.Path(sys.argv[1]).write_text('ready')

while True:
    headers = {}
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            sys.exit(4)
        if line in (b'\r\n', b'\n'):
            break
        key, value = line.decode().split(':', 1)
        headers[key.lower()] = value.strip()
    message = json.loads(sys.stdin.buffer.read(int(headers['content-length'])))
    method = message.get('method')
    if method == 'exit':
        pathlib.Path(sys.argv[1]).write_text('exit received')
        if sys.stdin.buffer.read() != b'':
            sys.exit(5)
        pathlib.Path(sys.argv[1]).write_text('exit and eof received')
        if sys.argv[2] == 'ignore-eof':
            while True:
                time.sleep(1)
        sys.exit(0)
    if 'id' not in message:
        continue
    if method == 'initialize':
        result = {'capabilities': {'textDocumentSync': 1, 'definitionProvider': True}}
    elif method == 'shutdown':
        if sys.argv[2] == 'ignore-shutdown':
            while True:
                time.sleep(1)
        result = None
    else:
        result = []
    body = json.dumps({'jsonrpc': '2.0', 'id': message['id'], 'result': result}).encode()
    sys.stdout.buffer.write(('Content-Length: %d\r\n\r\n' % len(body)).encode() + body)
    sys.stdout.buffer.flush()
"#,
    )
    .unwrap();
    let mut language = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("python")).unwrap();
    language.id = LanguageId::new("eof-probe");
    language.extensions = vec!["eof-probe".into()];
    language.language_servers.truncate(1);
    let server = &mut language.language_servers[0];
    server.name = "eof-probe".into();
    server.command = which::which("python3").expect("Python 3 for the owned server fixture").to_string_lossy().into();
    server.args = vec![script.to_string_lossy().into(), witness.to_string_lossy().into(), mode.into()];
    server.requires_env.clear();
    server.root_markers.clear();
    server.require_root_marker = false;
    server.install_hint = None;
    let launcher = Arc::new(NativeLauncher::default());
    let manager = LspManager::with_launcher(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![language])),
        Arc::new(|_| {}),
        launcher.clone(),
    );
    let path = root.join("document.eof-probe");
    std::fs::write(&path, "source\n").unwrap();
    let doc = DocId::next();
    manager.did_open(doc, &path, &LanguageId::new("eof-probe"), 1, "source\n");
    let definition = manager.goto(doc, Pos::new(0, 0), GotoKind::Definition).await;
    manager.shutdown_all().await;
    assert!(definition.unwrap().is_empty());
    let peer = launcher.0.lock().clone().unwrap();
    let code = tokio::time::timeout(Duration::from_secs(1), peer.closed()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), peer.exited()).await.unwrap();
    (std::fs::read_to_string(witness).unwrap(), code)
}
