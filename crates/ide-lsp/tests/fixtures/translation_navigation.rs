//! Typed translation keys through the actual LSP client.
use super::*;
use futures::FutureExt;

const COMPOSER: &str =
    r###"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"###;
const PRELUDE: &str = r###"<?php namespace App; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign { public function get($key, array $replace=[], $locale=null) {return 'foreign';} } "###;
const EN: &str = r###"<?php /* 😀 */ return ['hello'=>'Hello'];"###;
const FR: &str = r###"<?php return ['hello'=>'Bonjour'];"###;
const SELECTED: &str =
    r###"<?php namespace App\Shared; class Selected extends \Illuminate\Translation\Translator {}"###;
const FOREIGN: &str = r###"<?php namespace App\Shared; class Selected { public function get($key, array $replace=[], $locale=null) {return 'foreign';} }"###;
const CALLER: &str = r###"<?php namespace App\SelectedCaller; use App\Shared\Selected as Text; function render(Text $text) { /* 😀 */ return $text->get('messages.hello'); }"###;
const HELPER: &str = r###"<?php /* 😀 */ return trans('messages.hello');"###;
const DIAGNOSTICS: &str = r###"<?php namespace App\TranslationDiag; use Illuminate\Translation\Translator as Text; use Illuminate\Contracts\Translation\Translator as Contract; class Foreign { public function get($key, array $replace=[], $locale=null) {return 'foreign';} } function render(Text $text, Foreign $foreign, Text|Foreign $uncertain) { /* 😀 */ return [trans('messages.control_missing'), $text->get('messages.absent'), $foreign->get('messages.foreign'), $uncertain->get('messages.uncertain')]; }"###;

const CASES: &[(&str, bool)] = &[
    (r###"function render(Text $text) { /* 😀 */ return $text->get('messages.hello'); }"###, true),
    (r###"function render(Contract $text) { /* 😀 */ return $text->get('messages.hello'); }"###, true),
    (r###"function render(Text $text) { /* 😀 */ return $text->get(locale:'en', key:'messages.hello'); }"###, true),
    (r###"function render(?Text $text) { /* 😀 */ return $text?->get('messages.hello'); }"###, true),
    (r###"function render(?Text $text) { /* 😀 */ return $text->get('messages.hello'); }"###, true),
    (r###"function render(Text|Contract $text) { /* 😀 */ return $text->get('messages.hello'); }"###, true),
    (r###"function render(Contract|Text $text) { /* 😀 */ return $text->get('messages.hello'); }"###, true),
    (r###"function render(Text $text) { /* 😀 */ return $text /* receiver */ -> GET('messages.hello'); }"###, true),
    (r###"function render(Text $text) { /* 😀 */ return $translated = $text->get('messages.hello'); }"###, true),
    (
        r###"class Holder { public function __construct(private Text $text) {} public function render() { /* 😀 */ return $this->text->get('messages.hello'); } }"###,
        true,
    ),
    (r###"function render(Text $text) { /* 😀 */ return $text->has('messages.hello'); }"###, true),
    (
        r###"function render(Text $text) { /* 😀 */ return $text->hasForLocale(locale:'en', key:'messages.hello'); }"###,
        true,
    ),
    (r###"function render(Contract $text) { /* 😀 */ return $text->choice(number:2, key:'messages.hello'); }"###, true),
    (r###"function render(Foreign $text) { /* 😀 */ return $text->get('messages.hello'); }"###, false),
    (r###"function render(Text|Foreign $text) { /* 😀 */ return $text->get('messages.hello'); }"###, false),
    (r###"function render(Foreign|Text $text) { /* 😀 */ return $text->get('messages.hello'); }"###, false),
    (r###"function render(Text|int $text) { /* 😀 */ return $text->get('messages.hello'); }"###, false),
    (r###"function render(object $text) { /* 😀 */ return $text->get('messages.hello'); }"###, false),
    (r###"function render(Contract $text) { /* 😀 */ return $text->has('messages.hello'); }"###, false),
    (
        r###"function render(Text $text) { $text = new Foreign; /* 😀 */ return $text->get('messages.hello'); }"###,
        false,
    ),
];

fn range(text: &str, needle: &str) -> Range {
    let start = pos(text, needle);
    Range::new(start, Pos::new(start.line, start.col + needle.encode_utf16().count() as u32))
}

fn location_key(location: &ide_core::Location) -> (PathBuf, u32, u32, u32, u32) {
    assert!(location.uri.is_none(), "translation targets must be physical files");
    (
        location.path.to_path_buf(),
        location.range.start.line,
        location.range.start.col,
        location.range.end.line,
        location.range.end.col,
    )
}

fn expected_key(root: &std::path::Path, name: &str, text: &str, needle: &str) -> (PathBuf, u32, u32, u32, u32) {
    let range = range(text, needle);
    (root.join(name), range.start.line, range.start.col, range.end.line, range.end.col)
}

async fn navigation(mgr: &LspManager, doc: DocId, text: &str, root: &std::path::Path, locales: &[&str]) {
    let position = pos(text, "messages.hello");
    let mut targets: Vec<_> =
        mgr.goto(doc, position, GotoKind::Definition).await.unwrap().iter().map(location_key).collect();
    let mut expected: Vec<_> = locales
        .iter()
        .map(|locale| {
            expected_key(root, &format!("lang/{locale}/messages.php"), if *locale == "en" { EN } else { FR }, "hello")
        })
        .collect();
    targets.sort();
    expected.sort();
    assert_eq!(targets, expected, "typed translation client navigation must retain exact locale targets: {text}");
    let hover = mgr.hover(doc, position, Default::default()).await.unwrap();
    if locales.is_empty() {
        assert!(hover.is_none(), "foreign/uncertain typed translation must not gain hover: {text}: {hover:?}");
    } else {
        let hover = hover.expect("typed translation client hover");
        assert_eq!(hover.range, Some(range(text, "messages.hello")));
        assert_eq!(hover.contents.value.contains("Hello"), locales.contains(&"en"), "{hover:?}");
        assert_eq!(hover.contents.value.contains("Bonjour"), locales.contains(&"fr"), "{hover:?}");
        for locale in locales {
            assert!(hover.contents.value.contains(&format!("lang/{locale}/messages.php")));
        }
    }
}

async fn settled_diagnostics(
    events: &Mutex<Vec<IdeEvent>>,
    changed: &tokio::sync::Notify,
    path: &std::path::Path,
) -> Vec<ide_core::Diagnostic> {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let next = changed.notified();
            tokio::pin!(next);
            next.as_mut().enable();
            let current = events.lock().iter().rev().find_map(|event| match event {
                IdeEvent::Diagnostics { path: target, items, .. } if target.0 == path => Some(items.clone()),
                _ => None,
            });
            if let Some(items) = current
                && items.iter().any(|d| {
                    d.code.as_deref() == Some("invalid_laravel_trans") && d.message.contains("messages.control_missing")
                })
            {
                return items;
            }
            next.await;
        }
    })
    .await
    .expect("same-document helper warning must prove settled diagnostic publication")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and Laravel 13.37 vendor fixture"]
async fn typed_translation_navigation_keeps_exact_callers_diagnostics_dirty_dependencies_and_restart() {
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
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let mut files: Vec<(String, String)> = [
        ("composer.json", COMPOSER),
        ("lang/en/messages.php", EN),
        ("lang/fr/messages.php", FR),
        ("app/Shared/Selected.php", SELECTED),
        ("app/SelectedCaller.php", CALLER),
        ("helper.php", HELPER),
        ("app/Diagnostics.php", DIAGNOSTICS),
    ]
    .into_iter()
    .map(|(n, t)| (n.into(), t.into()))
    .collect();
    let sources: Vec<_> = CASES
        .iter()
        .enumerate()
        .map(|(index, (body, positive))| {
            let text = PRELUDE.replace("namespace App;", &format!("namespace App\\TranslationNav{index};")) + body;
            (format!("app/Case{index}.php"), text, *positive)
        })
        .collect();
    files.extend(sources.iter().map(|(n, t, _)| (n.clone(), t.clone())));
    for (name, text) in &files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    std::os::unix::fs::symlink(vendor, root.join("vendor")).unwrap();
    let engine = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").expect("pinned PHPantom"));
    let events: Arc<Mutex<Vec<IdeEvent>>> = Arc::default();
    let changed = Arc::new(tokio::sync::Notify::new());
    let capture = events.clone();
    let signal = changed.clone();
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![route_handler_language(&engine)])),
        Arc::new(move |event| {
            capture.lock().push(event);
            signal.notify_waiters();
        }),
    );
    let outcome = std::panic::AssertUnwindSafe(async {
        let docs: Vec<_> = sources
            .iter()
            .map(|(name, text, _)| {
                let doc = DocId::next();
                mgr.did_open(doc, &root.join(name), &LanguageId::new("php"), 1, text);
                doc
            })
            .collect();
        for (doc, (_, text, positive)) in docs.iter().zip(&sources) {
            navigation(&mgr, *doc, text, &root, if *positive { &["en", "fr"] } else { &[] }).await;
        }
        let refs = mgr
            .references(docs[0], pos(&sources[0].1, "messages.hello"), true, true, Default::default())
            .await
            .unwrap();
        assert!(refs.incomplete.is_none(), "typed translation caller scan must be complete: {refs:?}");
        let mut found: Vec<_> = refs.locations.iter().map(location_key).collect();
        let mut expected: Vec<_> = sources
            .iter()
            .filter(|(_, _, positive)| *positive)
            .map(|(name, text, _)| expected_key(&root, name, text, "messages.hello"))
            .collect();
        for (name, text, needle) in [
            ("helper.php", HELPER, "messages.hello"),
            ("app/SelectedCaller.php", CALLER, "messages.hello"),
            ("lang/en/messages.php", EN, "hello"),
            ("lang/fr/messages.php", FR, "hello"),
        ] {
            expected.push(expected_key(&root, name, text, needle));
        }
        found.sort();
        expected.sort();
        assert_eq!(
            found, expected,
            "typed translation client references must retain every exact caller and refuse ambiguous peers"
        );
        assert_eq!(found.len(), 17);
        for (name, text) in [("lang/en/messages.php", EN), ("lang/fr/messages.php", FR)] {
            let catalog = DocId::next();
            mgr.did_open(catalog, &root.join(name), &LanguageId::new("php"), 1, text);
            for include_declaration in [false, true] {
                let refs = mgr
                    .references(catalog, pos(text, "hello"), include_declaration, true, Default::default())
                    .await
                    .unwrap();
                assert!(refs.incomplete.is_none(), "catalog-origin scan must retain completeness metadata");
                let mut actual: Vec<_> = refs.locations.iter().map(location_key).collect();
                let mut expected = expected.clone();
                if !include_declaration {
                    expected.retain(|target| !target.0.starts_with(root.join("lang")));
                }
                actual.sort();
                expected.sort();
                assert_eq!(
                    actual, expected,
                    "client catalog-origin references must match exact callers and declaration policy"
                );
            }
            mgr.did_close(catalog);
        }

        let diagnostic_doc = DocId::next();
        mgr.did_open(diagnostic_doc, &root.join("app/Diagnostics.php"), &LanguageId::new("php"), 1, DIAGNOSTICS);
        let items = settled_diagnostics(&events, &changed, &root.join("app/Diagnostics.php")).await;
        let selected: Vec<_> = items.iter().filter(|d| d.code.as_deref() == Some("invalid_laravel_trans")).collect();
        assert_eq!(selected.len(), 2, "{items:?}");
        for key in ["messages.control_missing", "messages.absent"] {
            let d = selected.iter().find(|d| d.message.contains(key)).unwrap();
            assert_eq!(d.range, range(DIAGNOSTICS, key));
        }
        let caller = DocId::next();
        mgr.did_open(caller, &root.join("app/SelectedCaller.php"), &LanguageId::new("php"), 1, CALLER);
        navigation(&mgr, caller, CALLER, &root, &["en", "fr"]).await;
        let dependency = DocId::next();
        mgr.did_open(dependency, &root.join("app/Shared/Selected.php"), &LanguageId::new("php"), 1, SELECTED);
        replace(&mgr, dependency, 2, SELECTED, FOREIGN);
        navigation(&mgr, caller, CALLER, &root, &[]).await;
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        navigation(&mgr, caller, CALLER, &root, &[]).await;
        mgr.did_close(dependency);
        navigation(&mgr, caller, CALLER, &root, &["en", "fr"]).await;
        let catalog = DocId::next();
        mgr.did_open(catalog, &root.join("lang/en/messages.php"), &LanguageId::new("php"), 1, EN);
        let dirty = EN.replace("'hello'", "'other'");
        assert_eq!(dirty.len(), EN.len());
        replace(&mgr, catalog, 2, EN, &dirty);
        navigation(&mgr, caller, CALLER, &root, &["fr"]).await;
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        navigation(&mgr, caller, CALLER, &root, &["fr"]).await;
        mgr.did_close(catalog);
        navigation(&mgr, caller, CALLER, &root, &["en", "fr"]).await;
        mgr.did_close(caller);
        let reopened = DocId::next();
        mgr.did_open(reopened, &root.join("app/SelectedCaller.php"), &LanguageId::new("php"), 1, CALLER);
        navigation(&mgr, reopened, CALLER, &root, &["en", "fr"]).await;
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    assert!(mgr.servers().iter().all(|s| s.status == ServerStatus::Stopped && s.pid.is_none()));
    for (name, text) in &files {
        assert_eq!(
            &std::fs::read_to_string(root.join(name)).unwrap(),
            text,
            "typed navigation must preserve disk originals"
        );
    }
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and Laravel 13.37 vendor fixture"]
async fn catalog_client_tracks_unsaved_php_json_origins_restart_and_close() {
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").expect("Laravel vendor fixture"))
        .canonicalize()
        .unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let source = "<?php /* 😀 */ __('Fresh 😀'); trans('messages.draft');";
    let php = "<?php /* 😀 */ return ['draft'=>'PHP value'];";
    let json = r#"{"Fresh 😀":"JSON value"}"#;
    for (name, text) in [
        ("composer.json", COMPOSER),
        ("app/Caller.php", source),
        ("lang/en/control.php", "<?php return [];"),
        ("lang/en.json", "{}"),
    ] {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    std::os::unix::fs::symlink(vendor, root.join("vendor")).unwrap();
    let engine = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").expect("pinned PHPantom"));
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![route_handler_language(&engine)])),
        Arc::new(|_| {}),
    );
    let outcome = std::panic::AssertUnwindSafe(async {
        let caller = DocId::next();
        mgr.did_open(caller, &root.join("app/Caller.php"), &LanguageId::new("php"), 1, source);
        let php_doc = DocId::next();
        mgr.did_open(php_doc, &root.join("lang/en/messages.php"), &LanguageId::new("php"), 1, php);
        let json_doc = DocId::next();
        mgr.did_open(json_doc, &root.join("lang/fr.json"), &LanguageId::new("json"), 1, json);
        let cases = [
            (php_doc, "lang/en/messages.php", php, "draft", "messages.draft", "PHP value"),
            (json_doc, "lang/fr.json", json, "Fresh 😀", "Fresh 😀", "JSON value"),
        ];
        for restart in [false, true] {
            if restart {
                mgr.restart(mgr.servers()[0].id).await.unwrap();
            }
            for (doc, path, text, needle, key, value) in cases {
                let mut expected =
                    vec![expected_key(&root, "app/Caller.php", source, key), expected_key(&root, path, text, needle)];
                expected.sort();
                let targets = mgr.goto(caller, pos(source, key), GotoKind::Definition).await.unwrap();
                assert_eq!(
                    targets.iter().map(location_key).collect::<Vec<_>>(),
                    vec![expected_key(&root, path, text, needle)],
                    "caller must see unsaved catalog without a catalog request or disk file"
                );
                for origin in [caller, doc] {
                    let at = if origin == caller { pos(source, key) } else { pos(text, needle) };
                    let refs = mgr.references(origin, at, true, true, Default::default()).await.unwrap();
                    assert!(refs.incomplete.is_none());
                    let mut actual: Vec<_> = refs.locations.iter().map(location_key).collect();
                    actual.sort();
                    assert_eq!(
                        actual, expected,
                        "client unsaved PHP/JSON catalog origin must retain exact references after restart"
                    );
                }
                let hover = mgr
                    .hover(caller, pos(source, key), Default::default())
                    .await
                    .unwrap()
                    .expect("unsaved catalog hover");
                assert!(hover.contents.value.contains(value), "{hover:?}");
                let list = settled_completion(&mgr, caller, pos(source, key)).await;
                assert!(list.items.iter().any(|item| item.label == key), "{list:?}");
                assert!(!root.join(path).exists());
            }
        }
        mgr.did_close(php_doc);
        mgr.did_close(json_doc);
        for (_, path, _, _, key, _) in cases {
            assert!(mgr.goto(caller, pos(source, key), GotoKind::Definition).await.unwrap().is_empty());
            let refs = mgr.references(caller, pos(source, key), true, true, Default::default()).await.unwrap();
            assert!(refs.incomplete.is_none());
            assert_eq!(
                refs.locations.iter().map(location_key).collect::<Vec<_>>(),
                vec![expected_key(&root, "app/Caller.php", source, key)],
                "closing an unsaved catalog must retire its declaration"
            );
            assert!(!root.join(path).exists());
        }
        assert_eq!(std::fs::read_to_string(root.join("app/Caller.php")).unwrap(), source);
        assert_eq!(std::fs::read_to_string(root.join("lang/en.json")).unwrap(), "{}");
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    assert!(mgr.servers().iter().all(|s| s.status == ServerStatus::Stopped && s.pid.is_none()));
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
