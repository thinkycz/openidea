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
    let checked = mgr.rename(doc, pos(main, "openstorm_scope_probe"), "openstorm_scope_changed".into()).await.unwrap();
    assert_eq!(checked.files.len(), 2, "checked PHP rename still works with a shared JSON overlay");
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
