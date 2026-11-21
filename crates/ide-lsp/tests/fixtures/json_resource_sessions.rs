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
        ("app/SecondCaller.php", "<?php namespace App; class SecondCaller {}"),
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
        .expect("PHP consumer becomes ready with the catalog already open");
        let count = |method: &str| {
            std::fs::read_to_string(control.join("events.jsonl"))
                .unwrap_or_default()
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .filter(|event| event["method"] == method && event["uri"] == ide_lsp::path_to_uri_string(&catalog))
                .count()
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            while count("textDocument/didOpen") != 1 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("already-open catalog must be admitted on PHP readiness before the first PHP request");
        let second = DocId::next();
        mgr.did_open(
            second,
            &root.join("app/SecondCaller.php"),
            &LanguageId::new("php"),
            1,
            "<?php namespace App; class SecondCaller {}",
        );
        tokio::time::timeout(Duration::from_secs(5), async {
            while count("textDocument/didOpen") != 2 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("another PHP buffer must re-admit the catalog before the first PHP request");
        replace(
            &mgr,
            second,
            2,
            "<?php namespace App; class SecondCaller {}",
            "<?php namespace App; class SecondCaller { /* changed */ }",
        );
        tokio::time::timeout(Duration::from_secs(5), async {
            while count("textDocument/didOpen") != 3 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("PHP edit must re-admit the catalog before navigation");
        mgr.did_close(second);
        tokio::time::timeout(Duration::from_secs(5), async {
            while count("textDocument/didOpen") != 4 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("PHP close must re-admit the catalog for remaining consumers before navigation");
        let pid_before_reconfiguration =
            mgr.servers().into_iter().find(|server| server.name == "phpantom").unwrap().pid;
        assert!(pid_before_reconfiguration.is_some());
        for change in ["settings", "unchanged settings", "content roots"] {
            let before = count("textDocument/didOpen");
            match change {
                "settings" => {
                    mgr.set_config_hook(Box::new(|cfg, _| {
                        cfg.settings = Some(serde_json::json!({"openstormTest":{"revision":1}}))
                    }));
                    mgr.reconfigure().await;
                }
                "unchanged settings" => mgr.reconfigure().await,
                "content roots" => {
                    let additional = root.join("additional");
                    std::fs::create_dir(&additional).unwrap();
                    mgr.set_content_roots(vec![additional]);
                }
                _ => unreachable!(),
            }
            tokio::time::timeout(Duration::from_secs(5), async {
                while count("textDocument/didOpen") <= before {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .unwrap_or_else(|_| panic!("{change} must re-admit the catalog before PHP navigation"));
            assert_eq!(
                mgr.servers().into_iter().find(|server| server.name == "phpantom").unwrap().pid,
                pid_before_reconfiguration
            );
        }
        let manifest = root.join("composer.json");
        let composer = DocId::next();
        let mut saved_manifest: serde_json::Value = serde_json::from_str(COMPOSER).unwrap();
        saved_manifest["extra"] = serde_json::json!({"openstormTest":true});
        let saved_manifest = serde_json::to_string(&saved_manifest).unwrap();
        for action in ["clean open", "discard", "restore", "save", "restore saved", "close"] {
            let before = count("textDocument/didOpen");
            match action {
                "clean open" => mgr.did_open(composer, &manifest, &LanguageId::new("json"), 1, COMPOSER),
                "discard" => {
                    replace(&mgr, composer, 2, COMPOSER, "{}");
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    assert_eq!(count("textDocument/didOpen"), before, "dirty Composer cannot authorize catalog text");
                    assert_eq!(count("textDocument/didClose"), before);
                    mgr.did_close(composer);
                }
                "restore" => {
                    mgr.did_open(composer, &manifest, &LanguageId::new("json"), 3, "{}");
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    assert_eq!(count("textDocument/didOpen"), before);
                    assert_eq!(count("textDocument/didClose"), before);
                    replace(&mgr, composer, 4, "{}", COMPOSER);
                }
                "save" => {
                    replace(&mgr, composer, 5, COMPOSER, &saved_manifest);
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    assert_eq!(count("textDocument/didOpen"), before);
                    assert_eq!(count("textDocument/didClose"), before);
                    std::fs::write(&manifest, &saved_manifest).unwrap();
                    mgr.did_save(composer, &saved_manifest);
                    mgr.php_files_changed(&[], std::slice::from_ref(&manifest), &[]);
                }
                "restore saved" => {
                    replace(&mgr, composer, 6, &saved_manifest, COMPOSER);
                    std::fs::write(&manifest, COMPOSER).unwrap();
                    mgr.did_save(composer, COMPOSER);
                    mgr.php_files_changed(&[], std::slice::from_ref(&manifest), &[]);
                }
                "close" => mgr.did_close(composer),
                _ => unreachable!(),
            }
            tokio::time::timeout(Duration::from_secs(5), async {
                while count("textDocument/didOpen") <= before {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .unwrap_or_else(|_| panic!("Composer {action} must re-admit the catalog before PHP navigation"));
            assert_eq!(
                mgr.servers().into_iter().find(|server| server.name == "phpantom").unwrap().pid,
                pid_before_reconfiguration
            );
        }
        assert_eq!(std::fs::read_to_string(&manifest).unwrap(), COMPOSER);
        let opens_before_navigation = count("textDocument/didOpen");
        let closes_before_navigation = count("textDocument/didClose");
        for _ in 0..2 {
            let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
            assert!(hover.contents.value.contains("Private overlay"), "{hover:?}");
            let targets = mgr.goto(caller, pos(CALLER, "Greeting 😀"), GotoKind::Definition).await.unwrap();
            assert_eq!(targets.len(), 1);
            assert_eq!(targets[0].path.to_path_buf(), catalog);
            assert_eq!(targets[0].range.start, pos(OVERLAY, "Greeting 😀"));
        }
        assert_eq!(
            count("textDocument/didOpen"),
            opens_before_navigation,
            "unchanged PHP requests reuse the refreshed overlay"
        );
        assert_eq!(
            count("textDocument/didClose"),
            closes_before_navigation,
            "unchanged PHP requests retain the refreshed overlay"
        );
        let bootstrap = root.join("bootstrap/providers.php");
        let original_bootstrap = files.iter().find(|(name, _)| *name == "bootstrap/providers.php").unwrap().1;
        let watched = || {
            std::fs::read_to_string(control.join("events.jsonl"))
                .unwrap_or_default()
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .filter(|event| {
                    event["method"] == "workspace/didChangeWatchedFiles"
                        && event["uri"] == ide_lsp::path_to_uri_string(&bootstrap)
                })
                .count()
        };
        for (registered, text) in [(false, "<?php return [];"), (true, original_bootstrap)] {
            let before = count("textDocument/didOpen");
            let watch_before = watched();
            std::fs::write(&bootstrap, text).unwrap();
            let lint = std::process::Command::new(&php).args(["-n", "-l"]).arg(&bootstrap).output().unwrap();
            assert!(lint.status.success(), "{}", String::from_utf8_lossy(&lint.stderr));
            mgr.php_files_changed(&[], std::slice::from_ref(&bootstrap), &[]);
            tokio::time::timeout(Duration::from_secs(5), async {
                while watched() <= watch_before
                    || if registered {
                        count("textDocument/didOpen") <= before
                    } else {
                        count("textDocument/didClose") != count("textDocument/didOpen")
                    }
                {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .expect("closed provider-list changes reconcile private catalogs before PHP navigation");
            let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap();
            if registered {
                assert!(hover.unwrap().contents.value.contains("Private overlay"));
            } else if let Some(hover) = hover {
                assert!(!hover.contents.value.contains("Private overlay"), "an unregistered catalog stays private");
            }
            assert_eq!(
                mgr.servers().into_iter().find(|server| server.name == "phpantom").unwrap().pid,
                pid_before_reconfiguration
            );
        }
        assert_eq!(std::fs::read_to_string(&bootstrap).unwrap(), original_bootstrap);
        let opens_before_edit = count("textDocument/didOpen");
        let updated = r#"{"Greeting 😀":"Updated private value"}"#;
        replace(&mgr, json, 2, OVERLAY, updated);
        tokio::time::timeout(Duration::from_secs(5), async {
            while count("textDocument/didOpen") != opens_before_edit + 1 {
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
        let opens_before_restart = count("textDocument/didOpen");
        mgr.restart(server).await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while count("textDocument/didOpen") <= opens_before_restart {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("restart must re-admit the open catalog before another PHP request");
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
        let opens_before_provider_close = count("textDocument/didOpen");
        mgr.did_close(provider);
        tokio::time::timeout(Duration::from_secs(5), async {
            while count("textDocument/didOpen") <= opens_before_provider_close {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("closing an unsaved provider must restore its disk registration before PHP navigation");
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

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom 189, owned PHP and Laravel 13.37 vendor fixture"]
async fn shared_json_find_usages_restores_both_consuming_servers_without_php_navigation() {
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
    let provider = PROVIDER.replace("../packages/billing/json", "../../shared/catalogs");
    for project in ["a", "b"] {
        for (name, text) in [
            ("composer.json", COMPOSER),
            ("app/Caller.php", CALLER),
            ("app/Provider.php", provider.as_str()),
            ("bootstrap/providers.php", "<?php return [App\\Provider::class];"),
        ] {
            let path = root.join(project).join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, text).unwrap();
            if name.ends_with(".php") {
                let lint = std::process::Command::new(&php).args(["-n", "-l"]).arg(&path).output().unwrap();
                assert!(lint.status.success(), "{project}/{name}: {}", String::from_utf8_lossy(&lint.stderr));
            } else {
                serde_json::from_str::<serde_json::Value>(text).unwrap();
            }
        }
        std::os::unix::fs::symlink(&vendor, root.join(project).join("vendor")).unwrap();
        std::fs::create_dir(root.join(project).join("control")).unwrap();
        std::fs::write(root.join(project).join("control/methods.json"), "[]").unwrap();
    }
    let path = root.join("shared/catalogs/en.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, CATALOG).unwrap();
    serde_json::from_str::<serde_json::Value>(CATALOG).unwrap();
    serde_json::from_str::<serde_json::Value>(OVERLAY).unwrap();
    for project in ["a", "b"] {
        let runtime = std::process::Command::new(&php)
            .args(["-n", "-d", "zend.assertions=1", "-d", "assert.exception=1", "-r", r#"require $argv[1].'/vendor/autoload.php'; $loader=new Illuminate\Translation\FileLoader(new Illuminate\Filesystem\Filesystem,$argv[1].'/lang'); $loader->addJsonPath($argv[1].'/../shared/catalogs'); $translator=new Illuminate\Translation\Translator($loader,'en'); assert($translator->get('Greeting 😀')==='Disk welcome');"#])
            .arg(root.join(project))
            .output()
            .unwrap();
        assert!(runtime.status.success(), "{project}: {}", String::from_utf8_lossy(&runtime.stderr));
    }
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
        "control".into(),
    ];
    cfg.args.extend(args);
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![language])), Arc::new(|_| {}));
    mgr.set_content_roots(vec![root.join("a"), root.join("b")]);
    let frames = |project: &str| {
        std::fs::read_to_string(root.join(project).join("control/events.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|event| event["uri"] == ide_lsp::path_to_uri_string(&path))
            .collect::<Vec<_>>()
    };
    let opens =
        |project: &str| frames(project).iter().filter(|event| event["method"] == "textDocument/didOpen").count();
    let outcome = std::panic::AssertUnwindSafe(async {
        let catalog = DocId::next();
        mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, OVERLAY);
        assert!(mgr.servers().is_empty());
        let callers = [DocId::next(), DocId::next()];
        for (project, caller) in ["a", "b"].into_iter().zip(callers) {
            mgr.did_open(caller, &root.join(project).join("app/Caller.php"), &LanguageId::new("php"), 1, CALLER);
        }
        tokio::time::timeout(Duration::from_secs(15), async {
            while opens("a") == 0 || opens("b") == 0 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("both consuming servers admit the shared private catalog before PHP navigation");
        let servers = mgr.servers();
        assert_eq!(servers.len(), 2);
        assert!(servers.iter().all(|server| server.pid.is_some()));
        assert_ne!(servers[0].pid, servers[1].pid);
        let identities: Vec<_> = servers.into_iter().map(|server| (server.id, server.root, server.pid)).collect();
        for caller in callers {
            let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
            assert!(hover.contents.value.contains("Private overlay"));
        }
        let before = [opens("a"), opens("b")];
        for _ in 0..2 {
            let refs =
                mgr.references(catalog, pos(OVERLAY, "Greeting 😀"), false, true, Default::default()).await.unwrap();
            assert!(refs.incomplete.is_none(), "{refs:?}");
            let mut found: Vec<_> = refs.locations.iter().map(|location| location.path.to_path_buf()).collect();
            found.sort();
            assert_eq!(found, [root.join("a/app/Caller.php"), root.join("b/app/Caller.php")]);
            assert!(refs.locations.iter().all(|location| location.range.start == pos(CALLER, "Greeting 😀")));
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if ["a", "b"].into_iter().enumerate().all(|(i, project)| {
                    opens(project) >= before[i] + 3
                        && frames(project)
                            .last()
                            .is_some_and(|event| event["method"] == "textDocument/didOpen" && event["version"] == 1)
                }) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("Find Usages restores both persistent catalogs before another PHP command");
        let restored = [opens("a"), opens("b")];
        for caller in callers {
            let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
            assert!(hover.contents.value.contains("Private overlay"));
        }
        assert_eq!([opens("a"), opens("b")], restored, "PHP navigation reuses both restored catalogs");
        let updated = r#"{"Greeting 😀":"Updated shared private value"}"#;
        replace(&mgr, catalog, 2, OVERLAY, updated);
        tokio::time::timeout(Duration::from_secs(5), async {
            while !["a", "b"].into_iter().all(|project| {
                frames(project)
                    .last()
                    .is_some_and(|event| event["method"] == "textDocument/didOpen" && event["version"] == 2)
            }) {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("subsequent JSON edits refresh both consuming servers");
        for caller in callers {
            let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
            assert!(hover.contents.value.contains("Updated shared private value"));
        }
        for (id, root, pid) in identities {
            let current = mgr.servers().into_iter().find(|server| server.id == id).unwrap();
            assert_eq!((current.root, current.pid), (root, pid));
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), CATALOG);
        mgr.did_close(catalog);
        tokio::time::timeout(Duration::from_secs(5), async {
            while !["a", "b"]
                .into_iter()
                .all(|project| frames(project).last().is_some_and(|event| event["method"] == "textDocument/didClose"))
            {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("closing the catalog retires both consuming overlays");
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    if let Err(error) = outcome {
        std::panic::resume_unwind(error);
    }
}
