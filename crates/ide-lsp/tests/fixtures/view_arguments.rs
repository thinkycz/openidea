use super::*;
use futures::FutureExt;

async fn view_model_members(mgr: &LspManager, doc: DocId, required: &str, forbidden: &[&str]) {
    let list = settled_completion(mgr, doc, Pos::new(1, 10)).await;
    let labels: Vec<_> = list.items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&required), "named view data must expose {required}: {labels:?}");
    assert!(forbidden.iter().all(|name| !labels.contains(name)), "retired view data: {labels:?}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the pinned macOS Apple Silicon PHPantom executable"]
async fn named_view_arguments_keep_unsaved_models_origins_and_restart_through_client() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    for directory in ["app", "resources/views"] {
        std::fs::create_dir_all(root.join(directory)).unwrap();
    }
    let item = "<?php namespace App; /* 😀 */ class Item { public string $name = 'item'; public int $price = 1; }\n";
    let other =
        "<?php namespace App; /* 😀 */ class Other { public string $name = 'other'; public int $otherOnly = 1; }\n";
    let prefix =
        "<?php namespace App; /* 😀 žluťoučký */ use function view as render_view; function show(): mixed { return ";
    let cases = [
        ("view('page', ['item' => new Item])", "price", "otherOnly", "Item.php"),
        ("view(data: ['item' => new Item], view: 'page')", "price", "otherOnly", "Item.php"),
        ("render_view(data: ['item' => new Other], view: 'page')", "otherOnly", "price", "Other.php"),
        ("view(view: 'page', mergeData: ['item' => new Item])", "price", "otherOnly", "Item.php"),
        (
            "view(view: 'page', data: ['item' => new Item], mergeData: ['item' => new Other])",
            "price",
            "otherOnly",
            "Item.php",
        ),
        (
            "view(mergeData: ['item' => new Other], view: 'page', data: ['item' => new Item])",
            "price",
            "otherOnly",
            "Item.php",
        ),
        (
            "view(mergeData: ['item' => new Item], view: 'page', data: [])->with('item', new Other)",
            "otherOnly",
            "price",
            "Other.php",
        ),
        ("view('page', [], ['item' => new Item])", "price", "otherOnly", "Item.php"),
    ];
    let initial = format!("{prefix}{}; }}\n", cases[0].0);
    let template = "{{-- 😀 template --}}\n{{ $item->name }}\n";
    let files = [
        (
            "composer.json",
            r#"{"require":{"php":"^8.3","laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#,
        ),
        ("app/Item.php", item),
        ("app/Other.php", other),
        ("app/Caller.php", initial.as_str()),
        ("resources/views/page.blade.php", template),
    ];
    for (path, text) in files {
        std::fs::write(root.join(path), text).unwrap();
    }
    let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let mut php = LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
    php.language_servers.retain(|server| server.name == "phpantom");
    php.language_servers[0].command = bin.to_string_lossy().into_owned();
    let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
    let caller = DocId::next();
    let blade = DocId::next();
    mgr.did_open(caller, &root.join("app/Caller.php"), &LanguageId::new("php"), 1, &initial);
    mgr.did_open(blade, &root.join("resources/views/page.blade.php"), &LanguageId::new("php"), 1, template);
    let outcome = std::panic::AssertUnwindSafe(async {
        let mut previous = initial.clone();
        for (index, (body, required, forbidden, model)) in cases.iter().enumerate() {
            let text = format!("{prefix}{body}; }}\n");
            if index > 0 {
                replace(&mgr, caller, index as u32 + 1, &previous, &text);
            }
            view_model_members(&mgr, blade, required, &[forbidden]).await;
            let view = mgr.goto(caller, pos(&text, "page'"), GotoKind::Definition).await.unwrap();
            assert!(
                view.iter().any(|target| *target.path == root.join("resources/views/page.blade.php")),
                "named view target: {body}: {view:?}"
            );
            let target = mgr.goto(blade, Pos::new(1, 10), GotoKind::Definition).await.unwrap();
            let declaration = if *model == "Item.php" { item } else { other };
            assert!(
                target.iter().any(|target| *target.path == root.join("app").join(model)
                    && target.range.start == pos(declaration, "name =")),
                "UTF16 named model origin: {body}: {target:?}"
            );
            eprintln!("Accepted view data case {index}: {body}");
            if index == 6 {
                let dependency = DocId::next();
                let fresh = other.replace("otherOnly", "freshOnly");
                mgr.did_open(dependency, &root.join("app/Other.php"), &LanguageId::new("php"), 1, other);
                replace(&mgr, dependency, 2, other, &fresh);
                view_model_members(&mgr, blade, "freshOnly", &["price", "otherOnly"]).await;
                mgr.restart(mgr.servers()[0].id).await.unwrap();
                view_model_members(&mgr, blade, "freshOnly", &["price", "otherOnly"]).await;
                let replay = mgr.goto(blade, Pos::new(1, 10), GotoKind::Definition).await.unwrap();
                assert!(
                    replay.iter().any(|target| *target.path == root.join("app/Other.php")),
                    "restart must replay named caller and dirty model: {replay:?}"
                );
                mgr.did_close(dependency);
                view_model_members(&mgr, blade, "otherOnly", &["price", "freshOnly"]).await;
            }
            previous = text;
        }
        let invalid = format!("{prefix}view(data: 'page', view: 'missing'); }}\n");
        replace(&mgr, caller, 9, &previous, &invalid);
        let wrong = mgr.goto(caller, pos(&invalid, "page'"), GotoKind::Definition).await.unwrap();
        assert!(
            !wrong.iter().any(|target| *target.path == root.join("resources/views/page.blade.php")),
            "a data argument cannot borrow the view target: {wrong:?}"
        );
        mgr.did_close(caller);
        view_model_members(&mgr, blade, "price", &["otherOnly", "freshOnly"]).await;
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    for (path, original) in files {
        assert_eq!(std::fs::read_to_string(root.join(path)).unwrap(), original);
    }
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
