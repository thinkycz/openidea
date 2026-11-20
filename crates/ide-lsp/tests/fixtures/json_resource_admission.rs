use super::*;
use futures::FutureExt;

const COMPOSER: &str =
    r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#;
const PROVIDER: &str = r#"<?php namespace App; class Provider extends \Illuminate\Support\ServiceProvider { public function boot(): void { $this->loadJsonTranslationsFrom(__DIR__.'/../packages/billing/json'); } }"#;
const CALLER: &str = "<?php /* 😀 */ __('Greeting 😀');";
const CATALOG: &str = r#"{"Greeting 😀":"Disk welcome"}"#;

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom 188 and Laravel 13.37 vendor fixture"]
async fn registered_json_admission_selects_consuming_server_tracks_provider_changes_and_restart() {
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
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let files = [
        ("a/composer.json", COMPOSER),
        ("a/app/Caller.php", CALLER),
        ("a/app/Provider.php", PROVIDER),
        ("a/bootstrap/providers.php", "<?php return [App\\Provider::class];"),
        ("a/packages/billing/composer.json", r#"{"name":"acme/billing"}"#),
        ("a/packages/billing/own.php", "<?php /* separate Composer graph */"),
        ("a/packages/billing/json/en.json", CATALOG),
        ("a/packages/billing/alternate/en.json", r#"{"Greeting 😀":"Alternate welcome"}"#),
        ("b/composer.json", COMPOSER),
        ("b/app/Caller.php", CALLER),
    ];
    let php = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHP_BIN").expect("fixture PHP syntax validator"));
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        if path.extension().is_some_and(|ext| ext == "php") {
            let check = std::process::Command::new(&php).args(["-n", "-l"]).arg(&path).output().unwrap();
            assert!(check.status.success(), "{name}: {}", String::from_utf8_lossy(&check.stderr));
        } else {
            serde_json::from_str::<serde_json::Value>(text).unwrap();
        }
    }
    for project in ["a", "b"] {
        std::os::unix::fs::symlink(&vendor, root.join(project).join("vendor")).unwrap();
    }
    let runtime=std::process::Command::new(&php).args(["-n","-d","zend.assertions=1","-d","assert.exception=1","-r",
        r#"require $argv[1].'/vendor/autoload.php'; $loader=new Illuminate\Translation\FileLoader(new Illuminate\Filesystem\Filesystem, $argv[1].'/lang'); $loader->addJsonPath($argv[1].'/packages/billing/json'); $translator=new Illuminate\Translation\Translator($loader,'en'); assert($translator->get('Greeting 😀')==='Disk welcome'); echo 'Laravel translation fixture passed';"#])
        .arg(root.join("a")).output().unwrap();
    assert!(runtime.status.success(), "{}", String::from_utf8_lossy(&runtime.stderr));
    assert_eq!(String::from_utf8(runtime.stdout).unwrap(), "Laravel translation fixture passed");

    let engine = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").expect("pinned PHPantom"));
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![route_handler_language(&engine)])),
        Arc::new(|_: IdeEvent| {}),
    );
    mgr.set_content_roots(vec![root.join("a"), root.join("b"), root.join("a/packages/billing")]);
    let outcome = std::panic::AssertUnwindSafe(async {
        let manifest = root.join("a/composer.json");
        let catalog = root.join("a/packages/billing/json/en.json");
        let json = DocId::next();
        mgr.did_open(json, &catalog, &LanguageId::new("json"), 1, r#"{"Greeting 😀":"Unshared private overlay"}"#);
        assert!(mgr.servers().is_empty(), "a custom catalog alone must not disclose its buffer to PHP");
        let catalog_first =
            mgr.php_json_translation_resource_from_composer(&manifest, &catalog, Default::default()).await.unwrap();
        assert_eq!(catalog_first.project_path(), manifest);
        assert_eq!(catalog_first.root_path(), catalog.parent());
        mgr.php_json_translation_resource_current(&catalog_first, Default::default()).await.unwrap();
        assert!(
            mgr.php_json_translation_resource_from_composer(
                &root.join("b/composer.json"),
                &catalog,
                Default::default()
            )
            .await
            .unwrap()
            .root_path()
            .is_none()
        );
        assert!(
            mgr.php_json_translation_resource_from_composer(
                &root.join("a/packages/billing/composer.json"),
                &catalog,
                Default::default()
            )
            .await
            .unwrap()
            .root_path()
            .is_none()
        );
        let composer_doc = DocId::next();
        mgr.did_open(composer_doc, &manifest, &LanguageId::new("json"), 1, COMPOSER);
        assert!(mgr.php_json_translation_resource_current(&catalog_first, Default::default()).await.is_err());
        let opened_manifest =
            mgr.php_json_translation_resource_from_composer(&manifest, &catalog, Default::default()).await.unwrap();
        assert_eq!(opened_manifest.root_path(), catalog.parent());
        let dirty_composer = COMPOSER.replace("app/", "elsewhere/");
        replace(&mgr, composer_doc, 2, COMPOSER, &dirty_composer);
        assert!(mgr.php_json_translation_resource_current(&opened_manifest, Default::default()).await.is_err());
        assert!(
            mgr.php_json_translation_resource_from_composer(&manifest, &catalog, Default::default()).await.is_err()
        );
        assert_eq!(std::fs::read_to_string(&manifest).unwrap(), COMPOSER);
        mgr.did_close(composer_doc);
        let closed_manifest =
            mgr.php_json_translation_resource_from_composer(&manifest, &catalog, Default::default()).await.unwrap();
        std::fs::write(&manifest, format!("{COMPOSER}\n")).unwrap();
        assert!(
            mgr.php_json_translation_resource_current(&closed_manifest, Default::default()).await.is_err(),
            "an unreported disk change must retire a Composer receipt"
        );
        std::fs::write(&manifest, COMPOSER).unwrap();
        mgr.php_files_changed(&[], std::slice::from_ref(&manifest), &[]);
        let restored_composer =
            mgr.php_json_translation_resource_from_composer(&manifest, &catalog, Default::default()).await.unwrap();
        assert_eq!(restored_composer.root_path(), catalog.parent());
        let refs = mgr
            .references(
                json,
                pos(r#"{"Greeting 😀":"Unshared private overlay"}"#, "Greeting 😀"),
                false,
                true,
                Default::default(),
            )
            .await
            .unwrap();
        assert!(refs.incomplete.is_none());
        assert_eq!(refs.locations.len(), 1, "catalog-first Find Usages must find its consuming application");
        assert_eq!(refs.locations[0].path.to_path_buf(), root.join("a/app/Caller.php"));
        assert_eq!(refs.locations[0].range.start, pos(CALLER, "Greeting 😀"));
        let dirty_json = r#"{"Draft 😀":"Private draft"}"#;
        replace(&mgr, json, 2, r#"{"Greeting 😀":"Unshared private overlay"}"#, dirty_json);
        assert!(
            mgr.references(json, pos(dirty_json, "Draft 😀"), false, true, Default::default())
                .await
                .unwrap()
                .locations
                .is_empty()
        );
        replace(&mgr, json, 3, dirty_json, r#"{"Greeting 😀":"Unshared private overlay"}"#);
        let a = DocId::next();
        let b = DocId::next();
        let package = DocId::next();
        let a_path = root.join("a/app/Caller.php");
        let package_path = root.join("a/packages/billing/own.php");
        let alternate = root.join("a/packages/billing/alternate/en.json");
        mgr.did_open(a, &a_path, &LanguageId::new("php"), 1, CALLER);
        mgr.did_open(b, &root.join("b/app/Caller.php"), &LanguageId::new("php"), 1, CALLER);
        mgr.did_open(package, &package_path, &LanguageId::new("php"), 1, "<?php /* separate Composer graph */");
        let receipt = mgr.php_json_translation_resource(a, &catalog, Default::default()).await.unwrap();
        assert_eq!(receipt.project_path(), a_path);
        assert_eq!(receipt.resource_path(), catalog);
        assert_eq!(receipt.root_path(), catalog.parent());
        mgr.php_json_translation_resource_current(&receipt, Default::default()).await.unwrap();
        assert!(
            mgr.php_json_translation_resource(b, &catalog, Default::default()).await.unwrap().root_path().is_none()
        );
        assert!(
            mgr.php_json_translation_resource(package, &catalog, Default::default())
                .await
                .unwrap()
                .root_path()
                .is_none()
        );
        let selected =
            mgr.servers().into_iter().find(|server| *server.root == root.join("a")).expect("consuming PHP server");
        assert!(
            mgr.servers().iter().any(|server| *server.root == root.join("a/packages/billing")),
            "nested Composer server must actually be present"
        );
        let draft = catalog.parent().unwrap().join("fr.json");
        assert_eq!(
            mgr.php_json_translation_resource(a, &draft, Default::default()).await.unwrap().root_path(),
            draft.parent()
        );
        assert!(!draft.exists());
        let unregistered = root.join("a/config/en.json");
        assert!(
            mgr.php_json_translation_resource(a, &unregistered, Default::default())
                .await
                .unwrap()
                .root_path()
                .is_none()
        );
        let hover = mgr.hover(a, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
        assert!(
            hover.contents.value.contains("Unshared private overlay"),
            "ordinary PHP hover must automatically admit its current custom catalog: {hover:?}"
        );
        assert!(!hover.contents.value.contains("Disk welcome"));

        let provider = DocId::next();
        let provider_path = root.join("a/app/Provider.php");
        mgr.did_open(provider, &provider_path, &LanguageId::new("php"), 1, PROVIDER);
        let changed = PROVIDER.replace("billing/json", "billing/alternate");
        replace(&mgr, provider, 2, PROVIDER, &changed);
        assert!(mgr.php_json_translation_resource_current(&receipt, Default::default()).await.is_err());
        assert!(
            mgr.php_json_translation_resource(a, &catalog, Default::default()).await.unwrap().root_path().is_none()
        );
        let changed_receipt = mgr.php_json_translation_resource(a, &alternate, Default::default()).await.unwrap();
        assert_eq!(changed_receipt.root_path(), alternate.parent());
        assert_eq!(std::fs::read_to_string(&provider_path).unwrap(), PROVIDER);
        mgr.did_close(provider);
        assert!(mgr.php_json_translation_resource_current(&changed_receipt, Default::default()).await.is_err());
        let restored = mgr.php_json_translation_resource(a, &catalog, Default::default()).await.unwrap();
        assert_eq!(restored.root_path(), catalog.parent());
        mgr.restart(selected.id).await.unwrap();
        assert!(mgr.php_json_translation_resource_current(&restored, Default::default()).await.is_err());
        let restarted = mgr.php_json_translation_resource(a, &catalog, Default::default()).await.unwrap();
        assert_eq!(restarted.root_path(), catalog.parent());
        mgr.php_json_translation_resource_current(&restarted, Default::default()).await.unwrap();
        let restarted_composer =
            mgr.php_json_translation_resource_from_composer(&manifest, &catalog, Default::default()).await.unwrap();
        assert_eq!(restarted_composer.project_path(), manifest);
        assert_eq!(restarted_composer.root_path(), catalog.parent());
        let refs = mgr
            .references(
                json,
                pos(r#"{"Greeting 😀":"Unshared private overlay"}"#, "Greeting 😀"),
                false,
                true,
                Default::default(),
            )
            .await
            .unwrap();
        assert_eq!(refs.locations.len(), 1);
        assert_eq!(refs.locations[0].path.to_path_buf(), root.join("a/app/Caller.php"));
        assert_eq!(std::fs::read_to_string(&catalog).unwrap(), CATALOG);
        mgr.did_close(json);
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
    for (name, text) in files {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text, "{name}");
    }
    assert!(!root.join("a/packages/billing/json/fr.json").exists());
}
