//! Typed PHPantom extensions. Receipts refer to the exact UTF-8 overlay sent
//! to this engine generation, including unsaved dependencies.
use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExpectedSource {
    uri: lsp::Uri,
    version: i32,
    sha256_utf8: String,
}

impl ExpectedSource {
    pub(crate) fn new(uri: &lsp::Uri, version: i32, text: &str) -> Self {
        Self { uri: uri.clone(), version, sha256_utf8: format!("{:x}", Sha256::digest(text.as_bytes())) }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RenameCheckedParams {
    #[serde(flatten)]
    rename: Value,
    expected_sources: Vec<ExpectedSource>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OptimizeImportsParams {
    expected_source: ExpectedSource,
}
#[derive(Deserialize)]
struct OptimizeImportsResult {
    source: ExpectedSource,
    edit: Option<lsp::WorkspaceEdit>,
}

impl LspManager {
    /// PHPantom's static source index needs explicit disk-change notifications,
    /// including files closed after a checked rename query. This does not boot PHP.
    pub fn php_files_changed(&self, created: &[PathBuf], modified: &[PathBuf], removed: &[PathBuf]) {
        let changes: Vec<lsp::FileEvent> = [
            (created, lsp::FileChangeType::CREATED),
            (modified, lsp::FileChangeType::CHANGED),
            (removed, lsp::FileChangeType::DELETED),
        ]
        .into_iter()
        .flat_map(|(paths, typ)| {
            paths.iter().map(move |path| lsp::FileEvent { uri: crate::uri::path_to_uri(path), typ })
        })
        .collect();
        if changes.is_empty() {
            return;
        }
        let params = lsp::DidChangeWatchedFilesParams { changes };
        for server in self.servers.lock().iter() {
            let Some(run) = server.running() else { continue };
            if run.caps.read().raw.pointer("/experimental/phpWorkbenchRenameChecked").and_then(Value::as_bool)
                != Some(true)
            {
                continue;
            }
            if let Err(error) = run.peer.notify("workspace/didChangeWatchedFiles", &params) {
                tracing::debug!("{}: file-change notification: {error}", server.name);
            }
        }
    }

    fn php_sources(&self, snap: &DocSnap, t: &Target) -> IdeResult<Vec<ExpectedSource>> {
        let docs = self.docs.lock();
        let mut sources = Vec::new();
        for d in docs.values() {
            if d.uri.as_str().starts_with("file:")
                && d.servers.iter().any(|s| s.server.id == t.server.id && s.opened_gen == Some(t.run.generation))
            {
                if d.uri == snap.uri && d.text.as_str() != snap.text.as_str() {
                    return Err(IdeError::new(
                        ide_core::error::ErrorKind::OutOfSync,
                        "PHP source changed during the request",
                    ));
                }
                sources.push(ExpectedSource::new(&d.uri, d.version, d.text.as_str()));
            }
        }
        if !sources.iter().any(|s| s.uri == snap.uri) {
            return Err(IdeError::unavailable("PHP source has not synchronized with this server instance"));
        }
        sources.sort_by(|a, b| a.uri.as_str().cmp(b.uri.as_str()));
        Ok(sources)
    }

    pub(crate) async fn php_rename_checked(
        &self,
        snap: &DocSnap,
        t: &Target,
        pos: Pos,
        new_name: String,
        cancel: &CancellationToken,
    ) -> IdeResult<WorkspaceEdit> {
        let word =
            crate::scope::search_word(snap, pos).ok_or_else(|| IdeError::invalid("No PHP symbol at the caret"))?;
        let candidates = self
            .candidates(t, word, cancel)
            .await
            .ok_or_else(|| IdeError::unavailable("Project rename scope is unavailable"))?;
        if candidates.limit_hit {
            return Err(IdeError::unavailable(
                "Rename scope exceeds the checked file limit. Narrow the project scope.",
            ));
        }
        let paths = candidates.files;
        let opened = self.open_transient(t, paths.clone(), cancel).await;
        let overlays = self.php_sources(snap, t)?;
        let mut sources = overlays.clone();
        sources.extend(opened.sources.clone());
        if sources.len() > 1024 {
            return Err(IdeError::invalid("Too many PHP overlays for a checked rename"));
        }
        let witnessed: HashSet<_> = sources.iter().map(|s| s.uri.as_str()).collect();
        if paths.iter().any(|p| !witnessed.contains(crate::uri::path_to_uri(p).as_str())) {
            return Err(IdeError::unavailable("Some rename candidates could not be loaded. No changes were applied."));
        }
        let mut rename = snap.pos_params(t, pos);
        rename["newName"] = json!(new_name);
        let result: Option<lsp::WorkspaceEdit> = self
            .current_php_rename(
                snap,
                t,
                RenameCheckedParams { rename, expected_sources: sources.clone() },
                &overlays,
                cancel,
            )
            .await?;
        let edit = result.map(|e| convert::workspace_edit(&self.conv(t.enc()), e)).unwrap_or_default();
        // The engine must not return an edit to an unwitnessed disk file.
        for file in &edit.files {
            let ide_core::FileEdit::Edit { path, .. } = file else {
                return Err(IdeError::invalid("Checked PHP rename returned an unsupported file operation"));
            };
            if !sources.iter().any(|s| s.uri == crate::uri::path_to_uri(path)) {
                return Err(IdeError::invalid("Checked PHP rename returned an unwitnessed target"));
            }
        }
        Ok(edit)
    }

    /// A semantic publication can retire an attempt while transient files open.
    /// Keep the exact receipts and holds; never rebase a rename onto newer inputs.
    async fn current_php_rename(
        &self,
        snap: &DocSnap,
        t: &Target,
        params: RenameCheckedParams,
        overlays: &[ExpectedSource],
        cancel: &CancellationToken,
    ) -> IdeResult<Option<lsp::WorkspaceEdit>> {
        let method = "phpWorkbench/renameChecked";
        let deadline = tokio::time::Instant::now() + LONG_TIMEOUT;
        let current = || {
            let attached = {
                let docs = self.docs.lock();
                docs.values().any(|d| {
                    d.uri == snap.uri
                        && Arc::ptr_eq(&d.text, &snap.text)
                        && d.servers
                            .iter()
                            .any(|a| a.server.id == t.server.id && a.opened_gen == Some(t.run.generation))
                })
            };
            !cancel.is_cancelled()
                && t.server.running_gen(t.run.generation).is_some()
                && attached
                && self.php_sources(snap, t).is_ok_and(|sources| sources == overlays)
        };
        loop {
            if !current() {
                return Err(IdeError::cancelled());
            }
            let response = tokio::time::timeout_at(
                deadline,
                t.run.peer.request::<_, Option<lsp::WorkspaceEdit>>(method, &params, Some(cancel.clone())),
            )
            .await
            .unwrap_or(Err(RpcError::Timeout));
            if !current() {
                return Err(IdeError::cancelled());
            }
            match response {
                Err(RpcError::Server { code, .. }) if code == ide_jsonrpc::codes::CONTENT_MODIFIED => {
                    if tokio::time::Instant::now() + Duration::from_millis(50) >= deadline {
                        return Err(IdeError::unavailable(
                            "PHP project is still updating. Retry Rename after indexing finishes.",
                        ));
                    }
                    tokio::select! {
                        () = cancel.cancelled() => return Err(IdeError::cancelled()),
                        () = tokio::time::sleep(Duration::from_millis(50)) => {}
                    }
                }
                result => return result.map_err(|error| rpc_err(&t.server.name, method, error)),
            }
        }
    }

    pub(crate) async fn php_optimize_imports(
        &self,
        snap: &DocSnap,
        t: &Target,
        cancel: Option<&CancellationToken>,
    ) -> IdeResult<CodeAction> {
        let source = self
            .php_sources(snap, t)?
            .into_iter()
            .find(|s| s.uri == snap.uri)
            .ok_or_else(|| IdeError::invalid("No PHP source receipt"))?;
        let result: OptimizeImportsResult = self
            .call(
                t,
                "phpWorkbench/optimizeImports",
                OptimizeImportsParams { expected_source: source.clone() },
                cancel,
                LONG_TIMEOUT,
            )
            .await?;
        if result.source != source {
            return Err(IdeError::invalid("PHP Optimize Imports returned a mismatched source receipt"));
        }
        let edit = result.edit.map(|e| convert::workspace_edit(&self.conv(t.enc()), e)).unwrap_or_default();
        if edit
            .files
            .iter()
            .any(|f| !matches!(f, ide_core::FileEdit::Edit {path, ..} if crate::uri::path_to_uri(path) == snap.uri))
        {
            return Err(IdeError::invalid("PHP Optimize Imports returned an edit outside its source document"));
        }
        Ok(CodeAction {
            title: "Optimize Imports".into(),
            kind: Some("source.organizeImports".into()),
            is_preferred: true,
            disabled_reason: None,
            edit: Some(edit),
            command: None,
            data: None,
            server: t.server.id,
        })
    }
}

#[derive(Serialize)]
struct LaravelQuery {
    uri: lsp::Uri,
    name: Option<String>,
    limit: usize,
}

#[derive(Deserialize)]
struct RouteItem {
    name: String,
    uri: String,
    parameters: Option<Vec<String>>,
    from_vendor: bool,
}

#[derive(Deserialize)]
struct RouteReply {
    routes: Vec<RouteItem>,
    locations: Vec<lsp::Location>,
    incomplete: bool,
}

#[derive(Deserialize)]
struct ViewReply {
    views: Vec<String>,
    locations: Vec<lsp::Location>,
    incomplete: bool,
}

impl LspManager {
    async fn laravel_query<R: serde::de::DeserializeOwned>(
        &self,
        doc: DocId,
        name: Option<String>,
        limit: usize,
        cancel: &CancellationToken,
        feature: &str,
    ) -> IdeResult<(R, Target)> {
        if !(1..=5000).contains(&limit) {
            return Err(IdeError::invalid("Laravel query limit must be between 1 and 5000"));
        }
        let capability = format!("/experimental/phpWorkbench{feature}");
        let (snap, targets) = self
            .doc_targets(doc, feature, |caps| caps.raw.pointer(&capability).and_then(Value::as_bool) == Some(true))
            .await?;
        if snap.language.as_str() != "php" {
            return Err(IdeError::unavailable("Open a PHP source file to browse Laravel routes and views"));
        }
        let target = targets.into_iter().next().expect("doc_targets requires a target");
        let before = self.php_sources(&snap, &target)?;
        let method = match feature {
            "Routes" => "phpWorkbench/routes",
            "Views" => "phpWorkbench/views",
            _ => unreachable!(),
        };
        let reply = self
            .call(&target, method, LaravelQuery { uri: snap.uri.clone(), name, limit }, Some(cancel), LONG_TIMEOUT)
            .await?;
        if target.server.running_gen(target.run.generation).is_none()
            || self.php_sources(&self.snapshot(doc)?, &target)? != before
        {
            return Err(IdeError::new(ErrorKind::OutOfSync, "PHP sources or server changed during Laravel lookup"));
        }
        Ok((reply, target))
    }

    /// Static index only: never executes PHP, Composer, or Artisan.
    pub async fn laravel_routes(
        &self,
        doc: DocId,
        name: Option<String>,
        limit: usize,
        cancel: CancellationToken,
    ) -> IdeResult<LaravelRoutes> {
        let (reply, target): (RouteReply, _) = self.laravel_query(doc, name, limit, &cancel, "Routes").await?;
        let conv = self.conv(target.enc());
        Ok(LaravelRoutes {
            routes: reply
                .routes
                .into_iter()
                .map(|r| LaravelRoute {
                    name: r.name,
                    uri: r.uri,
                    parameters: r.parameters,
                    from_vendor: r.from_vendor,
                })
                .collect(),
            locations: reply.locations.iter().filter_map(|l| conv.location(l)).collect(),
            incomplete: reply.incomplete,
        })
    }

    /// Dotted template names and their current targets from PHPantom's static index.
    pub async fn laravel_views(
        &self,
        doc: DocId,
        name: Option<String>,
        limit: usize,
        cancel: CancellationToken,
    ) -> IdeResult<LaravelViews> {
        let (reply, target): (ViewReply, _) = self.laravel_query(doc, name, limit, &cancel, "Views").await?;
        let conv = self.conv(target.enc());
        Ok(LaravelViews {
            views: reply.views,
            locations: reply.locations.iter().filter_map(|l| conv.location(l)).collect(),
            incomplete: reply.incomplete,
        })
    }
}

impl LspManager {
    /// Share standard Laravel data overlays with the selected PHPantom instance.
    /// Their implicit bridge supports references only and requires an advertised
    /// safe-resource capability before sending any non-PHP document content.
    pub(crate) fn php_resource_servers(&self, language: &LanguageId, path: &Path) -> Vec<Arc<Server>> {
        use std::io::Read;
        let json = language.as_str() == "json"
            && path.extension().and_then(|e| e.to_str()) == Some("json")
            && path.parent().and_then(Path::file_name).and_then(|n| n.to_str()) == Some("lang");
        let dotenv = language.as_str() == "dotenv"
            && path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n == ".env" || n.starts_with(".env."));
        if !json && !dotenv {
            return Vec::new();
        }
        let php = LanguageId::new("php");
        self.language_servers(&php)
            .into_iter()
            .filter(|cfg| cfg.name == "phpantom")
            .filter_map(|cfg| {
                let root = launch::find_root(&self.project_root, path, &cfg.root_markers)?;
                let relative = path.strip_prefix(&root).ok()?;
                if json && !matches!(relative.parent()?.to_str()?, "lang" | "resources/lang")
                    || dotenv && relative.components().count() != 1
                {
                    return None;
                }
                let mut bytes = Vec::new();
                std::fs::File::open(root.join("composer.json")).ok()?.take(1_048_577).read_to_end(&mut bytes).ok()?;
                if bytes.len() > 1_048_576 {
                    return None;
                }
                let composer: Value = serde_json::from_slice(&bytes).ok()?;
                if !["require", "require-dev"].iter().any(|section| {
                    composer.get(section).and_then(|r| r.get("laravel/framework")).and_then(Value::as_str).is_some()
                }) {
                    return None;
                }
                Some(self.server_for(&cfg, &php, path))
            })
            .collect()
    }

    pub(crate) fn php_resource_bridge(&self, language: &LanguageId, server: &Server) -> bool {
        matches!(language.as_str(), "json" | "dotenv")
            && server.name == "phpantom"
            && !self.language_servers(language).iter().any(|cfg| cfg.name == server.name)
    }

    pub(crate) fn php_resource_supported(&self, language: &LanguageId, server: &Server, caps: &ServerCaps) -> bool {
        let capability = match language.as_str() {
            "dotenv" => "/experimental/phpWorkbenchEnvironmentResources",
            _ => "/experimental/phpWorkbenchJsonTranslations",
        };
        !self.php_resource_bridge(language, server)
            || caps.raw.pointer(capability).and_then(Value::as_bool) == Some(true)
    }
}
