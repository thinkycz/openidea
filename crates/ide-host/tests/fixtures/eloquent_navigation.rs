//! First-response Eloquent cast navigation through real Host commands.
use super::*;

const MODEL: &str = r"<?php
namespace App\Models;
class Bakery extends \Illuminate\Database\Eloquent\Model {
    protected $casts = ['legacy' => 'boolean'];
    protected function casts(): array {
        return ['quality' => 'float'];
    }
}
";
const FOREIGN: &str = r"<?php namespace App; class Foreign { public string $quality = 'foreign'; }
";
const CALLER: &str = r"<?php
use App\Models\Bakery as Bread;
use App\Foreign;
$bakery = new Bread();
$foreign = new Foreign();
/* 😀 cast receiver */ $bakery->quality;
/* 😀 foreign receiver */ $foreign->quality;
";

fn property(text: &str, receiver: &str, name: &str) -> Range {
    let needle = format!("{receiver}->{name}");
    let start = text.find(&needle).unwrap() + receiver.len() + 2;
    Range::new(at(text, start), at(text, start + name.len()))
}

fn declaration(model: &str, key: &str) -> Range {
    let start = model.find(&format!("'{key}'")).unwrap() + 1;
    let point = at(model, start);
    Range::new(point, point)
}

async fn definition(f: &Fixture, doc: DocId, text: &str, model: &str, key: &str) {
    let locations =
        ide_host::ops::lsp::lsp_goto(&f.host, "main", doc, property(text, "$bakery", key).start, GotoKind::Definition)
            .await
            .expect("first definition response");
    assert_eq!(locations.len(), 1, "one unambiguous cast declaration: {locations:?}");
    let location = &locations[0];
    assert!(location.uri.is_none(), "cast declaration must open a physical model");
    let path = f.root.join("app/Models/Bakery.php");
    assert_eq!(*location.path, path);
    assert_eq!(location.range, declaration(model, key), "exact UTF-16 cast key");
    let project = f.host.project("main").unwrap();
    let contents = match project.documents.by_path(&path) {
        Some(id) => project.documents.snapshot(id).unwrap().text,
        None => std::fs::read_to_string(&path).unwrap(),
    };
    assert_eq!(contents, model, "navigation must read the current declaration text");
}

async fn field(f: &Fixture, doc: DocId, text: &str, model: &str, key: &str, kind: &str, definition_first: bool) {
    assert_eq!(f.host.project("main").unwrap().documents.snapshot(doc).unwrap().text, text);
    if definition_first {
        definition(f, doc, text, model, key).await;
    }
    let hover =
        ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), doc, property(text, "$bakery", key).start)
            .await
            .expect("first hover response")
            .expect("first hover must already know the cast");
    assert_eq!(hover.range, Some(property(text, "$bakery", key)));
    assert!(hover.contents.value.contains(&format!("**var** `{kind}`")), "{}", hover.contents.value);
    if !definition_first {
        definition(f, doc, text, model, key).await;
    }
    let foreign = ide_host::ops::lsp::lsp_hover(
        &f.host,
        "main",
        RequestId::next(),
        doc,
        property(text, "$foreign", "quality").start,
    )
    .await
    .unwrap()
    .expect("ordinary declared property remains available");
    assert!(foreign.contents.value.contains("string"), "{}", foreign.contents.value);
    assert!(!foreign.contents.value.contains("source: cast"), "receiver identity must isolate cast metadata");
}

async fn removed(f: &Fixture, doc: DocId, text: &str, key: &str) {
    let position = property(text, "$bakery", key).start;
    let locations = ide_host::ops::lsp::lsp_goto(&f.host, "main", doc, position, GotoKind::Definition).await.unwrap();
    assert!(
        locations.iter().all(|target| *target.path != f.root.join("app/Models/Bakery.php")),
        "removed cast cannot navigate to a stale or unrelated cast: {locations:?}"
    );
    if let Some(hover) = ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), doc, position).await.unwrap()
    {
        assert!(!hover.contents.value.contains("source: cast"), "stale virtual cast: {}", hover.contents.value);
    }
}

fn pid(project: &Project) -> u32 {
    project.lsp.servers().into_iter().find(|server| server.name == "phpantom").unwrap().pid.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and Laravel 13.37 vendor fixture"]
async fn cast_fields_keep_first_response_types_and_exact_managed_declarations() {
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").expect("Laravel vendor fixture"))
        .canonicalize()
        .unwrap();
    let installed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(vendor.join("composer/installed.json")).unwrap()).unwrap();
    assert!(
        installed["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "laravel/framework" && p["version"] == "v13.37.0")
    );
    let dirty = MODEL.replace("'quality' => 'float'", "/* 😀 moved cast */ 'quality' => 'bool'");
    let renamed = dirty.replace("'quality' => 'bool'", "'rating' => 'integer'");
    let rating = CALLER.replace("$bakery->quality", "$bakery->rating");
    for text in [&dirty, &renamed, &rating] {
        Fixture::lint(text);
    }
    for definition_first in [false, true] {
        let extra = vec![
            (
                "composer.json".into(),
                r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#
                    .into(),
            ),
            ("app/Models/Bakery.php".into(), MODEL.into()),
            ("app/Foreign.php".into(), FOREIGN.into()),
        ];
        let (f, ()) = Fixture::new_at_with_fixture_setup(
            CALLER,
            &extra,
            "app/Probe.php",
            |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
            |_, _| (),
        );
        let mut reopened: Option<Arc<Project>> = None;
        let outcome = std::panic::AssertUnwindSafe(async {
            field(&f, f.doc, CALLER, MODEL, "quality", "float", definition_first).await;
            let initial_pid = pid(&f.project);
            let model =
                ide_host::ops::doc::doc_open(&f.host, "main", f.root.join("app/Models/Bakery.php")).await.unwrap();
            assert!(!model.read_only && !model.dirty);
            assert_eq!(model.text, MODEL);
            f.project.documents.replace_all(model.id, &dirty).unwrap();
            // Keep the caller unchanged: a caller edit must not mask a missing dependency flush.
            field(&f, f.doc, CALLER, &dirty, "quality", "bool", definition_first).await;
            assert_eq!(std::fs::read_to_string(f.root.join("app/Models/Bakery.php")).unwrap(), MODEL);
            let server = f.project.lsp.servers().into_iter().find(|server| server.name == "phpantom").unwrap().id;
            ide_host::ops::lsp::lsp_restart(&f.host, "main", server).await.unwrap();
            assert_ne!(pid(&f.project), initial_pid);
            field(&f, f.doc, CALLER, &dirty, "quality", "bool", definition_first).await;
            f.project.documents.replace_all(model.id, &renamed).unwrap();
            removed(&f, f.doc, CALLER, "quality").await;
            f.project.documents.replace_all(f.doc, &rating).unwrap();
            field(&f, f.doc, &rating, &renamed, "rating", "int", definition_first).await;
            ide_host::ops::doc::doc_close(&f.host, "main", model.id).await.unwrap();
            removed(&f, f.doc, &rating, "rating").await;
            f.project.documents.replace_all(f.doc, CALLER).unwrap();
            field(&f, f.doc, CALLER, MODEL, "quality", "float", definition_first).await;
            assert_eq!(std::fs::read_to_string(f.root.join("app/Probe.php")).unwrap(), CALLER);
            assert_eq!(std::fs::read_to_string(f.root.join("app/Models/Bakery.php")).unwrap(), MODEL);
            f.project.lsp.shutdown_all().await;
            f.host.close_project("main");
            let project = f.host.open_project("main", &f.root).unwrap();
            reopened = Some(project.clone());
            let bin = std::env::var_os("OPENSTORM_TEST_PHPANTOM_BIN").map(PathBuf::from).unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources/phpantom/phpantom_lsp")
            });
            project.lsp.set_config_hook(Box::new(move |cfg, _| {
                if cfg.name == "phpantom" {
                    cfg.command = bin.to_string_lossy().into_owned();
                }
            }));
            let caller = ide_host::ops::doc::doc_open(&f.host, "main", f.root.join("app/Probe.php")).await.unwrap();
            field(&f, caller.id, CALLER, MODEL, "quality", "float", definition_first).await;
        })
        .catch_unwind()
        .await;
        if let Some(project) = reopened {
            project.lsp.shutdown_all().await;
            assert!(project.lsp.servers().iter().all(|server| server.pid.is_none()));
        }
        f.finish(outcome).await;
    }
}

async fn effective_cast_origin(f: &Fixture, source: &str, label: &str, definition_first: bool) {
    let position = property(CALLER, "$bakery", "quality").start;
    let hover = async {
        let value = ide_host::ops::lsp::lsp_hover(&f.host, "main", RequestId::next(), f.doc, position)
            .await
            .unwrap()
            .expect("first hover knows effective cast");
        assert!(value.contents.value.contains("**var** `bool`"), "{label}: {}", value.contents.value);
    };
    let definition = async {
        let locations =
            ide_host::ops::lsp::lsp_goto(&f.host, "main", f.doc, position, GotoKind::Definition).await.unwrap();
        assert_eq!(locations.len(), 1, "{label}: {locations:?}");
        assert_eq!(*locations[0].path, f.root.join("app/Models/Bakery.php"));
        assert!(locations[0].uri.is_none());
        let marker = "/* effective */ ";
        let key = source.find(marker).unwrap() + marker.len() + 1;
        let expected = at(source, key);
        assert_eq!(locations[0].range, Range::new(expected, expected), "{label}: effective cast key");
    };
    if definition_first {
        definition.await;
        hover.await;
    } else {
        hover.await;
        definition.await;
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires pinned PHPantom, owned PHP and Laravel 13.37 vendor fixture"]
async fn cast_definitions_select_effective_literal_entries_without_text_matches() {
    let vendor = PathBuf::from(std::env::var_os("OPENSTORM_TEST_FACTORY_VENDOR").expect("Laravel vendor fixture"))
        .canonicalize()
        .unwrap();
    let variants = [
        (
            "comment key",
            "protected function casts(): array { return [/* 😀 'quality' => 'float', */ /* effective */ 'quality' => 'bool']; }",
        ),
        (
            "method overrides property",
            "protected $casts = ['quality' => 'float'];\n    protected function casts(): array { return [/* effective */ 'quality' => 'bool']; }",
        ),
        (
            "last duplicate key",
            "protected function casts(): array { return ['quality' => 'float', /* effective */ 'quality' => 'bool']; }",
        ),
        ("CRLF class bounds", "protected function casts(): array { return [/* effective */ 'quality' => 'bool']; }"),
        ("decoded key", "protected function casts(): array { return [/* effective */ \"qual\\x69ty\" => \"bool\"]; }"),
        ("legacy array", "protected function casts(): array { return array(/* effective */ 'quality' => 'bool'); }"),
    ];
    let sources: Vec<_> = variants
        .iter()
        .map(|(label, body)| {
            let preamble = if *label == "CRLF class bounds" { "// preamble\n".repeat(100) } else { String::new() };
            let source = format!(
                "<?php\n{preamble}namespace App\\Models;\nclass Bakery extends \\Illuminate\\Database\\Eloquent\\Model {{\n    {body}\n}}\n"
            );
            let source = if *label == "CRLF class bounds" { source.replace('\n', "\r\n") } else { source };
            Fixture::lint(&source);
            (*label, source)
        })
        .collect();
    for definition_first in [false, true] {
        let extra = vec![
            (
                "composer.json".into(),
                r#"{"require":{"php":"^8.3","laravel/framework":"13.37.0"},"autoload":{"psr-4":{"App\\":"app/"}}}"#
                    .into(),
            ),
            ("app/Models/Bakery.php".into(), sources[3].1.clone()),
            ("app/Foreign.php".into(), FOREIGN.into()),
        ];
        let (f, ()) = Fixture::new_at_with_fixture_setup(
            CALLER,
            &extra,
            "app/Probe.php",
            |root| std::os::unix::fs::symlink(&vendor, root.join("vendor")).unwrap(),
            |_, _| (),
        );
        let outcome = std::panic::AssertUnwindSafe(async {
            let path = f.root.join("app/Models/Bakery.php");
            assert!(f.project.documents.by_path(&path).is_none());
            effective_cast_origin(&f, &sources[3].1, "closed CRLF", definition_first).await;
            assert!(f.project.documents.by_path(&path).is_none(), "raw disk target stays unmanaged");
            let model = ide_host::ops::doc::doc_open(&f.host, "main", path.clone()).await.unwrap();
            for (label, source) in &sources {
                f.project.documents.replace_all(model.id, source).unwrap();
                effective_cast_origin(&f, source, label, definition_first).await;
                assert_eq!(f.project.documents.snapshot(f.doc).unwrap().text, CALLER);
                // Managed buffers use LF; the closed target above retains raw CRLF bytes.
                assert_eq!(f.project.documents.snapshot(model.id).unwrap().text, source.replace("\r\n", "\n"));
                assert_eq!(std::fs::read_to_string(&path).unwrap(), sources[3].1);
            }
            let negative = sources[0].1.replace("'quality' => 'bool'", "'other' => 'bool'");
            f.project.documents.replace_all(model.id, &negative).unwrap();
            removed(&f, f.doc, CALLER, "quality").await;
            let server = f.project.lsp.servers().into_iter().find(|server| server.name == "phpantom").unwrap().id;
            ide_host::ops::lsp::lsp_restart(&f.host, "main", server).await.unwrap();
            removed(&f, f.doc, CALLER, "quality").await;
            ide_host::ops::doc::doc_close(&f.host, "main", model.id).await.unwrap();
            effective_cast_origin(&f, &sources[3].1, "discard restores closed CRLF", definition_first).await;
            assert_eq!(std::fs::read_to_string(&path).unwrap(), sources[3].1);
        })
        .catch_unwind()
        .await;
        f.finish(outcome).await;
    }
}
