#![cfg(unix)]

use futures::FutureExt;
use ide_core::{FileEdit, IdeEvent, LanguageId};
use ide_host::{Host, HostConfig, ops};
use ide_lsp::types::FormatOptions;
use std::{path::PathBuf, sync::Arc, time::Duration};

const SOURCE: &str = "<?php /* 😀 */  echo 1;\n";
const TYPED: &str = "<?php echo 'user😀value';  echo 2;\n";

struct Fixture {
    _temporary: tempfile::TempDir,
    host: Arc<Host>,
    project: Arc<ide_host::Project>,
    path: PathBuf,
    control: PathBuf,
    doc: ide_core::DocId,
    events: Arc<std::sync::Mutex<Vec<IdeEvent>>>,
}

impl Fixture {
    async fn new() -> Self {
        Self::with_extension("php").await
    }

    async fn with_extension(extension: &str) -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("project");
        let control = temporary.path().join("control");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&control).unwrap();
        std::fs::write(root.join("composer.json"), "{}").unwrap();
        let path = root.join(format!("main.{extension}"));
        std::fs::write(&path, SOURCE).unwrap();
        let script = control.join("formatter.py");
        std::fs::write(&script, include_str!("fixtures/format_server.py")).unwrap();
        let mut language = ide_language::LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
        let mut server = language.language_servers[0].clone();
        server.name = "format-fixture".into();
        server.command = std::env::var("OPENSTORM_TEST_PYTHON").unwrap_or_else(|_| "python3".into());
        server.args = vec!["-I".into(), script.to_string_lossy().into_owned(), control.to_string_lossy().into_owned()];
        language.language_servers = vec![server];
        if extension == "txt" {
            language.id = LanguageId::new("plaintext");
            language.extensions = vec!["txt".into()];
            language.first_line = None;
            language.formatter = None;
        }
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = events.clone();
        let host = Arc::new(Host::new(HostConfig {
            config_dir: temporary.path().join("config"),
            data_dir: temporary.path().join("data"),
            local_data_dir: temporary.path().join("local-data"),
            languages: Arc::new(ide_language::LanguageRegistry::from_configs(vec![language])),
            syntax: Arc::default(),
            sink_for: Box::new(move |_| {
                let events = captured.clone();
                Arc::new(move |event| events.lock().unwrap().push(event))
            }),
            owner: |window| window,
            runtime: tokio::runtime::Handle::current(),
            project_environment: false,
        }));
        host.trust.decide(&root, true, false).unwrap();
        let project = host.open_project("main", &root).unwrap();
        let doc = ops::doc::doc_open(&host, "main", path.clone()).await.unwrap().id;
        Self { _temporary: temporary, host, project, path, control, doc, events }
    }

    async fn ready(&self) {
        tokio::time::timeout(Duration::from_secs(15), async {
            while !self.control.join("ready").exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the owned formatter must hold an actual response");
    }

    fn release(&self) {
        std::fs::write(self.control.join("release"), "release").unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn before_commit_formatting_does_not_apply_edits_from_an_older_buffer() {
    for edited in [false, true] {
        let f = Fixture::new().await;
        let outcome = std::panic::AssertUnwindSafe(async {
            let host = f.host.clone();
            let doc = f.doc;
            let input = f.project.documents.snapshot(doc).unwrap();
            let pending = tokio::spawn(async move {
                ops::lsp::lsp_format_edit(
                    &host,
                    "main",
                    doc,
                    input.version,
                    input.path.0,
                    FormatOptions { tab_size: 4, insert_spaces: true, on_save: false, tool: None },
                )
                .await
            });
            f.ready().await;
            if edited {
                f.project.documents.replace_all(f.doc, TYPED).unwrap();
            }
            f.release();
            let plan = pending.await.unwrap();
            let applied = match plan {
                Ok(plan) => ops::lsp::apply_workspace_edit(&f.host, "main", plan).await,
                Err(error) => Err(error),
            };
            if applied.is_ok() {
                ops::doc::doc_save(&f.host, "main", f.doc).await.unwrap();
            }
            if edited {
                assert!(
                    applied.is_err(),
                    "a stale formatter reply must not overwrite a newer managed buffer;buffer={:?},disk={:?}",
                    f.project.documents.snapshot(f.doc).unwrap().text,
                    std::fs::read_to_string(&f.path).unwrap()
                );
                assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, TYPED);
                assert_eq!(std::fs::read_to_string(&f.path).unwrap(), SOURCE);
            } else {
                applied.unwrap();
                assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, SOURCE.replace("  echo", "    echo"));
            }
        })
        .catch_unwind()
        .await;
        f.release();
        f.project.lsp.shutdown_all().await;
        f.host.close_project("main");
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
    }
}

fn options() -> FormatOptions {
    FormatOptions { tab_size: 4, insert_spaces: true, on_save: false, tool: None }
}

async fn mutate(f: &Fixture, change: &str) {
    match change {
        "buffer" => {
            f.project.documents.replace_all(f.doc, TYPED).unwrap();
        }
        "disk" => {
            std::fs::write(&f.path, TYPED).unwrap();
        }
        "config" => {
            std::fs::write(f.path.parent().unwrap().join("composer.json"), "{\"require\":{\"php\":\"^8.4\"}}").unwrap();
        }
        "settings" => {
            f.project
                .settings
                .set(ide_workspace::settings::SettingsScope::Project, "editor.tabSize", serde_json::json!(8))
                .unwrap();
        }
        "overlay" => {
            let other = f.path.parent().unwrap().join("other.php");
            std::fs::write(&other, SOURCE).unwrap();
            let doc = f.project.documents.open(&other).unwrap();
            f.project.documents.replace_all(doc.id, TYPED).unwrap();
        }
        "readonly" => {
            f.project.documents.set_writable(&f.path, false).unwrap();
        }
        "closed" => {
            f.project.documents.close(f.doc);
        }
        "path" => {
            ops::lsp::apply_workspace_edit(
                &f.host,
                "main",
                ide_core::WorkspaceEdit {
                    files: vec![FileEdit::Rename {
                        from: f.path.clone().into(),
                        to: f
                            .path
                            .with_file_name(format!("moved.{}", f.path.extension().unwrap().to_str().unwrap()))
                            .into(),
                    }],
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }
        "server" => {
            let id = f.project.lsp.servers()[0].id;
            ops::lsp::lsp_restart(&f.host, "main", id).await.unwrap();
        }
        "project" => {
            let next = f.path.parent().unwrap().parent().unwrap().join("other-project");
            std::fs::create_dir(&next).unwrap();
            f.host.open_project("main", &next).unwrap();
        }
        _ => panic!("unknown mutation"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn format_plans_reject_changes_during_the_request_and_before_application() {
    for extension in ["php", "txt"] {
        for during in [true, false] {
            for change in
                ["buffer", "disk", "config", "settings", "overlay", "readonly", "closed", "path", "server", "project"]
            {
                let f = Fixture::with_extension(extension).await;
                let outcome = std::panic::AssertUnwindSafe(async {
                    let host = f.host.clone();
                    let input = f.project.documents.snapshot(f.doc).unwrap();
                    let pending = tokio::spawn(async move {
                        ops::lsp::lsp_format_edit(&host, "main", input.id, input.version, input.path.0, options()).await
                    });
                    f.ready().await;
                    if during {
                        mutate(&f, change).await;
                    }
                    f.release();
                    let plan = pending.await.unwrap();
                    if !during {
                        mutate(&f, change).await;
                    }
                    let buffer_before = f.project.documents.snapshot(f.doc).ok().map(|snapshot| snapshot.text);
                    let applied = match plan {
                        Ok(plan) => ops::lsp::apply_workspace_edit(&f.host, "main", plan).await,
                        Err(error) => Err(error),
                    };
                    assert!(applied.is_err(), "accepted {extension}/{change}/during={during}");
                    if change == "path" {
                        assert!(!f.path.exists());
                        assert_eq!(
                            std::fs::read_to_string(f.path.with_file_name(format!("moved.{}", extension))).unwrap(),
                            SOURCE
                        );
                    } else {
                        assert_eq!(
                            std::fs::read_to_string(&f.path).unwrap(),
                            if change == "disk" { TYPED } else { SOURCE }
                        );
                    }
                    assert_eq!(f.project.documents.snapshot(f.doc).ok().map(|snapshot| snapshot.text), buffer_before);
                    if change == "buffer" {
                        assert_eq!(buffer_before.as_deref(), Some(TYPED));
                    }
                })
                .catch_unwind()
                .await;
                f.release();
                f.project.lsp.shutdown_all().await;
                f.host.close_project("main");
                if let Err(panic) = outcome {
                    std::panic::resume_unwind(panic);
                }
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn format_plan_preserves_dirty_input_undo_and_external_conflict_protection() {
    for extension in ["php", "txt"] {
        let f = Fixture::with_extension(extension).await;
        let outcome = std::panic::AssertUnwindSafe(async {
            f.project.documents.replace_all(f.doc, TYPED).unwrap();
            let input = f.project.documents.snapshot(f.doc).unwrap();
            assert!(
                ops::lsp::lsp_format_edit(&f.host, "main", f.doc, input.version - 1, f.path.clone(), options())
                    .await
                    .is_err()
            );
            assert!(
                ops::lsp::lsp_format_edit(
                    &f.host,
                    "main",
                    f.doc,
                    input.version,
                    f.path.with_file_name("wrong.php"),
                    options()
                )
                .await
                .is_err()
            );
            assert!(!f.control.join("ready").exists());
            f.release();
            let plan = ops::lsp::lsp_format_edit(&f.host, "main", f.doc, input.version, f.path.clone(), options())
                .await
                .unwrap();
            assert!(plan.guard.is_some());
            assert!(!plan.preconditions.is_empty());
            assert!(matches!(plan.files[0], FileEdit::Edit { version: Some(_), .. }));
            let mut duplicate = plan.clone();
            duplicate.files.push(plan.files[0].clone());
            assert!(ops::lsp::apply_workspace_edit(&f.host, "main", duplicate).await.is_err());
            let mut overlap = plan.clone();
            let FileEdit::Edit { edits, .. } = &mut overlap.files[0] else { unreachable!() };
            let first = edits[0].clone();
            edits.push(ide_core::TextEdit {
                range: ide_core::Range::new(ide_core::Pos::new(0, 0), ide_core::Pos::new(1, 0)),
                new_text: first.new_text,
            });
            assert!(ops::lsp::apply_workspace_edit(&f.host, "main", overlap).await.is_err());
            assert_eq!(std::fs::read_to_string(&f.path).unwrap(), SOURCE);
            ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.unwrap();
            let formatted = TYPED.replace("  echo", "    echo");
            assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, formatted);
            assert_eq!(std::fs::read_to_string(&f.path).unwrap(), formatted);
            let group = f
                .events
                .lock()
                .unwrap()
                .iter()
                .find_map(|event| match event {
                    IdeEvent::DocPatched { history: Some(history), .. } => Some(history.group.clone()),
                    _ => None,
                })
                .unwrap();
            ops::lsp::restore_reviewed_edit(&f.host, "main", group.clone(), false).await.unwrap();
            assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, TYPED);
            assert_eq!(std::fs::read_to_string(&f.path).unwrap(), SOURCE);
            ops::lsp::restore_reviewed_edit(&f.host, "main", group.clone(), true).await.unwrap();
            assert_eq!(std::fs::read_to_string(&f.path).unwrap(), formatted);
            std::fs::write(&f.path, "external").unwrap();
            assert!(ops::lsp::restore_reviewed_edit(&f.host, "main", group, false).await.is_err());
            assert_eq!(std::fs::read_to_string(&f.path).unwrap(), "external");
            let input = f.project.documents.snapshot(f.doc).unwrap();
            let plan = ops::lsp::lsp_format_edit(&f.host, "main", f.doc, input.version, f.path.clone(), options())
                .await
                .unwrap();
            assert!(ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.is_err());
            assert_eq!(std::fs::read_to_string(&f.path).unwrap(), "external");
        })
        .catch_unwind()
        .await;
        f.release();
        f.project.lsp.shutdown_all().await;
        f.host.close_project("main");
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
    }
}
