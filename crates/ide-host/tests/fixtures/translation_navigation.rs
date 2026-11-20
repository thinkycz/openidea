//! Typed translation navigation through host commands and managed dirty documents.
use super::*;

const CALLER: &str = r"<?php namespace App\TranslationHost;
use App\Shared\Selected as Text;
class Foreign {public function get($key) {return 'foreign';}}
function render(Text $text, Foreign $foreign) { /* 😀 */ return [$text->get('messages.hello'), $foreign->get('messages.hello')]; }
";
const EN: &str = "<?php /* 😀 */ return ['hello'=>'Hello'];";
const FR: &str = "<?php return ['hello'=>'Bonjour'];";
const HELPER: &str = "<?php /* 😀 */ return trans('messages.hello');";
const SELECTED: &str = r"<?php namespace App\Shared; class Selected extends \Illuminate\Translation\Translator {}";
const FOREIGN: &str = r"<?php namespace App\Shared; class Selected {public function get($key) {return 'foreign';}}";

fn key_range(text: &str, key: &str) -> Range {
    let start = text.find(key).unwrap();
    Range::new(at(text, start), at(text, start + key.len()))
}

async fn targets(f: &Fixture, locales: &[&str]) {
    let position = key_range(CALLER, "messages.hello").start;
    let mut found: Vec<_> = ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, position, GotoKind::Definition)
        .await
        .unwrap()
        .into_iter()
        .map(|target| {
            assert!(target.uri.is_none());
            (target.path.to_path_buf(), target.range)
        })
        .collect();
    let mut expected: Vec<_> = locales
        .iter()
        .map(|locale| {
            (
                f.root.join(format!("lang/{locale}/messages.php")),
                key_range(if *locale == "en" { EN } else { FR }, "hello"),
            )
        })
        .collect();
    found.sort();
    expected.sort();
    assert_eq!(found, expected, "managed typed translation must retain exact host targets");
    let hover = ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, position).await.unwrap();
    if locales.is_empty() {
        assert!(hover.is_none(), "{hover:?}");
    } else {
        let hover = hover.expect("managed typed translation hover");
        assert_eq!(hover.range, Some(key_range(CALLER, "messages.hello")));
        assert_eq!(hover.contents.value.contains("Hello"), locales.contains(&"en"));
        assert_eq!(hover.contents.value.contains("Bonjour"), locales.contains(&"fr"));
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and Laravel 13.37 vendor fixture"]
async fn typed_translation_host_navigation_tracks_managed_dependencies_catalogs_and_restart() {
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").expect("Laravel vendor fixture"))
        .canonicalize()
        .unwrap();
    let installed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(vendor.join("composer/installed.json")).unwrap()).unwrap();
    assert!(
        installed["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "laravel/framework" && p["version"] == "v13.37.0")
    );
    let extra = vec![
        (
            "composer.json".to_owned(),
            r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#
                .to_owned(),
        ),
        ("lang/en/messages.php".into(), EN.into()),
        ("lang/fr/messages.php".into(), FR.into()),
        ("app/Shared/Selected.php".into(), SELECTED.into()),
        ("helper.php".into(), HELPER.into()),
    ];
    let (f, ()) = Fixture::new_at_with_fixture_setup(
        CALLER,
        &extra,
        "app/Caller.php",
        |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
        |_, _| (),
    );
    let outcome = std::panic::AssertUnwindSafe(async {
        targets(&f, &["en", "fr"]).await;
        let foreign_at = at(CALLER, CALLER.rfind("messages.hello").unwrap());
        assert!(
            ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, foreign_at, GotoKind::Definition)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, foreign_at)
                .await
                .unwrap()
                .is_none()
        );
        let refs = ide_host::ops::lsp::lsp_references(
            &f.host,
            "main",
            RequestId::next(),
            f.doc,
            key_range(CALLER, "messages.hello").start,
            true,
            true,
            true,
            ide_host::ops::Channel::new(|_| {}),
        )
        .await
        .unwrap();
        assert!(refs.incomplete.is_none());
        let mut found: Vec<_> =
            refs.locations.into_iter().map(|target| (target.path.to_path_buf(), target.range)).collect();
        let mut expected: Vec<_> = [
            ("app/Caller.php", CALLER, "messages.hello"),
            ("helper.php", HELPER, "messages.hello"),
            ("lang/en/messages.php", EN, "hello"),
            ("lang/fr/messages.php", FR, "hello"),
        ]
        .into_iter()
        .map(|(name, text, key)| (f.root.join(name), key_range(text, key)))
        .collect();
        found.sort();
        expected.sort();
        assert_eq!(found, expected, "managed translation references must refuse foreign caller");
        let dependency = f.project.documents.open(&f.root.join("app/Shared/Selected.php")).unwrap();
        f.project.documents.replace_all(dependency.id, FOREIGN).unwrap();
        targets(&f, &[]).await;
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        targets(&f, &[]).await;
        f.project.documents.close(dependency.id);
        targets(&f, &["en", "fr"]).await;
        let catalog = f.project.documents.open(&f.root.join("lang/en/messages.php")).unwrap();
        let dirty = EN.replace("'hello'", "'other'");
        assert_eq!(dirty.len(), EN.len());
        f.project.documents.replace_all(catalog.id, &dirty).unwrap();
        targets(&f, &["fr"]).await;
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        targets(&f, &["fr"]).await;
        f.project.documents.close(catalog.id);
        targets(&f, &["en", "fr"]).await;
        for (name, text) in &extra {
            assert_eq!(&std::fs::read_to_string(f.root.join(name)).unwrap(), text);
        }
        assert_eq!(std::fs::read_to_string(f.root.join("app/Caller.php")).unwrap(), CALLER);
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}
