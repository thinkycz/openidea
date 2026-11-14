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

/// Preserve source receipts across ordered RenameFile + text edits. Never
/// treat a newly named destination as an independently witnessed disk file.
fn validate_checked_targets(edit: &WorkspaceEdit, sources: &[ExpectedSource]) -> IdeResult<()> {
    use ide_core::FileEdit;
    let witnessed = |path: &std::path::Path| sources.iter().any(|s| s.uri == crate::uri::path_to_uri(path));
    let mut renamed = std::collections::HashMap::<PathBuf, PathBuf>::new();
    let mut origins = HashSet::new();
    let mut saw_text = false;
    for file in &edit.files {
        let source = match file {
            FileEdit::Rename { from, to } => {
                if saw_text
                    || from == to
                    || origins.contains(&**from)
                    || renamed.contains_key(&**to)
                    || renamed.contains_key(&**from)
                    || origins.contains(&**to)
                    || from.parent() != to.parent()
                    || from.extension().is_none_or(|e| !e.eq_ignore_ascii_case("php"))
                    || to.extension().is_none_or(|e| !e.eq_ignore_ascii_case("php"))
                {
                    return Err(IdeError::invalid("Checked PHP rename returned an unsupported file move"));
                }
                renamed.insert(to.to_path_buf(), from.to_path_buf());
                origins.insert(from.to_path_buf());
                &**from
            }
            FileEdit::Edit { path, .. } => {
                saw_text = true;
                if origins.contains(&**path) {
                    return Err(IdeError::invalid("Checked PHP rename edited an obsolete source path"));
                }
                renamed.get(&**path).map_or(&**path, |p| p.as_path())
            }
            _ => return Err(IdeError::invalid("Checked PHP rename returned an unsupported file operation")),
        };
        if !witnessed(source) {
            return Err(IdeError::invalid("Checked PHP rename returned an unwitnessed target"));
        }
    }
    Ok(())
}

/// An owned write's disk witness. Absence is distinct from any regular file.
#[cfg(unix)]
#[derive(PartialEq, Eq)]
pub(super) struct ReviewedDiskState {
    metadata: Option<ReviewedDiskMetadata>,
    sha256: [u8; 32],
}

#[cfg(unix)]
#[derive(PartialEq, Eq)]
struct ReviewedDiskMetadata {
    device: u64,
    inode: u64,
    mode: u32,
    len: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

#[cfg(unix)]
impl ReviewedDiskMetadata {
    fn new(metadata: &std::fs::Metadata) -> Self {
        use std::os::unix::fs::MetadataExt;
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            len: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        }
    }
}

/// Refuse uncertain, symlinked, changing or large inputs instead of hiding a notification.
#[cfg(unix)]
fn reviewed_disk_state(path: &Path) -> Option<ReviewedDiskState> {
    use std::io::Read;
    const LIMIT: u64 = 1 << 20;
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Some(ReviewedDiskState { metadata: None, sha256: [0; 32] });
        }
        Err(_) => return None,
    };
    if !metadata.is_file() || metadata.len() > LIMIT {
        return None;
    }
    let stamp = ReviewedDiskMetadata::new(&metadata);
    let mut file = std::fs::File::open(path).ok()?;
    if ReviewedDiskMetadata::new(&file.metadata().ok()?) != stamp {
        return None;
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    (&mut file).take(LIMIT + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 != stamp.len
        || ReviewedDiskMetadata::new(&file.metadata().ok()?) != stamp
        || ReviewedDiskMetadata::new(&std::fs::symlink_metadata(path).ok()?) != stamp
    {
        return None;
    }
    Some(ReviewedDiskState { metadata: Some(stamp), sha256: Sha256::digest(&bytes).into() })
}

impl LspManager {
    /// Publish a committed reviewed edit and witness its disk state for watcher echoes.
    /// Only identical bounded regular files (or identical absence) are coalesced.
    pub fn php_reviewed_files_changed(&self, created: &[PathBuf], modified: &[PathBuf], removed: &[PathBuf]) {
        self.php_files_changed_in(created, modified, removed, true);
    }

    /// PHPantom's static source index needs explicit disk-change notifications,
    /// including files closed after a checked rename query. This does not boot PHP.
    pub fn php_files_changed(&self, created: &[PathBuf], modified: &[PathBuf], removed: &[PathBuf]) {
        self.php_files_changed_in(created, modified, removed, false);
    }

    fn php_files_changed_in(&self, created: &[PathBuf], modified: &[PathBuf], removed: &[PathBuf], reviewed: bool) {
        // Serialize the decision and publication with reviewed writes. A watcher
        // reading an earlier state cannot retire navigation after that write returns.
        #[cfg(unix)]
        let mut receipts = self.reviewed_php_files.lock();
        #[cfg(not(unix))]
        let _ = reviewed;
        let changes: Vec<lsp::FileEvent> = [
            (created, lsp::FileChangeType::CREATED),
            (modified, lsp::FileChangeType::CHANGED),
            (removed, lsp::FileChangeType::DELETED),
        ]
        .into_iter()
        .flat_map(|(paths, typ)| paths.iter().map(move |path| (path, typ)))
        .filter_map(|(path, typ)| {
            #[cfg(unix)]
            if reviewed {
                if let Some(state) = reviewed_disk_state(path)
                    .filter(|state| state.metadata.is_none() == (typ == lsp::FileChangeType::DELETED))
                {
                    if receipts.len() >= 4096 && !receipts.contains_key(path) {
                        receipts.clear();
                    }
                    receipts.insert(path.clone(), state);
                } else {
                    receipts.remove(path);
                }
            } else if let Some(expected) = receipts.get(path) {
                if reviewed_disk_state(path).as_ref() == Some(expected) {
                    return None;
                }
                receipts.remove(path);
            }
            Some(lsp::FileEvent { uri: crate::uri::path_to_uri(path), typ })
        })
        .collect();
        if changes.is_empty() {
            return;
        }
        // Closed dependencies are absent from open-buffer snapshots. Retire
        // pending navigation before notifying servers of the observed changes.
        self.project_files_revision.fetch_add(1, Ordering::AcqRel);
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
        #[cfg(unix)]
        drop(receipts);
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
        let before = self.navigation_sources(snap, t)?;
        let word =
            crate::scope::search_word(snap, pos).ok_or_else(|| IdeError::invalid("No PHP symbol at the caret"))?;
        let candidates = self
            .candidates(t, word, crate::scope::MAX_CANDIDATES, Vec::new(), cancel)
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
                &before,
                cancel,
            )
            .await?;
        let edit = result.map(|e| convert::workspace_edit(&self.conv(t.enc()), e)).unwrap_or_default();
        // Renamed text targets are witnessed by their original source URI.
        // The host still validates disk identities, roots, absence and revisions.
        validate_checked_targets(&edit, &sources)?;
        if t.enc() != crate::text::Encoding::Utf16
            && edit.files.iter().any(|f| matches!(f, ide_core::FileEdit::Rename { .. }))
        {
            return Err(IdeError::invalid("Checked PHP file renames require UTF-16 server positions"));
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
        before: &NavigationSnapshot,
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
                && self.navigation_current(t, before)
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
        let before = self.navigation_sources(snap, t)?;
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
        if cancel.is_some_and(|token| token.is_cancelled()) || !self.navigation_current(t, &before) {
            return Err(IdeError::cancelled());
        }
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
            diagnostics: Vec::new(),
            original_diagnostics: None,
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
    ) -> IdeResult<(R, Target, NavigationSnapshot)> {
        if !(1..=5000).contains(&limit) {
            return Err(IdeError::invalid("Laravel query limit must be between 1 and 5000"));
        }
        let capability = format!("/experimental/phpWorkbench{feature}");
        let snap = self.snapshot(doc)?;
        let files_revision = self.project_files_revision.load(std::sync::atomic::Ordering::Acquire);
        // Static project queries may use the existing safe JSON translation
        // bridge. Keep generic PHP operations excluded in doc_targets.
        let servers: Vec<_> = snap
            .servers
            .iter()
            .filter(|server| {
                snap.language.as_str() == "php"
                    || (snap.language.as_str() == "json"
                        && snap.path.starts_with(self.project_root_for(&snap.path))
                        && self.php_resource_bridge(&snap.language, server))
            })
            .cloned()
            .collect();
        if servers.is_empty() {
            return Err(IdeError::unavailable(
                "Open a PHP source or Laravel translation JSON file to browse routes and views",
            ));
        }
        let targets = self
            .ready_targets(&servers, |caps| caps.raw.pointer(&capability).and_then(Value::as_bool) == Some(true))
            .await;
        let target = targets
            .into_iter()
            .find(|target| self.php_resource_supported(&snap.language, &target.server, &target.caps()))
            .ok_or_else(|| unsupported(&snap.servers, feature))?;
        let before = self.navigation_sources(&snap, &target)?;
        if before.files_revision != files_revision {
            return Err(IdeError::new(ErrorKind::OutOfSync, "Project roots or files changed during Laravel lookup"));
        }
        let method = match feature {
            "Routes" => "phpWorkbench/routes",
            "Views" => "phpWorkbench/views",
            _ => unreachable!(),
        };
        let reply = self
            .call(&target, method, LaravelQuery { uri: snap.uri.clone(), name, limit }, Some(cancel), LONG_TIMEOUT)
            .await?;
        self.laravel_query_current(&target, &before)?;
        Ok((reply, target, before))
    }

    fn laravel_query_current(&self, target: &Target, before: &NavigationSnapshot) -> IdeResult<()> {
        if self.navigation_current(target, before) {
            Ok(())
        } else {
            Err(IdeError::new(
                ErrorKind::OutOfSync,
                "PHP sources, project files or server changed during Laravel lookup",
            ))
        }
    }

    /// Static index only: never executes PHP, Composer, or Artisan.
    pub async fn laravel_routes(
        &self,
        doc: DocId,
        name: Option<String>,
        limit: usize,
        cancel: CancellationToken,
    ) -> IdeResult<LaravelRoutes> {
        let (reply, target, before): (RouteReply, _, _) =
            self.laravel_query(doc, name, limit, &cancel, "Routes").await?;
        let conv = self.conv(target.enc());
        let result = LaravelRoutes {
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
        };
        self.laravel_query_current(&target, &before)?;
        Ok(result)
    }

    /// Dotted template names and their current targets from PHPantom's static index.
    pub async fn laravel_views(
        &self,
        doc: DocId,
        name: Option<String>,
        limit: usize,
        cancel: CancellationToken,
    ) -> IdeResult<LaravelViews> {
        let (reply, target, before): (ViewReply, _, _) = self.laravel_query(doc, name, limit, &cancel, "Views").await?;
        let conv = self.conv(target.enc());
        let result = LaravelViews {
            views: reply.views,
            locations: reply.locations.iter().filter_map(|l| conv.location(l)).collect(),
            incomplete: reply.incomplete,
        };
        self.laravel_query_current(&target, &before)?;
        Ok(result)
    }
}

impl LspManager {
    /// Share standard Laravel data overlays with the selected PHPantom instance.
    /// Their implicit bridge supports references and explicit static Laravel
    /// navigation, requiring a safe-resource capability before sharing content.
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
        if json && path.components().any(|part| part == std::path::Component::ParentDir) {
            return Vec::new();
        }
        let php = LanguageId::new("php");
        self.language_servers(&php)
            .into_iter()
            .filter(|cfg| cfg.name == "phpantom")
            .filter_map(|cfg| {
                let root = if json {
                    // The workspace server may own several Composer projects.
                    // Validate this resource against its nearest manifest, not
                    // the outermost server root or an enclosing Laravel app.
                    let boundary = self.project_root_for(path);
                    path.ancestors()
                        .skip(1)
                        .take_while(|ancestor| ancestor.starts_with(&boundary))
                        .find(|ancestor| ancestor.join("composer.json").is_file())?
                        .to_path_buf()
                } else {
                    launch::find_root(&self.project_root, path, &cfg.root_markers)?
                };
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
        let bridge = self.php_resource_bridge(language, server);
        if bridge && language.as_str() == "json" && !server.root.starts_with(self.project_root_for(&server.root)) {
            return false;
        }
        !bridge || caps.raw.pointer(capability).and_then(Value::as_bool) == Some(true)
    }
}

#[cfg(test)]
mod checked_move_tests {
    use super::*;
    use ide_core::FileEdit;
    fn source(path: &str) -> ExpectedSource {
        ExpectedSource::new(&crate::uri::path_to_uri(std::path::Path::new(path)), 1, "<?php")
    }
    fn rename(from: &str, to: &str) -> FileEdit {
        FileEdit::Rename { from: PathBuf::from(from).into(), to: PathBuf::from(to).into() }
    }
    fn edit(path: &str) -> FileEdit {
        FileEdit::Edit { path: PathBuf::from(path).into(), edits: vec![], version: None }
    }
    #[test]
    fn checked_file_move_uses_original_source_receipts() {
        let plan = WorkspaceEdit {
            files: vec![rename("/p/Old.php", "/p/New.php"), edit("/p/New.php"), edit("/p/caller.php")],
            ..Default::default()
        };
        let receipts = vec![source("/p/Old.php"), source("/p/caller.php")];
        validate_checked_targets(&plan, &receipts).unwrap();
        assert!(validate_checked_targets(&plan, &receipts[1..]).is_err());
        assert!(validate_checked_targets(&plan, &receipts[..1]).is_err());
    }
    #[test]
    fn checked_file_move_refuses_ambiguous_and_unwitnessed_operations() {
        let receipts = vec![source("/p/Old.php"), source("/p/Other.php")];
        for files in [
            vec![rename("/p/Absent.php", "/p/New.php")],
            vec![rename("/p/Old.php", "/p/New.php"), edit("/p/Old.php")],
            vec![edit("/p/Old.php"), rename("/p/Old.php", "/p/New.php")],
            vec![rename("/p/Old.php", "/p/New.php"), rename("/p/New.php", "/p/Next.php")],
            vec![rename("/p/Old.php", "/p/New.php"), rename("/p/Other.php", "/p/New.php")],
            vec![rename("/p/Old.php", "/outside/New.php")],
            vec![FileEdit::Delete { path: PathBuf::from("/p/Old.php").into(), recursive: false }],
        ] {
            assert!(validate_checked_targets(&WorkspaceEdit { files, ..Default::default() }, &receipts).is_err());
        }
    }
}
