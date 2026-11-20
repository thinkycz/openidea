use super::json_translation_resources::{caps, echo};
use super::*;

const METHOD: &str = "phpWorkbench/jsonTranslationResource";
const MANIFEST: &str = r#"{"require":{"laravel/framework":"13.37.0"}}"#;
const TEXT: &str = r#"{"Greeting 😀":"Private draft"}"#;

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
