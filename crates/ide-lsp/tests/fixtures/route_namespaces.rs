use super::*;
use futures::FutureExt;

const OUTER: &str = r#"<?php namespace App\Http\Controllers;
class Outer {
    /* 😀 Original controller documentation. */ public function showInvoice(): string { return 'outer'; }
}
"#;
const INNER: &str = r#"<?php namespace App\Http\Controllers;
class Inner {
    /* 😀 Other controller documentation. */ public function showInvoice(): bool { return true; }
}
"#;
const ROUTES: &str = r#"<?php
use Illuminate\Support\Facades\Route;
use App\Http\Controllers\Inner;
Route::namespace('App\\Http\\Controllers')->controller('Outer')->group(function () {
    /* 😀 */ Route::namespace('Decoy')->get('/leaf-1', 'showInvoice');
    /* 😀 */ Route::namespace('\\Decoy')->get('/leaf-2', 'showInvoice');
    /* 😀 */ Route::namespace(null)->get('/leaf-3', 'showInvoice');
    /* 😀 */ Route::namespace('')->get('/leaf-4', 'showInvoice');
    /* 😀 */ Route::middleware('web')->namespace('First')->controller(Inner::class)->namespace('Last')->post('/leaf-5', 'showInvoice');
    /* 😀 */ Route::namespace('Decoy')->match(action: 'showInvoice', methods: ['get'], uri: '/leaf-6');
});
"#;

fn actions(source: &str, method: &str) -> Vec<Pos> {
    source
        .match_indices(&format!("'{method}'"))
        .map(|(offset, _)| {
            let prefix = &source[..offset + 1];
            Pos::new(
                prefix.bytes().filter(|b| *b == b'\n').count() as u32,
                prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
            )
        })
        .collect()
}

async fn check_namespace_actions(
    mgr: &LspManager,
    doc: DocId,
    root: &std::path::Path,
    source: &str,
    method: &str,
    expected_owner: (&str, &str, &str),
) -> Vec<(PathBuf, Pos)> {
    let (owner, declaration, return_type) = expected_owner;
    let positions = actions(source, method);
    assert_eq!(positions.len(), 6);
    let target_path = root.join(format!("app/Http/Controllers/{owner}.php"));
    let declaration_pos = pos(declaration, &format!("{method}()"));
    for at in &positions {
        let targets = mgr.goto(doc, *at, GotoKind::Definition).await.unwrap();
        assert_eq!(targets.len(), 1, "leaf namespace client definition: {owner}, {at:?}: {targets:?}");
        assert_eq!(*targets[0].path, target_path);
        assert_eq!(targets[0].range.start, declaration_pos);
        let list = settled_completion(mgr, doc, Pos::new(at.line, at.col + 4)).await;
        assert!(
            list.items.iter().any(|item| item.label.split('(').next() == Some(method)),
            "leaf namespace client completion: {list:?}"
        );
        let hover = mgr.hover(doc, *at, Default::default()).await.unwrap().unwrap();
        assert!(hover.contents.value.contains(return_type), "leaf namespace client hover: {hover:?}");
    }
    let refs = mgr.references(doc, positions[0], true, true, Default::default()).await.unwrap();
    assert!(refs.incomplete.is_none(), "{refs:?}");
    let mut expected: Vec<_> = positions.iter().map(|at| (root.join("routes/web.php"), *at)).collect();
    expected.push((target_path, declaration_pos));
    expected.sort_by_key(|(path, at)| (path.clone(), at.line, at.col));
    let mut actual: Vec<_> =
        refs.locations.iter().map(|location| (location.path.to_path_buf(), location.range.start)).collect();
    actual.sort_by_key(|(path, at)| (path.clone(), at.line, at.col));
    assert_eq!(actual, expected, "leaf namespace references must exclude the other controller");
    expected
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom and Laravel 13.37 vendor fixture"]
async fn namespace_actions_keep_owners_dirty_dependencies_restart_and_checked_rename() {
    let vendor = PathBuf::from(
        std::env::var_os("OPENSTORM_TEST_LARAVEL_VENDOR")
            .or_else(|| std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR"))
            .expect("Laravel vendor fixture"),
    )
    .canonicalize()
    .unwrap();
    let installed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(vendor.join("composer/installed.json")).unwrap()).unwrap();
    assert!(installed["packages"].as_array().unwrap().iter().any(|package| package["name"] == "laravel/framework"
        && matches!(package["version"].as_str(), Some("v13.37.0" | "13.37.0"))));
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    std::fs::create_dir_all(root.join("app/Http/Controllers")).unwrap();
    std::fs::create_dir(root.join("routes")).unwrap();
    std::os::unix::fs::symlink(vendor, root.join("vendor")).unwrap();
    let files = [
        (
            "composer.json",
            r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#,
        ),
        ("app/Http/Controllers/Outer.php", OUTER),
        ("app/Http/Controllers/Inner.php", INNER),
        ("routes/web.php", ROUTES),
    ];
    for (path, text) in files {
        std::fs::write(root.join(path), text).unwrap();
    }
    let engine = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
    });
    let mgr = LspManager::new(
        root.clone(),
        Arc::new(LanguageRegistry::from_configs(vec![route_handler_language(&engine)])),
        Arc::new(|_| {}),
    );
    let candidates: Vec<_> =
        files.iter().filter(|(path, _)| path.ends_with(".php")).map(|(path, _)| root.join(path)).collect();
    mgr.set_candidates_hook(Box::new(move |_, _| ide_lsp::CandidateFiles {
        files: candidates.clone(),
        limit_hit: false,
    }));
    let caller = DocId::next();
    let dependency = DocId::next();
    mgr.did_open(caller, &root.join("routes/web.php"), &LanguageId::new("php"), 1, ROUTES);
    mgr.did_open(dependency, &root.join("app/Http/Controllers/Outer.php"), &LanguageId::new("php"), 1, OUTER);
    let outcome = std::panic::AssertUnwindSafe(async {
        check_namespace_actions(&mgr, caller, &root, ROUTES, "showInvoice", ("Outer", OUTER, "string")).await;
        let dirty = OUTER
            .replacen("<?php", "<?php\n", 1)
            .replace("Original controller", "Unsaved controller")
            .replace(": string", ": int")
            .replace("'outer'", "42");
        replace(&mgr, dependency, 2, OUTER, &dirty);
        check_namespace_actions(&mgr, caller, &root, ROUTES, "showInvoice", ("Outer", &dirty, "int")).await;
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        let expected =
            check_namespace_actions(&mgr, caller, &root, ROUTES, "showInvoice", ("Outer", &dirty, "int")).await;
        let rename = mgr
            .rename(caller, actions(ROUTES, "showInvoice")[0], "renderInvoice".into(), Default::default())
            .await
            .unwrap();
        assert!(rename.incomplete.is_none(), "{rename:?}");
        let mut actual = Vec::new();
        let mut changed_caller = None;
        let mut changed_dependency = None;
        for file in &rename.edit.files {
            let ide_core::FileEdit::Edit { path, edits, .. } = file else {
                panic!("leaf namespace rename must contain only text edits")
            };
            let original = if **path == root.join("routes/web.php") {
                ROUTES
            } else {
                assert_eq!(**path, root.join("app/Http/Controllers/Outer.php"));
                &dirty
            };
            let mut changed = ide_text::Document::new(original);
            changed.apply_edits(edits).unwrap();
            for edit in edits {
                assert_eq!(edit.new_text, "renderInvoice");
                actual.push((path.to_path_buf(), edit.range.start));
            }
            if **path == root.join("routes/web.php") {
                changed_caller = Some(changed.text());
            } else {
                changed_dependency = Some(changed.text());
            }
        }
        actual.sort_by_key(|(path, at)| (path.clone(), at.line, at.col));
        assert_eq!(actual, expected, "checked rename edits must cover only six actions and their current owner");
        let changed_caller = changed_caller.unwrap();
        let changed_dependency = changed_dependency.unwrap();
        assert_eq!(changed_caller, ROUTES.replace("'showInvoice'", "'renderInvoice'"));
        assert_eq!(changed_dependency, dirty.replace("showInvoice", "renderInvoice"));
        replace(&mgr, dependency, 3, &dirty, &changed_dependency);
        replace(&mgr, caller, 2, ROUTES, &changed_caller);
        check_namespace_actions(
            &mgr,
            caller,
            &root,
            &changed_caller,
            "renderInvoice",
            ("Outer", &changed_dependency, "int"),
        )
        .await;
        replace(&mgr, caller, 3, &changed_caller, ROUTES);
        mgr.did_close(dependency);
        check_namespace_actions(&mgr, caller, &root, ROUTES, "showInvoice", ("Outer", OUTER, "string")).await;
        let rebound = ROUTES.replace("controller('Outer')", "controller('Inner')");
        replace(&mgr, caller, 4, ROUTES, &rebound);
        check_namespace_actions(&mgr, caller, &root, &rebound, "showInvoice", ("Inner", INNER, "bool")).await;
        mgr.restart(mgr.servers()[0].id).await.unwrap();
        check_namespace_actions(&mgr, caller, &root, &rebound, "showInvoice", ("Inner", INNER, "bool")).await;
        mgr.did_close(caller);
        let reopened = DocId::next();
        mgr.did_open(reopened, &root.join("routes/web.php"), &LanguageId::new("php"), 1, ROUTES);
        check_namespace_actions(&mgr, reopened, &root, ROUTES, "showInvoice", ("Outer", OUTER, "string")).await;
    })
    .catch_unwind()
    .await;
    mgr.shutdown_all().await;
    for (path, text) in files {
        assert_eq!(
            std::fs::read_to_string(root.join(path)).unwrap(),
            text,
            "client operations must preserve all originals"
        );
    }
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
