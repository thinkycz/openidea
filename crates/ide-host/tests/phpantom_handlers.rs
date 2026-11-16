//! Checked handler lookup through the actual host, document and remote RPC paths.
use ide_core::{DocId, IdeError, LanguageId, Pos, RequestId};
use ide_host::rpc::{self, Dispatcher, Invoke};
use ide_host::{Host, HostConfig, Project};
use ide_jsonrpc::{NoopHandler, Peer};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::Duration,
};
const SOURCE: &str =
    "<?php\nuse App\\Action; use Illuminate\\Support\\Facades\\Route;\nRoute::get('/😀', Action::class);\n";
const BASE: &str =
    "<?php\nnamespace App;\nclass BaseAction { public function __invoke(): string { return 'base'; } }\n";
const ACTION: &str = "<?php namespace App; class Action extends BaseAction {}\n";
const OTHER: &str = "<?php namespace App; class Other { public function __invoke(): string { return 'other'; } }\n";
fn at(text: &str, needle: &str) -> Pos {
    let prefix = &text[..text.find(needle).unwrap()];
    Pos::new(
        prefix.bytes().filter(|b| *b == b'\n').count() as u32,
        prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
    )
}
fn engine(legacy: bool) -> PathBuf {
    if legacy {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/engines/0.11.1-openstorm.153/phpantom_lsp")
    } else {
        std::env::var_os("OPENSTORM_TEST_HANDLER_BIN").map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/engines/0.11.1-openstorm.154/phpantom_lsp")
        })
    }
    .canonicalize()
    .unwrap()
}
struct Fixture {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    host: Arc<Host>,
    project: Arc<Project>,
    doc: DocId,
    dependency: DocId,
    ready: PathBuf,
    release: PathBuf,
}
impl Fixture {
    fn new(legacy: bool, hold: bool) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        for dir in ["routes", "src"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        let root = root.canonicalize().unwrap();
        let files = [
            ("composer.json", r#"{"require":{"laravel/framework":"^12.0"},"autoload":{"psr-4":{"App\\":"src/"}}}"#),
            ("routes/web.php", SOURCE),
            ("src/BaseAction.php", BASE),
            ("src/Action.php", ACTION),
            ("src/Other.php", OTHER),
        ];
        let cli = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/toolchains/php-8.4.23-static/php");
        for (name, text) in files {
            let path = root.join(name);
            std::fs::write(&path, text).unwrap();
            if name.ends_with(".php") {
                let lint = std::process::Command::new(&cli).args(["-n", "-l"]).arg(path).output().unwrap();
                assert!(lint.status.success(), "{}", String::from_utf8_lossy(&lint.stderr));
            } else {
                let manifest: Value = serde_json::from_str(text).unwrap();
                assert_eq!(manifest["autoload"]["psr-4"]["App\\"], "src/");
            }
        }
        let mut php = ide_language::LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
        php.language_servers.retain(|s| s.name == "phpantom");
        let cfg = &mut php.language_servers[0];
        let bin = engine(legacy);
        cfg.command = bin.to_string_lossy().into_owned();
        assert!(cfg.args.iter().any(|a| a == "--no-project-tools"));
        let ready = tmp.path().join("reply.json");
        let release = tmp.path().join("release");
        if hold {
            let proxy = tmp.path().join("held-reply.py");
            std::fs::write(&proxy, include_str!("../../ide-lsp/tests/fixtures/phpantom-held-reply.py")).unwrap();
            let args = std::mem::take(&mut cfg.args);
            cfg.command = std::env::var("OPENSTORM_TEST_PYTHON").unwrap_or_else(|_| "python3".into());
            cfg.args = vec![
                "-I".into(),
                proxy.to_string_lossy().into_owned(),
                bin.to_string_lossy().into_owned(),
                "phpWorkbench/routeHandler".into(),
                ready.to_string_lossy().into_owned(),
                release.to_string_lossy().into_owned(),
            ];
            cfg.args.extend(args);
        }
        let host = Arc::new(Host::new(HostConfig {
            project_environment: false,
            config_dir: tmp.path().join("config"),
            data_dir: tmp.path().join("data"),
            local_data_dir: tmp.path().join("local"),
            languages: Arc::new(ide_language::LanguageRegistry::from_configs(vec![php])),
            syntax: Arc::default(),
            sink_for: Box::new(|_| Arc::new(|_| {})),
            owner: |w| if w == "aux" { "main" } else { w },
            runtime: tokio::runtime::Handle::current(),
        }));
        host.trust.decide(&root, true, false).unwrap();
        let project = host.open_project("main", &root).unwrap();
        let doc = project.documents.open(&root.join("routes/web.php")).unwrap().id;
        let dependency = project.documents.open(&root.join("src/Action.php")).unwrap().id;
        Self { _tmp: tmp, root, host, project, doc, dependency, ready, release }
    }
    async fn peer(&self) -> Peer {
        let slot = Arc::new(OnceLock::new());
        let (client, server) = tokio::io::duplex(1 << 20);
        let (sr, sw) = tokio::io::split(server);
        tokio::spawn(rpc::serve(self.host.clone(), Dispatcher::standard(), slot, sr, sw));
        let (cr, cw) = tokio::io::split(client);
        let peer = Peer::connect(cr, cw, Arc::new(NoopHandler));
        let _: Value = peer.request("initialize", json!({"version":rpc::PROTOCOL_VERSION}), None).await.unwrap();
        peer
    }
    async fn call(&self, peer: &Peer, request: RequestId) -> Result<Value, IdeError> {
        peer.request(
            "invoke",
            Invoke {
                window: "aux".into(),
                cmd: "lsp_laravel_route_handler".into(),
                args: json!({"request":request,"doc":self.doc,"pos":at(SOURCE,"Action::class")}),
            },
            None,
        )
        .await
        .map_err(rpc::error_from_reply)
    }
    async fn close(&self, peer: &Peer) {
        let _ = std::fs::write(&self.release, "release");
        self.project.lsp.shutdown_all().await;
        self.host.close_project("main");
        peer.kill();
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires candidate154 and the retained153 negative-control executable"]
async fn checked_handler_rpc_uses_host_overlays_and_capabilities() {
    for legacy in [false, true] {
        let f = Fixture::new(legacy, false);
        let peer = f.peer().await;
        let result: Result<(), String> = async {
            let request = RequestId::next();
            let first = f.call(&peer, request).await;
            if legacy {
                let error = first.expect_err("legacy server must refuse this extension");
                return if error.kind == ide_core::error::ErrorKind::Unavailable && error.message.contains("support") {
                    Ok(())
                } else {
                    Err(format!("legacy capability: {error}"))
                };
            }
            let first = first.map_err(|e| e.to_string())?;
            if first["incomplete"] != false
                || first["locations"][0]["path"] != f.root.join("src/BaseAction.php").to_string_lossy().as_ref()
            {
                return Err(format!("checked RPC handler: {first}"));
            }
            if first["locations"][0]["range"]["start"] != json!(at(BASE, "__invoke")) {
                return Err(format!("wrong UTF16 target: {first}"));
            }
            f.project
                .documents
                .replace_all(f.dependency, &ACTION.replace("BaseAction", "Other"))
                .map_err(|e| e.to_string())?;
            let current = f.call(&peer, RequestId::next()).await.map_err(|e| e.to_string())?;
            if current["locations"][0]["path"] != f.root.join("src/Other.php").to_string_lossy().as_ref() {
                return Err(format!("host overlay was not synchronized: {current}"));
            }
            let done = RequestId::next();
            f.call(&peer, done).await.map_err(|e| e.to_string())?;
            let detached = f.host.begin_request(done);
            f.host.close_project_detached("main");
            if detached.is_cancelled() {
                return Err("completed handler request retained a project cancellation entry".into());
            }
            f.host.end_request(done);
            Ok(())
        }
        .await;
        f.close(&peer).await;
        assert!(result.is_ok(), "legacy={legacy}: {result:?}");
        assert_eq!(std::fs::read_to_string(f.root.join("src/Action.php")).unwrap(), ACTION);
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), SOURCE);
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires candidate154 and Python for actual held replies"]
async fn checked_handler_rpc_cancels_with_its_project_and_window() {
    for mutation in ["cancel", "close", "replace", "dependency", "unchanged"] {
        let f = Fixture::new(false, true);
        let peer = f.peer().await;
        let result: Result<(), String> = async {
            // Warm ordinary class navigation, leaving the handler reply as the one held frame.
            f.project
                .lsp
                .goto(f.doc, at(SOURCE, "Action::class"), ide_lsp::types::GotoKind::Definition)
                .await
                .map_err(|e| e.to_string())?;
            let request = RequestId::next();
            let pending = {
                let peer = peer.clone();
                let doc = f.doc;
                tokio::spawn(async move {
                    peer.request::<_, Value>(
                        "invoke",
                        Invoke {
                            window: "aux".into(),
                            cmd: "lsp_laravel_route_handler".into(),
                            args: json!({"request":request,"doc":doc,"pos":at(SOURCE,"Action::class")}),
                        },
                        None,
                    )
                    .await
                    .map_err(rpc::error_from_reply)
                })
            };
            for _ in 0..750 {
                if f.ready.exists() || pending.is_finished() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            let captured = std::fs::read_to_string(&f.ready);
            if captured.is_ok() {
                match mutation {
                    "cancel" => f.host.cancel_request(request),
                    "close" | "replace" => {
                        f.host.close_project_detached("main");
                        if mutation == "replace" {
                            let next = f._tmp.path().join("replacement");
                            std::fs::create_dir_all(&next).unwrap();
                            f.host.open_project("main", &next).map_err(|e| e.to_string())?;
                        }
                    }
                    "dependency" => {
                        f.project
                            .documents
                            .replace_all(f.dependency, &ACTION.replace("BaseAction", "Other"))
                            .map_err(|e| e.to_string())?;
                    }
                    _ => {}
                }
            }
            std::fs::write(&f.release, "release").map_err(|e| e.to_string())?;
            let reply = pending.await.map_err(|e| e.to_string())?;
            let captured: Value =
                serde_json::from_str(&captured.map_err(|e| format!("no held engine response: {e}; client={reply:?}"))?)
                    .map_err(|e| e.to_string())?;
            if captured["result"]["locations"].as_array().is_none_or(|v| v.len() != 1) {
                return Err(format!("no real handler in held response: {captured}"));
            }
            if mutation == "unchanged" {
                if reply.is_err() {
                    return Err(format!("current reply refused: {reply:?}"));
                }
            } else {
                let expected = if mutation == "dependency" {
                    ide_core::error::ErrorKind::OutOfSync
                } else {
                    ide_core::error::ErrorKind::Cancelled
                };
                if !matches!(reply,Err(ref e) if e.kind==expected) {
                    return Err(format!("stale host response: {reply:?}"));
                }
            }
            Ok(())
        }
        .await;
        f.close(&peer).await;
        assert!(result.is_ok(), "{mutation}: {result:?}");
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), SOURCE);
        assert_eq!(std::fs::read_to_string(f.root.join("src/Action.php")).unwrap(), ACTION);
    }
}
