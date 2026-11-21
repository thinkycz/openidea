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
async fn laravel_json_provider_source_receipts_are_capability_gated_and_content_free() {
    use sha2::Digest;
    let content = "<?php /* closed provider */";
    let hash = format!("{:x}", sha2::Sha256::digest(content.as_bytes()));
    for advertised in [true, false] {
        let mut capabilities = caps();
        capabilities["experimental"]["phpWorkbenchProviderSources"] = json!(advertised);
        let h = harness_for("php", "php", &[("phpantom", capabilities)], &[]);
        let (caller, _) = h.open("Caller.php", "<?php");
        let provider = h.root.join("Provider.php");
        std::fs::write(&provider, content).unwrap();
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let catalog = h.root.join("catalogs/en.json");
        let resource = catalog.clone();
        let source = provider.clone();
        let source_hash = hash.clone();
        h.launcher.state("phpantom").responder.lock().replace(Arc::new(move |_, method, params| {
            (method == METHOD).then(|| {
                let mut reply = echo(params, None);
                reply["providerSources"] = json!({"providers":[{"className":"App\\Provider","uri":uri_of(&source),"sha256":source_hash}],"unresolved":["Missing\\Provider"],"limited":false});
                if !advertised { reply["providerSources"] = json!("unadvertised malformed metadata is ignored"); }
                assert_eq!(params["resourceUri"], uri_of(&resource));
                reply
            })
        }));
        let receipt = h.mgr.php_json_translation_resource(caller, &catalog, Default::default()).await.unwrap();
        if advertised {
            let providers = receipt.provider_sources().expect("advertised provider-source metadata must be retained");
            assert_eq!(providers.providers.len(), 1);
            assert_eq!(providers.providers[0].class_name, "App\\Provider");
            assert_eq!(providers.providers[0].path, provider);
            assert_eq!(providers.providers[0].sha256_utf8, <[u8; 32]>::from(sha2::Sha256::digest(content.as_bytes())));
            assert_eq!(providers.unresolved, ["Missing\\Provider"]);
            assert!(providers.incomplete());
        } else {
            assert!(receipt.provider_sources().is_none());
        }
        assert!(!catalog.exists());
        assert!(
            h.launcher
                .state("phpantom")
                .notified("textDocument/didOpen")
                .iter()
                .all(|event| event["textDocument"]["uri"] != uri_of(&catalog))
        );
        h.mgr.shutdown_all().await;
    }
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

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_provider_source_receipts_reject_invalid_or_contradictory_metadata() {
    use sha2::Digest;
    for variant in [
        "missing",
        "null",
        "array",
        "text",
        "bad digest",
        "uppercase digest",
        "short digest",
        "digest mismatch",
        "invalid name",
        "duplicate name",
        "conflicting file",
        "unresolved collision",
        "too many",
        "fragment",
        "remote",
        "directory",
        "missing file",
        "oversized file",
    ] {
        let mut capabilities = caps();
        capabilities["experimental"]["phpWorkbenchProviderSources"] = json!(true);
        let h = harness_for("php", "php", &[("phpantom", capabilities)], &[]);
        let (caller, _) = h.open("Caller.php", "<?php");
        h.wait_status("phpantom", ServerStatus::Ready).await;
        let provider = h.root.join("Provider.php");
        let text = "<?php /* source */";
        std::fs::write(&provider, text).unwrap();
        let entry = json!({"className":"App\\Provider","uri":uri_of(&provider),"sha256":format!("{:x}",sha2::Sha256::digest(text.as_bytes()))});
        let mut data = json!({"providers":[entry.clone()],"unresolved":[],"limited":false});
        match variant {
            "missing" | "null" => data = Value::Null,
            "array" => data = json!([]),
            "text" => data["providers"][0]["text"] = json!(text),
            "bad digest" => data["providers"][0]["sha256"] = json!("gg".repeat(32)),
            "uppercase digest" => data["providers"][0]["sha256"] = json!("AB".repeat(32)),
            "short digest" => data["providers"][0]["sha256"] = json!("ab"),
            "digest mismatch" => data["providers"][0]["sha256"] = json!("ab".repeat(32)),
            "invalid name" => data["providers"][0]["className"] = json!("App\\"),
            "duplicate name" => {
                let mut duplicate = entry.clone();
                duplicate["className"] = json!("app\\provider");
                data["providers"].as_array_mut().unwrap().push(duplicate);
            }
            "conflicting file" => {
                let mut duplicate = entry.clone();
                duplicate["className"] = json!("App\\Other");
                duplicate["sha256"] = json!("ab".repeat(32));
                data["providers"].as_array_mut().unwrap().push(duplicate);
            }
            "unresolved collision" => data["unresolved"] = json!(["App\\Provider"]),
            "too many" => data["providers"] = json!(vec![entry.clone(); 513]),
            "fragment" => data["providers"][0]["uri"] = json!(format!("{}#part", uri_of(&provider))),
            "remote" => data["providers"][0]["uri"] = json!("https://example.invalid/Provider.php"),
            "directory" => data["providers"][0]["uri"] = json!(uri_of(&h.root)),
            "missing file" => {
                std::fs::remove_file(&provider).unwrap();
            }
            "oversized file" => {
                std::fs::write(&provider, "x".repeat(1_048_577)).unwrap();
            }
            _ => unreachable!(),
        }
        let state = h.launcher.state("phpantom");
        state.responder.lock().replace(Arc::new(move |_, method, params| {
            (method == METHOD).then(|| {
                let mut reply = echo(params, None);
                if variant != "missing" {
                    reply["providerSources"] = data.clone();
                }
                reply
            })
        }));
        assert!(
            h.mgr
                .php_json_translation_resource(caller, &h.root.join("json/en.json"), Default::default())
                .await
                .is_err(),
            "{variant}"
        );
        assert_eq!(state.notified("textDocument/didOpen").len(), 1, "metadata must not open provider or JSON buffers");
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_provider_source_receipts_track_foreign_buffers_and_disk_revisions() {
    use sha2::Digest;
    for action in ["edit", "restore", "close", "open", "disk", "disk ABA", "replace", "cancel", "restart", "disable"] {
        let mut capabilities = caps();
        capabilities["experimental"]["phpWorkbenchProviderSources"] = json!(true);
        let h = harness_for("php", "php", &[("phpantom", capabilities)], &[]);
        let app = h.root.join("app");
        let dependency = h.root.join("dependency");
        std::fs::create_dir(&app).unwrap();
        std::fs::create_dir(&dependency).unwrap();
        h.mgr.set_content_roots(vec![app.clone(), dependency.clone()]);
        let target = h.launcher.state_for_root("phpantom", &app);
        let foreign = h.launcher.state_for_root("phpantom", &dependency);
        let provider = dependency.join("Provider.php");
        let text = "<?php /* disk and initial buffer */";
        let dirty = "<?php /* changed foreign buffer */";
        std::fs::write(&provider, text).unwrap();
        let (caller, _) = h.open("app/Caller.php", "<?php");
        let doc = DocId::next();
        if action != "open" {
            h.mgr.did_open(doc, &provider, &h.lang, 1, text);
        }
        eventually(|| {
            target.requested("initialize").len() == 1
                && target.docs.lock().contains_key(&uri_of(&app.join("Caller.php")))
        })
        .await;
        let source = provider.clone();
        target.responder.lock().replace(Arc::new(move |_,method,params| (method==METHOD).then(|| {
            let mut reply=echo(params,None);
            reply["providerSources"]=json!({"providers":[{"className":"Shared\\Provider","uri":uri_of(&source),"sha256":format!("{:x}",sha2::Sha256::digest(text.as_bytes()))}],"unresolved":[],"limited":false});reply
        })));
        let receipt = h
            .mgr
            .php_json_translation_resource(caller, &h.root.join("json/en.json"), Default::default())
            .await
            .unwrap();
        h.mgr.php_json_translation_resource_current(&receipt, Default::default()).await.unwrap();
        assert!(!receipt.provider_sources().unwrap().incomplete());
        assert!(!target.docs.lock().contains_key(&uri_of(&provider)), "metadata never forwards foreign buffers");
        match action {
            "edit" | "restore" => {
                h.mgr.did_change(
                    doc,
                    2,
                    &[AppliedChange { range: rng(0, 0, 0, text.encode_utf16().count() as u32), text: dirty.into() }],
                    &|| dirty.into(),
                );
                if action == "restore" {
                    h.mgr.did_change(
                        doc,
                        3,
                        &[AppliedChange {
                            range: rng(0, 0, 0, dirty.encode_utf16().count() as u32),
                            text: text.into(),
                        }],
                        &|| text.into(),
                    );
                }
            }
            "close" => h.mgr.did_close(doc),
            "open" => h.mgr.did_open(doc, &provider, &h.lang, 1, text),
            "disk" => std::fs::write(&provider, dirty).unwrap(),
            "disk ABA" => {
                std::fs::write(&provider, dirty).unwrap();
                tokio::time::sleep(Duration::from_millis(2)).await;
                std::fs::write(&provider, text).unwrap();
            }
            "replace" => {
                let replacement = dependency.join("replacement.php");
                std::fs::write(&replacement, text).unwrap();
                std::fs::rename(replacement, &provider).unwrap();
            }
            "restart" => {
                let id = h.mgr.servers().iter().find(|server| server.root == app).unwrap().id;
                h.mgr.restart(id).await.unwrap();
            }
            "disable" => h.mgr.set_execution_allowed(false),
            "cancel" => {}
            _ => unreachable!(),
        }
        let cancel = CancellationToken::new();
        if action == "cancel" {
            cancel.cancel();
        }
        assert!(h.mgr.php_json_translation_resource_current(&receipt, cancel).await.is_err(), "{action}");
        assert!(!target.docs.lock().contains_key(&uri_of(&provider)));
        if !matches!(action, "close" | "disable" | "open") {
            assert_eq!(foreign.launches.load(Ordering::SeqCst), 1);
        }
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_provider_source_receipts_reject_foreign_buffer_changes_during_query() {
    use sha2::Digest;
    for action in ["edit", "restore", "close", "open"] {
        let mut capabilities = caps();
        capabilities["experimental"]["phpWorkbenchProviderSources"] = json!(true);
        let h = harness_for("php", "php", &[("phpantom", capabilities)], &[]);
        let app = h.root.join("app");
        let dependency = h.root.join("dependency");
        std::fs::create_dir(&app).unwrap();
        std::fs::create_dir(&dependency).unwrap();
        h.mgr.set_content_roots(vec![app.clone(), dependency.clone()]);
        let target = h.launcher.state_for_root("phpantom", &app);
        h.launcher.state_for_root("phpantom", &dependency);
        let provider = dependency.join("Provider.php");
        let text = "<?php /* initial */";
        let dirty = "<?php /* changed */";
        std::fs::write(&provider, text).unwrap();
        let (caller, _) = h.open("app/Caller.php", "<?php");
        let doc = DocId::next();
        if action != "open" {
            h.mgr.did_open(doc, &provider, &h.lang, 1, text);
        }
        eventually(|| target.docs.lock().contains_key(&uri_of(&app.join("Caller.php")))).await;
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let source = provider.clone();
        target.responder.lock().replace(Arc::new(move |_, method, params| {
            (method == METHOD).then(|| {
                held.hold();
                let mut reply = echo(params, None);
                reply["providerSources"] = json!({"providers":[{"className":"Shared\\Provider","uri":uri_of(&source),"sha256":format!("{:x}",sha2::Sha256::digest(text.as_bytes()))}],"unresolved":[],"limited":false});
                reply
            })
        }));
        let mgr = h.mgr.clone();
        let catalog = h.root.join("json/en.json");
        let pending =
            tokio::spawn(async move { mgr.php_json_translation_resource(caller, &catalog, Default::default()).await });
        eventually(|| gate.entered()).await;
        match action {
            "edit" | "restore" => {
                h.mgr.did_change(
                    doc,
                    2,
                    &[AppliedChange { range: rng(0, 0, 0, text.len() as u32), text: dirty.into() }],
                    &|| dirty.into(),
                );
                if action == "restore" {
                    h.mgr.did_change(
                        doc,
                        3,
                        &[AppliedChange { range: rng(0, 0, 0, dirty.len() as u32), text: text.into() }],
                        &|| text.into(),
                    );
                }
            }
            "close" => h.mgr.did_close(doc),
            "open" => h.mgr.did_open(doc, &provider, &h.lang, 1, text),
            _ => unreachable!(),
        }
        gate.open();
        assert!(tokio::time::timeout(Duration::from_secs(3), pending).await.unwrap().unwrap().is_err(), "{action}");
        assert!(!target.docs.lock().contains_key(&uri_of(&provider)));
        h.mgr.shutdown_all().await;
    }
}
