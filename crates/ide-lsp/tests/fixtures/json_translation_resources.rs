use super::*;

const METHOD: &str = "phpWorkbench/jsonTranslationResource";

pub(super) fn caps() -> Value {
    let mut caps = incremental_caps();
    caps["experimental"] = json!({
        "phpWorkbenchJsonTranslationResource":true,
        "phpWorkbenchRegisteredJsonTranslations":true
    });
    caps
}

pub(super) fn echo(params: &Value, root: Option<&Path>) -> Value {
    json!({"projectUri":params["projectUri"],"resourceUri":params["resourceUri"],
        "rootUri":root.map(|p| format!("{}/", uri_of(p)))})
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_admits_without_opening_php_or_sharing_catalog_buffers() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    let manifest = h.root.join("composer.json");
    std::fs::write(&manifest, r#"{"require":{"laravel/framework":"13.37.0"}}"#).unwrap();
    let resource = h.root.join("packages/catalogs/😀 json/en.json");
    let state = h.launcher.state("phpantom");
    let directory = resource.parent().unwrap().to_owned();
    state
        .responder
        .lock()
        .replace(Arc::new(move |_, method, params| (method == METHOD).then(|| echo(params, Some(&directory)))));
    assert!(h.mgr.servers().is_empty());
    let receipt =
        h.mgr.php_json_translation_resource_from_composer(&manifest, &resource, Default::default()).await.unwrap();
    assert_eq!(receipt.project_path(), manifest);
    assert_eq!(receipt.resource_path(), resource);
    assert_eq!(receipt.root_path(), resource.parent());
    h.mgr.php_json_translation_resource_current(&receipt, Default::default()).await.unwrap();
    assert_eq!(state.requested(METHOD), [json!({"projectUri":uri_of(&manifest),"resourceUri":uri_of(&resource)})]);
    assert!(state.notified("textDocument/didOpen").is_empty());
    assert!(state.notified("textDocument/didChange").is_empty());
    assert!(state.docs.lock().is_empty());
    assert!(!resource.exists());
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_requires_both_capabilities_and_a_phpantom_php_source() {
    for (name, language, extension, query, buffers) in [
        ("phpantom", "php", "php", Value::Null, json!(true)),
        ("phpantom", "php", "php", json!(true), Value::Null),
        ("phpantom", "php", "php", json!(false), json!(true)),
        ("phpantom", "php", "php", json!(true), json!(false)),
        ("phpantom", "php", "php", json!("true"), json!(true)),
        ("phpantom", "php", "php", json!(true), json!("true")),
        ("phpactor", "php", "php", json!(true), json!(true)),
        ("phpantom", "json", "json", json!(true), json!(true)),
        ("phpantom", "php", "txt", json!(true), json!(true)),
    ] {
        let mut caps = incremental_caps();
        caps["experimental"] = json!({"phpWorkbenchJsonTranslationResource":query,
            "phpWorkbenchRegisteredJsonTranslations":buffers});
        let h = harness_for(language, extension, &[(name, caps)], &[]);
        let (doc, _) = h.open(&format!("caller.{extension}"), "<?php");
        h.wait_status(name, ServerStatus::Ready).await;
        let result = h.mgr.php_json_translation_resource(doc, &h.root.join("json/en.json"), Default::default()).await;
        assert!(result.is_err(), "{name}/{language}/{extension}");
        assert!(h.launcher.state(name).requested(METHOD).is_empty());
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_keeps_negative_decisions_and_never_creates_missing_paths() {
    for registered in [false, true] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let (doc, _) = h.open("caller.php", "<?php");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let resource = h.root.join("not-created/😀 catalogs/fr.json");
        let root = registered.then(|| resource.parent().unwrap().to_owned());
        h.launcher
            .state("phpantom")
            .responder
            .lock()
            .replace(Arc::new(move |_, method, params| (method == METHOD).then(|| echo(params, root.as_deref()))));
        let receipt = h.mgr.php_json_translation_resource(doc, &resource, Default::default()).await.unwrap();
        assert_eq!(receipt.root_path(), registered.then(|| resource.parent().unwrap()));
        h.mgr.php_json_translation_resource_current(&receipt, Default::default()).await.unwrap();
        assert!(!h.root.join("not-created").exists());
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_rejects_mismatched_or_malformed_replies() {
    for mutation in
        ["project", "resource", "other root", "nested root", "remote", "query", "fragment", "slash", "missing", "type"]
    {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let (doc, _) = h.open("caller.php", "<?php");
        let resource = h.root.join("json/en.json");
        let root = resource.parent().unwrap().to_owned();
        h.wait_status("phpantom", ServerStatus::Ready).await;
        h.launcher.state("phpantom").responder.lock().replace(Arc::new(move |_, method, params| {
            if method != METHOD {
                return None;
            }
            let mut reply = echo(params, Some(&root));
            match mutation {
                "project" => reply["projectUri"] = json!("file:///other.php"),
                "resource" => reply["resourceUri"] = json!("file:///other.json"),
                "other root" => reply["rootUri"] = json!(format!("{}/", uri_of(root.parent().unwrap()))),
                "nested root" => reply["rootUri"] = json!(format!("{}/", uri_of(&root.join("nested")))),
                "remote" => reply["rootUri"] = json!("https://example.invalid/json/"),
                "query" => reply["rootUri"] = json!(format!("{}/?context=other", uri_of(&root))),
                "fragment" => reply["rootUri"] = json!(format!("{}/#other", uri_of(&root))),
                "slash" => reply["rootUri"] = json!(uri_of(&root)),
                "missing" => {
                    reply.as_object_mut().unwrap().remove("rootUri");
                }
                "type" => reply["rootUri"] = json!(false),
                _ => unreachable!(),
            }
            Some(reply)
        }));
        let error = h.mgr.php_json_translation_resource(doc, &resource, Default::default()).await.unwrap_err();
        let expected = if matches!(mutation, "missing" | "type") {
            ide_core::error::ErrorKind::Protocol
        } else {
            ide_core::error::ErrorKind::InvalidInput
        };
        assert_eq!(error.kind, expected, "{mutation}: {error}");
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_refuses_invalid_local_authorities_before_query() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    let (doc, project) = h.open("caller.php", "<?php");
    h.wait_status("phpantom", ServerStatus::Ready).await;
    let directory = h.root.join("directory.json");
    std::fs::create_dir(&directory).unwrap();
    for path in [
        PathBuf::from("relative.json"),
        h.root.join("bad.txt"),
        h.root.join(".json"),
        h.root.join("json/../en.json"),
        directory,
    ] {
        let error = h.mgr.php_json_translation_resource(doc, &path, Default::default()).await.unwrap_err();
        assert_eq!(error.kind, ide_core::error::ErrorKind::InvalidInput, "{path:?}: {error}");
    }
    std::fs::remove_file(project).unwrap();
    assert!(h.mgr.php_json_translation_resource(doc, &h.root.join("json/en.json"), Default::default()).await.is_err());
    assert!(h.launcher.state("phpantom").requested(METHOD).is_empty());
    h.mgr.shutdown_all().await;
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_refuses_symlinks_and_sockets_without_reading_content() {
    use std::os::unix::{fs::symlink, net::UnixListener};
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    let (doc, _) = h.open("caller.php", "<?php");
    h.wait_status("phpantom", ServerStatus::Ready).await;
    let real = h.root.join("real");
    std::fs::create_dir(&real).unwrap();
    let file = real.join("en.json");
    std::fs::write(&file, "{}").unwrap();
    let alias = h.root.join("alias");
    symlink(&real, &alias).unwrap();
    let linked = h.root.join("linked.json");
    symlink(&file, &linked).unwrap();
    let socket = h.root.join("socket.json");
    let _listener = UnixListener::bind(&socket).unwrap();
    for path in [alias.join("en.json"), alias.join("missing/en.json"), linked, socket] {
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            h.mgr.php_json_translation_resource(doc, &path, Default::default()),
        )
        .await
        .unwrap();
        assert_eq!(result.unwrap_err().kind, ide_core::error::ErrorKind::InvalidInput, "{path:?}");
    }
    assert!(h.launcher.state("phpantom").requested(METHOD).is_empty());
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_rejects_changed_sources_roots_and_closed_file_revisions() {
    for mutation in ["caller", "dependency", "close", "disk", "roots", "no change"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let (doc, path) = h.open("caller.php", "<?php");
        let (dependency, _) = h.open("provider.php", "<?php");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let weak = Arc::downgrade(&h.mgr);
        let root = h.root.clone();
        h.launcher.state("phpantom").responder.lock().replace(Arc::new(move |_, method, params| {
            if method != METHOD {
                return None;
            }
            let mgr = weak.upgrade().unwrap();
            match mutation {
                "caller" | "dependency" => mgr.did_change(
                    if mutation == "caller" { doc } else { dependency },
                    2,
                    &[change(rng(0, 5, 0, 5), " // changed")],
                    &|| unreachable!(),
                ),
                "close" => mgr.did_close(dependency),
                "disk" => mgr.php_files_changed(&[], std::slice::from_ref(&path), &[]),
                "roots" => mgr.set_content_roots(vec![root.join("linked")]),
                _ => mgr.set_content_roots(vec![]),
            }
            Some(echo(params, None))
        }));
        let result = h.mgr.php_json_translation_resource(doc, &h.root.join("json/en.json"), Default::default()).await;
        if mutation == "no change" {
            assert!(result.unwrap().root_path().is_none());
        } else {
            assert_eq!(result.unwrap_err().kind, ide_core::error::ErrorKind::OutOfSync, "{mutation}");
        }
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_receipts_expire_after_settings_restart_or_source_changes() {
    for mutation in ["caller", "dependency", "new source", "disk", "roots", "settings", "restart", "resource text"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let (doc, path) = h.open("caller.php", "<?php");
        let (dependency, _) = h.open("provider.php", "<?php");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let resource = h.root.join("json/en.json");
        h.launcher
            .state("phpantom")
            .responder
            .lock()
            .replace(Arc::new(|_, method, params| (method == METHOD).then(|| echo(params, None))));
        let receipt = h.mgr.php_json_translation_resource(doc, &resource, Default::default()).await.unwrap();
        match mutation {
            "caller" | "dependency" => h.mgr.did_change(
                if mutation == "caller" { doc } else { dependency },
                2,
                &[change(rng(0, 5, 0, 5), " // changed")],
                &|| unreachable!(),
            ),
            "new source" => {
                h.open("new.php", "<?php");
            }
            "disk" => h.mgr.php_files_changed(&[], &[path], &[]),
            "roots" => h.mgr.set_content_roots(vec![h.root.join("linked")]),
            "settings" => h.mgr.reconfigure().await,
            "restart" => h.mgr.restart(h.mgr.servers()[0].id).await.unwrap(),
            "resource text" => {
                std::fs::create_dir(resource.parent().unwrap()).unwrap();
                std::fs::write(resource, "{}").unwrap();
            }
            _ => unreachable!(),
        }
        let result = h.mgr.php_json_translation_resource_current(&receipt, Default::default()).await;
        if mutation == "resource text" {
            result.unwrap();
        } else {
            assert_eq!(result.unwrap_err().kind, ide_core::error::ErrorKind::OutOfSync, "{mutation}");
        }
        h.mgr.shutdown_all().await;
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_revalidates_authority_after_reply_and_before_consumption() {
    for during_query in [false, true] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let (doc, _) = h.open("caller.php", "<?php");
        let real = h.root.join("real");
        std::fs::create_dir(&real).unwrap();
        let parent = h.root.join("json");
        let resource = parent.join("en.json");
        let alias = parent.clone();
        h.wait_status("phpantom", ServerStatus::Ready).await;
        h.launcher.state("phpantom").responder.lock().replace(Arc::new(move |_, method, params| {
            if method != METHOD {
                return None;
            }
            if during_query {
                std::os::unix::fs::symlink(&real, &alias).unwrap();
            }
            Some(echo(params, Some(&alias)))
        }));
        let result = h.mgr.php_json_translation_resource(doc, &resource, Default::default()).await;
        if during_query {
            assert!(result.is_err());
        } else {
            let receipt = result.unwrap();
            std::os::unix::fs::symlink(h.root.join("real"), &parent).unwrap();
            assert_eq!(
                h.mgr.php_json_translation_resource_current(&receipt, Default::default()).await.unwrap_err().kind,
                ide_core::error::ErrorKind::OutOfSync
            );
        }
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_cancels_before_query_during_query_and_while_initializing() {
    for stage in ["before", "query", "initialize"] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let state = h.launcher.state("phpantom");
        if stage == "initialize" {
            *state.initialize_gate.lock() = Some(gate.clone());
        }
        state.responder.lock().replace(Arc::new(move |_, method, params| {
            if method != METHOD {
                return None;
            }
            held.hold();
            Some(echo(params, None))
        }));
        let (doc, _) = h.open("caller.php", "<?php");
        if stage != "initialize" {
            h.wait_status("phpantom", ServerStatus::Ready).await;
        }
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        if stage == "before" {
            cancel.cancel();
        }
        let mgr = h.mgr.clone();
        let path = h.root.join("json/en.json");
        let pending = tokio::spawn(async move { mgr.php_json_translation_resource(doc, &path, token).await });
        if stage != "before" {
            eventually(|| gate.entered()).await;
            cancel.cancel();
        }
        let result = tokio::time::timeout(Duration::from_secs(2), pending).await;
        gate.open();
        assert_eq!(result.unwrap().unwrap().unwrap_err().kind, ide_core::error::ErrorKind::Cancelled);
        assert_eq!(state.requested(METHOD).len(), usize::from(stage == "query"));
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_only_retries_explicit_content_modified_for_same_sources() {
    for (code, expected) in [(-32801, 2), (-32800, 1), (-32603, 1)] {
        let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
        let (doc, _) = h.open("caller.php", "<?php");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let state = h.launcher.state("phpantom");
        let attempts = Arc::new(AtomicUsize::new(0));
        let counter = attempts.clone();
        state.fault.lock().replace(Arc::new(move |_, method| {
            (method == METHOD && counter.fetch_add(1, Ordering::SeqCst) == 0)
                .then(|| (code, "held semantic publication".into()))
        }));
        state.responder.lock().replace(Arc::new(|_, method, params| (method == METHOD).then(|| echo(params, None))));
        let result = h.mgr.php_json_translation_resource(doc, &h.root.join("json/en.json"), Default::default()).await;
        assert_eq!(result.is_ok(), code == -32801);
        assert_eq!(state.requested(METHOD).len(), expected);
        if expected == 2 {
            assert_eq!(state.requested(METHOD)[0], state.requested(METHOD)[1]);
        }
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_rejects_settings_changes_while_a_reply_is_held() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    let (doc, _) = h.open("caller.php", "<?php");
    h.wait_status("phpantom", ServerStatus::Ready).await;
    let gate = Arc::new(Gate::default());
    let held = gate.clone();
    h.launcher.state("phpantom").responder.lock().replace(Arc::new(move |_, method, params| {
        if method != METHOD {
            return None;
        }
        held.hold();
        Some(echo(params, None))
    }));
    let mgr = h.mgr.clone();
    let path = h.root.join("json/en.json");
    let pending = tokio::spawn(async move { mgr.php_json_translation_resource(doc, &path, Default::default()).await });
    eventually(|| gate.entered()).await;
    h.mgr.reconfigure().await;
    gate.open();
    assert_eq!(pending.await.unwrap().unwrap_err().kind, ide_core::error::ErrorKind::OutOfSync);
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_does_not_query_when_execution_is_disabled() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    let (doc, _) = h.open("caller.php", "<?php");
    h.wait_status("phpantom", ServerStatus::Ready).await;
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(|_, method, params| (method == METHOD).then(|| echo(params, None))));
    h.mgr.set_execution_allowed(false);
    assert!(h.mgr.php_json_translation_resource(doc, &h.root.join("json/en.json"), Default::default()).await.is_err());
    assert!(state.requested(METHOD).is_empty());
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_resource_does_not_retry_after_execution_is_disabled() {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    let (doc, _) = h.open("caller.php", "<?php");
    h.wait_status("phpantom", ServerStatus::Ready).await;
    let state = h.launcher.state("phpantom");
    let weak = Arc::downgrade(&h.mgr);
    state.fault.lock().replace(Arc::new(move |_, method| {
        if method != METHOD {
            return None;
        }
        weak.upgrade().unwrap().set_execution_allowed(false);
        Some((-32801, "semantic publication retired".into()))
    }));
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        h.mgr.php_json_translation_resource(doc, &h.root.join("json/en.json"), Default::default()),
    )
    .await
    .unwrap();
    assert_eq!(result.unwrap_err().kind, ide_core::error::ErrorKind::OutOfSync);
    assert_eq!(state.requested(METHOD).len(), 1);
    h.mgr.shutdown_all().await;
}
