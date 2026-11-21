use super::json_translation_resources::{caps, echo};
use super::*;

const METHOD: &str = "phpWorkbench/jsonTranslationResource";
const MANIFEST: &str = r#"{"require":{"laravel/framework":"13.37.0"}}"#;
const TEXT: &str = r#"{"Greeting 😀":"Private draft"}"#;

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_find_usages_restores_retained_catalogs_without_php_navigation() {
    for action in [
        "success",
        "repeat",
        "pre-cancel",
        "error",
        "cancel",
        "edit",
        "close",
        "disable",
        "restart",
        "composer",
        "foreground",
    ] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let state = h.launcher.state("phpantom");
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        state.responder.lock().replace(Arc::new(move |state, method, params| match method {
            METHOD => Some(echo(params, resource.parent())),
            "textDocument/references" => { held.hold(); Some(json!([])) },
            "textDocument/hover" => Some(json!({"contents":{"kind":"markdown","value":state.docs.lock().get(&uri_of(&resource)).cloned().unwrap_or_else(||"disk".into())}})),
            _ => None,
        }));
        if action == "error" {
            let held = gate.clone();
            state.fault.lock().replace(Arc::new(move |_, method| {
                (method == "textDocument/references").then(|| {
                    held.hold();
                    (-32001, "owned test failure".into())
                })
            }));
        }
        let (caller, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let catalog = DocId::next();
        h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        let opens = || {
            state
                .notified("textDocument/didOpen")
                .iter()
                .filter(|event| event["textDocument"]["uri"] == uri_of(&path))
                .count()
        };
        let before = opens();
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let manager = h.mgr.clone();
        let refs = tokio::spawn(async move { manager.references(catalog, Pos::new(0, 2), false, true, token).await });
        eventually(|| gate.entered()).await;
        let updated = r#"{"Greeting 😀":"Latest private buffer"}"#;
        let manifest_doc = DocId::next();
        match action {
            "cancel" => cancel.cancel(),
            "edit" => h.mgr.did_change(
                catalog,
                2,
                &[AppliedChange { range: rng(0, 0, 0, TEXT.encode_utf16().count() as u32), text: updated.into() }],
                &|| updated.into(),
            ),
            "close" => h.mgr.did_close(catalog),
            "disable" => h.mgr.set_execution_allowed(false),
            "restart" => {
                h.mgr.restart(h.mgr.servers().iter().find(|s| s.name == "phpantom").unwrap().id).await.unwrap();
            }
            "composer" => h.mgr.did_open(
                manifest_doc,
                &h.root.join("composer.json"),
                &LanguageId::new("json"),
                1,
                r#"{"require":{"laravel/framework":"13.37.0"},"extra":{"dirty":true}}"#,
            ),
            "foreground" => {
                let hover = tokio::time::timeout(
                    Duration::from_secs(2),
                    h.mgr.hover(caller, Pos::new(0, 10), Default::default()),
                )
                .await
                .expect("foreground hover must not wait for held references")
                .unwrap()
                .unwrap();
                assert_eq!(hover.contents.value, TEXT);
            }
            _ => {}
        }
        if !matches!(action, "success" | "repeat" | "pre-cancel" | "error" | "foreground") {
            tokio::time::timeout(Duration::from_secs(2), async {
                while !refs.is_finished() {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("retire references before releasing the held reply");
        }
        gate.open();
        let result = refs.await.unwrap();
        if matches!(action, "success" | "repeat" | "pre-cancel" | "foreground") {
            assert!(result.unwrap().locations.is_empty());
        } else {
            assert!(result.is_err(), "{action}");
        }
        if action == "repeat" {
            assert!(
                h.mgr
                    .references(catalog, Pos::new(0, 2), false, true, Default::default())
                    .await
                    .unwrap()
                    .locations
                    .is_empty()
            );
        } else if action == "pre-cancel" {
            let cancelled = CancellationToken::new();
            cancelled.cancel();
            assert!(h.mgr.references(catalog, Pos::new(0, 2), false, true, cancelled).await.is_err());
        }
        let expected_opens = before + if action == "repeat" { 3 } else { 2 };
        if action == "composer" {
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert!(!state.docs.lock().contains_key(&uri_of(&path)), "dirty Composer prevents readmission");
            h.mgr.did_close(manifest_doc);
        }
        if action == "close" || action == "disable" {
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert_eq!(opens(), before + 1, "{action}: do not resurrect a closed or disabled catalog");
        } else {
            tokio::time::timeout(Duration::from_secs(5), async {
                while opens() < expected_opens
                    || state.docs.lock().get(&uri_of(&path)).map(String::as_str)
                        != Some(if action == "edit" { updated } else { TEXT })
                {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap_or_else(|_| panic!("{action}: ordinary catalog discovery must resume after Find Usages"));
            tokio::time::sleep(Duration::from_millis(150)).await;
            assert_eq!(opens(), expected_opens, "{action}: one current restored overlay");
        }
        assert_eq!(state.requested("textDocument/hover").len(), usize::from(action == "foreground"));
        assert_eq!(state.launches.load(Ordering::SeqCst), if action == "restart" { 2 } else { 1 });
        assert!(!path.exists());
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_external_changes_recheck_every_consuming_context_before_readmission() {
    let mut capabilities = caps();
    capabilities["experimental"]["phpWorkbenchRenameChecked"] = json!(true);
    let h = harness_for("php", "php", &[("phpantom", capabilities)], &[]);
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let nested = h.root.join("consumers/other");
    std::fs::create_dir_all(&nested).unwrap();
    let second = nested.join("composer.json");
    std::fs::write(&second, MANIFEST).unwrap();
    let second_uri = uri_of(&second);
    let dependency = nested.join("Provider.php");
    std::fs::write(&dependency, "<?php /* not registered */").unwrap();
    let path = h.root.join("catalogs/en.json");
    let resource = path.clone();
    let ambiguous = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let positive = ambiguous.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move |_, method, params| {
        (method == METHOD).then(|| {
            echo(
                params,
                (params["projectUri"] != second_uri || positive.load(Ordering::SeqCst))
                    .then(|| resource.parent().unwrap()),
            )
        })
    }));
    let (caller, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
    h.open("consumers/other/Caller.php", "<?php __('Greeting 😀');");
    h.wait_status("phpantom", ServerStatus::Ready).await;
    h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
    eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
    h.mgr.hover(caller, Pos::new(0, 10), Default::default()).await.unwrap();
    let before = state.requested(METHOD).len();
    ambiguous.store(true, Ordering::SeqCst);
    std::fs::write(&dependency, "<?php /* now also registered */").unwrap();
    h.mgr.php_files_changed(&[], std::slice::from_ref(&dependency), &[]);
    eventually(|| state.requested(METHOD).len() >= before + 2).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!state.docs.lock().contains_key(&uri_of(&path)), "a newly positive second context is ambiguous");
    ambiguous.store(false, Ordering::SeqCst);
    std::fs::write(&dependency, "<?php /* no longer registered */").unwrap();
    h.mgr.php_files_changed(&[], std::slice::from_ref(&dependency), &[]);
    eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
    assert_eq!(state.requested("textDocument/hover").len(), 1);
    assert_eq!(state.launches.load(Ordering::SeqCst), 1);
    assert!(!path.exists());
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_external_discovery_keeps_foreground_priority_and_lifecycle_boundaries() {
    let isolated = harness_for("php", "php", &[("phpantom", caps())], &[]);
    isolated.mgr.did_open(DocId::next(), &isolated.root.join("catalogs/en.json"), &LanguageId::new("json"), 1, TEXT);
    isolated.mgr.php_files_changed(&[isolated.root.join("Provider.php")], &[], &[]);
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(isolated.mgr.servers().is_empty(), "watcher changes cannot boot PHP from JSON alone");
    isolated.mgr.shutdown_all().await;
    for (name, advertised) in [("phpantom", false), ("phpactor", true), ("intelephense", true)] {
        let mut capabilities = if advertised { caps() } else { incremental_caps() };
        capabilities["experimental"]["phpWorkbenchRenameChecked"] = json!(true);
        let h = harness_for("php", "php", &[(name, capabilities)], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status(name, ServerStatus::Ready).await;
        h.mgr.did_open(DocId::next(), &h.root.join("catalogs/en.json"), &LanguageId::new("json"), 1, TEXT);
        h.mgr.php_files_changed(&[h.root.join("Provider.php")], &[], &[]);
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(h.launcher.state(name).requested(METHOD).is_empty());
        assert_eq!(h.launcher.state(name).launches.load(Ordering::SeqCst), 1);
        h.mgr.shutdown_all().await;
    }
    for action in ["foreground", "edit", "close", "disable", "restart"] {
        let mut capabilities = caps();
        capabilities["experimental"]["phpWorkbenchRenameChecked"] = json!(true);
        let h = harness_for("php", "php", &[("phpantom", capabilities)], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let dependency = h.root.join("Provider.php");
        std::fs::write(&dependency, "<?php /* before */").unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let registered = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let active = registered.clone();
        let hold_next = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let hold = hold_next.clone();
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move |state, method, params| match method {
            METHOD => {
                if hold.swap(false, Ordering::SeqCst) { held.hold(); }
                Some(echo(params, active.load(Ordering::SeqCst).then(|| resource.parent().unwrap())))
            }
            "textDocument/hover" => Some(json!({"contents":{"kind":"markdown","value":state.docs.lock().get(&uri_of(&resource)).cloned().unwrap_or_else(||"disk".into())}})),
            _ => None,
        }));
        let (caller, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let catalog = DocId::next();
        h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| !state.requested(METHOD).is_empty()).await;
        h.mgr.hover(caller, Pos::new(0, 10), Default::default()).await.unwrap();
        registered.store(true, Ordering::SeqCst);
        hold_next.store(true, Ordering::SeqCst);
        std::fs::write(&dependency, "<?php /* after */").unwrap();
        h.mgr.php_files_changed(&[], std::slice::from_ref(&dependency), &[]);
        eventually(|| gate.entered()).await;
        let updated = r#"{"Greeting 😀":"Latest private buffer"}"#;
        match action {
            "foreground" => {
                let hover = tokio::time::timeout(
                    Duration::from_secs(2),
                    h.mgr.hover(caller, Pos::new(0, 10), Default::default()),
                )
                .await
                .expect("foreground supersedes held watcher discovery")
                .unwrap()
                .unwrap();
                assert_eq!(hover.contents.value, TEXT);
            }
            "edit" => h.mgr.did_change(
                catalog,
                2,
                &[AppliedChange { range: rng(0, 0, 0, TEXT.encode_utf16().count() as u32), text: updated.into() }],
                &|| updated.into(),
            ),
            "close" => h.mgr.did_close(catalog),
            "disable" => h.mgr.set_execution_allowed(false),
            "restart" => {
                h.mgr.restart(h.mgr.servers().iter().find(|s| s.name == "phpantom").unwrap().id).await.unwrap()
            }
            _ => unreachable!(),
        }
        gate.open();
        if action == "edit" || action == "restart" {
            eventually(|| {
                state.docs.lock().get(&uri_of(&path)).map(String::as_str)
                    == Some(if action == "edit" { updated } else { TEXT })
            })
            .await;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
        if action == "close" || action == "disable" {
            assert!(!state.docs.lock().contains_key(&uri_of(&path)), "{action}");
        }
        assert_eq!(state.requested("textDocument/hover").len(), if action == "foreground" { 2 } else { 1 });
        assert_eq!(state.launches.load(Ordering::SeqCst), if action == "restart" { 2 } else { 1 });
        let opens = state
            .notified("textDocument/didOpen")
            .iter()
            .filter(|event| event["textDocument"]["uri"] == uri_of(&path))
            .count();
        assert_eq!(
            opens,
            if action == "close" || action == "disable" { 0 } else { 1 },
            "{action}: no stale or duplicate opens"
        );
        assert!(!path.exists());
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_external_changes_recover_cancelled_negative_discovery_without_php_navigation() {
    for action in ["created provider", "modified provider", "removed override", "modified composer", "reviewed edit"] {
        let mut capabilities = caps();
        capabilities["experimental"]["phpWorkbenchRenameChecked"] = json!(true);
        let h = harness_for("php", "php", &[("phpantom", capabilities)], &[]);
        let manifest = h.root.join("composer.json");
        std::fs::write(&manifest, MANIFEST).unwrap();
        let dependency = if action == "modified composer" { manifest } else { h.root.join("Provider.php") };
        if action != "created provider" && action != "modified composer" {
            std::fs::write(&dependency, "<?php /* before */").unwrap();
        }
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let expected = dependency.clone();
        let registered = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let active = registered.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move |state, method, params| {
            (method == METHOD).then(|| {
                let registered = active.load(Ordering::SeqCst);
                if registered {
                    assert!(
                        state.notified("workspace/didChangeWatchedFiles").iter().any(|event| {
                            event["changes"].as_array().unwrap().iter().any(|change| change["uri"] == uri_of(&expected))
                        }),
                        "discovery must follow the dependency notification"
                    );
                }
                echo(params, registered.then(|| resource.parent().unwrap()))
            })
        }));
        let (caller, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| !state.requested(METHOD).is_empty()).await;
        h.mgr.hover(caller, Pos::new(0, 10), Default::default()).await.unwrap();
        assert!(!state.docs.lock().contains_key(&uri_of(&path)));
        registered.store(true, Ordering::SeqCst);
        match action {
            "created provider" => {
                std::fs::write(&dependency, "<?php /* new registration */").unwrap();
                h.mgr.php_files_changed(std::slice::from_ref(&dependency), &[], &[]);
            }
            "removed override" => {
                std::fs::remove_file(&dependency).unwrap();
                h.mgr.php_files_changed(&[], &[], std::slice::from_ref(&dependency));
            }
            "modified composer" => {
                let updated = r#"{"require":{"laravel/framework":"13.37.0"},"extra":{"openstormTest":true}}"#;
                serde_json::from_str::<Value>(updated).unwrap();
                std::fs::write(&dependency, updated).unwrap();
                h.mgr.php_files_changed(&[], std::slice::from_ref(&dependency), &[]);
            }
            "reviewed edit" => {
                std::fs::write(&dependency, "<?php /* reviewed registration */").unwrap();
                h.mgr.php_reviewed_files_changed(&[], std::slice::from_ref(&dependency), &[]);
            }
            _ => {
                std::fs::write(&dependency, "<?php /* changed registration */").unwrap();
                h.mgr.php_files_changed(&[], std::slice::from_ref(&dependency), &[]);
            }
        }
        eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        if action == "reviewed edit" {
            let metadata = state.requested(METHOD).len();
            let notifications = state.notified("workspace/didChangeWatchedFiles").len();
            h.mgr.php_files_changed(&[], std::slice::from_ref(&dependency), &[]);
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert_eq!(state.requested(METHOD).len(), metadata, "coalesced echoes do not rediscover");
            assert_eq!(state.notified("workspace/didChangeWatchedFiles").len(), notifications);
        }
        assert_eq!(state.requested("textDocument/hover").len(), 1, "{action}: no new PHP request");
        assert_eq!(state.launches.load(Ordering::SeqCst), 1);
        assert!(!path.exists());
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_find_usages_admits_current_overlay_without_a_php_editor() {
    let h = harness_languages(
        &[("php", "php", &[("phpantom", caps())]), ("json", "json", &[("json-lsp", incremental_caps())])],
        |_| {},
    );
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let path = h.root.join("catalogs/en.json");
    let doc = DocId::next();
    h.mgr.did_open(doc, &path, &LanguageId::new("json"), 7, TEXT);
    let state = h.launcher.state("phpantom");
    let resource = path.clone();
    let caller = h.root.join("app/Caller.php");
    let expected = caller.clone();
    state.responder.lock().replace(Arc::new(move |state, method, params| {
        if method == METHOD { return Some(echo(params, resource.parent())); }
        if method == "textDocument/references" {
            assert_eq!(state.docs.lock().get(&uri_of(&resource)).map(String::as_str), Some(TEXT));
            assert_eq!(params["position"], json!({"line":0,"character":2}));
            return Some(json!([{"uri":uri_of(&caller),"range":{"start":{"line":0,"character":20},"end":{"line":0,"character":31}}}]));
        }
        None
    }));
    let refs = h.mgr.references(doc, Pos::new(0, 2), false, true, Default::default()).await.unwrap();
    assert_eq!(refs.locations.len(), 1, "normal Find Usages must use the registered PHP consumer");
    assert_eq!(*refs.locations[0].path, expected);
    assert!(refs.incomplete.is_none());
    assert_eq!(state.requested(METHOD).len(), 1);
    h.wait_status("json-lsp", ServerStatus::Ready).await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while state.notified("textDocument/didClose").is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(state.docs.lock().is_empty(), "temporary PHP overlay must be closed");
    assert_eq!(h.launcher.state("json-lsp").docs.lock().get(&uri_of(&path)).map(String::as_str), Some(TEXT));
    assert!(!path.exists());
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_find_usages_keeps_native_json_results_without_registration() {
    for registered in [false, true] {
        let h = harness_languages(
            &[("php", "php", &[("phpantom", caps())]), ("json", "json", &[("json-lsp", incremental_caps())])],
            |_| {},
        );
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let doc = DocId::next();
        h.mgr.did_open(doc, &path, &LanguageId::new("json"), 1, TEXT);
        let native = h.root.join("schema.json");
        let php = h.root.join("Caller.php");
        h.launcher.state("json-lsp").respond(
            "textDocument/references",
            json!([{"uri":uri_of(&native),"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}}}]),
        );
        let resource = path.clone();
        let target = php.clone();
        h.launcher.state("phpantom").responder.lock().replace(Arc::new(move |_, method, params| {
            match method {
                METHOD => Some(echo(params, registered.then(|| resource.parent().unwrap()))),
                "textDocument/references" => Some(json!([{"uri":uri_of(&target),"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}}}])),
                _ => None,
            }
        }));
        let refs = h.mgr.references(doc, Pos::new(0, 2), false, true, Default::default()).await.unwrap();
        let mut found: Vec<_> = refs.locations.iter().map(|l| l.path.to_path_buf()).collect();
        found.sort();
        let mut expected = vec![native];
        if registered {
            expected.push(php);
        }
        expected.sort();
        assert_eq!(found, expected);
        assert_eq!(!h.launcher.state("phpantom").notified("textDocument/didOpen").is_empty(), registered);
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_find_usages_refuses_stale_or_cancelled_responses_and_closes_overlay() {
    for change in ["json", "provider", "composer", "roots", "cancel", "disable", "restart"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move |_, method, params| match method {
            METHOD => Some(echo(params, resource.parent())),
            "textDocument/references" => {
                held.hold();
                Some(json!([]))
            }
            _ => None,
        }));
        let provider = if change == "provider" {
            let (provider, _) = h.open("Provider.php", "<?php /* original provider */");
            h.wait_status("phpantom", ServerStatus::Ready).await;
            Some(provider)
        } else {
            None
        };
        let doc = DocId::next();
        h.mgr.did_open(doc, &path, &LanguageId::new("json"), 1, TEXT);
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let mgr = h.mgr.clone();
        let pending = tokio::spawn(async move { mgr.references(doc, Pos::new(0, 2), false, true, token).await });
        eventually(|| gate.entered()).await;
        match change {
            "json" => h.mgr.did_change(
                doc,
                2,
                &[AppliedChange { range: rng(0, 0, 0, TEXT.encode_utf16().count() as u32), text: "{}".into() }],
                &|| "{}".into(),
            ),
            "provider" => h.mgr.did_change(
                provider.unwrap(),
                2,
                &[AppliedChange {
                    range: rng(0, 0, 0, "<?php /* original provider */".encode_utf16().count() as u32),
                    text: "<?php /* changed provider */".into(),
                }],
                &|| "<?php /* changed provider */".into(),
            ),
            "composer" => std::fs::write(h.root.join("composer.json"), "{}").unwrap(),
            "roots" => h.mgr.set_content_roots(vec![h.root.join("other")]),
            "cancel" => cancel.cancel(),
            "disable" => h.mgr.set_execution_allowed(false),
            "restart" => {
                let manager = h.mgr.clone();
                let id = h.mgr.servers()[0].id;
                let restarting = tokio::spawn(async move { manager.restart(id).await });
                eventually(|| {
                    state.launches.load(Ordering::SeqCst) > 1 || h.mgr.servers()[0].status != ServerStatus::Ready
                })
                .await;
                gate.open();
                restarting.await.unwrap().unwrap();
            }
            _ => unreachable!(),
        }
        gate.open();
        assert!(pending.await.unwrap().is_err(), "{change}");
        if !matches!(change, "disable" | "restart") {
            eventually(|| !state.notified("textDocument/didClose").is_empty()).await;
        }
        assert!(!path.exists());
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_find_usages_never_shares_text_before_a_current_positive_admission() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let path = h.root.join("catalogs/en.json");
    let resource = path.clone();
    let gate = Arc::new(Gate::default());
    let held = gate.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move |_, method, params| {
        (method == METHOD).then(|| {
            held.hold();
            echo(params, resource.parent())
        })
    }));
    let doc = DocId::next();
    h.mgr.did_open(doc, &path, &LanguageId::new("json"), 1, TEXT);
    let manager = h.mgr.clone();
    let pending =
        tokio::spawn(async move { manager.references(doc, Pos::new(0, 2), false, true, Default::default()).await });
    eventually(|| gate.entered()).await;
    assert!(state.notified("textDocument/didOpen").is_empty());
    h.mgr.did_close(doc);
    gate.open();
    assert!(pending.await.unwrap().is_err());
    assert!(state.notified("textDocument/didOpen").is_empty());
    assert!(state.requested("textDocument/references").is_empty());
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_find_usages_serializes_admission_and_closes_when_aborted() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    let manifest = h.root.join("composer.json");
    std::fs::write(&manifest, MANIFEST).unwrap();
    let path = h.root.join("catalogs/en.json");
    let resource = path.clone();
    let gate = Arc::new(Gate::default());
    let held = gate.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move |_, method, params| match method {
        METHOD => Some(echo(params, resource.parent())),
        "textDocument/references" => {
            held.hold();
            Some(json!([]))
        }
        _ => None,
    }));
    let doc = DocId::next();
    h.mgr.did_open(doc, &path, &LanguageId::new("json"), 1, TEXT);
    let manager = h.mgr.clone();
    let pending =
        tokio::spawn(async move { manager.references(doc, Pos::new(0, 2), false, true, Default::default()).await });
    eventually(|| gate.entered()).await;
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let manager = h.mgr.clone();
    let proposed = path.clone();
    let project = manifest.clone();
    let waiting =
        tokio::spawn(
            async move { manager.php_json_translation_resource_from_composer(&project, &proposed, token).await },
        );
    tokio::task::yield_now().await;
    cancel.cancel();
    assert_eq!(waiting.await.unwrap().unwrap_err().kind, ide_core::error::ErrorKind::Cancelled);
    assert_eq!(state.requested(METHOD).len(), 1, "another selection must wait while the catalog is open");
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    gate.open();
    eventually(|| !state.notified("textDocument/didClose").is_empty()).await;
    assert!(state.docs.lock().is_empty());
    assert_eq!(
        h.mgr
            .php_json_translation_resource_from_composer(&manifest, &path, Default::default())
            .await
            .unwrap()
            .root_path(),
        path.parent()
    );
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_find_usages_requires_buffer_capability_and_preserves_native_fallback() {
    for enabled in [false, true] {
        let mut capabilities = caps();
        capabilities["experimental"]["phpWorkbenchRegisteredJsonTranslations"] = json!(enabled);
        let h = harness_languages(
            &[("php", "php", &[("phpantom", capabilities)]), ("json", "json", &[("json-lsp", incremental_caps())])],
            |_| {},
        );
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let state = h.launcher.state("phpantom");
        state
            .responder
            .lock()
            .replace(Arc::new(move |_, method, params| (method == METHOD).then(|| echo(params, resource.parent()))));
        let doc = DocId::next();
        h.mgr.did_open(doc, &path, &LanguageId::new("json"), 1, TEXT);
        let refs = h.mgr.references(doc, Pos::new(0, 2), false, true, Default::default()).await.unwrap();
        assert!(refs.locations.is_empty());
        assert_eq!(state.requested(METHOD).len(), usize::from(enabled));
        assert_eq!(state.notified("textDocument/didOpen").len(), usize::from(enabled));
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_find_usages_refuses_conflicting_buffers_before_forwarding() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let path = h.root.join("catalogs/en.json");
    let resource = path.clone();
    let state = h.launcher.state("phpantom");
    state
        .responder
        .lock()
        .replace(Arc::new(move |_, method, params| (method == METHOD).then(|| echo(params, resource.parent()))));
    let a = DocId::next();
    let b = DocId::next();
    h.mgr.did_open(a, &path, &LanguageId::new("json"), 1, TEXT);
    h.mgr.did_open(b, &path, &LanguageId::new("json"), 2, "{}");
    assert!(h.mgr.references(a, Pos::new(0, 2), false, true, Default::default()).await.is_err());
    assert!(state.notified("textDocument/didOpen").is_empty());
    assert!(state.requested("textDocument/references").is_empty());
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_find_usages_reports_failed_native_json_queries_as_incomplete() {
    let h = harness_languages(
        &[("php", "php", &[("phpantom", caps())]), ("json", "json", &[("json-lsp", incremental_caps())])],
        |_| {},
    );
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let path = h.root.join("catalogs/en.json");
    let resource = path.clone();
    h.launcher
        .state("json-lsp")
        .errors
        .lock()
        .insert("textDocument/references".into(), (-32603, "synthetic native failure".into()));
    h.launcher
        .state("phpantom")
        .responder
        .lock()
        .replace(Arc::new(move |_, method, params| (method == METHOD).then(|| echo(params, resource.parent()))));
    let doc = DocId::next();
    h.mgr.did_open(doc, &path, &LanguageId::new("json"), 1, TEXT);
    let refs = h.mgr.references(doc, Pos::new(0, 2), false, true, Default::default()).await.unwrap();
    assert_eq!(refs.incomplete, Some(ReferencesIncomplete::ServerFailed));
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_sessions_close_before_a_held_reference_reply_after_edit_or_close() {
    for change in ["edit", "close"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move |_, method, params| match method {
            METHOD => Some(echo(params, resource.parent())),
            "textDocument/references" => {
                held.hold();
                Some(json!([]))
            }
            _ => None,
        }));
        let doc = DocId::next();
        h.mgr.did_open(doc, &path, &LanguageId::new("json"), 1, TEXT);
        let manager = h.mgr.clone();
        let pending =
            tokio::spawn(async move { manager.references(doc, Pos::new(0, 2), false, true, Default::default()).await });
        eventually(|| gate.entered()).await;
        if change == "close" {
            h.mgr.did_close(doc);
        } else {
            h.mgr.did_change(
                doc,
                2,
                &[AppliedChange { range: rng(0, 0, 0, TEXT.encode_utf16().count() as u32), text: "{}".into() }],
                &|| "{}".into(),
            );
        }
        let closed = tokio::time::timeout(Duration::from_secs(2), async {
            while state.docs.lock().contains_key(&uri_of(&path)) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .is_ok();
        gate.open();
        let result = pending.await.unwrap();
        h.mgr.shutdown_all().await;
        assert!(closed, "{change}: discard the stale overlay before the held server reply is released");
        assert!(result.is_err());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_sessions_invalidate_php_replies_across_a_complete_open_close_cycle() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let resource = h.root.join("catalogs/en.json");
    let catalog = resource.clone();
    let gate = Arc::new(Gate::default());
    let held = gate.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move |_, method, params| match method {
        METHOD => Some(echo(params, resource.parent())),
        "textDocument/hover" => {
            held.hold();
            Some(json!({"contents":{"kind":"markdown","value":"old semantic reply"}}))
        }
        _ => None,
    }));
    let (php, _) = h.open("Caller.php", "<?php class Caller {}");
    h.wait_status("phpantom", ServerStatus::Ready).await;
    let json = DocId::next();
    h.mgr.did_open(json, &catalog, &LanguageId::new("json"), 1, TEXT);
    let manager = h.mgr.clone();
    let pending = tokio::spawn(async move { manager.hover(php, Pos::new(0, 12), Default::default()).await });
    eventually(|| gate.entered()).await;
    let result = h.mgr.references(json, Pos::new(0, 2), false, true, Default::default()).await;
    gate.open();
    let hover = pending.await.unwrap();
    h.mgr.shutdown_all().await;
    assert!(result.is_ok(), "{result:?}");
    assert!(
        hover.is_err(),
        "a PHP reply spanning a complete catalog session must be retired even when the document set matches again: {hover:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_sessions_cancel_before_held_replies_and_retire_concurrent_php_hover() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let resource = h.root.join("catalogs/en.json");
    let catalog = resource.clone();
    let ref_gate = Arc::new(Gate::default());
    let held_ref = ref_gate.clone();
    let hover_gate = Arc::new(Gate::default());
    let held_hover = hover_gate.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move |state, method, params| match method {
        METHOD => Some(echo(params, resource.parent())),
        "textDocument/references" => {
            held_ref.hold();
            Some(json!([]))
        }
        "textDocument/hover" => {
            assert_eq!(state.docs.lock().get(&uri_of(&resource)).map(String::as_str), Some(TEXT));
            held_hover.hold();
            Some(json!({"contents":{"kind":"markdown","value":"private overlay"}}))
        }
        _ => None,
    }));
    let (php, _) = h.open("Caller.php", "<?php class Caller {}");
    h.wait_status("phpantom", ServerStatus::Ready).await;
    let json = DocId::next();
    h.mgr.did_open(json, &catalog, &LanguageId::new("json"), 1, TEXT);
    let manager = h.mgr.clone();
    let refs =
        tokio::spawn(async move { manager.references(json, Pos::new(0, 2), false, true, Default::default()).await });
    eventually(|| ref_gate.entered()).await;
    let manager = h.mgr.clone();
    let hover = tokio::spawn(async move { manager.hover(php, Pos::new(0, 12), Default::default()).await });
    eventually(|| hover_gate.entered()).await;
    h.mgr.did_change(
        json,
        2,
        &[AppliedChange { range: rng(0, 0, 0, TEXT.encode_utf16().count() as u32), text: "{}".into() }],
        &|| "{}".into(),
    );
    let retired = tokio::time::timeout(Duration::from_secs(2), async {
        while !refs.is_finished() || state.docs.lock().contains_key(&uri_of(&catalog)) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    ref_gate.open();
    hover_gate.open();
    let refs = refs.await.unwrap();
    let hover = hover.await.unwrap();
    h.mgr.shutdown_all().await;
    assert!(retired, "retire the pending references and the overlay without waiting for the server reply");
    assert!(refs.is_err());
    assert!(hover.is_err(), "{hover:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_sessions_retire_for_relevant_inputs_and_preserve_unrelated_json_buffers() {
    for change in [
        "provider edit",
        "provider open",
        "provider close",
        "composer edit",
        "composer open",
        "composer close",
        "roots",
        "settings",
        "watch",
        "disable",
        "unrelated",
    ] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let manifest = h.root.join("composer.json");
        std::fs::write(&manifest, MANIFEST).unwrap();
        let resource = h.root.join("catalogs/en.json");
        let catalog = resource.clone();
        let state = h.launcher.state("phpantom");
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        state.responder.lock().replace(Arc::new(move |_, method, params| match method {
            METHOD => Some(echo(params, resource.parent())),
            "textDocument/references" => {
                held.hold();
                Some(json!([]))
            }
            _ => None,
        }));
        let (provider, _) = h.open("Provider.php", "<?php /* provider */");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let composer = DocId::next();
        if change.starts_with("composer") && change != "composer open" {
            h.mgr.did_open(composer, &manifest, &LanguageId::new("json"), 1, MANIFEST);
        }
        let json = DocId::next();
        h.mgr.did_open(json, &catalog, &LanguageId::new("json"), 1, TEXT);
        let manager = h.mgr.clone();
        let pending =
            tokio::spawn(
                async move { manager.references(json, Pos::new(0, 2), false, true, Default::default()).await },
            );
        eventually(|| gate.entered()).await;
        match change {
            "provider edit" => h.mgr.did_change(
                provider,
                2,
                &[AppliedChange {
                    range: rng(0, 0, 0, "<?php /* provider */".encode_utf16().count() as u32),
                    text: "<?php /* changed */".into(),
                }],
                &|| "<?php /* changed */".into(),
            ),
            "provider open" => {
                h.open("New.php", "<?php class NewProvider {}");
            }
            "provider close" => h.mgr.did_close(provider),
            "composer open" => h.mgr.did_open(composer, &manifest, &LanguageId::new("json"), 1, MANIFEST),
            "composer close" => h.mgr.did_close(composer),
            "composer edit" => h.mgr.did_change(
                composer,
                2,
                &[AppliedChange { range: rng(0, 0, 0, MANIFEST.len() as u32), text: "{}".into() }],
                &|| "{}".into(),
            ),
            "roots" => h.mgr.set_content_roots(vec![h.root.join("other")]),
            "settings" => h.mgr.reconfigure().await,
            "watch" => {
                let path = h.root.join("Changed.php");
                std::fs::write(&path, "<?php class Changed {}").unwrap();
                h.mgr.php_files_changed(&[path], &[], &[]);
            }
            "disable" => h.mgr.set_execution_allowed(false),
            "unrelated" => {
                h.mgr.did_open(DocId::next(), &h.root.join("unrelated.json"), &LanguageId::new("json"), 1, "{}")
            }
            _ => unreachable!(),
        }
        let retired = if change == "unrelated" {
            false
        } else {
            tokio::time::timeout(Duration::from_secs(2), async {
                while !pending.is_finished() || state.docs.lock().contains_key(&uri_of(&catalog)) {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .is_ok()
        };
        let unrelated_stays =
            change != "unrelated" || (!pending.is_finished() && state.docs.lock().contains_key(&uri_of(&catalog)));
        gate.open();
        let result = pending.await.unwrap();
        h.mgr.shutdown_all().await;
        if change == "unrelated" {
            assert!(unrelated_stays);
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(retired, "{change}: retire before the held reply is released");
            assert!(result.is_err());
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_sessions_keep_json_actions_and_require_new_admission_after_restart() {
    let h = harness_languages(
        &[("php", "php", &[("phpantom", caps())]), ("json", "json", &[("json-lsp", incremental_caps())])],
        |_| {},
    );
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let resource = h.root.join("catalogs/en.json");
    let catalog = resource.clone();
    let gate = Arc::new(Gate::default());
    let held = gate.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move |_, method, params| match method {
        METHOD => Some(echo(params, resource.parent())),
        "textDocument/references" => {
            held.hold();
            Some(json!([]))
        }
        _ => None,
    }));
    let json = DocId::next();
    h.mgr.did_open(json, &catalog, &LanguageId::new("json"), 1, TEXT);
    let manager = h.mgr.clone();
    let pending =
        tokio::spawn(async move { manager.references(json, Pos::new(0, 2), false, true, Default::default()).await });
    eventually(|| gate.entered()).await;
    let completion = h.mgr.completion(json, Pos::new(0, 2), None, Default::default()).await;
    let id = h.mgr.servers().into_iter().find(|server| server.name == "phpantom").unwrap().id;
    h.mgr.restart(id).await.unwrap();
    let reopened = state
        .notified("textDocument/didOpen")
        .iter()
        .filter(|params| params["textDocument"]["uri"] == uri_of(&catalog))
        .count();
    let retired = tokio::time::timeout(Duration::from_secs(2), pending).await;
    gate.open();
    assert!(completion.is_ok());
    assert!(state.requested("textDocument/completion").is_empty());
    assert_eq!(reopened, 1, "a new generation must not replay the old catalog before admission");
    assert!(retired.unwrap().unwrap().is_err());
    let refs = h.mgr.references(json, Pos::new(0, 2), false, true, Default::default()).await.unwrap();
    assert!(refs.locations.is_empty());
    assert_eq!(state.requested(METHOD).len(), 2);
    assert_eq!(h.launcher.state("json-lsp").docs.lock().get(&uri_of(&catalog)).map(String::as_str), Some(TEXT));
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_ordinary_php_hover_automatically_keeps_and_refreshes_catalog_overlays() {
    let h = harness_languages(
        &[("php", "php", &[("phpantom", caps())]), ("json", "json", &[("json-lsp", incremental_caps())])],
        |_| {},
    );
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let resource = h.root.join("catalogs/en.json");
    let catalog = resource.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move|state,method,params|match method{
        METHOD=>Some(echo(params,resource.parent())),
        "textDocument/hover"=>Some(json!({"contents":{"kind":"markdown","value":state.docs.lock().get(&uri_of(&resource)).cloned().unwrap_or_else(||"Disk value".into())}})),
        _=>None,
    }));
    let (php, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
    let json = DocId::next();
    h.mgr.did_open(json, &catalog, &LanguageId::new("json"), 1, TEXT);
    let hover = h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap().unwrap();
    let active = state.docs.lock().get(&uri_of(&catalog)).cloned();
    let reused = h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap().unwrap();
    assert_eq!(reused.contents.value, TEXT);
    assert_eq!(state.requested(METHOD).len(), 1, "reuse unchanged admission");
    let updated = r#"{"Greeting 😀":"Updated draft"}"#;
    h.mgr.did_change(
        json,
        2,
        &[AppliedChange { range: rng(0, 0, 0, TEXT.encode_utf16().count() as u32), text: updated.into() }],
        &|| updated.into(),
    );
    let changed = h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap().unwrap();
    assert_eq!(changed.contents.value, updated);
    let server = h.mgr.servers().into_iter().find(|server| server.name == "phpantom").unwrap().id;
    h.mgr.restart(server).await.unwrap();
    let restarted = h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap().unwrap();
    assert_eq!(restarted.contents.value, updated);
    h.mgr.did_close(json);
    let closed = h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap().unwrap();
    assert_eq!(closed.contents.value, "Disk value");
    h.mgr.shutdown_all().await;
    assert_eq!(
        hover.contents.value, TEXT,
        "ordinary PHP hover must see the unsaved registered catalog without Find Usages"
    );
    assert_eq!(active.as_deref(), Some(TEXT), "retain the admitted overlay after the PHP request");
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_automatic_sync_preserves_provider_choices_and_refuses_unproven_roots() {
    for (name, enabled, registered) in
        [("phpantom", false, true), ("phpantom", true, false), ("phpactor", true, true), ("intelephense", true, true)]
    {
        let mut capabilities = caps();
        capabilities["experimental"]["phpWorkbenchRegisteredJsonTranslations"] = json!(enabled);
        let h = harness_for("php", "php", &[(name, capabilities)], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let resource = h.root.join("catalogs/en.json");
        let catalog = resource.clone();
        let state = h.launcher.state(name);
        state.responder.lock().replace(Arc::new(move |_, method, params| match method {
            METHOD => Some(echo(params, registered.then(|| resource.parent().unwrap()))),
            "textDocument/hover" => Some(json!({"contents":{"kind":"markdown","value":"ordinary provider"}})),
            _ => None,
        }));
        let (php, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        h.mgr.did_open(DocId::next(), &catalog, &LanguageId::new("json"), 1, TEXT);
        let hover = h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap().unwrap();
        assert_eq!(hover.contents.value, "ordinary provider");
        assert!(
            state
                .notified("textDocument/didOpen")
                .iter()
                .all(|params| params["textDocument"]["uri"] != uri_of(&catalog))
        );
        assert_eq!(state.requested(METHOD).len(), usize::from(name == "phpantom" && enabled));
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_automatic_sync_cancels_pending_admission_before_sharing_text() {
    for change in ["cancel", "json edit", "json close", "php edit", "settings", "disable"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let resource = h.root.join("catalogs/en.json");
        let catalog = resource.clone();
        let state = h.launcher.state("phpantom");
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        state.responder.lock().replace(Arc::new(move |_, method, params| {
            (method == METHOD).then(|| {
                held.hold();
                echo(params, resource.parent())
            })
        }));
        let code = "<?php __('Greeting 😀');";
        let (php, _) = h.open("Caller.php", code);
        let json = DocId::next();
        h.mgr.did_open(json, &catalog, &LanguageId::new("json"), 1, TEXT);
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let manager = h.mgr.clone();
        let pending = tokio::spawn(async move { manager.hover(php, Pos::new(0, 10), token).await });
        eventually(|| gate.entered()).await;
        match change {
            "cancel" => cancel.cancel(),
            "json edit" => h.mgr.did_change(
                json,
                2,
                &[AppliedChange { range: rng(0, 0, 0, TEXT.encode_utf16().count() as u32), text: "{}".into() }],
                &|| "{}".into(),
            ),
            "json close" => h.mgr.did_close(json),
            "php edit" => h.mgr.did_change(
                php,
                2,
                &[AppliedChange {
                    range: rng(0, 0, 0, code.encode_utf16().count() as u32),
                    text: "<?php /* changed */".into(),
                }],
                &|| "<?php /* changed */".into(),
            ),
            "settings" => h.mgr.reconfigure().await,
            "disable" => h.mgr.set_execution_allowed(false),
            _ => unreachable!(),
        }
        let retired = tokio::time::timeout(Duration::from_secs(2), async {
            while !pending.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .is_ok();
        gate.open();
        let result = pending.await.unwrap();
        h.mgr.shutdown_all().await;
        assert!(retired, "{change}");
        assert!(result.is_err(), "{change}: {result:?}");
        assert!(
            state
                .notified("textDocument/didOpen")
                .iter()
                .all(|params| params["textDocument"]["uri"] != uri_of(&catalog))
        );
        assert!(state.requested("textDocument/hover").is_empty());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_opening_a_catalog_retires_a_php_reply_started_without_overlays() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let gate = Arc::new(Gate::default());
    let held = gate.clone();
    h.launcher.state("phpantom").responder.lock().replace(Arc::new(move |_, method, _| {
        (method == "textDocument/hover").then(|| {
            held.hold();
            json!({"contents":{"kind":"markdown","value":"old disk value"}})
        })
    }));
    let (php, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
    let manager = h.mgr.clone();
    let pending = tokio::spawn(async move { manager.hover(php, Pos::new(0, 10), Default::default()).await });
    eventually(|| gate.entered()).await;
    let json = DocId::next();
    h.mgr.did_open(json, &h.root.join("catalogs/en.json"), &LanguageId::new("json"), 1, TEXT);
    h.mgr.did_close(json);
    gate.open();
    let result = pending.await.unwrap();
    h.mgr.shutdown_all().await;
    assert!(result.is_err(), "a PHP reply spanning a potential catalog open/close must expire: {result:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_automatic_sync_uses_the_consuming_manifest_for_unsaved_php_files() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    let manifest = h.root.join("composer.json");
    std::fs::write(&manifest, MANIFEST).unwrap();
    let nested = h.root.join("packages/billing");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("composer.json"), "{}").unwrap();
    let resource = nested.join("json/en.json");
    let catalog = resource.clone();
    let expected_manifest = manifest.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move|state,method,params|match method{METHOD=>{assert_eq!(params["projectUri"],uri_of(&expected_manifest));Some(echo(params,resource.parent()))},"textDocument/hover"=>Some(json!({"contents":{"kind":"markdown","value":state.docs.lock().get(&uri_of(&resource)).cloned().unwrap_or_else(||"disk".into())}})),_=>None}));
    let php = DocId::next();
    let path = h.root.join("Unsaved.php");
    h.mgr.did_open(php, &path, &LanguageId::new("php"), 1, "<?php __('Greeting 😀');");
    h.mgr.did_open(DocId::next(), &catalog, &LanguageId::new("json"), 1, TEXT);
    let hover = h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap().unwrap();
    assert_eq!(hover.contents.value, TEXT);
    assert!(!path.exists());
    assert!(!catalog.exists());
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_automatic_sync_rechecks_retained_composer_inputs_before_reuse() {
    for change in ["disk", "buffer"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let manifest = h.root.join("composer.json");
        std::fs::write(&manifest, MANIFEST).unwrap();
        let catalog = h.root.join("catalogs/en.json");
        let resource = catalog.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move |state, method, params| match method {
            METHOD => Some(echo(params, resource.parent())),
            "textDocument/hover" => Some(json!({"contents":{"kind":"markdown","value":state.docs.lock().get(&uri_of(&resource)).cloned().unwrap_or_else(||"disk".into())}})),
            _ => None,
        }));
        let (php, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        h.mgr.did_open(DocId::next(), &catalog, &LanguageId::new("json"), 1, TEXT);
        let hover = h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap().unwrap();
        assert_eq!(hover.contents.value, TEXT);
        let composer = if change == "disk" {
            // Deliberately omit a watcher notification: a retained witness must still expire.
            std::fs::write(&manifest, "{}").unwrap();
            None
        } else {
            let doc = DocId::next();
            h.mgr.did_open(doc, &manifest, &LanguageId::new("json"), 1, "{}");
            Some(doc)
        };
        let stale = h.mgr.hover(php, Pos::new(0, 10), Default::default()).await;
        assert!(stale.is_err(), "{change}: {stale:?}");
        assert_eq!(state.requested("textDocument/hover").len(), 1, "do not dispatch using stale authority");
        eventually(|| !state.docs.lock().contains_key(&uri_of(&catalog))).await;
        assert_eq!(state.requested(METHOD).len(), 1, "do not admit while a manifest buffer is dirty");
        if let Some(doc) = composer {
            h.mgr.did_close(doc);
        } else {
            std::fs::write(&manifest, MANIFEST).unwrap();
        }
        let recovered = h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap().unwrap();
        assert_eq!(recovered.contents.value, TEXT);
        assert_eq!(state.requested(METHOD).len(), 2, "recovery requires a fresh admission");
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_automatic_sync_readmits_when_a_shared_server_changes_consuming_projects() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let nested = h.root.join("packages/consumer");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("composer.json"), MANIFEST).unwrap();
    let catalog = h.root.join("catalogs/en.json");
    let resource = catalog.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move |state, method, params| match method {
        METHOD => Some(echo(params, resource.parent())),
        "textDocument/hover" => Some(json!({"contents":{"kind":"markdown","value":state.docs.lock().get(&uri_of(&resource)).cloned().unwrap_or_else(||"disk".into())}})),
        _ => None,
    }));
    let (outer, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
    let (inner, _) = h.open("packages/consumer/Caller.php", "<?php __('Greeting 😀');");
    h.mgr.did_open(DocId::next(), &catalog, &LanguageId::new("json"), 1, TEXT);
    for doc in [outer, inner, outer] {
        let hover = h.mgr.hover(doc, Pos::new(0, 10), Default::default()).await.unwrap().unwrap();
        assert_eq!(hover.contents.value, TEXT);
    }
    let projects: Vec<_> = state.requested(METHOD).into_iter().map(|params| params["projectUri"].clone()).collect();
    assert_eq!(
        projects,
        vec![
            uri_of(&h.root.join("composer.json")),
            uri_of(&nested.join("composer.json")),
            uri_of(&h.root.join("composer.json"))
        ]
    );
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_php_loading_retry_cannot_adopt_a_replacement_catalog_session() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let catalog = h.root.join("catalogs/en.json");
    let resource = catalog.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move |state, method, params| match method {
        METHOD => Some(echo(params, resource.parent())),
        "textDocument/hover" => Some(if state.requested("textDocument/hover").len() == 1 {
            Value::Null
        } else {
            json!({"contents":{"kind":"markdown","value":"replacement session"}})
        }),
        "textDocument/references" => Some(json!([])),
        _ => None,
    }));
    let (php, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
    let json = DocId::next();
    h.mgr.did_open(json, &catalog, &LanguageId::new("json"), 1, TEXT);
    h.wait_status("phpantom", ServerStatus::Ready).await;
    state.peer().notify("experimental/serverStatus", json!({"health":"ok","quiescent":false})).unwrap();
    h.wait_status("phpantom", ServerStatus::Busy).await;
    let manager = h.mgr.clone();
    let pending = tokio::spawn(async move { manager.hover(php, Pos::new(0, 10), Default::default()).await });
    eventually(|| state.requested("textDocument/hover").len() == 1).await;
    h.mgr.references(json, Pos::new(0, 2), false, true, Default::default()).await.unwrap();
    let fresh = h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap().unwrap();
    assert_eq!(fresh.contents.value, "replacement session");
    state.peer().notify("experimental/serverStatus", json!({"health":"ok","quiescent":true})).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), pending).await.unwrap().unwrap();
    h.mgr.shutdown_all().await;
    assert!(result.is_err(), "the old preparation must not adopt a replacement session on retry: {result:?}");
    assert_eq!(state.requested("textDocument/hover").len(), 2, "reject before sending the old retry");
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_retained_catalog_refreshes_after_edits_and_saves_without_a_php_request() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let path = h.root.join("catalogs/en.json");
    let resource = path.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move |state, method, params| match method {
        METHOD => Some(echo(params, resource.parent())),
        "textDocument/hover" => Some(json!({"contents":{"kind":"markdown","value":state.docs.lock().get(&uri_of(&resource)).cloned().unwrap_or_else(||"disk".into())}})),
        _ => None,
    }));
    let (php, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
    let catalog = DocId::next();
    h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
    h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap();
    let updated = r#"{"Greeting 😀":"Updated without PHP action"}"#;
    h.mgr.did_change(
        catalog,
        2,
        &[AppliedChange { range: rng(0, 0, 0, TEXT.encode_utf16().count() as u32), text: updated.into() }],
        &|| updated.into(),
    );
    eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(updated)).await;
    assert_eq!(state.requested("textDocument/hover").len(), 1);
    assert_eq!(state.requested(METHOD).len(), 2, "re-admit before forwarding an edited catalog");
    h.mgr.did_save(catalog, updated);
    h.mgr.php_files_changed(&[], std::slice::from_ref(&path), &[]);
    eventually(|| {
        state.requested(METHOD).len() == 3 && state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(updated)
    })
    .await;
    assert_eq!(state.requested("textDocument/hover").len(), 1);
    h.mgr.did_close(catalog);
    eventually(|| !state.docs.lock().contains_key(&uri_of(&path))).await;
    assert!(!path.exists());
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_background_refresh_retires_held_admission_and_yields_to_foreground_context() {
    for change in ["edit", "foreground", "close", "disable", "restart"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let nested = h.root.join("packages/consumer");
        std::fs::create_dir_all(&nested).unwrap();
        let second_manifest = nested.join("composer.json");
        std::fs::write(&second_manifest, MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move|state,method,params|match method {
            METHOD=> {if state.requested(METHOD).len()==2 {held.hold();}Some(echo(params,(params["projectUri"]!=uri_of(&second_manifest)).then(||resource.parent().unwrap())))},
            "textDocument/hover"=>Some(json!({"contents":{"kind":"markdown","value":state.docs.lock().get(&uri_of(&resource)).cloned().unwrap_or_else(||"disk".into())}})),
            _=>None,
        }));
        let (php, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        let (other, _) = h.open("packages/consumer/Caller.php", "<?php __('Greeting 😀');");
        let catalog = DocId::next();
        h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
        h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap();
        let updated = r#"{"Greeting 😀":"Pending private value"}"#;
        h.mgr.did_change(
            catalog,
            2,
            &[AppliedChange { range: rng(0, 0, 0, TEXT.encode_utf16().count() as u32), text: updated.into() }],
            &|| updated.into(),
        );
        eventually(|| gate.entered()).await;
        let latest = r#"{"Greeting 😀":"Latest private value"}"#;
        match change {
            "edit" => {
                h.mgr.did_change(
                    catalog,
                    3,
                    &[AppliedChange {
                        range: rng(0, 0, 0, updated.encode_utf16().count() as u32),
                        text: latest.into(),
                    }],
                    &|| latest.into(),
                );
                eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(latest)).await;
            }
            "foreground" => {
                let result = tokio::time::timeout(
                    Duration::from_secs(2),
                    h.mgr.hover(other, Pos::new(0, 10), Default::default()),
                )
                .await
                .expect("foreground action must cancel the held background admission")
                .unwrap()
                .unwrap();
                assert_eq!(result.contents.value, "disk");
            }
            "close" => h.mgr.did_close(catalog),
            "disable" => h.mgr.set_execution_allowed(false),
            "restart" => {
                h.mgr.did_change(
                    catalog,
                    3,
                    &[AppliedChange {
                        range: rng(0, 0, 0, updated.encode_utf16().count() as u32),
                        text: latest.into(),
                    }],
                    &|| latest.into(),
                );
                let server = h.mgr.servers().into_iter().find(|s| s.name == "phpantom").unwrap().id;
                h.mgr.restart(server).await.unwrap();
                eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(latest)).await;
            }
            _ => unreachable!(),
        }
        gate.open();
        tokio::time::sleep(Duration::from_millis(200)).await;
        if change != "edit" && change != "restart" {
            assert!(!state.docs.lock().contains_key(&uri_of(&path)), "{change}");
        }
        assert!(
            state.notified("textDocument/didOpen").iter().all(|params| params["textDocument"]["text"] != updated),
            "{change}: stale text must not be shared"
        );
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_background_refresh_waits_for_a_clean_manifest_without_spinning() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    let manifest = h.root.join("composer.json");
    std::fs::write(&manifest, MANIFEST).unwrap();
    let path = h.root.join("catalogs/en.json");
    let resource = path.clone();
    let gate = Arc::new(Gate::default());
    let held = gate.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move |state, method, params| match method {
        METHOD => {
            if state.requested(METHOD).len() == 2 {
                held.hold();
            }
            Some(echo(params, resource.parent()))
        }
        "textDocument/hover" => Some(json!({"contents":{"kind":"markdown","value":"initial"}})),
        _ => None,
    }));
    let (php, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
    let catalog = DocId::next();
    h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
    h.mgr.hover(php, Pos::new(0, 10), Default::default()).await.unwrap();
    let updated = r#"{"Greeting 😀":"Recover without PHP action"}"#;
    h.mgr.did_change(
        catalog,
        2,
        &[AppliedChange { range: rng(0, 0, 0, TEXT.encode_utf16().count() as u32), text: updated.into() }],
        &|| updated.into(),
    );
    eventually(|| gate.entered()).await;
    let composer = DocId::next();
    h.mgr.did_open(composer, &manifest, &LanguageId::new("json"), 1, "{}");
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(state.requested(METHOD).len(), 2);
    assert!(!state.docs.lock().contains_key(&uri_of(&path)));
    h.mgr.did_close(composer);
    eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(updated)).await;
    gate.open();
    assert_eq!(state.requested("textDocument/hover").len(), 1);
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_opening_a_catalog_discovers_an_active_php_consumer_without_a_php_request() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let path = h.root.join("packages/billing/json/en.json");
    let resource = path.clone();
    let state = h.launcher.state("phpantom");
    state
        .responder
        .lock()
        .replace(Arc::new(move |_, method, params| (method == METHOD).then(|| echo(params, resource.parent()))));
    h.open("Caller.php", "<?php __('Greeting 😀');");
    h.wait_status("phpantom", ServerStatus::Ready).await;
    let catalog = DocId::next();
    h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
    eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
    assert!(state.requested("textDocument/hover").is_empty());
    assert!(state.requested(METHOD).iter().all(|params| params["projectUri"] == uri_of(&h.root.join("composer.json"))));
    assert!(!path.exists());
    h.mgr.did_close(catalog);
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_discovery_checks_every_open_context_before_unique_admission() {
    for mode in ["unique", "ambiguous", "incomplete"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let first = h.root.join("composer.json");
        std::fs::write(&first, MANIFEST).unwrap();
        let nested = h.root.join("consumers/other");
        std::fs::create_dir_all(&nested).unwrap();
        let second = nested.join("composer.json");
        std::fs::write(&second, if mode == "incomplete" { "broken JSON" } else { MANIFEST }).unwrap();
        let package = h.root.join("packages/billing");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("composer.json"), "{}").unwrap();
        let path = package.join("json/en.json");
        let resource = path.clone();
        let second_uri = uri_of(&second);
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move |_, method, params| {
            if method != METHOD {
                return None;
            }
            if params["projectUri"] == second_uri {
                held.hold();
            }
            Some(echo(
                params,
                (mode == "ambiguous" || params["projectUri"] != second_uri).then(|| resource.parent().unwrap()),
            ))
        }));
        h.open("Caller.php", "<?php __('Greeting 😀');");
        h.open("SecondCaller.php", "<?php __('Greeting 😀');");
        let (other, _) = h.open("consumers/other/Caller.php", "<?php __('Greeting 😀');");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let catalog = DocId::next();
        h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
        if mode != "incomplete" {
            eventually(|| gate.entered()).await;
            assert!(
                !state.docs.lock().contains_key(&uri_of(&path)),
                "do not send text while another consumer is unproven"
            );
            gate.open();
        }
        if mode == "unique" {
            eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
            let projects: Vec<_> =
                state.requested(METHOD).into_iter().map(|params| params["projectUri"].clone()).collect();
            assert_eq!(
                projects,
                vec![uri_of(&first), uri_of(&second), uri_of(&first)],
                "deduplicate PHP buffers, then select the unique positive context again"
            );
        } else {
            eventually(|| !state.requested(METHOD).is_empty()).await;
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert!(!state.docs.lock().contains_key(&uri_of(&path)), "{mode}");
            h.mgr.did_close(other);
            eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        }
        assert!(
            state.requested(METHOD).iter().all(|params| params["projectUri"] != uri_of(&package.join("composer.json"))),
            "the catalog's nearest package is not its consumer"
        );
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_discovery_yields_to_foreground_and_lifecycle_changes_before_sharing() {
    for change in ["foreground", "close", "disable", "restart"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move|state,method,params|match method {
            METHOD=>{if state.requested(METHOD).len()==1 {held.hold();}Some(echo(params,resource.parent()))},
            "textDocument/hover"=>Some(json!({"contents":{"kind":"markdown","value":state.docs.lock().get(&uri_of(&resource)).cloned().unwrap_or_else(||"disk".into())}})),_=>None,
        }));
        let (php, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let catalog = DocId::next();
        h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| gate.entered()).await;
        match change {
            "foreground" => {
                let hover =
                    tokio::time::timeout(Duration::from_secs(2), h.mgr.hover(php, Pos::new(0, 10), Default::default()))
                        .await
                        .expect("foreground PHP action must supersede discovery")
                        .unwrap()
                        .unwrap();
                assert_eq!(hover.contents.value, TEXT);
            }
            "close" => h.mgr.did_close(catalog),
            "disable" => h.mgr.set_execution_allowed(false),
            "restart" => {
                let latest = r#"{"Greeting 😀":"New generation overlay"}"#;
                h.mgr.did_change(
                    catalog,
                    2,
                    &[AppliedChange { range: rng(0, 0, 0, TEXT.encode_utf16().count() as u32), text: latest.into() }],
                    &|| latest.into(),
                );
                let server = h.mgr.servers().into_iter().find(|s| s.name == "phpantom").unwrap().id;
                h.mgr.restart(server).await.unwrap();
                eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(latest)).await;
            }
            _ => unreachable!(),
        }
        gate.open();
        tokio::time::sleep(Duration::from_millis(200)).await;
        let opens: Vec<_> = state
            .notified("textDocument/didOpen")
            .into_iter()
            .filter(|params| params["textDocument"]["uri"] == uri_of(&path))
            .collect();
        assert_eq!(opens.len(), usize::from(change == "foreground" || change == "restart"), "{change}");
        if change == "restart" {
            assert_eq!(opens[0]["textDocument"]["version"], 2);
            assert_eq!(opens[0]["textDocument"]["text"], r#"{"Greeting 😀":"New generation overlay"}"#);
        }
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_discovery_keeps_capabilities_and_server_choices_and_recovers_from_negative_results() {
    for (name, advertised) in [("phpantom", false), ("phpactor", true), ("intelephense", true)] {
        let h = harness_for("php", "php", &[(name, if advertised { caps() } else { incremental_caps() })], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status(name, ServerStatus::Ready).await;
        h.mgr.did_open(DocId::next(), &h.root.join("catalogs/en.json"), &LanguageId::new("json"), 1, TEXT);
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(h.launcher.state(name).requested(METHOD).is_empty());
        h.mgr.shutdown_all().await;
    }
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    h.mgr.did_open(DocId::next(), &h.root.join("unopened/en.json"), &LanguageId::new("json"), 1, TEXT);
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(h.mgr.servers().is_empty());
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let path = h.root.join("catalogs/en.json");
    let resource = path.clone();
    let registered = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let active = registered.clone();
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move |_, method, params| {
        (method == METHOD).then(|| echo(params, active.load(Ordering::Acquire).then(|| resource.parent().unwrap())))
    }));
    h.open("Caller.php", "<?php __('Greeting 😀');");
    h.wait_status("phpantom", ServerStatus::Ready).await;
    h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
    eventually(|| !state.requested(METHOD).is_empty()).await;
    assert!(!state.docs.lock().contains_key(&uri_of(&path)));
    registered.store(true, Ordering::Release);
    h.open("Provider.php", "<?php /* registration changed */");
    eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
    assert!(state.requested("textDocument/hover").is_empty());
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_discovery_rechecks_other_composer_witnesses_after_final_selection() {
    for change in ["existing", "nearest"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let nested = h.root.join("consumers/other");
        std::fs::create_dir_all(&nested).unwrap();
        let second = nested.join("composer.json");
        if change == "existing" {
            std::fs::write(&second, MANIFEST).unwrap();
        }
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let second_uri = uri_of(&second);
        let state = h.launcher.state("phpantom");
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        state.responder.lock().replace(Arc::new(move |state, method, params| {
            if method != METHOD {
                return None;
            }
            if state.requested(METHOD).len() == if change == "existing" { 3 } else { 1 } {
                held.hold();
            }
            Some(echo(
                params,
                (change == "nearest" || params["projectUri"] != second_uri).then(|| resource.parent().unwrap()),
            ))
        }));
        h.open("Caller.php", "<?php __('Greeting 😀');");
        let (other, _) = h.open("consumers/other/Caller.php", "<?php __('Greeting 😀');");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| gate.entered()).await;
        // Omit a watcher notification to test both disk content and context selection.
        std::fs::write(&second, r#"{"require":{"laravel/framework":"13.37.0"},"extra":{"changed":true}}"#).unwrap();
        gate.open();
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(
            !state.docs.lock().contains_key(&uri_of(&path)),
            "{change}: do not forward after consumer context changed during selection"
        );
        h.mgr.did_close(other);
        eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_readiness_discovers_catalogs_opened_before_php_without_a_php_request() {
    for timing in ["before PHP", "during initialize", "closed during initialize", "disabled during initialize"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let state = h.launcher.state("phpantom");
        state
            .responder
            .lock()
            .replace(Arc::new(move |_, method, params| (method == METHOD).then(|| echo(params, resource.parent()))));
        let gate = Arc::new(Gate::default());
        *state.initialize_gate.lock() = Some(gate.clone());
        let catalog = DocId::next();
        if timing == "before PHP" {
            h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
            tokio::time::sleep(Duration::from_millis(150)).await;
            assert!(h.mgr.servers().is_empty(), "JSON alone cannot start PHP");
        }
        h.open("Caller.php", "<?php __('Greeting 😀');");
        eventually(|| gate.entered()).await;
        if timing != "before PHP" {
            h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
        }
        let current = r#"{"Greeting 😀":"Latest during startup"}"#;
        h.mgr.did_change(
            catalog,
            2,
            &[AppliedChange { range: rng(0, 0, 0, TEXT.encode_utf16().count() as u32), text: current.into() }],
            &|| current.into(),
        );
        if timing == "closed during initialize" {
            h.mgr.did_close(catalog);
        } else if timing == "disabled during initialize" {
            h.mgr.set_execution_allowed(false);
        }
        assert!(!state.docs.lock().contains_key(&uri_of(&path)));
        assert!(state.requested(METHOD).is_empty());
        gate.open();
        if timing == "before PHP" || timing == "during initialize" {
            eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(current)).await;
            assert_eq!(state.requested(METHOD).len(), 1, "{timing}");
            if timing == "before PHP" {
                let opens = state.notified("textDocument/didOpen").len();
                let server = h.mgr.servers().into_iter().find(|server| server.name == "phpantom").unwrap().id;
                h.mgr.restart(server).await.unwrap();
                eventually(|| state.notified("textDocument/didOpen").len() >= opens + 2).await;
                assert_eq!(state.docs.lock().get(&uri_of(&path)).map(String::as_str), Some(current));
                assert_eq!(state.requested(METHOD).len(), 2, "restart revalidates registration");
            }
        } else {
            tokio::time::sleep(Duration::from_millis(250)).await;
            assert!(!state.docs.lock().contains_key(&uri_of(&path)), "{timing}");
            assert!(state.requested(METHOD).is_empty(), "{timing}");
        }
        assert!(state.requested("textDocument/hover").is_empty());
        assert!(!path.exists());
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_readiness_preserves_server_choices_and_requires_resource_capabilities() {
    for (name, advertised) in [("phpantom", false), ("phpactor", true), ("intelephense", true)] {
        let h = harness_for("php", "php", &[(name, if advertised { caps() } else { incremental_caps() })], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
        assert!(h.mgr.servers().is_empty());
        h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status(name, ServerStatus::Ready).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        let state = h.launcher.state(name);
        assert!(state.requested(METHOD).is_empty(), "{name}");
        assert!(!state.docs.lock().contains_key(&uri_of(&path)), "{name}");
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_new_php_buffer_reconciles_catalogs_on_a_running_server() {
    for change in ["same project", "replace last PHP buffer", "new consuming project"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let first = h.root.join("composer.json");
        std::fs::write(&first, MANIFEST).unwrap();
        let nested = h.root.join("consumers/other");
        std::fs::create_dir_all(&nested).unwrap();
        let second = nested.join("composer.json");
        std::fs::write(&second, MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let selected = Arc::new(Mutex::new(uri_of(&first)));
        let current = selected.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move |_, method, params| {
            (method == METHOD)
                .then(|| echo(params, (params["projectUri"] == *current.lock()).then(|| resource.parent().unwrap())))
        }));
        let (php, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let server = h.mgr.servers()[0].id;
        h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        let opens = state
            .notified("textDocument/didOpen")
            .iter()
            .filter(|params| params["textDocument"]["uri"] == uri_of(&path))
            .count();
        if change == "replace last PHP buffer" {
            h.mgr.did_close(php);
        } else if change == "new consuming project" {
            *selected.lock() = uri_of(&second);
        }
        h.open(
            if change == "new consuming project" { "consumers/other/Caller.php" } else { "Second.php" },
            "<?php __('Greeting 😀');",
        );
        eventually(|| !state.docs.lock().contains_key(&uri_of(&path))).await;
        eventually(|| {
            state
                .notified("textDocument/didOpen")
                .iter()
                .filter(|params| params["textDocument"]["uri"] == uri_of(&path))
                .count()
                > opens
        })
        .await;
        assert_eq!(state.docs.lock().get(&uri_of(&path)).map(String::as_str), Some(TEXT));
        assert_eq!(state.requested(METHOD).last().unwrap()["projectUri"], *selected.lock());
        assert!(state.requested("textDocument/hover").is_empty());
        assert_eq!(h.mgr.servers()[0].id, server);
        assert_eq!(state.launches.load(Ordering::SeqCst), 1);
        assert!(!path.exists());
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_new_php_consumer_checks_all_contexts_and_keeps_lifecycle_priority() {
    for mode in ["unique", "ambiguous", "incomplete", "foreground", "close", "disable"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let nested = h.root.join("consumers/other");
        std::fs::create_dir_all(&nested).unwrap();
        let manifest = nested.join("composer.json");
        std::fs::write(&manifest, if mode == "incomplete" { "broken JSON" } else { MANIFEST }).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let other_uri = uri_of(&manifest);
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move |state, method, params| match method {
            METHOD => {
                let other = params["projectUri"] == other_uri;
                if other { held.hold(); }
                Some(echo(params, (!other || mode == "ambiguous").then(|| resource.parent().unwrap())))
            }
            "textDocument/hover" => Some(json!({"contents":{"kind":"markdown","value":state.docs.lock().get(&uri_of(&resource)).cloned().unwrap_or_else(||"disk".into())}})),
            _ => None,
        }));
        let (php, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let catalog = DocId::next();
        h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        let (other, _) = h.open("consumers/other/Caller.php", "<?php __('Greeting 😀');");
        eventually(|| !state.docs.lock().contains_key(&uri_of(&path))).await;
        if mode != "incomplete" {
            eventually(|| gate.entered()).await;
        }
        assert!(!state.docs.lock().contains_key(&uri_of(&path)), "{mode}: wait for every consumer");
        match mode {
            "foreground" => {
                let hover =
                    tokio::time::timeout(Duration::from_secs(2), h.mgr.hover(php, Pos::new(0, 10), Default::default()))
                        .await
                        .expect("foreground PHP cancels held attachment reconciliation")
                        .unwrap()
                        .unwrap();
                assert_eq!(hover.contents.value, TEXT);
            }
            "close" => h.mgr.did_close(catalog),
            "disable" => h.mgr.set_execution_allowed(false),
            _ => {}
        }
        gate.open();
        if mode == "unique" {
            eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        } else if mode != "foreground" {
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert!(!state.docs.lock().contains_key(&uri_of(&path)), "{mode}");
            if mode == "ambiguous" || mode == "incomplete" {
                h.mgr.did_close(other);
                eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
            }
        }
        let opens = state
            .notified("textDocument/didOpen")
            .iter()
            .filter(|params| params["textDocument"]["uri"] == uri_of(&path))
            .count();
        assert_eq!(opens, if mode == "close" || mode == "disable" { 1 } else { 2 }, "{mode}");
        assert_eq!(state.requested("textDocument/hover").len(), usize::from(mode == "foreground"));
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_php_attachment_keeps_optional_servers_and_capability_boundaries() {
    for (name, advertised) in [("phpantom", false), ("phpactor", true), ("intelephense", true)] {
        let h = harness_for("php", "php", &[(name, if advertised { caps() } else { incremental_caps() })], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status(name, ServerStatus::Ready).await;
        let path = h.root.join("catalogs/en.json");
        h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
        h.open("Second.php", "<?php __('Greeting 😀');");
        tokio::time::sleep(Duration::from_millis(200)).await;
        let state = h.launcher.state(name);
        assert!(state.requested(METHOD).is_empty(), "{name}");
        assert!(!state.docs.lock().contains_key(&uri_of(&path)), "{name}");
        assert_eq!(state.launches.load(Ordering::SeqCst), 1);
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_php_edits_saves_and_closes_reconcile_finished_sessions() {
    for event in ["edit", "save", "close", "close last PHP"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let state = h.launcher.state("phpantom");
        state
            .responder
            .lock()
            .replace(Arc::new(move |_, method, params| (method == METHOD).then(|| echo(params, resource.parent()))));
        let (caller, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        let provider_text = "<?php /* provider */";
        let provider = if event == "close last PHP" { caller } else { h.open("Provider.php", provider_text).0 };
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let server = h.mgr.servers()[0].id;
        h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        let opens = || {
            state
                .notified("textDocument/didOpen")
                .iter()
                .filter(|params| params["textDocument"]["uri"] == uri_of(&path))
                .count()
        };
        match event {
            "edit" => {
                let updated = "<?php /* updated provider */";
                h.mgr.did_change(
                    provider,
                    2,
                    &[AppliedChange {
                        range: rng(0, 0, 0, provider_text.encode_utf16().count() as u32),
                        text: updated.into(),
                    }],
                    &|| updated.into(),
                );
            }
            "save" => h.mgr.did_save(provider, provider_text),
            "close" | "close last PHP" => h.mgr.did_close(provider),
            _ => unreachable!(),
        }
        eventually(|| {
            state.notified("textDocument/didClose").iter().any(|params| params["textDocument"]["uri"] == uri_of(&path))
        })
        .await;
        if event == "close last PHP" {
            tokio::time::sleep(Duration::from_millis(250)).await;
            assert_eq!(opens(), 1);
            assert!(!state.docs.lock().contains_key(&uri_of(&path)));
        } else {
            eventually(|| opens() == 2).await;
            assert_eq!(state.docs.lock().get(&uri_of(&path)).map(String::as_str), Some(TEXT));
        }
        assert!(state.requested("textDocument/hover").is_empty());
        assert_eq!(h.mgr.servers()[0].id, server);
        assert_eq!(state.launches.load(Ordering::SeqCst), 1);
        assert!(!path.exists());
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_php_provider_edits_switch_and_restore_open_catalogs_without_navigation() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let original = h.root.join("catalogs/en.json");
    let alternate = h.root.join("alternate/en.json");
    let first = original.clone();
    let second = alternate.clone();
    let provider_uri = uri_of(&h.root.join("Provider.php"));
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(move |state, method, params| {
        if method != METHOD {
            return None;
        }
        let alternate = state.docs.lock().get(&provider_uri).is_some_and(|text| text.contains("alternate"));
        let selected = if alternate { &second } else { &first };
        Some(echo(params, (params["resourceUri"] == uri_of(selected)).then(|| selected.parent().unwrap())))
    }));
    h.open("Caller.php", "<?php __('Greeting 😀');");
    let initial = "<?php /* original provider */";
    let changed = "<?php /* alternate provider */";
    let (provider, _) = h.open("Provider.php", initial);
    h.wait_status("phpantom", ServerStatus::Ready).await;
    h.mgr.did_open(DocId::next(), &original, &LanguageId::new("json"), 1, TEXT);
    h.mgr.did_open(DocId::next(), &alternate, &LanguageId::new("json"), 1, TEXT);
    eventually(|| state.docs.lock().get(&uri_of(&original)).map(String::as_str) == Some(TEXT)).await;
    eventually(|| state.requested(METHOD).iter().any(|params| params["resourceUri"] == uri_of(&alternate))).await;
    let edit = |version, before: &str, after: &str| {
        h.mgr.did_change(
            provider,
            version,
            &[AppliedChange { range: rng(0, 0, 0, before.encode_utf16().count() as u32), text: after.into() }],
            &|| after.into(),
        )
    };
    edit(2, initial, changed);
    eventually(|| state.docs.lock().get(&uri_of(&alternate)).map(String::as_str) == Some(TEXT)).await;
    assert!(!state.docs.lock().contains_key(&uri_of(&original)));
    edit(3, changed, initial);
    eventually(|| state.docs.lock().get(&uri_of(&original)).map(String::as_str) == Some(TEXT)).await;
    assert!(!state.docs.lock().contains_key(&uri_of(&alternate)));
    assert!(state.requested("textDocument/hover").is_empty());
    assert!(!original.exists() && !alternate.exists());
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_php_lifecycle_reconciliation_cancels_stale_admission_and_keeps_foreground_priority() {
    for action in ["edit again", "foreground", "close", "disable"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let provider_uri = uri_of(&h.root.join("Provider.php"));
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move |state, method, params| match method {
            METHOD => {
                let registered = state.docs.lock().get(&provider_uri).is_some_and(|text| !text.contains("unregistered"));
                if state.requested(METHOD).len() == 2 { held.hold(); }
                Some(echo(params, registered.then(|| resource.parent().unwrap())))
            }
            "textDocument/hover" => Some(json!({"contents":{"kind":"markdown","value":state.docs.lock().get(&uri_of(&resource)).cloned().unwrap_or_else(||"disk".into())}})),
            _ => None,
        }));
        let (caller, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        let initial = "<?php /* registered */";
        let updated = "<?php /* registered v2 */";
        let rejected = "<?php /* unregistered */";
        let (provider, _) = h.open("Provider.php", initial);
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let catalog = DocId::next();
        h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        let edit = |version, before: &str, after: &str| {
            h.mgr.did_change(
                provider,
                version,
                &[AppliedChange { range: rng(0, 0, 0, before.encode_utf16().count() as u32), text: after.into() }],
                &|| after.into(),
            )
        };
        edit(2, initial, updated);
        eventually(|| gate.entered()).await;
        assert!(!state.docs.lock().contains_key(&uri_of(&path)));
        match action {
            "edit again" => {
                edit(3, updated, rejected);
                eventually(|| state.requested(METHOD).len() >= 3).await;
            }
            "foreground" => {
                let hover = tokio::time::timeout(
                    Duration::from_secs(2),
                    h.mgr.hover(caller, Pos::new(0, 10), Default::default()),
                )
                .await
                .expect("foreground action supersedes held lifecycle reconciliation")
                .unwrap()
                .unwrap();
                assert_eq!(hover.contents.value, TEXT);
            }
            "close" => h.mgr.did_close(catalog),
            "disable" => h.mgr.set_execution_allowed(false),
            _ => unreachable!(),
        }
        gate.open();
        tokio::time::sleep(Duration::from_millis(200)).await;
        if action != "foreground" {
            assert!(!state.docs.lock().contains_key(&uri_of(&path)), "{action}");
        }
        if action == "edit again" {
            edit(4, rejected, initial);
            eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        }
        let opens = state
            .notified("textDocument/didOpen")
            .iter()
            .filter(|params| params["textDocument"]["uri"] == uri_of(&path))
            .count();
        assert_eq!(opens, if action == "foreground" || action == "edit again" { 2 } else { 1 }, "{action}");
        assert_eq!(state.requested("textDocument/hover").len(), usize::from(action == "foreground"));
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_reconfiguration_readmits_catalogs_without_navigation_or_restart() {
    for change in ["settings", "unchanged settings", "content roots"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        let settings: Arc<Mutex<Option<Value>>> = Default::default();
        let current = settings.clone();
        h.mgr.set_config_hook(Box::new(move |cfg, _| cfg.settings = current.lock().clone()));
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let state = h.launcher.state("phpantom");
        state
            .responder
            .lock()
            .replace(Arc::new(move |_, method, params| (method == METHOD).then(|| echo(params, resource.parent()))));
        h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let server = h.mgr.servers()[0].id;
        h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        match change {
            "settings" => {
                *settings.lock() = Some(json!({"openstormTest":{"revision":1}}));
                h.mgr.reconfigure().await;
            }
            "unchanged settings" => h.mgr.reconfigure().await,
            "content roots" => {
                let root = h.root.join("additional");
                std::fs::create_dir(&root).unwrap();
                h.mgr.set_content_roots(vec![root]);
            }
            _ => unreachable!(),
        }
        eventually(|| {
            state
                .notified("textDocument/didOpen")
                .iter()
                .filter(|params| params["textDocument"]["uri"] == uri_of(&path))
                .count()
                == 2
        })
        .await;
        assert_eq!(state.docs.lock().get(&uri_of(&path)).map(String::as_str), Some(TEXT));
        assert_eq!(state.requested(METHOD).len(), 2, "{change}: revalidate before sharing");
        assert_eq!(state.launches.load(Ordering::SeqCst), 1);
        assert_eq!(h.mgr.servers()[0].id, server);
        assert!(state.requested("textDocument/hover").is_empty());
        assert!(!path.exists());
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_content_root_reconciliation_preserves_boundaries_and_server_choices() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    let external = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(external.path()).unwrap();
    std::fs::write(root.join("composer.json"), MANIFEST).unwrap();
    let source = root.join("Caller.php");
    std::fs::write(&source, "<?php __('Greeting 😀');").unwrap();
    let path = root.join("catalogs/en.json");
    let resource = path.clone();
    let state = h.launcher.state("phpantom");
    state
        .responder
        .lock()
        .replace(Arc::new(move |_, method, params| (method == METHOD).then(|| echo(params, resource.parent()))));
    h.mgr.set_content_roots(vec![root.clone()]);
    h.mgr.did_open(DocId::next(), &source, &LanguageId::new("php"), 1, "<?php __('Greeting 😀');");
    h.wait_status("phpantom", ServerStatus::Ready).await;
    h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
    eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
    h.mgr.set_content_roots(vec![]);
    eventually(|| !state.docs.lock().contains_key(&uri_of(&path))).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(state.requested(METHOD).len(), 1, "removed roots cannot be queried or shared");
    h.mgr.set_content_roots(vec![root]);
    eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
    assert_eq!(state.requested(METHOD).len(), 2);
    assert_eq!(state.launches.load(Ordering::SeqCst), 1);
    h.mgr.shutdown_all().await;

    for (name, advertised) in [("phpantom", false), ("phpactor", true), ("intelephense", true)] {
        let h = harness_for("php", "php", &[(name, if advertised { caps() } else { incremental_caps() })], &[]);
        std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
        h.open("Caller.php", "<?php");
        h.wait_status(name, ServerStatus::Ready).await;
        let path = h.root.join("catalogs/en.json");
        h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
        h.mgr.reconfigure().await;
        h.mgr.set_content_roots(vec![h.root.join("additional")]);
        tokio::time::sleep(Duration::from_millis(200)).await;
        let state = h.launcher.state(name);
        assert!(state.requested(METHOD).is_empty(), "{name}");
        assert!(!state.docs.lock().contains_key(&uri_of(&path)), "{name}");
        assert_eq!(state.launches.load(Ordering::SeqCst), 1);
        h.mgr.shutdown_all().await;
    }
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    h.mgr.did_open(DocId::next(), &h.root.join("catalogs/en.json"), &LanguageId::new("json"), 1, TEXT);
    h.mgr.reconfigure().await;
    h.mgr.set_content_roots(vec![h.root.join("additional")]);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(h.mgr.servers().is_empty(), "JSON-only reconfiguration cannot start PHP");
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_discovery_waits_until_every_reconfiguration_pass_finishes() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    std::fs::write(h.root.join("composer.json"), MANIFEST).unwrap();
    let revision = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let gate = Arc::new(Gate::default());
    let (current, held) = (revision.clone(), gate.clone());
    h.mgr.set_config_hook(Box::new(move |cfg, _| {
        let value = current.load(Ordering::SeqCst);
        if value == 1 {
            held.hold();
        }
        cfg.settings = Some(json!({"openstormTest":{"revision":value}}));
    }));
    let path = h.root.join("catalogs/en.json");
    let resource = path.clone();
    let state = h.launcher.state("phpantom");
    state
        .responder
        .lock()
        .replace(Arc::new(move |_, method, params| (method == METHOD).then(|| echo(params, resource.parent()))));
    h.open("Caller.php", "<?php __('Greeting 😀');");
    h.wait_status("phpantom", ServerStatus::Ready).await;
    h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
    eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
    revision.store(1, Ordering::SeqCst);
    let first = tokio::spawn({
        let manager = h.mgr.clone();
        async move { manager.reconfigure().await }
    });
    eventually(|| gate.entered()).await;
    revision.store(2, Ordering::SeqCst);
    let mut second = Box::pin(h.mgr.reconfigure());
    assert!(futures::poll!(&mut second).is_pending(), "second pass waits for the reconfiguration lock");
    gate.open();
    first.await.unwrap();
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert!(!state.docs.lock().contains_key(&uri_of(&path)), "a pending configuration pass cannot publish a catalog");
    assert_eq!(state.requested(METHOD).len(), 1, "metadata waits for every pass to finish");
    second.await;
    eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
    assert_eq!(state.requested(METHOD).len(), 2);
    assert_eq!(
        state.notified("workspace/didChangeConfiguration").last().unwrap()["settings"]["openstormTest"]["revision"],
        2
    );
    assert_eq!(state.launches.load(Ordering::SeqCst), 1);
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_composer_buffers_recover_without_another_php_request() {
    for action in ["discard", "restore", "save", "clean open", "clean save"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let manifest = h.root.join("composer.json");
        std::fs::write(&manifest, MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let state = h.launcher.state("phpantom");
        state
            .responder
            .lock()
            .replace(Arc::new(move |_, method, params| (method == METHOD).then(|| echo(params, resource.parent()))));
        let (caller, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        h.mgr.hover(caller, Pos::new(0, 10), Default::default()).await.unwrap();
        let composer = DocId::next();
        let clean = action == "clean open" || action == "clean save";
        h.mgr.did_open(composer, &manifest, &LanguageId::new("json"), 1, if clean { MANIFEST } else { "{}" });
        if clean {
            eventually(|| {
                state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)
                    && state.requested(METHOD).len() == 2
            })
            .await;
            if action == "clean save" {
                h.mgr.did_save(composer, MANIFEST);
                eventually(|| {
                    state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)
                        && state.requested(METHOD).len() == 3
                })
                .await;
            }
        } else {
            eventually(|| !state.docs.lock().contains_key(&uri_of(&path))).await;
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert_eq!(state.requested(METHOD).len(), 1, "dirty manifests cannot authorize a catalog");
            match action {
                "discard" => h.mgr.did_close(composer),
                "restore" => h.mgr.did_change(
                    composer,
                    2,
                    &[AppliedChange { range: rng(0, 0, 0, 2), text: MANIFEST.into() }],
                    &|| MANIFEST.into(),
                ),
                "save" => {
                    let saved = r#"{"require":{"laravel/framework":"13.37.0"},"extra":{"openstormTest":true}}"#;
                    std::fs::write(&manifest, saved).unwrap();
                    h.mgr.did_save(composer, saved);
                    h.mgr.php_files_changed(&[], std::slice::from_ref(&manifest), &[]);
                }
                _ => unreachable!(),
            }
            eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
            assert_eq!(state.requested(METHOD).len(), 2, "{action}");
        }
        assert_eq!(state.requested("textDocument/hover").len(), 1, "{action}: no new PHP request");
        assert_eq!(state.launches.load(Ordering::SeqCst), 1);
        assert!(!path.exists());
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_composer_input_changes_cancel_held_admission_and_keep_foreground_priority() {
    for action in ["dirty edit", "foreground", "close", "disable"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let manifest = h.root.join("composer.json");
        std::fs::write(&manifest, MANIFEST).unwrap();
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move |state, method, params| match method {
            METHOD => {
                if state.requested(METHOD).len() == 2 { held.hold(); }
                Some(echo(params, resource.parent()))
            }
            "textDocument/hover" => Some(json!({"contents":{"kind":"markdown","value":state.docs.lock().get(&uri_of(&resource)).cloned().unwrap_or_else(||"disk".into())}})),
            _ => None,
        }));
        let (caller, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let catalog = DocId::next();
        h.mgr.did_open(catalog, &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        h.mgr.hover(caller, Pos::new(0, 10), Default::default()).await.unwrap();
        let composer = DocId::next();
        h.mgr.did_open(composer, &manifest, &LanguageId::new("json"), 1, MANIFEST);
        eventually(|| gate.entered()).await;
        match action {
            "dirty edit" => h.mgr.did_change(
                composer,
                2,
                &[AppliedChange { range: rng(0, 0, 0, MANIFEST.encode_utf16().count() as u32), text: "{}".into() }],
                &|| "{}".into(),
            ),
            "foreground" => {
                let hover = tokio::time::timeout(
                    Duration::from_secs(2),
                    h.mgr.hover(caller, Pos::new(0, 10), Default::default()),
                )
                .await
                .expect("foreground PHP supersedes Composer reconciliation")
                .unwrap()
                .unwrap();
                assert_eq!(hover.contents.value, TEXT);
            }
            "close" => h.mgr.did_close(catalog),
            "disable" => h.mgr.set_execution_allowed(false),
            _ => unreachable!(),
        }
        gate.open();
        tokio::time::sleep(Duration::from_millis(200)).await;
        if action != "foreground" {
            assert!(!state.docs.lock().contains_key(&uri_of(&path)), "{action}");
        }
        if action == "dirty edit" {
            assert_eq!(state.requested(METHOD).len(), 2, "dirty buffer cannot refresh from saved manifest");
            h.mgr.did_close(composer);
            eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        }
        let opens = state
            .notified("textDocument/didOpen")
            .iter()
            .filter(|params| params["textDocument"]["uri"] == uri_of(&path))
            .count();
        assert_eq!(opens, if action == "foreground" || action == "dirty edit" { 2 } else { 1 }, "{action}");
        assert_eq!(state.requested("textDocument/hover").len(), if action == "foreground" { 2 } else { 1 });
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_json_other_consumer_composer_changes_retire_selected_catalogs_and_recheck_ambiguity() {
    for action in ["discard dirty", "save ambiguous"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let first = h.root.join("composer.json");
        std::fs::write(&first, MANIFEST).unwrap();
        let nested = h.root.join("consumers/other");
        std::fs::create_dir_all(&nested).unwrap();
        let second = nested.join("composer.json");
        std::fs::write(&second, MANIFEST).unwrap();
        let second_uri = uri_of(&second);
        let path = h.root.join("catalogs/en.json");
        let resource = path.clone();
        let ambiguous = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let positive = ambiguous.clone();
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move |_, method, params| {
            (method == METHOD).then(|| {
                echo(
                    params,
                    (params["projectUri"] != second_uri || positive.load(Ordering::SeqCst))
                        .then(|| resource.parent().unwrap()),
                )
            })
        }));
        let (caller, _) = h.open("Caller.php", "<?php __('Greeting 😀');");
        let (other, _) = h.open("consumers/other/Caller.php", "<?php __('Greeting 😀');");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        h.mgr.did_open(DocId::next(), &path, &LanguageId::new("json"), 1, TEXT);
        eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        h.mgr.hover(caller, Pos::new(0, 10), Default::default()).await.unwrap();
        let composer = DocId::next();
        let updated = r#"{"require":{"laravel/framework":"13.37.0"},"extra":{"openstormTest":true}}"#;
        h.mgr.did_open(composer, &second, &LanguageId::new("json"), 1, updated);
        eventually(|| !state.docs.lock().contains_key(&uri_of(&path))).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!state.docs.lock().contains_key(&uri_of(&path)), "dirty unselected contexts invalidate uniqueness");
        if action == "discard dirty" {
            h.mgr.did_close(composer);
        } else {
            ambiguous.store(true, Ordering::SeqCst);
            std::fs::write(&second, updated).unwrap();
            let before = state.requested(METHOD).len();
            h.mgr.did_save(composer, updated);
            eventually(|| state.requested(METHOD).len() >= before + 2).await;
            tokio::time::sleep(Duration::from_millis(200)).await;
            assert!(
                !state.docs.lock().contains_key(&uri_of(&path)),
                "a newly positive second context remains ambiguous"
            );
            h.mgr.did_close(other);
        }
        eventually(|| state.docs.lock().get(&uri_of(&path)).map(String::as_str) == Some(TEXT)).await;
        assert_eq!(state.requested("textDocument/hover").len(), 1);
        assert_eq!(state.launches.load(Ordering::SeqCst), 1);
        assert!(!path.exists());
        h.mgr.shutdown_all().await;
    }
}
