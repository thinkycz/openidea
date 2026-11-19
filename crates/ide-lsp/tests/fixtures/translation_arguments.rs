use super::*;
use futures::FutureExt;

const COMPOSER: &str = r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"},"files":["helpers.php"]}}"#;
const EN: &str = r#"<?php return ['hello'=>'Hello :name :count'];
"#;
const FR: &str = r#"<?php return ['hello'=>'Bonjour :count'];
"#;
const HELPER: &str = r#"<?php namespace App\Scope; function local_trans($key, $replace = [], $locale = null) { return 'foreign'; }
"#;
const SCOPE: &str = r#"<?php namespace App\Scope; /* 😀 */ return trans('messages.hello', [], 'e|n');"#;
const REBIND: &str = r#"<?php namespace Foreign { function trans($key, $replace = [], $locale = null) { return 'foreign'; } } namespace App\Rebind { use function trans as localize; /* 😀 */ return localize('messages.hello', [], 'e|n'); }"#;
const CASES: &[(&str, &str, &[&str])] = &[
    (
        r###"<?php namespace App\Case0; use function trans as localize; /* 😀 */ return localize('messages.hello', [], 'e|n');"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Case1; use function trans as localize; /* 😀 */ return localize('messages.hello', ['n|ame'=>'Ada']);"###,
        "Translation placeholder",
        &["name"],
    ),
    (
        r###"<?php namespace App\Case2; use function trans_choice as choose; /* 😀 */ return choose('messages.hello', 2, [], 'e|n');"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Case3; use function trans_choice as choose; /* 😀 */ return choose('messages.hello', 2, ['n|ame'=>'Ada']);"###,
        "Translation placeholder",
        &["name"],
    ),
    (
        r###"<?php namespace App\Case4; use Illuminate\Support\Facades\Lang as L; /* 😀 */ return L::get('messages.hello', [], 'e|n');"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Case5; function trans($key,$replace=[],$locale=null){return 'foreign';} /* 😀 */ return trans('messages.hello', [], 'e|n');"###,
        "Translation locale",
        &[],
    ),
    (
        r###"<?php namespace Foreign {function trans($key,$replace=[],$locale=null){return 'foreign';}} namespace App\Case6 {use function Foreign\trans; /* 😀 */ return trans('messages.hello', [], 'e|n');}"###,
        "Translation locale",
        &[],
    ),
    (
        r###"<?php namespace App\Case7; class Lang {public static function get($key,$replace=[],$locale=null){return 'foreign';}} /* 😀 */ return Lang::get('messages.hello', [], 'e|n');"###,
        "Translation locale",
        &[],
    ),
    (
        r###"<?php namespace App\Case8; /* 😀 */ return trans('messages.hello', [], 'e|n');"###,
        "Translation locale",
        &["en"],
    ),
];

fn offset_position(text: &str, offset: usize) -> Pos {
    let prefix = &text[..offset];
    Pos::new(
        prefix.bytes().filter(|byte| *byte == b'\n').count() as u32,
        prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
    )
}

async fn translation_suggestions(
    mgr: &LspManager,
    doc: DocId,
    marked: &str,
    detail: &str,
    expected: &[&str],
) -> Option<String> {
    let cursor = marked.find('|').unwrap();
    let text = marked.replacen('|', "", 1);
    let list = settled_completion(mgr, doc, offset_position(&text, cursor)).await;
    let items: Vec<_> = list.items.iter().filter(|item| item.detail.as_deref() == Some(detail)).collect();
    let mut labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
    labels.sort_unstable();
    assert_eq!(labels, expected, "translation client callable identity: {marked}: {list:?}");
    let start = text[..cursor].rfind('\'').unwrap() + 1;
    let end = start + text[start..].find('\'').unwrap();
    let range = Range::new(offset_position(&text, start), offset_position(&text, end));
    let mut first = None;
    for item in items {
        let resolved = mgr.resolve_completion(doc, item.clone()).await.unwrap();
        assert_eq!(resolved.label, item.label);
        assert_eq!(resolved.replace_range, Some(range), "translation client UTF-16 full-token edit");
        assert_eq!(resolved.insert_text, item.label);
        assert!(resolved.additional_edits.is_empty() && resolved.command.is_none() && !resolved.is_snippet);
        let mut document = ide_text::Document::new(&text);
        document.apply_edits(&[ide_core::TextEdit { range, new_text: resolved.insert_text.clone() }]).unwrap();
        let applied = document.text();
        assert_eq!(applied, format!("{}{}{}", &text[..start], resolved.insert_text, &text[end..]));
        first.get_or_insert(applied);
    }
    first
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and Laravel 13.37 vendor fixture"]
async fn translation_arguments_keep_callable_identity_dirty_dependencies_edits_and_restart() {
    let vendor = PathBuf::from(
        std::env::var_os("OPENSTORM_TEST_LARAVEL_VENDOR")
            .or_else(|| std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR"))
            .expect("Laravel vendor fixture"),
    )
    .canonicalize()
    .unwrap();
    let installed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(vendor.join("composer/installed.json")).unwrap()).unwrap();
    assert!(installed["packages"].as_array().unwrap().iter().any(|package| package["name"] == "laravel/framework"
        && matches!(package["version"].as_str(), Some("v13.37.0" | "13.37.0"))));
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    for directory in ["app", "lang/en", "lang/fr"] {
        std::fs::create_dir_all(root.join(directory)).unwrap();
    }
    std::os::unix::fs::symlink(vendor, root.join("vendor")).unwrap();
    let initial = CASES[0].0.replacen('|', "", 1);
    let files = [
        ("composer.json", COMPOSER),
        ("helpers.php", HELPER),
        ("app/Caller.php", initial.as_str()),
        ("lang/en/messages.php", EN),
        ("lang/fr/messages.php", FR),
    ];
    for (name, text) in files {
        std::fs::write(root.join(name), text).unwrap();
    }
    let engine = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![route_handler_language(&engine)])),
        Arc::new(|_| {}),
    );
    let caller = DocId::next();
    mgr.did_open(caller, &root.join("app/Caller.php"), &LanguageId::new("php"), 1, &initial);
    let outcome = std::panic::AssertUnwindSafe(async {
        let mut previous = initial.clone();
        let mut version = 1;
        let mut update = |marked: &str| {
            let text = marked.replacen('|', "", 1);
            version += 1;
            replace(&mgr, caller, version, &previous, &text);
            previous = text;
        };
        for (index, (marked, detail, expected)) in CASES.iter().enumerate() {
            if index > 0 {
                update(marked);
            }
            translation_suggestions(&mgr, caller, marked, detail, expected).await;
        }
        let typo = CASES[1].0.replace("n|ame", "n|ame_typo");
        update(&typo);
        let applied = translation_suggestions(&mgr, caller, &typo, "Translation placeholder", &["name"]).await.unwrap();
        assert_eq!(
            applied,
            CASES[1].0.replacen('|', "", 1),
            "accepted translation edit must preserve imports and surrounding source"
        );
        update(&applied);
        translation_suggestions(&mgr, caller, CASES[1].0, "Translation placeholder", &["name"]).await;

        let catalog = DocId::next();
        let all_placeholders = CASES[1].0.replace("n|ame", "|");
        update(&all_placeholders);
        translation_suggestions(&mgr, caller, &all_placeholders, "Translation placeholder", &["count", "name"]).await;
        mgr.did_open(catalog, &root.join("lang/en/messages.php"), &LanguageId::new("php"), 1, EN);
        let dirty_catalog = EN.replace(":name", ":fresh");
        replace(&mgr, catalog, 2, EN, &dirty_catalog);
        translation_suggestions(&mgr, caller, &all_placeholders, "Translation placeholder", &["count", "fresh"]).await;
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        translation_suggestions(&mgr, caller, &all_placeholders, "Translation placeholder", &["count", "fresh"]).await;
        mgr.did_close(catalog);
        translation_suggestions(&mgr, caller, &all_placeholders, "Translation placeholder", &["count", "name"]).await;

        update(REBIND);
        translation_suggestions(&mgr, caller, REBIND, "Translation locale", &["en"]).await;
        let foreign = REBIND.replace("use function trans as", "use function Foreign\\trans as");
        update(&foreign);
        translation_suggestions(&mgr, caller, &foreign, "Translation locale", &[]).await;
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        translation_suggestions(&mgr, caller, &foreign, "Translation locale", &[]).await;

        let dependency = DocId::next();
        update(SCOPE);
        translation_suggestions(&mgr, caller, SCOPE, "Translation locale", &["en"]).await;
        mgr.did_open(dependency, &root.join("helpers.php"), &LanguageId::new("php"), 1, HELPER);
        let dirty_helper = HELPER.replace("function local_trans", "function trans");
        replace(&mgr, dependency, 2, HELPER, &dirty_helper);
        translation_suggestions(&mgr, caller, SCOPE, "Translation locale", &[]).await;
        let other_namespace = SCOPE.replace("App\\Scope", "App\\Other");
        update(&other_namespace);
        translation_suggestions(&mgr, caller, &other_namespace, "Translation locale", &["en"]).await;
        update(SCOPE);
        translation_suggestions(&mgr, caller, SCOPE, "Translation locale", &[]).await;
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        translation_suggestions(&mgr, caller, SCOPE, "Translation locale", &[]).await;
        mgr.did_close(dependency);
        translation_suggestions(&mgr, caller, SCOPE, "Translation locale", &["en"]).await;
        mgr.did_close(caller);
        let reopened = DocId::next();
        mgr.did_open(reopened, &root.join("app/Caller.php"), &LanguageId::new("php"), 1, &initial);
        translation_suggestions(&mgr, reopened, CASES[0].0, CASES[0].1, CASES[0].2).await;
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    assert!(mgr.servers().iter().all(|server| server.status == ServerStatus::Stopped));
    for (name, text) in files {
        assert_eq!(
            std::fs::read_to_string(root.join(name)).unwrap(),
            text,
            "translation client operations must preserve all originals"
        );
    }
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
