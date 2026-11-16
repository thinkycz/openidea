//! Controller URL literals through the host's checked edit and reviewed history paths.
use ide_core::{FileEdit, IdeEvent, Pos, RequestId};
use ide_host::{Host, HostConfig};
use std::{path::PathBuf, sync::Arc};

fn at(text: &str, needle: &str, inside: usize) -> Pos {
    let prefix = &text[..text.find(needle).unwrap() + inside];
    Pos::new(
        prefix.bytes().filter(|b| *b == b'\n').count() as u32,
        prefix.rsplit('\n').next().unwrap().encode_utf16().count() as u32,
    )
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires candidate155 and the retained Laravel/PHP runtime fixture"]
async fn controller_url_class_rename_preserves_overlays_runtime_and_history() {
    check_url_rename(true).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires candidate155 and the retained Laravel/PHP runtime fixture"]
async fn controller_url_method_rename_preserves_overlays_runtime_and_history() {
    check_url_rename(false).await;
}

async fn check_url_rename(class_rename: bool) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let engine = std::env::var_os("OPENSTORM_TEST_URL_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("artifacts/engines/0.11.1-openstorm.155/phpantom_lsp"));
    let laravel = std::env::var_os("OPENSTORM_TEST_URL_LARAVEL_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("artifacts/checks/engine-laravel13-37-155/project"));
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("project");
    std::fs::create_dir_all(root.join("src")).unwrap();
    let root = dunce::canonicalize(root).unwrap();
    std::fs::write(
        root.join("composer.json"),
        r#"{"require":{"laravel/framework":"^13.37"},"autoload":{"psr-4":{"App\\":"src/"}}}"#,
    )
    .unwrap();
    let old = root.join("src/Invoice.php");
    let new = if class_rename { root.join("src/Receipt.php") } else { old.clone() };
    let caller = root.join("src/Links.php");
    let class = "<?php\nnamespace App;\nclass Invoice { /** Display an invoice. */ public function show(): string { return 'invoice'; } }\n";
    let source = r#"<?php
namespace App;
use App\Invoice as LocalInvoice;
use Illuminate\Support\Facades\URL;
use function action as controller_url;
$ordinary = new LocalInvoice();
$direct = $ordinary->show();
$label = '😀'; $one = \action('\App\Invoice@show');
$two = controller_url(name: "\\App\\Invoice@show");
$three = URL::action(action: '\App\Invoice@show');
$four = \redirect()->action(action: '\App\Invoice@show');
$plain = '\App\Invoice@show';
"#;
    std::fs::write(&old, class).unwrap();
    std::fs::write(&caller, source).unwrap();
    let php_cli = repo.join("artifacts/toolchains/php-8.4.23-static/php");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("composer.json")).unwrap()).unwrap();
    assert_eq!(manifest["autoload"]["psr-4"]["App\\"], "src/");
    for path in [&old, &caller] {
        let check = std::process::Command::new(&php_cli).args(["-n", "-l"]).arg(path).output().unwrap();
        assert!(check.status.success(), "fixture PHP syntax: {}", String::from_utf8_lossy(&check.stderr));
    }
    let events = Arc::new(std::sync::Mutex::new(Vec::<IdeEvent>::new()));
    let capture = events.clone();
    let mut php = ide_language::LanguageRegistry::load(None).unwrap().get(&ide_core::LanguageId::new("php")).unwrap();
    php.language_servers.retain(|s| s.name == "phpantom");
    let host = Host::new(HostConfig {
        project_environment: false,
        config_dir: tmp.path().join("config"),
        data_dir: tmp.path().join("data"),
        local_data_dir: tmp.path().join("local-data"),
        languages: Arc::new(ide_language::LanguageRegistry::from_configs(vec![php])),
        syntax: Arc::default(),
        sink_for: Box::new(move |_| {
            let events = capture.clone();
            Arc::new(move |e| events.lock().unwrap().push(e))
        }),
        owner: |w| w,
        runtime: tokio::runtime::Handle::current(),
    });
    host.trust.decide(&root, true, false).unwrap();
    let project = host.open_project("main", &root).unwrap();
    project.lsp.set_config_hook(Box::new(move |cfg, _| cfg.command = engine.to_string_lossy().into_owned()));
    let result: Result<(), String> = async {
        let declaration = project.documents.open(&old).unwrap();
        let links = project.documents.open(&caller).unwrap();
        let overlay_class = class.replace("'invoice'", "'overlay'");
        let overlay_source = format!("{source}$five = \\url()->action(action: '\\App\\Invoice@show');\n");
        project.documents.replace_all(declaration.id, &overlay_class).unwrap();
        project.documents.replace_all(links.id, &overlay_source).unwrap();
        let position = if class_rename { at(source, "Invoice@show", 3) } else { at(source, "show');", 1) };
        let prepared = project
            .lsp
            .prepare_rename_details(links.id, position)
            .await
            .map_err(|e| format!("prepare URL rename: {e}"))?
            .ok_or("URL literal did not offer Rename")?;
        let placeholder = prepared.placeholder.as_deref().unwrap_or_default();
        if !(if class_rename { placeholder.ends_with("Invoice") } else { placeholder == "show" }) {
            return Err(format!("wrong URL rename target: {prepared:?}"));
        }
        let line = source.lines().nth(position.line as usize).unwrap().encode_utf16().collect::<Vec<_>>();
        if prepared.range.start.line != position.line || prepared.range.end.line != position.line {
            return Err(format!("URL prepare range left its source line: {prepared:?}"));
        }
        let raw = line
            .get(prepared.range.start.col as usize..prepared.range.end.col as usize)
            .ok_or("URL prepare range exceeds the UTF-16 source line")?;
        let raw = String::from_utf16(raw).map_err(|e| e.to_string())?;
        if if class_rename { !raw.ends_with("Invoice") || raw.contains(['@', '\'', '"']) } else { raw != "show" } {
            return Err(format!(
                "URL prepare range includes quotes, separator or wrong UTF-16 span: {prepared:?}, {raw:?}"
            ));
        }
        let replacement = if class_rename { "Receipt" } else { "display" };
        for uncertain in [r"'Invoice@show'", r#""\\App\\Invoice@sh\x6fw""#] {
            let ambiguous = format!("{overlay_source}$ambiguous = \\action({uncertain});\n");
            let syntax = tmp.path().join("syntax.php");
            std::fs::write(&syntax, &ambiguous).unwrap();
            let check = std::process::Command::new(&php_cli).args(["-n", "-l"]).arg(&syntax).output().unwrap();
            if !check.status.success() {
                return Err(format!("uncertain-literal fixture syntax: {}", String::from_utf8_lossy(&check.stderr)));
            }
            project.documents.replace_all(links.id, &ambiguous).unwrap();
            let refused = ide_host::ops::lsp::lsp_rename(
                &host,
                "main",
                RequestId::next(),
                links.id,
                position,
                replacement.into(),
            )
            .await;
            let reason = if class_rename { "runtime controller binding" } else { "runtime controller method" };
            if !refused.as_ref().is_err_and(|error| error.message.contains(reason))
                || std::fs::read_to_string(&old).unwrap() != class
                || std::fs::read_to_string(&caller).unwrap() != source
                || project.documents.snapshot(links.id).unwrap().text != ambiguous
            {
                return Err(format!("uncertain URL binding was partially renamed: {uncertain}: {refused:?}"));
            }
        }
        project.documents.replace_all(links.id, &overlay_source).unwrap();
        let plan =
            ide_host::ops::lsp::lsp_rename(&host, "main", RequestId::next(), links.id, position, replacement.into())
                .await
                .map_err(|e| format!("checked URL rename: {e}"))?
                .edit;
        if class_rename
            && !plan.files.iter().any(|f| matches!(f, FileEdit::Rename { from, to } if **from == old && **to == new))
        {
            return Err(format!("URL class rename omitted PSR-4 move: {plan:?}"));
        }
        if std::fs::read_to_string(&old).unwrap() != class || std::fs::read_to_string(&caller).unwrap() != source {
            return Err("preview wrote files".into());
        }
        let external = format!("{source}// External caller change.\n");
        std::fs::write(&caller, &external).unwrap();
        if ide_host::ops::lsp::apply_workspace_edit(&host, "main", plan).await.is_ok()
            || std::fs::read_to_string(&old).unwrap() != class
            || std::fs::read_to_string(&caller).unwrap() != external
            || project.documents.snapshot(links.id).unwrap().text != overlay_source
            || project.documents.snapshot(declaration.id).unwrap().text != overlay_class
        {
            return Err("stale URL plan applied or lost an overlay".into());
        }
        std::fs::write(&caller, source).unwrap();
        let plan =
            ide_host::ops::lsp::lsp_rename(&host, "main", RequestId::next(), links.id, position, replacement.into())
                .await
                .map_err(|e| format!("fresh URL rename: {e}"))?
                .edit;
        ide_host::ops::lsp::apply_workspace_edit(&host, "main", plan).await.map_err(|e| e.to_string())?;
        let changed = project.documents.snapshot(links.id).unwrap();
        let changed_class = project.documents.snapshot(declaration.id).unwrap();
        if changed.dirty
            || changed_class.dirty
            || std::fs::read_to_string(&caller).unwrap() != changed.text
            || std::fs::read_to_string(&new).unwrap() != changed_class.text
            || project.documents.by_path(&new) != Some(declaration.id)
            || !changed.text.contains("$plain = '\\App\\Invoice@show';")
            || !changed.text.contains("new LocalInvoice()")
            || !changed_class.text.contains("'overlay'")
            || (class_rename && old.exists())
        {
            return Err(format!("URL rename did not preserve runtime text, aliases or dirty overlays: {changed:?}"));
        }
        let php = repo.join("artifacts/toolchains/php-8.4.23-static/php");
        for path in [&new, &caller] {
            let output = std::process::Command::new(&php).args(["-n", "-l"]).arg(path).output().unwrap();
            if !output.status.success() {
                return Err(format!("renamed PHP syntax: {}", String::from_utf8_lossy(&output.stderr)));
            }
        }
        let script = r#"$loader = require $argv[2] . '/vendor/autoload.php';
$loader->addPsr4('App\\', $argv[1] . '/src', true);
$container = new \Illuminate\Container\Container();
\Illuminate\Container\Container::setInstance($container);
$router = new \Illuminate\Routing\Router(new \Illuminate\Events\Dispatcher(), $container);
$router->get('/invoice', $argv[3]);
$url = new \Illuminate\Routing\UrlGenerator($router->getRoutes(), \Illuminate\Http\Request::create('http://localhost'));
$redirect = new \Illuminate\Routing\Redirector($url);
$container->instance('url', $url);
$container->instance(\Illuminate\Contracts\Routing\UrlGenerator::class, $url);
$container->instance('redirect', $redirect);
\Illuminate\Support\Facades\Facade::setFacadeApplication($container);
require $argv[1] . '/src/Links.php';
foreach ([$one, $two, $three, $five] as $value) {
    if ($value !== 'http://localhost/invoice') { throw new \RuntimeException('URL changed: ' . $value); }
}
if ($four->getTargetUrl() !== 'http://localhost/invoice' || $plain !== '\App\Invoice@show' || $direct !== 'overlay') {
    throw new \RuntimeException('Redirect or metadata changed');
}
[$type, $method] = explode('@', $argv[3]);
if ((new $type)->$method() !== 'overlay') { throw new \RuntimeException('Overlay or PSR-4 rename lost'); }
echo '5 renamed controller URLs executed';"#;
        let action = if class_rename { "\\App\\Receipt@show" } else { "\\App\\Invoice@display" };
        let runtime = std::process::Command::new(&php)
            .args(["-n", "-r", script])
            .arg(&root)
            .arg(&laravel)
            .arg(action)
            .output()
            .unwrap();
        if !runtime.status.success() || runtime.stdout != b"5 renamed controller URLs executed" {
            return Err(format!(
                "URL runtime: stdout={}, stderr={}",
                String::from_utf8_lossy(&runtime.stdout),
                String::from_utf8_lossy(&runtime.stderr)
            ));
        }
        let group = events
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find_map(|event| match event {
                IdeEvent::DocPatched { history: Some(h), .. } => Some(h.group.clone()),
                _ => None,
            })
            .ok_or("URL rename omitted reviewed history")?;
        for redo in [false, true] {
            ide_host::ops::lsp::restore_reviewed_edit(&host, "main", group.clone(), redo)
                .await
                .map_err(|e| e.to_string())?;
            let current = project.documents.snapshot(links.id).unwrap();
            let current_class = project.documents.snapshot(declaration.id).unwrap();
            let expected_path = if redo { &new } else { &old };
            if project.documents.by_path(expected_path) != Some(declaration.id)
                || current.text != (if redo { &changed.text } else { &overlay_source }).as_str()
                || current_class.text != (if redo { &changed_class.text } else { &overlay_class }).as_str()
                || current.dirty == redo
                || current_class.dirty == redo
                || std::fs::read_to_string(&caller).unwrap() != (if redo { &changed.text } else { source })
                || std::fs::read_to_string(expected_path).unwrap() != (if redo { &changed_class.text } else { class })
            {
                return Err(format!("URL reviewed history lost overlay/disk state, redo={redo}"));
            }
            let needle = if class_rename && redo { "Receipt@show" } else { "Invoice@" };
            let targets = project
                .lsp
                .goto(links.id, at(&current.text, needle, 3), ide_lsp::types::GotoKind::Definition)
                .await
                .map_err(|e| e.to_string())?;
            if targets.len() != 1 || targets[0].path.to_path_buf() != *expected_path {
                return Err(format!("URL navigation stale after history, redo={redo}: {targets:?}"));
            }
        }
        std::fs::write(&new, "<?php // External declaration change.\n").unwrap();
        if ide_host::ops::lsp::restore_reviewed_edit(&host, "main", group, false).await.is_ok()
            || std::fs::read_to_string(&caller).unwrap() != changed.text
            || std::fs::read_to_string(&new).unwrap() != "<?php // External declaration change.\n"
            || project.documents.snapshot(links.id).unwrap().text != changed.text
            || project.documents.snapshot(declaration.id).unwrap().text != changed_class.text
            || (class_rename && old.exists())
        {
            return Err("URL history overwrote an external declaration or partially restored callers".into());
        }
        Ok(())
    }
    .await;
    project.lsp.shutdown_all().await;
    host.close_project("main");
    result.unwrap();
}
