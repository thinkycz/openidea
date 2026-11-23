//! Unsupported virtual-property transformations must never reach the edit applier.
use super::*;

fn observe(f: &Fixture, case: &serde_json::Value, property: &str) -> serde_json::Value {
    let quote = |value: &str| serde_json::to_string(value).unwrap();
    let mut script = format!(
        "$loader=require {};$loader->addPsr4('App\\\\',{},true);",
        quote(f.root.join("vendor/autoload.php").to_str().unwrap()),
        quote(f.root.join("app").to_str().unwrap())
    );
    for path in case["files"].as_object().unwrap().keys() {
        script.push_str(&format!("require_once {};", quote(f.root.join(path).to_str().unwrap())));
    }
    script.push_str("$m=new \\App\\Models\\Bakery();");
    if case["setter"] == true {
        script.push_str(&format!("$m->{property}='anything';"));
    } else if case["relation"] != true && case["defaults"] != true {
        script.push_str(&format!(
            "if(method_exists($m,'setRawAttributes'))$m->setRawAttributes([{}=>'1']);",
            quote(case["property"].as_str().unwrap())
        ));
    }
    if case["local_only"] == true {
        script.push_str("$v=$m->readLocal();$state=['type'=>get_debug_type($v),'value'=>$v];");
    } else {
        script.push_str(&format!("$state=['type'=>get_debug_type($m->{property}),'value'=>$m->{property}];"));
    }
    if case["marker"] == true {
        script.push_str("$state['marker']=$m->getAttributes()['written']??null;");
    }
    if case["local"] == true {
        script.push_str("$state['local']=$m->readLocal();");
    }
    script.push_str("echo json_encode($state);");
    let php = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHP_BIN").unwrap());
    let output = std::process::Command::new(php).args(["-n", "-r", &script]).output().unwrap();
    assert!(output.status.success(), "{}: {}", case["name"], String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and Laravel vendor fixture"]
async fn virtual_property_rename_refusals_and_supported_edits_preserve_runtime() {
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!("eloquent_property_rename.json")).unwrap();
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").unwrap()).canonicalize().unwrap();
    for case in cases {
        let files: Vec<(String, String)> =
            case["files"].as_object().unwrap().iter().map(|(p, s)| (p.clone(), s.as_str().unwrap().into())).collect();
        let caller = case["caller"].as_str().unwrap();
        let property = case["property"].as_str().unwrap();
        let (f, ()) = Fixture::new_at_with_fixture_setup(
            caller,
            &files,
            "app/Probe.php",
            |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
            |_, _| (),
        );
        let outcome = std::panic::AssertUnwindSafe(async {
            let mut opened = vec![("app/Probe.php".to_owned(), f.doc, caller.to_owned())];
            for (path, source) in &files {
                let doc = ide_host::ops::doc::doc_open(&f.host, "main", f.root.join(path)).await.unwrap();
                opened.push((path.clone(), doc.id, source.clone()));
            }
            // A dirty UTF-16 caller must be retained on refusal and in reviewed undo.
            let dirty = caller.replace("<?php", "<?php /* 😀 unsaved */");
            f.project.documents.replace_all(f.doc, &dirty).unwrap();
            opened[0].2 = dirty.clone();
            let query_path = case["query_file"].as_str().unwrap_or("app/Probe.php");
            let (_, doc, query) = opened.iter().find(|(p, _, _)| p == query_path).unwrap();
            let cursor = at(query, query.find(&format!("->{property}")).unwrap() + 2);
            let allowed = case["allowed_new_names"].as_array().unwrap();
            let prepared = ide_host::ops::lsp::lsp_prepare_rename_details(&f.host, "main", *doc, cursor).await.unwrap();
            assert_eq!(prepared.is_some(), !allowed.is_empty(), "{} prepare", case["name"]);
            let before = observe(&f, &case, property);
            for name in ["renamedQualityLabel", "renamed_quality_label"] {
                let permitted = allowed.iter().any(|v| v.as_str() == Some(name));
                // Call rename directly too: prepare refusal must not be the only protection.
                let result =
                    ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), *doc, cursor, name.into()).await;
                if permitted {
                    let plan = result.unwrap().edit;
                    assert!(!plan.files.is_empty(), "{} {name}", case["name"]);
                    assert!(plan.guard.is_some());
                    ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.unwrap();
                    assert_eq!(observe(&f, &case, name), before, "{} {name}", case["name"]);
                    for (path, _, _) in &opened {
                        Fixture::lint(&std::fs::read_to_string(f.root.join(path)).unwrap());
                    }
                    let group = f
                        .events
                        .lock()
                        .unwrap()
                        .iter()
                        .rev()
                        .find_map(|event| match event {
                            ide_core::IdeEvent::DocPatched { history: Some(h), .. } => Some(h.group.clone()),
                            _ => None,
                        })
                        .expect("reviewed property rename history");
                    for redo in [false, true, false] {
                        ide_host::ops::lsp::restore_reviewed_edit(&f.host, "main", group.clone(), redo).await.unwrap();
                        assert_eq!(observe(&f, &case, if redo { name } else { property }), before);
                    }
                } else {
                    let result = result.unwrap();
                    assert!(result.edit.files.is_empty(), "{} {name}: {:?}", case["name"], result.edit);
                }
                for (path, id, source) in &opened {
                    assert_eq!(f.project.documents.snapshot(*id).unwrap().text, *source, "{} {path}", case["name"]);
                    let disk = if path == "app/Probe.php" { caller } else { case["files"][path].as_str().unwrap() };
                    assert_eq!(std::fs::read_to_string(f.root.join(path)).unwrap(), disk);
                }
                assert!(f.project.documents.snapshot(f.doc).unwrap().dirty);
            }
        })
        .catch_unwind()
        .await;
        f.finish(outcome).await;
    }
}
