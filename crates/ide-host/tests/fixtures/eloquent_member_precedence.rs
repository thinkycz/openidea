//! Concrete PHP declarations must win over same-named Eloquent conventions.
use super::*;

const MODEL: &str = r"<?php
namespace App\Models;
class Bakery extends \Illuminate\Database\Eloquent\Model {
    protected $casts = ['quality' => 'bool', 'created_at' => 'datetime'];
    public string $quality = 'real';
    public function quality(): int { return 7; }
    public string $created_at = 'real';
    public function created_at(): int { return 8; }
}
";

fn member_position(text: &str, needle: &str, property: bool) -> Pos {
    at(text, text.find(needle).unwrap() + usize::from(property))
}

async fn check_member(f: &Fixture, model: &str, source: &str, name: &str, method: bool, definition_first: bool) {
    let cursor = at(source, source.find(&format!("->{name}")).unwrap() + 2);
    let expected =
        member_position(model, &if method { format!("{name}(): int") } else { format!("${name} =") }, !method);
    let definition = async {
        let locations = ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, cursor, GotoKind::Definition)
            .await
            .expect("first real-member definition response");
        assert_eq!(locations.len(), 1, "{name} method={method}: {locations:?}");
        assert_eq!(*locations[0].path, f.root.join("app/Models/Bakery.php"));
        assert!(locations[0].uri.is_none());
        assert_eq!(locations[0].range, Range::new(expected, expected), "real PHP declaration wins");
    };
    let hover = async {
        let hover = ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, cursor)
            .await
            .unwrap()
            .expect("real-member hover");
        let signature = if method { format!("function {name}(): int") } else { format!("string ${name}") };
        assert!(hover.contents.value.contains(&signature), "{}", hover.contents.value);
        assert!(!hover.contents.value.contains("source: cast"));
    };
    if definition_first {
        definition.await;
        hover.await;
    } else {
        hover.await;
        definition.await;
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and Laravel 13.37 vendor fixture"]
async fn real_php_members_win_over_cast_and_timestamp_navigation() {
    Fixture::lint(MODEL);
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").unwrap()).canonicalize().unwrap();
    let caller = |name: &str, method: bool| {
        format!(
            "<?php\nuse App\\Models\\Bakery as Bread;\n$bakery = new Bread();\n/* 😀 member */ $bakery->{name}{};\n",
            if method { "()" } else { "" }
        )
    };
    for definition_first in [false, true] {
        let initial = caller("quality", true);
        let extra = vec![
            (
                "composer.json".into(),
                r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#
                    .into(),
            ),
            ("app/Models/Bakery.php".into(), MODEL.into()),
        ];
        let (f, ()) = Fixture::new_at_with_fixture_setup(
            &initial,
            &extra,
            "app/Probe.php",
            |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
            |_, _| (),
        );
        let outcome = std::panic::AssertUnwindSafe(async {
            let path = f.root.join("app/Models/Bakery.php");
            let model = ide_host::ops::doc::doc_open(&f.host, "main", path.clone()).await.unwrap();
            for (name, method) in [("quality", true), ("quality", false), ("created_at", false), ("created_at", true)] {
                let source = caller(name, method);
                Fixture::lint(&source);
                f.project.documents.replace_all(model.id, MODEL).unwrap();
                f.project.documents.replace_all(f.doc, &source).unwrap();
                check_member(&f, MODEL, &source, name, method, definition_first).await;
                let needle = if method { format!("public function {name}") } else { format!("public string ${name}") };
                let moved = MODEL.replace(&needle, &format!("/* 😀 moved */ {needle}"));
                Fixture::lint(&moved);
                f.project.documents.replace_all(model.id, &moved).unwrap();
                check_member(&f, &moved, &source, name, method, definition_first).await;
                assert_eq!(
                    f.project.documents.snapshot(f.doc).unwrap().text,
                    source,
                    "caller stays unchanged during target edit"
                );
                assert_eq!(std::fs::read_to_string(&path).unwrap(), MODEL);
                let server = f.project.lsp.servers().into_iter().find(|s| s.name == "phpantom").unwrap().id;
                ide_host::ops::lsp::lsp_restart(&f.host, "main", server).await.unwrap();
                check_member(&f, &moved, &source, name, method, definition_first).await;
            }
            ide_host::ops::doc::doc_close(&f.host, "main", model.id).await.unwrap();
            let source = caller("created_at", true);
            check_member(&f, MODEL, &source, "created_at", true, definition_first).await;
            assert_eq!(std::fs::read_to_string(&path).unwrap(), MODEL);
            assert_eq!(std::fs::read_to_string(f.root.join("app/Probe.php")).unwrap(), initial);
        })
        .catch_unwind()
        .await;
        f.finish(outcome).await;
    }
}
