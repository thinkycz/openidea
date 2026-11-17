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

fn exclusion_files() -> Vec<(String, String)> {
    let mut files = fixture_files();
    files.extend([
        ("framework/Routing/Route.php".into(), r#"<?php namespace Illuminate\Routing; class Route {public function withoutMiddleware($middleware): static {return $this;}}"#.into()),
        ("framework/Routing/RouteRegistrar.php".into(), r#"<?php namespace Illuminate\Routing; /** @method \Illuminate\Routing\RouteRegistrar withoutMiddleware(array|string $middleware) */ class RouteRegistrar {public function __call(string $method, array $arguments): static {return $this;}}"#.into()),
        ("framework/Routing/Router.php".into(), r#"<?php namespace Illuminate\Routing; /** @method \Illuminate\Routing\RouteRegistrar withoutMiddleware(array|string $middleware) */ class Router {public function __call(string $method, array $arguments): RouteRegistrar {return new RouteRegistrar;}}"#.into()),
        ("framework/Routing/PendingResourceRegistration.php".into(), r#"<?php namespace Illuminate\Routing; class PendingResourceRegistration {public function withoutMiddleware($middleware): static {return $this;}}"#.into()),
        ("framework/Routing/PendingSingletonResourceRegistration.php".into(), r#"<?php namespace Illuminate\Routing; class PendingSingletonResourceRegistration {public function withoutMiddleware($middleware): static {return $this;}}"#.into()),
        ("framework/Support/Facades/Route.php".into(), r#"<?php namespace Illuminate\Support\Facades; class Route {public static function get($uri,$action): \Illuminate\Routing\Route {return new \Illuminate\Routing\Route;} public static function prefix($prefix): \Illuminate\Routing\RouteRegistrar {return new \Illuminate\Routing\RouteRegistrar;}}"#.into()),
    ]);
    files
}

fn exclusion_fixture(source: &str) -> Fixture {
    Fixture::new_at(source, &exclusion_files(), "routes/web.php")
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

fn resource_exclusion_fixture(source: &str) -> Fixture {
    let mut files = exclusion_files();
    for (name, text) in &mut files {
        if matches!(
            name.as_str(),
            "framework/Routing/PendingResourceRegistration.php"
                | "framework/Routing/PendingSingletonResourceRegistration.php"
        ) {
            text.insert_str(
                text.rfind('}').unwrap(),
                " public function withoutMiddlewareFor($methods,$middleware): static {return $this;}",
            );
        }
        if name == "framework/Support/Facades/Route.php" {
            text.insert_str(text.rfind('}').unwrap(), " public static function resource($name,$controller): \\Illuminate\\Routing\\PendingResourceRegistration {return new \\Illuminate\\Routing\\PendingResourceRegistration;} public static function singleton($name,$controller): \\Illuminate\\Routing\\PendingSingletonResourceRegistration {return new \\Illuminate\\Routing\\PendingSingletonResourceRegistration;}");
        }
    }
    files.extend([
        ("config/auth.php".into(), "<?php return ['guards'=>['admin'=>[], 'staff'=>[]]];".into()),
        (
            "app/Http/Controllers/ReportController.php".into(),
            "<?php namespace App\\Http\\Controllers; class ReportController {public function show() {return null;}}"
                .into(),
        ),
    ]);
    Fixture::new_at(source, &files, "routes/web.php")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom162 and owned PHP syntax-check executable"]
async fn resource_exclusions_binding_ranges_and_unsaved_registration_survive_restart() {
    let source = r#"<?php
use Illuminate\Support\Facades\Route;
new \App\Http\Middleware\Audit;
\Illuminate\Support\Facades\Gate::define('edit-report',fn()=>true);
function excluded(\Illuminate\Routing\PendingResourceRegistration $resource, \Illuminate\Routing\PendingSingletonResourceRegistration $singleton, \Illuminate\Routing\PendingResourceRegistration|\Illuminate\Routing\PendingSingletonResourceRegistration|null $pair, \Illuminate\Routing\Route $route, array $arguments) {
 /* 😀 */ $resource->withoutMiddlewareFor('show', "au\x64ited:encoded");
 $resource->withoutMiddlewareFor(methods: 'show', middleware: 'audited:named');
 $resource->withoutMiddlewareFor(middleware: 'audited:reordered', methods: ['show']);
 $resource->WITHOUTMIDDLEWAREFOR('show', 'audited:case');
 $singleton->withoutMiddlewareFor(['show'], 'audited:singleton');
 $pair?->withoutMiddlewareFor('show', 'audited:union');
 $resource->withoutMiddlewareFor('audited:selector', 'audited:extra', 'audited:ignored');
 $resource->withoutMiddlewareFor(middleware: 'audited:missing');
 $resource->withoutMiddlewareFor('show', 'audited:duplicate', middleware: 'audited:ignored');
 $resource->withoutMiddlewareFor('show', middleware: 'audited:unknown', unknown: true);
 $resource->withoutMiddlewareFor(...$arguments, middleware: 'audited:unpacked');
 $route->withoutMiddlewareFor('show', 'audited:foreign');
 $resource->withoutMiddlewareFor(['show'], middleware: ([...["auth:ad\x6din", 'can:edit-report']]));
}
(new \Illuminate\Routing\PendingResourceRegistration)->withoutMiddlewareFor('show', 'audited:constructed');
Route::resource('reports', \App\Http\Controllers\ReportController::class)->withoutMiddlewareFor('show', 'audited:fluent-resource');
Route::singleton('report', \App\Http\Controllers\ReportController::class)->withoutMiddlewareFor('show', 'audited:fluent-singleton');
"#;
    let f = resource_exclusion_fixture(source);
    let outcome = std::panic::AssertUnwindSafe(async {
        let boot = f.root.join("bootstrap/app.php");
        assert!(!definitions(&f, cursor(source, "Audit;")).await.is_empty(), "ordinary PHP readiness");
        let consumed = [
            r#"withoutMiddlewareFor('show', "au\x64"#,
            "withoutMiddlewareFor(methods: 'show', middleware: 'aud",
            "withoutMiddlewareFor(middleware: 'aud",
            "WITHOUTMIDDLEWAREFOR('show', 'aud",
            "$singleton->withoutMiddlewareFor(['show'], 'aud",
            "$pair?->withoutMiddlewareFor('show', 'aud",
            "withoutMiddlewareFor('audited:selector', 'aud",
            "Registration)->withoutMiddlewareFor('show', 'aud",
            "Route::resource('reports', \\App\\Http\\Controllers\\ReportController::class)->withoutMiddlewareFor('show', 'aud",
            "Route::singleton('report', \\App\\Http\\Controllers\\ReportController::class)->withoutMiddlewareFor('show', 'aud",
        ];
        for prefix in consumed {
            let found = definitions(&f, exclusion_position(source, prefix)).await;
            assert_eq!(found.len(), 1, "resource middleware definition {prefix}: {found:?}");
            assert_eq!(*found[0].path, boot);
            assert_eq!(found[0].range.start, cursor(BOOTSTRAP, r"au\x64ited"));
            assert_eq!(found[0].range.end, found[0].range.start);
        }
        for prefix in [
            "withoutMiddlewareFor('aud",
            "withoutMiddlewareFor('audited:selector', 'audited:extra', 'aud",
            "withoutMiddlewareFor(middleware: 'audited:mis",
            "withoutMiddlewareFor('show', 'audited:dup",
            "withoutMiddlewareFor('show', 'audited:duplicate', middleware: 'aud",
            "withoutMiddlewareFor('show', middleware: 'audited:unk",
            "withoutMiddlewareFor(...$arguments, middleware: 'aud",
            "$route->withoutMiddlewareFor('show', 'aud",
        ] {
            assert!(definitions(&f, exclusion_position(source, prefix)).await.is_empty(), "unconsumed resource {prefix}");
        }
        let guard = definitions(&f, exclusion_position(source, r"auth:ad\x6d")).await;
        assert_eq!(guard.len(), 1, "resource auth guard definition: {guard:?}");
        assert_eq!(*guard[0].path, f.root.join("config/auth.php"));
        let ability = definitions(&f, exclusion_position(source, "can:edit-rep")).await;
        assert_eq!(ability.len(), 1, "resource ability definition: {ability:?}");
        assert_eq!(*ability[0].path, f.root.join("routes/web.php"));
        let position = exclusion_position(source, consumed[0]);
        exclusion_hover(&f, source, consumed[0], "App\\Http\\Middleware\\Audit").await;
        let refs = ide_host::ops::lsp::lsp_references(
            &f.host, "main", RequestId::next(), f.doc, position, true, true, true,
            ide_host::ops::Channel::new(|_| {}),
        ).await.unwrap();
        assert!(refs.incomplete.is_none(), "{refs:?}");
        assert_eq!(refs.locations.len(), consumed.len() + 1, "only consumed resource aliases are references: {refs:?}");
        let encoded = refs.locations.iter().find(|location| *location.path == f.root.join("routes/web.php") && location.range.start == cursor(source, r"au\x64ited")).expect("encoded resource reference");
        assert_eq!(encoded.range.end.col - encoded.range.start.col, 10);
        assert!(f.project.lsp.prepare_rename_details(f.doc, position).await.unwrap().is_none(), "resource alias-key rename must refuse partial edits");
        let bootstrap = f.project.documents.open(&boot).unwrap();
        let target = BOOTSTRAP.replace("Middleware\\Audit as", "Middleware\\Trace as");
        assert_eq!(target.len(), BOOTSTRAP.len());
        f.project.documents.replace_all(bootstrap.id, &target).unwrap();
        exclusion_hover(&f, source, consumed[0], "App\\Http\\Middleware\\Trace").await;
        let missing = target.replace(r"au\x64ited", r"mi\x73sing");
        assert_eq!(missing.len(), target.len());
        f.project.documents.replace_all(bootstrap.id, &missing).unwrap();
        assert!(definitions(&f, position).await.is_empty(), "dirty resource alias registration must retire navigation");
        f.project.documents.replace_all(bootstrap.id, &target).unwrap();
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        exclusion_hover(&f, source, consumed[0], "App\\Http\\Middleware\\Trace").await;
        assert_eq!(std::fs::read_to_string(&boot).unwrap(), BOOTSTRAP);
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), source);
    }).catch_unwind().await;
    f.finish(outcome).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom162 and owned PHP syntax-check executable"]
async fn resource_exclusions_native_availability_and_unsaved_overrides_survive_restart() {
    let source = r#"<?php
new \App\Http\Middleware\Audit;
function remove(\App\ResourceChild $resource) {$resource->withoutMiddlewareFor('show', "au\x64ited");}
"#;
    let f = resource_exclusion_fixture(source);
    let outcome = std::panic::AssertUnwindSafe(async {
        assert!(!definitions(&f, cursor(source, "Audit;")).await.is_empty(), "ordinary PHP readiness");
        let position = exclusion_position(source, r#"withoutMiddlewareFor('show', "au\x64"#);
        assert!(definitions(&f, position).await.is_empty(), "unknown resource receiver must not invent navigation");
        let inherited = "<?php namespace App; class ResourceChild extends \\Illuminate\\Routing\\PendingResourceRegistration {}\n";
        let custom = "<?php namespace App; class ResourceChild extends \\Illuminate\\Routing\\PendingResourceRegistration {public function withoutMiddlewareFor($methods,$middleware): static {return $this;}}\n";
        Fixture::lint(inherited);
        Fixture::lint(custom);
        let path = f.root.join("app/ResourceChild.php");
        std::fs::write(&path, inherited).unwrap();
        let child = f.project.documents.open(&path).unwrap();
        let found = definitions(&f, position).await;
        assert_eq!(found.len(), 1, "published resource receiver must enable navigation: {found:?}");
        assert_eq!(*found[0].path, f.root.join("bootstrap/app.php"));
        f.project.documents.replace_all(child.id, custom).unwrap();
        assert!(definitions(&f, position).await.is_empty(), "custom resource origin must retire navigation");
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        assert!(definitions(&f, position).await.is_empty(), "restart must retain the dirty custom origin");
        f.project.documents.replace_all(child.id, inherited).unwrap();
        assert_eq!(definitions(&f, position).await.len(), 1);
        let native_path = f.root.join("framework/Routing/PendingResourceRegistration.php");
        let native = f.project.documents.open(&native_path).unwrap();
        let original = f.project.documents.snapshot(native.id).unwrap().text;
        let missing = r"<?php namespace Illuminate\Routing; class PendingResourceRegistration {public function withoutMiddleware($middleware): static {return $this;}}";
        Fixture::lint(missing);
        f.project.documents.replace_all(native.id, missing).unwrap();
        assert!(definitions(&f, position).await.is_empty(), "missing native method must retire resource semantics");
        f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
        assert!(definitions(&f, position).await.is_empty(), "restart must retain dirty native-method absence");
        f.project.documents.replace_all(native.id, &original).unwrap();
        exclusion_hover(&f, source, r#"withoutMiddlewareFor('show', "au\x64"#, "App\\Http\\Middleware\\Audit").await;
        assert_eq!(std::fs::read_to_string(&path).unwrap(), inherited);
        assert_eq!(std::fs::read_to_string(&native_path).unwrap(), original);
        assert_eq!(std::fs::read_to_string(f.root.join("bootstrap/app.php")).unwrap(), BOOTSTRAP);
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), source);
    }).catch_unwind().await;
    f.finish(outcome).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom163, Python3 and owned PHP syntax-check executable"]
async fn managed_stdio_releases_foreign_writer_and_preserves_dirty_restart() {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;

    let (f, (mut reader, expected_engine)) =
        Fixture::new_at_with_setup(CALLER, &fixture_files(), "routes/web.php", |project, scratch| {
            let fifo = scratch.join("foreign-writer.fifo");
            assert!(
                std::process::Command::new("/usr/bin/mkfifo")
                    .args(["-m", "600"])
                    .arg(&fifo)
                    .status()
                    .unwrap()
                    .success()
            );
            let reader = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32)
                .open(&fifo)
                .unwrap();
            let witness = scratch.join("stdio-launches.jsonl");
            let wrapper = scratch.join("inject_writer.py");
            std::fs::write(
                &wrapper,
                r#"import fcntl, json, os, sys
engine, fifo, witness = sys.argv[1:4]
writer = os.open(fifo, os.O_WRONLY | os.O_NONBLOCK)
foreign = fcntl.fcntl(writer, fcntl.F_DUPFD, 128)
os.close(writer)
os.set_inheritable(foreign, True)
with open(witness, "a", encoding="utf-8") as output:
    output.write(json.dumps({"pid": os.getpid(), "fd": foreign, "engine": engine}) + "\n")
os.execv(engine, [engine, *sys.argv[4:]])
"#,
            )
            .unwrap();
            let engine = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
            });
            let engine = dunce::canonicalize(engine).unwrap();
            let expected_engine = engine.to_string_lossy().into_owned();
            project.lsp.set_config_hook(Box::new(move |cfg, _| {
                if cfg.name == "phpantom" {
                    let mut arguments = vec![
                        wrapper.to_string_lossy().into_owned(),
                        engine.to_string_lossy().into_owned(),
                        fifo.to_string_lossy().into_owned(),
                        witness.to_string_lossy().into_owned(),
                    ];
                    arguments.append(&mut cfg.args);
                    cfg.command = "/usr/bin/python3".into();
                    cfg.args = arguments;
                }
            }));
            (reader, expected_engine)
        });
    let outcome = std::panic::AssertUnwindSafe(async {
        let boot = f.root.join("bootstrap/app.php");
        let mut pids = Vec::new();
        for launch in 0..2 {
            assert!(!definitions(&f, cursor(CALLER, "Audit;")).await.is_empty(), "ordinary PHP readiness");
            let found = definitions(&f, cursor(CALLER, r"64ited")).await;
            assert_eq!(found.len(), 1, "application alias after managed launch: {found:?}");
            assert_eq!(*found[0].path, boot);
            let launches = std::fs::read_to_string(f._tmp.path().join("stdio-launches.jsonl")).unwrap();
            let launches: Vec<serde_json::Value> =
                launches.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
            assert_eq!(launches.len(), launch + 1);
            let pid = f.project.lsp.servers()[0].pid.expect("managed server must remain alive");
            let last = launches.last().unwrap();
            assert_eq!(last["pid"].as_u64(), Some(u64::from(pid)), "wrapper must exec the managed server");
            assert_eq!(last["engine"].as_str(), Some(expected_engine.as_str()));
            assert!(last["fd"].as_u64().unwrap() >= 128);
            pids.push(pid);
            let mut byte = [0];
            let eof = reader.read(&mut byte);
            eprintln!("managed stdio launch {launch}: pid={pid} foreign_fd={} read={eof:?}", last["fd"]);
            assert!(matches!(eof, Ok(0)), "managed stdio server retained inherited writer after readiness: {eof:?}");
            target(&f, if launch == 0 { "App\\Http\\Middleware\\Audit" } else { "App\\Http\\Middleware\\Trace" }).await;
            if launch == 0 {
                let doc = f.project.documents.open(&boot).unwrap();
                f.project
                    .documents
                    .replace_all(doc.id, &BOOTSTRAP.replace("Middleware\\Audit as", "Middleware\\Trace as"))
                    .unwrap();
                target(&f, "App\\Http\\Middleware\\Trace").await;
                f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
            }
        }
        assert_ne!(pids[0], pids[1], "restart must launch a fresh managed process");
        assert_eq!(std::fs::read_to_string(&boot).unwrap(), BOOTSTRAP);
        assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), CALLER);
        f.project.lsp.shutdown_all().await;
        for raw in pids {
            let pid = rustix::process::Pid::from_raw(raw.try_into().unwrap()).unwrap();
            for _ in 0..100 {
                if rustix::process::test_kill_process(pid) == Err(rustix::io::Errno::SRCH) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            assert_eq!(
                rustix::process::test_kill_process(pid),
                Err(rustix::io::Errno::SRCH),
                "managed engine process must exit after shutdown"
            );
            eprintln!("managed stdio pid={raw} exited");
        }
    })
    .catch_unwind()
    .await;
    f.finish(outcome).await;
}
