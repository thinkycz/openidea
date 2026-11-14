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
    host.trust.decide(&root, true, false).unwrap();
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
    let edit = ide_host::ops::lsp::lsp_rename(
        &host,
        "main",
        ide_core::RequestId::next(),
        doc.id,
        Pos::new(3, 26),
        "subtotal".into(),
    )
    .await
    .unwrap()
    .edit;
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
    // Upstream's interactive action executor must retain OpenStorm's guarded application path.
    let current = project.documents.snapshot(doc.id).unwrap().text;
    let with_unused = current.replacen("use Demo\\Invoice;\n", "use Demo\\Invoice;\nuse Demo\\Unused;\n", 1);
    assert_ne!(current, with_unused);
    project.documents.replace_all(doc.id, &with_unused).unwrap();
    let actions = ide_host::ops::lsp::lsp_code_actions(
        &host,
        "main",
        doc.id,
        ide_core::Range::new(Pos::new(0, 0), Pos::new(0, 0)),
        vec!["source.organizeImports".into()],
    )
    .await
    .unwrap();
    let action = actions
        .into_iter()
        .find(|action| action.edit.as_ref().is_some_and(|edit| edit.guard.is_some()))
        .expect("guarded PHP Optimize Imports action");
    ide_host::ops::lsp::lsp_execute_code_action(&host, "main", ide_core::RequestId::next(), doc.id, action, vec![])
        .await
        .unwrap();
    let optimized = project.documents.snapshot(doc.id).unwrap();
    assert!(!optimized.text.contains("Demo\\Unused"));
    assert!(optimized.text.contains("use Demo\\Invoice;"));
    assert!(!optimized.dirty);
    assert_eq!(std::fs::read_to_string(&main).unwrap(), optimized.text);
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
    host.trust.decide(&root, true, false).unwrap();
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
    host.trust.decide(&root, true, false).unwrap();
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

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires OPENSTORM_TEST_ENGINE_LARAVEL_ROOT pinned Laravel 13.37.0 fixture"]
async fn framework_source_restart_keeps_host_widening_and_hover_responsive() {
    let root = dunce::canonicalize(PathBuf::from(
        std::env::var_os("OPENSTORM_TEST_ENGINE_LARAVEL_ROOT").expect("engine Laravel fixture"),
    ))
    .unwrap();
    let bin = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable"));
    let tmp = tempfile::tempdir().unwrap();
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
    host.trust.decide(&root, true, false).unwrap();
    let project = host.open_project("main", &root).unwrap();
    project.lsp.set_config_hook(Box::new(move |cfg, _| {
        if cfg.name == "phpantom" {
            cfg.command = bin.to_string_lossy().into_owned();
        }
    }));
    let weak = Arc::downgrade(&project);
    project.lsp.set_candidates_hook(Box::new(move |q, cancel| {
        let found = weak.upgrade().map(|p| p.reference_candidates(q, cancel)).unwrap_or_default();
        eprintln!("native host candidates word={} count={} files={:?}", q.word, found.files.len(), found.files);
        found
    }));
    let request_path = root.join("app/Http/Requests/StoreBakeryRequest.php");
    let trait_path = root.join("vendor/laravel/framework/src/Illuminate/Validation/Concerns/ValidatesAttributes.php");
    let request = std::fs::read_to_string(&request_path).unwrap();
    let trait_source = std::fs::read_to_string(&trait_path).unwrap();
    let doc = project.documents.open(&request_path).unwrap();
    let _trait_doc = project.documents.open(&trait_path).unwrap();
    let position = |needle: &str, extra: u32| {
        let offset = request.find(needle).unwrap();
        let prefix = &request[..offset];
        Pos::new(
            prefix.bytes().filter(|b| *b == b'\n').count() as u32,
            prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32 + extra,
        )
    };
    let at = position("'name');", 2);
    let class = position("StoreBakeryRequest extends", 0);
    let method = position("rules(): array", 0);
    let authorize = position("authorize(): bool", 0);
    let with_validator = position("withValidator(): void", 0);
    let outcome: Result<(), String> = async {
        for cycle in 0..3 {
            if cycle > 0 {
                project
                    .lsp
                    .restart(project.lsp.servers()[0].id)
                    .await
                    .map_err(|e| format!("restart {cycle}: {e:?}"))?;
            }
            let initial = project
                .lsp
                .hover(doc.id, at, Default::default())
                .await
                .map_err(|e| format!("initial hover {cycle}: {e:?}"))?;
            if !initial.is_some_and(|h| h.contents.value.contains("required|string|max:255")) {
                return Err(format!("initial request-field docs missing in host cycle {cycle}"));
            }
            let start = std::time::Instant::now();
            let results = tokio::time::timeout(std::time::Duration::from_secs(15), async {
                tokio::join!(
                    project.lsp.references(doc.id, class, false, true, Default::default()),
                    project.lsp.references(doc.id, authorize, false, true, Default::default()),
                    project.lsp.references(doc.id, method, false, true, Default::default()),
                    project.lsp.references(doc.id, with_validator, false, true, Default::default()),
                    async {
                        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                        project.lsp.hover(doc.id, at, Default::default()).await
                    }
                )
            })
            .await;
            eprintln!("framework host cycle {cycle}: {:?}, settled={}", start.elapsed(), results.is_ok());
            match results {
                Ok((Ok(class_refs), Ok(_authorize_refs), Ok(method_refs), Ok(_validator_refs), Ok(Some(hover))))
                    if !class_refs.locations.is_empty()
                        && !method_refs.locations.is_empty()
                        && hover.contents.value.contains("required|string|max:255") => {}
                other => return Err(format!("framework host cycle {cycle} did not settle: {other:?}")),
            }
        }
        Ok(())
    }
    .await;
    project.lsp.shutdown_all().await;
    host.close_project("main");
    assert_eq!(std::fs::read_to_string(request_path).unwrap(), request);
    assert_eq!(std::fs::read_to_string(trait_path).unwrap(), trait_source);
    assert!(outcome.is_ok(), "{}", outcome.unwrap_err());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires OPENSTORM_TEST_ROUTE_ROOT owned Laravel route-refactoring fixture"]
async fn grouped_route_checked_rename_keeps_php_blade_callers_and_group_prefix() {
    let root =
        dunce::canonicalize(PathBuf::from(std::env::var_os("OPENSTORM_TEST_ROUTE_ROOT").expect("route fixture")))
            .unwrap();
    let bin = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").expect("engine executable"));
    let names = [
        "routes/openstorm_refactor.php",
        "app/OpenStormFixtures/RoutesConsumer.php",
        "app/OpenStormFixtures/RouteMailer.php",
        "resources/views/openstorm-invoice.blade.php",
    ];
    let originals: Vec<_> =
        names.iter().map(|name| (root.join(name), std::fs::read_to_string(root.join(name)).unwrap())).collect();
    let tmp = tempfile::tempdir().unwrap();
    let events = Arc::new(std::sync::Mutex::new(Vec::<ide_core::IdeEvent>::new()));
    let recorded = events.clone();
    let host = Host::new(HostConfig {
        project_environment: false,
        config_dir: tmp.path().join("config"),
        data_dir: tmp.path().join("data"),
        languages: Arc::new(ide_language::LanguageRegistry::load(None).unwrap()),
        syntax: Arc::default(),
        sink_for: Box::new(move |_| {
            let events = recorded.clone();
            Arc::new(move |event| {
                events.lock().unwrap().push(event);
            })
        }),
        owner: |w| w,
        runtime: tokio::runtime::Handle::current(),
    });
    host.trust.decide(&root, true, false).unwrap();
    let project = host.open_project("main", &root).unwrap();
    project.lsp.set_config_hook(Box::new(move |cfg, _| {
        if cfg.name == "phpantom" {
            cfg.command = bin.to_string_lossy().into_owned();
        }
    }));
    let doc = project.documents.open(&originals[1].0).unwrap();
    let text = &originals[1].1;
    let at = |needle: &str| {
        let prefix = &text[..text.find(needle).unwrap()];
        Pos::new(
            prefix.bytes().filter(|b| *b == b'\n').count() as u32,
            prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
        )
    };
    let mut position = at("route('openstorm.billing.invoice'");
    position.col += 10;
    let outcome: Result<(), String> = async {
        let prepared = project.lsp.prepare_rename(doc.id, position).await.map_err(|e| format!("prepare: {e:?}"))?;
        if prepared.is_none() {
            return Err("route rename was not offered".into());
        }
        let refs = project
            .lsp
            .references(doc.id, position, true, true, Default::default())
            .await
            .map_err(|e| format!("references: {e:?}"))?;
        for index in [2, 3] {
            if !refs.locations.iter().any(|loc| loc.path == originals[index].0) {
                return Err(format!("missing route caller {}: {refs:?}", names[index]));
            }
        }
        let collision = ide_host::ops::lsp::lsp_rename(&host, "main", ide_core::RequestId::next(), doc.id, position, "openstorm.billing.list".into()).await;
        if !matches!(&collision, Err(error) if error.message.contains("Another route already uses the new name")) {
            return Err(format!("route collision was not evaluated: {collision:?}"));
        }
        for (needle, destination, reason) in [
            ("route('openstorm.ambiguous'", "openstorm.unique", "more than one declaration"),
            ("route('openstorm-items.show'", "openstorm-items.details", "route identity"),
            ("route('openstorm.billing.invoice'", "other.invoice", "Keep the route group prefix"),
        ] {
            let mut p = at(needle); p.col += 10;
            let result = ide_host::ops::lsp::lsp_rename(&host, "main", ide_core::RequestId::next(), doc.id, p, destination.into()).await;
            if !matches!(&result, Err(error) if error.message.contains(reason)) {
                return Err(format!("expected semantic refusal {reason:?}: {result:?}"));
            }
        }
        let blade = project.documents.open(&originals[3].0).unwrap();
        let unsaved_blade = format!("{}<a>{{{{ route('openstorm.billing.invoice', ['invoice' => 9]) }}}}</a>\n", originals[3].1.replace("=> 7", "=> 9"));
        project.documents.replace_all(blade.id, &unsaved_blade).unwrap();
        let edit = ide_host::ops::lsp::lsp_rename(&host, "main", ide_core::RequestId::next(), doc.id, position, "openstorm.billing.details".into())
            .await
            .map_err(|e| format!("checked rename: {e:?}"))?.edit;
        if edit.guard.is_none() || edit.files.len() != 4 {
            return Err(format!("expected guarded four-file route plan: {edit:?}"));
        }
        if !edit.files.iter().all(|file| matches!(file, ide_core::FileEdit::Edit { path, .. } if originals.iter().any(|(owned, _)| owned == path))) {
            return Err("route plan included a target outside the owned four-file corpus".into());
        }
        let external = format!("{}// external edit in owned fixture\n", originals[2].1);
        std::fs::write(&originals[2].0, &external).unwrap();
        if ide_host::ops::lsp::apply_workspace_edit(&host, "main", edit).await.is_ok() {
            return Err("stale route plan overwrote an external caller edit".into());
        }
        for (index, (path, source)) in originals.iter().enumerate() {
            let expected = if index == 2 { &external } else { source };
            if std::fs::read_to_string(path).unwrap() != *expected { return Err(format!("stale plan changed {}", names[index])); }
        }
        if project.documents.snapshot(blade.id).unwrap().text != unsaved_blade { return Err("stale plan changed an unsaved Blade overlay".into()); }
        std::fs::write(&originals[2].0, &originals[2].1).unwrap();
        let edit = ide_host::ops::lsp::lsp_rename(&host, "main", ide_core::RequestId::next(), doc.id, position, "openstorm.billing.details".into()).await.map_err(|e| format!("fresh checked rename: {e:?}"))?.edit;
        if edit.guard.is_none() || edit.files.len() != 4 || !edit.files.iter().all(|file| matches!(file, ide_core::FileEdit::Edit { path, .. } if originals.iter().any(|(owned, _)| owned == path))) { return Err("fresh route plan has unexpected targets".into()); }
        ide_host::ops::lsp::apply_workspace_edit(&host, "main", edit).await.map_err(|e| format!("apply: {e:?}"))?;
        let expected = text.replace("route('openstorm.billing.invoice'", "route('openstorm.billing.details'");
        if project.documents.snapshot(doc.id).unwrap().text != expected {
            return Err("consumer overlay changed unrelated bytes".into());
        }
        if std::fs::read_to_string(&originals[1].0).unwrap() != expected {
            return Err("reviewed refactoring did not persist the consumer consistently".into());
        }
        for index in [0, 2, 3] {
            let (path, source) = &originals[index];
            let expected = if index == 0 {
                source.replace("name('invoice')", "name('details')")
            } else if index == 3 {
                unsaved_blade.replace("openstorm.billing.invoice", "openstorm.billing.details")
            } else {
                source.replace("openstorm.billing.invoice", "openstorm.billing.details")
            };
            if std::fs::read_to_string(path).unwrap() != expected {
                return Err(format!("wrong persisted edit in {}", names[index]));
            }
        }
        if project.documents.snapshot(blade.id).unwrap().text != unsaved_blade.replace("openstorm.billing.invoice", "openstorm.billing.details") { return Err("Blade overlay did not retain unsaved content and both renamed callers".into()); }
        let route_doc = project.documents.open(&originals[0].0).unwrap();
        let route_source = std::fs::read_to_string(&originals[0].0).unwrap();
        let line = route_source.lines().nth(4).unwrap();
        let col = line.rfind("'details'").unwrap() as u32 + 2;
        let prepared = project.lsp.prepare_rename_details(route_doc.id, Pos::new(4, col)).await.map_err(|e| format!("declaration preparation: {e:?}"))?;
        if prepared.as_ref().and_then(|p| p.placeholder.as_deref()) != Some("openstorm.billing.details") { return Err(format!("grouped declaration lost its semantic name: {prepared:?}")); }
        let group = events.lock().unwrap().iter().rev().find_map(|event| match event {
            ide_core::IdeEvent::DocPatched { history: Some(h), .. } => Some(h.group.clone()), _ => None,
        }).ok_or("missing shared edit history receipt")?;
        ide_host::ops::lsp::restore_reviewed_edit(&host, "main", group.clone(), false).await.map_err(|e| format!("shared undo: {e:?}"))?;
        for (path, source) in &originals {
            if std::fs::read_to_string(path).unwrap() != *source { return Err(format!("shared undo left changed disk target {}", path.display())); }
        }
        let restored = project.documents.snapshot(blade.id).unwrap();
        if restored.text != unsaved_blade || !restored.dirty { return Err("shared undo lost the original unsaved Blade overlay".into()); }
        ide_host::ops::lsp::restore_reviewed_edit(&host, "main", group.clone(), true).await.map_err(|e| format!("shared redo: {e:?}"))?;
        if std::fs::read_to_string(&originals[0].0).unwrap() != originals[0].1.replace("name('invoice')", "name('details')") { return Err("shared redo did not rename closed declaration".into()); }
        if project.documents.snapshot(blade.id).unwrap().text != unsaved_blade.replace("openstorm.billing.invoice", "openstorm.billing.details") { return Err("shared redo lost Blade callers".into()); }
        let changed_mailer = format!("{}// external change after refactoring\n", std::fs::read_to_string(&originals[2].0).unwrap());
        std::fs::write(&originals[2].0, &changed_mailer).unwrap();
        if ide_host::ops::lsp::restore_reviewed_edit(&host, "main", group, false).await.is_ok() { return Err("shared undo overwrote a subsequent external change".into()); }
        if std::fs::read_to_string(&originals[2].0).unwrap() != changed_mailer || project.documents.snapshot(doc.id).unwrap().text != expected { return Err("failed shared undo changed a target".into()); }
        Ok(())
    }
    .await;
    project.lsp.shutdown_all().await;
    host.close_project("main");
    drop(project);
    for (path, source) in &originals {
        std::fs::write(path, source).unwrap();
    }
    assert!(outcome.is_ok(), "{}", outcome.unwrap_err());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn builtin_reopen_refreshes_existing_host_document_after_target_restart() {
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    // Host scratch/index writes must remain outside the watched source tree.
    let project_root = tmp.path().join("project");
    std::fs::create_dir(&project_root).unwrap();
    let root = dunce::canonicalize(&project_root).unwrap();
    let composer = root.join("composer.json");
    std::fs::write(&composer, r#"{"require":{"php":"^8.2"}}"#).unwrap();
    let main = root.join("main.php");
    std::fs::write(&main, "<?php\nnew \\DateTime();\n").unwrap();
    let filesystem_events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let observer = filesystem_events.clone();
    let host = Host::new(HostConfig {
        project_environment: false,
        config_dir: tmp.path().join("config"),
        data_dir: tmp.path().join("data"),
        languages: Arc::new(ide_language::LanguageRegistry::load(None).unwrap()),
        syntax: Arc::default(),
        sink_for: Box::new(move |_| {
            let observer = observer.clone();
            Arc::new(move |event| {
                if let ide_core::IdeEvent::FsChanged { created, modified, removed } = event {
                    observer
                        .lock()
                        .unwrap()
                        .push(format!("created={created:?} modified={modified:?} removed={removed:?}"));
                }
            })
        }),
        owner: |w| w,
        runtime: tokio::runtime::Handle::current(),
    });
    host.trust.decide(&root, true, false).unwrap();
    let project = host.open_project("main", &root).unwrap();
    project.lsp.set_config_hook(Box::new(move |cfg, _| {
        if cfg.name == "phpantom" {
            cfg.command = bin.to_string_lossy().into_owned();
        }
    }));
    let doc = project.documents.open(&main).unwrap();
    let definition_result = project.lsp.goto(doc.id, Pos::new(1, 7), ide_lsp::types::GotoKind::Definition).await;
    let definitions = match definition_result {
        Ok(value) => value,
        Err(error) => {
            project.lsp.shutdown_all().await;
            host.close_project("main");
            panic!(
                "first builtin lookup failed: {error:?}; filesystem events: {:?}",
                filesystem_events.lock().unwrap()
            );
        }
    };
    let uri = definitions.iter().find_map(|d| d.uri.clone()).unwrap();
    let before = ide_host::ops::lsp::lsp_virtual_document(&host, "main", uri.clone()).await.unwrap();
    assert!(before.description.as_deref().unwrap().contains("8.2"));
    // Seed an older server-derived snapshot to exercise replacement of its text too.
    let opened = project.documents.open_virtual(&uri, LanguageId::new("php"), "<?php // old stub snapshot\n").unwrap();
    std::fs::write(&composer, r#"{"require":{"php":"^8.2"},"config":{"platform":{"php":"8.3.0"}}}"#).unwrap();
    project.lsp.restart(project.lsp.servers()[0].id).await.unwrap();
    let after = ide_host::ops::lsp::lsp_virtual_document(&host, "main", uri).await.unwrap();
    let refreshed = project.documents.snapshot(opened.id).unwrap();
    project.lsp.shutdown_all().await;
    host.close_project("main");
    assert!(after.description.as_deref().unwrap().contains("8.3"));
    assert_eq!(refreshed.text, after.text);
    assert!(refreshed.read_only && !refreshed.dirty);
    assert_eq!(refreshed.version, opened.version + 1);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn checked_class_rename_moves_open_psr4_file_and_shared_history() {
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    std::fs::create_dir_all(root.join("src")).unwrap();
    let root = dunce::canonicalize(root).unwrap();
    std::fs::write(root.join("composer.json"), r#"{"autoload":{"psr-4":{"Demo\\":"src/"}}}"#).unwrap();
    let old = root.join("src/Invoice.php");
    let new = root.join("src/Receipt.php");
    let main = root.join("main.php");
    let original = "<?php namespace Demo; class Invoice {}\n";
    let caller = "<?php\nuse Demo\\Invoice;\n$invoice = new Invoice();\n";
    std::fs::write(&old, original).unwrap();
    std::fs::write(&main, caller).unwrap();
    let events = Arc::new(std::sync::Mutex::new(Vec::<ide_core::IdeEvent>::new()));
    let observer = events.clone();
    let host = Host::new(HostConfig {
        project_environment: false,
        config_dir: tmp.path().join("config"),
        data_dir: tmp.path().join("data"),
        languages: Arc::new(ide_language::LanguageRegistry::load(None).unwrap()),
        syntax: Arc::default(),
        sink_for: Box::new(move |_| {
            let events = observer.clone();
            Arc::new(move |event| events.lock().unwrap().push(event))
        }),
        owner: |w| w,
        runtime: tokio::runtime::Handle::current(),
    });
    host.trust.decide(&root, true, false).unwrap();
    let project = host.open_project("main", &root).unwrap();
    project.lsp.set_config_hook(Box::new(move |cfg, _| {
        if cfg.name == "phpantom" {
            cfg.command = bin.to_string_lossy().into_owned();
        }
    }));
    let result: Result<(), String> = async {
        let class = project.documents.open(&old).map_err(|e| format!("open class: {e:?}"))?;
        let doc = project.documents.open(&main).map_err(|e| format!("open caller: {e:?}"))?;
        let unsaved = format!("{original}// Unsaved class documentation.\n");
        project.documents.replace_all(class.id, &unsaved).unwrap();
        let plan = ide_host::ops::lsp::lsp_rename(
            &host,
            "main",
            ide_core::RequestId::next(),
            doc.id,
            Pos::new(2, 17),
            "Receipt".into(),
        )
        .await
        .map_err(|e| format!("rename: {e:?}"))?
        .edit;
        if !plan.files.iter().any(|f| matches!(f,ide_core::FileEdit::Rename{from,to} if **from==old && **to==new)) {
            return Err(format!("missing PSR-4 move: {plan:?}"));
        }
        // A file created after preview must never be overwritten, even with matching text.
        std::fs::write(&new, original).unwrap();
        if ide_host::ops::lsp::apply_workspace_edit(&host, "main", plan.clone()).await.is_ok() {
            return Err("occupied destination accepted".into());
        }
        if std::fs::read_to_string(&main).unwrap() != caller || std::fs::read_to_string(&old).unwrap() != original {
            return Err("failed plan changed another file".into());
        }
        std::fs::remove_file(&new).unwrap();
        ide_host::ops::lsp::apply_workspace_edit(&host, "main", plan).await.map_err(|e| format!("apply: {e:?}"))?;
        if old.exists()
            || project.documents.by_path(&old).is_some()
            || project.documents.by_path(&new) != Some(class.id)
        {
            return Err("open class identity did not move".into());
        }
        let snapshot = project.documents.snapshot(class.id).unwrap();
        if *snapshot.path != new
            || !snapshot.text.contains("class Receipt")
            || !snapshot.text.contains("Unsaved class documentation")
            || snapshot.dirty
        {
            return Err(format!("wrong moved overlay: {snapshot:?}"));
        }
        if std::fs::read_to_string(&new).unwrap() != snapshot.text
            || !std::fs::read_to_string(&main).unwrap().contains("new Receipt()")
        {
            return Err("disk edits missing".into());
        }
        let group = events
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find_map(|e| match e {
                ide_core::IdeEvent::DocPatched { history: Some(h), .. } => Some(h.group.clone()),
                _ => None,
            })
            .ok_or("missing history")?;
        for (redo, target, name) in [(false, &old, "Invoice"), (true, &new, "Receipt")] {
            ide_host::ops::lsp::restore_reviewed_edit(&host, "main", group.clone(), redo)
                .await
                .map_err(|e| format!("restore {redo}: {e:?}"))?;
            let snapshot = project.documents.snapshot(class.id).unwrap();
            if *snapshot.path != *target
                || snapshot.dirty == redo
                || !snapshot.text.contains("Unsaved class documentation")
            {
                return Err(format!("bad restored overlay {redo}: {snapshot:?}"));
            }
            let defs = project
                .lsp
                .goto(doc.id, Pos::new(2, 17), ide_lsp::types::GotoKind::Definition)
                .await
                .map_err(|e| format!("definition after {name}: {e:?}"))?;
            if !defs.iter().any(|d| d.path == *target) {
                return Err(format!("stale definition after {name}: {defs:?}"));
            }
        }
        std::fs::write(&old, "external file\n").unwrap();
        if ide_host::ops::lsp::restore_reviewed_edit(&host, "main", group, false).await.is_ok()
            || std::fs::read_to_string(&old).unwrap() != "external file\n"
            || !new.exists()
        {
            return Err("undo overwrote an occupied source".into());
        }
        Ok(())
    }
    .await;
    project.lsp.shutdown_all().await;
    host.close_project("main");
    result.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn checked_controller_class_rename_keeps_runtime_quotes_aliases_and_file_history() {
    fn at(text: &str, needle: &str) -> Pos {
        let offset = text.find(needle).unwrap() + needle.len();
        let prefix = &text[..offset];
        Pos::new(
            prefix.bytes().filter(|b| *b == b'\n').count() as u32,
            prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
        )
    }
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir(root.join("routes")).unwrap();
    let root = dunce::canonicalize(root).unwrap();
    std::fs::write(
        root.join("composer.json"),
        r#"{"require":{"laravel/framework":"^13.37"},"autoload":{"psr-4":{"App\\":"src/"}}}"#,
    )
    .unwrap();
    let old = root.join("src/InvoiceController.php");
    let new = root.join("src/ReceiptController.php");
    let route_path = root.join("routes/web.php");
    let class =
        "<?php\nnamespace App;\nclass InvoiceController { public function show(): string { return 'invoice'; } }\n";
    let route = r#"<?php
use Illuminate\Support\Facades\Route;
use App\InvoiceController;
use App\OtherController as ReceiptController;
$invoice = new InvoiceController();
$other = new ReceiptController();
Route::namespace('App')->controller('InvoiceController')->group(fn () => Route::get('/bound', 'show'));
Route::get('/😀/direct', "App\\Inv\x6ficeController@show");
"#;
    std::fs::write(&old, class).unwrap();
    std::fs::write(&route_path, route).unwrap();
    std::fs::write(root.join("src/OtherController.php"), "<?php namespace App; class OtherController {}\n").unwrap();
    let events = Arc::new(std::sync::Mutex::new(Vec::<ide_core::IdeEvent>::new()));
    let listener = events.clone();
    let host = Host::new(HostConfig {
        project_environment: false,
        config_dir: tmp.path().join("config"),
        data_dir: tmp.path().join("data"),
        languages: Arc::new(ide_language::LanguageRegistry::load(None).unwrap()),
        syntax: Arc::default(),
        sink_for: Box::new(move |_| {
            let events = listener.clone();
            Arc::new(move |e| events.lock().unwrap().push(e))
        }),
        owner: |w| w,
        runtime: tokio::runtime::Handle::current(),
    });
    host.trust.decide(&root, true, false).unwrap();
    let project = host.open_project("main", &root).unwrap();
    project.lsp.set_config_hook(Box::new(move |cfg, _| {
        if cfg.name == "phpantom" {
            cfg.command = bin.to_string_lossy().into_owned();
        }
    }));
    let result: Result<(), String> = async {
        let class_doc = project.documents.open(&old).unwrap();
        let route_doc = project.documents.open(&route_path).unwrap();
        let unsaved_class = format!("{class}// Unsaved controller documentation.\n");
        let unsaved_route = format!(
            "{route}Route::namespace('App')->group(fn () => Route::get('/unsaved', 'InvoiceController@show'));\n"
        );
        project.documents.replace_all(class_doc.id, &unsaved_class).unwrap();
        project.documents.replace_all(route_doc.id, &unsaved_route).unwrap();
        let control = project
            .lsp
            .goto(route_doc.id, at(&unsaved_route, "controller('In"), ide_lsp::types::GotoKind::Definition)
            .await
            .map_err(|e| format!("binding control: {e:?}"))?;
        if control.len() != 1 || control[0].path.to_path_buf() != old {
            return Err(format!("binding navigation control failed: {control:?}"));
        }
        let plan = ide_host::ops::lsp::lsp_rename(
            &host,
            "main",
            ide_core::RequestId::next(),
            class_doc.id,
            at(class, "class In"),
            "ReceiptController".into(),
        )
        .await
        .map_err(|e| format!("runtime checked rename: {e:?}"))?
        .edit;
        if !plan.files.iter().any(|f| matches!(f,ide_core::FileEdit::Rename{from,to} if **from==old && **to==new)) {
            return Err(format!("missing controller file move: {plan:?}"));
        }
        // A stale disk dependency is refused before moving or writing any target.
        std::fs::write(&route_path, format!("{route}// External route change.\n")).unwrap();
        if ide_host::ops::lsp::apply_workspace_edit(&host, "main", plan.clone()).await.is_ok()
            || new.exists()
            || std::fs::read_to_string(&old).unwrap() != class
        {
            return Err("stale dependency was accepted or partially applied".into());
        }
        std::fs::write(&route_path, route).unwrap();
        let plan = ide_host::ops::lsp::lsp_rename(
            &host,
            "main",
            ide_core::RequestId::next(),
            class_doc.id,
            at(class, "class In"),
            "ReceiptController".into(),
        )
        .await
        .map_err(|e| format!("fresh runtime checked rename: {e:?}"))?
        .edit;
        ide_host::ops::lsp::apply_workspace_edit(&host, "main", plan)
            .await
            .map_err(|e| format!("runtime apply: {e:?}"))?;
        let changed = project.documents.snapshot(route_doc.id).unwrap();
        for expected in [
            "use App\\ReceiptController as ReceiptControllerAlias;",
            "new ReceiptControllerAlias();",
            "new ReceiptController();",
            "controller('ReceiptController')",
            r#""App\\ReceiptController@show""#,
            "'/unsaved', 'ReceiptController@show'",
        ] {
            if !changed.text.contains(expected) {
                return Err(format!("missing correct literal/alias {expected:?}: {}", changed.text));
            }
        }
        if old.exists()
            || project.documents.by_path(&new) != Some(class_doc.id)
            || changed.dirty
            || std::fs::read_to_string(&route_path).unwrap() != changed.text
        {
            return Err("runtime commit did not move/persist both overlays".into());
        }
        // Verify the committed PHP quoting and PSR-4 path with the real Laravel router.
        // This is an explicit test action; editor refactoring stays entirely static.
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let laravel = std::env::var_os("OPENSTORM_TEST_ENGINE_LARAVEL_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("artifacts/checks/engine-laravel13-37-147/project"));
        let php = repo.join("artifacts/toolchains/php-8.4.23-static/php");
        let script = r#"$loader = require $argv[2] . '/vendor/autoload.php';
$loader->addPsr4('App\\', $argv[1] . '/src', true);
$router = new \Illuminate\Routing\Router(new \Illuminate\Events\Dispatcher());
\Illuminate\Support\Facades\Route::swap($router);
require $argv[1] . '/routes/web.php';
$routes = $router->getRoutes();
if (count($routes) !== 3) { throw new \RuntimeException('Expected three renamed routes'); }
foreach ($routes as $route) {
    if ($route->getActionName() !== 'App\\ReceiptController@show'
        || get_class($route->getController()) !== 'App\\ReceiptController'
        || $route->getController()->show() !== 'invoice') {
        throw new \RuntimeException('A renamed runtime controller failed');
    }
}
echo '3 renamed controller routes executed';"#;
        let runtime = std::process::Command::new(&php)
            .args(["-n", "-r", script])
            .arg(&root)
            .arg(&laravel)
            .output()
            .map_err(|e| format!("explicit PHP verification: {e}"))?;
        if !runtime.status.success() || runtime.stdout != b"3 renamed controller routes executed" {
            return Err(format!(
                "committed controller runtime verification failed: {}",
                String::from_utf8_lossy(&runtime.stderr)
            ));
        }
        let group = events
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find_map(|e| match e {
                ide_core::IdeEvent::DocPatched { history: Some(h), .. } => Some(h.group.clone()),
                _ => None,
            })
            .ok_or("missing runtime history")?;
        for redo in [false, true] {
            ide_host::ops::lsp::restore_reviewed_edit(&host, "main", group.clone(), redo)
                .await
                .map_err(|e| format!("runtime history {redo}: {e:?}"))?;
            let current = project.documents.snapshot(route_doc.id).unwrap();
            let expected = if redo { &new } else { &old };
            let needle = if redo { r#"'/😀/direct', "App\\Re"# } else { r#"'/😀/direct', "App\\Inv\x6f"# };
            let definitions = project
                .lsp
                .goto(route_doc.id, at(&current.text, needle), ide_lsp::types::GotoKind::Definition)
                .await
                .map_err(|e| format!("runtime definition {redo}: {e:?}"))?;
            if definitions.len() != 1 || definitions[0].path.to_path_buf() != *expected {
                return Err(format!("stale runtime definition after history {redo}: {definitions:?}"));
            }
            if !redo
                && (current.text != unsaved_route
                    || !current.dirty
                    || project.documents.snapshot(class_doc.id).unwrap().text != unsaved_class
                    || std::fs::read_to_string(&old).unwrap() != class)
            {
                return Err("runtime undo lost original unsaved overlays or disk bytes".into());
            }
        }
        std::fs::write(&new, "<?php // External controller change.\n").unwrap();
        if ide_host::ops::lsp::restore_reviewed_edit(&host, "main", group, false).await.is_ok()
            || std::fs::read_to_string(&route_path).unwrap() != changed.text
        {
            return Err("runtime history overwrote an external change or partially restored callers".into());
        }
        Ok(())
    }
    .await;
    project.lsp.shutdown_all().await;
    host.close_project("main");
    result.unwrap();
}
