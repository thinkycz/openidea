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
            let overlapping = original
                .replace("trait AliasInner", "class KernelBase extends \\Illuminate\\Foundation\\Http\\Kernel")
                .replace("Middleware\\Audit as", "Middleware\\Trace as");
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
                "incompatible trait property plan: {refused:?}"
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

const BUILDER_APP: &str = r"<?php namespace Illuminate\Foundation;
class Application {
    protected static string $applicationBuilder = Configuration\ApplicationBuilder::class;
    public static function configure(): Configuration\ApplicationBuilder {return (new static::$applicationBuilder(new static))->withKernels();}
    public function singleton($abstract,$concrete=null): void {}
}
";
const BUILDER: &str = r"<?php namespace Illuminate\Foundation\Configuration;
class ApplicationBuilder {
    public function __construct(protected \Illuminate\Foundation\Application $app) {}
    public function withKernels(): static {$this->app->singleton(\Illuminate\Contracts\Http\Kernel::class,\Illuminate\Foundation\Http\Kernel::class);return $this;}
    public function withSingletons(array $singletons): static {return $this;}
    public function create(): \Illuminate\Foundation\Application {return $this->app;}
}
";
const BUILDER_BOOTSTRAP: &str = r"<?php $app=\Illuminate\Foundation\Application::configure()->create();return $app;";
fn builder_fixture() -> (Fixture, String) {
    builder_fixture_with(BUILDER_BOOTSTRAP)
}
fn builder_fixture_with(bootstrap: &str) -> (Fixture, String) {
    let framework = KERNEL
        .replace("namespace App\\Http;", "namespace Illuminate\\Foundation\\Http;")
        .replace("use App\\Http\\KernelBase as BaseKernel;\n", "")
        .replace("class Kernel extends BaseKernel", "class Kernel");
    let conventional = KERNEL.replace("Middleware\\Audit as", "Middleware\\Trace as");
    let f = fixture_with(
        &conventional,
        &[
            ("composer.json".into(), r#"{"require":{"laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/","Illuminate\\":"framework/"}}}"#.into()),
            ("bootstrap/app.php".into(), bootstrap.into()),
            ("framework/Foundation/Application.php".into(), BUILDER_APP.into()),
            ("framework/Foundation/Configuration/ApplicationBuilder.php".into(), BUILDER.into()),
            ("framework/Foundation/Http/Kernel.php".into(), framework.clone()),
        ],
    );
    (f, framework)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom168 and owned PHP syntax-check executable"]
async fn builder_kernel_navigation_retires_dirty_dependency_plans_and_replays_restart() {
    let (f, framework) = builder_fixture();
    let outcome = std::panic::AssertUnwindSafe(async {
        let path = f.root.join("framework/Foundation/Http/Kernel.php");
        let found = definitions(&f).await;
        assert_eq!(found.len(), 1, "managed standard-builder Kernel: {found:?}");
        assert_eq!(*found[0].path, path);
        assert_eq!(found[0].range.start, cursor(&framework, r"au\x64ited"));
        hover(&f, "App\\Http\\Middleware\\Audit", "framework/Foundation/Http/Kernel.php").await;
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
        assert_eq!(refs.locations.len(), 2, "{refs:?}");
        assert!(
            refs.locations
                .iter()
                .all(|location| *location.path == path || *location.path == f.root.join("routes/web.php"))
        );
        let class = f.project.documents.open(&f.root.join("app/Http/Middleware/Audit.php")).unwrap();
        let plan = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            class.id,
            cursor(CLASS, "Audit {}"),
            "Auditor".into(),
        )
        .await
        .unwrap()
        .edit;
        let builder = f.root.join("framework/Foundation/Configuration/ApplicationBuilder.php");
        let doc = f.project.documents.open(&builder).unwrap();
        let changed = BUILDER.replace(r"\Illuminate\Foundation\Http\Kernel::class", r"\App\Http\Kernel::class");
        f.project.documents.replace_all(doc.id, &changed).unwrap();
        assert!(
            ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.is_err(),
            "dirty builder registration must retire a Kernel plan"
        );
        assert_eq!(*definitions(&f).await[0].path, f.root.join("app/Http/Kernel.php"));
        hover(&f, "App\\Http\\Middleware\\Trace", "app/Http/Kernel.php").await;
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        hover(&f, "App\\Http\\Middleware\\Trace", "app/Http/Kernel.php").await;
        f.project.documents.replace_all(doc.id, BUILDER).unwrap();
        assert_eq!(*definitions(&f).await[0].path, path);
        assert_eq!(std::fs::read_to_string(builder).unwrap(), BUILDER);
        assert_eq!(std::fs::read_to_string(path).unwrap(), framework);
        assert_eq!(std::fs::read_to_string(f.root.join("app/Http/Middleware/Audit.php")).unwrap(), CLASS);
        assert!(!f.root.join("app/Http/Middleware/Auditor.php").exists());
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom168 and owned PHP syntax-check executable"]
async fn builder_kernel_checked_edits_keep_conflict_guards_and_shared_history() {
    let (f, framework) = builder_fixture();
    checked_rename_case(f, &framework, "framework/Foundation/Http/Kernel.php").await;
}

const VARIABLE_BOOTSTRAP: &str = r"<?php
$builder=\Illuminate\Foundation\Application::configure();
$alias=$builder->withSingletons([]);
$app=($alias)->create();
return $app;
";
const REPEATED_BOOTSTRAP: &str = r"<?php
$builder=\Illuminate\Foundation\Application::configure();
$app=($builder->withSingletons([]))->create();
$app->singleton(\Illuminate\Contracts\Http\Kernel::class,\App\Http\Kernel::class);
return $builder->create();
";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom169 and owned PHP syntax-check executable"]
async fn variable_builder_navigation_preserves_alias_identity_dirty_ranges_and_restart() {
    let (f, framework) = builder_fixture_with(VARIABLE_BOOTSTRAP);
    let outcome = std::panic::AssertUnwindSafe(async {
        let path = f.root.join("framework/Foundation/Http/Kernel.php");
        let found = definitions(&f).await;
        assert_eq!(found.len(), 1, "managed variable builder declaration: {found:?}");
        assert_eq!(*found[0].path, path);
        assert_eq!(found[0].range.start, cursor(&framework, r"au\x64ited"));
        hover(&f, "App\\Http\\Middleware\\Audit", "framework/Foundation/Http/Kernel.php").await;
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
        assert_eq!(refs.locations.len(), 2, "{refs:?}");
        assert!(refs.incomplete.is_none(), "{refs:?}");
        assert!(
            refs.locations
                .iter()
                .all(|location| *location.path == path || *location.path == f.root.join("routes/web.php"))
        );
        assert!(refs.locations.iter().all(|location| location.range.end.col - location.range.start.col == 10));
        let bootstrap = f.root.join("bootstrap/app.php");
        let doc = f.project.documents.open(&bootstrap).unwrap();
        let unknown = VARIABLE_BOOTSTRAP.replace("$alias=$builder->withSingletons([]);", r"$alias=new \stdClass;");
        Fixture::lint(&unknown);
        f.project.documents.replace_all(doc.id, &unknown).unwrap();
        assert!(definitions(&f).await.is_empty(), "dirty variable alias must retire its declaration");
        let restored = VARIABLE_BOOTSTRAP
            .replace("$app=($alias)->create();\nreturn $app;", "return (($alias->withSingletons([])))->create();");
        assert_ne!(restored, VARIABLE_BOOTSTRAP);
        Fixture::lint(&restored);
        f.project.documents.replace_all(doc.id, &restored).unwrap();
        assert_eq!(*definitions(&f).await[0].path, path);
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        hover(&f, "App\\Http\\Middleware\\Audit", "framework/Foundation/Http/Kernel.php").await;
        assert_eq!(f.project.documents.snapshot(doc.id).unwrap().text, restored);
        assert_eq!(std::fs::read_to_string(bootstrap).unwrap(), VARIABLE_BOOTSTRAP);
        assert_eq!(std::fs::read_to_string(path).unwrap(), framework);
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom169 and owned PHP syntax-check executable"]
async fn variable_builder_checked_edits_preserve_conflicts_dirty_buffers_and_shared_history() {
    let (f, framework) = builder_fixture_with(VARIABLE_BOOTSTRAP);
    checked_rename_case(f, &framework, "framework/Foundation/Http/Kernel.php").await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom169 and owned PHP syntax-check executable"]
async fn variable_builder_repeated_creation_keeps_owner_and_retires_off_target_plans() {
    let (f, _) = builder_fixture_with(REPEATED_BOOTSTRAP);
    let outcome = std::panic::AssertUnwindSafe(async {
        let expected = f.root.join("app/Http/Kernel.php");
        let found = definitions(&f).await;
        assert_eq!(found.len(), 1, "managed repeated builder application owner: {found:?}");
        assert_eq!(*found[0].path, expected);
        hover(&f, "App\\Http\\Middleware\\Trace", "app/Http/Kernel.php").await;
        let path = f.root.join("app/Http/Middleware/Trace.php");
        let source = CLASS.replace("Audit", "Trace");
        let doc = f.project.documents.open(&path).unwrap();
        let plan = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            doc.id,
            cursor(&source, "Trace {}"),
            "Traced".into(),
        )
        .await
        .unwrap()
        .edit;
        assert!(
            plan.files.iter().any(|edit| matches!(edit,ide_core::FileEdit::Edit {path,..} if **path==expected)),
            "{plan:?}"
        );
        let bootstrap = f.root.join("bootstrap/app.php");
        let config = f.project.documents.open(&bootstrap).unwrap();
        let changed = REPEATED_BOOTSTRAP
            .replace("return $builder->create();", "$builder->withKernels(); return $builder->create();");
        Fixture::lint(&changed);
        f.project.documents.replace_all(config.id, &changed).unwrap();
        assert!(
            ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.is_err(),
            "variable builder dependency must retire a reviewed plan"
        );
        let refused = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            doc.id,
            cursor(&source, "Trace {}"),
            "Traced".into(),
        )
        .await;
        assert!(
            refused.as_ref().is_err_and(|error| error.message.contains("middleware")),
            "post-create mutation must remain incomplete: {refused:?}"
        );
        let restored =
            REPEATED_BOOTSTRAP.replace("return $builder->create();", "$same=$builder->create(); return $same;");
        Fixture::lint(&restored);
        f.project.documents.replace_all(config.id, &restored).unwrap();
        assert_eq!(*definitions(&f).await[0].path, expected);
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        let restored_plan = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            doc.id,
            cursor(&source, "Trace {}"),
            "Traced".into(),
        )
        .await
        .unwrap()
        .edit;
        assert!(
            restored_plan
                .files
                .iter()
                .any(|edit| matches!(edit,ide_core::FileEdit::Edit {path,..} if **path==expected))
        );
        assert_eq!(std::fs::read_to_string(bootstrap).unwrap(), REPEATED_BOOTSTRAP);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
        assert!(!f.root.join("app/Http/Middleware/Traced.php").exists());
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

const PROVIDER_BOOTSTRAP: &str = r"<?php
$app=new \Illuminate\Foundation\Application;
$app->register(\App\Providers\KernelProvider::class);
return $app;
";
const PROVIDER: &str = r"<?php namespace App\Providers;
use Illuminate\Contracts\Http\Kernel as HttpKernel;
use App\Runtime\SiteKernel as SelectedKernel;
class KernelProvider extends \Illuminate\Support\ServiceProvider {
    public function register(): void {$this->app->singleton(HttpKernel::class,SelectedKernel::class);}
}
";

fn immediate_provider_fixture(provider: &str) -> (Fixture, String) {
    let selected = KERNEL
        .replace("namespace App\\Http;", "namespace App\\Runtime;")
        .replace("use App\\Http\\KernelBase as BaseKernel;\n", "")
        .replace("class Kernel extends BaseKernel", "class SiteKernel extends \\Illuminate\\Foundation\\Http\\Kernel");
    let conventional = KERNEL.replace("Middleware\\Audit as", "Middleware\\Trace as");
    let f = fixture_with(
        &conventional,
        &[
            ("bootstrap/app.php".into(), PROVIDER_BOOTSTRAP.into()),
            ("app/Providers/KernelProvider.php".into(), provider.into()),
            ("app/Runtime/SiteKernel.php".into(), selected.clone()),
            ("framework/Contracts/Http/Kernel.php".into(), "<?php namespace Illuminate\\Contracts\\Http; interface Kernel {}\n".into()),
            ("framework/Foundation/Application.php".into(), r"<?php namespace Illuminate\Foundation; class Application {public function singleton($abstract,$concrete=null): void {} public function register($provider,$force=false): \Illuminate\Support\ServiceProvider {}}".into()),
            ("framework/Support/ServiceProvider.php".into(), r"<?php namespace Illuminate\Support; class ServiceProvider {protected $app; public function __construct($app) {$this->app=$app;} public function register() {}}".into()),
        ],
    );
    (f, selected)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom170 and owned PHP syntax-check executable"]
async fn immediate_provider_navigation_retires_off_target_plans_and_replays_dirty_sources() {
    let (f, selected) = immediate_provider_fixture(PROVIDER);
    let outcome = std::panic::AssertUnwindSafe(async {
        let path = f.root.join("app/Runtime/SiteKernel.php");
        let found = definitions(&f).await;
        assert_eq!(found.len(), 1, "managed immediate provider owner: {found:?}");
        assert_eq!(*found[0].path, path);
        assert_eq!(found[0].range.start, cursor(&selected, r"au\x64ited"));
        hover(&f, "App\\Http\\Middleware\\Audit", "app/Runtime/SiteKernel.php").await;
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
        assert_eq!(refs.locations.len(), 2, "{refs:?}");
        assert!(refs.locations.iter().all(|location| location.range.end.col - location.range.start.col == 10));
        let provider = f.root.join("app/Providers/KernelProvider.php");
        let doc = f.project.documents.open(&provider).unwrap();
        let class = f.project.documents.open(&f.root.join("app/Http/Middleware/Audit.php")).unwrap();
        let plan = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            class.id,
            cursor(CLASS, "Audit {}"),
            "Auditor".into(),
        )
        .await
        .unwrap()
        .edit;
        assert_eq!(plan.guard.as_ref().unwrap().documents_revision, f.project.documents.edit_revision().unwrap());
        assert!(
            !plan.files.iter().any(|edit| matches!(edit, ide_core::FileEdit::Edit {path,..} if **path==provider)),
            "provider dependency must be outside the edit targets"
        );
        let changed = PROVIDER.replace("App\\Runtime\\SiteKernel as", "App\\Http\\Kernel as");
        Fixture::lint(&changed);
        f.project.documents.replace_all(doc.id, &changed).unwrap();
        assert!(
            ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.is_err(),
            "provider dependency must retire a reviewed plan"
        );
        assert_eq!(*definitions(&f).await[0].path, f.root.join("app/Http/Kernel.php"));
        hover(&f, "App\\Http\\Middleware\\Trace", "app/Http/Kernel.php").await;
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        hover(&f, "App\\Http\\Middleware\\Trace", "app/Http/Kernel.php").await;
        f.project.documents.replace_all(doc.id, PROVIDER).unwrap();
        assert_eq!(*definitions(&f).await[0].path, path);
        assert_eq!(std::fs::read_to_string(provider).unwrap(), PROVIDER);
        assert_eq!(std::fs::read_to_string(path).unwrap(), selected);
        assert_eq!(std::fs::read_to_string(f.root.join("app/Http/Middleware/Audit.php")).unwrap(), CLASS);
        assert!(!f.root.join("app/Http/Middleware/Auditor.php").exists());
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom170 and owned PHP syntax-check executable"]
async fn immediate_provider_checked_edits_preserve_conflicts_dirty_buffers_and_shared_history() {
    let (f, selected) = immediate_provider_fixture(PROVIDER);
    checked_rename_case(f, &selected, "app/Runtime/SiteKernel.php").await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom170 and owned PHP syntax-check executable"]
async fn immediate_provider_maps_keep_order_and_unknown_lifecycle_refuses_checked_edits() {
    let provider = PROVIDER.replace("    public function register()", r"    public $bindings=[HttpKernel::class=>\App\Http\Kernel::class]; public $singletons=[HttpKernel::class=>SelectedKernel::class];
    public function register()");
    let (f, _) = immediate_provider_fixture(&provider);
    let outcome = std::panic::AssertUnwindSafe(async {
        let found = definitions(&f).await;
        assert_eq!(found.len(), 1, "managed provider map order: {found:?}");
        assert_eq!(*found[0].path, f.root.join("app/Runtime/SiteKernel.php"));
        let doc = f.project.documents.open(&f.root.join("app/Providers/KernelProvider.php")).unwrap();
        let class = f.project.documents.open(&f.root.join("app/Http/Middleware/Audit.php")).unwrap();
        for changed in [
            provider.replace("register(): void", "register($phase=new \\stdClass): void"),
            provider.replace("<?php namespace App\\Providers;", "<?php namespace App\\Providers; unmodeled();"),
        ] {
            Fixture::lint(&changed);
            f.project.documents.replace_all(doc.id, &changed).unwrap();
            let hover =
                ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, cursor(CALLER, r"64ited"))
                    .await
                    .unwrap()
                    .unwrap();
            assert!(hover.contents.value.contains("incomplete"), "{hover:?}");
            let refused = ide_host::ops::lsp::lsp_rename(
                &f.host,
                "main",
                RequestId::next(),
                class.id,
                cursor(CLASS, "Audit {}"),
                "Auditor".into(),
            )
            .await;
            assert!(
                refused.as_ref().is_err_and(|error| error.message.contains("middleware")),
                "unmodeled provider lifecycle cannot authorize a checked plan: {refused:?}"
            );
        }
        f.project.documents.replace_all(doc.id, &provider).unwrap();
        assert_eq!(*definitions(&f).await[0].path, f.root.join("app/Runtime/SiteKernel.php"));
        assert_eq!(std::fs::read_to_string(f.root.join("app/Providers/KernelProvider.php")).unwrap(), provider);
        assert!(!f.root.join("app/Http/Middleware/Auditor.php").exists());
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

const PRIORITY_BASE: &str = r"<?php namespace Illuminate\Foundation\Http;
class Kernel {public function __construct() {} protected function syncMiddlewareToRouter() {}}
";

fn kernel_priority_source() -> String {
    KERNEL.replace("\n}", "\nprotected $routeMiddleware=['audited'=>\\App\\Http\\Middleware\\Trace::class];\n}")
}

fn kernel_priority_fixture(kernel: &str, extra: &[(String, String)]) -> Fixture {
    let mut files = vec![("framework/Foundation/Http/Kernel.php".into(), PRIORITY_BASE.into())];
    files.extend_from_slice(extra);
    fixture_with(kernel, &files)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom171 and owned PHP syntax-check executable"]
async fn kernel_priority_parent_dependency_retires_plans_and_replays_dirty_overrides() {
    let parent = KERNEL.replace("class Kernel extends", "class PriorityKernelBase extends");
    let root = r"<?php namespace App\Http; class Kernel extends PriorityKernelBase {protected $routeMiddleware=['audited'=>\App\Http\Middleware\Trace::class];}";
    let f = kernel_priority_fixture(root, &[("app/Http/PriorityKernelBase.php".into(), parent.clone())]);
    let outcome = std::panic::AssertUnwindSafe(async {
        let declaration = f.root.join("app/Http/PriorityKernelBase.php");
        let found = definitions(&f).await;
        assert_eq!(found.len(), 1, "managed effective Kernel alias owner: {found:?}");
        assert_eq!(*found[0].path, declaration);
        assert_eq!(found[0].range.start, cursor(&parent, r"au\x64ited"));
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
        assert_eq!(refs.locations.len(), 2, "only the effective declaration is referenced: {refs:?}");
        assert!(refs.locations.iter().all(|location| location.range.end.col - location.range.start.col == 10));
        let dependency = f.root.join("app/Http/Kernel.php");
        let doc = f.project.documents.open(&dependency).unwrap();
        let class = f.project.documents.open(&f.root.join("app/Http/Middleware/Audit.php")).unwrap();
        let plan = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            class.id,
            cursor(CLASS, "Audit {}"),
            "Auditor".into(),
        )
        .await
        .unwrap()
        .edit;
        assert_eq!(plan.guard.as_ref().unwrap().documents_revision, f.project.documents.edit_revision().unwrap());
        assert!(
            !plan.files.iter().any(|edit| matches!(edit,ide_core::FileEdit::Edit{path,..} if **path==dependency)),
            "Kernel dependency must be outside the reviewed targets"
        );
        let changed =
            root.replace("protected $routeMiddleware", "protected $middlewareAliases=[];protected $routeMiddleware");
        Fixture::lint(&changed);
        f.project.documents.replace_all(doc.id, &changed).unwrap();
        assert!(
            ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.is_err(),
            "alias priority dependency must retire a reviewed plan"
        );
        assert_eq!(*definitions(&f).await[0].path, dependency);
        hover(&f, "App\\Http\\Middleware\\Trace", "app/Http/Kernel.php").await;
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        hover(&f, "App\\Http\\Middleware\\Trace", "app/Http/Kernel.php").await;
        f.project.documents.replace_all(doc.id, root).unwrap();
        assert_eq!(*definitions(&f).await[0].path, declaration);
        assert_eq!(std::fs::read_to_string(&dependency).unwrap(), root);
        assert_eq!(std::fs::read_to_string(&declaration).unwrap(), parent);
        assert_eq!(std::fs::read_to_string(f.root.join("app/Http/Middleware/Audit.php")).unwrap(), CLASS);
        assert!(!f.root.join("app/Http/Middleware/Auditor.php").exists());
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom171 and owned PHP syntax-check executable"]
async fn kernel_priority_checked_edits_preserve_conflicts_dirty_buffers_and_history() {
    let source = kernel_priority_source();
    let f = kernel_priority_fixture(&source, &[]);
    let readiness = std::panic::AssertUnwindSafe(async {
        let found = definitions(&f).await;
        assert_eq!(found.len(), 1, "managed modern Kernel alias must override the legacy declaration: {found:?}");
        assert_eq!(found[0].range.start, cursor(&source, r"au\x64ited"));
    })
    .catch_unwind()
    .await;
    if readiness.is_err() {
        f.finish(readiness).await;
        return;
    }
    checked_rename_case(f, &source, "app/Http/Kernel.php").await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom171 and owned PHP syntax-check executable"]
async fn kernel_priority_shadowed_class_strings_stay_guarded_after_effective_navigation() {
    let source = kernel_priority_source().replace(r"\App\Http\Middleware\Trace::class", r"'App\Http\Middleware\Trace'");
    let f = kernel_priority_fixture(&source, &[]);
    let outcome = std::panic::AssertUnwindSafe(async {
        let found = definitions(&f).await;
        assert_eq!(found.len(), 1, "shadowed physical alias must not be an effective definition: {found:?}");
        hover(&f, "App\\Http\\Middleware\\Audit", "app/Http/Kernel.php").await;
        let trace = CLASS.replace("Audit", "Trace");
        let doc = f.project.documents.open(&f.root.join("app/Http/Middleware/Trace.php")).unwrap();
        let refused = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            doc.id,
            cursor(&trace, "Trace {}"),
            "Traced".into(),
        )
        .await;
        assert!(
            refused.as_ref().is_err_and(|error| error.message.contains("middleware")),
            "shadowed physical class string must still refuse an unsupported edit: {refused:?}"
        );
        let audit = f.project.documents.open(&f.root.join("app/Http/Middleware/Audit.php")).unwrap();
        let plan = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            audit.id,
            cursor(CLASS, "Audit {}"),
            "Auditor".into(),
        )
        .await
        .unwrap()
        .edit;
        assert!(
            plan.files.iter().any(
                |edit| matches!(edit,ide_core::FileEdit::Edit{path,..} if **path==f.root.join("app/Http/Kernel.php"))
            ),
            "named class preview remains complete: {plan:?}"
        );
        assert_eq!(std::fs::read_to_string(f.root.join("app/Http/Kernel.php")).unwrap(), source);
        assert_eq!(std::fs::read_to_string(f.root.join("app/Http/Middleware/Trace.php")).unwrap(), trace);
        assert!(!f.root.join("app/Http/Middleware/Traced.php").exists());
        assert!(!f.root.join("app/Http/Middleware/Auditor.php").exists());
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

const CALLBACK_BOOTSTRAP: &str = r#"<?php
use Illuminate\Foundation\Application;
use Illuminate\Foundation\Configuration\Middleware;
$app=Application::configure()->withMiddleware(function(Middleware $middleware) {
    $middleware->alias([/* 😀 chosen */ "au\x64ited"=>\App\Http\Middleware\Trace::class]);
})->create();
$app->singleton(\Illuminate\Contracts\Http\Kernel::class,\App\Http\Kernel::class);
return $app;
"#;
const CALLBACK_MIDDLEWARE: &str = r"<?php namespace Illuminate\Foundation\Configuration;
class Middleware {
 protected $customAliases=[];
 public function alias(array $aliases): static {$this->customAliases=array_merge($this->customAliases,$aliases);return $this;}
 protected function defaultAliases() {return [];}
 public function getMiddlewareAliases() {return array_merge($this->defaultAliases(),$this->customAliases);}
}
";

fn callback_fixture() -> (Fixture, String) {
    callback_fixture_with_trait(false)
}

const COMPATIBLE_TRAIT: &str = r#"<?php
namespace App\Http;
use App\Http\Middleware\Audit as Shared;
trait AliasDefaults {
    protected $middlewareAliases = [/* 😀 */ "audited" => Shared::class];
}
"#;

fn callback_fixture_with_trait(composed: bool) -> (Fixture, String) {
    let kernel = if composed {
        kernel_priority_source()
            .replace("class Kernel extends BaseKernel {", "class Kernel extends BaseKernel {\n    use AliasDefaults;")
    } else {
        kernel_priority_source()
    };
    let builder = BUILDER.replace("    public function create()", "    public function withMiddleware(?callable $callback=null): static {return $this;}\n    public function create()");
    let framework = PRIORITY_BASE.replace("protected function syncMiddlewareToRouter() {}", "protected function syncMiddlewareToRouter() {} public function setMiddlewareAliases(array $aliases) {$this->middlewareAliases=$aliases;$this->syncMiddlewareToRouter();return $this;}");
    let mut extra = vec![
            ("composer.json".into(), r#"{"require":{"php":"^8.3","laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/","Illuminate\\":"framework/"}}}"#.into()),
            ("bootstrap/app.php".into(), CALLBACK_BOOTSTRAP.into()),
            ("framework/Foundation/Application.php".into(), BUILDER_APP.into()),
            ("framework/Foundation/Configuration/ApplicationBuilder.php".into(), builder),
            ("framework/Foundation/Configuration/Middleware.php".into(), CALLBACK_MIDDLEWARE.into()),
            ("framework/Foundation/Http/Kernel.php".into(), framework),
            (
                "modules/peer/composer.json".into(),
                r#"{"name":"fixture/peer","autoload":{"psr-4":{"Peer\\":"src/"}}}"#.into(),
            ),
            ("modules/peer/src/Peer.php".into(), "<?php namespace Peer; class Peer {}\n".into()),
        ];
    if composed {
        extra.push(("app/Http/AliasDefaults.php".into(), COMPATIBLE_TRAIT.into()));
    }
    let f = kernel_priority_fixture(&kernel, &extra);
    (f, kernel)
}

async fn callback_definition(f: &Fixture, expected: &str, rejected: &str) {
    let found = definitions(f).await;
    assert_eq!(found.len(), 1, "managed callback must retire shadowed Kernel declarations: {found:?}");
    assert_eq!(*found[0].path, f.root.join("bootstrap/app.php"));
    assert_eq!(found[0].range.start, cursor(CALLBACK_BOOTSTRAP, r"au\x64ited"));
    let hover = ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, cursor(CALLER, r"64ited"))
        .await
        .unwrap()
        .unwrap();
    assert!(
        hover.contents.value.contains(expected)
            && hover.contents.value.contains("bootstrap/app.php")
            && !hover.contents.value.contains(rejected)
            && !hover.contents.value.contains("incomplete"),
        "{hover:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom173 and owned PHP syntax-check executable"]
async fn builder_callback_checked_edits_preserve_shadowed_sources_conflicts_and_history() {
    let (f, kernel) = callback_fixture();
    let readiness = std::panic::AssertUnwindSafe(async {
        callback_definition(&f, "App\\Http\\Middleware\\Trace", "App\\Http\\Middleware\\Audit").await;
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
        assert_eq!(refs.locations.len(), 2, "{refs:?}");
        assert!(refs.locations.iter().all(|location| location.range.end.col - location.range.start.col == 10));
    })
    .catch_unwind()
    .await;
    if readiness.is_err() {
        f.finish(readiness).await;
        return;
    }
    checked_rename_case(f, &kernel, "app/Http/Kernel.php").await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom173 and owned PHP syntax-check executable"]
async fn builder_callback_dirty_dependencies_retire_plans_and_keep_nested_buffers_through_history() {
    let (f, _) = callback_fixture();
    let outcome = std::panic::AssertUnwindSafe(async {
        callback_definition(&f, "App\\Http\\Middleware\\Trace", "App\\Http\\Middleware\\Audit").await;
        let bootstrap = f.root.join("bootstrap/app.php");
        let config = f.project.documents.open(&bootstrap).unwrap();
        let foreign = f.root.join("modules/peer/src/Peer.php");
        let original = std::fs::read_to_string(&foreign).unwrap();
        let nested = f.project.documents.open(&foreign).unwrap();
        let nested_dirty = format!("{original}// Unsaved independent package.\n");
        f.project.documents.replace_all(nested.id, &nested_dirty).unwrap();
        assert!(
            !ide_host::ops::lsp::lsp_goto(
                &f.host,
                "main",
                nested.id,
                cursor(&original, "Peer {}"),
                GotoKind::Definition
            )
            .await
            .unwrap()
            .is_empty()
        );
        assert_eq!(f.project.lsp.servers().len(), 1);
        let old = f.root.join("app/Http/Middleware/Audit.php");
        let new = f.root.join("app/Http/Middleware/Auditor.php");
        let class = f.project.documents.open(&old).unwrap();
        let overlay = format!("{CLASS}// Unsaved callback middleware.\n");
        f.project.documents.replace_all(class.id, &overlay).unwrap();
        let plan = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            class.id,
            cursor(CLASS, "Audit {}"),
            "Auditor".into(),
        )
        .await
        .unwrap()
        .edit;
        assert_eq!(plan.guard.as_ref().unwrap().documents_revision, f.project.documents.edit_revision().unwrap());
        assert!(
            !plan
                .files
                .iter()
                .any(|edit| matches!(edit,ide_core::FileEdit::Edit{path,..} if **path==bootstrap || **path==foreign)),
            "dependency must be outside the first plan targets: {plan:?}"
        );
        let changed = CALLBACK_BOOTSTRAP.replace("Trace::class", "Audit::class");
        Fixture::lint(&changed);
        f.project.documents.replace_all(config.id, &changed).unwrap();
        assert!(
            ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.is_err(),
            "off-target callback dependency must retire the reviewed plan"
        );
        assert_eq!(f.project.documents.snapshot(class.id).unwrap().text, overlay);
        assert_eq!(std::fs::read_to_string(&old).unwrap(), CLASS);
        assert!(!new.exists());
        callback_definition(&f, "App\\Http\\Middleware\\Audit", "App\\Http\\Middleware\\Trace").await;
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        callback_definition(&f, "App\\Http\\Middleware\\Audit", "App\\Http\\Middleware\\Trace").await;
        let plan = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            class.id,
            cursor(CLASS, "Audit {}"),
            "Auditor".into(),
        )
        .await
        .unwrap()
        .edit;
        assert!(
            plan.files.iter().any(|edit| matches!(edit,ide_core::FileEdit::Edit{path,..} if **path==bootstrap)),
            "dirty callback must enter the fresh plan: {plan:?}"
        );
        ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.unwrap();
        let renamed = f.project.documents.snapshot(class.id).unwrap();
        assert!(renamed.text.contains("class Auditor") && renamed.text.contains("Unsaved callback middleware"));
        assert!(!old.exists());
        assert_eq!(std::fs::read_to_string(&new).unwrap(), renamed.text);
        let updated = f.project.documents.snapshot(config.id).unwrap();
        assert!(updated.text.contains("Middleware\\Auditor::class"));
        Fixture::lint(&updated.text);
        assert_eq!(std::fs::read_to_string(&bootstrap).unwrap(), updated.text);
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
            .expect("callback rename must share history");
        for redo in [false, true] {
            ide_host::ops::lsp::restore_reviewed_edit(&f.host, "main", group.clone(), redo).await.unwrap();
            let restored = f.project.documents.snapshot(config.id).unwrap();
            assert_eq!(restored.text, if redo { updated.text.as_str() } else { changed.as_str() });
            assert_eq!(restored.dirty, !redo);
            assert_eq!(
                std::fs::read_to_string(&bootstrap).unwrap(),
                if redo { updated.text.as_str() } else { CALLBACK_BOOTSTRAP }
            );
            assert_eq!(f.project.documents.by_path(if redo { &new } else { &old }), Some(class.id));
            let peer = f.project.documents.snapshot(nested.id).unwrap();
            assert_eq!(peer.text, nested_dirty);
            assert!(peer.dirty);
            assert_eq!(std::fs::read_to_string(&foreign).unwrap(), original);
        }
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

fn trait_defaults_runtime(f: &Fixture, expected: &str) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let php = std::env::var_os("OPENSTORM_TEST_PHP_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("artifacts/toolchains/php-8.4.23-static/php"));
    let script = r"foreach (['framework/Foundation/Http/Kernel.php','app/Http/KernelBase.php','app/Http/AliasDefaults.php','app/Http/Kernel.php'] as $file) { require $argv[1].'/'.$file; }
$defaults=(new ReflectionClass(\App\Http\Kernel::class))->getDefaultProperties();
if ($defaults['middlewareAliases']['audited'] !== 'App\\Http\\Middleware\\'.$argv[2]) { exit(9); }";
    let status = std::process::Command::new(php)
        .args(["-n", "-r", script])
        .arg(&f.root)
        .arg(expected)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "composed PHP defaults must survive reviewed edits: {status}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom174 and owned PHP syntax-check executable"]
async fn compatible_trait_checked_apply_preserves_physical_sources_conflicts_and_history() {
    let (f, kernel_source) = callback_fixture_with_trait(true);
    let outcome = std::panic::AssertUnwindSafe(async {
        callback_definition(&f, "App\\Http\\Middleware\\Trace", "App\\Http\\Middleware\\Audit").await;
        let old = f.root.join("app/Http/Middleware/Audit.php");
        let new = f.root.join("app/Http/Middleware/Auditor.php");
        let kernel = f.root.join("app/Http/Kernel.php");
        let trait_path = f.root.join("app/Http/AliasDefaults.php");
        let class = f.project.documents.open(&old).unwrap();
        let registration = f.project.documents.open(&kernel).unwrap();
        let trait_doc = f.project.documents.open(&trait_path).unwrap();
        let overlay = format!("{CLASS}// Unsaved compatible-trait target.\n");
        f.project.documents.replace_all(class.id, &overlay).unwrap();
        let dirty_trait = COMPATIBLE_TRAIT.replace("\"audited\"", "'audited'");
        assert_ne!(dirty_trait, COMPATIBLE_TRAIT);
        Fixture::lint(&dirty_trait);
        f.project.documents.replace_all(trait_doc.id, &dirty_trait).unwrap();
        let plan = ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, cursor(CLASS, "Audit {}"), "Auditor".into()).await.unwrap().edit;
        for path in [&kernel, &trait_path] {
            assert!(plan.files.iter().any(|edit| matches!(edit, ide_core::FileEdit::Edit{path:target,edits,..} if **target==*path && edits.iter().any(|edit| edit.new_text.contains("Middleware\\Auditor as")))), "compatible physical trait source must enter the reviewed plan: {path:?}: {plan:?}");
        }
        let literal = dirty_trait.replace("Shared::class", "'App\\\\Http\\\\Middleware\\\\Audit'");
        Fixture::lint(&literal);
        f.project.documents.replace_all(trait_doc.id, &literal).unwrap();
        assert!(ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.is_err(), "a changed physical trait must retire the reviewed plan");
        callback_definition(&f, "App\\Http\\Middleware\\Trace", "App\\Http\\Middleware\\Audit").await;
        let refused = ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, cursor(CLASS, "Audit {}"), "Auditor".into()).await;
        assert!(refused.as_ref().is_err_and(|error| error.message.contains("literal middleware")), "hidden compatible literal target must stay guarded: {refused:?}");
        f.project.documents.replace_all(trait_doc.id, &dirty_trait).unwrap();
        let stale = ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, cursor(CLASS, "Audit {}"), "Auditor".into()).await.unwrap().edit;
        let external = format!("{COMPATIBLE_TRAIT}// External trait edit.\n");
        std::fs::write(&trait_path, &external).unwrap();
        assert!(ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", stale).await.is_err());
        assert_eq!(std::fs::read_to_string(&trait_path).unwrap(), external);
        assert_eq!(std::fs::read_to_string(&old).unwrap(), CLASS);
        assert_eq!(f.project.documents.snapshot(class.id).unwrap().text, overlay);
        assert!(!new.exists());
        std::fs::write(&trait_path, COMPATIBLE_TRAIT).unwrap();
        let plan = ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, cursor(CLASS, "Audit {}"), "Auditor".into()).await.unwrap().edit;
        assert!(plan.files.iter().any(|edit| matches!(edit,ide_core::FileEdit::Rename{from,to} if **from==old && **to==new)));
        ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.unwrap();
        let renamed = f.project.documents.snapshot(class.id).unwrap();
        let kernel_updated = f.project.documents.snapshot(registration.id).unwrap();
        let trait_updated = f.project.documents.snapshot(trait_doc.id).unwrap();
        assert!(renamed.text.contains("class Auditor") && renamed.text.contains("Unsaved compatible-trait target"));
        assert!(kernel_updated.text.contains("Middleware\\Auditor as AuditAlias"));
        assert!(trait_updated.text.contains("Middleware\\Auditor as Shared"));
        assert!(trait_updated.text.contains("'audited'"));
        assert_eq!(f.project.documents.by_path(&new), Some(class.id));
        assert!(!old.exists());
        for (path, text) in [(&new, &renamed.text), (&kernel, &kernel_updated.text), (&trait_path, &trait_updated.text)] {
            assert_eq!(std::fs::read_to_string(path).unwrap(), *text);
            Fixture::lint(text);
        }
        trait_defaults_runtime(&f, "Auditor");
        let group = f.events.lock().unwrap().iter().rev().find_map(|event| match event {
            ide_core::IdeEvent::DocPatched{history:Some(history),..} => Some(history.group.clone()),
            _ => None,
        }).expect("compatible trait edits must share one history group");
        for redo in [false, true] {
            ide_host::ops::lsp::restore_reviewed_edit(&f.host, "main", group.clone(), redo).await.unwrap();
            let class_snapshot = f.project.documents.snapshot(class.id).unwrap();
            let trait_snapshot = f.project.documents.snapshot(trait_doc.id).unwrap();
            assert_eq!(class_snapshot.text, if redo { renamed.text.as_str() } else { overlay.as_str() });
            assert_eq!(trait_snapshot.text, if redo { trait_updated.text.as_str() } else { dirty_trait.as_str() });
            assert_eq!(class_snapshot.dirty, !redo);
            assert_eq!(trait_snapshot.dirty, !redo);
            assert_eq!(f.project.documents.by_path(if redo { &new } else { &old }), Some(class.id));
            assert_eq!(std::fs::read_to_string(if redo { &new } else { &old }).unwrap(), if redo { renamed.text.as_str() } else { CLASS });
            assert_eq!(std::fs::read_to_string(&kernel).unwrap(), if redo { kernel_updated.text.as_str() } else { kernel_source.as_str() });
            assert_eq!(std::fs::read_to_string(&trait_path).unwrap(), if redo { trait_updated.text.as_str() } else { COMPATIBLE_TRAIT });
            trait_defaults_runtime(&f, if redo { "Auditor" } else { "Audit" });
            callback_definition(&f, "App\\Http\\Middleware\\Trace", "App\\Http\\Middleware\\Audit").await;
        }
    }).catch_unwind().await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom174 and owned PHP syntax-check executable"]
async fn compatible_trait_dependencies_retire_plans_and_replay_dirty_refusals() {
    let (f, _) = callback_fixture_with_trait(true);
    let outcome = std::panic::AssertUnwindSafe(async {
        callback_definition(&f, "App\\Http\\Middleware\\Trace", "App\\Http\\Middleware\\Audit").await;
        let old = f.root.join("app/Http/Middleware/Audit.php");
        let new = f.root.join("app/Http/Middleware/Auditor.php");
        let class = f.project.documents.open(&old).unwrap();
        let dependency = f.root.join("app/Http/KernelBase.php");
        let base = f.project.documents.open(&dependency).unwrap();
        let trait_path = f.root.join("app/Http/AliasDefaults.php");
        let trait_doc = f.project.documents.open(&trait_path).unwrap();
        let plan = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            class.id,
            cursor(CLASS, "Audit {}"),
            "Auditor".into(),
        )
        .await
        .unwrap()
        .edit;
        assert!(!plan.files.iter().any(|edit| matches!(edit,ide_core::FileEdit::Edit{path,..} if **path==dependency)));
        let base_dirty = format!("{BASE}// Unsaved off-target trait dependency.\n");
        f.project.documents.replace_all(base.id, &base_dirty).unwrap();
        assert!(
            ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.is_err(),
            "off-target trait graph dependency must retire the reviewed plan"
        );
        let plan = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            class.id,
            cursor(CLASS, "Audit {}"),
            "Auditor".into(),
        )
        .await
        .unwrap()
        .edit;
        let incompatible = COMPATIBLE_TRAIT.replace("Shared::class", "\\App\\Http\\Middleware\\Trace::class");
        Fixture::lint(&incompatible);
        f.project.documents.replace_all(trait_doc.id, &incompatible).unwrap();
        assert!(ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.is_err());
        for restarted in [false, true] {
            if restarted {
                f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
            }
            let result =
                ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, cursor(CALLER, r"64ited"))
                    .await
                    .unwrap()
                    .unwrap();
            assert!(
                result.contents.value.contains("incomplete"),
                "dirty incompatible trait must survive restart: {result:?}"
            );
            let refused = ide_host::ops::lsp::lsp_rename(
                &f.host,
                "main",
                RequestId::next(),
                class.id,
                cursor(CLASS, "Audit {}"),
                "Auditor".into(),
            )
            .await;
            assert!(refused.as_ref().is_err_and(|error| error.message.contains("middleware")), "{refused:?}");
        }
        f.project.documents.replace_all(trait_doc.id, COMPATIBLE_TRAIT).unwrap();
        callback_definition(&f, "App\\Http\\Middleware\\Trace", "App\\Http\\Middleware\\Audit").await;
        let fresh = ide_host::ops::lsp::lsp_rename(
            &f.host,
            "main",
            RequestId::next(),
            class.id,
            cursor(CLASS, "Audit {}"),
            "Auditor".into(),
        )
        .await
        .unwrap();
        assert!(fresh.incomplete.is_none());
        assert_eq!(f.project.documents.snapshot(base.id).unwrap().text, base_dirty);
        assert!(f.project.documents.snapshot(base.id).unwrap().dirty);
        assert_eq!(std::fs::read_to_string(&dependency).unwrap(), BASE);
        assert_eq!(std::fs::read_to_string(&trait_path).unwrap(), COMPATIBLE_TRAIT);
        assert_eq!(std::fs::read_to_string(&old).unwrap(), CLASS);
        assert!(!new.exists());
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}
