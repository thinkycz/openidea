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
                let server = h.mgr.servers().into_iter().find(|s| s.name == "phpantom").unwrap().id;
                h.mgr.restart(server).await.unwrap();
            }
            _ => unreachable!(),
        }
        gate.open();
        tokio::time::sleep(Duration::from_millis(200)).await;
        if change != "edit" {
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
