use super::*;
use futures::FutureExt;

const COMPOSER: &str =
    r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#;
const EN: &str = "<?php return ['hello'=>'Hello :name :count'];\n";
const FR: &str = "<?php return ['hello'=>'Bonjour :count'];\n";
const SELECTED: &str = r#"<?php namespace App\Shared; class Selected extends \Illuminate\Translation\Translator {}"#;
const FOREIGN: &str = r#"<?php namespace App\Shared; class Selected { public function get($key, array $replace=[], $locale=null) {return 'foreign';} }"#;
const CASES: &[(&str, &str, &[&str])] = &[
    (
        r###"<?php namespace App\Typed0; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { /* 😀 */ return $translator->get('messages.hello', [], 'e<caret>n'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed1; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { /* 😀 */ return $translator->get('messages.hello', ['n<caret>ame'=>'Ada']); } return render($GLOBALS['probe_translator']);"###,
        "Translation placeholder",
        &["name"],
    ),
    (
        r###"<?php namespace App\Typed2; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { /* 😀 */ return $translator->get(locale: 'e<caret>n', key: 'messages.hello'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed3; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Contract $translator): mixed { /* 😀 */ return $translator->get('messages.hello', [], 'e<caret>n'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed4; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Contract $translator): mixed { /* 😀 */ return $translator->get('messages.hello', ['n<caret>ame'=>'Ada']); } return render($GLOBALS['probe_translator']);"###,
        "Translation placeholder",
        &["name"],
    ),
    (
        r###"<?php namespace App\Typed5; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { /* 😀 */ return $translator->choice('messages.hello', 2, [], 'e<caret>n'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed6; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Contract $translator): mixed { /* 😀 */ return $translator->choice(key: 'messages.hello', number: 2, replace: ['n<caret>ame'=>'Ada']); } return render($GLOBALS['probe_translator']);"###,
        "Translation placeholder",
        &["name"],
    ),
    (
        r###"<?php namespace App\Typed7; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { /* 😀 */ return $translator->has('messages.hello', 'e<caret>n'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed8; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { /* 😀 */ return $translator->hasForLocale(locale: 'e<caret>n', key: 'messages.hello'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed9; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(?Text $translator): mixed { /* 😀 */ return $translator?->get('messages.hello', [], 'e<caret>n'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed10; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text|Contract $translator): mixed { /* 😀 */ return $translator->get('messages.hello', [], 'e<caret>n'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed11; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { /* 😀 */ return $translator->GET('messages.hello', [], 'e<caret>n'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed12; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { /* 😀 */ return $translator /* receiver */ -> get('messages.hello', [], 'e<caret>n'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed13; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} class Holder {public function __construct(private Text $translator) {} public function render(): mixed { /* 😀 */ return $this->translator->get('messages.hello', [], 'e<caret>n'); }} function render(Text $translator): mixed {return (new Holder($translator))->render();} return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed14; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Foreign $translator): mixed { /* 😀 */ return $translator->get('messages.hello', [], 'e<caret>n'); } return render(new Foreign);"###,
        "Translation locale",
        &[],
    ),
    (
        r###"<?php namespace App\Typed15; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text|Foreign $translator): mixed { /* 😀 */ return $translator->get('messages.hello', [], 'e<caret>n'); } return render(new Foreign);"###,
        "Translation locale",
        &[],
    ),
    (
        r###"<?php namespace App\Typed16; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Foreign|Text $translator): mixed { /* 😀 */ return $translator->get('messages.hello', [], 'e<caret>n'); } return render(new Foreign);"###,
        "Translation locale",
        &[],
    ),
    (
        r###"<?php namespace App\Typed17; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text|int $translator): mixed { /* 😀 */ return $translator->get('messages.hello', [], 'e<caret>n'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &[],
    ),
    (
        r###"<?php namespace App\Typed18; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(object $translator): mixed { /* 😀 */ return $translator->get('messages.hello', [], 'e<caret>n'); } return render(new Foreign);"###,
        "Translation locale",
        &[],
    ),
    (
        r###"<?php namespace App\Typed19; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { /* 😀 */ return $translated = $translator->get('messages.hello', [], 'e<caret>n'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed20; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Contract|Text $translator): mixed { /* 😀 */ return $translator->get('messages.hello', [], 'e<caret>n'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &["en"],
    ),
    (
        r###"<?php namespace App\Typed21; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text|\Illuminate\Config\Repository $translator): mixed { /* 😀 */ return $translator->get('messages.hello', [], 'e<caret>n'); } return render($GLOBALS['probe_translator']);"###,
        "Translation locale",
        &[],
    ),
];
const BARE: &[&str] = &[
    r###"<?php namespace App\Typed0; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { $translator->get('messages.hello', [], 'e<caret>n'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed1; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { $translator->get('messages.hello', ['n<caret>ame'=>'Ada']); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed2; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { $translator->get(locale: 'e<caret>n', key: 'messages.hello'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed3; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Contract $translator): mixed { $translator->get('messages.hello', [], 'e<caret>n'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed4; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Contract $translator): mixed { $translator->get('messages.hello', ['n<caret>ame'=>'Ada']); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed5; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { $translator->choice('messages.hello', 2, [], 'e<caret>n'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed6; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Contract $translator): mixed { $translator->choice(key: 'messages.hello', number: 2, replace: ['n<caret>ame'=>'Ada']); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed7; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { $translator->has('messages.hello', 'e<caret>n'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed8; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { $translator->hasForLocale(locale: 'e<caret>n', key: 'messages.hello'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed9; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(?Text $translator): mixed { $translator?->get('messages.hello', [], 'e<caret>n'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed10; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text|Contract $translator): mixed { $translator->get('messages.hello', [], 'e<caret>n'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed11; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { $translator->GET('messages.hello', [], 'e<caret>n'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed12; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { $translator /* receiver */ -> get('messages.hello', [], 'e<caret>n'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed13; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} class Holder {public function __construct(private Text $translator) {} public function render(): mixed { $this->translator->get('messages.hello', [], 'e<caret>n'); return null; }} function render(Text $translator): mixed {return (new Holder($translator))->render();} return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed14; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Foreign $translator): mixed { $translator->get('messages.hello', [], 'e<caret>n'); return null; } return render(new Foreign);"###,
    r###"<?php namespace App\Typed15; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text|Foreign $translator): mixed { $translator->get('messages.hello', [], 'e<caret>n'); return null; } return render(new Foreign);"###,
    r###"<?php namespace App\Typed16; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Foreign|Text $translator): mixed { $translator->get('messages.hello', [], 'e<caret>n'); return null; } return render(new Foreign);"###,
    r###"<?php namespace App\Typed17; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text|int $translator): mixed { $translator->get('messages.hello', [], 'e<caret>n'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed18; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(object $translator): mixed { $translator->get('messages.hello', [], 'e<caret>n'); return null; } return render(new Foreign);"###,
    r###"<?php namespace App\Typed19; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text $translator): mixed { $translated = $translator->get('messages.hello', [], 'e<caret>n'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed20; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Contract|Text $translator): mixed { $translator->get('messages.hello', [], 'e<caret>n'); return null; } return render($GLOBALS['probe_translator']);"###,
    r###"<?php namespace App\Typed21; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign {public function get($key, array $replace=[], $locale=null) {return 'foreign';}} function render(Text|\Illuminate\Config\Repository $translator): mixed { $translator->get('messages.hello', [], 'e<caret>n'); return null; } return render($GLOBALS['probe_translator']);"###,
];

fn text(marked: &str) -> String {
    marked.replacen("<caret>", "", 1)
}
fn pos(text: &str, offset: usize) -> Pos {
    let prefix = &text[..offset];
    Pos::new(
        prefix.bytes().filter(|b| *b == b'\n').count() as u32,
        prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
    )
}
fn key_marked(marked: &str) -> String {
    let source = text(marked);
    let cursor = source.find("'messages.hello'").unwrap() + "'mess".len();
    format!("{}<caret>{}", &source[..cursor], &source[cursor..])
}

async fn suggestions(
    mgr: &LspManager,
    doc: DocId,
    marked: &str,
    detail: &str,
    expected: &[&str],
    key: bool,
) -> Option<String> {
    let cursor = marked.find("<caret>").unwrap();
    let source = text(marked);
    let list = settled_completion(mgr, doc, pos(&source, cursor)).await;
    let relevant: Vec<_> = list
        .items
        .iter()
        .filter(|item| if key { item.label == "messages.hello" } else { item.detail.as_deref() == Some(detail) })
        .collect();
    let mut labels: Vec<_> = relevant.iter().map(|x| x.label.as_str()).collect();
    labels.sort_unstable();
    assert_eq!(labels, expected, "typed translator client identity: {marked}; key={key}; {list:?}");
    let start = source[..cursor].rfind('\'').unwrap() + 1;
    let end = start + source[start..].find('\'').unwrap_or(source.len() - start);
    let full = Range::new(pos(&source, start), pos(&source, end));
    let mut first = None;
    for item in relevant {
        let resolved = mgr.resolve_completion(doc, item.clone()).await.unwrap();
        assert_eq!(resolved.label, item.label);
        assert!(resolved.additional_edits.is_empty() && resolved.command.is_none() && !resolved.is_snippet);
        assert_eq!(resolved.insert_text, item.label);
        if key {
            assert_eq!(
                resolved.replace_range,
                Some(Range::new(pos(&source, start), pos(&source, cursor))),
                "key Enter range must preserve insertion semantics"
            );
            assert_eq!(
                resolved.replace_range_tab,
                (end != cursor).then_some(full),
                "typed key Tab must replace the entire dotted token"
            );
        } else {
            assert_eq!(resolved.replace_range, Some(full), "typed argument UTF-16 full-token range");
        }
        let range =
            if key { resolved.replace_range_tab.or(resolved.replace_range) } else { resolved.replace_range }.unwrap();
        let mut document = ide_text::Document::new(&source);
        document.apply_edits(&[ide_core::TextEdit { range, new_text: resolved.insert_text.clone() }]).unwrap();
        let applied = document.text();
        assert_eq!(
            applied,
            format!("{}{}{}", &source[..start], resolved.insert_text, &source[end..]),
            "typed completion must preserve the original source and avoid duplicate suffixes"
        );
        first.get_or_insert(applied);
    }
    first
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and Laravel 13.37 vendor fixture"]
async fn typed_translation_receivers_keep_edits_dirty_types_incomplete_buffers_and_restart() {
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
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    for name in ["app/Shared", "lang/en", "lang/fr"] {
        std::fs::create_dir_all(root.join(name)).unwrap();
    }
    std::os::unix::fs::symlink(vendor, root.join("vendor")).unwrap();
    let initial = text(CASES[0].0);
    let files = [
        ("composer.json", COMPOSER),
        ("app/Caller.php", initial.as_str()),
        ("app/Shared/Selected.php", SELECTED),
        ("lang/en/messages.php", EN),
        ("lang/fr/messages.php", FR),
    ];
    for (name, value) in files {
        std::fs::write(root.join(name), value).unwrap();
    }
    let engine = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").expect("pinned PHPantom executable"));
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
            let next = text(marked);
            version += 1;
            replace(&mgr, caller, version, &previous, &next);
            previous = next;
        };
        for (index, (marked, detail, expected)) in CASES.iter().enumerate() {
            if index > 0 {
                update(marked);
            }
            suggestions(&mgr, caller, marked, detail, expected, false).await;
            let key = key_marked(marked);
            update(&key);
            suggestions(&mgr, caller, &key, "", if expected.is_empty() { &[] } else { &["messages.hello"] }, true)
                .await;
        }
        for (index, marked) in BARE.iter().enumerate() {
            update(marked);
            suggestions(&mgr, caller, marked, CASES[index].1, CASES[index].2, false).await;
            let key = key_marked(marked);
            update(&key);
            suggestions(
                &mgr,
                caller,
                &key,
                "",
                if CASES[index].2.is_empty() { &[] } else { &["messages.hello"] },
                true,
            )
            .await;
        }
        let typo = CASES[1].0.replace("n<caret>ame", "n<caret>ame_typo");
        update(&typo);
        let applied = suggestions(&mgr, caller, &typo, "Translation placeholder", &["name"], false).await.unwrap();
        assert_eq!(applied, text(CASES[1].0));
        update(&applied);
        for (index, key) in [(0, false), (1, false), (0, true), (13, false), (15, false)] {
            let marked = if key { key_marked(CASES[index].0) } else { CASES[index].0.to_owned() };
            let partial = format!("{}<caret>", marked.split("<caret>").next().unwrap());
            update(&partial);
            let expected = if key { &["messages.hello"][..] } else { CASES[index].2 };
            suggestions(&mgr, caller, &partial, CASES[index].1, expected, key).await;
            mgr.restart(mgr.servers()[0].id).await.unwrap();
            suggestions(&mgr, caller, &partial, CASES[index].1, expected, key).await;
            update(&marked);
            suggestions(&mgr, caller, &marked, CASES[index].1, expected, key).await;
        }
        let all = CASES[1].0.replace("n<caret>ame", "<caret>");
        update(&all);
        suggestions(&mgr, caller, &all, "Translation placeholder", &["count", "name"], false).await;
        let catalog = DocId::next();
        mgr.did_open(catalog, &root.join("lang/en/messages.php"), &LanguageId::new("php"), 1, EN);
        let dirty = EN.replace(":name", ":fresh");
        replace(&mgr, catalog, 2, EN, &dirty);
        suggestions(&mgr, caller, &all, "Translation placeholder", &["count", "fresh"], false).await;
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        suggestions(&mgr, caller, &all, "Translation placeholder", &["count", "fresh"], false).await;
        mgr.did_close(catalog);
        suggestions(&mgr, caller, &all, "Translation placeholder", &["count", "name"], false).await;
        let selected =
            CASES[0].0.replace("use Illuminate\\Translation\\Translator as Text", "use App\\Shared\\Selected as Text");
        update(&selected);
        suggestions(&mgr, caller, &selected, "Translation locale", &["en"], false).await;
        let dependency = DocId::next();
        mgr.did_open(dependency, &root.join("app/Shared/Selected.php"), &LanguageId::new("php"), 1, SELECTED);
        replace(&mgr, dependency, 2, SELECTED, FOREIGN);
        suggestions(&mgr, caller, &selected, "Translation locale", &[], false).await;
        let key = key_marked(&selected);
        update(&key);
        suggestions(&mgr, caller, &key, "", &[], true).await;
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        suggestions(&mgr, caller, &key, "", &[], true).await;
        mgr.did_close(dependency);
        suggestions(&mgr, caller, &key, "", &["messages.hello"], true).await;
        let namespace = selected.replace("App\\Typed0", "App\\OtherTyped");
        update(&namespace);
        suggestions(&mgr, caller, &namespace, "Translation locale", &["en"], false).await;
        let foreign = CASES[0]
            .0
            .replace("use Illuminate\\Translation\\Translator as Text", "use App\\Typed0\\Foreign as Text")
            .replace("render($GLOBALS['probe_translator'])", "render(new Foreign)");
        update(&foreign);
        suggestions(&mgr, caller, &foreign, "Translation locale", &[], false).await;
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        suggestions(&mgr, caller, &foreign, "Translation locale", &[], false).await;
        mgr.did_close(caller);
        let reopened = DocId::next();
        mgr.did_open(reopened, &root.join("app/Caller.php"), &LanguageId::new("php"), 1, &initial);
        suggestions(&mgr, reopened, CASES[0].0, CASES[0].1, CASES[0].2, false).await;
        let key = key_marked(CASES[0].0);
        suggestions(&mgr, reopened, &key, "", &["messages.hello"], true).await;
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    assert!(mgr.servers().iter().all(|s| s.status == ServerStatus::Stopped));
    for (name, value) in files {
        assert_eq!(
            std::fs::read_to_string(root.join(name)).unwrap(),
            value,
            "typed translation operations must preserve disk originals"
        );
    }
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
