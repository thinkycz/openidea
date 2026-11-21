use super::*;
use futures::FutureExt;
use sha2::{Digest, Sha256};

const COMPOSER: &str = r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"src/","Shared\\":"../shared/src/"}}}"#;
const PROVIDER: &str = r#"<?php namespace Shared; class Provider extends \Illuminate\Support\ServiceProvider {public function boot(): void {$this->loadJsonTranslationsFrom(__DIR__.'/../json');}}"#;
const CALLER: &str = "<?php namespace App; class Caller {public function greet(): string {return __('Greeting 😀');}}";
const SECOND_PROVIDER: &str = r#"<?php namespace Shared; class SecondProvider extends \Illuminate\Support\ServiceProvider {public function boot(): void {$this->loadJsonTranslationsFrom(__DIR__.'/../json-b');}}"#;
const SECOND_CALLER: &str =
    "<?php namespace Second; class Caller {public function greet(): string {return __('Greeting B 😀');}}";
const SECOND_COMPOSER: &str = r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"Second\\":"src/","Shared\\":"../../shared/src/"}}}"#;
const ORIGINAL: &str = r#"{"Greeting 😀":"Private original"}"#;
const ALTERNATE: &str = r#"{"Greeting 😀":"Private alternate"}"#;

fn syntax_text(php: &std::path::Path, text: &str) {
    let mut child = std::process::Command::new(php)
        .args(["-n", "-l"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(&mut child.stdin.take().unwrap(), text.as_bytes()).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
}

fn events(control: &std::path::Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(control.join("events.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn event_at(event: &serde_json::Value, root: &std::path::Path, method: &str, path: &std::path::Path) -> bool {
    event["root"] == root.to_str().unwrap()
        && event["method"] == method
        && event["uri"] == ide_lsp::path_to_uri_string(path)
}

async fn until_events(
    control: &std::path::Path,
    check: impl Fn(&[serde_json::Value]) -> bool,
) -> Vec<serde_json::Value> {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let log = events(control);
            if check(&log) {
                return log;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("provider synchronization did not publish current protocol events")
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom191 and Laravel13.37 vendor fixture"]
async fn ordinary_php_navigation_uses_declared_foreign_provider_buffers_and_recovers() {
    let root_dir = tempfile::tempdir().unwrap();
    let root = root_dir.path().canonicalize().unwrap();
    let app = root.join("app");
    let shared = root.join("shared");
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").expect("Laravel vendor fixture"))
        .canonicalize()
        .unwrap();
    let php = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHP_BIN").expect("fixture syntax validator"));
    let installed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(vendor.join("composer/installed.json")).unwrap()).unwrap();
    assert!(
        installed["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "laravel/framework" && p["version"] == "v13.37.0")
    );
    for (name, text) in [
        ("app/composer.json", COMPOSER),
        ("app/bootstrap/providers.php", "<?php return [Shared\\Provider::class];"),
        ("app/src/Caller.php", CALLER),
        ("shared/composer.json", r#"{"name":"acme/shared","autoload":{"psr-4":{"Shared\\":"src/"}}}"#),
        ("app/consumer/composer.json", SECOND_COMPOSER),
        ("app/consumer/bootstrap/providers.php", "<?php return [Shared\\SecondProvider::class];"),
        ("app/consumer/src/Caller.php", SECOND_CALLER),
        ("shared/src/SecondProvider.php", SECOND_PROVIDER),
        ("shared/json-b/en.json", r#"{"Greeting B 😀":"Disk B original"}"#),
        ("shared/alternate-b/en.json", r#"{"Greeting B 😀":"Disk B alternate"}"#),
        ("shared/src/Provider.php", PROVIDER),
        ("shared/src/Private.php", "<?php namespace Shared; class PrivateSource {}"),
        ("shared/json/en.json", r#"{"Greeting 😀":"Disk original"}"#),
        ("shared/alternate/en.json", r#"{"Greeting 😀":"Disk alternate"}"#),
    ] {
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
    for project in [&app, &app.join("consumer"), &shared] {
        std::os::unix::fs::symlink(&vendor, project.join("vendor")).unwrap();
    }
    let runtime = std::process::Command::new(&php).args(["-n","-d","zend.assertions=1","-d","assert.exception=1","-r",r#"require $argv[1].'/vendor/autoload.php'; foreach(['json'=>'Disk original','alternate'=>'Disk alternate'] as $dir=>$expected) { $loader=new Illuminate\Translation\FileLoader(new Illuminate\Filesystem\Filesystem, $argv[1].'/lang'); $loader->addJsonPath($argv[2].'/'.$dir); $translator=new Illuminate\Translation\Translator($loader,'en'); assert($translator->get('Greeting 😀')===$expected); } echo 'fixture passed';"#]).arg(&app).arg(&shared).output().unwrap();
    assert!(runtime.status.success(), "{}", String::from_utf8_lossy(&runtime.stderr));
    assert_eq!(runtime.stdout, b"fixture passed");
    let runtime_b = std::process::Command::new(&php).args(["-n","-d","zend.assertions=1","-d","assert.exception=1","-r",r#"require $argv[1].'/vendor/autoload.php'; foreach(['json-b'=>'Disk B original','alternate-b'=>'Disk B alternate'] as $dir=>$expected) { $loader=new Illuminate\Translation\FileLoader(new Illuminate\Filesystem\Filesystem, $argv[1].'/lang'); $loader->addJsonPath($argv[2].'/'.$dir); $translator=new Illuminate\Translation\Translator($loader,'en'); assert($translator->get('Greeting B 😀')===$expected); } echo 'fixture passed';"#]).arg(app.join("consumer")).arg(&shared).output().unwrap();
    assert!(runtime_b.status.success(), "{}", String::from_utf8_lossy(&runtime_b.stderr));
    assert_eq!(runtime_b.stdout, b"fixture passed");
    for project in [&app, &shared] {
        std::fs::write(project.join(".provider-fixture-root"), "").unwrap();
    }
    let engine = PathBuf::from(std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").expect("pinned PHPantom"));
    let control = root.join("control");
    std::fs::create_dir(&control).unwrap();
    let proxy = root.join("provider-trace.py");
    std::fs::write(&proxy, include_str!("phpantom-provider-trace.py")).unwrap();
    let mut language = route_handler_language(&engine);
    let config = &mut language.language_servers[0];
    config.root_markers = vec![".provider-fixture-root".into()];
    let arguments = std::mem::take(&mut config.args);
    config.command = std::env::var("OPENSTORM_TEST_PYTHON").unwrap_or_else(|_| "python3".into());
    config.args = vec![
        "-I".into(),
        proxy.to_string_lossy().into_owned(),
        engine.to_string_lossy().into_owned(),
        control.to_string_lossy().into_owned(),
    ];
    config.args.extend(arguments);
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![language])),
        Arc::new(|_: IdeEvent| {}),
    );
    mgr.set_content_roots(vec![app.clone(), shared.clone()]);
    let outcome = std::panic::AssertUnwindSafe(async {
        let original = DocId::next();
        let alternate = DocId::next();
        let provider = DocId::next();
        let caller = DocId::next();
        let private = DocId::next();
        let source = shared.join("src/Provider.php");
        let original_path = shared.join("json/en.json");
        let alternate_path = shared.join("alternate/en.json");
        let private_path = shared.join("src/Private.php");
        let dirty = PROVIDER.replace("../json", "../alternate");
        syntax_text(&php, &dirty);
        mgr.did_open(original, &original_path, &LanguageId::new("json"), 1, ORIGINAL);
        mgr.did_open(alternate, &alternate_path, &LanguageId::new("json"), 1, ALTERNATE);
        mgr.did_open(provider, &source, &LanguageId::new("php"), 1, &dirty);
        mgr.did_open(
            private,
            &private_path,
            &LanguageId::new("php"),
            1,
            "<?php /* unrelated unsaved private buffer */",
        );
        mgr.did_open(caller, &app.join("src/Caller.php"), &LanguageId::new("php"), 1, CALLER);
        let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
        assert!(
            hover.contents.value.contains("Private alternate"),
            "consuming server must use the unsaved foreign provider before ordinary hover: {hover:?}"
        );
        assert!(!hover.contents.value.contains("Private original"));
        let log = until_events(&control, |log| {
            log.iter().any(|event| {
                event_at(event, &app, "textDocument/didOpen", &source)
                    && event["sha256"] == format!("{:x}", Sha256::digest(dirty.as_bytes()))
            })
        })
        .await;
        assert!(!log.iter().any(|event| event_at(event, &app, "textDocument/didOpen", &private_path)));
        assert!(!log.iter().any(|event| event_at(event, &app, "textDocument/didOpen", &original_path)));
        let servers = mgr.servers();
        assert_eq!(servers.len(), 2);
        let app_server = servers.iter().find(|s| *s.root == app).unwrap().id;
        let shared_pid = servers.iter().find(|s| *s.root == shared).unwrap().pid;
        let starts = log
            .iter()
            .filter(|event| event["method"] == "engineStarted" && event["root"] == app.to_str().unwrap())
            .count();
        mgr.restart(app_server).await.unwrap();
        until_events(&control, |log| {
            let start = log
                .iter()
                .rposition(|event| event["method"] == "engineStarted" && event["root"] == app.to_str().unwrap());
            log.iter()
                .filter(|event| event["method"] == "engineStarted" && event["root"] == app.to_str().unwrap())
                .count()
                > starts
                && start.is_some_and(|start| {
                    log.iter()
                        .enumerate()
                        .any(|(i, event)| i > start && event_at(event, &app, "textDocument/didOpen", &alternate_path))
                })
        })
        .await;
        assert_eq!(mgr.servers().iter().find(|s| *s.root == shared).unwrap().pid, shared_pid);
        let before = events(&control);
        let hover_count = before
            .iter()
            .filter(|event| event_at(event, &app, "textDocument/hover", &app.join("src/Caller.php")))
            .count();
        let changed = format!("{PROVIDER}\n// unsaved edited 😀");
        syntax_text(&php, &changed);
        replace(&mgr, provider, 2, &dirty, &changed);
        let log = until_events(&control, |log| {
            log.iter().any(|event| {
                event_at(event, &app, "textDocument/didOpen", &source)
                    && event["version"] == 2
                    && event["sha256"] == format!("{:x}", Sha256::digest(changed.as_bytes()))
            }) && log.iter().any(|event| event_at(event, &app, "textDocument/didOpen", &original_path))
        })
        .await;
        assert_eq!(
            log.iter().filter(|event| event_at(event, &app, "textDocument/hover", &app.join("src/Caller.php"))).count(),
            hover_count,
            "background recovery must precede another PHP request"
        );
        let hover = mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("Private original"));
        mgr.did_close(provider);
        until_events(&control, |log| {
            let close = log.iter().rposition(|event| event_at(event, &app, "textDocument/didClose", &source));
            let open = log.iter().rposition(|event| event_at(event, &app, "textDocument/didOpen", &source));
            close.is_some_and(|close| {
                open.is_some_and(|open| close > open)
                    && log
                        .iter()
                        .enumerate()
                        .any(|(i, event)| i > close && event_at(event, &app, "textDocument/didOpen", &original_path))
            })
        })
        .await;
        assert!(
            mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default())
                .await
                .unwrap()
                .unwrap()
                .contents
                .value
                .contains("Private original")
        );
        mgr.did_close(original);
        mgr.did_close(alternate);
        mgr.did_open(provider, &source, &LanguageId::new("php"), 4, &dirty);
        assert!(
            mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default())
                .await
                .unwrap()
                .unwrap()
                .contents
                .value
                .contains("Disk alternate"),
            "closed catalogs must also use the unsaved provider"
        );
        let removed = "<?php namespace Shared; class Other {}";
        syntax_text(&php, removed);
        let hover_count = events(&control)
            .iter()
            .filter(|event| event_at(event, &app, "textDocument/hover", &app.join("src/Caller.php")))
            .count();
        replace(&mgr, provider, 5, &dirty, removed);
        until_events(&control, |log| {
            log.iter().any(|event| {
                event_at(event, &app, "textDocument/didOpen", &source)
                    && event["version"] == 5
                    && event["sha256"] == format!("{:x}", Sha256::digest(removed.as_bytes()))
            })
        })
        .await;
        assert_eq!(
            events(&control)
                .iter()
                .filter(|event| event_at(event, &app, "textDocument/hover", &app.join("src/Caller.php")))
                .count(),
            hover_count,
            "provider withdrawal must refresh before another PHP action with no open catalogs"
        );
        let hover = mgr
            .hover(caller, pos(CALLER, "Greeting 😀"), Default::default())
            .await
            .unwrap()
            .map(|h| h.contents.value)
            .unwrap_or_default();
        assert!(
            !hover.contains("Disk original") && !hover.contains("Disk alternate"),
            "removing a provider declaration must withdraw its disk registration: {hover}"
        );
        replace(&mgr, provider, 6, removed, &dirty);
        until_events(&control, |log| {
            log.iter().any(|event| {
                event_at(event, &app, "textDocument/didOpen", &source)
                    && event["version"] == 6
                    && event["sha256"] == format!("{:x}", Sha256::digest(dirty.as_bytes()))
            })
        })
        .await;
        assert!(
            mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default())
                .await
                .unwrap()
                .unwrap()
                .contents
                .value
                .contains("Disk alternate")
        );
        mgr.did_close(provider);
        assert!(
            mgr.hover(caller, pos(CALLER, "Greeting 😀"), Default::default())
                .await
                .unwrap()
                .unwrap()
                .contents
                .value
                .contains("Disk original")
        );
        assert!(!events(&control).iter().any(|event| event_at(event, &app, "textDocument/didOpen", &private_path)));
        assert_eq!(std::fs::read_to_string(&source).unwrap(), PROVIDER);
        assert_eq!(std::fs::read_to_string(&original_path).unwrap(), r#"{"Greeting 😀":"Disk original"}"#);
        assert_eq!(std::fs::read_to_string(&alternate_path).unwrap(), r#"{"Greeting 😀":"Disk alternate"}"#);
        assert!(!app.join("src/Caller.json").exists(), "dependency discovery must not create a probe file");
        let group_start = events(&control).len();
        let second_provider = DocId::next();
        let second_caller = DocId::next();
        let second_source = shared.join("src/SecondProvider.php");
        let second_path = app.join("consumer/src/Caller.php");
        let second_dirty = SECOND_PROVIDER.replace("../json-b", "../alternate-b");
        syntax_text(&php, &second_dirty);
        mgr.did_open(provider, &source, &LanguageId::new("php"), 7, &dirty);
        mgr.did_open(second_provider, &second_source, &LanguageId::new("php"), 1, &second_dirty);
        mgr.did_open(second_caller, &second_path, &LanguageId::new("php"), 1, SECOND_CALLER);
        until_events(&control, |log| {
            [(&source, 7, &dirty), (&second_source, 1, &second_dirty)].iter().all(|(path, version, text)| {
                log[group_start..].iter().any(|event| {
                    event_at(event, &app, "textDocument/didOpen", path)
                        && event["version"] == *version
                        && event["sha256"] == format!("{:x}", Sha256::digest(text.as_bytes()))
                })
            })
        })
        .await;
        assert!(
            !events(&control)[group_start..].iter().any(|event| event["method"] == "textDocument/hover"),
            "every shared-peer consumer must prepare before another PHP action"
        );
        let restart_start = events(&control).len();
        let server = mgr.servers().iter().find(|server| *server.root == app).unwrap().id;
        mgr.restart(server).await.unwrap();
        until_events(&control, |log| {
            [&source, &second_source].iter().all(|path| {
                log[restart_start..].iter().any(|event| event_at(event, &app, "textDocument/didOpen", path))
            })
        })
        .await;
        // Content-free queries wait for the active preparation gate without canceling it.
        for (manifest, path, text) in [
            (app.join("composer.json"), &source, &dirty),
            (app.join("consumer/composer.json"), &second_source, &second_dirty),
        ] {
            let receipt = mgr
                .php_json_translation_resource_from_composer(&manifest, &original_path, Default::default())
                .await
                .unwrap();
            let sources = receipt.provider_sources().unwrap();
            assert!(!sources.incomplete());
            assert!(sources.providers.iter().any(|provider| {
                provider.path == *path && provider.sha256_utf8 == <[u8; 32]>::from(Sha256::digest(text.as_bytes()))
            }));
        }
        assert!(!events(&control)[restart_start..].iter().any(|event| event["method"] == "textDocument/hover"));
        let navigation_start = events(&control).len();
        for (doc, text, expected) in [
            (caller, CALLER, "Disk alternate"),
            (second_caller, SECOND_CALLER, "Disk B alternate"),
            (caller, CALLER, "Disk alternate"),
        ] {
            let key = if doc == caller { "Greeting 😀" } else { "Greeting B 😀" };
            let hover = mgr.hover(doc, pos(text, key), Default::default()).await.unwrap().unwrap();
            assert!(hover.contents.value.contains(expected), "{hover:?}");
        }
        assert_eq!(mgr.servers().len(), 2, "both Composer consumers must use one actual PHP peer");
        let log = events(&control);
        for path in [&source, &second_source] {
            assert!(log[group_start..].iter().any(|event| event_at(event, &app, "textDocument/didOpen", path)));
            assert!(
                !log[navigation_start..].iter().any(|event| event_at(event, &app, "textDocument/didClose", path)),
                "switching live contexts must keep checked provider ownership"
            );
        }
        let removed = "<?php namespace Shared; class Other {}";
        syntax_text(&php, removed);
        replace(&mgr, provider, 8, &dirty, removed);
        let first = mgr
            .hover(caller, pos(CALLER, "Greeting 😀"), Default::default())
            .await
            .unwrap()
            .map(|h| h.contents.value)
            .unwrap_or_default();
        assert!(!first.contains("Disk alternate") && !first.contains("Disk original"));
        assert!(
            mgr.hover(second_caller, pos(SECOND_CALLER, "Greeting B 😀"), Default::default())
                .await
                .unwrap()
                .unwrap()
                .contents
                .value
                .contains("Disk B alternate")
        );
        let after_other_consumer = mgr
            .hover(caller, pos(CALLER, "Greeting 😀"), Default::default())
            .await
            .unwrap()
            .map(|h| h.contents.value)
            .unwrap_or_default();
        assert!(
            !after_other_consumer.contains("Disk alternate") && !after_other_consumer.contains("Disk original"),
            "another consumer must not restore a removed provider from disk: {after_other_consumer}"
        );
        replace(&mgr, provider, 9, removed, &dirty);
        for (doc, text, key, expected) in [
            (caller, CALLER, "Greeting 😀", "Disk alternate"),
            (second_caller, SECOND_CALLER, "Greeting B 😀", "Disk B alternate"),
        ] {
            assert!(
                mgr.hover(doc, pos(text, key), Default::default())
                    .await
                    .unwrap()
                    .unwrap()
                    .contents
                    .value
                    .contains(expected)
            );
        }
        mgr.did_close(caller);
        let hover =
            mgr.hover(second_caller, pos(SECOND_CALLER, "Greeting B 😀"), Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("Disk B alternate"));
        let log = events(&control);
        let open = log.iter().rposition(|event| event_at(event, &app, "textDocument/didOpen", &source));
        let close = log.iter().rposition(|event| event_at(event, &app, "textDocument/didClose", &source));
        assert!(
            close.is_some_and(|close| open.is_some_and(|open| close > open)),
            "closing the last consumer withdraws its provider copy"
        );
        assert!(!log.iter().any(|event| event_at(event, &app, "textDocument/didOpen", &private_path)));
        assert!(!app.join("consumer/src/Caller.json").exists());
        assert_eq!(std::fs::read_to_string(&source).unwrap(), PROVIDER);
        assert_eq!(std::fs::read_to_string(&second_source).unwrap(), SECOND_PROVIDER);
        assert_eq!(std::fs::read_to_string(app.join("consumer/composer.json")).unwrap(), SECOND_COMPOSER);
        mgr.did_close(second_caller);
        mgr.did_close(second_provider);
        mgr.did_close(provider);
        mgr.did_close(private);
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    if let Some(destination) = std::env::var_os("OPENSTORM_TEST_PROVIDER_TRACE_LOG") {
        let destination = PathBuf::from(destination);
        assert!(!destination.exists());
        std::fs::copy(control.join("events.jsonl"), destination).unwrap();
    }
    for event in events(&control) {
        if let Some(pid) = event["enginePid"].as_u64() {
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if !std::process::Command::new("/bin/kill")
                        .args(["-0", &pid.to_string()])
                        .output()
                        .unwrap()
                        .status
                        .success()
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .expect("owned engine process must exit after manager shutdown");
        }
    }
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}
