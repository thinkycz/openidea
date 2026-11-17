//! Application middleware through the actual client and guarded host edit paths.
use super::*;

const BOOTSTRAP: &str = r#"<?php
use Illuminate\Foundation\Application;
use Illuminate\Foundation\Configuration\Middleware as Stack;
use App\Http\Middleware\Audit as AuditAlias;
return Application::configure()->withMiddleware(function (Stack $pipeline): void {
    /* 😀 */ $pipeline->alias(["au\x64ited" => AuditAlias::class]);
})->create();
"#;
const CALLER: &str = r#"<?php
use Illuminate\Support\Facades\Route;
new \App\Http\Middleware\Audit;
/* 😀 */ Route::middleware("au\x64ited:strict");
class Child extends \Illuminate\Routing\Controller {public function boot() {$this->middleware('audited:soft');}}
class Foreign {public function middleware($value) {} public function boot() {$this->middleware('audited');}}
"#;
const CLASS: &str = "<?php namespace App\\Http\\Middleware; class Audit {}\n";

fn fixture_files() -> Vec<(String, String)> {
    vec![
        ("composer.json".into(), r#"{"require":{"laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/","Illuminate\\":"framework/"}}}"#.into()),
        ("bootstrap/app.php".into(), BOOTSTRAP.into()),
        ("app/Http/Middleware/Audit.php".into(), CLASS.into()),
        ("app/Http/Middleware/Trace.php".into(), CLASS.replace("Audit", "Trace")),
        ("framework/Foundation/Application.php".into(), "<?php namespace Illuminate\\Foundation; class Application {public static function configure(): \\Illuminate\\Foundation\\Configuration\\ApplicationBuilder {return new \\Illuminate\\Foundation\\Configuration\\ApplicationBuilder;}}".into()),
        ("framework/Foundation/Configuration/ApplicationBuilder.php".into(), "<?php namespace Illuminate\\Foundation\\Configuration; class ApplicationBuilder {public function withMiddleware(callable $callback): static {return $this;} public function create(): \\Illuminate\\Foundation\\Application {return new \\Illuminate\\Foundation\\Application;}}".into()),
        ("framework/Foundation/Configuration/Middleware.php".into(), "<?php namespace Illuminate\\Foundation\\Configuration; class Middleware {public function alias(array $aliases): static {return $this;}}".into()),
        ("framework/Routing/Controller.php".into(), "<?php namespace Illuminate\\Routing; class Controller {protected function middleware($middleware): static {return $this;}}".into()),
    ]
}

fn fixture() -> Fixture {
    Fixture::new_at(CALLER, &fixture_files(), "routes/web.php")
}

fn cursor(text: &str, needle: &str) -> Pos {
    at(text, text.find(needle).unwrap())
}

async fn definitions(f: &Fixture, position: Pos) -> Vec<ide_core::Location> {
    ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, position, GotoKind::Definition).await.unwrap()
}

async fn target(f: &Fixture, expected: &str) {
    let hover = ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, cursor(CALLER, r"64ited"))
        .await
        .unwrap()
        .expect("application alias hover");
    assert!(hover.contents.value.contains(expected), "{hover:?}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom160 and owned PHP syntax-check executable"]
async fn alias_navigation_preserves_decoded_ranges_unsaved_sources_and_restart() {
    let f = fixture();
    let outcome = std::panic::AssertUnwindSafe(async {
        let boot = f.root.join("bootstrap/app.php");
        assert!(!definitions(&f, cursor(CALLER, "Audit;")).await.is_empty(), "ordinary PHP readiness");
        let found = definitions(&f, cursor(CALLER, r"64ited")).await;
        assert_eq!(found.len(), 1, "application middleware alias definition: {found:?}");
        assert_eq!(*found[0].path, boot);
        assert_eq!(found[0].range.start, cursor(BOOTSTRAP, r"au\x64ited"));
        assert_eq!(found[0].range.end, found[0].range.start);
        target(&f, "App\\Http\\Middleware\\Audit").await;
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
        assert_eq!(refs.locations.len(), 3, "foreign middleware must not be a reference: {refs:?}");
        let encoded = refs
            .locations
            .iter()
            .find(|location| location.range.start == cursor(CALLER, r"au\x64ited"))
            .expect("encoded caller reference");
        assert_eq!(*encoded.path, f.root.join("routes/web.php"));
        assert_eq!(encoded.range.end.col - encoded.range.start.col, 10);
        let prepared = f.project.lsp.prepare_rename_details(f.doc, cursor(CALLER, r"64ited")).await.unwrap();
        assert!(prepared.is_none(), "alias-key Rename must not produce partial edits");
        let doc = f.project.documents.open(&boot).unwrap();
        let changed = BOOTSTRAP.replace("Middleware\\Audit as", "Middleware\\Trace as");
        f.project.documents.replace_all(doc.id, &changed).unwrap();
        target(&f, "App\\Http\\Middleware\\Trace").await;
        let renamed = changed.replace(r"au\x64ited", "different");
        f.project.documents.replace_all(doc.id, &renamed).unwrap();
        assert!(definitions(&f, cursor(CALLER, r"64ited")).await.is_empty());
        f.project.documents.replace_all(doc.id, &changed).unwrap();
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        target(&f, "App\\Http\\Middleware\\Trace").await;
        assert_eq!(std::fs::read_to_string(&boot).unwrap(), BOOTSTRAP);
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), CALLER);
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom160 and owned PHP syntax-check executable"]
async fn alias_target_rename_refuses_partial_plans_and_preserves_reviewed_history() {
    let f = fixture();
    let outcome = std::panic::AssertUnwindSafe(async {
        let old = f.root.join("app/Http/Middleware/Audit.php");
        let new = f.root.join("app/Http/Middleware/Auditor.php");
        let boot = f.root.join("bootstrap/app.php");
        let class = f.project.documents.open(&old).unwrap();
        let bootstrap = f.project.documents.open(&boot).unwrap();
        let overlay = format!("{CLASS}// Unsaved middleware documentation.\n");
        f.project.documents.replace_all(class.id, &overlay).unwrap();
        let position = cursor(CLASS, "Audit {}");
        for registration in [
            r"['audited'=>'App\\Http\\Middleware\\Audit']",
            r#"['audited'=>"App\\Http\\Middleware\\\x41udit"]"#,
            r"[strtolower('AUDITED')=>'App\\Http\\Middleware\\Audit']",
            r"['audited'=>$unknown]",
        ] {
            let source =
                format!("<?php\n(new \\Illuminate\\Foundation\\Configuration\\Middleware)->alias({registration});\n");
            Fixture::lint(&source);
            f.project.documents.replace_all(bootstrap.id, &source).unwrap();
            for name in ["Auditor", "App\\Moved\\Audit"] {
                let refused =
                    ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, position, name.into())
                        .await;
                assert!(
                    refused.as_ref().is_err_and(|e| e.message.contains("middleware")),
                    "unsafe middleware target plan: {refused:?}"
                );
                assert_eq!(f.project.documents.snapshot(class.id).unwrap().text, overlay);
                assert_eq!(f.project.documents.snapshot(bootstrap.id).unwrap().text, source);
                assert_eq!(std::fs::read_to_string(&old).unwrap(), CLASS);
                assert_eq!(std::fs::read_to_string(&boot).unwrap(), BOOTSTRAP);
                assert!(!new.exists());
            }
            let namespace = ide_host::ops::lsp::lsp_rename(
                &f.host,
                "main",
                RequestId::next(),
                class.id,
                cursor(CLASS, "Http\\Middleware"),
                "Changed".into(),
            )
            .await;
            assert!(
                namespace.as_ref().is_err_and(|error| error.message.contains("middleware")),
                "unsafe middleware namespace plan: {namespace:?}"
            );
        }
        f.project.documents.replace_all(bootstrap.id, BOOTSTRAP).unwrap();
        let plan =
            ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, position, "Auditor".into())
                .await
                .unwrap()
                .edit;
        assert!(
            plan.files.iter().any(|e| matches!(e, ide_core::FileEdit::Rename {from,to} if **from==old && **to==new)),
            "missing middleware PSR-4 move: {plan:?}"
        );
        let external = format!("{BOOTSTRAP}// External bootstrap change.\n");
        std::fs::write(&boot, &external).unwrap();
        assert!(ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.is_err());
        assert_eq!(std::fs::read_to_string(&boot).unwrap(), external);
        assert_eq!(f.project.documents.snapshot(class.id).unwrap().text, overlay);
        assert_eq!(std::fs::read_to_string(&old).unwrap(), CLASS);
        assert!(!new.exists());
        std::fs::write(&boot, BOOTSTRAP).unwrap();
        let plan =
            ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, position, "Auditor".into())
                .await
                .unwrap()
                .edit;
        ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", plan).await.unwrap();
        assert!(!old.exists());
        assert_eq!(f.project.documents.by_path(&new), Some(class.id));
        let renamed = f.project.documents.snapshot(class.id).unwrap();
        assert!(renamed.text.contains("class Auditor") && renamed.text.contains("Unsaved middleware documentation"));
        assert!(!renamed.dirty);
        assert_eq!(std::fs::read_to_string(&new).unwrap(), renamed.text);
        let updated = f.project.documents.snapshot(bootstrap.id).unwrap();
        assert!(updated.text.contains("Middleware\\Auditor as AuditAlias"));
        assert_eq!(std::fs::read_to_string(&boot).unwrap(), updated.text);
        Fixture::lint(&renamed.text);
        Fixture::lint(&updated.text);
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
            .expect("middleware rename must have shared history");
        for (redo, path, name) in [(false, &old, "Audit"), (true, &new, "Auditor")] {
            ide_host::ops::lsp::restore_reviewed_edit(&f.host, "main", group.clone(), redo).await.unwrap();
            assert_eq!(f.project.documents.by_path(path), Some(class.id));
            let text = f.project.documents.snapshot(class.id).unwrap().text;
            assert!(text.contains(&format!("class {name} {{")) && text.contains("Unsaved middleware documentation"));
            assert_eq!(f.project.documents.snapshot(class.id).unwrap().dirty, !redo);
            assert_eq!(
                std::fs::read_to_string(&boot).unwrap(),
                if redo { updated.text.clone() } else { BOOTSTRAP.into() }
            );
        }
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

fn exclusion_fixture(source: &str) -> Fixture {
    let mut files = fixture_files();
    files.extend([
        ("framework/Routing/Route.php".into(), r#"<?php namespace Illuminate\Routing; class Route {public function withoutMiddleware($middleware): static {return $this;}}"#.into()),
        ("framework/Routing/RouteRegistrar.php".into(), r#"<?php namespace Illuminate\Routing; /** @method \Illuminate\Routing\RouteRegistrar withoutMiddleware(array|string $middleware) */ class RouteRegistrar {public function __call(string $method, array $arguments): static {return $this;}}"#.into()),
        ("framework/Routing/Router.php".into(), r#"<?php namespace Illuminate\Routing; /** @method \Illuminate\Routing\RouteRegistrar withoutMiddleware(array|string $middleware) */ class Router {public function __call(string $method, array $arguments): RouteRegistrar {return new RouteRegistrar;}}"#.into()),
        ("framework/Routing/PendingResourceRegistration.php".into(), r#"<?php namespace Illuminate\Routing; class PendingResourceRegistration {public function withoutMiddleware($middleware): static {return $this;}}"#.into()),
        ("framework/Routing/PendingSingletonResourceRegistration.php".into(), r#"<?php namespace Illuminate\Routing; class PendingSingletonResourceRegistration {public function withoutMiddleware($middleware): static {return $this;}}"#.into()),
        ("framework/Support/Facades/Route.php".into(), r#"<?php namespace Illuminate\Support\Facades; class Route {public static function get($uri,$action): \Illuminate\Routing\Route {return new \Illuminate\Routing\Route;} public static function prefix($prefix): \Illuminate\Routing\RouteRegistrar {return new \Illuminate\Routing\RouteRegistrar;}}"#.into()),
    ]);
    Fixture::new_at(source, &files, "routes/web.php")
}

fn exclusion_position(text: &str, prefix: &str) -> Pos {
    at(text, text.find(prefix).unwrap() + prefix.len())
}

async fn exclusion_hover(f: &Fixture, source: &str, prefix: &str, expected: &str) {
    let hover =
        ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, exclusion_position(source, prefix))
            .await
            .unwrap()
            .expect("middleware exclusion hover");
    assert!(hover.contents.value.contains(expected), "{hover:?}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom161 and owned PHP syntax-check executable"]
async fn exclusion_navigation_preserves_binding_ranges_and_unsaved_registration_changes() {
    let source = r#"<?php
use Illuminate\Support\Facades\Route;
new \App\Http\Middleware\Audit;
/* 😀 */ Route::withoutMiddleware("au\x64ited:strict");
function exclusions(\Illuminate\Routing\Route $native, \Illuminate\Routing\RouteRegistrar $registrar, \Illuminate\Routing\Router $router, \Illuminate\Routing\PendingResourceRegistration $resource, \Illuminate\Routing\PendingSingletonResourceRegistration $singleton): void {
 $native->withoutMiddleware(middleware: 'audited:native');
 $native->WITHOUTMIDDLEWARE('audited:case');
 $registrar->withoutMiddleware('audited:registrar', middleware: 'audited:ignored');
 $registrar->withoutMiddleware(middleware: 'audited:ignored');
 $router->withoutMiddleware('audited:router');
 $router->withoutMiddleware(middleware: 'audited:ignored');
 $resource->withoutMiddleware(middleware: 'audited:resource');
 $singleton->withoutMiddleware(middleware: 'audited:singleton');
}
function unionReceiver(\Illuminate\Routing\Route|\Illuminate\Routing\RouteRegistrar|null $mixed) {$mixed?->withoutMiddleware('audited:union');}
class Foreign {public function withoutMiddleware($value){} public function run() {$this->withoutMiddleware('audited:ignored');}}
class Owner {public function __construct(private \Illuminate\Routing\Route $route){} public static function invalid() {$this->route->withoutMiddleware('audited:ignored');}}
Route::withoutMiddleware(middleware: 'audited:ignored');
Route::WITHOUTMIDDLEWARE('audited:ignored');
Route::get('/native',fn()=>null)->withoutMiddleware(middleware: 'audited:fluent');
Route::prefix('/magic')->withoutMiddleware(middleware: 'audited:ignored');
"#;
    let f = exclusion_fixture(source);
    let outcome = std::panic::AssertUnwindSafe(async {
        let boot = f.root.join("bootstrap/app.php");
        assert!(!definitions(&f, cursor(source, "Audit;")).await.is_empty(), "ordinary PHP readiness");
        let positions = [
            r#"Route::withoutMiddleware("au\x64"#,
            "$native->withoutMiddleware(middleware: 'aud",
            "$native->WITHOUTMIDDLEWARE('aud",
            "$registrar->withoutMiddleware('aud",
            "$router->withoutMiddleware('aud",
            "$resource->withoutMiddleware(middleware: 'aud",
            "$singleton->withoutMiddleware(middleware: 'aud",
            "$mixed?->withoutMiddleware('aud",
            "Route::get('/native',fn()=>null)->withoutMiddleware(middleware: 'aud",
        ];
        for prefix in positions {
            let found = definitions(&f, exclusion_position(source, prefix)).await;
            assert_eq!(found.len(), 1, "middleware exclusion definition {prefix}: {found:?}");
            assert_eq!(*found[0].path, boot);
            assert_eq!(found[0].range.start, cursor(BOOTSTRAP, r"au\x64ited"));
            assert_eq!(found[0].range.end, found[0].range.start);
        }
        for prefix in [
            "$registrar->withoutMiddleware('audited:registrar', middleware: 'aud",
            "$registrar->withoutMiddleware(middleware: 'aud",
            "$router->withoutMiddleware(middleware: 'aud",
            "$this->withoutMiddleware('aud",
            "$this->route->withoutMiddleware('aud",
            "Route::withoutMiddleware(middleware: 'aud",
            "Route::WITHOUTMIDDLEWARE('aud",
            "Route::prefix('/magic')->withoutMiddleware(middleware: 'aud",
        ] {
            assert!(
                definitions(&f, exclusion_position(source, prefix)).await.is_empty(),
                "unconsumed exclusion {prefix}"
            );
        }
        let position = exclusion_position(source, positions[0]);
        exclusion_hover(&f, source, positions[0], "App\\Http\\Middleware\\Audit").await;
        let refs = ide_host::ops::lsp::lsp_references(
            &f.host,
            "main",
            RequestId::next(),
            f.doc,
            position,
            true,
            true,
            true,
            ide_host::ops::Channel::new(|_| {}),
        )
        .await
        .unwrap();
        assert!(refs.incomplete.is_none(), "{refs:?}");
        assert_eq!(refs.locations.len(), positions.len() + 1, "only consumed exclusions are references: {refs:?}");
        let encoded = refs
            .locations
            .iter()
            .find(|location| {
                *location.path == f.root.join("routes/web.php") && location.range.start == cursor(source, r"au\x64ited")
            })
            .expect("encoded exclusion reference");
        assert_eq!(encoded.range.end.col - encoded.range.start.col, 10);
        assert!(
            f.project.lsp.prepare_rename_details(f.doc, position).await.unwrap().is_none(),
            "alias-key Rename must refuse partial exclusion edits"
        );
        let bootstrap = f.project.documents.open(&boot).unwrap();
        let target = BOOTSTRAP.replace("Middleware\\Audit as", "Middleware\\Trace as");
        assert_eq!(target.len(), BOOTSTRAP.len());
        f.project.documents.replace_all(bootstrap.id, &target).unwrap();
        exclusion_hover(&f, source, positions[0], "App\\Http\\Middleware\\Trace").await;
        let key = target.replace(r"au\x64ited", r"mi\x73sing");
        assert_eq!(key.len(), target.len());
        f.project.documents.replace_all(bootstrap.id, &key).unwrap();
        assert!(definitions(&f, position).await.is_empty(), "unsaved same-length alias key must retire navigation");
        f.project.documents.replace_all(bootstrap.id, &target).unwrap();
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        exclusion_hover(&f, source, positions[0], "App\\Http\\Middleware\\Trace").await;
        assert_eq!(std::fs::read_to_string(&boot).unwrap(), BOOTSTRAP);
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), source);
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom161 and owned PHP syntax-check executable"]
async fn exclusion_dependency_publication_and_unsaved_parent_survive_restart() {
    let source = r#"<?php
new \App\Http\Middleware\Audit;
function remove(\App\RouteBase $route) {$route->withoutMiddleware("au\x64ited");}
"#;
    let f = exclusion_fixture(source);
    let outcome = std::panic::AssertUnwindSafe(async {
        assert!(!definitions(&f, cursor(source, "Audit;")).await.is_empty(), "ordinary PHP readiness");
        let position = exclusion_position(source, r#"withoutMiddleware("au\x64"#);
        assert!(definitions(&f, position).await.is_empty(), "unknown receiver must not invent exclusion navigation");
        let parented = "<?php namespace App; class RouteBase extends \\Illuminate\\Routing\\Route {}\n";
        let plain = "<?php namespace App; class RouteBase {}\n";
        Fixture::lint(parented);
        Fixture::lint(plain);
        let path = f.root.join("app/RouteBase.php");
        std::fs::write(&path, parented).unwrap();
        let base = f.project.documents.open(&path).unwrap();
        let found = definitions(&f, position).await;
        assert_eq!(found.len(), 1, "new route receiver publication enables exclusion navigation: {found:?}");
        assert_eq!(*found[0].path, f.root.join("bootstrap/app.php"));
        f.project.documents.replace_all(base.id, plain).unwrap();
        assert!(definitions(&f, position).await.is_empty(), "unsaved parent removal must retire exclusion navigation");
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        assert!(definitions(&f, position).await.is_empty(), "restart must preserve unsaved parent removal");
        f.project.documents.replace_all(base.id, parented).unwrap();
        exclusion_hover(&f, source, r#"withoutMiddleware("au\x64"#, "App\\Http\\Middleware\\Audit").await;
        assert_eq!(std::fs::read_to_string(&path).unwrap(), parented);
        assert_eq!(std::fs::read_to_string(f.root.join("bootstrap/app.php")).unwrap(), BOOTSTRAP);
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), source);
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}
