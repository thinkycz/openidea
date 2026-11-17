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
    Fixture::new_at(
        CALLER,
        &[
            ("composer.json".into(), r#"{"require":{"laravel/framework":"^10.0"},"autoload":{"psr-4":{"App\\":"app/","Illuminate\\":"framework/"}}}"#.into()),
            ("app/Http/Kernel.php".into(), kernel.into()),
            ("app/Http/KernelBase.php".into(), BASE.into()),
            ("app/Http/Middleware/Audit.php".into(), CLASS.into()),
            ("app/Http/Middleware/Trace.php".into(), CLASS.replace("Audit", "Trace")),
            ("framework/Foundation/Http/Kernel.php".into(), "<?php namespace Illuminate\\Foundation\\Http; class Kernel {}\n".into()),
        ],
        "routes/web.php",
    )
}

fn cursor(text: &str, needle: &str) -> Pos {
    at(text, text.find(needle).unwrap())
}

async fn definitions(f: &Fixture) -> Vec<ide_core::Location> {
    ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, cursor(CALLER, r"64ited"), GotoKind::Definition).await.unwrap()
}

async fn hover(f: &Fixture, expected: &str) {
    let result = ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, cursor(CALLER, r"64ited"))
        .await
        .unwrap()
        .expect("Kernel middleware hover");
    assert!(result.contents.value.contains(expected), "{result:?}");
    assert!(result.contents.value.contains("app/Http/Kernel.php"), "{result:?}");
    assert!(!result.contents.value.contains("bootstrap/app.php"), "{result:?}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom164 and owned PHP syntax-check executable"]
async fn kernel_alias_navigation_tracks_dirty_ancestry_ranges_and_restart() {
    for property in ["middlewareAliases", "routeMiddleware"] {
        let original = KERNEL.replace("middlewareAliases", property);
        let f = fixture(&original);
        let outcome = std::panic::AssertUnwindSafe(async {
            let kernel = f.root.join("app/Http/Kernel.php");
            let parent = f.root.join("app/Http/KernelBase.php");
            let ready =
                ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, cursor(CALLER, "Audit;"), GotoKind::Definition)
                    .await
                    .unwrap();
            assert!(!ready.is_empty(), "ordinary PHP readiness");
            let found = definitions(&f).await;
            assert_eq!(found.len(), 1, "managed Kernel alias definition: {found:?}");
            assert_eq!(*found[0].path, kernel);
            assert_eq!(found[0].range.start, cursor(&original, r"au\x64ited"));
            assert_eq!(found[0].range.end, found[0].range.start);
            hover(&f, "App\\Http\\Middleware\\Audit").await;
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
            for (path, text) in [(&kernel, original.as_str()), (&f.root.join("routes/web.php"), CALLER)] {
                let location = refs.locations.iter().find(|location| *location.path == *path).unwrap();
                assert_eq!(location.range.start, cursor(text, r"au\x64ited"));
                assert_eq!(location.range.end.col - location.range.start.col, 10);
            }
            let doc = f.project.documents.open(&kernel).unwrap();
            for (id, text, needle) in [(f.doc, CALLER, r"64ited"), (doc.id, original.as_str(), r"64ited")] {
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
            hover(&f, "App\\Http\\Middleware\\Trace").await;
            let parent_doc = f.project.documents.open(&parent).unwrap();
            f.project.documents.replace_all(parent_doc.id, "<?php namespace App\\Http; class KernelBase {}\n").unwrap();
            assert!(definitions(&f).await.is_empty(), "dirty parent removes Kernel ancestry");
            let restored = format!("{BASE}// Unsaved Kernel parent documentation.\n");
            f.project.documents.replace_all(parent_doc.id, &restored).unwrap();
            assert_eq!(definitions(&f).await.len(), 1);
            f.project.lsp.restart(f.project.lsp.servers()[0].id).await.unwrap();
            hover(&f, "App\\Http\\Middleware\\Trace").await;
            let renamed = changed.replace(r"au\x64ited", "different");
            f.project.documents.replace_all(doc.id, &renamed).unwrap();
            assert!(definitions(&f).await.is_empty(), "dirty alias removes the old key");
            assert_eq!(std::fs::read_to_string(&kernel).unwrap(), original);
            assert_eq!(std::fs::read_to_string(&parent).unwrap(), BASE);
            assert_eq!(std::fs::read_to_string(f.root.join("routes/web.php")).unwrap(), CALLER);
            assert_eq!(f.project.documents.snapshot(parent_doc.id).unwrap().text, restored);
        })
        .catch_unwind()
        .await;
        f.finish(outcome).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires PHPantom164 and owned PHP syntax-check executable"]
async fn kernel_alias_checked_rename_preserves_conflicts_dirty_buffers_and_history() {
    let f = fixture(KERNEL);
    let outcome = std::panic::AssertUnwindSafe(async {
        let old = f.root.join("app/Http/Middleware/Audit.php");
        let new = f.root.join("app/Http/Middleware/Auditor.php");
        let kernel = f.root.join("app/Http/Kernel.php");
        let class = f.project.documents.open(&old).unwrap();
        let registration = f.project.documents.open(&kernel).unwrap();
        let overlay = format!("{CLASS}// Unsaved middleware documentation.\n");
        f.project.documents.replace_all(class.id, &overlay).unwrap();
        let position = cursor(CLASS, "Audit {}");
        for value in [r"['audited'=>'App\Http\Middleware\Audit']", "ALIASES", "['audited'=>AliasTargets::AUDIT]"] {
            let changed = KERNEL.replace(r#"[/* 😀 */ "au\x64ited" => AuditAlias::class]"#, value);
            assert_ne!(changed, KERNEL);
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
            assert_eq!(std::fs::read_to_string(&kernel).unwrap(), KERNEL);
            assert!(!new.exists());
        }
        f.project.documents.replace_all(registration.id, KERNEL).unwrap();
        let stale =
            ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, position, "Auditor".into())
                .await
                .unwrap()
                .edit;
        f.project.documents.replace_all(registration.id, &format!("{KERNEL}// Later buffer edit.\n")).unwrap();
        assert!(ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", stale).await.is_err());
        assert_eq!(f.project.documents.snapshot(class.id).unwrap().text, overlay);
        assert_eq!(std::fs::read_to_string(&old).unwrap(), CLASS);
        assert!(!new.exists());
        f.project.documents.replace_all(registration.id, KERNEL).unwrap();
        let stale =
            ide_host::ops::lsp::lsp_rename(&f.host, "main", RequestId::next(), class.id, position, "Auditor".into())
                .await
                .unwrap()
                .edit;
        let external = format!("{KERNEL}// External Kernel change.\n");
        std::fs::write(&kernel, &external).unwrap();
        assert!(ide_host::ops::lsp::apply_workspace_edit(&f.host, "main", stale).await.is_err());
        assert_eq!(std::fs::read_to_string(&kernel).unwrap(), external);
        assert_eq!(f.project.documents.snapshot(class.id).unwrap().text, overlay);
        assert_eq!(std::fs::read_to_string(&old).unwrap(), CLASS);
        assert!(!new.exists());
        std::fs::write(&kernel, KERNEL).unwrap();
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
            assert_eq!(std::fs::read_to_string(&kernel).unwrap(), if redo { updated.text.as_str() } else { KERNEL });
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
