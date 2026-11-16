use super::*;
use std::collections::BTreeMap;

fn engine() -> PathBuf {
    std::env::var_os("OPENSTORM_TEST_URL_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/engines/0.11.1-openstorm.155/phpantom_lsp")
    })
}

fn fixture() -> (tempfile::TempDir, PathBuf, BTreeMap<String, String>) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let source = r#"<?php
namespace App;
use Illuminate\Support\Facades\URL;
use function action as controller_url;
$label='😀'; $one=\action('\App\Invoice@show');
$two=controller_url(name: "\\App\\Invoice@show");
$three=URL::action(action: '\App\Invoice@show');
$four=\redirect()->action(action: '\App\Invoice@show');
$five=\response()->redirectToAction(action: '\App\Invoice@show');
$ordinary=new Invoice();
$ordinary->show();
$plain='\App\Invoice@show';
"#;
    let mut files: BTreeMap<String, String> = BTreeMap::from([
        ("composer.json".into(), r#"{"require":{"laravel/framework":"^13.37"},"autoload":{"psr-4":{"App\\":"src/"}}}"#.into()),
        ("src/Base.php".into(), "<?php\nnamespace App;\nclass Base { /** Invoice documentation. 😀 */ public function show(): string { return 'base'; } }\n".into()),
        ("src/Other.php".into(), "<?php\nnamespace App;\nclass Other { /** Other documentation. */ public function show(): string { return 'other'; } }\n".into()),
        ("src/Invoice.php".into(), "<?php\nnamespace App;\nclass Invoice extends Base {}\n".into()),
        ("src/Links.php".into(), source.into()),
        ("plain/composer.json".into(), r#"{"autoload":{"psr-4":{"App\\":"src/"}}}"#.into()),
    ]);
    for name in ["Base", "Other", "Invoice", "Links"] {
        files.insert(format!("plain/src/{name}.php"), files[&format!("src/{name}.php")].clone());
    }
    let php = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/toolchains/php-8.4.23-static/php");
    for (name, text) in &files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        if name.ends_with(".php") {
            let output = std::process::Command::new(&php).args(["-n", "-l"]).arg(&path).output().unwrap();
            assert!(output.status.success(), "{name}: {}", String::from_utf8_lossy(&output.stderr));
        } else {
            let manifest: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(manifest["autoload"]["psr-4"]["App\\"], "src/");
        }
    }
    (tmp, root, files)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires candidate155 and the PHP syntax-check executable"]
async fn controller_url_navigation_references_overlays_restart_and_project_isolation() {
    let (_tmp, root, files) = fixture();
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![route_handler_language(&engine())])),
        Arc::new(|_| {}),
    );
    let source = &files["src/Links.php"];
    let doc = DocId::next();
    let dependency = DocId::next();
    mgr.did_open(doc, &root.join("src/Links.php"), &LanguageId::new("php"), 1, source);
    mgr.did_open(dependency, &root.join("src/Invoice.php"), &LanguageId::new("php"), 1, &files["src/Invoice.php"]);
    let result: Result<(), String> = async {
        let ordinary =
            mgr.goto(doc, pos(source, "Invoice();"), GotoKind::Definition).await.map_err(|e| e.to_string())?;
        if ordinary.len() != 1 || ordinary[0].path.to_path_buf() != root.join("src/Invoice.php") {
            return Err(format!("ordinary PHP control failed: {ordinary:?}"));
        }
        for (restart, parent, text) in [
            (false, "Base", files["src/Invoice.php"].clone()),
            (false, "Other", files["src/Invoice.php"].replace("Base", "Other")),
            (true, "Other", files["src/Invoice.php"].replace("Base", "Other")),
        ] {
            if text != files["src/Invoice.php"] && !restart {
                replace(&mgr, dependency, 2, &files["src/Invoice.php"], &text);
            }
            if restart {
                mgr.restart(mgr.servers()[0].id).await.map_err(|e| e.to_string())?;
            }
            for needle in ["show');", "show\");", "show');\n$four", "show');\n$five", "show');\n$ordinary"] {
                let at = pos(source, needle);
                let targets = mgr.goto(doc, at, GotoKind::Definition).await.map_err(|e| e.to_string())?;
                if targets.len() != 1
                    || targets[0].path.to_path_buf() != root.join(format!("src/{parent}.php"))
                    || targets[0].range.start != pos(&files[&format!("src/{parent}.php")], "show()")
                {
                    return Err(format!("URL method target {needle:?}, {parent}, restart={restart}: {targets:?}"));
                }
            }
            let refs = mgr
                .references(doc, pos(source, "show');"), false, true, Default::default())
                .await
                .map_err(|e| e.to_string())?;
            let actual = refs
                .locations
                .iter()
                .filter(|location| location.path.to_path_buf() == root.join("src/Links.php"))
                .map(|location| location.range.start)
                .collect::<std::collections::BTreeSet<_>>();
            let expected = source
                .match_indices("show")
                .filter(|(offset, _)| !source[..*offset].rsplit('\n').next().unwrap().starts_with("$plain="))
                .map(|(offset, _)| {
                    let prefix = &source[..offset];
                    Pos::new(
                        prefix.bytes().filter(|b| *b == b'\n').count() as u32,
                        prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
                    )
                })
                .collect::<std::collections::BTreeSet<_>>();
            if actual != expected || refs.incomplete.is_some() {
                return Err(format!(
                    "URL references mismatch, {parent}: actual={actual:?}, expected={expected:?}, incomplete={:?}",
                    refs.incomplete
                ));
            }
            let hover = mgr.hover(doc, pos(source, "show');"), Default::default()).await.map_err(|e| e.to_string())?;
            let documentation = if parent == "Base" { "Invoice documentation." } else { "Other documentation." };
            if hover.as_ref().is_none_or(|h| !h.contents.value.contains(documentation)) {
                return Err(format!("URL hover lost inherited documentation: {hover:?}"));
            }
        }
        let plain = DocId::next();
        mgr.did_open(plain, &root.join("plain/src/Links.php"), &LanguageId::new("php"), 1, source);
        let control =
            mgr.goto(plain, pos(source, "Invoice();"), GotoKind::Definition).await.map_err(|e| e.to_string())?;
        let urls = mgr.goto(plain, pos(source, "show');"), GotoKind::Definition).await.map_err(|e| e.to_string())?;
        if control.len() != 1 || control[0].path.to_path_buf() != root.join("plain/src/Invoice.php") || !urls.is_empty()
        {
            return Err(format!("nested plain project borrowed Laravel semantics: control={control:?}, URLs={urls:?}"));
        }
        Ok(())
    }
    .await;
    mgr.shutdown_all().await;
    assert!(result.is_ok(), "{result:?}");
    for (name, text) in files {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text);
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires candidate155 and the retained Laravel13.37 fixture"]
async fn controller_url_navigation_resolves_the_real_laravel_example() {
    let root = std::env::var_os("OPENSTORM_TEST_URL_LARAVEL_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/checks/engine-laravel13-37-155/project")
        })
        .canonicalize()
        .unwrap();
    let path = root.join("app/Demo.php");
    let target = root.join("app/Http/Controllers/BakeryController.php");
    let source = std::fs::read_to_string(&path).unwrap();
    let declaration = std::fs::read_to_string(&target).unwrap();
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![route_handler_language(&engine())])),
        Arc::new(|_| {}),
    );
    let doc = DocId::next();
    mgr.did_open(doc, &path, &LanguageId::new("php"), 1, &source);
    let result: Result<(), String> = async {
        let section = source.find("public function controllerActionUrls").ok_or("missing candidate155 example")?;
        let mut count = 0;
        for (offset, _) in source[section..].match_indices("BakeryController@index") {
            let prefix = &source[..section + offset + "BakeryController@".len()];
            let at = Pos::new(
                prefix.bytes().filter(|b| *b == b'\n').count() as u32,
                prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
            );
            let targets = mgr.goto(doc, at, GotoKind::Definition).await.map_err(|e| e.to_string())?;
            if targets.len() != 1
                || targets[0].path.to_path_buf() != target
                || targets[0].range.start != pos(&declaration, "index()")
            {
                return Err(format!("real Laravel URL definition: {targets:?}"));
            }
            count += 1;
        }
        if count != 3 {
            return Err(format!("expected three candidate155 examples, got {count}"));
        }
        Ok(())
    }
    .await;
    mgr.shutdown_all().await;
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
    assert_eq!(std::fs::read_to_string(target).unwrap(), declaration);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires candidate155 and Python for held real-engine replies"]
async fn controller_url_navigation_rejects_stale_replies_and_cancels_references() {
    for (method, mutation) in [
        ("definition", "dependency"),
        ("definition", "caller"),
        ("definition", "disk"),
        ("definition", "roots"),
        ("definition", "settings"),
        ("definition", "restart"),
        ("definition", "no change"),
        ("references", "dependency"),
        ("references", "caller"),
        ("references", "cancel"),
        ("references", "no change"),
    ] {
        let (_tmp, root, files) = fixture();
        let engine = engine().canonicalize().unwrap();
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
        let dependency = DocId::next();
        let source = &files["src/Links.php"];
        mgr.did_open(doc, &root.join("src/Links.php"), &LanguageId::new("php"), 1, source);
        mgr.did_open(dependency, &root.join("src/Invoice.php"), &LanguageId::new("php"), 1, &files["src/Invoice.php"]);
        let result:Result<(),String>=async {
            // Hover starts indexing without consuming the reply held by the proxy.
            mgr.hover(doc,pos(source,"show');"),Default::default()).await.map_err(|e|e.to_string())?;
            let client=mgr.clone();let at=pos(source,"show');");
            let token=ide_core::CancellationToken::new();let cancel=token.clone();
            let pending=tokio::spawn(async move{
                if method=="references" {client.references(doc,at,true,true,cancel).await.map(|r|r.locations)}
                else {client.goto(doc,at,GotoKind::Definition).await}
            });
            for _ in 0..750 {
                if ready.exists() || pending.is_finished(){break;}
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            let captured=std::fs::read_to_string(&ready);
            if captured.is_ok() {
                match mutation {
                    "dependency"=>replace(&mgr,dependency,2,&files["src/Invoice.php"],&files["src/Invoice.php"].replace("Base","Other")),
                    "caller"=>replace(&mgr,doc,2,source,&source.replace("$label=","$changed=")),
                    "disk"=>{let path=root.join("src/Base.php");std::fs::write(&path,files["src/Base.php"].replace("'base'","'external'")).unwrap();mgr.php_files_changed(&[],&[path],&[]);},
                    "roots"=>mgr.set_content_roots(vec![root.join("linked")]),
                    "settings"=>{mgr.set_config_hook(Box::new(|cfg,_|cfg.settings=Some(serde_json::json!({"controllerUrlFixture":true}))));mgr.reconfigure().await;},
                    "restart"=>mgr.restart(mgr.servers()[0].id).await.map_err(|e|e.to_string())?,
                    "cancel"=>token.cancel(),
                    _=>{},
                }
            }
            std::fs::write(&release,"release").unwrap();
            let reply=pending.await.map_err(|e|e.to_string())?;
            let captured:serde_json::Value=serde_json::from_str(&captured.map_err(|e|format!("no held URL definition: {e}; reply={reply:?}"))?).map_err(|e|e.to_string())?;
            if !captured["result"].to_string().contains("Base.php") { return Err(format!("proxy held the wrong result: {captured}")); }
            if mutation=="no change" {
                let count=if method=="references"{7}else{1};
                if !reply.as_ref().is_ok_and(|locations|locations.len()==count && locations.iter().any(|l|l.path.to_path_buf()==root.join("src/Base.php"))) {return Err(format!("current URL response refused: {reply:?}"));}
            } else if !matches!(reply,Err(ref e) if matches!(e.kind,ide_core::error::ErrorKind::Cancelled|ide_core::error::ErrorKind::OutOfSync)) {
                return Err(format!("stale URL response accepted, {mutation}: {reply:?}"));
            }
            Ok(())
        }.await;
        let _ = std::fs::write(&release, "release");
        mgr.shutdown_all().await;
        assert!(result.is_ok(), "{method}/{mutation}: {result:?}");
        for (name, text) in &files {
            let expected = if mutation == "disk" && name == "src/Base.php" {
                text.replace("'base'", "'external'")
            } else {
                text.clone()
            };
            assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), expected);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires candidate156 and Python for held real-engine completion replies"]
async fn controller_url_method_completion_rejects_stale_replies() {
    for (method, mutation) in [
        ("completion", "dependency"),
        ("completion", "caller"),
        ("completion", "close"),
        ("completion", "disk"),
        ("completion", "roots"),
        ("completion", "settings"),
        ("completion", "restart"),
        ("completion", "cancel"),
        ("completion", "no change"),
    ] {
        let (_tmp, root, files) = fixture();
        let engine = std::env::var_os("OPENSTORM_TEST_URL_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../artifacts/engines/0.11.1-openstorm.156/phpantom_lsp")
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
        let dependency = DocId::next();
        let source = &files["src/Links.php"];
        mgr.did_open(doc, &root.join("src/Links.php"), &LanguageId::new("php"), 1, source);
        mgr.did_open(dependency, &root.join("src/Invoice.php"), &LanguageId::new("php"), 1, &files["src/Invoice.php"]);
        let result:Result<(),String>=async {
            // Hover starts indexing without consuming the reply held by the proxy.
            mgr.hover(doc,pos(source,"show');"),Default::default()).await.map_err(|e|e.to_string())?;
            let client=mgr.clone();let at=pos(source,"show');");
            let token=ide_core::CancellationToken::new();let cancel=token.clone();
            let pending=tokio::spawn(async move{
                client.completion(doc,at,None,cancel).await
            });
            for _ in 0..750 {
                if ready.exists() || pending.is_finished(){break;}
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            let captured=std::fs::read_to_string(&ready);
            if captured.is_ok() {
                match mutation {
                    "dependency"=>replace(&mgr,dependency,2,&files["src/Invoice.php"],&files["src/Invoice.php"].replace("Base","Other")),
                    "close"=>mgr.did_close(dependency),
                    "caller"=>replace(&mgr,doc,2,source,&source.replace("$label=","$changed=")),
                    "disk"=>{let path=root.join("src/Base.php");std::fs::write(&path,files["src/Base.php"].replace("'base'","'external'")).unwrap();mgr.php_files_changed(&[],&[path],&[]);},
                    "roots"=>mgr.set_content_roots(vec![root.join("linked")]),
                    "settings"=>{mgr.set_config_hook(Box::new(|cfg,_|cfg.settings=Some(serde_json::json!({"controllerUrlFixture":true}))));mgr.reconfigure().await;},
                    "restart"=>mgr.restart(mgr.servers()[0].id).await.map_err(|e|e.to_string())?,
                    "cancel"=>token.cancel(),
                    _=>{},
                }
            }
            std::fs::write(&release,"release").unwrap();
            let reply=pending.await.map_err(|e|e.to_string())?;
            let captured:serde_json::Value=serde_json::from_str(&captured.map_err(|e|format!("no held URL definition: {e}; reply={reply:?}"))?).map_err(|e|e.to_string())?;
            if !captured["result"]["items"].as_array().is_some_and(|items| items.iter().any(|item|item["filterText"]=="show")) {return Err(format!("proxy held no URL method completion: {captured}"));}
            if mutation=="no change" {
                if !reply.as_ref().is_ok_and(|list|!list.is_incomplete && list.items.iter().any(|i|i.filter_text.as_deref()==Some("show"))) {return Err(format!("current URL completion refused: {reply:?}"));}
            } else if !matches!(reply,Err(ref e) if matches!(e.kind,ide_core::error::ErrorKind::Cancelled|ide_core::error::ErrorKind::OutOfSync)) {
                return Err(format!("stale URL response accepted, {mutation}: {reply:?}"));
            }
            Ok(())
        }.await;
        let _ = std::fs::write(&release, "release");
        mgr.shutdown_all().await;
        assert!(result.is_ok(), "{method}/{mutation}: {result:?}");
        for (name, text) in &files {
            let expected = if mutation == "disk" && name == "src/Base.php" {
                text.replace("'base'", "'external'")
            } else {
                text.clone()
            };
            assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), expected);
        }
    }
}
