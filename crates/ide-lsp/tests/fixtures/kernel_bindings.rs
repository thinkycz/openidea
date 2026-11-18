use super::*;
use futures::FutureExt;

const CALLER: &str = r#"<?php
use Illuminate\Support\Facades\Route;
new \App\Http\Middleware\Audit;
/* 😀 */ Route::middleware("au\x64ited:strict");
"#;
const BOOTSTRAP: &str = r"<?php
use Illuminate\Contracts\Http\Kernel as HttpKernel;
use App\Runtime\SiteKernel as ProjectKernel;
$app = new \Illuminate\Foundation\Application;
$app->singleton(HttpKernel::class, ProjectKernel::class);
return $app;
";
const CLASS: &str = "<?php namespace App\\Http\\Middleware; class Audit {}\n";
const SELECTED: &str = r#"<?php
namespace App\Runtime;
use App\Http\Middleware\Audit as MiddlewareTarget;
class SiteKernel extends \Illuminate\Foundation\Http\Kernel {
    protected $middlewareAliases = [/* 😀 */ "au\x64ited" => MiddlewareTarget::class];
}
"#;

fn rename_candidates(root: &std::path::Path) -> Vec<PathBuf> {
    fn collect(root: &std::path::Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            if matches!(entry.file_name().to_str(), Some("vendor" | "node_modules" | ".git")) {
                continue;
            }
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                collect(&entry.path(), files);
            } else if kind.is_file() && entry.path().extension().is_some_and(|extension| extension == "php") {
                files.push(entry.path());
            }
        }
    }
    let mut files = Vec::new();
    collect(root, &mut files);
    files.sort();
    assert!(files.len() < 1024);
    files
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom167 and owned PHP syntax-check executable"]
async fn bound_kernel_client_tracks_utf16_references_dirty_selection_and_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let next = SELECTED
        .replace("SiteKernel extends", "NextKernel extends")
        .replace("Middleware\\Audit as", "Middleware\\Trace as");
    let conventional = SELECTED
        .replace("namespace App\\Runtime;", "namespace App\\Http;")
        .replace("SiteKernel extends", "Kernel extends")
        .replace("Middleware\\Audit as", "Middleware\\Trace as");
    let files = [
        ("composer.json", r#"{"require":{"laravel/framework":"^10.0"},"autoload":{"psr-4":{"App\\":"app/","Illuminate\\":"framework/"}}}"#.to_owned()),
        ("routes/web.php", CALLER.into()),
        ("bootstrap/app.php", BOOTSTRAP.into()),
        ("app/Runtime/SiteKernel.php", SELECTED.into()),
        ("app/Runtime/NextKernel.php", next),
        ("app/Http/Kernel.php", conventional),
        ("app/Http/Middleware/Audit.php", CLASS.into()),
        ("app/Http/Middleware/Trace.php", CLASS.replace("Audit", "Trace")),
        ("framework/Foundation/Http/Kernel.php", "<?php namespace Illuminate\\Foundation\\Http; class Kernel {}\n".into()),
        ("framework/Contracts/Http/Kernel.php", "<?php namespace Illuminate\\Contracts\\Http; interface Kernel {}\n".into()),
        ("framework/Foundation/Application.php", "<?php namespace Illuminate\\Foundation; class Application {public function singleton($abstract,$concrete=null): void {}}\n".into()),
    ];
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let php = std::env::var_os("OPENSTORM_TEST_PHP_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("artifacts/toolchains/php-8.4.23-static/php"));
    for (name, text) in &files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        if name.ends_with(".php") {
            let result = std::process::Command::new(&php).args(["-n", "-l"]).arg(&path).output().unwrap();
            assert!(result.status.success(), "{name}: {}", String::from_utf8_lossy(&result.stderr));
        }
    }
    let engine = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("src-tauri/resources/phpantom/phpantom_lsp"));
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![route_handler_language(&engine)])),
        Arc::new(|_| {}),
    );
    let candidates = rename_candidates(&root);
    mgr.set_candidates_hook(Box::new(move |query, cancel| ide_lsp::CandidateFiles {
        // The host searches by word; retain a conservative superset in this bounded inventory.
        files: candidates
            .iter()
            .filter(|path| {
                !cancel.is_cancelled()
                    && path.starts_with(&query.root)
                    && !query.skip.iter().any(|skip| path.starts_with(skip))
                    && std::fs::read_to_string(path).unwrap().contains(&query.word)
            })
            .cloned()
            .collect(),
        limit_hit: false,
    }));
    let doc = DocId::next();
    let bootstrap = DocId::next();
    let class = DocId::next();
    let selected = root.join("app/Runtime/SiteKernel.php");
    mgr.did_open(doc, &root.join("routes/web.php"), &LanguageId::new("php"), 1, CALLER);
    mgr.did_open(bootstrap, &root.join("bootstrap/app.php"), &LanguageId::new("php"), 1, BOOTSTRAP);
    mgr.did_open(class, &root.join("app/Http/Middleware/Audit.php"), &LanguageId::new("php"), 1, CLASS);
    let outcome = std::panic::AssertUnwindSafe(async {
        assert!(!mgr.goto(doc, pos(CALLER, "Audit;"), GotoKind::Definition).await.unwrap().is_empty());
        let position = pos(CALLER, r"64ited");
        let found = mgr.goto(doc, position, GotoKind::Definition).await.unwrap();
        assert_eq!(found.len(), 1, "client bound Kernel selection: {found:?}");
        assert_eq!(*found[0].path, selected);
        assert_eq!(found[0].range.start, pos(SELECTED, r"au\x64ited"));
        let refs = mgr.references(doc, position, true, true, Default::default()).await.unwrap();
        assert!(refs.incomplete.is_none(), "{refs:?}");
        assert_eq!(refs.locations.len(), 2, "{refs:?}");
        for location in refs.locations {
            assert!(*location.path == selected || *location.path == root.join("routes/web.php"));
            assert_eq!(location.range.end.col - location.range.start.col, 10);
        }
        let rename = mgr.rename(class, pos(CLASS, "Audit {}"), "Auditor".into(), Default::default()).await.unwrap().edit;
        assert!(rename.files.iter().any(|edit| matches!(edit, ide_core::FileEdit::Edit {path, edits, ..} if **path == selected && edits.iter().any(|edit| edit.new_text.contains("Auditor")))), "{rename:?}");
        let changed = BOOTSTRAP.replace("SiteKernel as", "NextKernel as");
        assert_eq!(changed.len(), BOOTSTRAP.len());
        replace(&mgr, bootstrap, 2, BOOTSTRAP, &changed);
        let found = mgr.goto(doc, position, GotoKind::Definition).await.unwrap();
        assert_eq!(found.len(), 1, "client dirty bound Kernel: {found:?}");
        assert_eq!(*found[0].path, root.join("app/Runtime/NextKernel.php"));
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        let found = mgr.goto(doc, position, GotoKind::Definition).await.unwrap();
        assert_eq!(found.len(), 1, "client replayed bound Kernel: {found:?}");
        assert_eq!(*found[0].path, root.join("app/Runtime/NextKernel.php"));
        replace(&mgr, bootstrap, 3, &changed, BOOTSTRAP);
        assert_eq!(*mgr.goto(doc, position, GotoKind::Definition).await.unwrap()[0].path, selected);
    }).catch_unwind().await;
    mgr.shutdown_all().await;
    assert!(mgr.servers().iter().all(|server| server.pid.is_none()));
    for (name, text) in files {
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), text);
    }
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom167 and the retained Laravel13.37 bound Kernel fixture"]
async fn bound_kernel_client_resolves_real_laravel_and_refuses_incomplete_edits() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = std::env::var_os("OPENSTORM_TEST_BOUND_KERNEL_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("artifacts/checks/slice149-laravel-example-13-37"))
        .canonicalize()
        .unwrap();
    let engine = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("src-tauri/resources/phpantom/phpantom_lsp"));
    let caller = root.join("app/Demo.php");
    let declaration = root.join("app/Runtime/SiteKernel.php");
    let bootstrap_path = root.join("bootstrap/app.php");
    let class_path = root.join("app/Http/Middleware/AuditRequests.php");
    let source = std::fs::read_to_string(&caller).unwrap();
    let registration = std::fs::read_to_string(&declaration).unwrap();
    let initial = std::fs::read_to_string(&bootstrap_path).unwrap();
    let class_text = std::fs::read_to_string(&class_path).unwrap();
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![route_handler_language(&engine)])),
        Arc::new(|_| {}),
    );
    let candidates = rename_candidates(&root);
    mgr.set_candidates_hook(Box::new(move |query, cancel| ide_lsp::CandidateFiles {
        // The host searches by word; retain a conservative superset in this bounded inventory.
        files: candidates
            .iter()
            .filter(|path| {
                !cancel.is_cancelled()
                    && path.starts_with(&query.root)
                    && !query.skip.iter().any(|skip| path.starts_with(skip))
                    && std::fs::read_to_string(path).unwrap().contains(&query.word)
            })
            .cloned()
            .collect(),
        limit_hit: false,
    }));
    let doc = DocId::next();
    let bootstrap = DocId::next();
    let class = DocId::next();
    let selected_doc = DocId::next();
    mgr.did_open(doc, &caller, &LanguageId::new("php"), 1, &source);
    mgr.did_open(bootstrap, &bootstrap_path, &LanguageId::new("php"), 1, &initial);
    mgr.did_open(class, &class_path, &LanguageId::new("php"), 1, &class_text);
    mgr.did_open(selected_doc, &declaration, &LanguageId::new("php"), 1, &registration);
    let outcome = std::panic::AssertUnwindSafe(async {
        assert!(
            !mgr.goto(selected_doc, pos(&registration, "AuditRequests;"), GotoKind::Definition)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(source.matches("demo.kernel.bound").count(), 1);
        let position = pos(&source, "demo.kernel.bound");
        let found = mgr.goto(doc, position, GotoKind::Definition).await.unwrap();
        assert_eq!(found.len(), 1, "real Laravel bound Kernel client navigation: {found:?}");
        assert_eq!(*found[0].path, declaration);
        assert_eq!(found[0].range.start, pos(&registration, "demo.kernel.bound"));
        let hover = mgr.hover(doc, position, Default::default()).await.unwrap().unwrap();
        assert!(
            hover.contents.value.contains("app/Runtime/SiteKernel.php") && hover.contents.value.contains("incomplete"),
            "{hover:?}"
        );
        let refs = mgr.references(doc, position, true, true, Default::default()).await.unwrap();
        assert!(
            refs.locations.iter().any(
                |location| *location.path == declaration && location.range.end.col - location.range.start.col == 17
            ),
            "{refs:?}"
        );
        let refused =
            mgr.rename(class, pos(&class_text, "AuditRequests\n"), "ReviewedAudit".into(), Default::default()).await;
        assert!(
            refused.as_ref().is_err_and(|error| error.message.contains("middleware")),
            "real Laravel incomplete Kernel plan: {refused:?}"
        );
        let changed = initial.replace("\\App\\Runtime\\SiteKernel::class", "\\App\\Http\\Kernel::class");
        assert_ne!(changed, initial);
        replace(&mgr, bootstrap, 2, &initial, &changed);
        assert!(mgr.goto(doc, position, GotoKind::Definition).await.unwrap().is_empty());
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        assert!(mgr.goto(doc, position, GotoKind::Definition).await.unwrap().is_empty());
        replace(&mgr, bootstrap, 3, &changed, &initial);
        assert_eq!(*mgr.goto(doc, position, GotoKind::Definition).await.unwrap()[0].path, declaration);
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    assert!(mgr.servers().iter().all(|server| server.pid.is_none()));
    for (path, text) in
        [(&caller, &source), (&declaration, &registration), (&bootstrap_path, &initial), (&class_path, &class_text)]
    {
        assert_eq!(std::fs::read_to_string(path).unwrap(), *text);
    }
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom168 and the retained Laravel13.37 standard-builder fixture"]
async fn builder_kernel_client_retires_inactive_legacy_aliases_and_keeps_queued_binding_phase() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = std::env::var_os("OPENSTORM_TEST_BUILDER_KERNEL_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("artifacts/checks/slice152-laravel-example-13-37"))
        .canonicalize()
        .unwrap();
    let engine = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("src-tauri/resources/phpantom/phpantom_lsp"));
    let path = root.join("app/Demo.php");
    let source = std::fs::read_to_string(&path).unwrap();
    let bootstrap = root.join("bootstrap/app.php");
    let original = std::fs::read_to_string(&bootstrap).unwrap();
    let binding = r"$application->singleton(HttpKernelContract::class, \App\Runtime\SiteKernel::class);";
    assert_eq!(original.matches(binding).count(), 1);
    let standard = original.replace(binding, "");
    let parent = root.join("app/Http/ApplicationKernel.php");
    let parent_source = std::fs::read_to_string(&parent).unwrap();
    let declaration = root.join("app/Http/Middleware/AuditRequests.php");
    let class_source = std::fs::read_to_string(&declaration).unwrap();
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![route_handler_language(&engine)])),
        Arc::new(|_| {}),
    );
    let doc = DocId::next();
    let bootstrap_doc = DocId::next();
    let parent_doc = DocId::next();
    mgr.did_open(doc, &path, &LanguageId::new("php"), 1, &source);
    mgr.did_open(bootstrap_doc, &bootstrap, &LanguageId::new("php"), 1, &standard);
    mgr.did_open(parent_doc, &parent, &LanguageId::new("php"), 1, &parent_source);
    let outcome = std::panic::AssertUnwindSafe(async {
        assert!(
            !mgr.goto(parent_doc, pos(&parent_source, "Middleware\\AuditRequests;"), GotoKind::Definition)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(source.matches("demo.kernel.parent").count(), 1);
        let position = pos(&source, "demo.kernel.parent");
        let inactive = mgr.goto(doc, position, GotoKind::Definition).await.unwrap();
        assert!(inactive.is_empty(), "client standard builder must retire inactive legacy aliases: {inactive:?}");
        let queued = standard.replace(
            "->create()",
            r"->withSingletons([HttpKernelContract::class=>\App\Runtime\SiteKernel::class])->create()",
        );
        assert_ne!(queued, standard);
        replace(&mgr, bootstrap_doc, 2, &standard, &queued);
        assert!(mgr.goto(doc, position, GotoKind::Definition).await.unwrap().is_empty());
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        assert!(mgr.goto(doc, position, GotoKind::Definition).await.unwrap().is_empty());
        replace(&mgr, bootstrap_doc, 3, &queued, &original);
        let active = mgr.goto(doc, position, GotoKind::Definition).await.unwrap();
        assert_eq!(active.len(), 1, "explicit binding must restore inherited aliases: {active:?}");
        assert_eq!(*active[0].path, parent);
        let mut refs = None;
        for _ in 0..80 {
            match mgr.references(doc, position, true, true, Default::default()).await {
                Ok(found) => {
                    refs = Some(found);
                    break;
                }
                Err(error) if error.kind == ide_core::error::ErrorKind::Cancelled => {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                Err(error) => panic!("restored Kernel references failed: {error}"),
            }
        }
        let refs = refs.expect("restored Kernel references must settle");
        assert!(
            refs.locations
                .iter()
                .any(|location| *location.path == parent && location.range.end.col - location.range.start.col == 18),
            "{refs:?}"
        );
        replace(&mgr, bootstrap_doc, 4, &original, &standard);
        assert!(mgr.goto(doc, position, GotoKind::Definition).await.unwrap().is_empty());
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    assert!(mgr.servers().iter().all(|server| server.pid.is_none()));
    for (path, text) in
        [(&path, &source), (&bootstrap, &original), (&parent, &parent_source), (&declaration, &class_source)]
    {
        assert_eq!(std::fs::read_to_string(path).unwrap(), *text);
    }
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

fn builder_variable_client(root: &std::path::Path, engine: &std::path::Path) -> Arc<LspManager> {
    let mgr = LspManager::new(
        root.to_path_buf(),
        Arc::new(LanguageRegistry::from_configs(vec![route_handler_language(engine)])),
        Arc::new(|_| {}),
    );
    let candidates = rename_candidates(root);
    mgr.set_candidates_hook(Box::new(move |query, cancel| ide_lsp::CandidateFiles {
        files: candidates
            .iter()
            .filter(|path| {
                !cancel.is_cancelled()
                    && path.starts_with(&query.root)
                    && !query.skip.iter().any(|skip| path.starts_with(skip))
                    && std::fs::read_to_string(path).unwrap().contains(&query.word)
            })
            .cloned()
            .collect(),
        limit_hit: false,
    }));
    mgr
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom169 and retained Laravel13.37 builder-variable fixtures"]
async fn variable_builder_client_keeps_real_parentheses_repeated_identity_and_checked_previews() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let engine = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("src-tauri/resources/phpantom/phpantom_lsp"));
    for (setting, fixture) in [
        ("OPENSTORM_TEST_VARIABLE_BUILDER_ROOT", "slice155-laravel-builder-variable"),
        ("OPENSTORM_TEST_PAREN_BUILDER_ROOT", "slice155-laravel-builder-parenthesized-queued"),
        ("OPENSTORM_TEST_REPEAT_BUILDER_ROOT", "slice155-laravel-builder-repeat-direct"),
    ] {
        let root = std::env::var_os(setting)
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("artifacts/checks").join(fixture))
            .canonicalize()
            .unwrap();
        let caller = root.join("routes/builder.php");
        let bootstrap = root.join("bootstrap/app.php");
        let declaration = root.join("app/Runtime/BuilderKernel.php");
        let target = root.join("app/Http/Middleware/BuilderAudit.php");
        let source = std::fs::read_to_string(&caller).unwrap();
        let initial = std::fs::read_to_string(&bootstrap).unwrap();
        let registration = std::fs::read_to_string(&declaration).unwrap();
        let class = std::fs::read_to_string(&target).unwrap();
        let php = std::env::var_os("OPENSTORM_TEST_PHP_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("artifacts/toolchains/php-8.4.23-static/php"));
        for path in [&caller, &bootstrap, &declaration, &target] {
            let lint = std::process::Command::new(&php).args(["-n", "-l"]).arg(path).output().unwrap();
            assert!(lint.status.success(), "{}: {}", path.display(), String::from_utf8_lossy(&lint.stderr));
        }
        let mgr = builder_variable_client(&root, &engine);
        let doc = DocId::next();
        let config = DocId::next();
        let class_doc = DocId::next();
        let declaration_doc = DocId::next();
        mgr.did_open(doc, &caller, &LanguageId::new("php"), 1, &source);
        mgr.did_open(config, &bootstrap, &LanguageId::new("php"), 1, &initial);
        mgr.did_open(class_doc, &target, &LanguageId::new("php"), 1, &class);
        mgr.did_open(declaration_doc, &declaration, &LanguageId::new("php"), 1, &registration);
        let outcome=std::panic::AssertUnwindSafe(async {
            assert!(!mgr.goto(doc,pos(&source,"BuilderAudit;"),GotoKind::Definition).await.unwrap().is_empty());
            let position=pos(&source,"builder.audit");
            let found=mgr.goto(doc,position,GotoKind::Definition).await.unwrap();
            assert_eq!(found.len(),1,"real variable builder client owner: {found:?}");
            assert_eq!(*found[0].path,declaration);
            assert_eq!(found[0].range.start,pos(&registration,"builder.audit"));
            let refs=mgr.references(doc,position,true,true,Default::default()).await.unwrap();
            assert!(refs.locations.iter().any(|location|*location.path==declaration && location.range.end.col-location.range.start.col==13),"{refs:?}");
            let overlay=format!("{class}// Unsaved builder middleware documentation.\n");
            replace(&mgr,class_doc,2,&class,&overlay);
            let rename=mgr.rename(class_doc,pos(&class,"BuilderAudit {}"),"ReviewedBuilderAudit".into(),Default::default()).await.unwrap();
            assert!(rename.incomplete.is_none(),"{rename:?}");
            assert!(rename.edit.files.iter().any(|edit|matches!(edit,ide_core::FileEdit::Edit {path,edits,..} if **path==declaration && edits.iter().any(|edit|edit.new_text.contains("ReviewedBuilderAudit")))),"{rename:?}");
            mgr.restart(mgr.servers()[0].id).await.unwrap();
            let replayed=mgr.rename(class_doc,pos(&class,"BuilderAudit {}"),"ReviewedBuilderAudit".into(),Default::default()).await.unwrap();
            assert!(replayed.edit.files.iter().any(|edit|matches!(edit,ide_core::FileEdit::Edit {path,edits,..} if **path==declaration && edits.iter().any(|edit|edit.new_text.contains("ReviewedBuilderAudit")))));
        }).catch_unwind().await;
        mgr.shutdown_all().await;
        assert!(mgr.servers().iter().all(|server| server.pid.is_none()));
        for (path, text) in
            [(&caller, &source), (&bootstrap, &initial), (&declaration, &registration), (&target, &class)]
        {
            assert_eq!(std::fs::read_to_string(path).unwrap(), *text);
        }
        assert!(!root.join("app/Http/Middleware/ReviewedBuilderAudit.php").exists());
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom169 and the retained Laravel13.37 builder-variable fixture"]
async fn variable_builder_client_keeps_mutation_refusals_and_restores_dirty_dependencies() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = std::env::var_os("OPENSTORM_TEST_VARIABLE_BUILDER_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("artifacts/checks/slice155-laravel-builder-variable"))
        .canonicalize()
        .unwrap();
    let engine = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("src-tauri/resources/phpantom/phpantom_lsp"));
    let caller = root.join("routes/builder.php");
    let bootstrap = root.join("bootstrap/app.php");
    let declaration = root.join("app/Runtime/BuilderKernel.php");
    let target = root.join("app/Http/Middleware/BuilderAudit.php");
    let source = std::fs::read_to_string(&caller).unwrap();
    let initial = std::fs::read_to_string(&bootstrap).unwrap();
    let registration = std::fs::read_to_string(&declaration).unwrap();
    let class = std::fs::read_to_string(&target).unwrap();
    let mgr = builder_variable_client(&root, &engine);
    let doc = DocId::next();
    let config = DocId::next();
    let class_doc = DocId::next();
    let declaration_doc = DocId::next();
    mgr.did_open(doc, &caller, &LanguageId::new("php"), 1, &source);
    mgr.did_open(config, &bootstrap, &LanguageId::new("php"), 1, &initial);
    mgr.did_open(class_doc, &target, &LanguageId::new("php"), 1, &class);
    mgr.did_open(declaration_doc, &declaration, &LanguageId::new("php"), 1, &registration);
    let outcome=std::panic::AssertUnwindSafe(async {
        let rename=mgr.rename(class_doc,pos(&class,"BuilderAudit {}"),"ReviewedBuilderAudit".into(),Default::default()).await.unwrap();
        assert!(rename.edit.files.iter().any(|edit|matches!(edit,ide_core::FileEdit::Edit {path,..} if **path==declaration)),"real variable builder complete plan: {rename:?}");
        assert_eq!(initial.matches("$alias=$builder;").count(),1);
        let changed=initial.replace("$alias=$builder;","$alias=$builder; $escaped=$builder; mutate($escaped);");
        replace(&mgr,config,2,&initial,&changed);
        let hover=mgr.hover(doc,pos(&source,"builder.audit"),Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains("incomplete"),"{hover:?}");
        let refused=mgr.rename(class_doc,pos(&class,"BuilderAudit {}"),"ReviewedBuilderAudit".into(),Default::default()).await;
        assert!(refused.as_ref().is_err_and(|error|error.message.contains("middleware")),"escaped builder cannot authorize a checked rename: {refused:?}");
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        let refused=mgr.rename(class_doc,pos(&class,"BuilderAudit {}"),"ReviewedBuilderAudit".into(),Default::default()).await;
        assert!(refused.as_ref().is_err_and(|error|error.message.contains("middleware")),"escaped builder refusal must survive restart: {refused:?}");
        replace(&mgr,config,3,&changed,&initial);
        let restored=mgr.rename(class_doc,pos(&class,"BuilderAudit {}"),"ReviewedBuilderAudit".into(),Default::default()).await.unwrap();
        assert!(restored.incomplete.is_none(),"{restored:?}");
        assert!(restored.edit.files.iter().any(|edit|matches!(edit,ide_core::FileEdit::Edit {path,edits,..} if **path==declaration && edits.iter().any(|edit|edit.new_text.contains("ReviewedBuilderAudit")))));
    }).catch_unwind().await;
    mgr.shutdown_all().await;
    assert!(mgr.servers().iter().all(|server| server.pid.is_none()));
    for (path, text) in [(&caller, &source), (&bootstrap, &initial), (&declaration, &registration), (&target, &class)] {
        assert_eq!(std::fs::read_to_string(path).unwrap(), *text);
    }
    assert!(!root.join("app/Http/Middleware/ReviewedBuilderAudit.php").exists());
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom170 and retained Laravel13.37 immediate-provider fixtures"]
async fn immediate_provider_client_keeps_real_method_map_order_dirty_ownership_and_restart() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let engine = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("src-tauri/resources/phpantom/phpantom_lsp"));
    for (setting, fixture) in [
        ("OPENSTORM_TEST_IMMEDIATE_PROVIDER_ROOT", "slice158-laravel-immediate-provider"),
        ("OPENSTORM_TEST_PROVIDER_MAPS_ROOT", "slice158-laravel-provider-properties"),
    ] {
        let root = std::env::var_os(setting)
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("artifacts/checks").join(fixture))
            .canonicalize()
            .unwrap();
        let caller = root.join("routes/builder.php");
        let provider = root.join("app/Providers/KernelProvider.php");
        let declaration = root.join("app/Runtime/BuilderKernel.php");
        let target = root.join("app/Http/Middleware/BuilderAudit.php");
        let source = std::fs::read_to_string(&caller).unwrap();
        let initial = std::fs::read_to_string(&provider).unwrap();
        let registration = std::fs::read_to_string(&declaration).unwrap();
        let class = std::fs::read_to_string(&target).unwrap();
        let mgr = builder_variable_client(&root, &engine);
        let doc = DocId::next();
        let config = DocId::next();
        let class_doc = DocId::next();
        mgr.did_open(doc, &caller, &LanguageId::new("php"), 1, &source);
        mgr.did_open(config, &provider, &LanguageId::new("php"), 1, &initial);
        mgr.did_open(class_doc, &target, &LanguageId::new("php"), 1, &class);
        let outcome=std::panic::AssertUnwindSafe(async {
            assert!(!mgr.goto(doc,pos(&source,"BuilderAudit;"),GotoKind::Definition).await.unwrap().is_empty());
            let position=pos(&source,"builder.audit");
            let found=mgr.goto(doc,position,GotoKind::Definition).await.unwrap();
            assert_eq!(found.len(),1,"real immediate provider client owner: {found:?}");
            assert_eq!(*found[0].path,declaration);
            assert_eq!(found[0].range.start,pos(&registration,"builder.audit"));
            let refs=mgr.references(doc,position,true,true,Default::default()).await.unwrap();
            assert!(refs.incomplete.is_none(),"{refs:?}");
            assert!(refs.locations.iter().any(|location|*location.path==declaration && location.range.end.col-location.range.start.col==13),"{refs:?}");
            let overlay=format!("{class}// Unsaved provider middleware documentation.\n");
            replace(&mgr,class_doc,2,&class,&overlay);
            let rename=mgr.rename(class_doc,pos(&class,"BuilderAudit {}"),"ReviewedProviderAudit".into(),Default::default()).await.unwrap();
            assert!(rename.incomplete.is_none(),"{rename:?}");
            assert!(rename.edit.files.iter().any(|edit|matches!(edit,ide_core::FileEdit::Edit{path,edits,..} if **path==declaration && edits.iter().any(|edit|edit.new_text.contains("ReviewedProviderAudit")))),"{rename:?}");
            let changed=initial.replace("App\\Runtime\\BuilderKernel as RuntimeKernel","Illuminate\\Foundation\\Http\\Kernel as RuntimeKernel");
            assert_ne!(changed,initial);
            replace(&mgr,config,2,&initial,&changed);
            assert!(mgr.goto(doc,position,GotoKind::Definition).await.unwrap().is_empty(),"dirty provider must retire the initial owner");
            mgr.restart(mgr.servers()[0].id).await.unwrap();
            assert!(mgr.goto(doc,position,GotoKind::Definition).await.unwrap().is_empty(),"restart must replay the dirty provider");
            replace(&mgr,config,3,&changed,&initial);
            if fixture.ends_with("properties") {
                let changed=initial.replace("public $singletons=[HttpKernel::class=>RuntimeKernel::class]",r"public $singletons=[HttpKernel::class=>\Illuminate\Foundation\Http\Kernel::class]");
                assert_ne!(changed,initial);
                replace(&mgr,config,4,&initial,&changed);
                assert!(mgr.goto(doc,position,GotoKind::Definition).await.unwrap().is_empty(),"singleton map must run after register and bindings");
                replace(&mgr,config,5,&changed,&initial);
            }
            let restored=mgr.rename(class_doc,pos(&class,"BuilderAudit {}"),"ReviewedProviderAudit".into(),Default::default()).await.unwrap();
            assert!(restored.incomplete.is_none(),"{restored:?}");
            assert!(restored.edit.files.iter().any(|edit|matches!(edit,ide_core::FileEdit::Edit{path,..} if **path==declaration)),"{restored:?}");
        }).catch_unwind().await;
        mgr.shutdown_all().await;
        assert!(mgr.servers().iter().all(|server| server.pid.is_none()));
        for (path, text) in
            [(&caller, &source), (&provider, &initial), (&declaration, &registration), (&target, &class)]
        {
            assert_eq!(std::fs::read_to_string(path).unwrap(), *text);
        }
        assert!(!root.join("app/Http/Middleware/ReviewedProviderAudit.php").exists());
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom170 and retained real Laravel13.37 provider lifecycle controls"]
async fn immediate_provider_client_keeps_lifecycle_refusals_across_restart() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let engine = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("src-tauri/resources/phpantom/phpantom_lsp"));
    for (setting, fixture) in [
        ("OPENSTORM_TEST_PROVIDER_EARLY_BOOT_ROOT", "slice158-laravel-provider-external-boot"),
        ("OPENSTORM_TEST_PROVIDER_DEFAULT_ROOT", "slice158-laravel-provider-default-phase"),
        ("OPENSTORM_TEST_PROVIDER_LOAD_ROOT", "slice158-laravel-provider-load-phase"),
    ] {
        let root = std::env::var_os(setting)
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("artifacts/checks").join(fixture))
            .canonicalize()
            .unwrap();
        let caller = root.join("routes/builder.php");
        let target = root.join("app/Http/Middleware/BuilderAudit.php");
        let source = std::fs::read_to_string(&caller).unwrap();
        let class = std::fs::read_to_string(&target).unwrap();
        let mgr = builder_variable_client(&root, &engine);
        let doc = DocId::next();
        let class_doc = DocId::next();
        mgr.did_open(doc, &caller, &LanguageId::new("php"), 1, &source);
        mgr.did_open(class_doc, &target, &LanguageId::new("php"), 1, &class);
        let outcome = std::panic::AssertUnwindSafe(async {
            assert!(!mgr.goto(doc, pos(&source, "BuilderAudit;"), GotoKind::Definition).await.unwrap().is_empty());
            for restarted in [false, true] {
                if restarted {
                    mgr.restart(mgr.servers()[0].id).await.unwrap();
                }
                let hover = mgr.hover(doc, pos(&source, "builder.audit"), Default::default()).await.unwrap().unwrap();
                assert!(hover.contents.value.contains("incomplete"), "{fixture}: {hover:?}");
                let refused = mgr
                    .rename(
                        class_doc,
                        pos(&class, "BuilderAudit {}"),
                        "ReviewedProviderAudit".into(),
                        Default::default(),
                    )
                    .await;
                assert!(
                    refused.as_ref().is_err_and(|error| error.message.contains("middleware")),
                    "real lifecycle refusal must survive client restart: {fixture}: {refused:?}"
                );
            }
        })
        .catch_unwind()
        .await;
        mgr.shutdown_all().await;
        assert!(mgr.servers().iter().all(|server| server.pid.is_none()));
        assert_eq!(std::fs::read_to_string(&caller).unwrap(), source);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), class);
        assert!(!root.join("app/Http/Middleware/ReviewedProviderAudit.php").exists());
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
    }
}
