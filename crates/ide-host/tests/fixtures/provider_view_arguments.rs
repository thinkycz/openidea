//! Provider arguments follow Laravel binding order without leaking template data.
use super::*;

const ITEM: &str = "<?php\nnamespace App;\nclass Item { /* 😀 */ public string $itemOnly='item'; }\n";
const OTHER: &str = "<?php\nnamespace App;\nclass Other { /* 😀 */ public int $otherOnly=7; }\n";
const TEMPLATE: &str = "{{-- 😀 provider --}}\n{{ $item->itemOnly }}\n";
const COMPOSER: &str = "<?php namespace App; class Composer {public function compose($view):void {$view->with(value:new Item(),key:'item');}}";

fn provider(call: &str) -> String {
    format!(
        "<?php\nnamespace App\\Providers;\nuse Illuminate\\Support\\Facades\\View;\nclass ViewServiceProvider extends \\Illuminate\\Support\\ServiceProvider {{public function boot():void {{{call}}}}}\n"
    )
}

async fn check(f: &Fixture, doc: DocId, source: &str, class: Option<&str>, case: &str) {
    let cursor = at(source, source.find("->").unwrap() + 2);
    let mut completion = None;
    for _ in 0..8 {
        match ide_host::ops::lsp::lsp_completion(&f.host, "main", RequestId::next(), doc, cursor, None).await {
            Ok(list) => {
                completion = Some(list);
                break;
            }
            Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {
                tokio::time::sleep(Duration::from_millis(25)).await
            }
            Err(error) => panic!("{case}: {error}"),
        }
    }
    let completion = completion.expect("provider semantic snapshot must settle");
    assert_eq!(
        completion.items.iter().any(|item| item.label == "itemOnly"),
        class == Some("Item"),
        "{case}: {completion:?}"
    );
    assert_eq!(
        completion.items.iter().any(|item| item.label == "otherOnly"),
        class == Some("Other"),
        "{case}: {completion:?}"
    );
    let locations = ide_host::ops::lsp::lsp_goto(&f.host, "main", doc, cursor, GotoKind::Definition).await.unwrap();
    if let Some(class) = class {
        let (model, member) = if class == "Item" { (ITEM, "$itemOnly") } else { (OTHER, "$otherOnly") };
        let point = at(model, model.find(member).unwrap() + 1);
        // The caret remains on the existing member name in the template; query
        // the matching spelling when a dirty provider changes its model type.
        if source.contains(if class == "Item" { "itemOnly" } else { "otherOnly" }) {
            let hover = ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), doc, cursor)
                .await
                .unwrap()
                .expect("typed provider property hover");
            let expected = if class == "Item" { "public string $itemOnly" } else { "public int $otherOnly" };
            assert!(hover.contents.value.contains(expected), "{case}: {}", hover.contents.value);
            assert_eq!(locations.len(), 1, "{case}: {locations:?}");
            assert_eq!(*locations[0].path, f.root.join(format!("app/{class}.php")), "{case}");
            assert_eq!(locations[0].range, Range::new(point, point), "{case}");
        } else {
            assert!(locations.is_empty(), "{case}: removed model member must not remain navigable");
        }
    } else {
        assert!(locations.is_empty(), "{case}: {locations:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and Laravel vendor fixture"]
async fn provider_named_arguments_keep_dirty_scope_navigation_and_restart() {
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!("provider_view_arguments.json")).unwrap();
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").unwrap()).canonicalize().unwrap();
    for case in cases {
        let registration = provider(case["call"].as_str().unwrap());
        let files = vec![
            ("app/Item.php".into(), ITEM.into()),
            ("app/Other.php".into(), OTHER.into()),
            ("app/Composer.php".into(), COMPOSER.into()),
            ("app/Providers/ViewServiceProvider.php".into(), registration.clone()),
            ("bootstrap/providers.php".into(), "<?php return [App\\Providers\\ViewServiceProvider::class];".into()),
            ("resources/views/other.blade.php".into(), TEMPLATE.into()),
        ];
        let (f, ()) = Fixture::new_at_with_fixture_setup(
            TEMPLATE,
            &files,
            "resources/views/profile.blade.php",
            |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
            |_, _| (),
        );
        let outcome = std::panic::AssertUnwindSafe(async {
            let name = case["name"].as_str().unwrap();
            let expected = case["expected"] == true;
            let shared = case["scope"] == "shared";
            let provider_doc =
                ide_host::ops::doc::doc_open(&f.host, "main", f.root.join("app/Providers/ViewServiceProvider.php"))
                    .await
                    .unwrap()
                    .id;
            let composer_doc =
                ide_host::ops::doc::doc_open(&f.host, "main", f.root.join("app/Composer.php")).await.unwrap().id;
            let other_doc =
                ide_host::ops::doc::doc_open(&f.host, "main", f.root.join("resources/views/other.blade.php"))
                    .await
                    .unwrap()
                    .id;
            check(&f, f.doc, TEMPLATE, expected.then_some("Item"), name).await;
            check(&f, other_doc, TEMPLATE, (expected && shared).then_some("Item"), name).await;
            if expected {
                let dirty_view = format!("{{{{-- 😀 unsaved --}}}}\n{TEMPLATE}");
                f.project.documents.replace_all(f.doc, &dirty_view).unwrap();
                let changed = registration
                    .replace("<?php", "<?php /* 😀 moved */")
                    .replace("new \\App\\Item()", "new \\App\\Other()");
                let changed_composer =
                    COMPOSER.replace("<?php", "<?php /* 😀 moved */").replace("new Item()", "new Other()");
                Fixture::lint(&changed);
                Fixture::lint(&changed_composer);
                f.project.documents.replace_all(provider_doc, &changed).unwrap();
                f.project.documents.replace_all(composer_doc, &changed_composer).unwrap();
                check(&f, f.doc, &dirty_view, Some("Other"), name).await;
                check(&f, other_doc, TEMPLATE, shared.then_some("Other"), name).await;
                let other_view = dirty_view.replace("itemOnly", "otherOnly");
                f.project.documents.replace_all(f.doc, &other_view).unwrap();
                check(&f, f.doc, &other_view, Some("Other"), name).await;
                let server = f.project.lsp.servers().into_iter().find(|s| s.name == "phpantom").unwrap().id;
                ide_host::ops::lsp::lsp_restart(&f.host, "main", server).await.unwrap();
                check(&f, f.doc, &other_view, Some("Other"), name).await;
                let invalid = provider(if shared {
                    "View::share(key:'item', invalid:new \\App\\Other());"
                } else {
                    "View::composer(views:'profile', invalid:fn($view)=>$view->with('item',new \\App\\Other()));"
                });
                Fixture::lint(&invalid);
                f.project.documents.replace_all(provider_doc, &invalid).unwrap();
                check(&f, f.doc, &other_view, None, name).await;
                check(&f, other_doc, TEMPLATE, None, name).await;
                f.project.documents.replace_all(f.doc, &dirty_view).unwrap();
                ide_host::ops::doc::doc_close(&f.host, "main", provider_doc).await.unwrap();
                ide_host::ops::doc::doc_close(&f.host, "main", composer_doc).await.unwrap();
                check(&f, f.doc, &dirty_view, Some("Item"), name).await;
                check(&f, other_doc, TEMPLATE, shared.then_some("Item"), name).await;
                assert!(f.project.documents.snapshot(f.doc).unwrap().dirty);
                assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, dirty_view);
            }
            for (path, source) in &files {
                assert_eq!(std::fs::read_to_string(f.root.join(path)).unwrap(), *source, "{name} {path}");
            }
            assert_eq!(std::fs::read_to_string(f.root.join("resources/views/profile.blade.php")).unwrap(), TEMPLATE);
        })
        .catch_unwind()
        .await;
        f.finish(outcome).await;
    }
}
