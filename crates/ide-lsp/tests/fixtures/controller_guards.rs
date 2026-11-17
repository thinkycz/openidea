use super::*;

const SOURCE: &str = "<?php\nnamespace App;\nclass GuardController extends BaseController {public function boot() {$note='😀'; $this->middleware('auth:adOLD,web');}}\n";
const PARENT: &str = "<?php namespace App; class BaseController extends \\Illuminate\\Routing\\Controller {}\n";
const CONFIG: &str = "<?php return ['guards'=>['web'=>[], 'admin'=>[]]];\n";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires candidate157 and owned PHP syntax-check executable"]
async fn typed_guard_completion_and_resolve_reject_changed_sources() {
    for (method, mutation) in [
        ("completion", "parent"),
        ("completion", "config"),
        ("completion", "restart"),
        ("completion", "cancel"),
        ("completion", "unchanged"),
        ("resolve", "parent"),
        ("resolve", "config"),
        ("resolve", "restart"),
        ("resolve", "unchanged"),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let files = [
            (
                "composer.json",
                r#"{"require":{"laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/","Illuminate\\Routing\\":"framework/"}}}"#,
            ),
            ("app/GuardController.php", SOURCE),
            ("app/BaseController.php", PARENT),
            ("config/auth.php", CONFIG),
            (
                "framework/Controller.php",
                "<?php namespace Illuminate\\Routing; class Controller {public function middleware($middleware) {}}\n",
            ),
        ];
        let php_bin =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/toolchains/php-8.4.23-static/php");
        for (relative, text) in files {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, text).unwrap();
            if relative.ends_with(".php") {
                let result = std::process::Command::new(&php_bin).args(["-n", "-l"]).arg(&path).output().unwrap();
                assert!(result.status.success(), "{relative}: {}", String::from_utf8_lossy(&result.stderr));
            }
        }
        let engine = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../artifacts/engines/0.11.1-openstorm.157/phpantom_lsp")
            })
            .canonicalize()
            .unwrap();
        let proxy = root.join("held-reply.py");
        std::fs::write(&proxy, include_str!("phpantom-held-reply.py")).unwrap();
        let ready = root.join("reply.json");
        let release = root.join("release");
        let mut php = route_handler_language(&engine);
        let cfg = &mut php.language_servers[0];
        let args = std::mem::take(&mut cfg.args);
        cfg.command = std::env::var("OPENSTORM_TEST_PYTHON").unwrap_or_else(|_| "python3".into());
        cfg.args = vec![
            "-I".into(),
            proxy.to_string_lossy().into_owned(),
            engine.to_string_lossy().into_owned(),
            if method == "resolve" { "completionItem/resolve".into() } else { "textDocument/completion".into() },
            ready.to_string_lossy().into_owned(),
            release.to_string_lossy().into_owned(),
        ];
        cfg.args.extend(args);
        let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
        let doc = DocId::next();
        let parent = DocId::next();
        let config = DocId::next();
        mgr.did_open(doc, &root.join("app/GuardController.php"), &LanguageId::new("php"), 1, SOURCE);
        mgr.did_open(parent, &root.join("app/BaseController.php"), &LanguageId::new("php"), 1, PARENT);
        mgr.did_open(config, &root.join("config/auth.php"), &LanguageId::new("php"), 1, CONFIG);
        let result: Result<(), String> = async {
            let position = pos(SOURCE,"OLD");
            mgr.hover(doc,position,Default::default()).await.map_err(|e|e.to_string())?;
            let item = if method == "resolve" {
                Some(settled_completion(&mgr,doc,position).await.items.into_iter().find(|item|item.label=="admin").ok_or("no guard completion before resolve")?)
            } else { None };
            let client=mgr.clone();let token=ide_core::CancellationToken::new();let cancel=token.clone();
            let pending=tokio::spawn(async move {
                if let Some(item)=item {client.resolve_completion(doc,item).await.map(|item|item.label=="admin")}
                else {client.completion(doc,position,None,cancel).await.map(|list|list.items.iter().any(|item|item.label=="admin"))}
            });
            for _ in 0..750 {
                if ready.exists() || pending.is_finished() {break;}
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            let captured=std::fs::read_to_string(&ready);
            if captured.is_ok() {
                match mutation {
                    "parent"=>replace(&mgr,parent,2,PARENT,"<?php namespace App; class BaseController {public function middleware($middleware) {}}\n"),
                    "config"=>replace(&mgr,config,2,CONFIG,&CONFIG.replace("admin","staff")),
                    "restart"=>mgr.restart(mgr.servers()[0].id).await.map_err(|e|e.to_string())?,
                    "cancel"=>token.cancel(),
                    _=>{},
                }
            }
            std::fs::write(&release,"release").unwrap();
            let reply=pending.await.map_err(|e|e.to_string())?;
            let captured:serde_json::Value=serde_json::from_str(&captured.map_err(|e|format!("missing held guard reply: {e}; {reply:?}"))?).map_err(|e|e.to_string())?;
            let body=&captured["result"];
            let witnessed=if method=="resolve" {body["label"]=="admin"} else {
                body.get("items").unwrap_or(body).as_array().is_some_and(|items|items.iter().any(|item|item["label"]=="admin"))
            };
            if !witnessed {return Err(format!("proxy did not hold a real guard result: {captured}"));}
            if mutation=="unchanged" {
                if !matches!(reply,Ok(true)) {return Err(format!("current result refused: {reply:?}"));}
            } else if !matches!(reply,Err(ref error) if matches!(error.kind,ide_core::error::ErrorKind::Cancelled|ide_core::error::ErrorKind::OutOfSync)) {
                return Err(format!("stale {method}/{mutation} accepted: {reply:?}"));
            }
            Ok(())
        }.await;
        let _ = std::fs::write(&release, "release");
        mgr.shutdown_all().await;
        assert!(mgr.servers().iter().all(|server| server.pid.is_none()));
        for (relative, text) in files {
            assert_eq!(std::fs::read_to_string(root.join(relative)).unwrap(), text);
        }
        assert!(result.is_ok(), "{method}/{mutation}: {result:?}");
    }
}

const ENCODED_SOURCE: &str = r#"<?php
namespace App;
class GuardController extends BaseController {public function boot() {$note='😀';$this->middleware("auth:ad\x6din,web");$this->middleware("auth\x3aweb\054ad\u{6d}in");}}
"#;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires candidate158 and owned PHP syntax-check executable"]
async fn encoded_guard_navigation_and_hover_reject_changed_sources() {
    for (method, mutation) in [
        ("hover", "parent"),
        ("hover", "config"),
        ("hover", "caller"),
        ("hover", "disk"),
        ("hover", "roots"),
        ("hover", "settings"),
        ("hover", "restart"),
        ("hover", "cancel"),
        ("hover", "unchanged"),
        ("definition", "parent"),
        ("definition", "config"),
        ("definition", "restart"),
        ("definition", "unchanged"),
        ("references", "parent"),
        ("references", "config"),
        ("references", "cancel"),
        ("references", "unchanged"),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().canonicalize().unwrap();
        let files = [
            (
                "composer.json",
                r#"{"require":{"laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/","Illuminate\\Routing\\":"framework/"}}}"#,
            ),
            ("app/GuardController.php", ENCODED_SOURCE),
            ("app/BaseController.php", PARENT),
            ("config/auth.php", CONFIG),
            (
                "framework/Controller.php",
                "<?php namespace Illuminate\\Routing; class Controller {public function middleware($middleware) {}}\n",
            ),
        ];
        let php_bin =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/toolchains/php-8.4.23-static/php");
        for (relative, text) in files {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, text).unwrap();
            if relative.ends_with(".php") {
                let result = std::process::Command::new(&php_bin).args(["-n", "-l"]).arg(&path).output().unwrap();
                assert!(result.status.success(), "{relative}: {}", String::from_utf8_lossy(&result.stderr));
            }
        }
        let engine = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../artifacts/engines/0.11.1-openstorm.158/phpantom_lsp")
            })
            .canonicalize()
            .unwrap();
        let proxy = root.join("held-reply.py");
        std::fs::write(&proxy, include_str!("phpantom-held-reply.py")).unwrap();
        let ready = root.join("reply.json");
        let release = root.join("release");
        let mut php = route_handler_language(&engine);
        let cfg = &mut php.language_servers[0];
        let args = std::mem::take(&mut cfg.args);
        cfg.command = std::env::var("OPENSTORM_TEST_PYTHON").unwrap_or_else(|_| "python3".into());
        cfg.args = vec![
            "-I".into(),
            proxy.to_string_lossy().into_owned(),
            engine.to_string_lossy().into_owned(),
            format!("textDocument/{method}"),
            ready.to_string_lossy().into_owned(),
            release.to_string_lossy().into_owned(),
        ];
        cfg.args.extend(args);
        let mgr = LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![php])), Arc::new(|_| {}));
        let doc = DocId::next();
        let parent = DocId::next();
        let config = DocId::next();
        mgr.did_open(doc, &root.join("app/GuardController.php"), &LanguageId::new("php"), 1, ENCODED_SOURCE);
        mgr.did_open(parent, &root.join("app/BaseController.php"), &LanguageId::new("php"), 1, PARENT);
        mgr.did_open(config, &root.join("config/auth.php"), &LanguageId::new("php"), 1, CONFIG);
        let result: Result<(), String> = async {
            let position = pos(ENCODED_SOURCE,r"\x6d");
            mgr.completion(doc,pos(ENCODED_SOURCE,"BaseController"),None,Default::default()).await.map_err(|e|e.to_string())?;
            let client=mgr.clone();let token=ide_core::CancellationToken::new();let cancel=token.clone();
            let pending=tokio::spawn(async move {
                match method {
                    "hover"=>client.hover(doc,position,cancel).await.map(|hover|hover.is_some_and(|h|h.contents.value.contains("admin"))),
                    "references"=>client.references(doc,position,true,true,cancel).await.map(|r|r.locations.len()==3),
                    _=>client.goto(doc,position,GotoKind::Definition).await.map(|targets|targets.len()==1 && targets[0].path.to_path_buf().ends_with("config/auth.php")),
                }
            });
            for _ in 0..750 {
                if ready.exists() || pending.is_finished() {break;}
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            let captured=std::fs::read_to_string(&ready);
            if captured.is_ok() {
                match mutation {
                    "parent"=>replace(&mgr,parent,2,PARENT,"<?php namespace App; class BaseController {public function middleware($middleware) {}}\n"),
                    "config"=>replace(&mgr,config,2,CONFIG,&CONFIG.replace("admin","staff")),
                    "caller"=>replace(&mgr,doc,2,ENCODED_SOURCE,&ENCODED_SOURCE.replace("$note","$memo")),
                    "disk"=>{let path=root.join("config/auth.php");std::fs::write(&path,CONFIG.replace("admin","staff")).unwrap();mgr.php_files_changed(&[],&[path],&[]);},
                    "roots"=>mgr.set_content_roots(vec![root.join("linked")]),
                    "settings"=>{mgr.set_config_hook(Box::new(|cfg,_|cfg.settings=Some(serde_json::json!({"encodedGuardFixture":true}))));mgr.reconfigure().await;},
                    "restart"=>mgr.restart(mgr.servers()[0].id).await.map_err(|e|e.to_string())?,
                    "cancel"=>token.cancel(),
                    _=>{},
                }
            }
            std::fs::write(&release,"release").unwrap();
            let reply=pending.await.map_err(|e|e.to_string())?;
            let captured:serde_json::Value=serde_json::from_str(&captured.map_err(|e|format!("missing held guard reply: {e}; {reply:?}"))?).map_err(|e|e.to_string())?;
            let body=&captured["result"];
            let witnessed=if method=="hover" {body["contents"].to_string().contains("admin")}
                else {body.to_string().contains("config/auth.php")};
            if !witnessed {return Err(format!("proxy did not hold a real guard result: {captured}"));}
            if mutation=="unchanged" {
                if !matches!(reply,Ok(true)) {return Err(format!("current result refused: {reply:?}"));}
            } else if !matches!(reply,Err(ref error) if matches!(error.kind,ide_core::error::ErrorKind::Cancelled|ide_core::error::ErrorKind::OutOfSync)) {
                return Err(format!("stale {method}/{mutation} accepted: {reply:?}"));
            }
            Ok(())
        }.await;
        let _ = std::fs::write(&release, "release");
        mgr.shutdown_all().await;
        assert!(mgr.servers().iter().all(|server| server.pid.is_none()));
        for (relative, text) in files {
            assert_eq!(
                std::fs::read_to_string(root.join(relative)).unwrap(),
                if mutation == "disk" && relative == "config/auth.php" {
                    text.replace("admin", "staff")
                } else {
                    text.to_string()
                }
            );
        }
        assert!(result.is_ok(), "{method}/{mutation}: {result:?}");
    }
}
