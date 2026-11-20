use super::json_translation_resources::{caps, echo};
use super::*;

const METHOD: &str = "phpWorkbench/jsonTranslationResource";
const MANIFEST: &str = r#"{"name":"test/app","require":{"laravel/framework":"13.37.0"}}"#;

fn fixture() -> (Harness, PathBuf, PathBuf) {
    let h = harness_for("php", "php", &[("phpantom", caps())], &[]);
    let manifest = h.root.join("composer.json");
    std::fs::write(&manifest, MANIFEST).unwrap();
    let resource = h.root.join("catalogs/en.json");
    h.launcher
        .state("phpantom")
        .responder
        .lock()
        .replace(Arc::new(|_, method, params| (method == METHOD).then(|| echo(params, None))));
    (h, manifest, resource)
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_keeps_open_json_buffers_on_the_json_server() {
    let h = harness_languages(
        &[("php", "php", &[("phpantom", caps())]), ("json", "json", &[("json-lsp", incremental_caps())])],
        |_| {},
    );
    let manifest = h.root.join("composer.json");
    std::fs::write(&manifest, MANIFEST).unwrap();
    let composer_doc = DocId::next();
    let catalog = h.root.join("catalogs/en.json");
    h.mgr.did_open(composer_doc, &manifest, &LanguageId::new("json"), 1, MANIFEST);
    h.mgr.did_open(DocId::next(), &catalog, &LanguageId::new("json"), 1, r#"{"Secret":"Private unsaved text"}"#);
    h.wait_status("json-lsp", ServerStatus::Ready).await;
    let state = h.launcher.state("phpantom");
    state.responder.lock().replace(Arc::new(|_, method, params| (method == METHOD).then(|| echo(params, None))));
    let receipt =
        h.mgr.php_json_translation_resource_from_composer(&manifest, &catalog, Default::default()).await.unwrap();
    h.mgr.php_json_translation_resource_current(&receipt, Default::default()).await.unwrap();
    assert!(state.notified("textDocument/didOpen").is_empty());
    assert!(state.notified("textDocument/didChange").is_empty());
    assert!(state.docs.lock().is_empty());
    assert!(h.launcher.state("json-lsp").docs.lock().contains_key(&uri_of(&manifest)));
    assert_eq!(std::fs::read_to_string(&manifest).unwrap(), MANIFEST);
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_refuses_unsaved_semantic_changes_before_starting_php() {
    for buffer in [r#"{"name":"changed/app"}"#, "{invalid", "[]"] {
        let (h, manifest, resource) = fixture();
        h.mgr.did_open(DocId::next(), &manifest, &LanguageId::new("json"), 1, buffer);
        assert!(
            h.mgr.php_json_translation_resource_from_composer(&manifest, &resource, Default::default()).await.is_err()
        );
        assert_eq!(h.launcher.state("phpantom").launches.load(Ordering::SeqCst), 0);
        assert!(h.launcher.state("phpantom").requested(METHOD).is_empty());
        assert_eq!(std::fs::read_to_string(manifest).unwrap(), MANIFEST);
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_accepts_normalized_line_endings_and_json_layout() {
    let (h, manifest, resource) = fixture();
    let disk = "{\r\n  \"name\": \"test/app\",\r\n  \"require\": {\"laravel/framework\": \"13.37.0\"}\r\n}\r\n";
    std::fs::write(&manifest, disk).unwrap();
    h.mgr.did_open(DocId::next(), &manifest, &LanguageId::new("json"), 1, MANIFEST);
    let receipt =
        h.mgr.php_json_translation_resource_from_composer(&manifest, &resource, Default::default()).await.unwrap();
    assert!(receipt.root_path().is_none());
    h.mgr.php_json_translation_resource_current(&receipt, Default::default()).await.unwrap();
    assert_eq!(std::fs::read_to_string(manifest).unwrap(), disk);
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_refuses_invalid_manifests_without_launching_php() {
    for text in ["[]".to_owned(), "null".to_owned(), "{invalid".to_owned(), " ".repeat(1_048_577)] {
        let (h, manifest, resource) = fixture();
        std::fs::write(&manifest, text).unwrap();
        let error = h
            .mgr
            .php_json_translation_resource_from_composer(&manifest, &resource, Default::default())
            .await
            .unwrap_err();
        assert_eq!(error.kind, ide_core::error::ErrorKind::InvalidInput);
        assert_eq!(h.launcher.state("phpantom").launches.load(Ordering::SeqCst), 0);
        h.mgr.shutdown_all().await;
    }
    for kind in ["missing", "directory", "wrong filename", "relative"] {
        let (h, manifest, resource) = fixture();
        let selected = match kind {
            "missing" => {
                std::fs::remove_file(&manifest).unwrap();
                manifest
            }
            "directory" => {
                std::fs::remove_file(&manifest).unwrap();
                std::fs::create_dir(&manifest).unwrap();
                manifest
            }
            "wrong filename" => {
                let path = h.root.join("composer.lock");
                std::fs::write(&path, MANIFEST).unwrap();
                path
            }
            _ => PathBuf::from("composer.json"),
        };
        assert!(
            h.mgr.php_json_translation_resource_from_composer(&selected, &resource, Default::default()).await.is_err()
        );
        assert_eq!(h.launcher.state("phpantom").launches.load(Ordering::SeqCst), 0);
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_requires_an_opened_project_root_or_explicit_content_root() {
    let (h, _, _) = fixture();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let manifest = root.join("composer.json");
    let resource = root.join("catalogs/en.json");
    std::fs::write(&manifest, MANIFEST).unwrap();
    assert!(h.mgr.php_json_translation_resource_from_composer(&manifest, &resource, Default::default()).await.is_err());
    assert_eq!(h.launcher.state("phpantom").launches.load(Ordering::SeqCst), 0);
    h.mgr.set_content_roots(vec![root.clone()]);
    h.mgr.php_json_translation_resource_from_composer(&manifest, &resource, Default::default()).await.unwrap();
    assert_eq!(h.launcher.state("phpantom").requested("initialize")[0]["rootUri"], uri_of(&root));
    h.mgr.shutdown_all().await;
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_refuses_aliased_manifests_and_dirty_alias_buffers() {
    use std::os::unix::fs::symlink;
    let (h, manifest, resource) = fixture();
    let alias = h.root.join("alias");
    std::fs::create_dir(&alias).unwrap();
    let linked = alias.join("composer.json");
    symlink(&manifest, &linked).unwrap();
    assert!(h.mgr.php_json_translation_resource_from_composer(&linked, &resource, Default::default()).await.is_err());
    assert_eq!(h.launcher.state("phpantom").launches.load(Ordering::SeqCst), 0);
    h.mgr.did_open(DocId::next(), &linked, &LanguageId::new("json"), 1, r#"{"name":"dirty/alias"}"#);
    assert!(h.mgr.php_json_translation_resource_from_composer(&manifest, &resource, Default::default()).await.is_err());
    assert_eq!(h.launcher.state("phpantom").launches.load(Ordering::SeqCst), 0);
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_retires_receipts_after_disk_and_buffer_changes() {
    for mutation in [
        "disk",
        "deleted",
        "buffer opened",
        "buffer changed",
        "buffer closed",
        "settings",
        "roots",
        "restart",
        "php opened",
    ] {
        let (h, manifest, resource) = fixture();
        let doc = DocId::next();
        if matches!(mutation, "buffer changed" | "buffer closed") {
            h.mgr.did_open(doc, &manifest, &LanguageId::new("json"), 1, MANIFEST);
        }
        let receipt =
            h.mgr.php_json_translation_resource_from_composer(&manifest, &resource, Default::default()).await.unwrap();
        match mutation {
            "disk" => std::fs::write(&manifest, format!("{MANIFEST}\n")).unwrap(),
            "deleted" => std::fs::remove_file(&manifest).unwrap(),
            "buffer opened" => h.mgr.did_open(doc, &manifest, &LanguageId::new("json"), 1, MANIFEST),
            "buffer changed" => h.mgr.did_change(doc, 2, &[change(rng(0, 0, 0, 0), " ")], &|| unreachable!()),
            "buffer closed" => h.mgr.did_close(doc),
            "settings" => h.mgr.reconfigure().await,
            "roots" => h.mgr.set_content_roots(vec![h.root.join("nested")]),
            "restart" => h.mgr.restart(h.mgr.servers()[0].id).await.unwrap(),
            "php opened" => {
                h.open("new.php", "<?php");
            }
            _ => unreachable!(),
        }
        assert_eq!(
            h.mgr.php_json_translation_resource_current(&receipt, Default::default()).await.unwrap_err().kind,
            ide_core::error::ErrorKind::OutOfSync,
            "{mutation}"
        );
        h.mgr.shutdown_all().await;
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_rejects_identical_replacement_with_preserved_mtime() {
    let (h, manifest, resource) = fixture();
    let receipt =
        h.mgr.php_json_translation_resource_from_composer(&manifest, &resource, Default::default()).await.unwrap();
    let mtime = std::fs::metadata(&manifest).unwrap().modified().unwrap();
    let replacement = h.root.join("replacement.json");
    std::fs::write(&replacement, MANIFEST).unwrap();
    std::fs::File::options().write(true).open(&replacement).unwrap().set_modified(mtime).unwrap();
    std::fs::rename(replacement, &manifest).unwrap();
    assert_eq!(
        h.mgr.php_json_translation_resource_current(&receipt, Default::default()).await.unwrap_err().kind,
        ide_core::error::ErrorKind::OutOfSync
    );
    h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_checks_disk_between_explicit_stale_retries() {
    for change_manifest in [false, true] {
        let (h, manifest, resource) = fixture();
        let selected = manifest.clone();
        let attempts = Arc::new(AtomicUsize::new(0));
        let count = attempts.clone();
        let state = h.launcher.state("phpantom");
        state.fault.lock().replace(Arc::new(move |_, method| {
            if method != METHOD || count.fetch_add(1, Ordering::SeqCst) != 0 {
                return None;
            }
            if change_manifest {
                std::fs::write(&selected, format!("{MANIFEST}\n")).unwrap();
            }
            Some((-32801, "retired semantic publication".into()))
        }));
        let result = h.mgr.php_json_translation_resource_from_composer(&manifest, &resource, Default::default()).await;
        if change_manifest {
            assert_eq!(result.unwrap_err().kind, ide_core::error::ErrorKind::OutOfSync);
            assert_eq!(state.requested(METHOD).len(), 1);
        } else {
            assert!(result.unwrap().root_path().is_none());
            assert_eq!(state.requested(METHOD).len(), 2);
            assert_eq!(state.requested(METHOD)[0], state.requested(METHOD)[1]);
        }
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_rejects_input_changes_while_a_reply_is_held() {
    for mutation in ["disk", "buffer", "php", "settings"] {
        let (h, manifest, resource) = fixture();
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
        let selected = manifest.clone();
        let pending = tokio::spawn(async move {
            mgr.php_json_translation_resource_from_composer(&selected, &resource, Default::default()).await
        });
        eventually(|| gate.entered()).await;
        match mutation {
            "disk" => std::fs::write(&manifest, format!("{MANIFEST}\n")).unwrap(),
            "buffer" => h.mgr.did_open(DocId::next(), &manifest, &LanguageId::new("json"), 1, MANIFEST),
            "php" => {
                h.open("new.php", "<?php");
            }
            "settings" => h.mgr.reconfigure().await,
            _ => unreachable!(),
        }
        gate.open();
        assert_eq!(pending.await.unwrap().unwrap_err().kind, ide_core::error::ErrorKind::OutOfSync, "{mutation}");
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_cancels_during_startup_or_query_without_sharing_text() {
    for phase in ["before", "initialize", "query"] {
        let (h, manifest, resource) = fixture();
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let state = h.launcher.state("phpantom");
        if phase == "initialize" {
            *state.initialize_gate.lock() = Some(gate.clone());
        }
        state.responder.lock().replace(Arc::new(move |_, method, params| {
            if method != METHOD {
                return None;
            }
            held.hold();
            Some(echo(params, None))
        }));
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        if phase == "before" {
            cancel.cancel();
        }
        let mgr = h.mgr.clone();
        let pending =
            tokio::spawn(
                async move { mgr.php_json_translation_resource_from_composer(&manifest, &resource, token).await },
            );
        if phase != "before" {
            eventually(|| gate.entered()).await;
            cancel.cancel();
        }
        let result = tokio::time::timeout(Duration::from_secs(2), pending).await;
        gate.open();
        assert_eq!(result.unwrap().unwrap().unwrap_err().kind, ide_core::error::ErrorKind::Cancelled);
        assert_eq!(state.requested(METHOD).len(), usize::from(phase == "query"));
        assert!(state.notified("textDocument/didOpen").is_empty());
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_respects_execution_and_capability_boundaries() {
    for mode in ["disabled", "query-only", "other"] {
        let mut config = caps();
        if mode == "query-only" {
            config["experimental"]["phpWorkbenchRegisteredJsonTranslations"] = json!(false);
        }
        let name = if mode == "other" { "phpactor" } else { "phpantom" };
        let h = harness_for("php", "php", &[(name, config)], &[]);
        let manifest = h.root.join("composer.json");
        std::fs::write(&manifest, MANIFEST).unwrap();
        if mode == "disabled" {
            h.mgr.set_execution_allowed(false);
        }
        assert!(
            h.mgr
                .php_json_translation_resource_from_composer(
                    &manifest,
                    &h.root.join("json/en.json"),
                    Default::default()
                )
                .await
                .is_err()
        );
        assert!(h.launcher.state(name).requested(METHOD).is_empty());
        if mode != "query-only" {
            assert_eq!(h.launcher.state(name).launches.load(Ordering::SeqCst), 0);
        }
        h.mgr.shutdown_all().await;
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_refuses_socket_and_fifo_manifests_without_blocking() {
    for kind in ["socket", "fifo"] {
        let (h, manifest, resource) = fixture();
        std::fs::remove_file(&manifest).unwrap();
        let _listener = if kind == "socket" {
            Some(std::os::unix::net::UnixListener::bind(&manifest).unwrap())
        } else {
            assert!(std::process::Command::new("mkfifo").arg(&manifest).status().unwrap().success());
            None
        };
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            h.mgr.php_json_translation_resource_from_composer(&manifest, &resource, Default::default()),
        )
        .await
        .unwrap();
        assert!(result.is_err());
        assert_eq!(h.launcher.state("phpantom").launches.load(Ordering::SeqCst), 0);
        h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn laravel_json_composer_stops_when_execution_is_disabled_during_initialization() {
    let (h, manifest, resource) = fixture();
    let state = h.launcher.state("phpantom");
    let gate = Arc::new(Gate::default());
    *state.initialize_gate.lock() = Some(gate.clone());
    let mgr = h.mgr.clone();
    let pending = tokio::spawn(async move {
        mgr.php_json_translation_resource_from_composer(&manifest, &resource, Default::default()).await
    });
    eventually(|| gate.entered()).await;
    h.mgr.set_execution_allowed(false);
    let result = tokio::time::timeout(Duration::from_secs(2), pending).await;
    gate.open();
    assert_eq!(result.unwrap().unwrap().unwrap_err().kind, ide_core::error::ErrorKind::OutOfSync);
    assert!(state.requested(METHOD).is_empty());
    h.mgr.shutdown_all().await;
}
