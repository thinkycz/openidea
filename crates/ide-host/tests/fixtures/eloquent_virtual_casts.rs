//! Virtual cast properties and real trait methods have distinct declaration owners.
use super::*;

#[derive(serde::Deserialize)]
struct CastCase {
    name: String,
    model: String,
    base: String,
    target: String,
    value: Option<String>,
    // Trait casts() hover inference is tracked separately from this declaration check.
    #[serde(default)]
    navigation_only: bool,
}

fn php(body: &str) -> String {
    format!("<?php\nnamespace App\\Models;\n/* 😀 owner */ {body}\n")
}

async fn check_cast_and_method(
    f: &Fixture,
    source: &str,
    target: &str,
    casts: &str,
    method: &str,
    definition_first: bool,
    verify_cast_type: bool,
) {
    for property in [true, false] {
        let offset = if property { source.find("->quality;") } else { source.find("->quality()") }.unwrap() + 2;
        let cursor = at(source, offset);
        let (target_path, target_text, target_offset) = if property {
            let marker = "/* target */ ";
            (target, casts, casts.find(marker).unwrap() + marker.len() + 1)
        } else {
            ("app/Models/Value.php", method, method.find("quality(): int").unwrap())
        };
        let expected = at(target_text, target_offset);
        let definition = async {
            let locations = ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, cursor, GotoKind::Definition)
                .await
                .expect("first cast/member definition response");
            assert_eq!(locations.len(), 1, "property={property}: {locations:?}");
            assert_eq!(*locations[0].path, f.root.join(target_path), "cast property keeps its own declaration owner");
            assert!(locations[0].uri.is_none());
            assert_eq!(locations[0].range, Range::new(expected, expected));
        };
        let hover = async {
            let hover = ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, cursor)
                .await
                .unwrap()
                .expect("cast/member hover");
            let expected = if property { "**var** `bool`" } else { "function quality(): int" };
            if !property || verify_cast_type {
                assert!(hover.contents.value.contains(expected), "{}", hover.contents.value);
            } else {
                assert!(hover.contents.value.contains("source: cast"), "{}", hover.contents.value);
            }
        };
        if definition_first {
            definition.await;
            hover.await;
        } else {
            hover.await;
            definition.await;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and Laravel 13.37 vendor fixture"]
async fn cast_properties_keep_effective_origins_beside_real_trait_methods() {
    let cases: Vec<CastCase> = serde_json::from_str(include_str!("eloquent_virtual_casts.json")).unwrap();
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").unwrap()).canonicalize().unwrap();
    let source = "<?php\nuse App\\Models\\Bakery as Bread;\n$bakery = new Bread();\n/* 😀 property */ $bakery->quality;\n/* 😀 method */ $bakery->quality();\n";
    Fixture::lint(source);
    for case in cases {
        let method =
            php(case.value.as_deref().unwrap_or("trait Value { public function quality(): int { return 7; } }"));
        Fixture::lint(&method);
        let model = php(&case.model);
        let base = php(&case.base);
        Fixture::lint(&model);
        Fixture::lint(&base);
        for definition_first in [false, true] {
            let extra = vec![
                (
                    "composer.json".into(),
                    r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#
                        .into(),
                ),
                ("app/Models/Bakery.php".into(), model.clone()),
                ("app/Models/Base.php".into(), base.clone()),
                ("app/Models/Value.php".into(), method.clone()),
            ];
            let (f, ()) = Fixture::new_at_with_fixture_setup(
                source,
                &extra,
                "app/Probe.php",
                |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
                |_, _| (),
            );
            let outcome = std::panic::AssertUnwindSafe(async {
                let original = match case.target.as_str() {
                    "app/Models/Bakery.php" => &model,
                    "app/Models/Base.php" => &base,
                    "app/Models/Value.php" => &method,
                    target => panic!("unexpected cast source {target}"),
                };
                let target = ide_host::ops::doc::doc_open(&f.host, "main", f.root.join(&case.target)).await.unwrap();
                check_cast_and_method(
                    &f,
                    source,
                    &case.target,
                    original,
                    &method,
                    definition_first,
                    !case.navigation_only,
                )
                .await;
                let moved = original.replace("/* target */", "/* 😀 moved */ /* target */");
                Fixture::lint(&moved);
                f.project.documents.replace_all(target.id, &moved).unwrap();
                let moved_method = if case.target.ends_with("Value.php") { &moved } else { &method };
                check_cast_and_method(
                    &f,
                    source,
                    &case.target,
                    &moved,
                    moved_method,
                    definition_first,
                    !case.navigation_only,
                )
                .await;
                assert_eq!(
                    f.project.documents.snapshot(f.doc).unwrap().text,
                    source,
                    "{}: caller is unchanged",
                    case.name
                );
                assert_eq!(std::fs::read_to_string(f.root.join(&case.target)).unwrap(), *original);
                let server = f.project.lsp.servers().into_iter().find(|s| s.name == "phpantom").unwrap().id;
                ide_host::ops::lsp::lsp_restart(&f.host, "main", server).await.unwrap();
                let moved_method = if case.target.ends_with("Value.php") { &moved } else { &method };
                check_cast_and_method(
                    &f,
                    source,
                    &case.target,
                    &moved,
                    moved_method,
                    definition_first,
                    !case.navigation_only,
                )
                .await;
                ide_host::ops::doc::doc_close(&f.host, "main", target.id).await.unwrap();
                check_cast_and_method(
                    &f,
                    source,
                    &case.target,
                    original,
                    &method,
                    definition_first,
                    !case.navigation_only,
                )
                .await;
                assert_eq!(std::fs::read_to_string(f.root.join("app/Probe.php")).unwrap(), source);
                assert_eq!(std::fs::read_to_string(f.root.join(&case.target)).unwrap(), *original);
            })
            .catch_unwind()
            .await;
            f.finish(outcome).await;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and Laravel 13.37 vendor fixture"]
async fn cast_properties_keep_existing_setter_targets_on_reads_and_writes() {
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").unwrap()).canonicalize().unwrap();
    for (property, method) in [("quality", "setQualityAttribute"), ("qualityLabel", "setQualityLabelAttribute")] {
        let model = php(&format!(
            "class Bakery extends \\Illuminate\\Database\\Eloquent\\Model {{ protected $casts = ['{property}' => 'bool']; public function {method}($value): void {{ $this->attributes['{property}'] = $value; }} }}"
        ));
        let source = format!(
            "<?php\n$bakery = new \\App\\Models\\Bakery();\n/* 😀 read */ $bakery->{property};\n/* 😀 write */ $bakery->{property} = '1';\n"
        );
        Fixture::lint(&model);
        Fixture::lint(&source);
        let (f, ()) = Fixture::new_at_with_fixture_setup(
            &source,
            &[
                (
                    "composer.json".into(),
                    r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#
                        .into(),
                ),
                ("app/Models/Bakery.php".into(), model.clone()),
            ],
            "app/Probe.php",
            |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
            |_, _| (),
        );
        let outcome = std::panic::AssertUnwindSafe(async {
            let target =
                ide_host::ops::doc::doc_open(&f.host, "main", f.root.join("app/Models/Bakery.php")).await.unwrap();
            let moved = model.replace("public function", "/* 😀 moved */ public function");
            for text in [&model, &moved] {
                f.project.documents.replace_all(target.id, text).unwrap();
                let token = format!("->{property}");
                for (offset, _) in source.match_indices(&token) {
                    let locations = ide_host::ops::lsp::lsp_goto(
                        &f.host,
                        "main",
                        f.doc,
                        at(&source, offset + 2),
                        GotoKind::Definition,
                    )
                    .await
                    .unwrap();
                    assert_eq!(locations.len(), 1);
                    assert_eq!(*locations[0].path, f.root.join("app/Models/Bakery.php"));
                    let expected = at(text, text.find(method).unwrap());
                    assert_eq!(
                        locations[0].range,
                        Range::new(expected, expected),
                        "{property}: existing setter keeps read/write priority"
                    );
                }
                assert_eq!(std::fs::read_to_string(f.root.join("app/Models/Bakery.php")).unwrap(), model);
                assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, source);
            }
        })
        .catch_unwind()
        .await;
        f.finish(outcome).await;
    }
}
