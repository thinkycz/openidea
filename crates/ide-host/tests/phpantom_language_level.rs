//! Existing PHP settings through the production host configuration and managed server.
use futures::FutureExt;
use ide_core::{DocId, LanguageId, Pos};
use ide_host::{Host, HostConfig, Project};
use ide_lsp::types::GotoKind;
use ide_workspace::settings::SettingsScope;
use std::{path::PathBuf, sync::Arc, time::Duration};

const SOURCE: &str = "<?php /* 😀 */ new \\DateTime(); array_find([1], static fn($v)=>true);\n";

fn at(text: &str, needle: &str) -> Pos {
    let prefix = &text[..text.find(needle).unwrap()];
    Pos::new(0, prefix.encode_utf16().count() as u32)
}

async fn applied(project: &Project, doc: DocId, source: &str, expected: &str) {
    let mut last = String::new();
    for _ in 0..160 {
        match project.lsp.goto(doc, at(source, "DateTime"), GotoKind::Definition).await {
            Ok(locations) => {
                if let Some(uri) = locations.iter().find_map(|location| location.uri.as_ref()) {
                    match project.lsp.virtual_document(uri).await {
                        Ok(value) if value.description.as_deref().is_some_and(|text| text.contains(expected)) => {
                            let found = project.lsp.goto(doc, at(source, "array_find"), GotoKind::Definition).await;
                            if let Ok(found) = found {
                                assert_eq!(!found.is_empty(), expected == "8.4", "version-gated builtin");
                                return;
                            }
                        }
                        result => last = format!("{result:?}"),
                    }
                }
            }
            Err(error) => last = error.to_string(),
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("PHP target {expected} did not apply: {last}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom166 and the owned PHP syntax-check executable"]
async fn php_setting_restarts_with_selected_target_and_restores_auto_and_dirty_buffers() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("artifacts/engines/0.11.1-openstorm.166/phpantom_lsp"));
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    std::fs::create_dir_all(root.join("packages/nested")).unwrap();
    let root = dunce::canonicalize(root).unwrap();
    let files = [
        ("composer.json", "{\"require\":{\"php\":\"^8.3\"}}"),
        (".phpantom.toml", "[php]\nversion='8.4'\n"),
        ("main.php", SOURCE),
        ("packages/nested/composer.json", "{\"require\":{\"php\":\"^8.2\"}}"),
        ("packages/nested/.phpantom.toml", "[php]\nversion='8.3'\n"),
        ("packages/nested/main.php", SOURCE),
    ];
    for (name, text) in files {
        let path = root.join(name);
        std::fs::write(&path, text).unwrap();
        if name.ends_with(".php") {
            let result = std::process::Command::new(repo.join("artifacts/toolchains/php-8.4.23-static/php"))
                .args(["-n", "-l"])
                .arg(path)
                .output()
                .unwrap();
            assert!(result.status.success());
        } else if name.ends_with(".json") {
            serde_json::from_str::<serde_json::Value>(text).unwrap();
        }
    }
    let mut php = ide_language::LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|server| server.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let host = Arc::new(Host::new(HostConfig {
        project_environment: false,
        config_dir: tmp.path().join("config"),
        data_dir: tmp.path().join("data"),
        local_data_dir: tmp.path().join("local"),
        languages: Arc::new(ide_language::LanguageRegistry::from_configs(vec![php])),
        syntax: Arc::default(),
        sink_for: Box::new(|_| Arc::new(|_| {})),
        owner: |window| window,
        runtime: tokio::runtime::Handle::current(),
    }));
    host.trust.decide(&root, true, false).unwrap();
    let project = host.open_project("main", &root).unwrap();
    let outcome = std::panic::AssertUnwindSafe(async {
        let docs =
            ["main.php", "packages/nested/main.php"].map(|name| project.documents.open(&root.join(name)).unwrap().id);
        for (doc, target) in docs.into_iter().zip(["8.4", "8.3"]) {
            applied(&project, doc, SOURCE, target).await;
        }
        let dirty = SOURCE.replace("/* 😀 */", "/* 😀 unsaved */");
        project.documents.replace_all(docs[0], &dirty).unwrap();
        for (selection, targets) in [("8.1", ["8.1", "8.1"]), ("8.4", ["8.4", "8.4"]), ("", ["8.4", "8.3"])] {
            ide_host::ops::project::settings_set(
                &host,
                "main",
                SettingsScope::Project,
                "languages.php.languageLevel".into(),
                serde_json::to_string(selection).unwrap(),
            )
            .unwrap();
            project.lsp.reconfigure().await;
            for (index, target) in targets.into_iter().enumerate() {
                applied(&project, docs[index], if index == 0 { &dirty } else { SOURCE }, target).await;
            }
            assert_eq!(project.documents.snapshot(docs[0]).unwrap().text, dirty);
            eprintln!("selected {selection:?}: targets {targets:?}, dirty buffer retained");
        }
        project.lsp.restart(project.lsp.servers()[0].id).await.unwrap();
        applied(&project, docs[0], &dirty, "8.4").await;
        applied(&project, docs[1], SOURCE, "8.3").await;
        assert_eq!(project.documents.snapshot(docs[0]).unwrap().text, dirty);
    })
    .catch_unwind()
    .await;
    project.lsp.shutdown_all().await;
    assert!(project.lsp.servers().iter().all(|server| server.pid.is_none()));
    host.close_project("main");
    for (name, text) in files {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text, "original: {name}");
    }
    assert!(outcome.is_ok(), "host language-level acceptance failed");
}
