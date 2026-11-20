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
