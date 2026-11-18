//! Legacy Kernel aliases through managed documents and reviewed host edits.
use super::*;

const KERNEL: &str = r#"<?php
namespace App\Http;
use App\Http\KernelBase as BaseKernel;
use App\Http\Middleware\Audit as AuditAlias;
class Kernel extends BaseKernel {
    protected $middlewareAliases = [/* 😀 */ "au\x64ited" => AuditAlias::class];
}
"#;
const BASE: &str = "<?php namespace App\\Http; class KernelBase extends \\Illuminate\\Foundation\\Http\\Kernel {}\n";
const CLASS: &str = "<?php namespace App\\Http\\Middleware; class Audit {}\n";
const CALLER: &str = r#"<?php
use Illuminate\Support\Facades\Route;
new \App\Http\Middleware\Audit;
/* 😀 */ Route::middleware("au\x64ited:strict");
class Foreign {public function middleware($value){} public function run() {$this->middleware('audited');}}
"#;

fn fixture(kernel: &str) -> Fixture {
    fixture_with(kernel, &[])
}

fn fixture_with(kernel: &str, extra: &[(String, String)]) -> Fixture {
    let mut files = vec![
            ("composer.json".into(), r#"{"require":{"laravel/framework":"^10.0"},"autoload":{"psr-4":{"App\\":"app/","Illuminate\\":"framework/"}}}"#.into()),
            ("app/Http/Kernel.php".into(), kernel.into()),
            ("app/Http/KernelBase.php".into(), BASE.into()),
            ("app/Http/Middleware/Audit.php".into(), CLASS.into()),
            ("app/Http/Middleware/Trace.php".into(), CLASS.replace("Audit", "Trace")),
            ("framework/Foundation/Http/Kernel.php".into(), "<?php namespace Illuminate\\Foundation\\Http; class Kernel {}\n".into()),
    ];
    files.extend_from_slice(extra);
    Fixture::new_at(CALLER, &files, "routes/web.php")
}

fn cursor(text: &str, needle: &str) -> Pos {
    at(text, text.find(needle).unwrap())
}

async fn definitions(f: &Fixture) -> Vec<ide_core::Location> {
    ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, cursor(CALLER, r"64ited"), GotoKind::Definition).await.unwrap()
}

async fn hover(f: &Fixture, expected: &str, relative: &str) {
    let result = ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, cursor(CALLER, r"64ited"))
        .await
        .unwrap()
        .expect("Kernel middleware hover");
    assert!(result.contents.value.contains(expected), "{result:?}");
    assert!(result.contents.value.contains(relative), "{result:?}");
    assert!(!result.contents.value.contains("bootstrap/app.php"), "{result:?}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom164 and owned PHP syntax-check executable"]
async fn kernel_alias_navigation_tracks_dirty_ancestry_ranges_and_restart() {
    for property in ["middlewareAliases", "routeMiddleware"] {
        let original = KERNEL.replace("middlewareAliases", property);
        navigation_case(
            fixture(&original),
            &original,
            "app/Http/Kernel.php",
            "app/Http/KernelBase.php",
            BASE,
            "<?php namespace App\\Http; class KernelBase {}\n",
        )
        .await;
    }
}

async fn navigation_case(
    f: Fixture,
    original: &str,
    relative: &str,
    ancestry_relative: &str,
    ancestry_original: &str,
    removed: &str,
) {
    let outcome = std::panic::AssertUnwindSafe(async {
        let kernel = f.root.join(relative);
        let parent = f.root.join(ancestry_relative);
        let ready =
            ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, cursor(CALLER, "Audit;"), GotoKind::Definition)
                .await
                .unwrap();
        assert!(!ready.is_empty(), "ordinary PHP readiness");
        let found = definitions(&f).await;
        assert_eq!(found.len(), 1, "managed Kernel alias definition: {found:?}");
        assert_eq!(*found[0].path, kernel);
        assert_eq!(found[0].range.start, cursor(original, r"au\x64ited"));
        assert_eq!(found[0].range.end, found[0].range.start);
        hover(&f, "App\\Http\\Middleware\\Audit", relative).await;
        let refs = ide_host::ops::lsp::lsp_references(
            &f.host,
            "main",
            RequestId::next(),
            f.doc,
            cursor(CALLER, r"64ited"),
            true,
            true,
            true,
            ide_host::ops::Channel::new(|_| {}),
        )
        .await
        .unwrap();
        assert!(refs.incomplete.is_none(), "{refs:?}");
        assert_eq!(refs.locations.len(), 2, "foreign middleware must stay excluded: {refs:?}");
        for (path, text) in [(&kernel, original), (&f.root.join("routes/web.php"), CALLER)] {
            let location = refs.locations.iter().find(|location| *location.path == *path).unwrap();
            assert_eq!(location.range.start, cursor(text, r"au\x64ited"));
            assert_eq!(location.range.end.col - location.range.start.col, 10);
        }
        let doc = f.project.documents.open(&kernel).unwrap();
        for (id, text, needle) in [(f.doc, CALLER, r"64ited"), (doc.id, original, r"64ited")] {
            let position = cursor(text, needle);
            assert!(f.project.lsp.prepare_rename_details(id, position).await.unwrap().is_none());
            let refused =
                ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), id, position, "reviewed".into())
                    .await;
            assert!(
                refused.as_ref().is_err_and(|error| error.message.contains("Middleware alias rename")),
                "{refused:?}"
            );
        }
        let changed = original.replace("Middleware\\Audit as", "Middleware\\Trace as");
        f.project.documents.replace_all(doc.id, &changed).unwrap();
        hover(&f, "App\\Http\\Middleware\\Trace", relative).await;
        let parent_doc = f.project.documents.open(&parent).unwrap();
        f.project.documents.replace_all(parent_doc.id, removed).unwrap();
        assert!(definitions(&f).await.is_empty(), "dirty parent removes Kernel ancestry");
        let restored = format!("{ancestry_original}// Unsaved Kernel parent documentation.\n");
        f.project.documents.replace_all(parent_doc.id, &restored).unwrap();
        assert_eq!(definitions(&f).await.len(), 1);
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        hover(&f, "App\\Http\\Middleware\\Trace", relative).await;
        let renamed = changed.replace(r"au\x64ited", "different");
        f.project.documents.replace_all(doc.id, &renamed).unwrap();
        assert!(definitions(&f).await.is_empty(), "dirty alias removes the old key");
        assert_eq!(std::fs::read_to_string(&kernel).unwrap(), original);
        assert_eq!(std::fs::read_to_string(&parent).unwrap(), ancestry_original);
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), CALLER);
        assert_eq!(f.project.documents.snapshot(parent_doc.id).unwrap().text, restored);
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom164 and owned PHP syntax-check executable"]
async fn kernel_alias_checked_rename_preserves_conflicts_dirty_buffers_and_history() {
    checked_rename_case(fixture(KERNEL), KERNEL, "app/Http/Kernel.php").await;
}

async fn checked_rename_case(f: Fixture, original: &str, relative: &str) {
    let outcome = std::panic::AssertUnwindSafe(async {
        let old = f.root.join("app/Http/Middleware/Audit.php");
        let new = f.root.join("app/Http/Middleware/Auditor.php");
        let kernel = f.root.join(relative);
        let class = f.project.documents.open(&old).unwrap();
        let registration = f.project.documents.open(&kernel).unwrap();
        let overlay = format!("{CLASS}// Unsaved middleware documentation.\n");
        f.project.documents.replace_all(class.id, &overlay).unwrap();
        let position = cursor(CLASS, "Audit {}");
        if relative == "app/Http/AliasInner.php" {
            let parent = f.root.join("app/Http/KernelBase.php");
            let parent_doc = f.project.documents.open(&parent).unwrap();
            let overlapping =
                original.replace("trait AliasInner", "class KernelBase extends \\Illuminate\\Foundation\\Http\\Kernel");
            assert_ne!(overlapping, original);
            Fixture::lint(&overlapping);
            f.project.documents.replace_all(parent_doc.id, &overlapping).unwrap();
            let refused = ide_host::ops::lsp::lsp_rename(
                &f.host,
                "main",
                RequestId::next(),
                class.id,
                position,
                "Auditor".into(),
            )
            .await;
            assert!(
                refused.as_ref().is_err_and(|error| error.message.contains("middleware")),
                "overlapping trait property plan: {refused:?}"
            );
            assert_eq!(f.project.documents.snapshot(class.id).unwrap().text, overlay);
            assert_eq!(std::fs::read_to_string(&parent).unwrap(), BASE);
            assert!(!new.exists());
            f.project.documents.replace_all(parent_doc.id, BASE).unwrap();
        }
        for value in [r"['audited'=>'App\Http\Middleware\Audit']", "ALIASES", "['audited'=>AliasTargets::AUDIT]"] {
            let changed = original.replace(r#"[/* 😀 */ "au\x64ited" => AuditAlias::class]"#, value);
            assert_ne!(changed, original);
            Fixture::lint(&changed);
            f.project.documents.replace_all(registration.id, &changed).unwrap();
            for name in ["Auditor", "App\\Moved\\Audit"] {
                let refused =
                    ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, position, name.into())
                        .await;
                assert!(
                    refused.as_ref().is_err_and(|error| error.message.contains("middleware")),
                    "unsafe Kernel class plan: {refused:?}"
                );
            }
            let refused = ide_host::ops::lsp::lsp_rename(
                &f.host,
                "main",
                RequestId::next(),
                class.id,
                cursor(CLASS, "Http\\Middleware"),
                "Changed".into(),
            )
            .await;
            assert!(
                refused.as_ref().is_err_and(|error| error.message.contains("middleware")),
                "unsafe Kernel namespace plan: {refused:?}"
            );
            assert_eq!(f.project.documents.snapshot(class.id).unwrap().text, overlay);
            assert_eq!(f.project.documents.snapshot(registration.id).unwrap().text, changed);
            assert_eq!(std::fs::read_to_string(&old).unwrap(), CLASS);
            assert_eq!(std::fs::read_to_string(&kernel).unwrap(), original);
            assert!(!new.exists());
        }
        f.project.documents.replace_all(registration.id, original).unwrap();
        let stale =
            ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, position, "Auditor".into())
                .await
                .unwrap()
                .edit;
        f.project.documents.replace_all(registration.id, &format!("{original}// Later buffer edit.\n")).unwrap();
        assert!(ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", stale).await.is_err());
        assert_eq!(f.project.documents.snapshot(class.id).unwrap().text, overlay);
        assert_eq!(std::fs::read_to_string(&old).unwrap(), CLASS);
        assert!(!new.exists());
        f.project.documents.replace_all(registration.id, original).unwrap();
        let stale =
            ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, position, "Auditor".into())
                .await
                .unwrap()
                .edit;
        let external = format!("{original}// External Kernel change.\n");
        std::fs::write(&kernel, &external).unwrap();
        assert!(ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", stale).await.is_err());
        assert_eq!(std::fs::read_to_string(&kernel).unwrap(), external);
        assert_eq!(f.project.documents.snapshot(class.id).unwrap().text, overlay);
        assert_eq!(std::fs::read_to_string(&old).unwrap(), CLASS);
        assert!(!new.exists());
        std::fs::write(&kernel, original).unwrap();
        let plan =
            ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, position, "Auditor".into())
                .await
                .unwrap()
                .edit;
        assert!(
            plan.files
                .iter()
                .any(|edit| matches!(edit, ide_core::FileEdit::Rename {from,to} if **from==old && **to==new)),
            "{plan:?}"
        );
        ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.unwrap();
        assert!(!old.exists());
        assert_eq!(f.project.documents.by_path(&new), Some(class.id));
        let renamed = f.project.documents.snapshot(class.id).unwrap();
        assert!(renamed.text.contains("class Auditor") && renamed.text.contains("Unsaved middleware documentation"));
        assert!(!renamed.dirty);
        assert_eq!(std::fs::read_to_string(&new).unwrap(), renamed.text);
        let updated = f.project.documents.snapshot(registration.id).unwrap();
        assert!(updated.text.contains("Middleware\\Auditor as AuditAlias"));
        assert_eq!(std::fs::read_to_string(&kernel).unwrap(), updated.text);
        let caller = f.project.documents.snapshot(f.doc).unwrap();
        assert!(caller.text.contains("new \\App\\Http\\Middleware\\Auditor;"));
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), caller.text);
        for text in [&renamed.text, &updated.text, &caller.text] {
            Fixture::lint(text);
        }
        let group = f
            .events
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find_map(|event| match event {
                ide_core::IdeEvent::DocPatched { history: Some(history), .. } => Some(history.group.clone()),
                _ => None,
            })
            .expect("Kernel rename must have shared history");
        for (redo, path, name) in [(false, &old, "Audit"), (true, &new, "Auditor")] {
            ide_host::ops::lsp::restore_reviewed_edit(&f.host, "main", group.clone(), redo).await.unwrap();
            assert_eq!(f.project.documents.by_path(path), Some(class.id));
            let restored = f.project.documents.snapshot(class.id).unwrap();
            assert!(
                restored.text.contains(&format!("class {name} {{"))
                    && restored.text.contains("Unsaved middleware documentation")
            );
            assert_eq!(restored.dirty, !redo);
            assert_eq!(std::fs::read_to_string(&kernel).unwrap(), if redo { updated.text.as_str() } else { original });
            assert_eq!(
                std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(),
                if redo { caller.text.as_str() } else { CALLER }
            );
        }
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

fn inherited_fixture(is_trait: bool, property: &str) -> (Fixture, String, String) {
    let root = if is_trait {
        "<?php namespace App\\Http; class Kernel extends KernelBase {use AliasOuter;}\n"
    } else {
        "<?php namespace App\\Http; class Kernel extends KernelBase {}\n"
    };
    let declaration = KERNEL
        .replace("use App\\Http\\KernelBase as BaseKernel;\n", "")
        .replace(
            "class Kernel extends BaseKernel",
            if is_trait {
                "trait AliasInner"
            } else {
                "class KernelBase extends \\Illuminate\\Foundation\\Http\\Kernel"
            },
        )
        .replace("middlewareAliases", property);
    let relative = if is_trait { "app/Http/AliasInner.php" } else { "app/Http/KernelBase.php" };
    let mut extra = vec![(relative.into(), declaration.clone())];
    if is_trait {
        extra.push((
            "app/Http/AliasOuter.php".into(),
            "<?php namespace App\\Http; trait AliasOuter {use AliasInner;}\n".into(),
        ));
    }
    let f = fixture_with(root, &extra);
    (f, declaration, root.into())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom165 and owned PHP syntax-check executable"]
async fn inherited_kernel_alias_navigation_keeps_declaration_ownership_and_dirty_replay() {
    for is_trait in [false, true] {
        for property in ["middlewareAliases", "routeMiddleware"] {
            let (f, declaration, root) = inherited_fixture(is_trait, property);
            let relative = if is_trait { "app/Http/AliasInner.php" } else { "app/Http/KernelBase.php" };
            let removed = if is_trait {
                root.replace("use AliasOuter;", "")
            } else {
                "<?php namespace App\\Http; class Kernel {}\n".into()
            };
            navigation_case(f, &declaration, relative, "app/Http/Kernel.php", &root, &removed).await;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom165 and owned PHP syntax-check executable"]
async fn inherited_kernel_alias_checked_plans_preserve_conflicts_and_reviewed_history() {
    for is_trait in [false, true] {
        let (f, declaration, _) = inherited_fixture(is_trait, "middlewareAliases");
        let relative = if is_trait { "app/Http/AliasInner.php" } else { "app/Http/KernelBase.php" };
        checked_rename_case(f, &declaration, relative).await;
    }
}

const BOUND_BOOTSTRAP: &str = r"<?php
use Illuminate\Contracts\Http\Kernel as HttpKernel;
use App\Runtime\SiteKernel as ProjectKernel;
$app = new \Illuminate\Foundation\Application;
$app->singleton(HttpKernel::class, ProjectKernel::class);
return $app;
";

fn bound_fixture(property: &str) -> (Fixture, String) {
    let declaration = KERNEL
        .replacen("namespace App\\Http;", "namespace App\\Runtime;", 1)
        .replace("class Kernel extends", "class SiteKernel extends")
        .replace("middlewareAliases", property);
    let conventional = KERNEL.replace("Middleware\\Audit as", "Middleware\\Trace as");
    let next = declaration
        .replace("class SiteKernel extends", "class NextKernel extends")
        .replace("Middleware\\Audit as", "Middleware\\Trace as");
    let f = fixture_with(&conventional, &[
        ("bootstrap/app.php".into(), BOUND_BOOTSTRAP.into()),
        ("app/Runtime/SiteKernel.php".into(), declaration.clone()),
        ("app/Runtime/NextKernel.php".into(), next),
        ("framework/Contracts/Http/Kernel.php".into(), "<?php namespace Illuminate\\Contracts\\Http; interface Kernel {}\n".into()),
        ("framework/Foundation/Application.php".into(), "<?php namespace Illuminate\\Foundation; class Application {public function singleton($abstract,$concrete=null): void {}}\n".into()),
    ]);
    (f, declaration)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom167 and owned PHP syntax-check executable"]
async fn bound_kernel_navigation_preserves_managed_ancestry_and_ranges() {
    for property in ["middlewareAliases", "routeMiddleware"] {
        let (f, declaration) = bound_fixture(property);
        navigation_case(
            f,
            &declaration,
            "app/Runtime/SiteKernel.php",
            "app/Http/KernelBase.php",
            BASE,
            "<?php namespace App\\Http; class KernelBase {}\n",
        )
        .await;
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom167 and owned PHP syntax-check executable"]
async fn bound_kernel_checked_plans_preserve_external_conflicts_and_reviewed_history() {
    let (f, declaration) = bound_fixture("middlewareAliases");
    checked_rename_case(f, &declaration, "app/Runtime/SiteKernel.php").await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom167 and owned PHP syntax-check executable"]
async fn bound_kernel_bootstrap_changes_retire_plans_and_replay_dirty_selection() {
    let (f, _) = bound_fixture("middlewareAliases");
    let outcome = std::panic::AssertUnwindSafe(async {
        let bootstrap = f.root.join("bootstrap/app.php");
        let bootstrap_doc = f.project.documents.open(&bootstrap).unwrap();
        let class_path = f.root.join("app/Http/Middleware/Audit.php");
        let class = f.project.documents.open(&class_path).unwrap();
        let position = cursor(CLASS, "Audit {}");
        hover(&f, "App\\Http\\Middleware\\Audit", "app/Runtime/SiteKernel.php").await;
        let plan =
            ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, position, "Auditor".into())
                .await
                .unwrap()
                .edit;
        let changed = BOUND_BOOTSTRAP.replace("SiteKernel as", "NextKernel as");
        assert_eq!(changed.len(), BOUND_BOOTSTRAP.len());
        f.project.documents.replace_all(bootstrap_doc.id, &changed).unwrap();
        assert!(
            ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.is_err(),
            "changed bootstrap must retire a reviewed Kernel plan"
        );
        let found = definitions(&f).await;
        assert_eq!(found.len(), 1, "bound Kernel must follow dirty bootstrap: {found:?}");
        assert_eq!(*found[0].path, f.root.join("app/Runtime/NextKernel.php"));
        hover(&f, "App\\Http\\Middleware\\Trace", "app/Runtime/NextKernel.php").await;
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        hover(&f, "App\\Http\\Middleware\\Trace", "app/Runtime/NextKernel.php").await;
        f.project.documents.replace_all(bootstrap_doc.id, BOUND_BOOTSTRAP).unwrap();
        assert_eq!(*definitions(&f).await[0].path, f.root.join("app/Runtime/SiteKernel.php"));
        assert_eq!(std::fs::read_to_string(&bootstrap).unwrap(), BOUND_BOOTSTRAP);
        assert_eq!(std::fs::read_to_string(&class_path).unwrap(), CLASS);
        assert!(!f.root.join("app/Http/Middleware/Auditor.php").exists());
        assert!(f.project.documents.snapshot(bootstrap_doc.id).unwrap().text == BOUND_BOOTSTRAP);
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), CALLER);
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}
