use super::*;
use futures::FutureExt;

const COMPOSER: &str =
    r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#;
const PROVIDER: &str = r#"<?php namespace App; class Provider extends \Illuminate\Support\ServiceProvider {public function boot(): void {$this->loadJsonTranslationsFrom(__DIR__.'/../packages/billing/json');}}"#;
const CALLER: &str =
    "<?php namespace App; class Caller {public function greet(): string { /* 😀 */ return __('Greeting 😀');}}";
const CATALOG: &str = r#"{"Greeting 😀":"Disk welcome"}"#;
const OVERLAY: &str = r#"{"Greeting 😀":"Private overlay"}"#;

async fn ready(path: &std::path::Path) -> serde_json::Value {
    for _ in 0..750 {
        if path.is_file() {
            return serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("real JSON session reply was not held: {}", path.display());
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom 188, owned PHP and Laravel 13.37 vendor fixture"]
async fn registered_json_real_sessions_retire_overlays_and_cross_action_replies_without_waiting_for_refs() {
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").expect("Laravel vendor"))
        .canonicalize()
        .unwrap();
    let installed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(vendor.join("composer/installed.json")).unwrap()).unwrap();
    assert!(
        installed["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|package| package["name"] == "laravel/framework" && package["version"] == "v13.37.0")
    );
    let engine = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").expect("pinned PHPantom"))
        .canonicalize()
        .unwrap();
    let php = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHP_BIN").expect("owned PHP"));
    for mutation in ["edit", "close", "provider", "hover", "round trip"] {
        println!("real JSON session case: {mutation}");
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let files = [
            ("composer.json", COMPOSER),
            ("app/Caller.php", CALLER),
            ("app/Provider.php", PROVIDER),
            ("bootstrap/providers.php", "<?php return [App\\Provider::class];"),
            ("packages/billing/composer.json", r#"{"name":"acme/billing"}"#),
            ("packages/billing/json/en.json", CATALOG),
            ("packages/billing/alternate/en.json", r#"{"Greeting 😀":"Alternate welcome"}"#),
        ];
        for (name, text) in files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, text).unwrap();
            if path.extension().is_some_and(|ext| ext == "php") {
                let lint = std::process::Command::new(&php).args(["-n", "-l"]).arg(&path).output().unwrap();
                assert!(lint.status.success(), "{name}: {}", String::from_utf8_lossy(&lint.stderr));
            } else {
                serde_json::from_str::<serde_json::Value>(text).unwrap();
            }
        }
        std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap();
        let control = root.join("control");
        std::fs::create_dir(&control).unwrap();
        let methods = if mutation == "round trip" {
            vec!["textDocument/hover"]
        } else if mutation == "hover" {
            vec!["textDocument/references", "textDocument/hover"]
        } else {
            vec!["textDocument/references"]
        };
        std::fs::write(control.join("methods.json"), serde_json::to_vec(&methods).unwrap()).unwrap();
        let proxy = root.join("json-session-replies.py");
        std::fs::write(&proxy, include_str!("phpantom-json-session-replies.py")).unwrap();
        let mut language = route_handler_language(&engine);
        let cfg = &mut language.language_servers[0];
        let args = std::mem::take(&mut cfg.args);
        cfg.command = std::env::var("OPENSTORM_TEST_PYTHON").unwrap_or_else(|_| "python3".into());
        cfg.args = vec![
            "-I".into(),
            proxy.to_string_lossy().into_owned(),
            engine.to_string_lossy().into_owned(),
            control.to_string_lossy().into_owned(),
        ];
        cfg.args.extend(args);
        let mgr =
            LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![language])), Arc::new(|_| {}));
        let candidates: Vec<_> =
            files.iter().filter(|(name, _)| name.ends_with(".php")).map(|(name, _)| root.join(name)).collect();
        mgr.set_candidates_hook(Box::new(move |_, _| ide_lsp::CandidateFiles {
            files: candidates.clone(),
            limit_hit: false,
        }));
        let outcome = std::panic::AssertUnwindSafe(async {
            let caller = DocId::next();
            let catalog = DocId::next();
            let provider = DocId::next();
            let catalog_path = root.join("packages/billing/json/en.json");
            mgr.did_open(caller, &root.join("app/Caller.php"), &LanguageId::new("php"), 1, CALLER);
            mgr.did_open(provider, &root.join("app/Provider.php"), &LanguageId::new("php"), 1, PROVIDER);
            mgr.did_open(catalog, &catalog_path, &LanguageId::new("json"), 1, OVERLAY);
            let at = pos(CALLER, "Greeting 😀");
            let hover_ready = control.join("hover.json");
            if mutation == "round trip" {
                let client = mgr.clone();
                let mut hover = tokio::spawn(async move { client.hover(caller, at, Default::default()).await });
                let captured = tokio::select! {
                    captured = ready(&hover_ready) => captured,
                    result = &mut hover => panic!("{mutation}: PHP hover returned before its reply gate: {result:?}"),
                };
                assert!(captured["result"]["contents"].to_string().contains("Private overlay"), "{captured}");
                let refs = mgr
                    .references(catalog, pos(OVERLAY, "Greeting 😀"), false, true, Default::default())
                    .await
                    .unwrap();
                assert_eq!(refs.locations.len(), 1);
                std::fs::write(control.join("hover.release"), "release").unwrap();
                assert!(hover.await.unwrap().is_err(), "PHP reply spanning the catalog session must expire");
            } else {
                let client = mgr.clone();
                let refs = tokio::spawn(async move {
                    client.references(catalog, pos(OVERLAY, "Greeting 😀"), false, true, Default::default()).await
                });
                let captured = ready(&control.join("references.json")).await;
                assert_eq!(captured["result"].as_array().unwrap().len(), 1, "{captured}");
                let pending_hover = if mutation == "hover" {
                    let client = mgr.clone();
                    let mut hover = tokio::spawn(async move { client.hover(caller, at, Default::default()).await });
                    let captured = tokio::select! {
                        captured = ready(&hover_ready) => captured,
                        result = &mut hover => panic!("{mutation}: PHP hover returned before its reply gate: {result:?}"),
                    };
                    assert!(captured["result"]["contents"].to_string().contains("Private overlay"), "{captured}");
                    Some(hover)
                } else {
                    let hover = mgr.hover(caller, at, Default::default()).await.unwrap().unwrap();
                    assert!(hover.contents.value.contains("Private overlay"), "{hover:?}");
                    None
                };
                if mutation == "edit" {
                    let plan = mgr
                        .rename(caller, pos(CALLER, "Caller"), "RenamedCaller".into(), Default::default())
                        .await
                        .unwrap();
                    assert!(
                        !plan.edit.files.is_empty(),
                        "checked rename remains available while a JSON resource is active"
                    );
                }
                match mutation {
                    "close" => mgr.did_close(catalog),
                    "provider" => {
                        replace(&mgr, provider, 2, PROVIDER, &PROVIDER.replace("billing/json", "billing/alternate"))
                    }
                    _ => replace(&mgr, catalog, 2, OVERLAY, r#"{"Greeting 😀":"Current overlay"}"#),
                }
                tokio::time::timeout(Duration::from_secs(2), async {
                    while !refs.is_finished() {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .expect("retire references before releasing the real held reply");
                assert!(refs.await.unwrap().is_err());
                let fresh = mgr.hover(caller, at, Default::default()).await.unwrap().unwrap();
                let expected = match mutation { "provider" => "Alternate welcome", "close" => "Disk welcome", _ => "Current overlay" };
                assert!(fresh.contents.value.contains(expected), "{mutation}: {fresh:?}");
                assert!(!fresh.contents.value.contains("Private overlay"));
                if let Some(hover) = pending_hover {
                    std::fs::write(control.join("hover.release"), "release").unwrap();
                    assert!(hover.await.unwrap().is_err(), "held PHP reply must expire after catalog retirement");
                }
                std::fs::write(control.join("references.release"), "release").unwrap();
            }
            let events = std::fs::read_to_string(control.join("events.jsonl")).unwrap();
            let events: Vec<serde_json::Value> =
                events.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
            let uri = ide_lsp::path_to_uri_string(&catalog_path);
            for method in ["textDocument/didOpen", "textDocument/didClose"] {
                assert!(
                    events.iter().any(|event| event["method"] == method && event["uri"] == uri),
                    "{mutation}: resource lifecycle must reach the actual engine"
                );
            }
            for (name, text) in files {
                assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text);
            }
        })
        .catch_unwind()
        .await;
        for name in ["hover.release", "references.release"] {
            std::fs::write(control.join(name), "release").unwrap();
        }
        mgr.shutdown_all().await;
        assert!(mgr.servers().iter().all(|server| server.pid.is_none()));
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom 188, owned PHP and Laravel 13.37 vendor fixture"]
async fn ordinary_php_navigation_automatically_reuses_unsaved_custom_catalogs_and_recovers() {
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").expect("Laravel vendor"))
        .canonicalize()
        .unwrap();
    let installed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(vendor.join("composer/installed.json")).unwrap()).unwrap();
    assert!(
        installed["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|package| package["name"] == "laravel/framework" && package["version"] == "v13.37.0")
    );
    let engine = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").expect("pinned PHPantom"))
        .canonicalize()
        .unwrap();
    let php = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHP_BIN").expect("owned PHP"));
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let files = [
        ("composer.json", COMPOSER),
        ("app/Caller.php", CALLER),
        ("app/Provider.php", PROVIDER),
        ("bootstrap/providers.php", "<?php return [App\\Provider::class];"),
        ("packages/billing/composer.json", r#"{"name":"acme/billing"}"#),
        ("packages/billing/json/en.json", CATALOG),
        ("packages/billing/alternate/en.json", r#"{"Greeting 😀":"Alternate welcome"}"#),
    ];
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        if name.ends_with(".php") {
            let check = std::process::Command::new(&php).args(["-n", "-l"]).arg(&path).output().unwrap();
            assert!(check.status.success(), "{name}: {}", String::from_utf8_lossy(&check.stderr));
        } else {
            serde_json::from_str::<serde_json::Value>(text).unwrap();
        }
    }
    std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap();
    let control = root.join("control");
    std::fs::create_dir(&control).unwrap();
    std::fs::write(control.join("methods.json"), "[]").unwrap();
    let proxy = root.join("json-session-replies.py");
    std::fs::write(&proxy, include_str!("phpantom-json-session-replies.py")).unwrap();
    let mut language = route_handler_language(&engine);
    let cfg = &mut language.language_servers[0];
    let args = std::mem::take(&mut cfg.args);
    cfg.command = std::env::var("OPENSTORM_TEST_PYTHON").unwrap_or_else(|_| "python3".into());
    cfg.args = vec![
        "-I".into(),
        proxy.to_string_lossy().into_owned(),
        engine.to_string_lossy().into_owned(),
        control.to_string_lossy().into_owned(),
    ];
    cfg.args.extend(args);
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![language])), Arc::new(|_| {}));
    let outcome = std::panic::AssertUnwindSafe(async {
        let json = DocId::next();
        let caller = DocId::next();
        let provider = DocId::next();
        let catalog = root.join("packages/billing/json/en.json");
        let alternate = root.join("packages/billing/alternate/en.json");
        mgr.did_open(json, &catalog, &LanguageId::new("json"), 1, OVERLAY);
        assert!(mgr.servers().is_empty(), "catalog opening alone does not launch a PHP server");
        mgr.did_close(json);
        mgr.did_open(caller, &root.join("app/Caller.php"), &LanguageId::new("php"), 1, CALLER);
        mgr.did_open(provider, &root.join("app/Provider.php"), &LanguageId::new("php"), 1, PROVIDER);
        tokio::time::timeout(Duration::from_secs(10), async {
            while !mgr.servers().iter().any(|server| {
                server.name == "phpantom" && matches!(server.status, ServerStatus::Ready | ServerStatus::Busy)
            }) {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("PHP consumer ready before catalog opening");
        let count = |method: &str| {
            std::fs::read_to_string(control.join("events.jsonl"))
                .unwrap_or_default()
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .filter(|event| event["method"] == method && event["uri"] == ide_lsp::path_to_uri_string(&catalog))
                .count()
        };
        let json = DocId::next();
        mgr.did_open(json, &catalog, &LanguageId::new("json"), 1, OVERLAY);
        tokio::time::timeout(Duration::from_secs(5), async {
            while count("textDocument/didOpen") != 1 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("new catalog must be admitted before the first PHP request");
        for _ in 0..2 {
            let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
            assert!(hover.contents.value.contains("Private overlay"), "{hover:?}");
            let targets = mgr.goto(caller, pos(CALLER, "Greeting 😀"), GotoKind::Definition).await.unwrap();
            assert_eq!(targets.len(), 1);
            assert_eq!(targets[0].path.to_path_buf(), catalog);
            assert_eq!(targets[0].range.start, pos(OVERLAY, "Greeting 😀"));
        }
        assert_eq!(count("textDocument/didOpen"), 1, "unchanged PHP requests reuse one admitted overlay");
        assert_eq!(count("textDocument/didClose"), 0, "retain the catalog after ordinary PHP navigation");
        let updated = r#"{"Greeting 😀":"Updated private value"}"#;
        replace(&mgr, json, 2, OVERLAY, updated);
        tokio::time::timeout(Duration::from_secs(5), async {
            while count("textDocument/didOpen") != 2 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("edited catalog must refresh before another PHP request");
        let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("Updated private value"));
        assert!(!hover.contents.value.contains("Private overlay"));
        let draft = r#"{"Draft 😀":"Private draft"}"#;
        let draft_caller = CALLER.replace("Greeting 😀", "Draft 😀");
        replace(&mgr, json, 3, updated, draft);
        replace(&mgr, caller, 2, CALLER, &draft_caller);
        let hover = mgr.hover(caller, pos(&draft_caller, "Draft 😀"), Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("Private draft"));
        let targets = mgr.goto(caller, pos(&draft_caller, "Draft 😀"), GotoKind::Definition).await.unwrap();
        assert_eq!(targets[0].path.to_path_buf(), catalog);
        assert_eq!(targets[0].range.start, pos(draft, "Draft 😀"));
        replace(&mgr, json, 4, draft, updated);
        replace(&mgr, caller, 3, &draft_caller, CALLER);
        let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("Updated private value"));
        let server = mgr.servers().into_iter().find(|server| server.name == "phpantom").unwrap().id;
        mgr.restart(server).await.unwrap();
        let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
        assert!(
            hover.contents.value.contains("Updated private value"),
            "restart must re-admit before ordinary PHP hover: {hover:?}"
        );
        let changed = PROVIDER.replace("billing/json", "billing/alternate");
        replace(&mgr, provider, 2, PROVIDER, &changed);
        let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("Alternate welcome"));
        assert!(!hover.contents.value.contains("Updated private value"));
        let alternate_doc = DocId::next();
        let alternate_overlay = r#"{"Greeting 😀":"Private alternate"}"#;
        mgr.did_open(alternate_doc, &alternate, &LanguageId::new("json"), 1, alternate_overlay);
        let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("Private alternate"));
        mgr.did_close(alternate_doc);
        let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("Alternate welcome"));
        mgr.did_close(provider);
        let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("Updated private value"));
        let refs = mgr.references(json, pos(updated, "Greeting 😀"), false, true, Default::default()).await.unwrap();
        assert_eq!(refs.locations.len(), 1, "Find Usages works after a persistent PHP overlay");
        assert_eq!(refs.locations[0].path.to_path_buf(), root.join("app/Caller.php"));
        mgr.did_close(json);
        let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("Disk welcome"));
        mgr.did_open(json, &catalog, &LanguageId::new("json"), 5, OVERLAY);
        let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("Private overlay"));
        for (name, text) in files {
            assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text);
        }
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    assert!(mgr.servers().iter().all(|server| server.pid.is_none()));
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
