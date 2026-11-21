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
        for (name, text) in [("lang/en/messages.php", EN), ("lang/fr/messages.php", FR)] {
            let catalog = f.project.documents.open(&f.root.join(name)).unwrap();
            for include_declaration in [false, true] {
                let refs = ide_host::ops::lsp::lsp_references(
                    &f.host,
                    "main",
                    RequestId::next(),
                    catalog.id,
                    key_range(text, "hello").start,
                    include_declaration,
                    true,
                    true,
                    ide_host::ops::Channel::new(|_| {}),
                )
                .await
                .unwrap();
                assert!(refs.incomplete.is_none());
                let mut actual: Vec<_> =
                    refs.locations.into_iter().map(|target| (target.path.to_path_buf(), target.range)).collect();
                let mut expected = expected.clone();
                if !include_declaration {
                    expected.retain(|target| !target.0.starts_with(f.root.join("lang")));
                }
                actual.sort();
                expected.sort();
                assert_eq!(
                    actual, expected,
                    "Host catalog-origin references must retain exact managed callers and declaration policy"
                );
            }
            f.project.documents.close(catalog.id);
        }

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

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and Laravel 13.37 vendor fixture"]
async fn catalog_host_tracks_managed_php_json_changes_restart_and_discard() {
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").expect("Laravel vendor fixture"))
        .canonicalize()
        .unwrap();
    let caller = "<?php /* 😀 */ __('Fresh 😀'); trans('messages.draft');";
    let php = "<?php /* 😀 */ return ['draft'=>'PHP value'];";
    let json = r#"{"Fresh 😀":"JSON value"}"#;
    let extra = vec![("lang/en/messages.php".into(), "<?php return [];".into()), ("lang/fr.json".into(), "{}".into())];
    let (f, ()) = Fixture::new_at_with_fixture_setup(
        caller,
        &extra,
        "app/Caller.php",
        |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
        |_, _| (),
    );
    let outcome = std::panic::AssertUnwindSafe(async {
        let php_doc = f.project.documents.open(&f.root.join("lang/en/messages.php")).unwrap();
        assert_eq!(php_doc.language, ide_core::LanguageId::new("php"));
        let json_doc = f.project.documents.open(&f.root.join("lang/fr.json")).unwrap();
        assert_eq!(json_doc.language, ide_core::LanguageId::new("json"));
        f.project.documents.replace_all(php_doc.id, php).unwrap();
        f.project.documents.replace_all(json_doc.id, json).unwrap();
        let cases = [
            (php_doc.id, "lang/en/messages.php", php, "draft", "messages.draft", "PHP value"),
            (json_doc.id, "lang/fr.json", json, "Fresh 😀", "Fresh 😀", "JSON value"),
        ];
        for restart in [false, true] {
            if restart {
                f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
            }
            for (doc, path, text, needle, key, value) in cases {
                let targets = ide_host::ops::lsp::lsp_goto(
                    &f.host,
                    "main",
                    f.doc,
                    key_range(caller, key).start,
                    GotoKind::Definition,
                )
                .await
                .unwrap();
                assert_eq!(
                    targets.len(),
                    1,
                    "managed dirty catalog must be visible from its caller without a resource query"
                );
                assert_eq!(targets[0].path.to_path_buf(), f.root.join(path));
                assert_eq!(targets[0].range, key_range(text, needle));
                let mut expected = vec![
                    (f.root.join("app/Caller.php"), key_range(caller, key)),
                    (f.root.join(path), key_range(text, needle)),
                ];
                expected.sort();
                for origin in [f.doc, doc] {
                    let position =
                        if origin == f.doc { key_range(caller, key).start } else { key_range(text, needle).start };
                    let refs = ide_host::ops::lsp::lsp_references(
                        &f.host,
                        "main",
                        RequestId::next(),
                        origin,
                        position,
                        true,
                        true,
                        true,
                        ide_host::ops::Channel::new(|_| {}),
                    )
                    .await
                    .unwrap();
                    assert!(refs.incomplete.is_none());
                    let mut actual: Vec<_> =
                        refs.locations.into_iter().map(|target| (target.path.to_path_buf(), target.range)).collect();
                    actual.sort();
                    assert_eq!(
                        actual, expected,
                        "Host dirty PHP/JSON catalog origin must preserve exact managed references"
                    );
                }
                let hover = ide_host::ops::lsp::lsp_hover(
                    &f.host,
                    "main",
                    RequestId::next(),
                    f.doc,
                    key_range(caller, key).start,
                )
                .await
                .unwrap()
                .expect("managed catalog hover");
                assert!(hover.contents.value.contains(value), "{hover:?}");
                let list = f.complete(key_range(caller, key).start).await;
                assert!(list.items.iter().any(|item| item.label == key));
                let snapshot = f.project.documents.snapshot(doc).unwrap();
                assert!(snapshot.dirty);
                assert_eq!(snapshot.text, text);
            }
        }
        f.project.documents.close(php_doc.id);
        f.project.documents.close(json_doc.id);
        for (_, path, _, _, key, _) in cases {
            assert!(
                ide_host::ops::lsp::lsp_goto(
                    &f.host,
                    "main",
                    f.doc,
                    key_range(caller, key).start,
                    GotoKind::Definition
                )
                .await
                .unwrap()
                .is_empty()
            );
            let refs = ide_host::ops::lsp::lsp_references(
                &f.host,
                "main",
                RequestId::next(),
                f.doc,
                key_range(caller, key).start,
                true,
                true,
                true,
                ide_host::ops::Channel::new(|_| {}),
            )
            .await
            .unwrap();
            assert!(refs.incomplete.is_none());
            assert_eq!(refs.locations.len(), 1);
            assert_eq!(refs.locations[0].path.to_path_buf(), f.root.join("app/Caller.php"));
            assert_eq!(refs.locations[0].range, key_range(caller, key));
            assert_eq!(
                std::fs::read_to_string(f.root.join(path)).unwrap(),
                if path.ends_with(".json") { "{}" } else { "<?php return [];" }
            );
        }
        assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, caller);
        assert_eq!(std::fs::read_to_string(f.root.join("app/Caller.php")).unwrap(), caller);
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom 188, owned PHP and Laravel 13.37 vendor fixture"]
async fn registered_json_host_find_usages_tracks_dirty_keys_restart_and_discard() {
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").expect("Laravel vendor fixture"))
        .canonicalize()
        .unwrap();
    let caller = "<?php /* 😀 */ __('Greeting 😀');";
    let json = r#"{"Greeting 😀":"Disk welcome"}"#;
    let draft = r#"{"Draft 😀":"Private draft"}"#;
    let provider = r#"<?php namespace App; class Provider extends \Illuminate\Support\ServiceProvider {public function boot(): void {$this->loadJsonTranslationsFrom(__DIR__.'/../packages/billing/json');}}"#;
    let extra = vec![
        (
            "composer.json".into(),
            r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#.into(),
        ),
        ("app/Provider.php".into(), provider.into()),
        ("bootstrap/providers.php".into(), "<?php return [App\\Provider::class];".into()),
        ("packages/billing/composer.json".into(), r#"{"name":"acme/billing"}"#.into()),
        ("packages/billing/json/en.json".into(), json.into()),
    ];
    let (f, ()) = Fixture::new_at_with_fixture_setup(
        caller,
        &extra,
        "app/Caller.php",
        |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
        |_, _| (),
    );
    let outcome = std::panic::AssertUnwindSafe(async {
        let catalog = f.project.documents.open(&f.root.join("packages/billing/json/en.json")).unwrap();
        assert_eq!(catalog.language.as_str(), "json");
        for restart in [false, true] {
            if restart {
                f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
            }
            for text in [json, draft, json] {
                f.project.documents.replace_all(catalog.id, text).unwrap();
                let key = if text == draft { "Draft 😀" } else { "Greeting 😀" };
                for include in [false, true] {
                    let refs = ide_host::ops::lsp::lsp_references(
                        &f.host,
                        "main",
                        RequestId::next(),
                        catalog.id,
                        key_range(text, key).start,
                        include,
                        true,
                        true,
                        ide_host::ops::Channel::new(|_| {}),
                    )
                    .await
                    .unwrap();
                    assert!(refs.incomplete.is_none());
                    let mut found: Vec<_> = refs.locations.iter().map(|l| (l.path.to_path_buf(), l.range)).collect();
                    let mut expected = Vec::new();
                    if text == json {
                        expected.push((f.root.join("app/Caller.php"), key_range(caller, "Greeting 😀")));
                    }
                    if include {
                        expected.push((f.root.join("packages/billing/json/en.json"), key_range(text, key)));
                    }
                    found.sort();
                    expected.sort();
                    assert_eq!(found, expected);
                }
            }
        }
        f.project.documents.replace_all(catalog.id, draft).unwrap();
        f.project.documents.close(catalog.id);
        let reopened = f.project.documents.open(&f.root.join("packages/billing/json/en.json")).unwrap();
        let refs = ide_host::ops::lsp::lsp_references(
            &f.host,
            "main",
            RequestId::next(),
            reopened.id,
            key_range(json, "Greeting 😀").start,
            false,
            true,
            true,
            ide_host::ops::Channel::new(|_| {}),
        )
        .await
        .unwrap();
        assert_eq!(refs.locations.len(), 1);
        assert_eq!(refs.locations[0].path.to_path_buf(), f.root.join("app/Caller.php"));
        for (name, text) in &extra {
            assert_eq!(&std::fs::read_to_string(f.root.join(name)).unwrap(), text);
        }
        assert_eq!(std::fs::read_to_string(f.root.join("app/Caller.php")).unwrap(), caller);
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom 188, owned PHP and Laravel 13.37 vendor fixture"]
async fn registered_json_host_retires_managed_overlays_before_held_real_reference_replies() {
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").expect("Laravel vendor"))
        .canonicalize()
        .unwrap();
    let installed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(vendor.join("composer/installed.json")).unwrap()).unwrap();
    assert!(
        installed["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|package| package["name"] == "laravel/framework" && package["version"] == "v13.37.0")
    );
    let engine = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").expect("pinned PHPantom"))
        .canonicalize()
        .unwrap();
    let caller = "<?php /* 😀 */ __('Greeting 😀');";
    let json = r#"{"Greeting 😀":"Disk welcome"}"#;
    let overlay = r#"{"Greeting 😀":"Private overlay"}"#;
    let provider = r#"<?php namespace App; class Provider extends \Illuminate\Support\ServiceProvider {public function boot(): void {$this->loadJsonTranslationsFrom(__DIR__.'/../packages/billing/json');}}"#;
    for mutation in ["edit", "close"] {
        let extra = vec![
            (
                "composer.json".into(),
                r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#
                    .into(),
            ),
            ("app/Provider.php".into(), provider.into()),
            ("bootstrap/providers.php".into(), "<?php return [App\\Provider::class];".into()),
            ("packages/billing/composer.json".into(), r#"{"name":"acme/billing"}"#.into()),
            ("packages/billing/json/en.json".into(), json.into()),
        ];
        let engine = engine.clone();
        let (f, control) = Fixture::new_at_with_fixture_setup(
            caller,
            &extra,
            "app/Caller.php",
            |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
            move |project, temp| {
                let control = temp.join("control");
                std::fs::create_dir(&control).unwrap();
                std::fs::write(control.join("methods.json"), r#"["textDocument/references"]"#).unwrap();
                let proxy = temp.join("json-session-replies.py");
                std::fs::write(
                    &proxy,
                    include_str!("../../../ide-lsp/tests/fixtures/phpantom-json-session-replies.py"),
                )
                .unwrap();
                let selected = control.clone();
                project.lsp.set_config_hook(Box::new(move |cfg, _| {
                    if cfg.name == "phpantom" {
                        let args = std::mem::take(&mut cfg.args);
                        cfg.command = std::env::var("OPENSTORM_TEST_PYTHON").unwrap_or_else(|_| "python3".into());
                        cfg.args = vec![
                            "-I".into(),
                            proxy.to_string_lossy().into_owned(),
                            engine.to_string_lossy().into_owned(),
                            selected.to_string_lossy().into_owned(),
                        ];
                        cfg.args.extend(args);
                    }
                }));
                control
            },
        );
        let outcome=std::panic::AssertUnwindSafe(async{
            let catalog=f.project.documents.open(&f.root.join("packages/billing/json/en.json")).unwrap();f.project.documents.replace_all(catalog.id,overlay).unwrap();
            let pending=ide_host::ops::lsp::lsp_references(&f.host,"main",RequestId::next(),catalog.id,key_range(overlay,"Greeting 😀").start,false,true,true,ide_host::ops::Channel::new(|_|{}));tokio::pin!(pending);
            let wait=async{for _ in 0..750 {if control.join("references.json").is_file(){return;}tokio::time::sleep(Duration::from_millis(20)).await;}panic!("Host real reference reply was not held");};
            tokio::select! {result=&mut pending=>panic!("Host references returned before the gate: {result:?}"),()=wait=>{}}
            let captured:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(control.join("references.json")).unwrap()).unwrap();assert_eq!(captured["result"].as_array().unwrap().len(),1,"{captured}");
            let at=key_range(caller,"Greeting 😀").start;
            let hover=ide_host::ops::lsp::lsp_hover(&f.host,"main",RequestId::next(),f.doc,at).await.unwrap().unwrap();assert!(hover.contents.value.contains("Private overlay"),"{hover:?}");
            if mutation=="edit" {f.project.documents.replace_all(catalog.id,r#"{"Greeting 😀":"Current overlay"}"#).unwrap();} else {f.project.documents.close(catalog.id);}
            let result=tokio::time::timeout(Duration::from_secs(2),&mut pending).await.expect("managed document lifecycle must retire the operation before its real reply is released");assert!(result.is_err(),"{result:?}");
            let hover=ide_host::ops::lsp::lsp_hover(&f.host,"main",RequestId::next(),f.doc,at).await.unwrap().unwrap();let expected=if mutation=="edit" {"Current overlay"} else {"Disk welcome"};assert!(hover.contents.value.contains(expected),"{mutation}: {hover:?}");assert!(!hover.contents.value.contains("Private overlay"));
            std::fs::write(control.join("references.release"),"release").unwrap();for(name,text)in &extra{assert_eq!(&std::fs::read_to_string(f.root.join(name)).unwrap(),text);}assert_eq!(std::fs::read_to_string(f.root.join("app/Caller.php")).unwrap(),caller);
        }).catch_unwind().await;
        std::fs::write(control.join("references.release"), "release").unwrap();
        f.finish(outcome).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom 188, owned PHP and Laravel 13.37 vendor fixture"]
async fn ordinary_php_host_navigation_uses_managed_unsaved_catalogs_without_find_usages() {
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").expect("Laravel vendor"))
        .canonicalize()
        .unwrap();
    let installed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(vendor.join("composer/installed.json")).unwrap()).unwrap();
    assert!(
        installed["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|package| package["name"] == "laravel/framework" && package["version"] == "v13.37.0")
    );
    let engine = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").expect("pinned PHPantom"))
        .canonicalize()
        .unwrap();
    let caller = "<?php /* 😀 */ __('Greeting 😀');";
    let json = r#"{"Greeting 😀":"Disk welcome"}"#;
    let overlay = r#"{"Greeting 😀":"Private overlay"}"#;
    let updated = r#"{"Greeting 😀":"Updated private value"}"#;
    let provider = r#"<?php namespace App; class Provider extends \Illuminate\Support\ServiceProvider {public function boot(): void {$this->loadJsonTranslationsFrom(__DIR__.'/../packages/billing/json');}}"#;
    let extra = vec![
        (
            "composer.json".into(),
            r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#.into(),
        ),
        ("app/Provider.php".into(), provider.into()),
        ("app/SecondCaller.php".into(), "<?php namespace App; class SecondCaller {}".into()),
        ("bootstrap/providers.php".into(), "<?php return [App\\Provider::class];".into()),
        ("packages/billing/composer.json".into(), r#"{"name":"acme/billing"}"#.into()),
        ("packages/billing/json/en.json".into(), json.into()),
    ];
    let configuration_revision = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let configured_revision = configuration_revision.clone();
    let (f, control) = Fixture::new_at_with_fixture_setup(
        caller,
        &extra,
        "app/Caller.php",
        |root| {
            std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap();
            std::fs::create_dir(root.join("additional")).unwrap();
        },
        move |project, temp| {
            let control = temp.join("control");
            std::fs::create_dir(&control).unwrap();
            std::fs::write(control.join("methods.json"), "[]").unwrap();
            let proxy = temp.join("json-session-replies.py");
            std::fs::write(&proxy, include_str!("../../../ide-lsp/tests/fixtures/phpantom-json-session-replies.py"))
                .unwrap();
            let selected = control.clone();
            project.lsp.set_config_hook(Box::new(move |cfg, _| {
                if cfg.name == "phpantom" {
                    cfg.settings = Some(serde_json::json!({"openstormTest":{"revision":
                        configured_revision.load(std::sync::atomic::Ordering::SeqCst)}}));
                    let args = std::mem::take(&mut cfg.args);
                    cfg.command = std::env::var("OPENSTORM_TEST_PYTHON").unwrap_or_else(|_| "python3".into());
                    cfg.args = vec![
                        "-I".into(),
                        proxy.to_string_lossy().into_owned(),
                        engine.to_string_lossy().into_owned(),
                        selected.to_string_lossy().into_owned(),
                    ];
                    cfg.args.extend(args);
                }
            }));
            control
        },
    );
    let outcome = std::panic::AssertUnwindSafe(async {
        let path = f.root.join("packages/billing/json/en.json");
        tokio::time::timeout(Duration::from_secs(10), async {
            while !f.project.lsp.servers().iter().any(|server| {
                server.name == "phpantom"
                    && matches!(
                        server.status,
                        ide_core::event::ServerStatus::Ready | ide_core::event::ServerStatus::Busy
                    )
            }) {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("managed PHP consumer ready before catalog opening");
        let frames = || {
            std::fs::read_to_string(control.join("events.jsonl"))
                .unwrap_or_default()
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .filter(|event| event["uri"] == ide_lsp::path_to_uri_string(&path))
                .collect::<Vec<_>>()
        };
        let opens = || frames().iter().filter(|event| event["method"] == "textDocument/didOpen").count();
        let catalog = f.project.documents.open(&path).unwrap();
        assert_eq!(catalog.language.as_str(), "json");
        f.project.documents.replace_all(catalog.id, overlay).unwrap();
        let private_version = f.project.documents.version(catalog.id).unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while !frames()
                .iter()
                .any(|event| event["method"] == "textDocument/didOpen" && event["version"] == private_version)
            {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("current unsaved catalog must be admitted before the first PHP command");
        let opens_before_attach = opens();
        let second = f.project.documents.open(&f.root.join("app/SecondCaller.php")).unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while !frames()
                .iter()
                .filter(|event| event["method"] == "textDocument/didOpen")
                .skip(opens_before_attach)
                .any(|event| event["version"] == private_version)
            {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("managed PHP attachment must re-admit the private catalog before another PHP command");
        let second_saved = "<?php namespace App; class SecondCaller { /* saved */ }";
        for action in ["edit", "save", "close"] {
            let before = opens();
            match action {
                "edit" => {
                    f.project.documents.replace_all(second.id, second_saved).unwrap();
                }
                "save" => {
                    f.project.documents.save(second.id).unwrap();
                    let source = f.root.join("app/SecondCaller.php");
                    assert_eq!(std::fs::read_to_string(&source).unwrap(), second_saved);
                    let lint =
                        std::process::Command::new(std::env::var_os("OPENSTORM_TEST_PHP_BIN").expect("owned PHP"))
                            .args(["-n", "-l"])
                            .arg(source)
                            .output()
                            .unwrap();
                    assert!(lint.status.success(), "{}", String::from_utf8_lossy(&lint.stderr));
                }
                "close" => {
                    f.project.documents.close(second.id);
                }
                _ => unreachable!(),
            }
            tokio::time::timeout(Duration::from_secs(5), async {
                while !frames()
                    .iter()
                    .filter(|event| event["method"] == "textDocument/didOpen")
                    .skip(before)
                    .any(|event| event["version"] == private_version)
                {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .unwrap_or_else(|_| panic!("managed PHP {action} must re-admit the private catalog before navigation"));
        }
        let pid_before_reconfiguration =
            f.project.lsp.servers().into_iter().find(|server| server.name == "phpantom").unwrap().pid;
        assert!(pid_before_reconfiguration.is_some());
        for change in ["settings", "unchanged settings", "content roots"] {
            let before = opens();
            match change {
                "settings" => {
                    configuration_revision.store(1, std::sync::atomic::Ordering::SeqCst);
                    f.project.lsp.reconfigure().await;
                }
                "unchanged settings" => f.project.lsp.reconfigure().await,
                "content roots" => f.project.lsp.set_content_roots(vec![f.root.join("additional")]),
                _ => unreachable!(),
            }
            tokio::time::timeout(Duration::from_secs(5), async {
                while !frames()
                    .iter()
                    .filter(|event| event["method"] == "textDocument/didOpen")
                    .skip(before)
                    .any(|event| event["version"] == private_version)
                {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .unwrap_or_else(|_| panic!("managed {change} must re-admit the private catalog before PHP navigation"));
            assert_eq!(
                f.project.lsp.servers().into_iter().find(|server| server.name == "phpantom").unwrap().pid,
                pid_before_reconfiguration
            );
        }
        let opens_before_navigation = opens();
        for _ in 0..2 {
            let hover = ide_host::ops::lsp::lsp_hover(
                &f.host,
                "main",
                RequestId::next(),
                f.doc,
                key_range(caller, "Greeting 😀").start,
            )
            .await
            .unwrap()
            .unwrap();
            assert!(hover.contents.value.contains("Private overlay"), "{hover:?}");
            let targets = ide_host::ops::lsp::lsp_goto(
                &f.host,
                "main",
                f.doc,
                key_range(caller, "Greeting 😀").start,
                GotoKind::Definition,
            )
            .await
            .unwrap();
            assert_eq!(targets.len(), 1);
            assert_eq!(targets[0].path.to_path_buf(), path);
            assert_eq!(targets[0].range, key_range(overlay, "Greeting 😀"));
        }
        assert_eq!(opens(), opens_before_navigation, "normal Host navigation reuses the current admitted catalog");
        let before_edit = opens();
        f.project.documents.replace_all(catalog.id, updated).unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while opens() != before_edit + 1 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("managed edit must refresh before another PHP command");
        let frames_before_save = frames().len();
        let events_before_save = f.events.lock().unwrap().len();
        f.project.documents.save(catalog.id).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), updated);
        let watched_path = ide_core::IpcPath::from(path.as_path());
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let changed = f.events_changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if f.events.lock().unwrap()[events_before_save..].iter().any(|event| {
                    matches!(event, ide_core::IdeEvent::FsChanged { created, modified, .. }
                        if created.iter().chain(modified).any(|path| path == &watched_path))
                }) {
                    break;
                }
                changed.await;
            }
        })
        .await
        .expect("the saved catalog must publish a new watcher event");
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let all = frames();
                let saved = &all[frames_before_save..];
                let watched = saved.iter().rposition(|event| event["method"] == "workspace/didChangeWatchedFiles");
                let reopened = saved.iter().rposition(|event| event["method"] == "textDocument/didOpen");
                if watched.zip(reopened).is_some_and(|(watched, reopened)| reopened > watched)
                    && saved.last().is_some_and(|event| event["method"] == "textDocument/didOpen")
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("managed save must refresh after the actual watcher protocol frame, before another PHP command");
        let hover = ide_host::ops::lsp::lsp_hover(
            &f.host,
            "main",
            RequestId::next(),
            f.doc,
            key_range(caller, "Greeting 😀").start,
        )
        .await
        .unwrap()
        .unwrap();
        assert!(hover.contents.value.contains("Updated private value"));
        let draft = r#"{"Draft 😀":"Private draft"}"#;
        let draft_caller = caller.replace("Greeting 😀", "Draft 😀");
        f.project.documents.replace_all(catalog.id, draft).unwrap();
        f.project.documents.replace_all(f.doc, &draft_caller).unwrap();
        let hover = ide_host::ops::lsp::lsp_hover(
            &f.host,
            "main",
            RequestId::next(),
            f.doc,
            key_range(&draft_caller, "Draft 😀").start,
        )
        .await
        .unwrap()
        .unwrap();
        assert!(hover.contents.value.contains("Private draft"));
        let targets = ide_host::ops::lsp::lsp_goto(
            &f.host,
            "main",
            f.doc,
            key_range(&draft_caller, "Draft 😀").start,
            GotoKind::Definition,
        )
        .await
        .unwrap();
        assert_eq!(targets[0].path.to_path_buf(), path);
        assert_eq!(targets[0].range, key_range(draft, "Draft 😀"));
        let server = f.project.lsp.servers().into_iter().find(|server| server.name == "phpantom").unwrap().id;
        let opens_before_restart = opens();
        let restart_version = f.project.documents.version(catalog.id).unwrap();
        f.project.lsp.restart(server).await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while !frames()
                .iter()
                .filter(|event| event["method"] == "textDocument/didOpen")
                .skip(opens_before_restart)
                .any(|event| event["version"] == restart_version)
            {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("managed catalog must be re-admitted after restart before another PHP command");
        let hover = ide_host::ops::lsp::lsp_hover(
            &f.host,
            "main",
            RequestId::next(),
            f.doc,
            key_range(&draft_caller, "Draft 😀").start,
        )
        .await
        .unwrap()
        .unwrap();
        assert!(hover.contents.value.contains("Private draft"));
        f.project.documents.replace_all(f.doc, caller).unwrap();
        f.project.documents.close(catalog.id);
        let hover = ide_host::ops::lsp::lsp_hover(
            &f.host,
            "main",
            RequestId::next(),
            f.doc,
            key_range(caller, "Greeting 😀").start,
        )
        .await
        .unwrap()
        .unwrap();
        assert!(hover.contents.value.contains("Updated private value"));
        let reopened = f.project.documents.open(&path).unwrap();
        f.project.documents.replace_all(reopened.id, overlay).unwrap();
        let hover = ide_host::ops::lsp::lsp_hover(
            &f.host,
            "main",
            RequestId::next(),
            f.doc,
            key_range(caller, "Greeting 😀").start,
        )
        .await
        .unwrap()
        .unwrap();
        assert!(hover.contents.value.contains("Private overlay"));
        let refs = ide_host::ops::lsp::lsp_references(
            &f.host,
            "main",
            RequestId::next(),
            reopened.id,
            key_range(overlay, "Greeting 😀").start,
            false,
            true,
            true,
            ide_host::ops::Channel::new(|_| {}),
        )
        .await
        .unwrap();
        assert_eq!(refs.locations.len(), 1);
        assert_eq!(refs.locations[0].path.to_path_buf(), f.root.join("app/Caller.php"));
        assert_eq!(refs.locations[0].range, key_range(caller, "Greeting 😀"));
        for (name, text) in &extra {
            let expected = match name.as_str() {
                "packages/billing/json/en.json" => updated,
                "app/SecondCaller.php" => second_saved,
                _ => text,
            };
            assert_eq!(std::fs::read_to_string(f.root.join(name)).unwrap(), expected);
        }
        assert_eq!(std::fs::read_to_string(f.root.join("app/Caller.php")).unwrap(), caller);
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}
