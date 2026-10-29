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
    ) -> IdeResult<WorkspaceEdit> {
        let cancel = CancellationToken::new();
        let word =
            crate::scope::search_word(snap, pos).ok_or_else(|| IdeError::invalid("No PHP symbol at the caret"))?;
        let candidates = self
            .candidates(t, word, &cancel)
            .await
            .ok_or_else(|| IdeError::unavailable("Project rename scope is unavailable"))?;
        if candidates.limit_hit {
            return Err(IdeError::unavailable(
                "Rename scope exceeds the checked file limit. Narrow the project scope.",
            ));
        }
        let paths = candidates.files;
        let opened = self.open_transient(t, paths.clone(), &cancel).await;
        let mut sources = self.php_sources(snap, t)?;
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
            .call(
                t,
                "phpWorkbench/renameChecked",
                RenameCheckedParams { rename, expected_sources: sources.clone() },
                None,
                LONG_TIMEOUT,
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
        let reply = self.call(&target, method, LaravelQuery { name, limit }, Some(cancel), LONG_TIMEOUT).await?;
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
