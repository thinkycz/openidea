use super::*;
use futures::FutureExt;
use sha2::{Digest, Sha256};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom 190 provider-source metadata"]
async fn provider_source_receipts_preserve_analyzed_hashes_and_retire_foreign_changes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let app = root.join("app");
    let external = root.join("shared");
    let provider_text = "<?php namespace Shared; class Provider {public function boot(): void {$this->loadJsonTranslationsFrom(__DIR__.'/../json');}}";
    let own_text = "<?php namespace App; class Provider {}";
    let files = [
        (
            "app/composer.json",
            r#"{"require":{"laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"src/","Shared\\":"../shared/src/"}}}"#,
        ),
        (
            "app/bootstrap/providers.php",
            "<?php return [App\\Provider::class, Shared\\Provider::class, Missing\\Provider::class];",
        ),
        ("app/src/Provider.php", own_text),
        ("shared/composer.json", r#"{"autoload":{"psr-4":{"Shared\\":"src/"}}}"#),
        ("shared/src/Provider.php", provider_text),
    ];
    let php = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHP_BIN").expect("fixture syntax validator"));
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        if path.extension().is_some_and(|ext| ext == "php") {
            let syntax = std::process::Command::new(&php).args(["-n", "-l"]).arg(&path).output().unwrap();
            assert!(syntax.status.success(), "{name}: {}", String::from_utf8_lossy(&syntax.stderr));
        } else {
            serde_json::from_str::<serde_json::Value>(text).unwrap();
        }
    }
    let engine = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").expect("pinned PHPantom"));
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![route_handler_language(&engine)])),
        Arc::new(|_: IdeEvent| {}),
    );
    mgr.set_content_roots(vec![app.clone(), external.clone()]);
    let outcome = std::panic::AssertUnwindSafe(async {
        let manifest = app.join("composer.json");
        let catalog = external.join("json/fr.json");
        let source = external.join("src/Provider.php");
        let first =
            mgr.php_json_translation_resource_from_composer(&manifest, &catalog, Default::default()).await.unwrap();
        assert_eq!(first.root_path(), catalog.parent());
        let sources = first.provider_sources().expect("engine190 advertises metadata");
        assert_eq!(sources.providers.len(), 2);
        assert_eq!(sources.unresolved, ["Missing\\Provider"]);
        assert!(!sources.limited);
        assert!(sources.incomplete());
        let shared = sources.providers.iter().find(|p| p.class_name == "Shared\\Provider").unwrap();
        assert_eq!(shared.path, source);
        assert_eq!(shared.sha256_utf8, <[u8; 32]>::from(Sha256::digest(provider_text.as_bytes())));
        mgr.php_json_translation_resource_current(&first, Default::default()).await.unwrap();
        let own = DocId::next();
        let own_path = app.join("src/Provider.php");
        let dirty = own_text.replace("{}", "{ /* unsaved 😀 */ }");
        mgr.did_open(own, &own_path, &LanguageId::new("php"), 1, &dirty);
        assert!(mgr.php_json_translation_resource_current(&first, Default::default()).await.is_err());
        let second = mgr.php_json_translation_resource(own, &catalog, Default::default()).await.unwrap();
        let own_source =
            second.provider_sources().unwrap().providers.iter().find(|p| p.class_name == "App\\Provider").unwrap();
        assert_eq!(own_source.sha256_utf8, <[u8; 32]>::from(Sha256::digest(dirty.as_bytes())));
        assert_eq!(std::fs::read_to_string(&own_path).unwrap(), own_text);
        mgr.php_json_translation_resource_current(&second, Default::default()).await.unwrap();
        let foreign = DocId::next();
        mgr.did_open(foreign, &source, &LanguageId::new("php"), 1, provider_text);
        assert!(mgr.php_json_translation_resource_current(&second, Default::default()).await.is_err());
        let current = mgr.php_json_translation_resource(own, &catalog, Default::default()).await.unwrap();
        mgr.php_json_translation_resource_current(&current, Default::default()).await.unwrap();
        let changed = provider_text.replace("../json", "../alternate");
        replace(&mgr, foreign, 2, provider_text, &changed);
        assert!(mgr.php_json_translation_resource_current(&current, Default::default()).await.is_err());
        assert_eq!(std::fs::read_to_string(&source).unwrap(), provider_text);
        assert!(!catalog.exists());
        assert_eq!(mgr.servers().len(), 2, "separate Composer graphs retain separate servers");
        mgr.did_close(foreign);
        mgr.did_close(own);
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}
