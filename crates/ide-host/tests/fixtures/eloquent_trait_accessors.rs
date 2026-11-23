//! Trait-renamed Eloquent accessors keep their physical declaration origins.
use super::*;

#[derive(serde::Deserialize)]
struct TraitAccessorCase {
    name: String,
    files: std::collections::BTreeMap<String, String>,
    caller: String,
    property: String,
    expected_type: String,
    definition_file: String,
    definition_name: String,
    physical: bool,
}

async fn check(f: &Fixture, case: &TraitAccessorCase, declaration: &str, expected: &str, definition_first: bool) {
    let cursor = at(&case.caller, case.caller.find(&format!("->{};", case.property)).unwrap() + 2);
    let definition = async {
        let token = format!("{}{}", if case.physical { "$" } else { "function " }, case.definition_name);
        let prefix = if case.physical { 1 } else { "function ".len() };
        let point = at(declaration, declaration.find(&token).unwrap() + prefix);
        for offset in [
            case.caller.find(&format!("->{};", case.property)).unwrap() + 2,
            case.caller.find(&format!("->{} =", case.property)).unwrap() + 2,
        ] {
            let locations =
                ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, at(&case.caller, offset), GotoKind::Definition)
                    .await
                    .unwrap();
            assert_eq!(locations.len(), 1, "{}: {locations:?}", case.name);
            assert_eq!(*locations[0].path, f.root.join(&case.definition_file), "{}", case.name);
            assert_eq!(locations[0].range, Range::new(point, point), "{}", case.name);
        }
    };
    let hover = async {
        let result =
            ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, cursor).await.unwrap().unwrap();
        let expected = if case.physical { "public bool $qualityLabel".into() } else { format!("**var** `{expected}`") };
        assert!(result.contents.value.contains(&expected), "{}: {}", case.name, result.contents.value);
    };
    if definition_first {
        definition.await;
        hover.await;
    } else {
        hover.await;
        definition.await;
    }
    let completion =
        ide_host::ops::lsp::lsp_completion(&f.host, "main", RequestId::next(), f.doc, cursor, None).await.unwrap();
    let item = completion.items.iter().find(|item| item.label == case.property).expect("trait attribute completion");
    assert_eq!(item.detail.as_deref(), Some(expected), "{}", case.name);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and Laravel 13.37 vendor fixture"]
async fn renamed_trait_accessors_keep_cross_file_origins_and_unsaved_recovery() {
    let cases: Vec<TraitAccessorCase> = serde_json::from_str(include_str!("eloquent_trait_accessors.json")).unwrap();
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").unwrap()).canonicalize().unwrap();
    for case in cases {
        for source in case.files.values() {
            Fixture::lint(source);
        }
        Fixture::lint(&case.caller);
        for definition_first in [false, true] {
            let mut files: Vec<_> = case.files.iter().map(|(path, source)| (path.clone(), source.clone())).collect();
            files.push((
                "composer.json".into(),
                r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#
                    .into(),
            ));
            let (f, ()) = Fixture::new_at_with_fixture_setup(
                &case.caller,
                &files,
                "app/Probe.php",
                |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
                |_, _| (),
            );
            let outcome = std::panic::AssertUnwindSafe(async {
                let original = &case.files[&case.definition_file];
                let target =
                    ide_host::ops::doc::doc_open(&f.host, "main", f.root.join(&case.definition_file)).await.unwrap();
                check(&f, &case, original, &case.expected_type, definition_first).await;
                let moved = original.replace("<?php", "<?php /* 😀 moved */");
                let changed = moved
                    .replace("return 'real';", "return 7;")
                    .replace(": string", ": int")
                    .replace("=> 'real'", "=> 7")
                    .replace("Attribute<string, never>", "Attribute<int, never>");
                for (source, expected) in [
                    (&moved, case.expected_type.as_str()),
                    (&changed, if case.expected_type == "string" { "int" } else { &case.expected_type }),
                    (&moved, case.expected_type.as_str()),
                ] {
                    Fixture::lint(source);
                    f.project.documents.replace_all(target.id, source).unwrap();
                    check(&f, &case, source, expected, definition_first).await;
                    assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, case.caller);
                }
                let server = f.project.lsp.servers().into_iter().find(|server| server.name == "phpantom").unwrap().id;
                ide_host::ops::lsp::lsp_restart(&f.host, "main", server).await.unwrap();
                check(&f, &case, &moved, &case.expected_type, definition_first).await;
                ide_host::ops::doc::doc_close(&f.host, "main", target.id).await.unwrap();
                check(&f, &case, original, &case.expected_type, definition_first).await;
                for (path, original) in &case.files {
                    assert_eq!(std::fs::read_to_string(f.root.join(path)).unwrap(), *original);
                }
                assert_eq!(std::fs::read_to_string(f.root.join("app/Probe.php")).unwrap(), case.caller);
            })
            .catch_unwind()
            .await;
            f.finish(outcome).await;
        }
    }
}
