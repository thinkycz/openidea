//! Accessor spellings follow Laravel reads without masking real PHP members.
use super::*;

#[derive(serde::Deserialize)]
struct AccessorCase {
    name: String,
    model: String,
    caller: String,
    property: String,
    expected_type: String,
    definition_name: String,
    physical: bool,
    ordinary_method: bool,
}

fn target_offset(case: &AccessorCase, model: &str) -> usize {
    if case.physical {
        model.find("$qualityLabel").unwrap() + 1
    } else {
        model.find(&format!("function {}", case.definition_name)).unwrap() + "function ".len()
    }
}

async fn check(f: &Fixture, case: &AccessorCase, model: &str, expected_type: &str, definition_first: bool) {
    let cursor = at(&case.caller, case.caller.find(&format!("->{};", case.property)).unwrap() + 2);
    let definition = async {
        let locations = ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, cursor, GotoKind::Definition)
            .await
            .expect("first accessor definition");
        assert_eq!(locations.len(), 1, "{}: {locations:?}", case.name);
        assert_eq!(*locations[0].path, f.root.join("app/Models/Bakery.php"));
        let point = at(model, target_offset(case, model));
        assert_eq!(locations[0].range, Range::new(point, point), "{}", case.name);
    };
    let hover = async {
        let result = ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, cursor)
            .await
            .unwrap()
            .expect("first accessor hover");
        let expected =
            if case.physical { "public bool $qualityLabel".into() } else { format!("**var** `{expected_type}`") };
        assert!(result.contents.value.contains(&expected), "{}: {}", case.name, result.contents.value);
    };
    if definition_first {
        definition.await;
        hover.await;
    } else {
        hover.await;
        definition.await;
    }
    let completion = ide_host::ops::lsp::lsp_completion(&f.host, "main", RequestId::next(), f.doc, cursor, None)
        .await
        .expect("first accessor completion");
    let item = completion.items.iter().find(|item| item.label == case.property).expect("runtime attribute completion");
    assert_eq!(item.detail.as_deref(), Some(expected_type), "{}", case.name);
    if case.ordinary_method {
        let method = at(&case.caller, case.caller.find("->qualityLabel()").unwrap() + 2);
        let hover =
            ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, method).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("function qualityLabel(): int"), "{}", hover.contents.value);
        let locations =
            ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, method, GotoKind::Definition).await.unwrap();
        assert_eq!(locations.len(), 1);
        assert_eq!(*locations[0].path, f.root.join("app/Models/Bakery.php"));
        let point = at(model, model.find("function qualityLabel").unwrap() + "function ".len());
        assert_eq!(locations[0].range, Range::new(point, point));
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and Laravel 13.37 vendor fixture"]
async fn accessor_aliases_keep_types_completions_origins_and_unsaved_recovery() {
    let cases: Vec<AccessorCase> = serde_json::from_str(include_str!("eloquent_accessor_aliases.json")).unwrap();
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").unwrap()).canonicalize().unwrap();
    for case in cases {
        Fixture::lint(&case.model);
        Fixture::lint(&case.caller);
        for definition_first in [false, true] {
            let extra = vec![
                (
                    "composer.json".into(),
                    r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#
                        .into(),
                ),
                ("app/Models/Bakery.php".into(), case.model.clone()),
            ];
            let (f, ()) = Fixture::new_at_with_fixture_setup(
                &case.caller,
                &extra,
                "app/Probe.php",
                |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
                |_, _| (),
            );
            let outcome = std::panic::AssertUnwindSafe(async {
                let target =
                    ide_host::ops::doc::doc_open(&f.host, "main", f.root.join("app/Models/Bakery.php")).await.unwrap();
                check(&f, &case, &case.model, &case.expected_type, definition_first).await;
                let moved = case.model.replace("<?php", "<?php /* 😀 moved */");
                let changed = moved
                    .replace("return 'real';", "return 7;")
                    .replace(": string {", ": int {")
                    .replace("fn($value): string => 'real'", "fn($value): int => 7")
                    .replace("Attribute<string, never>", "Attribute<int, never>");
                for (model, expected) in [
                    (&moved, case.expected_type.as_str()),
                    (&changed, if case.expected_type == "string" { "int" } else { "bool" }),
                    (&moved, case.expected_type.as_str()),
                ] {
                    Fixture::lint(model);
                    f.project.documents.replace_all(target.id, model).unwrap();
                    check(&f, &case, model, expected, definition_first).await;
                    assert_eq!(std::fs::read_to_string(f.root.join("app/Models/Bakery.php")).unwrap(), case.model);
                    assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, case.caller);
                }
                let server = f.project.lsp.servers().into_iter().find(|server| server.name == "phpantom").unwrap().id;
                ide_host::ops::lsp::lsp_restart(&f.host, "main", server).await.unwrap();
                check(&f, &case, &moved, &case.expected_type, definition_first).await;
                ide_host::ops::doc::doc_close(&f.host, "main", target.id).await.unwrap();
                check(&f, &case, &case.model, &case.expected_type, definition_first).await;
                assert_eq!(std::fs::read_to_string(f.root.join("app/Models/Bakery.php")).unwrap(), case.model);
                assert_eq!(std::fs::read_to_string(f.root.join("app/Probe.php")).unwrap(), case.caller);
            })
            .catch_unwind()
            .await;
            f.finish(outcome).await;
        }
    }
}
