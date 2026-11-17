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

fn fixture() -> Fixture {
    Fixture::new_at(
        CALLER,
        &[
            ("composer.json".into(), r#"{"require":{"laravel/framework":"^13.0"},"autoload":{"psr-4":{"App\\":"app/","Illuminate\\":"framework/"}}}"#.into()),
            ("bootstrap/app.php".into(), BOOTSTRAP.into()),
            ("app/Http/Middleware/Audit.php".into(), CLASS.into()),
            ("app/Http/Middleware/Trace.php".into(), CLASS.replace("Audit", "Trace")),
            ("framework/Foundation/Application.php".into(), "<?php namespace Illuminate\\Foundation; class Application {public static function configure(): \\Illuminate\\Foundation\\Configuration\\ApplicationBuilder {return new \\Illuminate\\Foundation\\Configuration\\ApplicationBuilder;}}".into()),
            ("framework/Foundation/Configuration/ApplicationBuilder.php".into(), "<?php namespace Illuminate\\Foundation\\Configuration; class ApplicationBuilder {public function withMiddleware(callable $callback): static {return $this;} public function create(): \\Illuminate\\Foundation\\Application {return new \\Illuminate\\Foundation\\Application;}}".into()),
            ("framework/Foundation/Configuration/Middleware.php".into(), "<?php namespace Illuminate\\Foundation\\Configuration; class Middleware {public function alias(array $aliases): static {return $this;}}".into()),
            ("framework/Routing/Controller.php".into(), "<?php namespace Illuminate\\Routing; class Controller {protected function middleware($middleware): static {return $this;}}".into()),
        ],
        "routes/web.php",
    )
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
