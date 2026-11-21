use super::json_translation_resources::{caps, echo};
use super::*;
use sha2::Digest;

const METHOD: &str = "phpWorkbench/jsonTranslationResource";
const PROVIDER: &str = "<?php /* json */";
const DIRTY: &str = "<?php /* alternate 😀 */";
const CALLER: &str = "<?php __('Greeting');";

struct ProviderFixture {
    h: Harness,
    target: Arc<MockState>,
    start: Arc<Gate>,
    caller: DocId,
    provider: DocId,
    path: PathBuf,
    original: PathBuf,
    original_doc: DocId,
    alternate: PathBuf,
    alternate_doc: DocId,
    unrelated: PathBuf,
}

impl ProviderFixture {
    fn new(advertised: bool, open_provider: bool) -> Self {
        let mut capabilities = caps();
        capabilities["experimental"]["phpWorkbenchProviderSources"] = json!(advertised);
        capabilities["experimental"]["phpWorkbenchProviderSourceSync"] = json!(advertised);
        let h = harness_for("php", "php", &[("phpantom", capabilities)], &[]);
        let app = h.root.join("app");
        let shared = h.root.join("shared");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::write(app.join("composer.json"), r#"{"require":{"laravel/framework":"13.37.0"}}"#).unwrap();
        std::fs::write(shared.join("composer.json"), r#"{"name":"fixture/shared"}"#).unwrap();
        h.mgr.set_content_roots(vec![app.clone(), shared.clone()]);
        let target = h.launcher.state_for_root("phpantom", &app);
        let start = Arc::new(Gate::default());
        *target.initialize_gate.lock() = Some(start.clone());
        let foreign = h.launcher.state_for_root("phpantom", &shared);
        *foreign.caps.lock() = incremental_caps();
        let path = shared.join("Provider.php");
        std::fs::write(&path, PROVIDER).unwrap();
        let original = shared.join("json/en.json");
        let alternate = shared.join("alternate/en.json");
        for p in [&original, &alternate] {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, r#"{"Greeting":"disk"}"#).unwrap();
        }
        let source = path.clone();
        let first = original.clone();
        let second = alternate.clone();
        target.responder.lock().replace(Arc::new(move |state, method, params| {
            let docs = state.docs.lock();
            let provider = docs.get(&uri_of(&source)).map_or(PROVIDER, String::as_str);
            let selected = if provider.contains("alternate") { &second } else { &first };
            match method {
                METHOD => {
                    let mut reply = echo(params, (params["resourceUri"] == uri_of(selected)).then(|| selected.parent().unwrap()));
                    reply["providerSources"] = json!({"providers":[{"className":"Shared\\Provider","uri":uri_of(&source),"sha256":format!("{:x}",sha2::Sha256::digest(provider.as_bytes()))}],"unresolved":[],"limited":false});
                    Some(reply)
                }
                "textDocument/hover" => Some(json!({"contents":{"kind":"markdown","value":docs.get(&uri_of(selected)).cloned().unwrap_or_else(|| "disk".into())}})),
                "textDocument/references" => Some(json!([])),
                _ => None,
            }
        }));
        let original_doc = DocId::next();
        h.mgr.did_open(original_doc, &original, &LanguageId::new("json"), 1, r#"{"Greeting":"private original"}"#);
        let alternate_doc = DocId::next();
        h.mgr.did_open(alternate_doc, &alternate, &LanguageId::new("json"), 1, r#"{"Greeting":"private alternate"}"#);
        let provider = DocId::next();
        if open_provider {
            h.mgr.did_open(provider, &path, &h.lang, 1, DIRTY);
        }
        let (caller, _) = h.open("app/Caller.php", CALLER);
        let (_, unrelated) = h.open("shared/Private.php", "<?php /* unrelated private buffer */");
        Self { h, target, start, caller, provider, path, original, original_doc, alternate, alternate_doc, unrelated }
    }

    async fn hover(&self) -> String {
        self.start.open();
        self.h.mgr.hover(self.caller, Pos::new(0, 10), Default::default()).await.unwrap().unwrap().contents.value
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn json_provider_sync_only_shares_declared_sources_with_an_advertised_consuming_server() {
    for advertised in [true, false] {
        let f = ProviderFixture::new(advertised, true);
        let hover = f.hover().await;
        assert!(
            hover.contains(if advertised { "private alternate" } else { "private original" }),
            "foreign provider must be prepared before ordinary hover: {hover}"
        );
        assert_eq!(f.target.docs.lock().get(&uri_of(&f.path)).cloned(), advertised.then(|| DIRTY.to_owned()));
        assert!(!f.target.docs.lock().contains_key(&uri_of(&f.unrelated)));
        assert_eq!(std::fs::read_to_string(&f.path).unwrap(), PROVIDER);
        f.h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn json_provider_sync_reconciles_foreign_source_lifecycle_without_another_php_action() {
    for action in ["edit", "restore", "close", "open", "restart", "disable", "references"] {
        println!("provider lifecycle: {action}");
        let f = ProviderFixture::new(true, action != "open");
        assert!(f.hover().await.contains(if action == "open" { "private original" } else { "private alternate" }));
        let change = |old: &str, new: &str, version| {
            f.h.mgr.did_change(
                f.provider,
                version,
                &[AppliedChange { range: rng(0, 0, 0, old.encode_utf16().count() as u32), text: new.into() }],
                &|| new.into(),
            )
        };
        match action {
            "edit" => change(DIRTY, PROVIDER, 2),
            "restore" => {
                change(DIRTY, PROVIDER, 2);
                change(PROVIDER, DIRTY, 3);
            }
            "close" => f.h.mgr.did_close(f.provider),
            "open" => f.h.mgr.did_open(f.provider, &f.path, &f.h.lang, 1, DIRTY),
            "restart" => {
                let id = f.h.mgr.servers().iter().find(|s| *s.root == f.h.root.join("app")).unwrap().id;
                f.h.mgr.restart(id).await.unwrap();
            }
            "disable" => f.h.mgr.set_execution_allowed(false),
            "references" => {
                let catalog = f.alternate_doc;
                f.h.mgr.references(catalog, Pos::new(0, 3), false, true, Default::default()).await.unwrap();
            }
            _ => unreachable!(),
        }
        let selected = if matches!(action, "edit" | "close") { &f.original } else { &f.alternate };
        if action == "disable" {
            eventually(|| {
                !f.target.docs.lock().contains_key(&uri_of(&f.path))
                    && !f.target.docs.lock().contains_key(&uri_of(&f.alternate))
            })
            .await;
        } else {
            eventually(|| f.target.docs.lock().contains_key(&uri_of(selected))).await;
            if action == "close" {
                assert!(!f.target.docs.lock().contains_key(&uri_of(&f.path)));
            } else {
                assert_eq!(
                    f.target.docs.lock().get(&uri_of(&f.path)).cloned(),
                    Some(if action == "edit" { PROVIDER } else { DIRTY }.into())
                );
            }
        }
        assert!(!f.target.docs.lock().contains_key(&uri_of(&f.unrelated)), "{action}");
        assert_eq!(std::fs::read_to_string(&f.path).unwrap(), PROVIDER);
        f.h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn json_provider_sync_refuses_unverifiable_or_ambiguous_buffers_before_forwarding() {
    for action in ["oversized", "ambiguous", "non-PHP", "stale disk", "missing metadata"] {
        println!("invalid provider source: {action}");
        let f = ProviderFixture::new(true, true);
        let mut changed_disk = false;
        match action {
            "oversized" => {
                let text = format!("<?php /*{}*/", "x".repeat(1_048_576));
                f.h.mgr.did_change(
                    f.provider,
                    2,
                    &[AppliedChange { range: rng(0, 0, 0, DIRTY.encode_utf16().count() as u32), text: text.clone() }],
                    &|| text.clone(),
                );
            }
            "ambiguous" => f.h.mgr.did_open(DocId::next(), &f.path, &f.h.lang, 1, DIRTY),
            "non-PHP" => {
                f.h.mgr.did_close(f.provider);
                f.h.mgr.did_open(f.provider, &f.path, &LanguageId::new("plaintext"), 2, DIRTY);
            }
            "stale disk" => {
                std::fs::write(&f.path, "<?php /* unreported disk edit */").unwrap();
                changed_disk = true;
            }
            "missing metadata" => {
                f.target
                    .responder
                    .lock()
                    .replace(Arc::new(|_, method, params| (method == METHOD).then(|| echo(params, None))));
                f.h.mgr.did_change(
                    f.provider,
                    2,
                    &[AppliedChange {
                        range: rng(0, 0, 0, DIRTY.encode_utf16().count() as u32),
                        text: format!("{DIRTY} "),
                    }],
                    &|| format!("{DIRTY} "),
                );
            }
            _ => unreachable!(),
        }
        f.start.open();
        assert!(f.h.mgr.hover(f.caller, Pos::new(0, 10), Default::default()).await.is_err(), "{action}");
        assert!(
            f.target.notified("textDocument/didOpen").iter().all(|v| v["textDocument"]["uri"] != uri_of(&f.path)),
            "{action}: reject every provider target before sending text"
        );
        assert!(!f.target.docs.lock().contains_key(&uri_of(&f.unrelated)));
        assert_eq!(
            std::fs::read_to_string(&f.path).unwrap(),
            if changed_disk { "<?php /* unreported disk edit */" } else { PROVIDER }
        );
        f.h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn json_provider_sync_rejects_changes_while_dependency_discovery_is_held() {
    for action in ["edit", "restore", "cancel"] {
        let f = ProviderFixture::new(true, true);
        let responder = f.target.responder.lock().clone().unwrap();
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        f.target.responder.lock().replace(Arc::new(move |state, method, params| {
            if method == METHOD {
                held.hold();
            }
            responder(state, method, params)
        }));
        let mgr = f.h.mgr.clone();
        let caller = f.caller;
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        f.start.open();
        let pending = tokio::spawn(async move { mgr.hover(caller, Pos::new(0, 10), token).await });
        eventually(|| gate.entered()).await;
        if action == "cancel" {
            cancel.cancel();
        } else {
            f.h.mgr.did_change(
                f.provider,
                2,
                &[AppliedChange { range: rng(0, 0, 0, DIRTY.encode_utf16().count() as u32), text: PROVIDER.into() }],
                &|| PROVIDER.into(),
            );
            if action == "restore" {
                f.h.mgr.did_change(
                    f.provider,
                    3,
                    &[AppliedChange {
                        range: rng(0, 0, 0, PROVIDER.encode_utf16().count() as u32),
                        text: DIRTY.into(),
                    }],
                    &|| DIRTY.into(),
                );
            }
        }
        let result = tokio::time::timeout(Duration::from_secs(2), pending).await;
        gate.open();
        assert!(result.unwrap().unwrap().is_err(), "{action}");
        assert!(!f.target.notified("textDocument/didOpen").iter().any(|v| v["textDocument"]["uri"] == uri_of(&f.path)));
        f.h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn json_provider_sync_closes_new_dependencies_on_cancellation_and_failed_requery() {
    for action in ["cancel", "error"] {
        let f = ProviderFixture::new(true, true);
        let responder = f.target.responder.lock().clone().unwrap();
        let gate = Arc::new(Gate::default());
        let held = gate.clone();
        let source = f.path.clone();
        let fail = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let failed = fail.clone();
        f.target.responder.lock().replace(Arc::new(move |state, method, params| {
            if method == METHOD && state.docs.lock().contains_key(&uri_of(&source)) {
                held.hold();
                if failed.load(Ordering::SeqCst) {
                    return Some(echo(params, None));
                }
            }
            responder(state, method, params)
        }));
        let mgr = f.h.mgr.clone();
        let caller = f.caller;
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        f.start.open();
        let pending = tokio::spawn(async move { mgr.hover(caller, Pos::new(0, 10), token).await });
        eventually(|| gate.entered()).await;
        assert!(f.target.docs.lock().contains_key(&uri_of(&f.path)));
        if action == "cancel" {
            cancel.cancel();
        } else {
            fail.store(true, Ordering::SeqCst);
            gate.open();
        }
        let result = tokio::time::timeout(Duration::from_secs(2), pending).await;
        gate.open();
        assert!(result.unwrap().unwrap().is_err(), "{action}");
        eventually(|| !f.target.docs.lock().contains_key(&uri_of(&f.path))).await;
        assert!(!f.target.docs.lock().contains_key(&uri_of(&f.original)));
        assert!(!f.target.docs.lock().contains_key(&uri_of(&f.alternate)));
        assert_eq!(std::fs::read_to_string(&f.path).unwrap(), PROVIDER);
        f.h.mgr.shutdown_all().await;
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn json_provider_sync_refuses_an_editor_alias_for_a_declared_physical_provider() {
    let f = ProviderFixture::new(true, false);
    let alias = f.h.root.join("shared/Alias.php");
    std::os::unix::fs::symlink(&f.path, &alias).unwrap();
    f.h.mgr.did_open(f.provider, &alias, &f.h.lang, 1, DIRTY);
    f.start.open();
    assert!(f.h.mgr.hover(f.caller, Pos::new(0, 10), Default::default()).await.is_err());
    assert!(
        !f.target
            .notified("textDocument/didOpen")
            .iter()
            .any(|event| event["textDocument"]["uri"] == uri_of(&f.path)
                || event["textDocument"]["uri"] == uri_of(&alias))
    );
    f.h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn json_provider_sync_keeps_metadata_only_engines_content_free() {
    let f = ProviderFixture::new(true, true);
    f.target.caps.lock()["experimental"]["phpWorkbenchProviderSourceSync"] = json!(false);
    assert!(f.hover().await.contains("private original"));
    let receipt = f.h.mgr.php_json_translation_resource(f.caller, &f.original, Default::default()).await.unwrap();
    assert!(receipt.provider_sources().is_some());
    assert!(!f.target.docs.lock().contains_key(&uri_of(&f.path)));
    assert!(!f.target.docs.lock().contains_key(&uri_of(&f.unrelated)));
    f.h.mgr.shutdown_all().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn json_provider_sync_recovers_without_open_catalogs_or_another_php_action() {
    for action in ["edit", "restore", "restart", "open", "disable"] {
        println!("provider recovery without catalogs: {action}");
        let f = ProviderFixture::new(true, action != "open");
        f.h.mgr.did_close(f.original_doc);
        f.h.mgr.did_close(f.alternate_doc);
        assert_eq!(f.hover().await, "disk");
        assert_eq!(f.target.docs.lock().get(&uri_of(&f.path)).cloned(), (action != "open").then(|| DIRTY.into()));
        let change = |old: &str, new: &str, version| {
            f.h.mgr.did_change(
                f.provider,
                version,
                &[AppliedChange { range: rng(0, 0, 0, old.encode_utf16().count() as u32), text: new.into() }],
                &|| new.into(),
            )
        };
        match action {
            "edit" => change(DIRTY, PROVIDER, 2),
            "restore" => {
                change(DIRTY, PROVIDER, 2);
                change(PROVIDER, DIRTY, 3);
            }
            "restart" => {
                let id = f.h.mgr.servers().iter().find(|s| *s.root == f.h.root.join("app")).unwrap().id;
                f.h.mgr.restart(id).await.unwrap();
            }
            "open" => {
                f.h.mgr.did_open(f.provider, &f.path, &f.h.lang, 2, DIRTY);
            }
            "disable" => f.h.mgr.set_execution_allowed(false),
            _ => unreachable!(),
        }
        let expected =
            if action == "disable" { None } else { Some(if action == "edit" { PROVIDER } else { DIRTY }.to_owned()) };
        eventually(|| f.target.docs.lock().get(&uri_of(&f.path)).cloned() == expected).await;
        assert!(!f.target.docs.lock().contains_key(&uri_of(&f.original)));
        assert!(!f.target.docs.lock().contains_key(&uri_of(&f.alternate)));
        assert!(!f.target.docs.lock().contains_key(&uri_of(&f.unrelated)));
        assert_eq!(
            f.target.requested("textDocument/hover").len(),
            1,
            "background recovery must not invoke PHP navigation"
        );
        assert!(!f.h.root.join("app/Caller.json").exists());
        assert_eq!(std::fs::read_to_string(&f.path).unwrap(), PROVIDER);
        f.h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn json_provider_sync_without_catalogs_preserves_legacy_and_metadata_only_boundaries() {
    for metadata_only in [true, false] {
        let f = ProviderFixture::new(metadata_only, true);
        f.target.caps.lock()["experimental"]["phpWorkbenchProviderSourceSync"] = json!(false);
        f.h.mgr.did_close(f.original_doc);
        f.h.mgr.did_close(f.alternate_doc);
        assert_eq!(f.hover().await, "disk");
        assert!(!f.target.docs.lock().contains_key(&uri_of(&f.path)));
        assert!(!f.target.docs.lock().contains_key(&uri_of(&f.unrelated)));
        assert!(
            f.target.requested(METHOD).is_empty(),
            "legacy no-catalog navigation must not query provider inventory"
        );
        f.h.mgr.shutdown_all().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn json_provider_sync_keeps_only_current_live_consumers_on_a_shared_peer() {
    let mut capabilities = caps();
    capabilities["experimental"]["phpWorkbenchProviderSources"] = json!(true);
    capabilities["experimental"]["phpWorkbenchProviderSourceSync"] = json!(true);
    let h = harness_for("php", "php", &[("phpantom", capabilities)], &[]);
    let shared = h.root.join("shared");
    for name in ["apps/a", "apps/b", "shared"] {
        std::fs::create_dir_all(h.root.join(name)).unwrap();
    }
    std::fs::write(h.root.join("mock.toml"), "").unwrap();
    std::fs::write(shared.join("mock.toml"), "").unwrap();
    for name in ["apps/a", "apps/b"] {
        std::fs::write(h.root.join(name).join("composer.json"), r#"{"require":{"laravel/framework":"13.37.0"}}"#)
            .unwrap();
    }
    std::fs::write(shared.join("composer.json"), r#"{"name":"fixture/shared"}"#).unwrap();
    h.mgr.set_content_roots(vec![h.root.clone(), shared.clone()]);
    let target = h.launcher.state_for_root("phpantom", &h.root);
    let start = Arc::new(Gate::default());
    *target.initialize_gate.lock() = Some(start.clone());
    *h.launcher.state_for_root("phpantom", &shared).caps.lock() = incremental_caps();
    let first = shared.join("First.php");
    let second = shared.join("Second.php");
    for path in [&first, &second] {
        std::fs::write(path, PROVIDER).unwrap();
        h.mgr.did_open(DocId::next(), path, &h.lang, 1, DIRTY);
    }
    let (_, private) = h.open("shared/Private.php", "<?php /* unrelated private */");
    let (a, _) = h.open("apps/a/Caller.php", CALLER);
    let (b, _) = h.open("apps/b/Caller.php", CALLER);
    let a_manifest = h.root.join("apps/a/composer.json");
    let allow_a = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let registered = allow_a.clone();
    let unresolved_a = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let unresolved = unresolved_a.clone();
    let limited_a = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let limited = limited_a.clone();
    let p = first.clone();
    let q = second.clone();
    target.responder.lock().replace(Arc::new(move |state,method,params| {
        let docs=state.docs.lock();
        match method {
            METHOD => {
                let is_a=params["projectUri"]==uri_of(&a_manifest);
                let path=if is_a {&p} else {&q};
                let text=docs.get(&uri_of(path)).map_or(PROVIDER,String::as_str);
                let providers=if is_a && (!registered.load(Ordering::SeqCst) || unresolved.load(Ordering::SeqCst)) {vec![]} else {vec![json!({"className":if is_a {"Shared\\First"} else {"Shared\\Second"},"uri":uri_of(path),"sha256":format!("{:x}",sha2::Sha256::digest(text.as_bytes()))})]};
                let mut reply=echo(params,None);reply["providerSources"]=json!({"providers":providers,"unresolved":if is_a && unresolved.load(Ordering::SeqCst) {vec!["Shared\\First"]} else {vec![]},"limited":is_a && limited.load(Ordering::SeqCst)});Some(reply)
            },
            "textDocument/hover" => {let path=if params["textDocument"]["uri"].as_str().unwrap().contains("/apps/a/") {&p} else {&q};Some(json!({"contents":{"kind":"markdown","value":docs.get(&uri_of(path)).cloned().unwrap_or_else(||PROVIDER.into())}}))},
            _=>None,
        }
    }));
    start.open();
    for caller in [a, b, a] {
        assert_eq!(
            h.mgr.hover(caller, Pos::new(0, 10), Default::default()).await.unwrap().unwrap().contents.value,
            DIRTY
        );
    }
    assert_eq!(h.mgr.servers().len(), 2, "both consumers share one peer; provider files use the other");
    assert_eq!(target.docs.lock().get(&uri_of(&first)).cloned(), Some(DIRTY.into()));
    assert_eq!(
        target.docs.lock().get(&uri_of(&second)).cloned(),
        Some(DIRTY.into()),
        "switching live contexts must not close an independently declared provider"
    );
    assert!(!target.docs.lock().contains_key(&uri_of(&private)));
    unresolved_a.store(true, Ordering::SeqCst);
    h.mgr.hover(b, Pos::new(0, 10), Default::default()).await.unwrap();
    assert_eq!(
        target.docs.lock().get(&uri_of(&first)).cloned(),
        Some(DIRTY.into()),
        "an unresolved registered class must not restore the disk provider"
    );
    let closes = target.notified("textDocument/didClose").len();
    limited_a.store(true, Ordering::SeqCst);
    assert!(
        h.mgr.hover(b, Pos::new(0, 10), Default::default()).await.is_err(),
        "a limited inventory cannot prove old ownership expired"
    );
    assert_eq!(target.notified("textDocument/didClose").len(), closes);
    limited_a.store(false, Ordering::SeqCst);
    unresolved_a.store(false, Ordering::SeqCst);
    h.mgr.hover(b, Pos::new(0, 10), Default::default()).await.unwrap();
    allow_a.store(false, Ordering::SeqCst);
    h.mgr.hover(b, Pos::new(0, 10), Default::default()).await.unwrap();
    assert!(
        !target.docs.lock().contains_key(&uri_of(&first)),
        "re-query the other live consumer before retaining its provider"
    );
    h.mgr.did_close(a);
    assert_eq!(h.mgr.hover(b, Pos::new(0, 10), Default::default()).await.unwrap().unwrap().contents.value, DIRTY);
    assert!(!target.docs.lock().contains_key(&uri_of(&first)));
    assert_eq!(std::fs::read_to_string(&first).unwrap(), PROVIDER);
    assert_eq!(std::fs::read_to_string(&second).unwrap(), PROVIDER);
    h.mgr.shutdown_all().await;
}
