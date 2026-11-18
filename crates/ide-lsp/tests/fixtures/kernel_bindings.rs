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
