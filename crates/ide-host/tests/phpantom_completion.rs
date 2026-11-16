//! Runtime controller completion through the real host/client/document paths.
use futures::FutureExt;
use ide_core::{DocId, LanguageId, Pos, Range, RequestId, TextEdit};
use ide_host::{Host, HostConfig, Project};
use ide_lsp::types::{CompletionItem, CompletionKind, CompletionList, GotoKind};
use std::{io::Write, path::PathBuf, process::Stdio, sync::Arc, time::Duration};

const HEADER: &str = "<?php\nnamespace Harness;\nuse Illuminate\\Support\\Facades\\Route as Routes;\nuse App\\Other\\InvoiceController as InvoiceController;\n";
const MAIN: &str = "<?php\nnamespace App;\n/** Main controller completion witness. */\nclass InvoiceController {public function __invoke(): bool {return true;} public function showInvoice(): bool {return true;}}\n";
const OTHER: &str =
    "<?php namespace App\\Other; class InvoiceController {public function showInvoice(): string {return 'other';}}\n";

fn at(text: &str, offset: usize) -> Pos {
    let prefix = &text[..offset];
    Pos::new(
        prefix.bytes().filter(|b| *b == b'\n').count() as u32,
        prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
    )
}

fn routes(call: &str) -> String {
    format!("{HEADER}{call}\n")
}

struct Fixture {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    host: Host,
    project: Arc<Project>,
    doc: DocId,
    events: Arc<std::sync::Mutex<Vec<ide_core::IdeEvent>>>,
}

impl Fixture {
    fn new(initial: &str, extra: &[(String, String)]) -> Self {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("src-tauri/resources/phpantom/phpantom_lsp"));
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        std::fs::create_dir_all(&root).unwrap();
        let root = dunce::canonicalize(root).unwrap();
        let files = [
            (
                "composer.json".to_owned(),
                r#"{"require":{"php":"^8.3","laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#
                    .to_owned(),
            ),
            ("app/InvoiceController.php".to_owned(), MAIN.to_owned()),
            ("app/Other/InvoiceController.php".to_owned(), OTHER.to_owned()),
            ("routes/web.php".to_owned(), initial.to_owned()),
        ];
        for (name, text) in files.iter().chain(extra) {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
            if name.ends_with(".php") {
                Self::lint(text);
            }
        }
        let composer: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join("composer.json")).unwrap()).unwrap();
        assert_eq!(composer["autoload"]["psr-4"]["App\\"], "app/");
        let mut php = ide_language::LanguageRegistry::load(None).unwrap().get(&LanguageId::new("php")).unwrap();
        php.language_servers.retain(|s| s.name == "phpantom");
        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let listener = events.clone();
        let host = Host::new(HostConfig {
            project_environment: false,
            config_dir: tmp.path().join("config"),
            data_dir: tmp.path().join("data"),
            local_data_dir: tmp.path().join("local-data"),
            languages: Arc::new(ide_language::LanguageRegistry::from_configs(vec![php])),
            syntax: Arc::default(),
            sink_for: Box::new(move |_| {
                let events = listener.clone();
                Arc::new(move |event| events.lock().unwrap().push(event))
            }),
            owner: |w| w,
            runtime: tokio::runtime::Handle::current(),
        });
        host.trust.decide(&root, true, false).unwrap();
        let project = host.open_project("main", &root).unwrap();
        project.lsp.set_config_hook(Box::new(move |cfg, _| {
            if cfg.name == "phpantom" {
                cfg.command = bin.to_string_lossy().into_owned();
            }
        }));
        let doc = project.documents.open(&root.join("routes/web.php")).unwrap().id;
        Self { _tmp: tmp, root, host, project, doc, events }
    }

    fn lint(text: &str) {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let php = std::env::var_os("OPENSTORM_TEST_PHP_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("artifacts/toolchains/php-8.4.23-static/php"));
        let mut child = std::process::Command::new(php)
            .args(["-n", "-l"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "invalid PHP fixture: {}", String::from_utf8_lossy(&output.stderr));
    }

    async fn complete(&self, position: Pos) -> CompletionList {
        for _ in 0..80 {
            match ide_host::ops::lsp::lsp_completion(&self.host, "main", RequestId::next(), self.doc, position, None)
                .await
            {
                Ok(list) => return list,
                Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                Err(error) => panic!("host completion failed: {error}"),
            }
        }
        panic!("runtime class completion never settled");
    }

    async fn finish(&self, outcome: std::thread::Result<()>) {
        self.project.lsp.shutdown_all().await;
        assert!(self.project.lsp.servers().iter().all(|s| s.pid.is_none()));
        self.host.close_project("main");
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
    }
}

fn controller(list: CompletionList) -> CompletionItem {
    list.items
        .into_iter()
        .find(|i| i.detail.as_deref() == Some("App\\InvoiceController"))
        .expect("host must offer the runtime controller class")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and owned PHP syntax-check executable"]
async fn runtime_class_completion_applies_only_the_class_and_keeps_save_conflict_consent() {
    let original = routes("Routes::get('/original','App\\InvoiceController@showInvoice');");
    let f = Fixture::new(&original, &[]);
    let outcome = std::panic::AssertUnwindSafe(async {
        for (call, body, prefix, expected) in [
            (r#"Routes::get('/😀/direct','App\InvOLD@showInvoice');"#, r"App\InvOLD", r"App\In",
             r#"Routes::get('/😀/direct','\\App\\InvoiceController@showInvoice');"#),
            (r#"Routes::namespace('App')->group(fn () => Routes::get('/short','InvOLD@showInvoice'));"#, "InvOLD", "In",
             r#"Routes::namespace('App')->group(fn () => Routes::get('/short','\\App\\InvoiceController@showInvoice'));"#),
            (r#"Routes::get('/😀/hex',"App\\Inv\x4fLD@showInvoice");"#, r"App\\Inv\x4fLD", r"App\\Inv",
             r#"Routes::get('/😀/hex',"\\App\\InvoiceController@showInvoice");"#),
            (r#"Routes::get('/unicode',"\u{41}pp\\InvOLD@showInvoice");"#, r"\u{41}pp\\InvOLD", r"\u{41}pp\\In",
             r#"Routes::get('/unicode',"\\App\\InvoiceController@showInvoice");"#),
            (r#"Routes::namespace('App')->controller('InvOLD')->group(fn () => Routes::get('/bound','showInvoice'));"#, "InvOLD", "In",
             r#"Routes::namespace('App')->controller('\\App\\InvoiceController')->group(fn () => Routes::get('/bound','showInvoice'));"#),
            (r#"Routes::group(['namespace'=>'App','controller'=>'InvOLD'],fn () => Routes::get('/attrs','showInvoice'));"#, "InvOLD", "In",
             r#"Routes::group(['namespace'=>'App','controller'=>'\\App\\InvoiceController'],fn () => Routes::get('/attrs','showInvoice'));"#),
            (r#"Routes::get('/😀/invokable','App\InvOLD');"#, r"App\InvOLD", r"App\In",
             r#"Routes::get('/😀/invokable','\\App\\InvoiceController');"#),
            (r#"Routes::match(action:'App\InvOLD',methods:['post'],uri:'/named');"#, r"App\InvOLD", r"App\In",
             r#"Routes::match(action:'\\App\\InvoiceController',methods:['post'],uri:'/named');"#),
            (r#"Routes::namespace('App')->group(fn () => Routes::get('/invokable-relative','InvOLD'));"#, "InvOLD", "In",
             r#"Routes::namespace('App')->group(fn () => Routes::get('/invokable-relative','\\App\\InvoiceController'));"#),
            (r#"Routes::get('/😀/invokable-hex',"App\\Inv\x4fLD");"#, r"App\\Inv\x4fLD", r"App\\Inv",
             r#"Routes::get('/😀/invokable-hex',"\\App\\InvoiceController");"#),
        ] {
            let input = routes(call);
            Fixture::lint(&input);
            let expected = routes(expected);
            Fixture::lint(&expected);
            f.project.documents.replace_all(f.doc, &input).unwrap();
            let before = f.project.documents.snapshot(f.doc).unwrap();
            let start = input.find(body).unwrap();
            let range = Range::new(at(&input, start), at(&input, start + body.len()));
            let item = controller(f.complete(at(&input, start + prefix.len())).await);
            assert_eq!(item.kind, Some(CompletionKind::Class));
            assert_eq!(item.replace_range, Some(range), "UTF16 class interior: {call}");
            assert!(!item.is_snippet);
            assert!(item.additional_edits.is_empty(), "runtime strings must never add PHP imports");
            assert!(item.filter_text.as_deref().is_some_and(|text| text.starts_with(prefix) && !text.contains("OLD")));
            let resolved = ide_host::ops::lsp::lsp_resolve_completion(&f.host, "main", f.doc, item).await.unwrap();
            assert_eq!(resolved.replace_range, Some(range));
            assert!(resolved.additional_edits.is_empty());
            assert!(resolved.documentation.as_ref().unwrap().value.contains("Main controller completion witness."));
            let edit = TextEdit { range, new_text: resolved.insert_text };
            assert!(f.project.documents.apply_edits_at(f.doc, before.version, &[edit]).unwrap().is_some());
            let current = f.project.documents.snapshot(f.doc).unwrap();
            assert_eq!(current.text, expected);
            assert!(current.dirty);
            assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), original);
            let targets = f.project.lsp.goto(f.doc, range.start, GotoKind::Definition).await.unwrap();
            assert_eq!(targets.len(), 1, "completed runtime class must navigate: {targets:?}");
            assert_eq!(targets[0].path.to_path_buf(), f.root.join("app/InvoiceController.php"));
            let hover = f.project.lsp.hover(f.doc, range.start, Default::default()).await.unwrap().unwrap();
            assert!(hover.contents.value.contains("Main controller completion witness."));
        }
        let current = f.project.documents.snapshot(f.doc).unwrap().text;
        let path = f.root.join("routes/web.php");
        let external = format!("{original}// External first\n");
        std::fs::write(&path, &external).unwrap();
        let consent = ide_host::ops::doc::save_checked(&f.host, "main", f.doc, None).await.unwrap().unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), external);
        let newer = format!("{original}// External second\n");
        std::fs::write(&path, &newer).unwrap();
        let replacement = ide_host::ops::doc::save_checked(&f.host, "main", f.doc, Some(consent)).await.unwrap().unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), newer, "old consent cannot overwrite newer external data");
        assert!(ide_host::ops::doc::save_checked(&f.host, "main", f.doc, Some(replacement)).await.unwrap().is_none());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), current);
        assert_eq!(std::fs::read_to_string(f.root.join("app/InvoiceController.php")).unwrap(), MAIN);
        assert_eq!(std::fs::read_to_string(f.root.join("app/Other/InvoiceController.php")).unwrap(), OTHER);
    }).catch_unwind().await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and owned PHP syntax-check executable"]
async fn runtime_class_completion_keeps_catalog_metadata_unsaved_overlays_and_restart() {
    check_runtime_class_catalog(false).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and owned PHP syntax-check executable"]
async fn invokable_completion_keeps_catalog_metadata_unsaved_overlays_and_restart() {
    check_runtime_class_catalog(true).await;
}

async fn check_runtime_class_catalog(invokable: bool) {
    let draft =
        "<?php namespace App; class InvoiceDraftController {public function showInvoice(): bool {return true;}}\n";
    let published = draft.replace("InvoiceDraftController", "InvoicePublishedController");
    let mut extra = vec![("app/InvoiceDraftController.php".into(), draft.to_owned())];
    for n in 0..104 {
        extra.push((format!("app/C{n:03}.php"), format!("<?php namespace App; class C{n:03} {{}}\n")));
    }
    let original = routes(if invokable {
        "Routes::namespace('App')->group(fn () => Routes::get('/😀/main','InvOLD'));"
    } else {
        "Routes::namespace('App')->controller('InvOLD')->group(fn () => Routes::get('/😀/main','showInvoice'));"
    });
    let f = Fixture::new(&original, &extra);
    let outcome = std::panic::AssertUnwindSafe(async {
        let cursor = at(&original, original.find("InvOLD").unwrap() + 2);
        controller(f.complete(cursor).await);
        let dep = f.project.documents.open(&f.root.join("app/InvoiceDraftController.php")).unwrap().id;
        for (text, present, absent) in [
            (draft, "App\\InvoiceDraftController", "App\\InvoicePublishedController"),
            (published.as_str(), "App\\InvoicePublishedController", "App\\InvoiceDraftController"),
        ] {
            f.project.documents.replace_all(dep, text).unwrap();
            let list = f.complete(cursor).await;
            assert!(list.items.iter().any(|i| i.detail.as_deref() == Some(present)), "unsaved candidate missing: {present}");
            assert!(!list.items.iter().any(|i| i.detail.as_deref() == Some(absent)), "retired candidate survived: {absent}");
        }
        let server = f.project.lsp.servers().into_iter().find(|s| s.name == "phpantom").unwrap().id;
        f.project.lsp.restart(server).await.unwrap();
        let list = f.complete(cursor).await;
        assert!(list.items.iter().any(|i| i.detail.as_deref() == Some("App\\InvoicePublishedController")));
        assert!(!list.items.iter().any(|i| i.detail.as_deref() == Some("App\\InvoiceDraftController")));
        assert_eq!(std::fs::read_to_string(f.root.join("app/InvoiceDraftController.php")).unwrap(), draft);
        f.project.documents.close(dep);
        let list = f.complete(cursor).await;
        assert!(list.items.iter().any(|i| i.detail.as_deref() == Some("App\\InvoiceDraftController")));
        assert!(!list.items.iter().any(|i| i.detail.as_deref() == Some("App\\InvoicePublishedController")));
        for call in [
            "Routes::namespace($selected)->controller('InvOLD')->group(fn () => Routes::get('/unknown','showInvoice'));",
            "Routes::namespace('App')->controller('InvOLD')->group(function () {Routes::get('/main','showInvoice');Routes::namespace('Other')->group(fn () => Routes::get('/other','showInvoice'));});",
        ] {
            let input = routes(call);
            Fixture::lint(&input);
            f.project.documents.replace_all(f.doc, &input).unwrap();
            let list = f.complete(at(&input, input.find("InvOLD").unwrap() + 2)).await;
            assert!(list.is_incomplete && list.items.is_empty(), "ambiguous context must remain incomplete: {list:?}");
        }
        if invokable {
            let input = routes("Routes::namespace($selected)->group(fn () => Routes::get('/unknown','InvoiceController'));");
            f.project.documents.replace_all(f.doc, &input).unwrap();
            let cursor = at(&input, input.rfind("InvoiceController").unwrap() + 2);
            let list = f.complete(cursor).await;
            assert!(list.is_incomplete && list.items.is_empty(), "unknown invokable action must retain incomplete metadata: {list:?}");
            assert!(f.project.lsp.prepare_rename_details(f.doc, cursor).await.unwrap().is_none());
            let main = f.project.documents.open(&f.root.join("app/InvoiceController.php")).unwrap().id;
            let failure = ide_host::ops::lsp::lsp_rename(
                &f.host, "main", RequestId::next(), main,
                at(MAIN, MAIN.find("class InvoiceController").unwrap() + "class In".len()), "ReceiptController".into(),
            ).await.unwrap_err();
            assert!(failure.to_string().to_lowercase().contains("runtime controller"), "{failure}");
            assert_eq!(f.project.documents.snapshot(main).unwrap().text, MAIN);
            assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, input);
            assert!(!f.root.join("app/ReceiptController.php").exists());
        }
        let input = routes(if invokable {
            "Routes::namespace('App')->group(fn () => Routes::get('/limit','COLD'));"
        } else {
            "Routes::namespace('App')->controller('COLD')->group(fn () => Routes::get('/limit','showInvoice'));"
        });
        f.project.documents.replace_all(f.doc, &input).unwrap();
        let list = f.complete(at(&input, input.find("COLD").unwrap() + 1)).await;
        assert!(list.is_incomplete);
        assert_eq!(list.items.len(), 100);
        assert!(list.items.iter().all(|i| i.kind == Some(CompletionKind::Class) && i.additional_edits.is_empty()));
        f.project.documents.replace_all(f.doc, &original).unwrap();
        let before = f.project.documents.snapshot(f.doc).unwrap();
        let item = controller(f.complete(cursor).await);
        let changed = format!("{original}// New edit while completion is visible\n");
        f.project.documents.replace_all(f.doc, &changed).unwrap();
        assert!(f.project.documents.apply_edits_at(f.doc, before.version, &[TextEdit {
            range: item.replace_range.unwrap(), new_text: item.insert_text,
        }]).unwrap().is_none());
        assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, changed);
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), original);
    }).catch_unwind().await;
    f.finish(outcome).await;
}

const GLOBAL: &str = "class InvoiceController {public function __invoke():string {return 'global';}}";
const GROUP: &str =
    "<?php namespace App; class GroupController {public function InvoiceController():string {return 'method';}}\n";

fn available_routes(declaration: &str, call: &str) -> String {
    format!(
        "<?php\nnamespace {{{declaration}}}\nnamespace Harness {{use Illuminate\\Support\\Facades\\Route as Routes;use App\\InvoiceController as InvoiceController;{call}}}\n"
    )
}

fn literal(text: &str, prefix: &str) -> Pos {
    at(text, text.rfind(prefix).unwrap() + prefix.len())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and disposable Laravel dependencies"]
async fn available_route_class_uses_guarded_cross_file_rename_and_history() {
    let caller = "<?php namespace App; class Caller {public function make(): \\InvoiceController {return new \\InvoiceController();}}\n";
    let original = available_routes(
        GLOBAL,
        "Routes::controller(\\App\\GroupController::class)->group(fn()=>Routes::get('/😀/available','InvoiceController'));",
    );
    let f = Fixture::new(
        &original,
        &[("app/GroupController.php".into(), GROUP.into()), ("app/Caller.php".into(), caller.into())],
    );
    let outcome=std::panic::AssertUnwindSafe(async {
        let cursor=literal(&original,"'/😀/available','In");
        let targets=f.project.lsp.goto(f.doc,cursor,GotoKind::Definition).await.unwrap();
        assert_eq!(targets.len(),1);assert_eq!(targets[0].path.to_path_buf(),f.root.join("routes/web.php"),"available class must override group method through host");
        assert_eq!(targets[0].range.start,literal(&original,"namespace {"));
        let list=f.complete(cursor).await;assert!(list.items.iter().any(|i|i.kind==Some(CompletionKind::Class)&&i.detail.as_deref()==Some("InvoiceController")),"host must complete the available global class: {list:?}");assert!(!list.items.iter().any(|i|i.kind==Some(CompletionKind::Method)));
        let refs=ide_host::ops::lsp::lsp_references(&f.host,"main",RequestId::next(),f.doc,cursor,true,true,true,ide_host::ops::Channel::new(|_|{})).await.unwrap();
        assert_eq!(refs.locations.len(),4,"class references include declaration/action/closed caller type and constructor: {refs:?}");assert!(refs.incomplete.is_none());assert!(refs.locations.iter().any(|l|l.path.ends_with("app/Caller.php")));
        let group_doc=f.project.documents.open(&f.root.join("app/GroupController.php")).unwrap().id;
        let method=literal(GROUP,"public function ");
        let refs=ide_host::ops::lsp::lsp_references(&f.host,"main",RequestId::next(),group_doc,method,true,true,true,ide_host::ops::Channel::new(|_|{})).await.unwrap();assert_eq!(refs.locations.len(),1,"class action must not count as a group method caller: {refs:?}");
        let unsaved=format!("{original}// Unsaved available-route witness.\n");Fixture::lint(&unsaved);f.project.documents.replace_all(f.doc,&unsaved).unwrap();
        let plan=ide_host::ops::lsp::lsp_rename(&f.host,"main",RequestId::next(),f.doc,literal(&unsaved,"'/😀/available','In"),"ReceiptController".into()).await.unwrap().edit;
        assert!(plan.guard.is_some());assert_eq!(plan.files.len(),2);
        let caller_path=f.root.join("app/Caller.php");let external=format!("{caller}// External caller witness.\n");std::fs::write(&caller_path,&external).unwrap();
        assert!(ide_host::ops::lsp::apply_workspace_edit(&f.host,"main",plan).await.is_err());assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(),original);assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text,unsaved);assert_eq!(std::fs::read_to_string(&caller_path).unwrap(),external);
        std::fs::write(&caller_path,caller).unwrap();
        let plan=ide_host::ops::lsp::lsp_rename(&f.host,"main",RequestId::next(),f.doc,literal(&unsaved,"'/😀/available','In"),"ReceiptController".into()).await.unwrap().edit;
        ide_host::ops::lsp::apply_workspace_edit(&f.host,"main",plan).await.unwrap();
        let expected=unsaved.replacen("namespace {class InvoiceController","namespace {class ReceiptController",1).replacen("'/😀/available','InvoiceController'","'/😀/available','ReceiptController'",1);let changed_caller=caller.replace("\\InvoiceController","\\ReceiptController");
        assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text,expected);assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(),expected);assert_eq!(std::fs::read_to_string(&caller_path).unwrap(),changed_caller);assert_eq!(std::fs::read_to_string(f.root.join("app/GroupController.php")).unwrap(),GROUP);assert_eq!(std::fs::read_to_string(f.root.join("app/InvoiceController.php")).unwrap(),MAIN);
        let repo=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");let laravel=std::env::var_os("OPENSTORM_TEST_ENGINE_LARAVEL_ROOT").map(PathBuf::from).unwrap_or_else(||repo.join("artifacts/checks/engine-laravel13-37-148/project"));
        let script=r#"$loader=require $argv[2].'/vendor/autoload.php';$loader->addPsr4('App\\',$argv[1].'/app',true);$router=new \Illuminate\Routing\Router(new \Illuminate\Events\Dispatcher());\Illuminate\Support\Facades\Route::swap($router);require $argv[1].'/routes/web.php';$routes=$router->getRoutes();assert(count($routes)===1);foreach($routes as $route){assert($route->getAction('uses')==='ReceiptController@__invoke');assert(get_class($route->getController())==='ReceiptController');assert(($route->getController())()==='global');}assert((new \App\Caller())->make() instanceof \ReceiptController);assert((new \App\GroupController())->InvoiceController()==='method');echo 'available class rename runtime passed';"#;
        let output=std::process::Command::new(repo.join("artifacts/toolchains/php-8.4.23-static/php")).args(["-n","-d","zend.assertions=1","-r",script]).arg(&f.root).arg(&laravel).output().unwrap();assert!(output.status.success(),"runtime {} stdout={} stderr={}",output.status,String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));assert_eq!(output.stdout,b"available class rename runtime passed");
        let group=f.events.lock().unwrap().iter().rev().find_map(|event|match event {ide_core::IdeEvent::DocPatched{history:Some(h),..}=>Some(h.group.clone()),_=>None}).expect("reviewed availability history");
        for redo in [false,true] {
            ide_host::ops::lsp::restore_reviewed_edit(&f.host,"main",group.clone(),redo).await.unwrap();
            let current=f.project.documents.snapshot(f.doc).unwrap();assert_eq!(current.text,if redo{&expected}else{&unsaved}.as_str());assert_eq!(current.dirty,!redo);assert_eq!(std::fs::read_to_string(&caller_path).unwrap(),if redo{&changed_caller}else{caller});assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(),if redo{&expected}else{&original}.as_str());
            let prefix=if redo{"'/😀/available','Re"}else{"'/😀/available','In"};let targets=f.project.lsp.goto(f.doc,literal(&current.text,prefix),GotoKind::Definition).await.unwrap();assert_eq!(targets.len(),1);assert_eq!(targets[0].path.to_path_buf(),f.root.join("routes/web.php"));
        }
        std::fs::write(&caller_path,&external).unwrap();assert!(ide_host::ops::lsp::restore_reviewed_edit(&f.host,"main",group,false).await.is_err());assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(),expected);assert_eq!(std::fs::read_to_string(&caller_path).unwrap(),external);
    }).catch_unwind().await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and owned PHP syntax-check executable"]
async fn available_route_class_roles_and_namespace_dependencies_stay_current() {
    let call = "Routes::controller(\\App\\GroupController::class)->group(fn()=>Routes::get('/available','InvoiceController'));";
    let original = available_routes(GLOBAL, call);
    let f = Fixture::new(&original, &[("app/GroupController.php".into(), GROUP.into())]);
    let outcome=std::panic::AssertUnwindSafe(async {
        let first=f.project.lsp.goto(f.doc,literal(&original,"'/available','In"),GotoKind::Definition).await.unwrap();assert_eq!(first.len(),1);assert_eq!(first[0].path.to_path_buf(),f.root.join("routes/web.php"),"availability role must start at global class");
        for (declaration,class_role) in [("",false),("interface InvoiceController {}",false),("trait InvoiceController {}",false),(GLOBAL,true)] {
            let input=available_routes(declaration,call);Fixture::lint(&input);f.project.documents.replace_all(f.doc,&input).unwrap();let targets=f.project.lsp.goto(f.doc,literal(&input,"'/available','In"),GotoKind::Definition).await.unwrap();assert_eq!(targets.len(),1);assert_eq!(targets[0].path.to_path_buf(),f.root.join(if class_role{"routes/web.php"}else{"app/GroupController.php"}),"unsaved availability role: {declaration}");
        }
        let inherited = GLOBAL.replacen("class InvoiceController {", "class InvoiceController extends AvailabilityBase {", 1);
        let deferred = available_routes(&format!("\\Harness\\register_forward();{inherited} class AvailabilityBase {{}}"), &format!("function register_forward(){{{call}}}"));
        let skipped = format!("{}namespace {{class AvailabilityBase {{}}}}\n", available_routes(&format!("goto declaration_skipped;{inherited}declaration_skipped:"),call));
        for input in [deferred, skipped] {
            Fixture::lint(&input);f.project.documents.replace_all(f.doc,&input).unwrap();
            let targets=f.project.lsp.goto(f.doc,literal(&input,"'/available','In"),GotoKind::Definition).await.unwrap();
            assert_eq!(targets.len(),1);assert_eq!(targets[0].path.to_path_buf(),f.root.join("app/GroupController.php"),"lexical order must not invent a host class availability proof");
        }
        f.project.documents.replace_all(f.doc,&original).unwrap();
        let server=f.project.lsp.servers().into_iter().find(|s|s.name=="phpantom").unwrap().id;f.project.lsp.restart(server).await.unwrap();let targets=f.project.lsp.goto(f.doc,literal(&original,"'/available','In"),GotoKind::Definition).await.unwrap();assert_eq!(targets[0].path.to_path_buf(),f.root.join("routes/web.php"));
        let dependent=available_routes(GLOBAL,"Routes::namespace('App')->controller('GroupController')->group(fn()=>Routes::get('/dependent','InvoiceController'));");Fixture::lint(&dependent);f.project.documents.replace_all(f.doc,&dependent).unwrap();let cursor=literal(&dependent,"'/dependent','In");let targets=f.project.lsp.goto(f.doc,cursor,GotoKind::Definition).await.unwrap();assert_eq!(targets.len(),1);assert_eq!(targets[0].path.to_path_buf(),f.root.join("app/InvoiceController.php"),"namespace qualification follows raw availability check");let hover=f.project.lsp.hover(f.doc,cursor,Default::default()).await.unwrap().unwrap();assert!(hover.contents.value.contains("class availability")&&hover.contents.value.contains("Main controller completion witness."));assert!(f.project.lsp.prepare_rename_details(f.doc,cursor).await.unwrap().is_none());let list=f.complete(cursor).await;assert!(list.is_incomplete&&list.items.is_empty());
        let app=f.project.documents.open(&f.root.join("app/InvoiceController.php")).unwrap().id;
        for (doc,pos) in [(f.doc,literal(&dependent,"namespace {class In")),(app,literal(MAIN,"class In"))] {
            let error=ide_host::ops::lsp::lsp_rename(&f.host,"main",RequestId::next(),doc,pos,"ReceiptController".into()).await.unwrap_err();assert!(error.to_string().to_lowercase().contains("runtime controller"),"{error}");assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text,dependent);assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(),original);assert_eq!(std::fs::read_to_string(f.root.join("app/InvoiceController.php")).unwrap(),MAIN);
        }
        let enumerated=available_routes("enum InvoiceController {case Loaded;}","Routes::namespace('App')->controller('GroupController')->group(fn()=>Routes::get('/enum','InvoiceController'));");
        Fixture::lint(&enumerated);f.project.documents.replace_all(f.doc,&enumerated).unwrap();let cursor=literal(&enumerated,"'/enum','In");
        let targets=f.project.lsp.goto(f.doc,cursor,GotoKind::Definition).await.unwrap();assert_eq!(targets.len(),1);assert_eq!(targets[0].path.to_path_buf(),f.root.join("app/InvoiceController.php"),"enum proof must select the namespaced invokable class");
        assert!(f.project.lsp.prepare_rename_details(f.doc,cursor).await.unwrap().is_none());
    }).catch_unwind().await;
    f.finish(outcome).await;
}

async fn handler_references(f: &Fixture, doc: DocId, source: &str, name: &str) -> ide_lsp::types::References {
    let cursor = at(source, source.find(name).unwrap() + 2);
    for _ in 0..80 {
        match ide_host::ops::lsp::lsp_references(
            &f.host,
            "main",
            RequestId::next(),
            doc,
            cursor,
            false,
            true,
            true,
            ide_host::ops::Channel::new(|_| {}),
        )
        .await
        {
            Ok(refs) => return refs,
            Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(error) => panic!("handler references failed: {error}"),
        }
    }
    panic!("handler references did not settle");
}

async fn handler_count(f: &Fixture, doc: DocId, source: &str, name: &str) -> ide_lsp::types::Command {
    let line = at(source, source.find(name).unwrap()).line;
    for _ in 0..80 {
        match ide_host::ops::lsp::lsp_code_lens(&f.host, "main", doc).await {
            Ok(lenses) => {
                let command = lenses
                    .into_iter()
                    .filter(|l| l.range.start.line == line)
                    .filter_map(|l| l.command)
                    .find(|c| c.title.ends_with("reference") || c.title.ends_with("references"))
                    .expect("implicit handler declaration must expose a reference lens");
                if command.title != "- references" {
                    return command;
                }
            }
            Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {}
            Err(error) => panic!("handler count failed: {error}"),
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("handler reference lens remained pending");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and owned PHP syntax-check executable"]
async fn implicit_handler_references_counts_and_unsaved_inheritance_through_host() {
    const BASE: &str =
        "<?php\nnamespace App;\nclass BaseInvoice {\n public function __invoke(): string {return 'base';}\n}\n";
    const CHILD: &str = "<?php namespace App; class InvoiceController extends BaseInvoice {}\n";
    const INVOKABLE_OTHER: &str = "<?php\nnamespace App\\Other;\nclass InvoiceController {\n public function __invoke(): bool {return true;}\n}\n";
    const UNUSED: &str = "<?php\nnamespace App;\nclass Unused {\n public function __INVOKE(): float {return 1.0;}\n}\n";
    const GROUP: &str =
        "<?php namespace App; class GroupController {public function InvoiceController():int{return 1;}}\n";
    const ROUTES: &str = "<?php\nnamespace Harness;\nuse Illuminate\\Support\\Facades\\Route as Routes;\nuse App\\InvoiceController as InvoiceAlias;\nRoutes::get('/😀/literal','App\\InvoiceController');\nRoutes::get('/constant',InvoiceAlias::class);\nRoutes::namespace('App')->group(fn()=>Routes::get('/namespace','InvoiceController'));\nRoutes::match(action:(InvoiceAlias::class),uri:'/named',methods:['GET']);\nRoutes::controller(\\App\\GroupController::class)->group(fn()=>Routes::get('/method','InvoiceController'));\nRoutes::get('/other',\\App\\Other\\InvoiceController::class);\n$metadata=InvoiceAlias::class;$ordinary='App\\InvoiceController';\n";
    let f = Fixture::new(
        ROUTES,
        &[
            ("app/BaseInvoice.php".into(), BASE.into()),
            ("app/InvoiceController.php".into(), CHILD.into()),
            ("app/Other/InvoiceController.php".into(), INVOKABLE_OTHER.into()),
            ("app/Unused.php".into(), UNUSED.into()),
            ("app/GroupController.php".into(), GROUP.into()),
        ],
    );
    let outcome = std::panic::AssertUnwindSafe(async {
        tokio::time::timeout(Duration::from_secs(90), async {
            let base = f.project.documents.open(&f.root.join("app/BaseInvoice.php")).unwrap().id;
            let child = f.project.documents.open(&f.root.join("app/InvoiceController.php")).unwrap().id;
            let other = f.project.documents.open(&f.root.join("app/Other/InvoiceController.php")).unwrap().id;
            let unused = f.project.documents.open(&f.root.join("app/Unused.php")).unwrap().id;
            let changed = ROUTES
                .replace("'/😀/literal','App\\InvoiceController'", "'/😀/literal','App\\Unused'")
                .replace("'/constant',InvoiceAlias::class", "'/constant',\\App\\Other\\InvoiceController::class");
            Fixture::lint(&changed);
            for phase in 0..3 {
                if phase == 1 {
                    f.project.documents.replace_all(f.doc, &changed).unwrap();
                }
                if phase == 2 {
                    f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
                }
                let source = if phase == 0 { ROUTES } else { &changed };
                let expected = if phase == 0 {
                    vec![
                        literal(source, "'/😀/literal','"),
                        literal(source, "'/constant',"),
                        literal(source, "'/namespace','"),
                        literal(source, "action:("),
                    ]
                } else {
                    vec![literal(source, "'/namespace','"), literal(source, "action:(")]
                };
                let refs = handler_references(&f, base, BASE, "__invoke").await;
                let mut actual: Vec<_> = refs
                    .locations
                    .iter()
                    .map(|l| {
                        assert_eq!(l.path.to_path_buf(), f.root.join("routes/web.php"));
                        l.range.start
                    })
                    .collect();
                let mut expected = expected;
                actual.sort_by_key(|p| (p.line, p.col));
                expected.sort_by_key(|p| (p.line, p.col));
                assert_eq!(
                    actual, expected,
                    "implicit inherited handler references must retain exact UTF16 action ranges"
                );
                for (doc, text, name, count) in [
                    (base, BASE, "__invoke", if phase == 0 { 4 } else { 2 }),
                    (other, INVOKABLE_OTHER, "__invoke", if phase == 0 { 1 } else { 2 }),
                    (unused, UNUSED, "__INVOKE", if phase == 0 { 0 } else { 1 }),
                ] {
                    let refs = handler_references(&f, doc, text, name).await;
                    assert!(refs.incomplete.is_none(), "owned handler fixture unexpectedly incomplete: {refs:?}");
                    assert_eq!(refs.locations.len(), count);
                    let lens = handler_count(&f, doc, text, name).await;
                    assert_eq!(lens.title, format!("{count} reference{}", if count == 1 { "" } else { "s" }));
                    assert_eq!(
                        lens.arguments[2].as_array().unwrap().len(),
                        count,
                        "lens command must carry the exact reference set"
                    );
                    let mut lens_sites: Vec<_> = lens.arguments[2]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|location| {
                            (
                                location["uri"].as_str().unwrap().to_owned(),
                                location["range"]["start"]["line"].as_u64().unwrap(),
                                location["range"]["start"]["character"].as_u64().unwrap(),
                            )
                        })
                        .collect();
                    let mut reference_sites: Vec<_> = refs
                        .locations
                        .iter()
                        .map(|location| {
                            (
                                ide_lsp::path_to_uri_string(&location.path),
                                u64::from(location.range.start.line),
                                u64::from(location.range.start.col),
                            )
                        })
                        .collect();
                    lens_sites.sort();
                    reference_sites.sort();
                    assert_eq!(lens_sites, reference_sites);
                }
                let targets =
                    f.project.lsp.goto(f.doc, literal(source, "action:("), GotoKind::Definition).await.unwrap();
                assert_eq!(targets.len(), 1);
                assert_eq!(
                    targets[0].path.to_path_buf(),
                    f.root.join("app/InvoiceController.php"),
                    "class constant must keep PHP class navigation"
                );
                assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), ROUTES);
            }
            let explicit = "<?php $controller=new \\App\\InvoiceController(); $controller->__invoke();\n";
            Fixture::lint(explicit);
            let explicit_path = f.root.join("app/ExplicitCaller.php");
            std::fs::write(&explicit_path, explicit).unwrap();
            let explicit_doc = f.project.documents.open(&explicit_path).unwrap().id;
            let call_position = literal(explicit, "$controller->__in");
            let before_override = f.project.lsp.goto(explicit_doc, call_position, GotoKind::Definition).await.unwrap();
            assert_eq!(before_override.len(), 1);
            assert_eq!(before_override[0].path.to_path_buf(), f.root.join("app/BaseInvoice.php"));
            let overridden = CHILD.replace(
                "extends BaseInvoice {}",
                "extends BaseInvoice {\n public function __invoke():string{return 'override';}\n}",
            );
            Fixture::lint(&overridden);
            f.project.documents.replace_all(child, &overridden).unwrap();
            let overridden_targets =
                f.project.lsp.goto(explicit_doc, call_position, GotoKind::Definition).await.unwrap();
            assert_eq!(overridden_targets.len(), 1);
            assert_eq!(overridden_targets[0].path.to_path_buf(), f.root.join("app/InvoiceController.php"));
            assert_eq!(handler_references(&f, base, BASE, "__invoke").await.locations.len(), 3);
            assert_eq!(handler_count(&f, base, BASE, "__invoke").await.title, "3 references");
            assert_eq!(handler_references(&f, child, &overridden, "__invoke").await.locations.len(), 3);
            assert_eq!(handler_count(&f, child, &overridden, "__invoke").await.title, "3 references");
            f.project.documents.replace_all(child, CHILD).unwrap();
            let restored_targets = f.project.lsp.goto(explicit_doc, call_position, GotoKind::Definition).await.unwrap();
            assert_eq!(restored_targets.len(), 1);
            assert_eq!(restored_targets[0].path.to_path_buf(), f.root.join("app/BaseInvoice.php"));
            assert_eq!(handler_references(&f, base, BASE, "__invoke").await.locations.len(), 3);
            assert_eq!(handler_count(&f, base, BASE, "__invoke").await.title, "3 references");
            assert_eq!(std::fs::read_to_string(f.root.join("app/InvoiceController.php")).unwrap(), CHILD);
            assert!(f.project.documents.snapshot(f.doc).unwrap().dirty);
        })
        .await
        .expect("implicit handler host workflow exceeded its bound");
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and disposable Laravel dependencies"]
async fn implicit_handler_checked_rename_guards_and_reviewed_class_edits_through_host() {
    const ORIGINAL: &str = "<?php\nnamespace Harness;\nuse Illuminate\\Support\\Facades\\Route as Routes;\nuse App\\InvoiceController as InvoiceAlias;\nRoutes::get('/😀/literal','App\\InvoiceController');\nRoutes::get('/constant',InvoiceAlias::class);\n";
    const CALLER: &str = "<?php namespace App; class Caller {public function make():InvoiceController{return new InvoiceController();}}\n";
    const QUALIFIED: &str =
        "<?php namespace Other\\App; class InvoiceController {public function __invoke():int{return 42;}}\n";
    let f = Fixture::new(
        ORIGINAL,
        &[("app/Caller.php".into(), CALLER.into()), ("other/InvoiceController.php".into(), QUALIFIED.into()), ("composer.json".into(), r#"{"require":{"php":"^8.3","laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/","Other\\App\\":"other/"}}}"#.into())],
    );
    let outcome=std::panic::AssertUnwindSafe(async {
        tokio::time::timeout(Duration::from_secs(90),async {
            let main_path=f.root.join("app/InvoiceController.php");let new_path=f.root.join("app/ReceiptController.php");let caller_path=f.root.join("app/Caller.php");let route_path=f.root.join("routes/web.php");
            let main=f.project.documents.open(&main_path).unwrap().id;
            let unsaved=ORIGINAL.replace("/😀/literal","/😀/unsaved-literal");f.project.documents.replace_all(f.doc,&unsaved).unwrap();
            let error=ide_host::ops::lsp::lsp_rename(&f.host,"main",RequestId::next(),main,literal(MAIN,"function __in"),"dispatch".into()).await.unwrap_err();
            assert!(error.to_string().contains("__invoke contract"),"checked rename must expose the magic-contract refusal: {error}");
            assert_eq!(f.project.documents.snapshot(main).unwrap().text,MAIN);assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text,unsaved);assert_eq!(std::fs::read_to_string(&route_path).unwrap(),ORIGINAL);assert_eq!(std::fs::read_to_string(&caller_path).unwrap(),CALLER);assert!(!new_path.exists());
            let plan=ide_host::ops::lsp::lsp_rename(&f.host,"main",RequestId::next(),main,literal(MAIN,"class In"),"ReceiptController".into()).await.unwrap();
            assert!(plan.edit.guard.is_some() && plan.edit.files.len() >= 3 && plan.incomplete.is_none());assert!(plan.edit.files.iter().any(|op|matches!(op,ide_core::FileEdit::Rename{from,to} if **from==main_path&&**to==new_path)),"checked class rename must retain PSR4 move");
            // Computing a preview leaves disk and unsaved overlays intact.
            assert_eq!(std::fs::read_to_string(&main_path).unwrap(),MAIN);assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text,unsaved);
            let external=format!("{CALLER}// external\n");std::fs::write(&caller_path,&external).unwrap();
            assert!(ide_host::ops::lsp::apply_workspace_edit(&f.host,"main",plan.edit).await.is_err());assert_eq!(std::fs::read_to_string(&caller_path).unwrap(),external);assert_eq!(std::fs::read_to_string(&route_path).unwrap(),ORIGINAL);assert_eq!(std::fs::read_to_string(&main_path).unwrap(),MAIN);assert!(!new_path.exists());
            std::fs::write(&caller_path,CALLER).unwrap();
            let plan=ide_host::ops::lsp::lsp_rename(&f.host,"main",RequestId::next(),main,literal(MAIN,"class In"),"ReceiptController".into()).await.unwrap();
            ide_host::ops::lsp::apply_workspace_edit(&f.host,"main",plan.edit).await.unwrap();
            assert!(!main_path.exists());assert_eq!(f.project.documents.by_path(&new_path),Some(main));
            let changed=f.project.documents.snapshot(f.doc).unwrap().text;assert!(changed.contains("as InvoiceAlias;")&&changed.contains("InvoiceAlias::class"));assert!(!changed.contains("InvoiceController"));assert!(changed.contains("unsaved-literal"));assert!(!f.project.documents.snapshot(f.doc).unwrap().dirty);
            for path in [&new_path,&caller_path,&route_path] {Fixture::lint(&std::fs::read_to_string(path).unwrap());}
            let repo=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");let laravel=std::env::var_os("OPENSTORM_TEST_ENGINE_LARAVEL_ROOT").map(PathBuf::from).unwrap_or_else(||repo.join("artifacts/checks/engine-laravel13-37-148/project"));
            let php=std::env::var_os("OPENSTORM_TEST_PHP_BIN").map(PathBuf::from).unwrap_or_else(||repo.join("artifacts/toolchains/php-8.4.23-static/php"));
            let script=r#"$loader=require $argv[2].'/vendor/autoload.php';$loader->addPsr4('App\\',$argv[1].'/app',true);$router=new \Illuminate\Routing\Router(new \Illuminate\Events\Dispatcher());\Illuminate\Support\Facades\Route::swap($router);require $argv[1].'/routes/web.php';assert(count($router->getRoutes())===2);foreach($router->getRoutes() as $route){assert(ltrim($route->getAction('uses'),'\\')==='App\\ReceiptController@__invoke');assert(get_class($route->getController())==='App\\ReceiptController');assert(($route->getController())()===true);}assert((new \App\Caller())->make() instanceof \App\ReceiptController);echo 'implicit checked class rename runtime passed';"#;
            let output=std::process::Command::new(&php).args(["-n","-d","zend.assertions=1","-r",script]).arg(&f.root).arg(&laravel).output().unwrap();assert!(output.status.success(),"runtime stdout={} stderr={}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));assert_eq!(output.stdout,b"implicit checked class rename runtime passed");
            let group=f.events.lock().unwrap().iter().rev().find_map(|e|match e{ide_core::IdeEvent::DocPatched{history:Some(h),..}=>Some(h.group.clone()),_=>None}).unwrap();
            for redo in [false,true,false] {
                ide_host::ops::lsp::restore_reviewed_edit(&f.host,"main",group.clone(),redo).await.unwrap();let snap=f.project.documents.snapshot(f.doc).unwrap();assert_eq!(snap.text,if redo{&changed}else{&unsaved}.as_str());assert_eq!(snap.dirty,!redo);assert_eq!(std::fs::read_to_string(&route_path).unwrap(),if redo { changed.as_str() } else { ORIGINAL });assert_eq!(f.project.documents.by_path(if redo{&new_path}else{&main_path}),Some(main));
            }
            let dependency_script=r#"$loader=require $argv[2].'/vendor/autoload.php';$loader->addPsr4('App\\',$argv[1].'/app',true);$loader->addPsr4('Other\\App\\',$argv[1].'/other',true);$router=new \Illuminate\Routing\Router(new \Illuminate\Events\Dispatcher());\Illuminate\Support\Facades\Route::swap($router);\Illuminate\Support\Facades\Route::namespace('Other')->group(fn()=>\Illuminate\Support\Facades\Route::get('/dependent',\App\InvoiceController::class));$route=$router->getRoutes()->getRoutes()[0];assert($route->getAction('uses')==='Other\\App\\InvoiceController@__invoke');assert(($route->getController())()===42);echo 'namespace dependency runtime passed';"#;
            let output=std::process::Command::new(&php).args(["-n","-d","zend.assertions=1","-r",dependency_script]).arg(&f.root).arg(&laravel).output().unwrap();assert!(output.status.success(),"namespace oracle stdout={} stderr={}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));assert_eq!(output.stdout,b"namespace dependency runtime passed");
            let final_doc=f.project.documents.open(&f.root.join("other/InvoiceController.php")).unwrap().id;
            for namespace in ["'Other'","$selected"] {
                let dependent=format!("{HEADER}Routes::namespace({namespace})->group(fn()=>Routes::get('/prefixed',\\App\\InvoiceController::class));\n");Fixture::lint(&dependent);f.project.documents.replace_all(f.doc,&dependent).unwrap();
                for (doc,text) in [(main,MAIN),(final_doc,QUALIFIED)] {
                    let error=ide_host::ops::lsp::lsp_rename(&f.host,"main",RequestId::next(),doc,literal(text,"class In"),"ReceiptController".into()).await.unwrap_err();assert!(error.to_string().to_lowercase().contains("namespace-dependent controller class constant"),"namespace-dependent class constant must refuse partial edits: {error}");assert_eq!(f.project.documents.snapshot(doc).unwrap().text,text);
                }
                assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text,dependent);assert_eq!(std::fs::read_to_string(&route_path).unwrap(),ORIGINAL);assert_eq!(std::fs::read_to_string(&main_path).unwrap(),MAIN);assert_eq!(std::fs::read_to_string(&caller_path).unwrap(),CALLER);assert!(!new_path.exists());
            }
        }).await.expect("implicit checked-rename host workflow exceeded its bound");
    }).catch_unwind().await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires candidate156 and owned PHP syntax-check executable"]
async fn controller_url_method_completion_uses_real_host_edits_and_save_conflicts() {
    let original = routes(r"\action('\App\InvoiceController@showInvoice');");
    let declaration = "<?php namespace App; class InvoiceController {/** URL method documentation. */ public function showInvoice():string{return 'ok';} /** URL method documentation. */ public function shøw():string{return 'unicode';} protected function secret(){} private function hidden(){}}\n";
    let f = Fixture::new(&original, &[("app/InvoiceController.php".into(), declaration.into())]);
    let outcome = std::panic::AssertUnwindSafe(async {
        for (call, suffix, method) in [
            (r"$label='😀'; \action('\App\InvoiceController@shOLD');", "shOLD", "showInvoice"),
            (
                r#"use function action as controller_url; controller_url(name:"\\App\\InvoiceController@shøOLD");"#,
                "shøOLD",
                "shøw",
            ),
            (
                r"\Illuminate\Support\Facades\URL::action(action:'\App\InvoiceController@shOLD');",
                "shOLD",
                "showInvoice",
            ),
        ] {
            let input = routes(call);
            Fixture::lint(&input);
            f.project.documents.replace_all(f.doc, &input).unwrap();
            let before = f.project.documents.snapshot(f.doc).unwrap();
            let start = input.find(&format!("@{suffix}")).unwrap() + 1;
            let range = Range::new(at(&input, start), at(&input, start + suffix.len()));
            let cursor = at(&input, start + suffix.len() - "OLD".len());
            let list = f.complete(cursor).await;
            assert!(!list.is_incomplete);
            let item = list
                .items
                .into_iter()
                .find(|i| i.filter_text.as_deref() == Some(method))
                .expect("runtime URL method completion");
            assert_eq!(item.kind, Some(CompletionKind::Method));
            assert_eq!(item.replace_range, Some(range));
            assert!(!item.is_snippet && item.additional_edits.is_empty() && item.command.is_none());
            let resolved = ide_host::ops::lsp::lsp_resolve_completion(&f.host, "main", f.doc, item).await.unwrap();
            assert_eq!(resolved.insert_text, method);
            assert_eq!(resolved.replace_range, Some(range));
            assert!(resolved.documentation.as_ref().unwrap().value.contains("URL method documentation."));
            assert!(resolved.additional_edits.is_empty() && resolved.command.is_none() && !resolved.is_snippet);
            let edit = TextEdit { range, new_text: resolved.insert_text };
            assert!(f.project.documents.apply_edits_at(f.doc, before.version, &[edit]).unwrap().is_some());
            let current = f.project.documents.snapshot(f.doc).unwrap();
            assert_eq!(current.text, input.replace(&format!("@{suffix}"), &format!("@{method}")));
            assert!(current.dirty);
            assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), original);
            let targets = f.project.lsp.goto(f.doc, range.start, GotoKind::Definition).await.unwrap();
            assert_eq!(targets.len(), 1);
            assert_eq!(targets[0].path.to_path_buf(), f.root.join("app/InvoiceController.php"));
        }
        let path = f.root.join("routes/web.php");
        let current = f.project.documents.snapshot(f.doc).unwrap().text;
        let external = format!("{original}// External change\n");
        std::fs::write(&path, &external).unwrap();
        let consent = ide_host::ops::doc::save_checked(&f.host, "main", f.doc, None).await.unwrap().unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), external);
        assert!(ide_host::ops::doc::save_checked(&f.host, "main", f.doc, Some(consent)).await.unwrap().is_none());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), current);
        assert_eq!(std::fs::read_to_string(f.root.join("app/InvoiceController.php")).unwrap(), declaration);
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires candidate156 and owned PHP syntax-check executable"]
async fn controller_url_method_completion_refreshes_unsaved_parents_and_restart() {
    let original = routes(r"\action('\App\InvoiceController@sh');");
    let declaration = "<?php namespace App; class InvoiceController extends BaseController {}\n";
    let f = Fixture::new(&original, &[
        ("app/InvoiceController.php".into(), declaration.into()),
        ("app/BaseController.php".into(), "<?php namespace App; class BaseController {/** Base URL docs. */ public function showInvoice():string{return 'base';} protected function secret(){}}\n".into()),
        ("app/OtherController.php".into(), "<?php namespace App; class OtherController {/** Other URL docs. */ public function showOther():string{return 'other';}}\n".into()),
    ]);
    let outcome = std::panic::AssertUnwindSafe(async {
        let parent = f.project.documents.open(&f.root.join("app/InvoiceController.php")).unwrap().id;
        let at = at(&original, original.find("@sh").unwrap() + 3);
        for (name, restart, expected, absent, docs) in [
            ("BaseController", false, "showInvoice", "showOther", "Base URL docs."),
            ("OtherController", false, "showOther", "showInvoice", "Other URL docs."),
            ("OtherController", true, "showOther", "showInvoice", "Other URL docs."),
            ("BaseController", false, "showInvoice", "showOther", "Base URL docs."),
        ] {
            if restart {
                f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
            } else {
                f.project.documents.replace_all(parent, &declaration.replace("BaseController", name)).unwrap();
            }
            let list = f.complete(at).await;
            assert!(!list.is_incomplete);
            assert!(
                !list
                    .items
                    .iter()
                    .any(|i| i.filter_text.as_deref() == Some(absent) || i.filter_text.as_deref() == Some("secret"))
            );
            let item = list
                .items
                .into_iter()
                .find(|i| i.filter_text.as_deref() == Some(expected))
                .expect("current unsaved parent method");
            let resolved = ide_host::ops::lsp::lsp_resolve_completion(&f.host, "main", f.doc, item).await.unwrap();
            assert!(resolved.documentation.as_ref().unwrap().value.contains(docs));
            assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, original);
            assert_eq!(std::fs::read_to_string(f.root.join("app/InvoiceController.php")).unwrap(), declaration);
        }
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}
