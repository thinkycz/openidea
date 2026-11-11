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

async fn settled_completion(mgr: &LspManager, doc: DocId, at: Pos) -> ide_lsp::types::CompletionList {
    for _ in 0..80 {
        match mgr.completion(doc, at, None, Default::default()).await {
            Ok(list) => return list,
            Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(error) => panic!("completion failed: {error}"),
        }
    }
    panic!("view-data semantic snapshot did not settle");
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
    let renamed = mgr.rename(doc, pos(main, "total(2"), "subtotal".into(), Default::default()).await.unwrap();
    assert_eq!(renamed.edit.files.len(), 2, "rename must include declaration and caller");
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
        mgr.rename(doc, pos(&unsaved, "total(2"), "discount".into(), Default::default()).await.is_err(),
        "rename collision with an unsaved method must be refused"
    );
    let full = unsaved.replace("$invoice->disc", "$invoice->discount();");
    replace(&mgr, doc, 4, &unsaved, &full);
    let unsaved_rename = mgr.rename(doc, pos(&full, "discount();"), "rebate".into(), Default::default()).await.unwrap();
    assert_eq!(unsaved_rename.edit.files.len(), 2, "rename includes the unsaved declaration and caller");
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
#[ignore = "requires OPENSTORM_TEST_LARAVEL_ROOT with Laravel and App\\Models\\User::displayName fixture"]
async fn laravel_model_and_eloquent_builder_completion() {
    let root = PathBuf::from(std::env::var_os("OPENSTORM_TEST_LARAVEL_ROOT").expect("Laravel fixture root"))
        .canonicalize()
        .unwrap();
    assert!(root.join("vendor/laravel/framework/src/Illuminate/Database/Eloquent/Model.php").is_file());
    let composer: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("composer.json")).unwrap()).unwrap();
    assert_eq!(composer["autoload"]["psr-4"].get("App\\").and_then(serde_json::Value::as_str), Some("app/"));
    let model = std::fs::read_to_string(root.join("app/Models/User.php"))
        .expect("OpenStorm User/displayName acceptance fixture");
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

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_route_parameters_controller_targets_and_unsaved_registrations() {
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    for dir in ["routes", "app"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(
        root.join("composer.json"),
        r#"{"require":{"php":"^8.2","laravel/framework":"13.*"},"autoload":{"psr-4":{"App\\":"app/"}}}"#,
    )
    .unwrap();
    let routes = "<?php\nuse Illuminate\\Support\\Facades\\Route as Routes;\nuse App\\InvoiceController;\nRoutes::name('billing.')->controller(InvoiceController::class)->group(function () {\n    Routes::get('/😀/invoices/{invoice}/{tab?}', 'show')->name('show');\n});\nRoutes::apiResource('receipts', InvoiceController::class)->parameters(['receipts' => 'record']);\n";
    let controller =
        "<?php\nnamespace App;\nclass InvoiceController { public function show(): string { return 'invoice'; } }\n";
    let caller = "<?php\n// 😀 žluťoučký\nroute('billing.show', ['in']);\nroute(name: 'billing.show', parameters: ['ta']);\nroute('receipts.show', ['re']);\n";
    for (name, text) in
        [("routes/web.php", routes), ("app/InvoiceController.php", controller), ("app/Caller.php", caller)]
    {
        std::fs::write(root.join(name), text).unwrap();
    }
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let mgr =
        LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_: IdeEvent| {}));
    let route_doc = DocId::next();
    let caller_doc = DocId::next();
    let controller_doc = DocId::next();
    for (doc, name, text) in [
        (route_doc, "routes/web.php", routes),
        (caller_doc, "app/Caller.php", caller),
        (controller_doc, "app/InvoiceController.php", controller),
    ] {
        mgr.did_open(doc, &root.join(name), &LanguageId::new("php"), 1, text);
    }
    let mut indexed = false;
    for _ in 0..80 {
        if let Ok(reply) = mgr.laravel_routes(route_doc, None, 2000, Default::default()).await {
            indexed = reply
                .routes
                .iter()
                .any(|r| r.name == "billing.show" && r.parameters.as_ref().is_some_and(|p| p == &["invoice", "tab"]));
            if indexed {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(indexed, "grouped optional URI parameters must survive the typed adapter");
    for (needle, label) in [("in']);", "invoice"), ("ta']);", "tab"), ("re']);", "record")] {
        let mut at = pos(caller, needle);
        at.col += 2;
        let list = settled_completion(&mgr, caller_doc, at).await;
        assert!(list.items.iter().any(|i| i.label == label), "{label}: {:?}", list.items);
    }
    let action_start = pos(routes, "show')->");
    let action_at = Pos::new(action_start.line, action_start.col + 1);
    let list = settled_completion(&mgr, route_doc, action_at).await;
    let method = list.items.into_iter().find(|item| item.insert_text == "show").expect("controller method candidate");
    let expected_range = Range::new(action_start, Pos::new(action_start.line, action_start.col + 4));
    assert_eq!(
        method.replace_range,
        Some(expected_range),
        "quoted action must replace its whole UTF16 identifier on ordinary insertion"
    );
    assert!(!method.is_snippet, "route action must remain plain text");
    let resolved = mgr.resolve_completion(route_doc, method).await.unwrap();
    assert_eq!(resolved.replace_range, Some(expected_range), "resolving docs must retain the provider edit");
    let target = mgr.goto(route_doc, pos(routes, "show')->"), GotoKind::Definition).await.unwrap();
    let canonical = routes.replace(" as Routes", "").replace("Routes::", "Route::");
    replace(&mgr, route_doc, 2, routes, &canonical);
    let control = mgr.goto(route_doc, pos(&canonical, "show')->"), GotoKind::Definition).await.unwrap();
    assert!(
        control.iter().any(|l| l.path == root.join("app/InvoiceController.php") && l.range.start.line == 2),
        "canonical controller target: {control:?}"
    );
    replace(&mgr, route_doc, 3, &canonical, routes);
    assert!(
        target.iter().any(|l| l.path == root.join("app/InvoiceController.php") && l.range.start.line == 2),
        "aliased controller group target: {target:?}; canonical control: {control:?}"
    );
    let unsaved = routes.replace("{invoice}", "{receipt}");
    replace(&mgr, route_doc, 4, routes, &unsaved);
    let updated = caller.replace("['in']", "['re']");
    replace(&mgr, caller_doc, 2, caller, &updated);
    let mut at = pos(&updated, "re']);");
    at.col += 2;
    let mut labels = Vec::new();
    for _ in 0..80 {
        labels = settled_completion(&mgr, caller_doc, at).await.items.into_iter().map(|i| i.label).collect();
        if labels.iter().any(|l| l == "receipt") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        labels.iter().any(|l| l == "receipt") && !labels.iter().any(|l| l == "invoice"),
        "unsaved URI parameters: {labels:?}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("routes/web.php")).unwrap(),
        routes,
        "inspection must not persist unsaved registrations"
    );
    assert_eq!(std::fs::read_to_string(root.join("app/Caller.php")).unwrap(), caller);
    mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_config_and_translation_navigation_completion_and_unsaved_sources() {
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    for dir in ["config", "lang/en", "lang/cs", "app"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(
        root.join("composer.json"),
        r#"{"require":{"php":"^8.2","laravel/framework":"12.*"},"autoload":{"psr-4":{"App\\":"app/"}}}"#,
    )
    .unwrap();
    let config = "<?php\nreturn [\n    'retries' => 3,\n    'limits' => ['daily' => 10],\n];\n";
    std::fs::write(root.join("config/billing.php"), config).unwrap();
    let translation = "<?php\nreturn [\n    'welcome' => 'Welcome',\n    'invoice' => ['paid' => 'Paid'],\n];\n";
    for locale in ["en", "cs"] {
        std::fs::write(root.join(format!("lang/{locale}/messages.php")), translation).unwrap();
    }
    std::fs::write(root.join("lang/en.json"), r#"{"Hello invoice":"Invoice greeting"}"#).unwrap();
    let main = "<?php\n$label = '😀'; $count = config('billing.retries');\n$title = __('messages.welcome');\n$greeting = __('Hello invoice');\nopenstorm_scope_probe();\n";
    let other = "<?php\nfunction openstorm_scope_probe(): void {}\nconfig('billing.retries');\n__('messages.welcome');\n__('Hello invoice');\n";
    std::fs::write(root.join("main.php"), main).unwrap();
    std::fs::write(root.join("app/other.php"), other).unwrap();
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let mgr =
        LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_: IdeEvent| {}));
    let candidates =
        ["main.php", "app/other.php", "config/billing.php", "lang/en/messages.php", "lang/cs/messages.php"]
            .map(|p| root.join(p));
    mgr.set_candidates_hook(Box::new(move |_, _| ide_lsp::CandidateFiles {
        files: candidates.to_vec(),
        limit_hit: false,
    }));
    let doc = DocId::next();
    mgr.did_open(doc, &root.join("main.php"), &LanguageId::new("php"), 1, main);
    let mut config_target = Vec::new();
    for _ in 0..80 {
        config_target = mgr.goto(doc, pos(main, "retries'"), GotoKind::Definition).await.unwrap_or_default();
        if !config_target.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert_eq!(config_target.len(), 1, "config definition: {config_target:?}");
    assert!(config_target[0].path.ends_with("config/billing.php"));
    assert_eq!(config_target[0].range.start.line, 2);
    let target = mgr.goto(doc, pos(main, "welcome'"), GotoKind::Definition).await.unwrap();
    assert_eq!(target.len(), 2, "keep locale alternatives: {target:?}");
    for loc in &target {
        assert!(loc.path.ends_with("messages.php"));
        assert_eq!(loc.range.start.line, 2);
    }
    let json_target = mgr.goto(doc, pos(main, "Hello invoice"), GotoKind::Definition).await.unwrap();
    assert_eq!(json_target.len(), 1);
    assert!(json_target[0].path.ends_with("lang/en.json"));
    let json_doc = DocId::next();
    mgr.did_open(
        json_doc,
        &root.join("lang/en.json"),
        &LanguageId::new("json"),
        1,
        r#"{"Hello invoice":"Invoice greeting"}"#,
    );
    let json_refs = mgr.references(json_doc, Pos::new(0, 4), false, true, Default::default()).await.unwrap();
    assert!(json_refs.locations.iter().any(|l| l.path.ends_with("main.php")));
    assert!(json_refs.locations.iter().any(|l| l.path.ends_with("app/other.php")));
    let checked = mgr
        .rename(doc, pos(main, "openstorm_scope_probe"), "openstorm_scope_changed".into(), Default::default())
        .await
        .unwrap();
    assert_eq!(checked.edit.files.len(), 2, "checked PHP rename still works with a shared JSON overlay");
    for (needle, expected) in [("retries'", "billing.php"), ("welcome'", "messages.php"), ("Hello invoice", "en.json")]
    {
        let refs = mgr.references(doc, pos(main, needle), true, true, Default::default()).await.unwrap();
        assert!(refs.locations.iter().any(|l| l.path.ends_with("main.php")), "{expected} main usage: {refs:?}");
        assert!(
            refs.locations.iter().any(|l| l.path.ends_with("app/other.php")),
            "{expected} cross-file usage: {refs:?}"
        );
    }
    let mut previous = main.to_owned();
    for (version, expression, expected) in [
        (2, "config('billing.ret');", "billing.retries"),
        (3, "__('messages.wel');", "messages.welcome"),
        (4, "config('billing.limits.da');", "billing.limits.daily"),
        (5, "__('messages.invoice.pa');", "messages.invoice.paid"),
        (6, "__('Hello inv');", "Hello invoice"),
    ] {
        let text = format!("<?php\n{expression}\n");
        replace(&mgr, doc, version, &previous, &text);
        let result = mgr.completion(doc, pos(&text, "');"), None, Default::default()).await.unwrap();
        assert!(
            result.items.iter().any(|i| i.label == expected),
            "{expression}: {:?}",
            result.items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
        previous = text;
    }
    let config_doc = DocId::next();
    mgr.did_open(config_doc, &root.join("config/billing.php"), &LanguageId::new("php"), 1, config);
    let changed = config.replace("'retries'", "'attempts'");
    replace(&mgr, config_doc, 2, config, &changed);
    let text = "<?php\nconfig('billing.');\n";
    replace(&mgr, doc, 7, &previous, text);
    let result = mgr.completion(doc, pos(text, "');"), None, Default::default()).await.unwrap();
    assert!(
        result.items.iter().any(|i| i.label == "billing.attempts"),
        "unsaved config key: {:?}",
        result.items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
    assert!(!result.items.iter().any(|i| i.label == "billing.retries"));
    assert_eq!(std::fs::read_to_string(root.join("config/billing.php")).unwrap(), config);
    let translation_doc = DocId::next();
    mgr.did_open(translation_doc, &root.join("lang/cs/messages.php"), &LanguageId::new("php"), 1, translation);
    replace(&mgr, translation_doc, 2, translation, &translation.replace("'welcome'", "'greeting'"));
    let text2 = "<?php\n__('messages.greeting');\n__('messages.welcome');\n";
    replace(&mgr, doc, 8, text, text2);
    let new_target = mgr.goto(doc, pos(text2, "greeting'"), GotoKind::Definition).await.unwrap();
    assert_eq!(new_target.len(), 1, "new unsaved locale key: {new_target:?}");
    assert!(new_target[0].path.ends_with("lang/cs/messages.php"));
    let old_target = mgr.goto(doc, pos(text2, "welcome'"), GotoKind::Definition).await.unwrap();
    assert_eq!(old_target.len(), 1, "only the unchanged locale retains the old key: {old_target:?}");
    assert!(old_target[0].path.ends_with("lang/en/messages.php"));
    assert_eq!(std::fs::read_to_string(root.join("lang/cs/messages.php")).unwrap(), translation);
    let original_json = r#"{"Hello invoice":"Invoice greeting"}"#;
    replace(&mgr, json_doc, 2, original_json, &original_json.replace("Hello invoice", "Hello account"));
    let json_caller = "<?php\n__('Hello account');\n";
    replace(&mgr, doc, 9, text2, json_caller);
    let refs = mgr.references(json_doc, Pos::new(0, 4), false, true, Default::default()).await.unwrap();
    assert!(refs.locations.iter().any(|l| l.path.ends_with("main.php")), "unsaved JSON key: {refs:?}");
    assert!(!refs.locations.iter().any(|l| l.path.ends_with("app/other.php")), "old key must stay separate");
    assert_eq!(std::fs::read_to_string(root.join("lang/en.json")).unwrap(), original_json);
    let account_json = original_json.replace("Hello invoice", "Hello account");
    let code_sample = r#"{"Hello account":"<?php function openstorm_translation_fake(): void {} ?>"}"#;
    replace(&mgr, json_doc, 3, &account_json, code_sample);
    let sample_caller = "<?php\nopenstorm_translation_fake();\n";
    replace(&mgr, doc, 10, json_caller, sample_caller);
    let spurious = mgr.goto(doc, pos(sample_caller, "openstorm_translation_fake"), GotoKind::Definition).await.unwrap();
    assert!(
        spurious.is_empty(),
        "PHP examples inside JSON translation values must not define PHP symbols: {spurious:?}"
    );
    mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_environment_helper_keys_use_only_approved_synthetic_values() {
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    std::fs::create_dir(root.join("app")).unwrap();
    std::fs::write(
        root.join("composer.json"),
        r#"{"require":{"php":"^8.2","laravel/framework":"12.*"},"autoload":{"psr-4":{"App\\":"app/"}}}"#,
    )
    .unwrap();
    // The user authorized exactly these fake values in disposable test fixtures.
    let environment = "OPENSTORM_DEMO_LABEL=sample\nOPENSTORM_DEMO_MODE=local\n";
    std::fs::write(root.join(".env"), environment).unwrap();
    let main = "<?php\n$label = env('OPENSTORM_DEMO_LABEL');\n";
    std::fs::write(root.join("main.php"), main).unwrap();
    std::fs::write(root.join("app/other.php"), "<?php\nenv('OPENSTORM_DEMO_LABEL');\n").unwrap();
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let mgr =
        LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_: IdeEvent| {}));
    let candidates = [root.join("main.php"), root.join("app/other.php")];
    mgr.set_candidates_hook(Box::new(move |_, _| ide_lsp::CandidateFiles {
        files: candidates.to_vec(),
        limit_hit: false,
    }));
    let doc = DocId::next();
    mgr.did_open(doc, &root.join("main.php"), &LanguageId::new("php"), 1, main);
    let mut targets = Vec::new();
    for _ in 0..80 {
        targets = mgr.goto(doc, pos(main, "OPENSTORM_DEMO_LABEL"), GotoKind::Definition).await.unwrap_or_default();
        if !targets.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert_eq!(targets.len(), 1);
    assert!(targets[0].path.ends_with(".env"));
    assert_eq!(targets[0].range.start.line, 0);
    let env_doc = DocId::next();
    mgr.did_open(env_doc, &root.join(".env"), &LanguageId::new("dotenv"), 1, environment);
    let declaration_refs = mgr.references(env_doc, Pos::new(0, 5), false, true, Default::default()).await.unwrap();
    assert!(declaration_refs.locations.iter().any(|l| l.path.ends_with("app/other.php")));
    assert!(declaration_refs.locations.iter().any(|l| l.path.ends_with("main.php")));
    let refs = mgr.references(doc, pos(main, "OPENSTORM_DEMO_LABEL"), false, true, Default::default()).await.unwrap();
    assert!(refs.locations.iter().any(|l| l.path.ends_with("app/other.php")));
    let partial = "<?php\nenv('OPENSTORM_DEMO_L');\n";
    replace(&mgr, doc, 2, main, partial);
    let completions = mgr.completion(doc, pos(partial, "');"), None, Default::default()).await.unwrap();
    assert!(completions.items.iter().any(|i| i.label == "OPENSTORM_DEMO_LABEL"));
    let environment_updated = "  export OPENSTORM_DEMO_MODE=local\n";
    replace(&mgr, env_doc, 2, environment, environment_updated);
    let all_keys = "<?php\nenv('OPENSTORM_DEMO_');\n";
    replace(&mgr, doc, 3, partial, all_keys);
    let completions = mgr.completion(doc, pos(all_keys, "');"), None, Default::default()).await.unwrap();
    assert!(completions.items.iter().any(|i| i.label == "OPENSTORM_DEMO_MODE"));
    assert!(!completions.items.iter().any(|i| i.label == "OPENSTORM_DEMO_LABEL"));
    let mode_call = "<?php\nenv('OPENSTORM_DEMO_MODE');\n";
    replace(&mgr, doc, 4, all_keys, mode_call);
    let definitions = mgr.goto(doc, pos(mode_call, "OPENSTORM_DEMO_MODE"), GotoKind::Definition).await.unwrap();
    let target = definitions.iter().find(|target| target.path.ends_with(".env")).unwrap();
    assert_eq!(target.range.start.col, 9, "navigation must land on the key after indentation and export");
    let refs = mgr.references(env_doc, target.range.start, false, true, Default::default()).await.unwrap();
    assert!(refs.locations.iter().any(|l| l.path.ends_with("main.php")));
    mgr.did_close(env_doc);
    replace(&mgr, doc, 5, mode_call, all_keys);
    let completions = mgr.completion(doc, pos(all_keys, "');"), None, Default::default()).await.unwrap();
    assert!(completions.items.iter().any(|i| i.label == "OPENSTORM_DEMO_LABEL"));
    assert_eq!(std::fs::read_to_string(root.join(".env")).unwrap(), environment);
    mgr.shutdown_all().await;
}

/// This corpus is acquired by the pinned engine build recipe and installed with
/// explicit Composer actions; opening the document never boots the framework.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires OPENSTORM_TEST_ENGINE_LARAVEL_ROOT with pinned engine example and Laravel 13.37.0"]
async fn laravel_casts_relationships_scopes_and_facades_through_client() {
    let root = PathBuf::from(std::env::var_os("OPENSTORM_TEST_ENGINE_LARAVEL_ROOT").expect("engine Laravel fixture"))
        .canonicalize()
        .unwrap();
    let lock: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("composer.lock")).unwrap()).unwrap();
    let framework = lock["packages"].as_array().unwrap().iter().find(|p| p["name"] == "laravel/framework").unwrap();
    assert_eq!(framework["version"], "v13.37.0");
    for name in ["app/Models/Bakery.php", "app/Models/BlogPost.php", "app/Facades/Oven.php"] {
        assert!(root.join(name).is_file(), "missing pinned corpus file {name}");
    }
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let doc = DocId::next();
    let path = root.join("openstorm-eloquent-client.php");
    assert!(!path.exists(), "acceptance document must remain an overlay");
    let prefix = "<?php\nuse App\\Models\\{Bakery, BlogPost};\nuse App\\Facades\\Oven;\n$bakery = new Bakery();\n$post = new BlogPost();\n";
    let mut text = format!("{prefix}$bakery->apricot;\n");
    mgr.did_open(doc, &path, &LanguageId::new("php"), 1, &text);
    let cases = [
        ("$bakery->apricot;", "apricot", "bool"),
        ("$bakery->quality;", "quality", "float"),
        ("$bakery->proved_at;", "proved_at", "Carbon"),
        ("$bakery->jam_flavor;", "jam_flavor", "JamFlavor"),
        ("$bakery->icing;", "icing", "?Frosting"),
        ("$bakery->baguettes;", "baguettes", "Collection<Loaf>"),
        ("$post->author;", "author", "BlogAuthor"),
        ("$post->author()->first()->displayName;", "displayName", "string"),
        ("Bakery::query()->freshlyBaked()->first()->quality;", "quality", "float"),
        ("Oven::bake('rye');", "bake", "string"),
        ("app('pastry.oven')->bake('rye');", "bake", "string"),
    ];
    for (index, (expression, symbol, expected)) in cases.iter().enumerate() {
        let next = format!("{prefix}{expression}\n");
        replace(&mgr, doc, index as u32 + 2, &text, &next);
        text = next;
        let mut content = String::new();
        for _ in 0..80 {
            if let Ok(Some(hover)) =
                mgr.hover(doc, Pos::new(5, expression.find(symbol).unwrap() as u32), Default::default()).await
            {
                content = hover.contents.value;
                if content.contains(expected) {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(content.contains(expected), "{expression}: expected {expected} in hover, got {content:?}");
        if index < 9 {
            assert!(content.contains(&format!("**var** `{expected}`")), "precise type for {expression}: {content}");
        }
        eprintln!("accepted hover {expression}: {content}");
    }
    let completions = [
        ("Bakery::query()->fre", "freshlyBaked"),
        ("Bakery::query()->freshlyBaked()->first()->qua", "quality"),
        ("$post->author()->first()->dis", "displayName"),
        ("$bakery->baguettes->first()->getW", "getWeight"),
        ("Oven::ba", "bake"),
        ("app('pastry.oven')->ba", "bake"),
    ];
    for (index, (expression, expected)) in completions.iter().enumerate() {
        let next = format!("{prefix}{expression}");
        replace(&mgr, doc, index as u32 + 100, &text, &next);
        text = next;
        let list = mgr.completion(doc, Pos::new(5, expression.len() as u32), None, Default::default()).await.unwrap();
        assert!(list.items.iter().any(|i| i.label.starts_with(expected)), "missing {expected} for {expression}");
    }
    let targets = [
        ("Oven::bake('rye');", "bake", "app/Support/BakeryService.php"),
        ("$post->author();", "author", "app/Models/BlogPost.php"),
        ("Bakery::query()->freshlyBaked();", "freshlyBaked", "app/Models/Bakery.php"),
    ];
    for (index, (expression, symbol, target)) in targets.iter().enumerate() {
        let next = format!("{prefix}{expression}\n");
        replace(&mgr, doc, index as u32 + 200, &text, &next);
        text = next;
        let locations =
            mgr.goto(doc, Pos::new(5, expression.find(symbol).unwrap() as u32), GotoKind::Definition).await.unwrap();
        assert!(
            locations.iter().any(|l| l.path.ends_with(target)),
            "{expression}: expected {target}, got {locations:?}"
        );
        let location = locations.iter().find(|l| l.path.ends_with(target)).unwrap();
        let source = std::fs::read_to_string(&location.path).unwrap();
        assert!(
            source.lines().nth(location.range.start.line as usize).unwrap().contains(&format!("function {symbol}(")),
            "destination must be the exact declaration"
        );
    }
    let model_path = root.join("app/Models/Bakery.php");
    let original = std::fs::read_to_string(&model_path).unwrap();
    let changed = original.replace("'quality' => 'float'", "'quality' => 'bool'");
    assert_ne!(original, changed, "pinned quality cast fixture");
    let model_doc = DocId::next();
    mgr.did_open(model_doc, &model_path, &LanguageId::new("php"), 1, &changed);
    let next = format!("{prefix}$bakery->quality;\n");
    replace(&mgr, doc, 300, &text, &next);
    let hover = mgr.hover(doc, Pos::new(5, 9), Default::default()).await.unwrap().unwrap();
    assert!(hover.contents.value.contains("bool"), "unsaved cast overlay: {}", hover.contents.value);
    assert_eq!(std::fs::read_to_string(&model_path).unwrap(), original);
    mgr.did_close(model_doc);
    let hover = mgr.hover(doc, Pos::new(5, 9), Default::default()).await.unwrap().unwrap();
    assert!(hover.contents.value.contains("float"), "closing overlay restores disk cast: {}", hover.contents.value);
    assert!(!path.exists(), "intelligence queries must not write the overlay to disk");
    mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_view_data_tracks_unsaved_callers_and_template_shadowing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    for dir in ["app", "resources/views"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(
        root.join("composer.json"),
        r#"{"require":{"php":"^8.2","laravel/framework":"12.*"},"autoload":{"psr-4":{"App\\":"app/"}}}"#,
    )
    .unwrap();
    let item = "<?php\nnamespace App;\nclass Item { public string $name; public int $price; }\n";
    let stock = "<?php\nnamespace App;\nclass Stock { public string $sku; public int $quantity; }\n";
    std::fs::write(root.join("app/Item.php"), item).unwrap();
    std::fs::write(root.join("app/Stock.php"), stock).unwrap();
    let controller = "<?php\nnamespace App;\nclass Controller { public function show(): mixed {\n$item = new Item();\nreturn view('shop', compact('item'));\n} }\n";
    std::fs::write(root.join("app/Controller.php"), controller).unwrap();
    let template = "<h1>😀 shop</h1>\n{{ $item->name }}\n";
    let template_path = root.join("resources/views/shop.blade.php");
    std::fs::write(&template_path, template).unwrap();
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let caller = DocId::next();
    mgr.did_open(caller, &root.join("app/Controller.php"), &LanguageId::new("php"), 1, controller);
    let blade = DocId::next();
    // OpenStorm currently detects .blade.php as PHP; test the actual client path.
    mgr.did_open(blade, &template_path, &LanguageId::new("php"), 1, template);
    let mut hover = String::new();
    for _ in 0..80 {
        if let Ok(Some(h)) = mgr.hover(blade, Pos::new(1, 10), Default::default()).await {
            hover = h.contents.value;
            if hover.contains("string") {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(hover.contains("string"), "call-site data reaches Blade hover: {hover}");
    let locations = mgr.goto(blade, Pos::new(1, 10), GotoKind::Definition).await.unwrap();
    assert!(
        locations.iter().any(|l| l.path == root.join("app/Item.php") && l.range.start.line == 2),
        "Blade member definition: {locations:?}"
    );
    let list = settled_completion(&mgr, blade, Pos::new(1, 10)).await;
    assert!(list.items.iter().any(|i| i.label == "name"));
    assert!(list.items.iter().any(|i| i.label == "price"));
    let changed_template = "{{-- unsaved template --}}\n<h1>😀 shop</h1>\n{{ $item->sku }}\n";
    replace(&mgr, blade, 2, template, changed_template);
    let changed_controller = controller.replace("new Item()", "new Stock()");
    replace(&mgr, caller, 2, controller, &changed_controller);
    let list = settled_completion(&mgr, blade, Pos::new(2, 10)).await;
    let labels: Vec<_> = list.items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"sku") && labels.contains(&"quantity"), "unsaved caller updates template: {labels:?}");
    assert!(!labels.contains(&"price"), "stale caller type must retire: {labels:?}");
    let locations = mgr.goto(blade, Pos::new(2, 10), GotoKind::Definition).await.unwrap();
    assert!(locations.iter().any(|l| l.path == root.join("app/Stock.php")), "new member maps to Stock: {locations:?}");
    let partial = "{{-- unsaved template --}}\n<h1>😀 shop</h1>\n{{ $item->qu";
    replace(&mgr, blade, 3, changed_template, partial);
    let list = settled_completion(&mgr, blade, Pos::new(2, 12)).await;
    assert!(
        list.items.iter().any(|i| i.label == "quantity"),
        "incomplete Blade echo must retain PHP member completion: {:?}",
        list.items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
    let completed_partial = partial.replace("->qu", "->quantity");
    replace(&mgr, blade, 4, partial, &completed_partial);
    let _ = settled_completion(&mgr, blade, Pos::new(2, 18)).await;
    let before_restart = mgr.goto(blade, Pos::new(2, 12), GotoKind::Definition).await.unwrap();
    assert!(
        before_restart.iter().any(|l| l.path == root.join("app/Stock.php")),
        "unsaved incomplete member before restart: {before_restart:?}"
    );
    mgr.restart(mgr.servers()[0].id).await.unwrap();
    let list = settled_completion(&mgr, blade, Pos::new(2, 18)).await;
    assert!(
        list.items.iter().any(|i| i.label == "quantity"),
        "restart restores unsaved caller and template: {:?}",
        list.items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
    let after_restart = mgr.goto(blade, Pos::new(2, 12), GotoKind::Definition).await.unwrap();
    assert!(
        after_restart.iter().any(|l| l.path == root.join("app/Stock.php")),
        "unsaved incomplete member after restart: {after_restart:?}"
    );
    replace(&mgr, blade, 5, &completed_partial, changed_template);
    let shadowed = "@php $item = new \\App\\Item(); @endphp\n{{ $item->name }}\n";
    replace(&mgr, blade, 6, changed_template, shadowed);
    let list = settled_completion(&mgr, blade, Pos::new(1, 10)).await;
    let labels: Vec<_> = list.items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"name") && labels.contains(&"price"), "template-local assignment wins: {labels:?}");
    assert!(!labels.contains(&"quantity"), "caller cannot override local binding: {labels:?}");
    assert_eq!(std::fs::read_to_string(root.join("app/Controller.php")).unwrap(), controller);
    assert_eq!(std::fs::read_to_string(template_path).unwrap(), template);
    mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires OPENSTORM_TEST_VIEW_ROOT disposable native view-data fixture"]
async fn laravel_native_view_fixture_reopens_unsaved_dependencies() {
    let root = PathBuf::from(std::env::var_os("OPENSTORM_TEST_VIEW_ROOT").expect("native view fixture"))
        .canonicalize()
        .unwrap();
    let caller_path = root.join("app/OpenStormFixtures/Controller.php");
    let original = std::fs::read_to_string(&caller_path).unwrap();
    let caller_text = original.replace("new Item()", "new Stock()");
    assert_ne!(original, caller_text);
    let template_path = root.join("resources/views/openstorm-stock.blade.php");
    let original_template = std::fs::read_to_string(&template_path).unwrap();
    let template_text = "{{-- unsaved template --}}\n<h1>😀 shop</h1>\n{{ $item->quantity";
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = std::env::var("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable");
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let blade = DocId::next();
    let caller = DocId::next();
    mgr.did_open(blade, &template_path, &LanguageId::new("php"), 1, template_text);
    mgr.did_open(caller, &caller_path, &LanguageId::new("php"), 1, &caller_text);
    for cycle in 0..3 {
        if cycle > 0 {
            mgr.restart(mgr.servers()[0].id).await.unwrap();
        }
        let mut ready = false;
        let mut cancellations = 0;
        for _ in 0..150 {
            match mgr.completion(blade, Pos::new(2, 18), None, Default::default()).await {
                Ok(list) => {
                    let labels: Vec<_> = list.items.iter().map(|i| i.label.as_str()).collect();
                    assert!(labels.contains(&"quantity"), "first successful response in cycle {cycle}: {labels:?}");
                    ready = true;
                    break;
                }
                Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {
                    cancellations += 1;
                }
                Err(error) => panic!("{error}"),
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(ready, "cycle {cycle}: {cancellations} cancellations without a current response");
        let hover = mgr.hover(blade, Pos::new(2, 12), Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("public int $quantity"), "cycle {cycle}: {}", hover.contents.value);
        let targets = mgr.goto(blade, Pos::new(2, 12), GotoKind::Definition).await.unwrap();
        assert!(
            targets.iter().any(|l| l.path == root.join("app/OpenStormFixtures/Stock.php")),
            "cycle {cycle}: {targets:?}"
        );
    }
    assert_eq!(std::fs::read_to_string(caller_path).unwrap(), original);
    assert_eq!(std::fs::read_to_string(template_path).unwrap(), original_template);
    mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_view_data_propagates_through_unsaved_includes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    for dir in ["app", "resources/views/partials"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    let controller = "<?php\nnamespace App;\nclass Controller { public function show(): mixed {\n$item = new Item();\nreturn view('page', compact('item'));\n} }\n";
    let page = "@include('partials.row', ['row' => $item])\n";
    let partial = "<p>😀</p>\n{{ $row->name }}\n";
    let files = [
        (
            "composer.json",
            r#"{"require":{"php":"^8.2","laravel/framework":"12.*"},"autoload":{"psr-4":{"App\\":"app/"}}}"#,
        ),
        ("app/Item.php", "<?php\nnamespace App;\nclass Item { public string $name; public int $price; }\n"),
        ("app/Stock.php", "<?php\nnamespace App;\nclass Stock { public string $sku; public int $quantity; }\n"),
        ("app/Controller.php", controller),
        ("resources/views/page.blade.php", page),
        ("resources/views/partials/row.blade.php", partial),
    ];
    for (name, text) in files {
        std::fs::write(root.join(name), text).unwrap();
    }
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = std::env::var("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable");
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let leaf = DocId::next();
    let middle = DocId::next();
    let caller = DocId::next();
    // Open the leaf first: earlier startup publication must not strand its inferred scope.
    for (doc, name, text) in [
        (leaf, "resources/views/partials/row.blade.php", partial),
        (middle, "resources/views/page.blade.php", page),
        (caller, "app/Controller.php", controller),
    ] {
        mgr.did_open(doc, &root.join(name), &LanguageId::new("php"), 1, text);
    }
    let list = settled_completion(&mgr, leaf, Pos::new(1, 9)).await;
    assert!(list.items.iter().any(|i| i.label == "price"), "controller -> page -> partial: {:?}", list.items);
    let changed_controller = controller.replace("new Item()", "new Stock()");
    replace(&mgr, caller, 2, controller, &changed_controller);
    let changed_partial = partial.replace("->name", "->quantity");
    replace(&mgr, leaf, 2, partial, &changed_partial);
    let list = settled_completion(&mgr, leaf, Pos::new(1, 9)).await;
    let labels: Vec<_> = list.items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"quantity") && !labels.contains(&"price"), "current caller reaches partial: {labels:?}");
    let targets = mgr.goto(leaf, Pos::new(1, 11), GotoKind::Definition).await.unwrap();
    assert!(targets.iter().any(|l| l.path == root.join("app/Stock.php")), "mapped nested member target: {targets:?}");
    let shadowed_page = format!("@php $item = new \\App\\Item(); @endphp\n{page}");
    replace(&mgr, middle, 2, page, &shadowed_page);
    let list = settled_completion(&mgr, leaf, Pos::new(1, 9)).await;
    let labels: Vec<_> = list.items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"price") && !labels.contains(&"quantity"),
        "unsaved intermediate binding wins: {labels:?}"
    );
    mgr.restart(mgr.servers()[0].id).await.unwrap();
    let list = settled_completion(&mgr, leaf, Pos::new(1, 9)).await;
    assert!(list.items.iter().any(|i| i.label == "price"), "restart retains unsaved intermediate include scope");
    mgr.did_close(middle);
    let list = settled_completion(&mgr, leaf, Pos::new(1, 9)).await;
    let labels: Vec<_> = list.items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"quantity") && !labels.contains(&"price"),
        "closing intermediate overlay restores disk include and current caller: {labels:?}"
    );
    for (name, text) in files {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text);
    }
    mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires OPENSTORM_TEST_ENGINE_LARAVEL_ROOT pinned Laravel 13.37.0 fixture"]
async fn laravel_policy_and_request_keys_through_client() {
    let root = PathBuf::from(std::env::var_os("OPENSTORM_TEST_ENGINE_LARAVEL_ROOT").expect("engine Laravel fixture"))
        .canonicalize()
        .unwrap();
    let lock: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("composer.lock")).unwrap()).unwrap();
    assert!(
        lock["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "laravel/framework" && p["version"] == "v13.37.0")
    );
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = std::env::var("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable");
    let events: Arc<Mutex<Vec<IdeEvent>>> = Arc::default();
    let capture = events.clone();
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![php])),
        Arc::new(move |event| capture.lock().push(event)),
    );
    let path = root.join("openstorm-policy-client.php");
    assert!(!path.exists(), "acceptance caller must remain an overlay");
    let prefix = "<?php\n// 😀 policy and validation acceptance\nuse App\\Http\\Requests\\{StoreBakeryRequest, UpdateBakeryRequest};\nuse App\\Models\\{Bakery, BlogPost};\nuse Illuminate\\Support\\Facades\\Gate;\nfunction probe(StoreBakeryRequest $request, UpdateBakeryRequest $update): void {\n$bakery = new Bakery();\n";
    let mut text = format!("{prefix}Gate::allows('');\n}}\n");
    let doc = DocId::next();
    mgr.did_open(doc, &path, &LanguageId::new("php"), 1, &text);
    let abilities = settled_completion(&mgr, doc, pos(&text, "');")).await;
    let labels: Vec<_> = abilities.items.iter().map(|i| i.label.as_str()).collect();
    for name in ["manage-bakery-network", "update", "publish", "moderate"] {
        assert!(labels.contains(&name), "missing ability {name}: {labels:?}");
    }
    assert!(
        !labels.contains(&"before") && !labels.contains(&"sameOwner"),
        "hooks/protected methods are not abilities: {labels:?}"
    );
    let targets = [
        (
            "Gate::allows('manage-bakery-network', 'north');",
            "manage-bakery-network",
            "app/Providers/DemoServiceProvider.php",
            "Gate::define('manage-bakery-network'",
        ),
        ("Gate::allows('update', $bakery);", "update", "app/Policies/BakeryPolicy.php", "function update("),
        (
            "Gate::allows('publish', BlogPost::class);",
            "publish",
            "app/Policies/PublishingPolicy.php",
            "function publish(",
        ),
        ("$request->input('name');", "name", "app/Http/Requests/StoreBakeryRequest.php", "'name' =>"),
        ("$update->input('name');", "name", "app/Http/Requests/StoreBakeryRequest.php", "'name' =>"),
        ("$update->input('slug');", "slug", "app/Http/Requests/UpdateBakeryRequest.php", "'slug' =>"),
    ];
    for (index, (expression, symbol, target, declaration)) in targets.iter().enumerate() {
        let next = format!("{prefix}{expression}\n}}\n");
        replace(&mgr, doc, index as u32 + 2, &text, &next);
        text = next;
        let at = Pos::new(7, expression.find(symbol).unwrap() as u32);
        let _ = settled_completion(&mgr, doc, at).await;
        let locations = mgr.goto(doc, at, GotoKind::Definition).await.unwrap();
        let found = locations
            .iter()
            .find(|l| l.path == root.join(target))
            .unwrap_or_else(|| panic!("{expression}: {locations:?}"));
        let source = std::fs::read_to_string(&found.path).unwrap();
        assert!(
            source.lines().nth(found.range.start.line as usize).unwrap().contains(declaration),
            "exact declaration for {expression}: {found:?}"
        );
        let hover = mgr
            .hover(doc, at, Default::default())
            .await
            .unwrap()
            .unwrap_or_else(|| panic!("missing hover for {expression} at {at:?}"));
        assert!(hover.contents.value.contains(symbol), "documentation for {expression}: {}", hover.contents.value);
    }
    let expression = "$update->input('');";
    let next = format!("{prefix}{expression}\n}}\n");
    replace(&mgr, doc, 20, &text, &next);
    text = next;
    let at = pos(&text, "');");
    let list = settled_completion(&mgr, doc, at).await;
    let labels: Vec<_> = list.items.iter().map(|i| i.label.as_str()).collect();
    for key in ["slug", "name", "apricot", "notes", "owner.email"] {
        assert!(labels.contains(&key), "inherited validation field {key}: {labels:?}");
    }
    assert!(!labels.contains(&"notes.*.body"), "wildcard rule offers its addressable root: {labels:?}");
    let request_path = root.join("app/Http/Requests/StoreBakeryRequest.php");
    let original = std::fs::read_to_string(&request_path).unwrap();
    let changed =
        original.replace("'name' => 'required|string|max:255'", "'business_name' => 'required|string|max:255'");
    assert_ne!(original, changed);
    let request = DocId::next();
    mgr.did_open(request, &request_path, &LanguageId::new("php"), 1, &changed);
    let list = settled_completion(&mgr, doc, at).await;
    let labels: Vec<_> = list.items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"business_name") && !labels.contains(&"name"),
        "unsaved inherited rules refresh input keys: {labels:?}"
    );
    let current = format!("{prefix}$update->input('business_name');\n}}\n");
    replace(&mgr, doc, 21, &text, &current);
    text = current;
    let current_at = pos(&text, "business_name");
    let _ = settled_completion(&mgr, doc, current_at).await;
    let hover = mgr.hover(doc, current_at, Default::default()).await.unwrap().expect("unsaved field documentation");
    assert!(hover.contents.value.contains("business_name") && hover.contents.value.contains("required|string|max:255"));
    mgr.restart(mgr.servers()[0].id).await.unwrap();
    let list = settled_completion(&mgr, doc, at).await;
    assert!(list.items.iter().any(|i| i.label == "business_name"), "restart preserves unsaved rule declaration");
    let hover =
        mgr.hover(doc, current_at, Default::default()).await.unwrap().expect("restart restores field documentation");
    assert!(hover.contents.value.contains("business_name"));
    mgr.did_close(request);
    let list = settled_completion(&mgr, doc, at).await;
    let labels: Vec<_> = list.items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"name") && !labels.contains(&"business_name"),
        "closing rule overlay restores disk contract: {labels:?}"
    );
    assert!(
        mgr.hover(doc, current_at, Default::default()).await.unwrap().is_none(),
        "retired field must not retain stale documentation"
    );
    let bad = format!("{prefix}Gate::allows('publish', $bakery);\n}}\n");
    replace(&mgr, doc, 40, &text, &bad);
    let mut rejected = false;
    for _ in 0..100 {
        rejected = events
            .lock()
            .iter()
            .rev()
            .find_map(|e| match e {
                IdeEvent::Diagnostics { path: diagnostic_path, items, .. } if diagnostic_path.0 == path => {
                    Some(items.iter().any(|d| {
                        d.code.as_deref() == Some("invalid_laravel_ability")
                            && d.message.contains("publish")
                            && d.message.contains("BakeryPolicy")
                    }))
                }
                _ => None,
            })
            .unwrap_or(false);
        if rejected {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(rejected, "an ability from another model must be diagnosed");
    assert_eq!(std::fs::read_to_string(request_path).unwrap(), original);
    assert!(!path.exists(), "queries must not persist the caller overlay");
    mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires OPENSTORM_TEST_ENGINE_LARAVEL_ROOT pinned Laravel 13.37.0 fixture"]
async fn laravel_request_documentation_as_first_request_after_open_and_restart() {
    let root = PathBuf::from(std::env::var_os("OPENSTORM_TEST_ENGINE_LARAVEL_ROOT").expect("engine Laravel fixture"))
        .canonicalize()
        .unwrap();
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = std::env::var("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable");
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let path = root.join("app/Demo.php");
    let text = std::fs::read_to_string(&path).unwrap();
    let at = pos(&text, "'name');          // → 'name' => 'required|string|max:255'");
    let at = Pos::new(at.line, at.col + 2);
    let doc = DocId::next();
    mgr.did_open(doc, &path, &LanguageId::new("php"), 1, &text);
    for cycle in 0..3 {
        if cycle > 0 {
            mgr.restart(mgr.servers()[0].id).await.unwrap();
        }
        let start = std::time::Instant::now();
        let hover = mgr.hover(doc, at, Default::default()).await.unwrap();
        eprintln!("first request cycle {cycle}: {:?}, hover={hover:?}", start.elapsed());
        assert!(
            hover.is_some_and(|h| h.contents.value.contains("required|string|max:255")),
            "first request cycle {cycle}"
        );
    }
    assert_eq!(std::fs::read_to_string(path).unwrap(), text);
    mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires OPENSTORM_TEST_ENGINE_LARAVEL_ROOT pinned Laravel 13.37.0 fixture"]
async fn laravel_validation_rule_names_through_client() {
    let root = PathBuf::from(std::env::var_os("OPENSTORM_TEST_ENGINE_LARAVEL_ROOT").expect("engine Laravel fixture"))
        .canonicalize()
        .unwrap();
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = std::env::var("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable");
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let path = root.join("openstorm-rule-names-client.php");
    assert!(!path.exists(), "the acceptance source stays an overlay");
    let prefix = "<?php\nnamespace App;\nuse Illuminate\\Foundation\\Http\\FormRequest;\nuse Illuminate\\Http\\Request;\nuse Illuminate\\Support\\Facades\\Validator;\n";
    let scenarios = [
        "class RuleNameProbe extends FormRequest { public function rules(): array { /* 😀 */ return ['title' => 'string|requ']; } }",
        "class RuleNameProbe extends FormRequest { public function rules(): array { return ['title' => ['string', 'requ']]; } }",
        "function probe(Request $request): void { $request->validate(['title' => 'string|requ']); }",
        "function probe(Request|UnknownRuleReceiver $request): void { if ($request instanceof Request) { $request->validate(['title' => 'string|requ']); } }",
        "function probe(array $data): void { Validator::make($data, ['title' => 'string|requ']); }",
        "function probe(array $data): void { Validator::make($data, ['title' => 'string|requ'], messages: []); }",
        "function probe(array $data): void { Validator::make(rules: ['title' => 'string|requ'], data: $data); }",
    ];
    let doc = DocId::next();
    let mut text = format!("{prefix}{}\n", scenarios[0]);
    mgr.did_open(doc, &path, &LanguageId::new("php"), 1, &text);
    for (index, scenario) in scenarios.iter().enumerate() {
        let next = format!("{prefix}{scenario}\n");
        if index > 0 {
            replace(&mgr, doc, index as u32 + 1, &text, &next);
            text = next;
        }
        let start = pos(&text, "requ'");
        let end = Pos::new(start.line, start.col + 4);
        let list = settled_completion(&mgr, doc, end).await;
        assert!(list.is_incomplete, "runtime extensions remain explicitly incomplete");
        let required = list
            .items
            .iter()
            .find(|i| i.label == "required")
            .unwrap_or_else(|| panic!("validation rule names for {scenario}: {list:?}"));
        assert_eq!(required.insert_text, "required");
        assert_eq!(required.replace_range, Some(Range::new(start, end)), "only the current rule name is replaced");
        assert!(required.additional_edits.is_empty());
    }
    let next = format!("{prefix}{}\n", scenarios[0].replace("requ'", "required'"));
    replace(&mgr, doc, 10, &text, &next);
    text = next;
    let at = pos(&text, "required'");
    let _ = settled_completion(&mgr, doc, at).await;
    let definitions = mgr.goto(doc, at, GotoKind::Definition).await.unwrap();
    let definition = definitions
        .iter()
        .find(|d| d.path.ends_with("Validation/Concerns/ValidatesAttributes.php"))
        .unwrap_or_else(|| panic!("installed validation method origin: {definitions:?}"));
    let declaration = std::fs::read_to_string(&definition.path).unwrap();
    assert!(
        declaration.lines().nth(definition.range.start.line as usize).unwrap().contains("function validateRequired(")
    );
    let hover = mgr.hover(doc, at, Default::default()).await.unwrap().expect("validation rule documentation");
    assert!(hover.contents.value.to_lowercase().contains("required"));
    for (index, expression) in [
        "function unrelated(): array { return ['title' => 'requ']; }",
        "function probe(Request|UnknownRuleReceiver $request): void { $request->validate(['title' => 'requ']); }",
        "class Ordinary { public function rules(): array { return ['title' => 'requ']; } }",
        "class RuleNameProbe extends FormRequest { public function messages(): array { return ['title' => 'requ']; } }",
        "class RuleNameProbe extends FormRequest { public function rules(): array { return ['title' => 'min:requ']; } }",
        "class RuleNameProbe extends FormRequest { public function rules(): array { return ['title' => ['regex:/requ/']]; } }",
    ].iter().enumerate() {
        let next = format!("{prefix}{expression}\n");
        replace(&mgr, doc, 20 + index as u32, &text, &next); text = next;
        let marker = if expression.contains("/requ/") { "requ/" } else { "requ'" };
        let start = pos(&text, marker);
        let list = settled_completion(&mgr, doc, Pos::new(start.line, start.col + 4)).await;
        assert!(!list.items.iter().any(|i| i.label == "required"), "not a validation rule name: {expression}");
    }
    let next = format!(
        "{prefix}class RuleNameProbe extends FormRequest {{ public function rules(): array {{ return ['title' => 'string|requ"
    );
    replace(&mgr, doc, 30, &text, &next);
    text = next;
    let start = pos(&text, "requ");
    let list = settled_completion(&mgr, doc, Pos::new(start.line, start.col + 4)).await;
    let item = list.items.iter().find(|item| item.label == "required").expect("unfinished EOF rule completion");
    assert_eq!(item.replace_range, Some(Range::new(start, Pos::new(start.line, start.col + 4))));
    mgr.shutdown_all().await;
    assert!(!path.exists());
}

/// Control without the host candidate-file hook; the host regression covers widening.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires OPENSTORM_TEST_ENGINE_LARAVEL_ROOT pinned Laravel 13.37.0 fixture"]
async fn laravel_framework_source_direct_references_and_hover_control() {
    let root = PathBuf::from(std::env::var_os("OPENSTORM_TEST_ENGINE_LARAVEL_ROOT").expect("engine Laravel fixture"))
        .canonicalize()
        .unwrap();
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = std::env::var("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable");
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let request_path = root.join("app/Http/Requests/StoreBakeryRequest.php");
    let trait_path = root.join("vendor/laravel/framework/src/Illuminate/Validation/Concerns/ValidatesAttributes.php");
    let request = std::fs::read_to_string(&request_path).unwrap();
    let trait_source = std::fs::read_to_string(&trait_path).unwrap();
    let doc = DocId::next();
    mgr.did_open(doc, &request_path, &LanguageId::new("php"), 1, &request);
    mgr.did_open(DocId::next(), &trait_path, &LanguageId::new("php"), 1, &trait_source);
    let at = pos(&request, "'name');");
    let at = Pos::new(at.line, at.col + 2);
    let class = pos(&request, "StoreBakeryRequest extends");
    let method = pos(&request, "rules(): array");
    let outcome: Result<(), String> = async {
        for cycle in 0..3 {
            if cycle > 0 {
                mgr.restart(mgr.servers()[0].id).await.map_err(|e| format!("restart {cycle}: {e:?}"))?;
            }
            let initial =
                mgr.hover(doc, at, Default::default()).await.map_err(|e| format!("initial hover {cycle}: {e:?}"))?;
            if !initial.is_some_and(|h| h.contents.value.contains("required|string|max:255")) {
                return Err(format!("initial request-field docs missing in cycle {cycle}"));
            }
            let start = std::time::Instant::now();
            let results = tokio::time::timeout(Duration::from_secs(15), async {
                tokio::join!(
                    mgr.references(doc, class, false, true, Default::default()),
                    mgr.references(doc, method, false, true, Default::default()),
                    async {
                        tokio::time::sleep(Duration::from_millis(150)).await;
                        mgr.hover(doc, at, Default::default()).await
                    }
                )
            })
            .await;
            eprintln!("framework direct-reference control {cycle}: {:?}, settled={}", start.elapsed(), results.is_ok());
            match results {
                Ok((Ok(class_refs), Ok(method_refs), Ok(Some(hover))))
                    if !class_refs.locations.is_empty()
                        && !method_refs.locations.is_empty()
                        && hover.contents.value.contains("required|string|max:255") => {}
                other => return Err(format!("framework direct-reference control {cycle} did not settle: {other:?}")),
            }
        }
        Ok(())
    }
    .await;
    mgr.shutdown_all().await;
    assert_eq!(std::fs::read_to_string(request_path).unwrap(), request);
    assert_eq!(std::fs::read_to_string(trait_path).unwrap(), trait_source);
    assert!(outcome.is_ok(), "{}", outcome.unwrap_err());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn unsaved_route_registration_refreshes_sibling_diagnostics_through_client() {
    async fn wait_duplicate(
        events: &Mutex<Vec<IdeEvent>>,
        path: &std::path::Path,
        since: usize,
        expected: bool,
    ) -> bool {
        for _ in 0..200 {
            let state = events.lock()[since..].iter().rev().find_map(|event| match event {
                IdeEvent::Diagnostics { path: target, items, .. } if target.0 == path => {
                    Some(items.iter().any(|d| d.code.as_deref() == Some("duplicate_laravel_route")))
                }
                _ => None,
            });
            if state == Some(expected) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        false
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    std::fs::create_dir(root.join("routes")).unwrap();
    std::fs::write(root.join("composer.json"), r#"{"require":{"laravel/framework":"^13.0"}}"#).unwrap();
    std::fs::write(root.join(".phpantom.toml"), "[indexing]\nstrategy = \"none\"\n[diagnostics]\nworkspace = false\n")
        .unwrap();
    let base =
        "<?php\nuse Illuminate\\Support\\Facades\\Route as Routes;\nRoutes::get('/a/😀', 'show')->name('shared');\n";
    let other =
        "<?php\nuse Illuminate\\Support\\Facades\\Route as Routes;\nRoutes::get('/b', 'show')->name('shared');\n";
    let paths = [root.join("routes/web.php"), root.join("routes/api.php")];
    for (path, text) in paths.iter().zip([base, other]) {
        std::fs::write(path, text).unwrap();
    }
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let events: Arc<Mutex<Vec<IdeEvent>>> = Arc::default();
    let capture = events.clone();
    let mgr = LspManager::new(
        root,
        Arc::new(LanguageRegistry::from_configs(vec![php])),
        Arc::new(move |e| capture.lock().push(e)),
    );
    let docs = [DocId::next(), DocId::next()];
    for ((path, text), doc) in paths.iter().zip([base, other]).zip(docs) {
        mgr.did_open(doc, path, &LanguageId::new("php"), 1, text);
    }
    let initial =
        wait_duplicate(&events, &paths[0], 0, true).await && wait_duplicate(&events, &paths[1], 0, true).await;
    if !initial {
        mgr.shutdown_all().await;
        panic!("fixture must first publish both duplicate warnings");
    }
    let since = events.lock().len();
    replace(&mgr, docs[1], 2, other, &other.replace("'shared'", "'unique'"));
    let edited_refreshed = wait_duplicate(&events, &paths[1], since, false).await;
    let sibling_refreshed = wait_duplicate(&events, &paths[0], since, false).await;
    mgr.shutdown_all().await;
    for (path, text) in paths.iter().zip([base, other]) {
        assert_eq!(std::fs::read_to_string(path).unwrap(), text, "diagnostics must not persist the overlay");
    }
    assert!(edited_refreshed, "edited registration must clear its own duplicate warning");
    assert!(sibling_refreshed, "unchanged open sibling must receive fresh diagnostics after unsaved registration edit");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_view_roots_packages_and_unsaved_provider_navigation() {
    async fn target(mgr: &LspManager, doc: DocId, name: &str, expected: &std::path::Path) -> Result<(), String> {
        let mut last = Vec::new();
        for _ in 0..80 {
            let reply =
                mgr.laravel_views(doc, Some(name.into()), 2000, Default::default()).await.map_err(|e| e.to_string())?;
            if reply.locations.len() == 1 && reply.locations[0].path.0 == expected {
                return Ok(());
            }
            last = reply.locations;
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        Err(format!("{name} did not resolve to {expected:?}: {last:?}"))
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let provider = "<?php\nnamespace App\\Providers;\nclass NavigationViewsProvider {\n public function boot(): void {\n  $this->loadViewsFrom(__DIR__.'/../../packages/widgets/templates', 'openstorm-widgets');\n }\n}\n";
    let config = "<?php\nreturn ['paths' => [base_path('themes/current'), resource_path('views')]];\n";
    let caller = "<?php\nnamespace App;\nclass NavigationViews {\n public function render(): void {\n  $label = '😀 žluťoučký';\n  view('openstorm-widgets::card');\n  view('panel');\n  view('legacy.report');\n }\n}\n";
    let files = [
        (
            "composer.json",
            r#"{"require":{"php":"^8.3","laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#,
        ),
        (".phpantom.toml", "[indexing]\nstrategy = \"none\"\n[diagnostics]\nworkspace = false\n"),
        ("bootstrap/providers.php", "<?php\nreturn [App\\Providers\\NavigationViewsProvider::class];\n"),
        ("app/Providers/NavigationViewsProvider.php", provider),
        ("config/view.php", config),
        ("app/NavigationViews.php", caller),
        ("packages/widgets/templates/card.blade.php", "<p>Package original</p>\n"),
        ("resources/views/vendor/openstorm-widgets/card.blade.php", "<p>Published override</p>\n"),
        ("resources/views/vendor/openstorm-widgets/banner.blade.php", "<p>Published only</p>\n"),
        ("themes/current/panel.blade.php", "<p>Current theme</p>\n"),
        ("themes/alternate/panel.blade.php", "<p>Alternate theme</p>\n"),
        ("resources/views/panel.blade.php", "<p>Default theme</p>\n"),
        ("resources/views/legacy/report.php", "<p>Plain PHP template</p>\n"),
    ];
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let doc = DocId::next();
    let provider_doc = DocId::next();
    let config_doc = DocId::next();
    mgr.did_open(doc, &root.join("app/NavigationViews.php"), &LanguageId::new("php"), 1, caller);
    mgr.did_open(
        provider_doc,
        &root.join("app/Providers/NavigationViewsProvider.php"),
        &LanguageId::new("php"),
        1,
        provider,
    );
    mgr.did_open(config_doc, &root.join("config/view.php"), &LanguageId::new("php"), 1, config);
    let outcome: Result<(), String> = async {
        let names = mgr.laravel_views(doc, None, 2000, Default::default()).await.map_err(|e| e.to_string())?;
        for name in ["openstorm-widgets::card", "openstorm-widgets::banner", "panel", "legacy.report"] {
            if !names.views.iter().any(|v| v == name) {
                return Err(format!("missing view {name}: {:?}", names.views));
            }
        }
        for (name, expected) in [
            ("openstorm-widgets::card", "resources/views/vendor/openstorm-widgets/card.blade.php"),
            ("openstorm-widgets::banner", "resources/views/vendor/openstorm-widgets/banner.blade.php"),
            ("panel", "themes/current/panel.blade.php"),
            ("legacy.report", "resources/views/legacy/report.php"),
        ] {
            let reply =
                mgr.laravel_views(doc, Some(name.into()), 2000, Default::default()).await.map_err(|e| e.to_string())?;
            if reply.locations.len() != 1 || reply.locations[0].path.0 != root.join(expected) {
                return Err(format!("{name} must use rendering precedence {expected}: {:?}", reply.locations));
            }
        }
        let changed = provider.replace("'openstorm-widgets'", "'openstorm-preview'");
        replace(&mgr, provider_doc, 2, provider, &changed);
        let mut refreshed = false;
        for _ in 0..80 {
            let reply = mgr
                .laravel_views(doc, Some("openstorm-preview::card".into()), 2000, Default::default())
                .await
                .map_err(|e| e.to_string())?;
            if reply.locations.len() == 1
                && reply.locations[0].path.0 == root.join("packages/widgets/templates/card.blade.php")
            {
                refreshed = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        if !refreshed {
            return Err("unsaved provider namespace did not refresh package view targets".into());
        }
        let old = mgr
            .laravel_views(doc, Some("openstorm-widgets::card".into()), 2000, Default::default())
            .await
            .map_err(|e| e.to_string())?;
        if !old.locations.is_empty() {
            return Err(format!("old namespace must no longer resolve: {:?}", old.locations));
        }
        let config_changed = config
            .replace("themes/current", "themes/alternate")
            .replace("resource_path('views')", "resource_path('views'), base_path('themes/later')");
        replace(&mgr, config_doc, 2, config, &config_changed);
        target(&mgr, doc, "panel", &root.join("themes/alternate/panel.blade.php")).await?;

        let created = root.join("themes/alternate/live-report.php");
        std::fs::write(&created, "<p>Added plain PHP view</p>\n").map_err(|e| e.to_string())?;
        mgr.php_files_changed(std::slice::from_ref(&created), &[], &[]);
        target(&mgr, doc, "live-report", &created).await?;
        let mut added = false;
        let mut last_names = Vec::new();
        for _ in 0..80 {
            let names = mgr.laravel_views(doc, None, 2000, Default::default()).await.map_err(|e| e.to_string())?;
            last_names = names.views;
            if last_names.iter().any(|v| v == "live-report") {
                added = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        if !added {
            return Err(format!(
                "created custom-root PHP template missing from chooser after watched-file deadline: {last_names:?}"
            ));
        }
        std::fs::remove_file(&created).map_err(|e| e.to_string())?;
        mgr.php_files_changed(&[], &[], std::slice::from_ref(&created));
        let mut removed = false;
        for _ in 0..80 {
            let names = mgr.laravel_views(doc, None, 2000, Default::default()).await.map_err(|e| e.to_string())?;
            let reply = mgr
                .laravel_views(doc, Some("live-report".into()), 2000, Default::default())
                .await
                .map_err(|e| e.to_string())?;
            if !names.views.iter().any(|v| v == "live-report") && reply.locations.is_empty() {
                removed = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        if !removed {
            return Err("deleted custom-root template remains in chooser or navigation".into());
        }
        let first_in_new_root = root.join("themes/later/first.php");
        std::fs::create_dir_all(first_in_new_root.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(&first_in_new_root, "<p>First template in a configured root</p>\n")
            .map_err(|e| e.to_string())?;
        mgr.php_files_changed(std::slice::from_ref(&first_in_new_root), &[], &[]);
        target(&mgr, doc, "first", &first_in_new_root).await?;
        let mut discovered = false;
        for _ in 0..80 {
            let names = mgr.laravel_views(doc, None, 2000, Default::default()).await.map_err(|e| e.to_string())?;
            if names.views.iter().any(|v| v == "first") {
                discovered = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        if !discovered {
            return Err("first plain PHP template in newly created configured root missing from chooser".into());
        }
        std::fs::remove_file(&first_in_new_root).map_err(|e| e.to_string())?;
        mgr.php_files_changed(&[], &[], std::slice::from_ref(&first_in_new_root));
        mgr.restart(mgr.servers()[0].id).await.map_err(|e| e.to_string())?;
        target(&mgr, doc, "panel", &root.join("themes/alternate/panel.blade.php")).await?;
        target(&mgr, doc, "openstorm-preview::card", &root.join("packages/widgets/templates/card.blade.php")).await?;
        let bounded = mgr.laravel_views(doc, None, 1, Default::default()).await.map_err(|e| e.to_string())?;
        if bounded.views.len() != 1 || !bounded.incomplete {
            return Err(format!("view list must retain bounded incomplete result: {bounded:?}"));
        }
        let cancel = ide_core::CancellationToken::new();
        cancel.cancel();
        match mgr.laravel_views(doc, None, 2000, cancel).await {
            Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {}
            other => return Err(format!("cancelled view query must not produce results: {other:?}")),
        }
        mgr.did_close(provider_doc);
        mgr.did_close(config_doc);
        target(&mgr, doc, "panel", &root.join("themes/current/panel.blade.php")).await?;
        target(
            &mgr,
            doc,
            "openstorm-widgets::card",
            &root.join("resources/views/vendor/openstorm-widgets/card.blade.php"),
        )
        .await?;
        Ok(())
    }
    .await;
    mgr.shutdown_all().await;
    for (name, original) in files {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), original, "query must preserve {name}");
    }
    assert!(outcome.is_ok(), "{}", outcome.unwrap_err());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_closed_view_config_disk_changes_refresh_navigation() {
    async fn diagnostic(events: &Mutex<Vec<IdeEvent>>, path: &std::path::Path, since: usize, invalid: bool) -> bool {
        for _ in 0..200 {
            let current = events.lock()[since..].iter().rev().find_map(|event| match event {
                IdeEvent::Diagnostics { path: target, items, .. } if target.0 == path => {
                    Some(items.iter().any(|item| item.code.as_deref() == Some("invalid_laravel_view")))
                }
                _ => None,
            });
            if current == Some(invalid) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        false
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let original = "<?php return ['paths' => [base_path('themes/current')]];\n";
    let caller = "<?php view('current-only');\n";
    let files = [
        ("composer.json", r#"{"require":{"laravel/framework":"^13.0"}}"#),
        (".phpantom.toml", "[indexing]\nstrategy = \"none\"\n[diagnostics]\nworkspace = false\n"),
        ("config/view.php", original),
        ("main.php", caller),
        ("themes/current/current-only.blade.php", "<p>Current root</p>\n"),
        ("themes/alternate/alternate-only.blade.php", "<p>Alternate root</p>\n"),
        ("resources/views/default-only.blade.php", "<p>Default root</p>\n"),
    ];
    for (name, text) in files {
        let p = root.join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let events: Arc<Mutex<Vec<IdeEvent>>> = Arc::default();
    let capture = events.clone();
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![php])),
        Arc::new(move |event| capture.lock().push(event)),
    );
    let doc = DocId::next();
    mgr.did_open(doc, &root.join("main.php"), &LanguageId::new("php"), 1, caller);
    let config_path = root.join("config/view.php");
    let outcome: Result<(), String> = async {
        let initial = mgr.laravel_views(doc, None, 2000, Default::default()).await.map_err(|e| e.to_string())?;
        if initial.views != ["current-only"] {
            return Err(format!("initial closed config must select current root: {:?}", initial.views));
        }
        eprintln!("Initial closed configuration selects current-only; config document has never been opened");
        if !diagnostic(&events, &root.join("main.php"), 0, false).await { return Err("initial view caller diagnostics were not published".into()); }
        for (deleted, expected, absent) in
            [(false, "alternate-only", "current-only"), (true, "default-only", "alternate-only")]
        {
            let since = events.lock().len();
            if deleted {
                std::fs::remove_file(&config_path).map_err(|e| e.to_string())?;
                mgr.php_files_changed(&[], &[], std::slice::from_ref(&config_path));
            } else {
                std::fs::write(&config_path, original.replace("themes/current", "themes/alternate"))
                    .map_err(|e| e.to_string())?;
                mgr.php_files_changed(&[], std::slice::from_ref(&config_path), &[]);
            }
            let mut refreshed = false;
            let mut last = Vec::new();
            for _ in 0..80 {
                let reply = mgr.laravel_views(doc, None, 2000, Default::default()).await.map_err(|e| e.to_string())?;
                last = reply.views;
                if last.iter().any(|v| v == expected) && !last.iter().any(|v| v == absent) {
                    refreshed = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            if !refreshed {
                return Err(format!(
                    "closed config deleted={deleted}: expected {expected}, absent {absent}; got {last:?}"
                ));
            }
            let reply = mgr
                .laravel_views(doc, Some(expected.into()), 2000, Default::default())
                .await
                .map_err(|e| e.to_string())?;
            let suffix = if deleted {
                "resources/views/default-only.blade.php"
            } else {
                "themes/alternate/alternate-only.blade.php"
            };
            if reply.locations.len() != 1 || reply.locations[0].path.0 != root.join(suffix) {
                return Err(format!("closed config target must follow {suffix}: {:?}", reply.locations));
            }
            if !deleted && !diagnostic(&events, &root.join("main.php"), since, true).await {
                return Err("closed configuration lookup refreshed, but unchanged open caller never received its invalid-view diagnostic".into());
            }
        }
        let since = events.lock().len();
        std::fs::write(&config_path, original).map_err(|e| e.to_string())?;
        mgr.php_files_changed(&[], std::slice::from_ref(&config_path), &[]);
        if !diagnostic(&events, &root.join("main.php"), since, false).await { return Err("restoring closed view configuration did not clear the caller diagnostic".into()); }
        Ok(())
    }
    .await;
    mgr.shutdown_all().await;
    std::fs::write(&config_path, original).unwrap();
    for (name, text) in files {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text);
    }
    assert!(outcome.is_ok(), "{}", outcome.unwrap_err());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_view_data_cycles_retire_unsaved_external_seeds_through_client() {
    async fn members(
        mgr: &LspManager,
        doc: DocId,
        text: &str,
        required: &[&str],
        forbidden: &[&str],
    ) -> Result<(), String> {
        tokio::time::timeout(Duration::from_secs(8), async {
            let mut last = Vec::new();
            for _ in 0..80 {
                match mgr
                    .completion(
                        doc,
                        pos(text, if text.contains("userOnly") { "userOnly" } else { "otherOnly" }),
                        None,
                        Default::default(),
                    )
                    .await
                {
                    Ok(reply) => {
                        last = reply.items.into_iter().map(|item| item.label).collect();
                        if required.iter().all(|name| last.iter().any(|label| label == name))
                            && forbidden.iter().all(|name| last.iter().all(|label| label != name))
                        {
                            return Ok(());
                        }
                    }
                    Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {}
                    Err(error) => return Err(error.to_string()),
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(format!("required {required:?}, forbidden {forbidden:?}; got {last:?}"))
        })
        .await
        .map_err(|_| "cycle completion exceeded eight-second deadline".to_string())?
    }
    async fn unknown_variable(
        events: &Mutex<Vec<IdeEvent>>,
        path: &std::path::Path,
        since: usize,
        expected: bool,
    ) -> Result<(), String> {
        for _ in 0..200 {
            let latest = events.lock()[since..].iter().rev().find_map(|event| match event {
                IdeEvent::Diagnostics { path: target, items, .. } if target.0 == path => {
                    Some(items.iter().any(|item| item.code.as_deref() == Some("unknown_variable")))
                }
                _ => None,
            });
            if latest == Some(expected) {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        Err(format!("cycle unknown_variable diagnostic did not become {expected} after event {since}"))
    }
    // These recursive templates are a static analysis robustness fixture, never rendered.
    for explicit in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let data = if explicit { ", ['person' => $person]" } else { "" };
        let page = format!("@include('partial'{data})\n{{{{ $person->userOnly }}}}\n");
        let partial = format!("@include('page'{data})\n{{{{ $person->userOnly }}}}\n");
        let caller = "<?php\nnamespace App;\nclass Controller { public function show(): mixed {\n$person = new User();\nreturn view('page', compact('person'));\n} }\n";
        let files = [
            ("composer.json", r#"{"require":{"laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#),
            ("app/User.php", "<?php namespace App; class User { public string $userOnly; }\n"),
            ("app/Other.php", "<?php namespace App; class Other { public int $otherOnly; }\n"),
            ("app/Controller.php", caller),
            ("resources/views/page.blade.php", page.as_str()),
            ("resources/views/partial.blade.php", partial.as_str()),
        ];
        for (name, text) in files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
        php.language_servers.retain(|server| server.name == "phpantom");
        php.language_servers[0].command = std::env::var("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable");
        let events: Arc<Mutex<Vec<IdeEvent>>> = Arc::default();
        let capture = events.clone();
        let mgr = LspManager::new(
            root.clone(),
            Arc::new(LanguageRegistry::from_configs(vec![php])),
            Arc::new(move |event| capture.lock().push(event)),
        );
        let leaf = DocId::next();
        let middle = DocId::next();
        let controller = DocId::next();
        for (doc, name, text) in [
            (leaf, "resources/views/partial.blade.php", partial.as_str()),
            (middle, "resources/views/page.blade.php", page.as_str()),
            (controller, "app/Controller.php", caller),
        ] {
            mgr.did_open(doc, &root.join(name), &LanguageId::new("php"), 1, text);
        }
        let outcome: Result<(), String> = async {
            for (doc, text) in [(leaf, partial.as_str()), (middle, page.as_str())] {
                members(&mgr, doc, text, &["userOnly"], &["otherOnly"]).await?;
            }
            let initial = mgr
                .completion(leaf, pos(&partial, "userOnly"), None, Default::default())
                .await
                .map_err(|error| error.to_string())?;
            if let Some(member) = initial.items.iter().find(|item| item.label == "userOnly") {
                eprintln!(
                    "Cycle explicit={explicit} member insert range {:?}, Tab replace range {:?}",
                    member.replace_range, member.replace_range_tab
                );
            }
            let changed = caller.replace("new User()", "new Other()");
            replace(&mgr, controller, 2, caller, &changed);
            for (doc, text) in [(leaf, partial.as_str()), (middle, page.as_str())] {
                members(&mgr, doc, text, &["otherOnly"], &["userOnly"]).await?;
                let hover =
                    mgr.hover(doc, pos(text, "$person"), Default::default()).await.map_err(|e| e.to_string())?;
                let value = hover.map(|value| value.contents.value).unwrap_or_default();
                if !value.contains("Other") || value.contains("App\\User") {
                    return Err(format!("cycle hover must use current external seed: {value}"));
                }
            }
            let changed_partial = partial.replace("userOnly", "otherOnly");
            replace(&mgr, leaf, 2, &partial, &changed_partial);
            members(&mgr, leaf, &changed_partial, &["otherOnly"], &["userOnly"]).await?;
            let targets = mgr
                .goto(leaf, pos(&changed_partial, "otherOnly"), GotoKind::Definition)
                .await
                .map_err(|error| error.to_string())?;
            if !targets.iter().any(|target| target.path == root.join("app/Other.php") && target.range.start.line == 0) {
                return Err(format!("current cycle member must map to Other declaration: {targets:?}"));
            }
            replace(&mgr, leaf, 3, &changed_partial, &partial);
            mgr.restart(mgr.servers()[0].id).await.map_err(|error| error.to_string())?;
            members(&mgr, leaf, &partial, &["otherOnly"], &["userOnly"]).await?;
            let removed = changed.replace("return view('page', compact('person'));", "return null;");
            let before_remove = events.lock().len();
            replace(&mgr, controller, 3, &changed, &removed);
            for (doc, text) in [(leaf, partial.as_str()), (middle, page.as_str())] {
                members(&mgr, doc, text, &[], &["userOnly", "otherOnly"]).await?;
            }
            unknown_variable(&events, &root.join("resources/views/partial.blade.php"), before_remove, true).await?;
            let before_restore = events.lock().len();
            replace(&mgr, controller, 4, &removed, caller);
            for (doc, text) in [(leaf, partial.as_str()), (middle, page.as_str())] {
                members(&mgr, doc, text, &["userOnly"], &["otherOnly"]).await?;
            }
            unknown_variable(&events, &root.join("resources/views/partial.blade.php"), before_restore, false).await?;
            let before_second_remove = events.lock().len();
            replace(&mgr, controller, 5, caller, &removed);
            unknown_variable(&events, &root.join("resources/views/partial.blade.php"), before_second_remove, true)
                .await?;
            let before_close = events.lock().len();
            mgr.did_close(controller);
            for (doc, text) in [(leaf, partial.as_str()), (middle, page.as_str())] {
                members(&mgr, doc, text, &["userOnly"], &["otherOnly"]).await?;
            }
            unknown_variable(&events, &root.join("resources/views/partial.blade.php"), before_close, false).await?;
            Ok(())
        }
        .await;
        mgr.shutdown_all().await;
        for (name, text) in files {
            assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text, "disk must remain original: {name}");
        }
        assert!(outcome.is_ok(), "explicit={explicit}: {}", outcome.unwrap_err());
        eprintln!("Cycle explicit={explicit}: completion/hover type retirement, restart and closing overlay passed");
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_component_props_preserve_supplied_models_and_scope_through_client() {
    async fn members(
        mgr: &LspManager,
        doc: DocId,
        text: &str,
        needle: &str,
        yes: &[&str],
        no: &[&str],
    ) -> Result<(), String> {
        tokio::time::timeout(Duration::from_secs(8), async {
            let mut last = Vec::new();
            for _ in 0..80 {
                match mgr.completion(doc, pos(text, needle), None, Default::default()).await {
                    Ok(reply) => {
                        last = reply.items.into_iter().map(|item| item.label).collect();
                        if yes.iter().all(|name| last.iter().any(|item| item == name))
                            && no.iter().all(|name| last.iter().all(|item| item != name))
                        {
                            return Ok(());
                        }
                    }
                    Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {}
                    Err(error) => return Err(error.to_string()),
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(format!("at {needle}: required {yes:?}, forbidden {no:?}; got {last:?}"))
        })
        .await
        .map_err(|_| "component scope query exceeded eight-second deadline".to_string())?
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let caller = "<?php\nnamespace App;\nclass Caller { public function show(): mixed {\n$person = new User();\n$leaked = new Other();\nreturn view('page', compact('person', 'leaked'));\n} }\n";
    let page = "<x-card :person=\"$person\" />\n";
    let component = "@props(['person'])\n<p>😀</p>{{ $person->userOnly }}\n{{ $leaked->otherOnly }}\n";
    let bare = component.replace("@props(['person'])\n", "");
    let files = [
        ("composer.json", r#"{"require":{"laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#),
        ("app/User.php", "<?php namespace App; class User { public string $userOnly; }\n"),
        ("app/Other.php", "<?php namespace App; class Other { public int $otherOnly; }\n"),
        ("app/Caller.php", caller),
        ("resources/views/page.blade.php", page),
        ("resources/views/components/card.blade.php", component),
    ];
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|server| server.name == "phpantom");
    php.language_servers[0].command = std::env::var("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable");
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let leaf = DocId::next();
    let middle = DocId::next();
    let controller = DocId::next();
    for (doc, path, text) in [
        (leaf, "resources/views/components/card.blade.php", bare.as_str()),
        (middle, "resources/views/page.blade.php", page),
        (controller, "app/Caller.php", caller),
    ] {
        mgr.did_open(doc, &root.join(path), &LanguageId::new("php"), 1, text);
    }
    let outcome: Result<(), String> = async {
        members(&mgr, leaf, &bare, "userOnly", &["userOnly"], &["otherOnly"]).await?;
        eprintln!("Control: bare component receives supplied User model");
        replace(&mgr, leaf, 2, &bare, component);
        members(&mgr, leaf, component, "userOnly", &["userOnly"], &["otherOnly"]).await?;
        eprintln!("Required component prop receives supplied User model");
        let hover = mgr.hover(leaf, pos(component, "$person"), Default::default()).await.map_err(|e| e.to_string())?;
        let value = hover.map(|reply| reply.contents.value).unwrap_or_default();
        if !value.contains("User") || value.contains("Other") {
            return Err(format!("required prop hover must retain User: {value}"));
        }
        let targets =
            mgr.goto(leaf, pos(component, "userOnly"), GotoKind::Definition).await.map_err(|e| e.to_string())?;
        let declaration = pos(files[1].1, "userOnly");
        if !targets.iter().any(|target| target.path == root.join("app/User.php") && target.range.start == declaration) {
            return Err(format!("required prop member must map to User property at {declaration:?}: {targets:?}"));
        }
        members(&mgr, leaf, component, "otherOnly", &[], &["userOnly", "otherOnly"]).await?;
        eprintln!("Component does not inherit unbound parent variable");
        let changed = caller.replace("new User()", "new Other()");
        replace(&mgr, controller, 2, caller, &changed);
        members(&mgr, leaf, component, "userOnly", &["otherOnly"], &["userOnly"]).await?;
        eprintln!("Unsaved caller updates required component prop to Other");
        let nullable = component.replace("@props(['person'])", "@props(['person' => null])");
        replace(&mgr, leaf, 3, component, &nullable);
        members(&mgr, leaf, &nullable, "userOnly", &["otherOnly"], &["userOnly"]).await?;
        eprintln!("Nullable prop default preserves the supplied model");
        mgr.restart(mgr.servers()[0].id).await.map_err(|error| error.to_string())?;
        members(&mgr, leaf, &nullable, "userOnly", &["otherOnly"], &["userOnly"]).await?;
        mgr.did_close(controller);
        members(&mgr, leaf, &nullable, "userOnly", &["userOnly"], &["otherOnly"]).await?;
        Ok(())
    }
    .await;
    mgr.shutdown_all().await;
    for (name, text) in files {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text, "disk original: {name}");
    }
    assert!(outcome.is_ok(), "{}", outcome.unwrap_err());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_nested_component_forwarding_tracks_unsaved_scope_boundaries() {
    async fn members(mgr: &LspManager, doc: DocId, text: &str, yes: &[&str], no: &[&str]) -> Result<(), String> {
        tokio::time::timeout(Duration::from_secs(8), async {
            let mut last = Vec::new();
            for _ in 0..80 {
                match mgr.completion(doc, pos(text, "userOnly"), None, Default::default()).await {
                    Ok(reply) => {
                        last = reply.items.into_iter().map(|item| item.label).collect();
                        if yes.iter().all(|name| last.iter().any(|item| item == name))
                            && no.iter().all(|name| last.iter().all(|item| item != name))
                        {
                            return Ok(());
                        }
                    }
                    Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {}
                    Err(error) => return Err(error.to_string()),
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(format!("required {yes:?}, forbidden {no:?}; got {last:?}"))
        })
        .await
        .map_err(|_| "nested component query exceeded eight-second deadline".to_string())?
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let caller = "<?php\nnamespace App;\nclass Caller { public function show(): mixed {\n$person = new User();\n$leaked = new Other();\nreturn view('page', compact('person', 'leaked'));\n} }\n";
    let page = "<x-card :person=\"$person\"><x-slot:label><p>😀</p>{{ $leaked->userOnly }}</x-slot:label></x-card>\n";
    let card = "@props(['person'])\n<x-badge :account=\"$person\" />\n";
    let badge = "@props(['account'])\n<p>😀</p>{{ $account->userOnly }}\n";
    let files = [
        ("composer.json", r#"{"require":{"laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#),
        ("app/User.php", "<?php namespace App; class User { public string $userOnly; }\n"),
        ("app/Other.php", "<?php namespace App; class Other { public int $otherOnly; }\n"),
        ("app/Caller.php", caller),
        ("resources/views/page.blade.php", page),
        ("resources/views/components/card.blade.php", card),
        ("resources/views/components/badge.blade.php", badge),
    ];
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|server| server.name == "phpantom");
    php.language_servers[0].command = std::env::var("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable");
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let leaf = DocId::next();
    let middle = DocId::next();
    let controller = DocId::next();
    let parent = DocId::next();
    for (doc, path, text) in [
        (leaf, "resources/views/components/badge.blade.php", badge),
        (middle, "resources/views/components/card.blade.php", card),
        (controller, "app/Caller.php", caller),
    ] {
        mgr.did_open(doc, &root.join(path), &LanguageId::new("php"), 1, text);
    }
    let outcome: Result<(), String> = async {
        members(&mgr, leaf, badge, &["userOnly"], &["otherOnly"]).await?;
        eprintln!("Nested aliased prop follows closed page and open outer component");
        mgr.did_open(parent, &root.join("resources/views/page.blade.php"), &LanguageId::new("php"), 1, page);
        members(&mgr, parent, page, &["otherOnly"], &["userOnly"]).await?;
        let slot_targets = mgr.goto(parent, pos(page, "userOnly"), GotoKind::Definition).await.map_err(|e| e.to_string())?;
        // Completion uses an intentionally obsolete member; navigation must use
        // the selected current member to verify the mapped Unicode range.
        let selected = page.replace("userOnly", "otherOnly");
        replace(&mgr, parent, 2, page, &selected);
        let current = settled_completion(&mgr, parent, pos(&selected, "otherOnly")).await;
        if !current.items.iter().any(|item| item.label == "otherOnly") || current.items.iter().any(|item| item.label == "userOnly") {
            return Err(format!("settled slot completion must retain Other: {:?}", current.items));
        }
        let current_hover = mgr.hover(parent, pos(&selected, "$leaked"), Default::default()).await.map_err(|e| e.to_string())?;
        let slot_type = current_hover.map(|reply| reply.contents.value).unwrap_or_default();
        if !slot_type.contains("Other") || slot_type.contains("User") {
            return Err(format!("slot hover must retain caller Other: {slot_type}"));
        }
        let selected_targets = mgr.goto(parent, pos(&selected, "otherOnly"), GotoKind::Definition).await.map_err(|e| e.to_string())?;
        if !slot_targets.is_empty() || !selected_targets.iter().any(|target| target.path == root.join("app/Other.php") && target.range.start == pos(files[2].1, "otherOnly")) {
            return Err(format!("slot body must retain caller scope and exact Other property: obsolete={slot_targets:?}, selected={selected_targets:?}"));
        }
        eprintln!("Named slot body retains caller scope after same-line Unicode prefix");
        let targets = mgr.goto(leaf, pos(badge, "userOnly"), GotoKind::Definition).await.map_err(|e| e.to_string())?;
        if !targets
            .iter()
            .any(|target| target.path == root.join("app/User.php") && target.range.start == pos(files[1].1, "userOnly"))
        {
            return Err(format!("nested prop must reach exact User property: {targets:?}"));
        }
        let changed = caller.replace("new User()", "new Other()");
        replace(&mgr, controller, 2, caller, &changed);
        members(&mgr, leaf, badge, &["otherOnly"], &["userOnly"]).await?;
        eprintln!("Unsaved caller retires User through both component edges");
        // An unbound parent variable must not become available merely because
        // a child names it in a binding. Blade components have isolated data.
        let isolated = card.replace("$person", "$leaked");
        replace(&mgr, middle, 2, card, &isolated);
        members(&mgr, leaf, badge, &[], &["userOnly", "otherOnly"]).await?;
        eprintln!("Unbound grandparent data does not leak through a nested binding");
        replace(&mgr, middle, 3, &isolated, card);
        members(&mgr, leaf, badge, &["otherOnly"], &["userOnly"]).await?;
        mgr.restart(mgr.servers()[0].id).await.map_err(|error| error.to_string())?;
        members(&mgr, leaf, badge, &["otherOnly"], &["userOnly"]).await?;
        mgr.did_close(controller);
        members(&mgr, leaf, badge, &["userOnly"], &["otherOnly"]).await?;
        eprintln!("Restart replays nested overlays; closing caller restores disk User");
        Ok(())
    }
    .await;
    mgr.shutdown_all().await;
    for (name, text) in files {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text, "disk original: {name}");
    }
    assert!(outcome.is_ok(), "{}", outcome.unwrap_err());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_aware_component_data_tracks_explicit_ancestors_and_unsaved_callers() {
    async fn members(mgr: &LspManager, doc: DocId, text: &str, yes: &[&str], no: &[&str]) -> Result<(), String> {
        tokio::time::timeout(Duration::from_secs(8), async {
            let mut last = Vec::new();
            for _ in 0..80 {
                match mgr.completion(doc, pos(text, "userOnly"), None, Default::default()).await {
                    Ok(reply) => {
                        last = reply.items.into_iter().map(|item| item.label).collect();
                        if yes.iter().all(|name| last.iter().any(|item| item == name))
                            && no.iter().all(|name| last.iter().all(|item| item != name))
                        {
                            return Ok(());
                        }
                    }
                    Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {}
                    Err(error) => return Err(error.to_string()),
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(format!("required {yes:?}, forbidden {no:?}; got {last:?}"))
        })
        .await
        .map_err(|_| "nested component query exceeded eight-second deadline".to_string())?
    }
    async fn type_at(
        mgr: &LspManager,
        doc: DocId,
        text: &str,
        expected_any: &[&str],
        absent: &[&str],
    ) -> Result<(), String> {
        tokio::time::timeout(Duration::from_secs(8), async {
            let mut last = String::new();
            for _ in 0..80 {
                match mgr.hover(doc, pos(text, "$person"), Default::default()).await {
                    Ok(reply) => {
                        last = reply.map(|reply| reply.contents.value).unwrap_or_default();
                        if expected_any.iter().any(|ty| last.contains(ty)) && absent.iter().all(|ty| !last.contains(ty))
                        {
                            return Ok(());
                        }
                    }
                    Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {}
                    Err(error) => return Err(error.to_string()),
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(format!("aware type needs one of {expected_any:?}, excludes {absent:?}; got {last}"))
        })
        .await
        .map_err(|_| "aware hover exceeded eight-second deadline".to_string())?
    }
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let caller = "<?php\nnamespace App;\nclass Caller { public function show(): mixed {\n$person = new User();\n$leaked = new Other();\nreturn view('page', compact('person', 'leaked'));\n} }\n";
    let page = "<x-card :person=\"$person\" />\n";
    let card = "@props(['person'])\n<x-badge :person=\"$person\" />\n";
    let badge = "@props(['person'])\n<p>😀</p>{{ $person->userOnly }}\n";
    let user_source = "<?php namespace App; class User { public string $userOnly; }\n";
    let files = [
        (
            "composer.json",
            r#"{"require":{"laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/","Illuminate\\":"stubs/Illuminate/"}}}"#,
        ),
        (
            "stubs/Illuminate/View/ComponentAttributeBag.php",
            r#"<?php namespace Illuminate\View; class ComponentAttributeBag { public function __construct(private array $attributes = []) {} public function getAttributes(): array { return $this->attributes; } }"#,
        ),
        ("app/User.php", user_source),
        ("app/Other.php", "<?php namespace App; class Other { public int $otherOnly; }\n"),
        ("app/Caller.php", caller),
        ("resources/views/page.blade.php", page),
        ("resources/views/components/card.blade.php", card),
        ("resources/views/components/badge.blade.php", badge),
    ];
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|server| server.name == "phpantom");
    php.language_servers[0].command = std::env::var("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable");
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let leaf = DocId::next();
    let middle = DocId::next();
    let controller = DocId::next();
    let parent = DocId::next();
    for (doc, path, text) in [
        (leaf, "resources/views/components/badge.blade.php", badge),
        (middle, "resources/views/components/card.blade.php", card),
        (controller, "app/Caller.php", caller),
    ] {
        mgr.did_open(doc, &root.join(path), &LanguageId::new("php"), 1, text);
    }
    let outcome: Result<(), String> = async {
        members(&mgr, leaf, badge, &["userOnly"], &["otherOnly"]).await?;
        eprintln!("Control: explicit prop forwarding reaches User");
        let aware = badge.replace("@props(['person'])", "@aware(['person'])");
        let implicit = card.replace(" :person=\"$person\"", "");
        replace(&mgr, leaf, 2, badge, &aware);
        replace(&mgr, middle, 2, card, &implicit);
        members(&mgr, leaf, &aware, &["userOnly"], &["otherOnly"]).await?;
        eprintln!("Aware binding consumes the explicit parent data without child forwarding");
        let targets = mgr.goto(leaf, pos(&aware, "userOnly"), GotoKind::Definition).await.map_err(|e| e.to_string())?;
        if !targets
            .iter()
            .any(|target| target.path == root.join("app/User.php") && target.range.start == pos(user_source, "userOnly"))
        {
            return Err(format!("aware model must map to exact User property: {targets:?}"));
        }
        mgr.did_open(parent, &root.join("resources/views/page.blade.php"), &LanguageId::new("php"), 1, page);
        let slotted = "<x-card :person=\"$person\"><x-slot:person>Owned slot</x-slot></x-card>\n";
        replace(&mgr, parent, 2, page, slotted);
        type_at(&mgr, leaf, &aware, &["ComponentSlot"], &["User", "Other"]).await?;
        mgr.did_close(parent);
        members(&mgr, leaf, &aware, &["userOnly"], &["otherOnly"]).await?;
        eprintln!("Unsaved named-slot data overrides the attribute only at render; close restores User");
        let bag_page = DocId::next();
        mgr.did_open(bag_page, &root.join("resources/views/page.blade.php"), &LanguageId::new("php"), 1, page);
        let bag = r#"<x-card :attributes="new \Illuminate\View\ComponentAttributeBag(['person' => new \App\Other()])" />"#;
        replace(&mgr, bag_page, 2, page, bag);
        type_at(&mgr, leaf, &aware, &["mixed", "Other"], &["User"]).await?;
        eprintln!("Unknown expanded attribute-bag keys are not proven absent");
        let explicit = r#"<x-card :attributes="new \Illuminate\View\ComponentAttributeBag(['person' => new \App\Other()])" :person="$person" />"#;
        replace(&mgr, bag_page, 3, bag, explicit);
        members(&mgr, leaf, &aware, &["userOnly"], &["otherOnly"]).await?;
        let raw = r#"@php $attributes = new \Illuminate\View\ComponentAttributeBag(['person' => new \App\Other()]); @endphp
<x-card {{ $attributes }} />"#;
        replace(&mgr, bag_page, 4, explicit, raw);
        type_at(&mgr, leaf, &aware, &["mixed", "Other"], &["User"]).await?;
        mgr.did_close(bag_page);
        members(&mgr, leaf, &aware, &["userOnly"], &["otherOnly"]).await?;
        eprintln!("Explicit data wins over a bag; raw-bag close restores the disk model");
        let changed = caller.replace("new User()", "new Other()");
        replace(&mgr, controller, 2, caller, &changed);
        members(&mgr, leaf, &aware, &["otherOnly"], &["userOnly"]).await?;
        mgr.restart(mgr.servers()[0].id).await.map_err(|error| error.to_string())?;
        members(&mgr, leaf, &aware, &["otherOnly"], &["userOnly"]).await?;
        mgr.did_close(controller);
        members(&mgr, leaf, &aware, &["userOnly"], &["otherOnly"]).await?;
        Ok(())
    }
    .await;
    mgr.shutdown_all().await;
    for (name, text) in files {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text, "disk original: {name}");
    }
    assert!(outcome.is_ok(), "{}", outcome.unwrap_err());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_consumable_slot_map_accepts_numeric_keys_through_client() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let leaf = "@aware(['__laravel_slots'])\n<p>😀</p>{{ $__laravel_slots }}\n";
    let files = [
        ("composer.json", r#"{"require":{"laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#),
        (
            "app/MapCaller.php",
            "<?php namespace App; class MapCaller { public function show(): mixed { return view('page'); } }\n",
        ),
        ("resources/views/page.blade.php", "<x-parent><x-slot name=\"1\">Owned</x-slot></x-parent>\n"),
        ("resources/views/components/parent.blade.php", "@include('components.leaf')\n"),
        ("resources/views/components/leaf.blade.php", leaf),
    ];
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|server| server.name == "phpantom");
    php.language_servers[0].command = std::env::var("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable");
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let mut leaf_doc = None;
    for (name, text) in files.iter().skip(1) {
        let doc = DocId::next();
        mgr.did_open(doc, &root.join(name), &LanguageId::new("php"), 1, text);
        if *name == "resources/views/components/leaf.blade.php" {
            leaf_doc = Some(doc);
        }
    }
    let outcome = tokio::time::timeout(Duration::from_secs(8), async {
        let mut last = String::new();
        for _ in 0..80 {
            match mgr.hover(leaf_doc.unwrap(), pos(leaf, "$__laravel_slots"), Default::default()).await {
                Ok(reply) => {
                    last = reply.map(|reply| reply.contents.value).unwrap_or_default();
                    let normalized = last.chars().filter(|ch| !ch.is_whitespace()).collect::<String>();
                    if normalized.contains("array") && !normalized.contains("array<string,mixed>") {
                        return Ok(());
                    }
                }
                Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {}
                Err(error) => return Err(error.to_string()),
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        Err(format!("factory slot map must allow integer keys, not only string keys: {last}"))
    })
    .await
    .map_err(|_| "slot-map hover exceeded eight-second deadline".to_string())
    .and_then(|reply| reply);
    mgr.shutdown_all().await;
    for (name, text) in files {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text, "disk original: {name}");
    }
    assert!(outcome.is_ok(), "{}", outcome.unwrap_err());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn laravel_component_binding_correlation_ignores_frontend_bindings_and_blade_arguments() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let caller = "<?php namespace App; class Caller { public function show(): mixed { $person = new User(); return view('page', compact('person')); } }\n";
    let page = "<x-parent :person=\"$person\" />\n";
    let leaf = "@aware(['person'])\n<p>😀</p>{{ $person->userOnly }}\n";
    let files = [
        ("composer.json", r#"{"require":{"laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#),
        ("app/User.php", "<?php namespace App; class User { public string $userOnly; }\n"),
        ("app/Other.php", "<?php namespace App; class Other { public int $otherOnly; }\n"),
        ("app/Caller.php", caller),
        ("resources/views/page.blade.php", page),
        ("resources/views/components/parent.blade.php", "@props(['person'])\n<x-leaf />\n"),
        ("resources/views/components/leaf.blade.php", leaf),
    ];
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|server| server.name == "phpantom");
    php.language_servers[0].command = std::env::var("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable");
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let leaf_doc = DocId::next();
    let page_doc = DocId::next();
    let caller_doc = DocId::next();
    for (doc, name, text) in [
        (leaf_doc, "resources/views/components/leaf.blade.php", leaf),
        (DocId::next(), "resources/views/components/parent.blade.php", files[5].1),
        (page_doc, "resources/views/page.blade.php", page),
        (caller_doc, "app/Caller.php", caller),
    ] {
        mgr.did_open(doc, &root.join(name), &LanguageId::new("php"), 1, text);
    }
    let model = |yes: &'static str, no: &'static str| {
        let mgr = &mgr;
        async move {
            tokio::time::timeout(Duration::from_secs(8), async {
                let mut last = Vec::new();
                for _ in 0..80 {
                    match mgr.completion(leaf_doc, pos(leaf, "userOnly"), None, Default::default()).await {
                        Ok(reply) => {
                            last = reply.items.into_iter().map(|item| item.label).collect::<Vec<_>>();
                            if last.iter().any(|item| item == yes) && last.iter().all(|item| item != no) {
                                return Ok(());
                            }
                        }
                        Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {}
                        Err(error) => return Err(error.to_string()),
                    }
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                Err(format!("required {yes}, excluded {no}; got {last:?}"))
            })
            .await
            .map_err(|_| "binding correlation exceeded eight-second deadline".to_string())?
        }
    };
    let outcome: Result<(), String> = async {
        model("userOnly", "otherOnly").await?;
        let frontend = "<div :class=\"{ active: true }\"></div>\n<x-parent :person=\"$person\" />\n";
        let other = caller.replace("new User()", "new Other()");
        replace(&mgr, page_doc, 2, page, frontend);
        replace(&mgr, caller_doc, 2, caller, &other);
        model("otherOnly", "userOnly").await?;
        eprintln!("Frontend JavaScript binding does not consume a PHP marker index");
        let directive = "<x-parent @class(['active' => true]) :person=\"$person\" />\n";
        replace(&mgr, page_doc, 3, frontend, directive);
        replace(&mgr, caller_doc, 3, &other, caller);
        model("userOnly", "otherOnly").await?;
        eprintln!("Blade directive arguments do not truncate the following component attribute");
        Ok(())
    }
    .await;
    mgr.shutdown_all().await;
    for (name, text) in files {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text, "disk original: {name}");
    }
    assert!(outcome.is_ok(), "{}", outcome.unwrap_err());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn php_builtin_target_refreshes_after_composer_change_and_restart() {
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let composer = root.join("composer.json");
    std::fs::write(&composer, r#"{"require":{"php":"^8.2"}}"#).unwrap();
    let source = "<?php\nnew \\DateTime();\nnew \\DateTimeImmutable();\n";
    let path = root.join("main.php");
    std::fs::write(&path, source).unwrap();
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let mgr =
        LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_: IdeEvent| {}));
    let doc = DocId::next();
    mgr.did_open(doc, &path, &LanguageId::new("php"), 1, source);
    for _ in 0..120 {
        if mgr.servers().iter().any(|s| matches!(s.status, ServerStatus::Ready | ServerStatus::Busy)) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let locations = mgr.goto(doc, pos(source, "DateTime()"), GotoKind::Definition).await.unwrap();
    let uri = locations.iter().find_map(|l| l.uri.clone()).expect("DateTime builtin URI");
    let before = mgr.virtual_document(&uri).await.unwrap();
    assert!(before.description.as_deref().unwrap().contains("8.2"), "{:?}", before.description);
    std::fs::write(&composer, r#"{"require":{"php":"^8.2"},"config":{"platform":{"php":"8.3.0"}}}"#).unwrap();
    let server = mgr.servers()[0].id;
    mgr.restart(server).await.unwrap();
    // An uncached class independently witnesses that the new engine applied 8.3.
    let fresh = mgr.goto(doc, pos(source, "DateTimeImmutable()"), GotoKind::Definition).await.unwrap();
    let fresh_uri = fresh.iter().find_map(|l| l.uri.as_ref()).unwrap();
    let applied = mgr.virtual_document(fresh_uri).await.unwrap();
    assert!(applied.description.as_deref().unwrap().contains("8.3"), "{:?}", applied.description);
    let definitions = mgr.goto(doc, pos(source, "DateTime()"), GotoKind::Definition).await.unwrap();
    let current_uri = definitions.iter().find_map(|l| l.uri.as_ref()).unwrap();
    assert_eq!(current_uri, &uri, "exercise a reused builtin identity");
    let after = mgr.virtual_document(current_uri).await.unwrap();
    assert!(after.description.as_deref().unwrap().contains("8.3"), "stale builtin target: {:?}", after.description);
    mgr.shutdown_all().await;
}
