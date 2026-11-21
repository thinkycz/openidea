use super::*;
mod composer;
mod provider_sync;
mod providers;
pub use providers::{PhpProviderSource, PhpProviderSources};
mod references;
mod refresh;
pub(crate) use refresh::JsonRefreshKey;
mod sync;
use composer::ComposerInputs;

/// A content-free registration decision for one consuming PHP server generation.
/// Revalidate before using it; it does not authorize forwarding buffer contents.
pub struct PhpJsonTranslationResource {
    project: PathBuf,
    resource: PathBuf,
    root: Option<PathBuf>,
    provider_sources: Option<Arc<PhpProviderSources>>,
    provider_sessions: Vec<references::ResourceSession>,
    target: Target,
    sources: NavigationSnapshot,
    composer: Option<Arc<ComposerInputs>>,
}

pub(crate) struct JsonResourceWitness {
    project: PathBuf,
    resource: PathBuf,
    composer: Option<Arc<ComposerInputs>>,
    provider_sources: Option<Arc<PhpProviderSources>>,
}

struct JsonInputSnapshot {
    revision: u64,
    _changes: tokio::sync::watch::Receiver<u64>,
}

impl std::fmt::Debug for PhpJsonTranslationResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhpJsonTranslationResource")
            .field("project", &self.project)
            .field("resource", &self.resource)
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl PhpJsonTranslationResource {
    pub fn project_path(&self) -> &Path {
        &self.project
    }

    pub fn resource_path(&self) -> &Path {
        &self.resource
    }

    /// None means the current PHP graph did not prove a static registration.
    pub fn root_path(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Static locations and server-held digests; this does not authorize forwarding buffers.
    pub fn provider_sources(&self) -> Option<&PhpProviderSources> {
        self.provider_sources.as_deref()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceQuery {
    project_uri: lsp::Uri,
    resource_uri: lsp::Uri,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceReply {
    project_uri: String,
    resource_uri: String,
    #[serde(deserialize_with = "Deserialize::deserialize")]
    root_uri: Option<String>,
    #[serde(default)]
    provider_sources: Option<Value>,
}

fn supports_resources(caps: &ServerCaps) -> bool {
    ["phpWorkbenchJsonTranslationResource", "phpWorkbenchRegisteredJsonTranslations"]
        .iter()
        .all(|cap| caps.raw.pointer(&format!("/experimental/{cap}")).and_then(Value::as_bool) == Some(true))
}

/// Check spelling and file kind without reading a catalog or following an alias.
fn physical_path(path: &Path, existing_source: bool) -> IdeResult<()> {
    if !path.is_absolute()
        || path.to_str().is_none_or(|p| p.contains('\0'))
        || path.components().any(|part| part == std::path::Component::ParentDir)
        || uri::path_to_uri_string(path).len() > 1_048_576
    {
        return Err(IdeError::invalid("Expected a bounded physical local path"));
    }
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() => return Err(IdeError::invalid("Expected a regular file")),
        Ok(_) => {}
        Err(error) if !existing_source && error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(IdeError::invalid("File authority is unavailable")),
    }
    let mut ancestor = path;
    let mut missing = Vec::new();
    let physical = loop {
        match std::fs::symlink_metadata(ancestor) {
            Ok(_) => {
                break dunce::canonicalize(ancestor).map_err(|_| IdeError::invalid("Unresolvable file authority"))?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(ancestor.file_name().ok_or_else(|| IdeError::invalid("Missing file authority"))?);
                ancestor = ancestor.parent().ok_or_else(|| IdeError::invalid("Missing file authority"))?;
            }
            Err(_) => return Err(IdeError::invalid("Unreadable file authority")),
        }
    };
    let physical = missing.into_iter().rev().fold(physical, |path, part| path.join(part));
    if physical != path || uri::path_to_uri_string(&physical) != uri::path_to_uri_string(path) {
        return Err(IdeError::invalid("Use the preferred physical file path"));
    }
    Ok(())
}

async fn resource_authority(project: &Path, resource: &Path, cancel: &CancellationToken) -> IdeResult<()> {
    let (project, resource) = (project.to_owned(), resource.to_owned());
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(IdeError::cancelled()),
        result = tokio::task::spawn_blocking(move || {
            physical_path(&project, true)?;
            physical_path(&resource, false)
        }) => result.map_err(|error| IdeError::internal(error.to_string()))?,
    }
}

impl LspManager {
    fn json_input_snapshot(&self) -> JsonInputSnapshot {
        let changes = self.json_preparation_changed.subscribe();
        let revision = *changes.borrow();
        JsonInputSnapshot { revision, _changes: changes }
    }

    /// Classify a proposed catalog through its consuming open PHP source, never
    /// through the catalog's nearest Composer project. No JSON text is sent.
    pub async fn php_json_translation_resource(
        &self,
        php_doc: DocId,
        resource: &Path,
        cancel: CancellationToken,
    ) -> IdeResult<PhpJsonTranslationResource> {
        if cancel.is_cancelled() {
            return Err(IdeError::cancelled());
        }
        if !self.execution_allowed.load(Ordering::Acquire) {
            return Err(IdeError::unavailable("Language-server execution is disabled"));
        }
        let _selection = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(IdeError::cancelled()),
            gate = self.php_json_resource_gate.lock() => gate,
        };
        let files_revision = self.project_files_revision.load(Ordering::Acquire);
        let snap = self.snapshot(php_doc)?;
        if snap.language.as_str() != "php"
            || snap.path.extension().is_none_or(|ext| ext != "php")
            || snap.uri != uri::path_to_uri(&snap.path)
        {
            return Err(IdeError::invalid("Select an existing physical PHP source"));
        }
        if resource.extension().is_none_or(|ext| ext != "json")
            || resource.file_stem().is_none_or(|stem| stem.is_empty())
        {
            return Err(IdeError::invalid("Expected a JSON locale file"));
        }
        resource_authority(&snap.path, resource, &cancel).await?;
        let servers: Vec<_> = snap.servers.iter().filter(|server| server.name == "phpantom").cloned().collect();
        let targets = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(IdeError::cancelled()),
            targets = self.ready_targets(&servers, supports_resources) => targets,
        };
        let target = targets.into_iter().next().ok_or_else(|| unsupported(&servers, "registered JSON translations"))?;
        let sources = self.navigation_sources(&snap, &target)?;
        if sources.files_revision != files_revision {
            return Err(IdeError::new(
                ErrorKind::OutOfSync,
                "Project roots or files changed during JSON resource lookup",
            ));
        }
        let receipt = PhpJsonTranslationResource {
            project: snap.path,
            resource: resource.to_owned(),
            root: None,
            provider_sources: None,
            provider_sessions: Vec::new(),
            target,
            sources,
            composer: None,
        };
        self.query_json_translation_resource(receipt, cancel).await
    }

    async fn query_json_translation_resource(
        &self,
        mut receipt: PhpJsonTranslationResource,
        cancel: CancellationToken,
    ) -> IdeResult<PhpJsonTranslationResource> {
        let inputs = self.json_input_snapshot();
        let provider_capability = providers::supported(&receipt.target.caps());
        let params = ResourceQuery {
            project_uri: uri::path_to_uri(&receipt.project),
            resource_uri: uri::path_to_uri(&receipt.resource),
        };
        let server_cancel = receipt.target.server.start_token();
        let reply: IdeResult<ResourceReply> = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(IdeError::cancelled()),
            () = server_cancel.cancelled() => {
                return Err(IdeError::new(ErrorKind::OutOfSync, "JSON resource server stopped or execution was disabled"));
            }
            reply = async {
                if receipt.composer.is_some() {
                    self.current_composer_resource_query(&receipt, &params, &cancel).await
                } else {
                    self.current_navigation(
                        &receipt.target,
                        "phpWorkbench/jsonTranslationResource",
                        serde_json::to_value(&params).map_err(|error| IdeError::internal(error.to_string()))?,
                        &receipt.sources,
                        Some(&cancel),
                        LONG_TIMEOUT,
                    ).await
                }
            } => reply,
        };
        if cancel.is_cancelled() {
            return Err(IdeError::cancelled());
        }
        self.laravel_query_current(&receipt.target, &receipt.sources)?;
        let reply = reply?;
        if reply.project_uri != params.project_uri.as_str() || reply.resource_uri != params.resource_uri.as_str() {
            return Err(IdeError::invalid("JSON resource lookup returned mismatched URI receipts"));
        }
        let root = if let Some(root_uri) = reply.root_uri {
            let root = receipt.resource.parent().ok_or_else(|| IdeError::invalid("Missing resource directory"))?;
            let mut expected = uri::path_to_uri_string(root);
            if !expected.ends_with('/') {
                expected.push('/');
            }
            if root_uri != expected {
                return Err(IdeError::invalid("JSON resource lookup returned an invalid locale directory"));
            }
            Some(root.to_owned())
        } else {
            None
        };
        receipt.root = root;
        if providers::supported(&receipt.target.caps()) != provider_capability {
            return Err(IdeError::new(ErrorKind::OutOfSync, "Provider source capabilities changed"));
        }
        if provider_capability {
            let value = reply
                .provider_sources
                .ok_or_else(|| IdeError::invalid("Missing advertised provider source metadata"))?;
            receipt.provider_sources = Some(Arc::new(self.read_provider_sources(&receipt, value, &cancel).await?));
            if *self.json_preparation_changed.borrow() != inputs.revision {
                return Err(IdeError::new(ErrorKind::OutOfSync, "Managed inputs changed during provider lookup"));
            }
        }
        self.php_json_translation_resource_current(&receipt, cancel).await?;
        Ok(receipt)
    }

    /// Recheck the same source receipt, server generation and physical authority;
    /// do not turn an old decision into a fresh query against different inputs.
    pub async fn php_json_translation_resource_current(
        &self,
        receipt: &PhpJsonTranslationResource,
        cancel: CancellationToken,
    ) -> IdeResult<()> {
        if cancel.is_cancelled() {
            return Err(IdeError::cancelled());
        }
        self.laravel_query_current(&receipt.target, &receipt.sources)?;
        if !receipt.provider_sessions.is_empty() && !provider_sync::supported(&receipt.target.caps()) {
            return Err(IdeError::new(ErrorKind::OutOfSync, "Provider synchronization capability changed"));
        }
        if !self.execution_allowed.load(Ordering::Acquire) || !supports_resources(&receipt.target.caps()) {
            return Err(IdeError::new(ErrorKind::OutOfSync, "JSON resource server capabilities changed"));
        }
        let authority = if let Some(composer) = &receipt.composer {
            self.composer_inputs_current(&receipt.project, &receipt.resource, composer, &cancel).await
        } else {
            resource_authority(&receipt.project, &receipt.resource, &cancel).await
        };
        if cancel.is_cancelled() {
            return Err(IdeError::cancelled());
        }
        authority.map_err(|_| IdeError::new(ErrorKind::OutOfSync, "JSON resource file authority changed"))?;
        if let Some(providers) = &receipt.provider_sources {
            self.provider_sources_current(receipt, providers, &cancel).await?;
        }
        self.laravel_query_current(&receipt.target, &receipt.sources)
    }
}
