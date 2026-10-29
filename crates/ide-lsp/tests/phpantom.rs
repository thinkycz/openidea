//! Runs the released engine through OpenStorm's actual LSP client.
//! OPENSTORM_TEST_PHPANTOM_BIN can point at the executable in the packaged app.
use ide_core::event::ServerStatus;
use ide_core::{DocId, IdeEvent, LanguageId, Pos, Range};
use ide_language::LanguageRegistry;
use ide_lsp::{LspManager, types::GotoKind};
use ide_text::AppliedChange;
use parking_lot::Mutex;
use std::{path::PathBuf, sync::Arc, time::Duration};

fn pos(text: &str, needle: &str) -> Pos {
    let offset = text.find(needle).unwrap();
    let prefix = &text[..offset];
    Pos::new(
        prefix.bytes().filter(|b| *b == b'\n').count() as u32,
        prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
    )
}
fn replace(mgr: &LspManager, doc: DocId, version: u32, old: &str, new: &str) {
    let end = Pos::new(
        old.bytes().filter(|b| *b == b'\n').count() as u32,
        old.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
    );
    mgr.did_change(
        doc,
        version,
        &[AppliedChange { range: Range::new(Pos::new(0, 0), end), text: new.into() }],
        &|| new.into(),
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn composer_intelligence_builtins_unsaved_changes_and_restart() {
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(
        root.join("composer.json"),
        r#"{"require":{"php":"^8.2"},"autoload":{"psr-4":{"EvalApp\\":"src/"},"files":["helpers.php"]}}"#,
    )
    .unwrap();
    std::fs::write(root.join("helpers.php"), "<?php function evaluation_label(string $s): string { return $s; }\n")
        .unwrap();
    std::fs::write(root.join("src/Customer.php"), "<?php namespace EvalApp; class Customer {}\n").unwrap();
    let invoice = "<?php\nnamespace EvalApp;\nclass Invoice {\n    public function total(int $count, float $price): float { return $count * $price; }\n}\n";
    let main = "<?php\nuse EvalApp\\Invoice;\n$invoice = new Invoice();\n$invoice->total(2, 5.0);\nevaluation_label('test');\nnew \\DateTime();\n";
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let events: Arc<Mutex<Vec<IdeEvent>>> = Arc::default();
    let capture = events.clone();
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![php])),
        Arc::new(move |e| capture.lock().push(e)),
    );
    let candidate_root = root.clone();
    mgr.set_candidates_hook(Box::new(move |_, _| {
        let mut files = vec![candidate_root.join("main.php"), candidate_root.join("helpers.php")];
        files.extend(std::fs::read_dir(candidate_root.join("src")).unwrap().map(|e| e.unwrap().path()));
        ide_lsp::CandidateFiles { files, limit_hit: false }
    }));
    let open = |name: &str, text: &str| {
        let path = root.join(name);
        std::fs::write(&path, text).unwrap();
        let doc = DocId::next();
        mgr.did_open(doc, &path, &LanguageId::new("php"), 1, text);
        doc
    };
    let decl = open("src/Invoice.php", invoice);
    let doc = open("main.php", main);
    let mut ready = false;
    for _ in 0..120 {
        if mgr.servers().iter().any(|s| matches!(s.status, ServerStatus::Ready | ServerStatus::Busy)) {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert!(ready, "engine did not become ready");
    let hover = mgr.hover(doc, pos(main, "total(2"), Default::default()).await.unwrap().unwrap();
    assert!(hover.contents.value.contains("total"));
    let signature = mgr.signature_help(doc, pos(main, "5.0)")).await.unwrap().unwrap();
    assert!(signature.signatures[0].label.contains("count"));
    let defs = mgr.goto(doc, pos(main, "total(2"), GotoKind::Definition).await.unwrap();
    assert!(defs.iter().any(|d| d.path == root.join("src/Invoice.php")));
    let helper = mgr.goto(doc, pos(main, "evaluation_label"), GotoKind::Definition).await.unwrap();
    assert!(helper.iter().any(|d| d.path == root.join("helpers.php")));
    let refs = mgr.references(doc, pos(main, "total(2"), true, true, Default::default()).await.unwrap();
    assert!(refs.locations.len() >= 2);
    let renamed = mgr.rename(doc, pos(main, "total(2"), "subtotal".into()).await.unwrap();
    assert_eq!(renamed.files.len(), 2, "rename must include declaration and caller");
    let builtin = mgr.goto(doc, pos(main, "DateTime"), GotoKind::Definition).await.unwrap();
    let uri = builtin.iter().find_map(|d| d.uri.as_ref()).expect("builtin URI");
    let virtual_doc = mgr.virtual_document(uri).await.unwrap();
    assert!(virtual_doc.text.contains("class DateTime"));
    assert_eq!(virtual_doc.language_id.as_str(), "php");
    assert!(virtual_doc.description.as_ref().unwrap().contains("8.2"));
    let imported = main.replace("new \\DateTime();", "new Cust");
    replace(&mgr, doc, 2, main, &imported);
    let at_end = Pos::new(5, 8);
    let completions = mgr.completion(doc, at_end, None, Default::default()).await.unwrap();
    let customer = completions.items.iter().find(|i| i.label == "Customer").expect("PSR-4 class completion");
    let resolved = mgr.resolve_completion(doc, customer.clone()).await.unwrap();
    assert!(
        resolved.additional_edits.iter().any(|e| e.new_text.contains("use EvalApp\\Customer")),
        "class import edits: {resolved:?}"
    );
    let revised = invoice.replace(
        "    public function total",
        "    public function discount(): int { return 1; }\n    public function total",
    );
    replace(&mgr, decl, 2, invoice, &revised);
    let unsaved = main.replace("new \\DateTime();", "$invoice->disc");
    replace(&mgr, doc, 3, &imported, &unsaved);
    let list = mgr.completion(doc, Pos::new(5, 14), None, Default::default()).await.unwrap();
    assert!(list.items.iter().any(|i| i.label.starts_with("discount")), "unsaved member completion");
    assert_eq!(std::fs::read_to_string(root.join("src/Invoice.php")).unwrap(), invoice);
    assert!(
        mgr.rename(doc, pos(&unsaved, "total(2"), "discount".into()).await.is_err(),
        "rename collision with an unsaved method must be refused"
    );
    let full = unsaved.replace("$invoice->disc", "$invoice->discount();");
    replace(&mgr, doc, 4, &unsaved, &full);
    let unsaved_rename = mgr.rename(doc, pos(&full, "discount();"), "rebate".into()).await.unwrap();
    assert_eq!(unsaved_rename.files.len(), 2, "rename includes the unsaved declaration and caller");
    assert_eq!(std::fs::read_to_string(root.join("src/Invoice.php")).unwrap(), invoice);
    let broken = main.replace("$invoice->total(2, 5.0);", "$invoice->missingMethod();");
    replace(&mgr, doc, 5, &full, &broken);
    let mut diagnostics = false;
    for _ in 0..80 {
        diagnostics = events.lock().iter().any(|e| matches!(e, IdeEvent::Diagnostics { items, .. } if items.iter().any(|d| d.message.contains("missingMethod"))));
        if diagnostics {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(diagnostics, "missing method diagnostic");
    let server = mgr.servers()[0].id;
    mgr.restart(server).await.unwrap();
    let list = mgr.completion(doc, Pos::new(3, 10), None, Default::default()).await.unwrap();
    assert!(!list.items.is_empty(), "server restores open document state after restart");
    mgr.shutdown_all().await;
    assert!(mgr.servers().iter().all(|s| s.status == ServerStatus::Stopped));
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires OPENSTORM_TEST_LARAVEL_ROOT with installed Laravel dependencies"]
async fn laravel_model_and_eloquent_builder_completion() {
    let root = PathBuf::from(std::env::var_os("OPENSTORM_TEST_LARAVEL_ROOT").expect("Laravel fixture root"))
        .canonicalize()
        .unwrap();
    assert!(root.join("vendor/laravel/framework/src/Illuminate/Database/Eloquent/Model.php").is_file());
    let composer: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("composer.json")).unwrap()).unwrap();
    assert_eq!(composer["autoload"]["psr-4"].get("App\\").and_then(serde_json::Value::as_str), Some("app/"));
    let model = std::fs::read_to_string(root.join("app/Models/User.php")).unwrap();
    assert!(model.contains("namespace App\\Models;"), "fixture must use valid PHP namespace separators");
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let mgr =
        LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_: IdeEvent| {}));
    let mut text = "<?php\nuse App\\Models\\User;\n$user = new User();\n$user->dis".to_owned();
    let doc = DocId::next();
    mgr.did_open(doc, &root.join("openstorm-intelligence.php"), &LanguageId::new("php"), 1, &text);
    let cases = [
        ("$user->dis", "displayName"),
        ("User::query()->wh", "where"),
        ("User::query()->where('name', 'Ada')->first()->dis", "displayName"),
    ];
    for (index, (expression, expected)) in cases.iter().enumerate() {
        let next = format!("<?php\nuse App\\Models\\User;\n$user = new User();\n{expression}");
        replace(&mgr, doc, index as u32 + 2, &text, &next);
        text = next;
        let mut found = false;
        let mut labels = Vec::new();
        for _ in 0..80 {
            if let Ok(list) = mgr.completion(doc, Pos::new(3, expression.len() as u32), None, Default::default()).await
            {
                labels = list.items.iter().map(|i| i.label.clone()).collect();
                found = labels.iter().any(|label| label.starts_with(expected));
                if found {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        assert!(found, "{expression}: expected {expected}; got {labels:?}");
    }
    mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn checked_import_optimization_removes_unused_imports_and_keeps_used_alias() {
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    std::fs::write(root.join("composer.json"), r#"{"require":{"php":"^8.2"}}"#).unwrap();
    let source =
        "<?php\nuse DateTimeImmutable as Clock;\nuse RuntimeException as UnusedException;\n$clock = new Clock();\n";
    let path = root.join("main.php");
    std::fs::write(&path, source).unwrap();
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let mgr = LspManager::new(root, Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let doc = DocId::next();
    mgr.did_open(doc, &path, &LanguageId::new("php"), 1, source);
    let actions = mgr
        .code_actions(doc, Range::new(Pos::new(0, 0), Pos::new(4, 0)), vec!["source.organizeImports".into()])
        .await
        .unwrap();
    let edit =
        actions.iter().find(|a| a.kind.as_deref() == Some("source.organizeImports")).unwrap().edit.as_ref().unwrap();
    assert!(!edit.files.is_empty(), "unused import must produce an edit");
    let mut result = ide_text::Document::new(source);
    for file in &edit.files {
        let ide_core::FileEdit::Edit { path: target, edits } = file else { panic!("text edit required") };
        assert_eq!(&**target, path);
        result.apply_edits(edits).unwrap();
    }
    assert!(!result.text().contains("UnusedException"));
    assert!(result.text().contains("DateTimeImmutable as Clock"));
    assert!(result.text().contains("new Clock()"));
    assert_eq!(std::fs::read_to_string(path).unwrap(), source, "query must not write files");
    mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_static_routes_and_views_through_typed_client() {
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    std::fs::create_dir_all(root.join("routes")).unwrap();
    std::fs::create_dir_all(root.join("resources/views/invoices")).unwrap();
    std::fs::write(
        root.join("composer.json"),
        r#"{"require":{"php":"^8.2","laravel/framework":"12.*"},"autoload":{"psr-4":{"App\\":"app/"}}}"#,
    )
    .unwrap();
    let routes = "<?php\nuse Illuminate\\Support\\Facades\\Route;\nRoute::get('/invoices/{invoice}', fn () => 'a')->name('invoice.show');\nRoute::get('/invoices/{invoice}/copy', fn () => 'b')->name('invoice.show');\nRoute::get('/health', fn () => 'ok')->name('health');\n";
    std::fs::write(root.join("routes/web.php"), routes).unwrap();
    std::fs::write(root.join("resources/views/invoices/show.blade.php"), "<h1>Invoice</h1>\n").unwrap();
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let mgr =
        LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_: IdeEvent| {}));
    let doc = DocId::next();
    mgr.did_open(doc, &root.join("routes/web.php"), &LanguageId::new("php"), 1, routes);
    let mut names = Vec::new();
    for _ in 0..80 {
        if let Ok(reply) = mgr.laravel_routes(doc, None, 2000, Default::default()).await {
            names = reply.routes.into_iter().map(|r| r.name).collect();
            if names.iter().any(|n| n == "invoice.show") {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert!(names.iter().any(|n| n == "invoice.show"), "got {names:?}");
    let bounded = mgr.laravel_routes(doc, None, 1, Default::default()).await.unwrap();
    assert!(bounded.incomplete);
    assert_eq!(bounded.routes.len(), 1);
    let targets = mgr.laravel_routes(doc, Some("invoice.show".into()), 2000, Default::default()).await.unwrap();
    assert_eq!(targets.locations.len(), 2);
    assert_eq!(targets.locations[0].range.start.line, 2);
    assert_eq!(targets.locations[1].range.start.line, 3);
    let views = mgr.laravel_views(doc, None, 2000, Default::default()).await.unwrap();
    assert!(views.views.iter().any(|v| v == "invoices.show"), "got {:?}", views.views);
    let target = mgr.laravel_views(doc, Some("invoices.show".into()), 2000, Default::default()).await.unwrap();
    assert_eq!(target.locations.len(), 1);
    assert!(target.locations[0].path.to_string_lossy().ends_with("resources/views/invoices/show.blade.php"));
    assert!(mgr.laravel_views(doc, Some("../secret".into()), 2000, Default::default()).await.is_err());
    mgr.shutdown_all().await;
}
