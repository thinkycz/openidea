//! Runtime controller completion through the real host/client/document paths.
use futures::FutureExt;
use ide_core::{DocId, LanguageId, Pos, Range, RequestId, TextEdit};
use ide_host::{Host, HostConfig, Project};
use ide_lsp::types::{CompletionItem, CompletionKind, CompletionList, GotoKind};
use std::{io::Write, path::PathBuf, process::Stdio, sync::Arc, time::Duration};

const HEADER: &str = "<?php\nnamespace Harness;\nuse Illuminate\\Support\\Facades\\Route as Routes;\nuse App\\Other\\InvoiceController as InvoiceController;\n";
const MAIN: &str = "<?php\nnamespace App;\n/** Main controller completion witness. */\nclass InvoiceController {public function showInvoice(): bool {return true;}}\n";
const OTHER: &str =
    "<?php namespace App\\Other; class InvoiceController {public function showInvoice(): string {return 'other';}}\n";

fn at(text: &str, offset: usize) -> Pos {
    let prefix = &text[..offset];
    Pos::new(
        prefix.bytes().filter(|b| *b == b'\n').count() as u32,
        prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
    )
}

fn routes(call: &str) -> String {
    format!("{HEADER}{call}\n")
}

struct Fixture {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    host: Host,
    project: Arc<Project>,
    doc: DocId,
}

impl Fixture {
    fn new(initial: &str, extra: &[(String, String)]) -> Self {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("src-tauri/resources/phpantom/phpantom_lsp"));
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        std::fs::create_dir_all(&root).unwrap();
        let root = dunce::canonicalize(root).unwrap();
        let files = [
            (
                "composer.json".to_owned(),
                r#"{"require":{"php":"^8.3","laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#
                    .to_owned(),
            ),
            ("app/InvoiceController.php".to_owned(), MAIN.to_owned()),
            ("app/Other/InvoiceController.php".to_owned(), OTHER.to_owned()),
            ("routes/web.php".to_owned(), initial.to_owned()),
        ];
        for (name, text) in files.iter().chain(extra) {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
            if name.ends_with(".php") {
                Self::lint(text);
            }
        }
        let composer: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join("composer.json")).unwrap()).unwrap();
        assert_eq!(composer["autoload"]["psr-4"]["App\\"], "app/");
        let mut php = ide_language::LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
        php.language_servers.retain(|s| s.name == "phpantom");
        let host = Host::new(HostConfig {
            project_environment: false,
            config_dir: tmp.path().join("config"),
            data_dir: tmp.path().join("data"),
            local_data_dir: tmp.path().join("local-data"),
            languages: Arc::new(ide_language::LanguageRegistry::from_configs(vec![php])),
            syntax: Arc::default(),
            sink_for: Box::new(|_| Arc::new(|_| {})),
            owner: |w| w,
            runtime: tokio::runtime::Handle::current(),
        });
        host.trust.decide(&root, true, false).unwrap();
        let project = host.open_project("main", &root).unwrap();
        project.lsp.set_config_hook(Box::new(move |cfg, _| {
            if cfg.name == "phpantom" {
                cfg.command = bin.to_string_lossy().into_owned();
            }
        }));
        let doc = project.documents.open(&root.join("routes/web.php")).unwrap().id;
        Self { _tmp: tmp, root, host, project, doc }
    }

    fn lint(text: &str) {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let php = std::env::var_os("OPENSTORM_TEST_PHP_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("artifacts/toolchains/php-8.4.23-static/php"));
        let mut child = std::process::Command::new(php)
            .args(["-n", "-l"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "invalid PHP fixture: {}", String::from_utf8_lossy(&output.stderr));
    }

    async fn complete(&self, position: Pos) -> CompletionList {
        for _ in 0..80 {
            match ide_host::ops::lsp::lsp_completion(&self.host, "main", RequestId::next(), self.doc, position, None)
                .await
            {
                Ok(list) => return list,
                Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                Err(error) => panic!("host completion failed: {error}"),
            }
        }
        panic!("runtime class completion never settled");
    }

    async fn finish(&self, outcome: std::thread::Result<()>) {
        self.project.lsp.shutdown_all().await;
        assert!(self.project.lsp.servers().iter().all(|s| s.pid.is_none()));
        self.host.close_project("main");
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
    }
}

fn controller(list: CompletionList) -> CompletionItem {
    list.items
        .into_iter()
        .find(|i| i.detail.as_deref() == Some("App\\InvoiceController"))
        .expect("host must offer the runtime controller class")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and owned PHP syntax-check executable"]
async fn runtime_class_completion_applies_only_the_class_and_keeps_save_conflict_consent() {
    let original = routes("Routes::get('/original','App\\InvoiceController@showInvoice');");
    let f = Fixture::new(&original, &[]);
    let outcome = std::panic::AssertUnwindSafe(async {
        for (call, body, prefix, expected) in [
            (r#"Routes::get('/😀/direct','App\InvOLD@showInvoice');"#, r"App\InvOLD", r"App\In",
             r#"Routes::get('/😀/direct','\\App\\InvoiceController@showInvoice');"#),
            (r#"Routes::namespace('App')->group(fn () => Routes::get('/short','InvOLD@showInvoice'));"#, "InvOLD", "In",
             r#"Routes::namespace('App')->group(fn () => Routes::get('/short','\\App\\InvoiceController@showInvoice'));"#),
            (r#"Routes::get('/😀/hex',"App\\Inv\x4fLD@showInvoice");"#, r"App\\Inv\x4fLD", r"App\\Inv",
             r#"Routes::get('/😀/hex',"\\App\\InvoiceController@showInvoice");"#),
            (r#"Routes::get('/unicode',"\u{41}pp\\InvOLD@showInvoice");"#, r"\u{41}pp\\InvOLD", r"\u{41}pp\\In",
             r#"Routes::get('/unicode',"\\App\\InvoiceController@showInvoice");"#),
            (r#"Routes::namespace('App')->controller('InvOLD')->group(fn () => Routes::get('/bound','showInvoice'));"#, "InvOLD", "In",
             r#"Routes::namespace('App')->controller('\\App\\InvoiceController')->group(fn () => Routes::get('/bound','showInvoice'));"#),
            (r#"Routes::group(['namespace'=>'App','controller'=>'InvOLD'],fn () => Routes::get('/attrs','showInvoice'));"#, "InvOLD", "In",
             r#"Routes::group(['namespace'=>'App','controller'=>'\\App\\InvoiceController'],fn () => Routes::get('/attrs','showInvoice'));"#),
        ] {
            let input = routes(call);
            Fixture::lint(&input);
            let expected = routes(expected);
            Fixture::lint(&expected);
            f.project.documents.replace_all(f.doc, &input).unwrap();
            let before = f.project.documents.snapshot(f.doc).unwrap();
            let start = input.find(body).unwrap();
            let range = Range::new(at(&input, start), at(&input, start + body.len()));
            let item = controller(f.complete(at(&input, start + prefix.len())).await);
            assert_eq!(item.kind, Some(CompletionKind::Class));
            assert_eq!(item.replace_range, Some(range), "UTF16 class interior: {call}");
            assert!(!item.is_snippet);
            assert!(item.additional_edits.is_empty(), "runtime strings must never add PHP imports");
            assert!(item.filter_text.as_deref().is_some_and(|text| text.starts_with(prefix) && !text.contains("OLD")));
            let resolved = ide_host::ops::lsp::lsp_resolve_completion(&f.host, "main", f.doc, item).await.unwrap();
            assert_eq!(resolved.replace_range, Some(range));
            assert!(resolved.additional_edits.is_empty());
            assert!(resolved.documentation.as_ref().unwrap().value.contains("Main controller completion witness."));
            let edit = TextEdit { range, new_text: resolved.insert_text };
            assert!(f.project.documents.apply_edits_at(f.doc, before.version, &[edit]).unwrap().is_some());
            let current = f.project.documents.snapshot(f.doc).unwrap();
            assert_eq!(current.text, expected);
            assert!(current.dirty);
            assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), original);
            let targets = f.project.lsp.goto(f.doc, range.start, GotoKind::Definition).await.unwrap();
            assert_eq!(targets.len(), 1, "completed runtime class must navigate: {targets:?}");
            assert_eq!(targets[0].path.to_path_buf(), f.root.join("app/InvoiceController.php"));
            let hover = f.project.lsp.hover(f.doc, range.start, Default::default()).await.unwrap().unwrap();
            assert!(hover.contents.value.contains("Main controller completion witness."));
        }
        let current = f.project.documents.snapshot(f.doc).unwrap().text;
        let path = f.root.join("routes/web.php");
        let external = format!("{original}// External first\n");
        std::fs::write(&path, &external).unwrap();
        let consent = ide_host::ops::doc::save_checked(&f.host, "main", f.doc, None).await.unwrap().unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), external);
        let newer = format!("{original}// External second\n");
        std::fs::write(&path, &newer).unwrap();
        let replacement = ide_host::ops::doc::save_checked(&f.host, "main", f.doc, Some(consent)).await.unwrap().unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), newer, "old consent cannot overwrite newer external data");
        assert!(ide_host::ops::doc::save_checked(&f.host, "main", f.doc, Some(replacement)).await.unwrap().is_none());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), current);
        assert_eq!(std::fs::read_to_string(f.root.join("app/InvoiceController.php")).unwrap(), MAIN);
        assert_eq!(std::fs::read_to_string(f.root.join("app/Other/InvoiceController.php")).unwrap(), OTHER);
    }).catch_unwind().await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and owned PHP syntax-check executable"]
async fn runtime_class_completion_keeps_catalog_metadata_unsaved_overlays_and_restart() {
    let draft =
        "<?php namespace App; class InvoiceDraftController {public function showInvoice(): bool {return true;}}\n";
    let published = draft.replace("InvoiceDraftController", "InvoicePublishedController");
    let mut extra = vec![("app/InvoiceDraftController.php".into(), draft.to_owned())];
    for n in 0..104 {
        extra.push((format!("app/C{n:03}.php"), format!("<?php namespace App; class C{n:03} {{}}\n")));
    }
    let original = routes(
        "Routes::namespace('App')->controller('InvOLD')->group(fn () => Routes::get('/😀/main','showInvoice'));",
    );
    let f = Fixture::new(&original, &extra);
    let outcome = std::panic::AssertUnwindSafe(async {
        let cursor = at(&original, original.find("InvOLD").unwrap() + 2);
        controller(f.complete(cursor).await);
        let dep = f.project.documents.open(&f.root.join("app/InvoiceDraftController.php")).unwrap().id;
        for (text, present, absent) in [
            (draft, "App\\InvoiceDraftController", "App\\InvoicePublishedController"),
            (published.as_str(), "App\\InvoicePublishedController", "App\\InvoiceDraftController"),
        ] {
            f.project.documents.replace_all(dep, text).unwrap();
            let list = f.complete(cursor).await;
            assert!(list.items.iter().any(|i| i.detail.as_deref() == Some(present)), "unsaved candidate missing: {present}");
            assert!(!list.items.iter().any(|i| i.detail.as_deref() == Some(absent)), "retired candidate survived: {absent}");
        }
        let server = f.project.lsp.servers().into_iter().find(|s| s.name == "phpantom").unwrap().id;
        f.project.lsp.restart(server).await.unwrap();
        let list = f.complete(cursor).await;
        assert!(list.items.iter().any(|i| i.detail.as_deref() == Some("App\\InvoicePublishedController")));
        assert!(!list.items.iter().any(|i| i.detail.as_deref() == Some("App\\InvoiceDraftController")));
        assert_eq!(std::fs::read_to_string(f.root.join("app/InvoiceDraftController.php")).unwrap(), draft);
        f.project.documents.close(dep);
        let list = f.complete(cursor).await;
        assert!(list.items.iter().any(|i| i.detail.as_deref() == Some("App\\InvoiceDraftController")));
        assert!(!list.items.iter().any(|i| i.detail.as_deref() == Some("App\\InvoicePublishedController")));
        for call in [
            "Routes::namespace($selected)->controller('InvOLD')->group(fn () => Routes::get('/unknown','showInvoice'));",
            "Routes::namespace('App')->controller('InvOLD')->group(function () {Routes::get('/main','showInvoice');Routes::namespace('Other')->group(fn () => Routes::get('/other','showInvoice'));});",
        ] {
            let input = routes(call);
            Fixture::lint(&input);
            f.project.documents.replace_all(f.doc, &input).unwrap();
            let list = f.complete(at(&input, input.find("InvOLD").unwrap() + 2)).await;
            assert!(list.is_incomplete && list.items.is_empty(), "ambiguous context must remain incomplete: {list:?}");
        }
        let input = routes("Routes::namespace('App')->controller('COLD')->group(fn () => Routes::get('/limit','showInvoice'));");
        f.project.documents.replace_all(f.doc, &input).unwrap();
        let list = f.complete(at(&input, input.find("COLD").unwrap() + 1)).await;
        assert!(list.is_incomplete);
        assert_eq!(list.items.len(), 100);
        assert!(list.items.iter().all(|i| i.kind == Some(CompletionKind::Class) && i.additional_edits.is_empty()));
        f.project.documents.replace_all(f.doc, &original).unwrap();
        let before = f.project.documents.snapshot(f.doc).unwrap();
        let item = controller(f.complete(cursor).await);
        let changed = format!("{original}// New edit while completion is visible\n");
        f.project.documents.replace_all(f.doc, &changed).unwrap();
        assert!(f.project.documents.apply_edits_at(f.doc, before.version, &[TextEdit {
            range: item.replace_range.unwrap(), new_text: item.insert_text,
        }]).unwrap().is_none());
        assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, changed);
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), original);
    }).catch_unwind().await;
    f.finish(outcome).await;
}
