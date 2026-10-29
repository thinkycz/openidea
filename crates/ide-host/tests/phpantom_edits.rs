//! Pinned PHPantom through the actual host document, checked-plan and commit paths.
use ide_core::{LanguageId, Pos};
use ide_host::{Host, HostConfig};
use std::{path::PathBuf, sync::Arc};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn checked_rename_refreshes_a_closed_declaration_after_commit() {
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    std::fs::create_dir_all(root.join("src")).unwrap();
    let root = dunce::canonicalize(root).unwrap();
    std::fs::write(root.join("composer.json"), r#"{"autoload":{"psr-4":{"Demo\\":"src/"}}}"#).unwrap();
    let declaration = root.join("src/Invoice.php");
    let main = root.join("main.php");
    std::fs::write(
        &declaration,
        "<?php namespace Demo; class Invoice { public function total(): int { return 1; } }\n",
    )
    .unwrap();
    std::fs::write(&main, "<?php\nuse Demo\\Invoice;\n$invoice = new Invoice();\n$label = '😀'; $invoice->total();\n")
        .unwrap();
    let host = Host::new(HostConfig {
        project_environment: false,
        config_dir: tmp.path().join("config"),
        data_dir: tmp.path().join("data"),
        languages: Arc::new(ide_language::LanguageRegistry::load(None).unwrap()),
        syntax: Arc::default(),
        sink_for: Box::new(|_| Arc::new(|_| {})),
        owner: |w| w,
        runtime: tokio::runtime::Handle::current(),
    });
    let project = host.open_project("main", &root).unwrap();
    project.lsp.set_config_hook(Box::new(move |cfg, _| {
        if cfg.name == "phpantom" {
            cfg.command = bin.to_string_lossy().into_owned();
        }
    }));
    let doc = project.documents.open(&main).unwrap();
    assert_eq!(doc.language, LanguageId::new("php"));
    // The definition remains a disk file throughout, rather than an editor overlay.
    assert!(project.documents.by_path(&declaration).is_none());
    let edit = ide_host::ops::lsp::lsp_rename(&host, "main", doc.id, Pos::new(3, 26), "subtotal".into()).await.unwrap();
    assert_eq!(edit.files.len(), 2);
    ide_host::ops::lsp::apply_workspace_edit(&host, "main", edit).await.unwrap();
    assert!(std::fs::read_to_string(&declaration).unwrap().contains("function subtotal()"));
    assert!(project.documents.snapshot(doc.id).unwrap().text.contains("😀'; $invoice->subtotal()"));
    let definitions = project.lsp.goto(doc.id, Pos::new(3, 27), ide_lsp::types::GotoKind::Definition).await.unwrap();
    assert!(
        definitions.iter().any(|d| d.path == dunce::canonicalize(&declaration).unwrap()),
        "closed-file declaration must refresh without restarting: {definitions:?}"
    );
    assert!(project.documents.by_path(&declaration).is_none());
    project.lsp.shutdown_all().await;
    host.close_project("main");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn json_translation_first_open_starts_phpantom_and_finds_php_callers() {
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    std::fs::create_dir_all(root.join("lang")).unwrap();
    std::fs::create_dir_all(root.join("app")).unwrap();
    let root = dunce::canonicalize(root).unwrap();
    std::fs::write(
        root.join("composer.json"),
        r#"{"require":{"laravel/framework":"12.*"},"autoload":{"psr-4":{"App\\":"app/"}}}"#,
    )
    .unwrap();
    let json = r#"{"Note":"😀","Hello invoice":"Invoice greeting"}"#;
    std::fs::write(root.join("lang/en.json"), json).unwrap();
    let position = Pos::new(0, json[..json.find("Hello invoice").unwrap()].encode_utf16().count() as u32 + 2);
    for name in ["first.php", "app/second.php"] {
        std::fs::write(root.join(name), "<?php\n__('Hello invoice');\n").unwrap();
    }
    let host = Host::new(HostConfig {
        project_environment: false,
        config_dir: tmp.path().join("config"),
        data_dir: tmp.path().join("data"),
        languages: Arc::new(ide_language::LanguageRegistry::load(None).unwrap()),
        syntax: Arc::default(),
        sink_for: Box::new(|_| Arc::new(|_| {})),
        owner: |w| w,
        runtime: tokio::runtime::Handle::current(),
    });
    let project = host.open_project("main", &root).unwrap();
    project.lsp.set_config_hook(Box::new(move |cfg, _| {
        if cfg.name == "phpantom" {
            cfg.command = bin.to_string_lossy().into_owned();
        }
    }));
    let doc = project.documents.open(&root.join("lang/en.json")).unwrap();
    assert_eq!(doc.language, LanguageId::new("json"));
    let refs =
        ide_host::ops::lsp::lsp_references(&host, "main", ide_core::RequestId::next(), doc.id, position, false, true)
            .await
            .unwrap();
    assert!(refs.locations.iter().any(|l| l.path.ends_with("first.php")), "{refs:?}");
    assert!(refs.locations.iter().any(|l| l.path.ends_with("app/second.php")), "{refs:?}");
    assert!(project.documents.by_path(&root.join("first.php")).is_none());
    assert!(project.documents.by_path(&root.join("app/second.php")).is_none());
    project.lsp.shutdown_all().await;
    host.close_project("main");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn environment_first_open_starts_phpantom_and_finds_php_callers() {
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    std::fs::create_dir_all(root.join("app")).unwrap();
    let root = dunce::canonicalize(root).unwrap();
    std::fs::write(
        root.join("composer.json"),
        r#"{"require":{"laravel/framework":"12.*"},"autoload":{"psr-4":{"App\\":"app/"}}}"#,
    )
    .unwrap();
    let environment = "OPENSTORM_DEMO_LABEL=sample\nOPENSTORM_DEMO_MODE=local\n";
    std::fs::write(root.join(".env"), environment).unwrap();
    let position = Pos::new(0, 5);
    for name in ["first.php", "app/second.php"] {
        std::fs::write(root.join(name), "<?php\nenv('OPENSTORM_DEMO_LABEL');\n").unwrap();
    }
    let host = Host::new(HostConfig {
        project_environment: false,
        config_dir: tmp.path().join("config"),
        data_dir: tmp.path().join("data"),
        languages: Arc::new(ide_language::LanguageRegistry::load(None).unwrap()),
        syntax: Arc::default(),
        sink_for: Box::new(|_| Arc::new(|_| {})),
        owner: |w| w,
        runtime: tokio::runtime::Handle::current(),
    });
    let project = host.open_project("main", &root).unwrap();
    project.lsp.set_config_hook(Box::new(move |cfg, _| {
        if cfg.name == "phpantom" {
            cfg.command = bin.to_string_lossy().into_owned();
        }
    }));
    let doc = project.documents.open(&root.join(".env")).unwrap();
    assert_eq!(doc.language, LanguageId::new("dotenv"));
    let refs =
        ide_host::ops::lsp::lsp_references(&host, "main", ide_core::RequestId::next(), doc.id, position, false, true)
            .await
            .unwrap();
    assert!(refs.locations.iter().any(|l| l.path.ends_with("first.php")), "{refs:?}");
    assert!(refs.locations.iter().any(|l| l.path.ends_with("app/second.php")), "{refs:?}");
    assert!(project.documents.by_path(&root.join("first.php")).is_none());
    assert!(project.documents.by_path(&root.join("app/second.php")).is_none());
    assert_eq!(std::fs::read_to_string(root.join(".env")).unwrap(), environment);
    project.lsp.shutdown_all().await;
    host.close_project("main");
}
