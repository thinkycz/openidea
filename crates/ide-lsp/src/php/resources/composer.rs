use super::*;
use std::io::Read;

const LIMIT: u64 = 1_048_576;

pub(super) struct ComposerInputs {
    disk: ComposerDisk,
    buffers: Vec<NavigationSource>,
}

#[derive(PartialEq, Eq)]
struct ComposerDisk {
    stamp: ComposerStamp,
    sha256: [u8; 32],
}

#[cfg(unix)]
type ComposerStamp = ReviewedDiskMetadata;

#[cfg(not(unix))]
#[derive(PartialEq, Eq)]
struct ComposerStamp {
    len: u64,
    modified: Option<std::time::SystemTime>,
    created: Option<std::time::SystemTime>,
    readonly: bool,
}

#[cfg(not(unix))]
impl ComposerStamp {
    fn new(metadata: &std::fs::Metadata) -> Self {
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
            readonly: metadata.permissions().readonly(),
        }
    }
}

fn read_manifest(path: &Path, buffers: &[Arc<DocText>]) -> IdeResult<ComposerDisk> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| IdeError::invalid("Composer manifest is unavailable"))?;
    if !metadata.is_file() || metadata.len() > LIMIT {
        return Err(IdeError::invalid("Expected a bounded regular Composer manifest"));
    }
    let stamp = ComposerStamp::new(&metadata);
    #[cfg(unix)]
    let mut file = {
        use rustix::fs::{Mode, OFlags, open};
        let fd = open(path, OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK, Mode::empty())
            .map_err(|_| IdeError::invalid("Composer manifest cannot be opened safely"))?;
        std::fs::File::from(fd)
    };
    #[cfg(not(unix))]
    let mut file = std::fs::File::open(path).map_err(|_| IdeError::invalid("Composer manifest cannot be opened"))?;
    let opened = file.metadata().map_err(|_| IdeError::invalid("Composer manifest metadata is unavailable"))?;
    if !opened.is_file() || ComposerStamp::new(&opened) != stamp {
        return Err(IdeError::new(ErrorKind::OutOfSync, "Composer manifest changed while opening"));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    (&mut file)
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| IdeError::invalid("Composer manifest cannot be read"))?;
    if bytes.len() as u64 > LIMIT {
        return Err(IdeError::invalid("Expected a bounded Composer manifest"));
    }
    let manifest: Value = serde_json::from_slice(&bytes).map_err(|_| IdeError::invalid("Invalid Composer JSON"))?;
    if !manifest.is_object() {
        return Err(IdeError::invalid("Expected a Composer JSON object"));
    }
    for text in buffers {
        if text.as_str().len() as u64 > LIMIT
            || serde_json::from_str::<Value>(text.as_str()).ok().as_ref() != Some(&manifest)
        {
            return Err(IdeError::new(
                ErrorKind::OutOfSync,
                "Save or discard the changed Composer manifest before querying its project",
            ));
        }
    }
    let after = file.metadata().map_err(|_| IdeError::invalid("Composer manifest metadata is unavailable"))?;
    let current = std::fs::symlink_metadata(path).map_err(|_| IdeError::invalid("Composer manifest is unavailable"))?;
    if !current.is_file()
        || bytes.len() as u64 != metadata.len()
        || ComposerStamp::new(&after) != stamp
        || ComposerStamp::new(&current) != stamp
    {
        return Err(IdeError::new(ErrorKind::OutOfSync, "Composer manifest changed while reading"));
    }
    Ok(ComposerDisk { stamp, sha256: Sha256::digest(&bytes).into() })
}

async fn composer_disk(
    project: &Path,
    resource: &Path,
    buffers: &[NavigationSource],
    cancel: &CancellationToken,
) -> IdeResult<ComposerDisk> {
    let (project, resource) = (project.to_owned(), resource.to_owned());
    let texts: Vec<_> = buffers.iter().map(|source| source.text.clone()).collect();
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(IdeError::cancelled()),
        result = tokio::task::spawn_blocking(move || {
            physical_path(&project, true)?;
            physical_path(&resource, false)?;
            let disk = read_manifest(&project, &texts)?;
            physical_path(&project, true)?;
            Ok(disk)
        }) => result.map_err(|error| IdeError::internal(error.to_string()))?,
    }
}

impl LspManager {
    fn composer_buffers(&self, project: &Path) -> Vec<NavigationSource> {
        self.docs
            .lock()
            .iter()
            .filter(|(_, doc)| doc.path == project || doc.canonical == project)
            .map(|(id, doc)| NavigationSource {
                doc: *id,
                version: doc.version,
                uri: doc.uri.clone(),
                language: doc.language.clone(),
                text: doc.text.clone(),
            })
            .collect()
    }

    fn composer_buffers_current(&self, project: &Path, sources: &[NavigationSource]) -> bool {
        let docs = self.docs.lock();
        docs.values().filter(|doc| doc.path == project || doc.canonical == project).count() == sources.len()
            && sources.iter().all(|source| {
                docs.get(&source.doc).is_some_and(|doc| {
                    (doc.path == project || doc.canonical == project)
                        && doc.uri == source.uri
                        && doc.version == source.version
                        && doc.language == source.language
                        && Arc::ptr_eq(&doc.text, &source.text)
                })
            })
    }

    pub(super) async fn composer_inputs_current(
        &self,
        project: &Path,
        resource: &Path,
        inputs: &ComposerInputs,
        cancel: &CancellationToken,
    ) -> IdeResult<()> {
        if !self.composer_buffers_current(project, &inputs.buffers) {
            return Err(IdeError::new(ErrorKind::OutOfSync, "Composer buffers changed during resource lookup"));
        }
        let disk = composer_disk(project, resource, &inputs.buffers, cancel)
            .await
            .map_err(|_| IdeError::new(ErrorKind::OutOfSync, "Composer manifest authority or content changed"))?;
        if disk != inputs.disk || !self.composer_buffers_current(project, &inputs.buffers) {
            return Err(IdeError::new(ErrorKind::OutOfSync, "Composer inputs changed during resource lookup"));
        }
        Ok(())
    }

    /// Select a consuming project before a PHP editor exists. The manifest is
    /// witnessed locally; neither its text nor the proposed JSON overlay is sent.
    pub async fn php_json_translation_resource_from_composer(
        &self,
        composer: &Path,
        resource: &Path,
        cancel: CancellationToken,
    ) -> IdeResult<PhpJsonTranslationResource> {
        let _selection = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(IdeError::cancelled()),
            gate = self.php_json_resource_gate.lock() => gate,
        };
        self.json_resource_from_composer(composer, resource, cancel).await
    }

    pub(super) async fn json_resource_from_composer(
        &self,
        composer: &Path,
        resource: &Path,
        cancel: CancellationToken,
    ) -> IdeResult<PhpJsonTranslationResource> {
        if cancel.is_cancelled() {
            return Err(IdeError::cancelled());
        }
        if !self.execution_allowed.load(Ordering::Acquire) {
            return Err(IdeError::unavailable("Language-server execution is disabled"));
        }
        let (inputs, revision) = self.composer_resource_inputs(composer, resource, &cancel).await?;
        let manager = self.me.upgrade().ok_or_else(IdeError::cancelled)?;
        let project = composer.to_owned();
        let servers = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(IdeError::cancelled()),
            result = tokio::task::spawn_blocking(move || {
                if !manager.execution_allowed.load(Ordering::Acquire) || manager.project_files_revision.load(Ordering::Acquire) != revision {
                    return Err(IdeError::new(ErrorKind::OutOfSync, "Composer project changed before server selection"));
                }
                let servers: Vec<_> = manager.servers_for(&LanguageId::new("php"), &project).into_iter()
                    .filter(|server| server.name == "phpantom").collect();
                for server in &servers { manager.ensure_started(server); }
                Ok(servers)
            }) => result.map_err(|error| IdeError::internal(error.to_string()))??,
        };
        let startup_cancel =
            servers.first().ok_or_else(|| unsupported(&servers, "registered JSON translations"))?.start_token();
        let targets = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(IdeError::cancelled()),
            () = startup_cancel.cancelled() => return Err(IdeError::new(ErrorKind::OutOfSync, "Composer resource server stopped or execution was disabled")),
            targets = self.ready_targets(&servers, supports_resources) => targets,
        };
        let target = targets.into_iter().next().ok_or_else(|| unsupported(&servers, "registered JSON translations"))?;
        let sources = self.navigation_snapshot(&target)?;
        if sources.files_revision != revision {
            return Err(IdeError::new(ErrorKind::OutOfSync, "Composer project changed while starting its server"));
        }
        let receipt = PhpJsonTranslationResource {
            project: composer.to_owned(),
            resource: resource.to_owned(),
            root: None,
            target,
            sources,
            composer: Some(Arc::new(inputs)),
        };
        self.php_json_translation_resource_current(&receipt, cancel.clone()).await?;
        self.query_json_translation_resource(receipt, cancel).await
    }

    async fn composer_resource_inputs(
        &self,
        composer: &Path,
        resource: &Path,
        cancel: &CancellationToken,
    ) -> IdeResult<(ComposerInputs, u64)> {
        if composer.file_name().is_none_or(|name| name != "composer.json")
            || !composer.is_absolute()
            || !composer.starts_with(self.project_root_for(composer))
        {
            return Err(IdeError::invalid("Select a Composer manifest in an opened project or content root"));
        }
        if resource.extension().is_none_or(|ext| ext != "json")
            || resource.file_stem().is_none_or(|stem| stem.is_empty())
        {
            return Err(IdeError::invalid("Expected a JSON locale file"));
        }
        let revision = self.project_files_revision.load(Ordering::Acquire);
        let buffers = self.composer_buffers(composer);
        let disk = composer_disk(composer, resource, &buffers, cancel).await?;
        if !self.composer_buffers_current(composer, &buffers)
            || self.project_files_revision.load(Ordering::Acquire) != revision
        {
            return Err(IdeError::new(ErrorKind::OutOfSync, "Composer project changed before resource lookup"));
        }
        Ok((ComposerInputs { disk, buffers }, revision))
    }

    pub(super) async fn json_resource_on_target(
        &self,
        composer: &Path,
        resource: &Path,
        source: &DocSnap,
        target: &Target,
        cancel: &CancellationToken,
    ) -> IdeResult<PhpJsonTranslationResource> {
        let (inputs, revision) = self.composer_resource_inputs(composer, resource, cancel).await?;
        let sources = self.navigation_sources(source, target)?;
        if sources.files_revision != revision || !self.execution_allowed.load(Ordering::Acquire) {
            return Err(IdeError::cancelled());
        }
        let receipt = PhpJsonTranslationResource {
            project: composer.to_owned(),
            resource: resource.to_owned(),
            root: None,
            target: target.clone(),
            sources,
            composer: Some(Arc::new(inputs)),
        };
        self.query_json_translation_resource(receipt, cancel.clone()).await
    }

    pub(super) async fn current_composer_resource_query(
        &self,
        receipt: &PhpJsonTranslationResource,
        params: &ResourceQuery,
        cancel: &CancellationToken,
    ) -> IdeResult<ResourceReply> {
        let method = "phpWorkbench/jsonTranslationResource";
        let deadline = tokio::time::Instant::now() + LONG_TIMEOUT;
        loop {
            tokio::time::timeout_at(deadline, self.php_json_translation_resource_current(receipt, cancel.clone()))
                .await
                .map_err(|_| rpc_err(&receipt.target.server.name, method, RpcError::Timeout))??;
            let response = tokio::time::timeout_at(
                deadline,
                receipt.target.run.peer.request::<_, ResourceReply>(method, params, Some(cancel.clone())),
            )
            .await
            .unwrap_or(Err(RpcError::Timeout));
            if cancel.is_cancelled() {
                return Err(IdeError::cancelled());
            }
            tokio::time::timeout_at(deadline, self.php_json_translation_resource_current(receipt, cancel.clone()))
                .await
                .map_err(|_| rpc_err(&receipt.target.server.name, method, RpcError::Timeout))??;
            match response {
                Err(RpcError::Server { code, .. }) if code == ide_jsonrpc::codes::CONTENT_MODIFIED => {
                    if tokio::time::Instant::now() + Duration::from_millis(50) >= deadline {
                        return Err(IdeError::cancelled());
                    }
                    tokio::select! {
                        () = cancel.cancelled() => return Err(IdeError::cancelled()),
                        () = tokio::time::sleep(Duration::from_millis(50)) => {}
                    }
                }
                result => return result.map_err(|error| rpc_err(&receipt.target.server.name, method, error)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ide_jsonrpc::{NoopHandler, Peer, framing};
    use ide_text::AppliedChange;
    use parking_lot::RwLock;

    const MANIFEST: &str = r#"{"require":{"laravel/framework":"13.37.0"}}"#;
    const PRIVATE: &str = r#"{"Greeting 😀":"Private catalog"}"#;

    fn replace(manager: &LspManager, doc: DocId, version: u32, before: &str, after: &str) {
        manager.did_change(
            doc,
            version,
            &[AppliedChange {
                range: ide_core::Range::new(
                    ide_core::Pos::new(0, 0),
                    ide_core::Pos::new(0, before.encode_utf16().count() as u32),
                ),
                text: after.into(),
            }],
            &|| after.into(),
        );
    }

    #[tokio::test]
    async fn json_publication_rejects_other_composer_changes_after_async_validation() {
        publication_after_validation(true).await;
    }

    #[tokio::test]
    async fn json_foreground_publication_rejects_composer_changes_after_async_validation() {
        publication_after_validation(false).await;
    }

    async fn publication_after_validation(background: bool) {
        for action in ["edit", "restore", "close", "open", "unchanged"] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().canonicalize().unwrap();
            let project = root.join("composer.json");
            let other = root.join("consumers/other/composer.json");
            std::fs::create_dir_all(other.parent().unwrap()).unwrap();
            std::fs::write(&project, MANIFEST).unwrap();
            std::fs::write(&other, MANIFEST).unwrap();
            let manager =
                LspManager::new(root.clone(), Arc::new(LanguageRegistry::from_configs(vec![])), Arc::new(|_| {}));
            let catalog = DocId::next();
            let resource = root.join("catalogs/en.json");
            manager.did_open(catalog, &resource, &LanguageId::new("json"), 1, PRIVATE);
            let composer = DocId::next();
            let edited_manifest = if background { &other } else { &project };
            if action != "open" {
                manager.did_open(composer, edited_manifest, &LanguageId::new("json"), 1, MANIFEST);
            }
            let registry = LanguageRegistry::load(None).unwrap();
            let php = LanguageId::new("php");
            let config =
                registry.get(&php).unwrap().language_servers.iter().find(|s| s.name == "phpantom").cloned().unwrap();
            let server = Arc::new(Server::new(config, root.clone(), php.clone()));
            let generation = server.begin_start().unwrap();
            let (client, wire) = tokio::io::duplex(65_536);
            let (read, write) = tokio::io::split(client);
            let run = Running {
                generation,
                peer: Peer::connect(read, write, Arc::new(NoopHandler)),
                caps: Arc::new(RwLock::new(ServerCaps::parse(json!({"experimental":{
                    "phpWorkbenchJsonTranslationResource":true,
                    "phpWorkbenchRegisteredJsonTranslations":true
                }})))),
                watchers: Arc::new(RwLock::new(Default::default())),
                diagnostic_slots: Arc::new(tokio::sync::Semaphore::new(1)),
            };
            assert!(server.mark_ready(generation, run.clone()));
            let caller = DocId::next();
            let second = DocId::next();
            for (doc, path) in [(caller, root.join("Caller.php")), (second, other.parent().unwrap().join("Caller.php"))]
            {
                manager.did_open(doc, &path, &php, 1, "<?php __('Greeting 😀');");
                manager.docs.lock().get_mut(&doc).unwrap().servers.push(Attached {
                    server: server.clone(),
                    opened_gen: Some(generation),
                    resource: None,
                });
            }
            let target = Target { server, run };
            let cancel = CancellationToken::new();
            let source = manager.snapshot(caller).unwrap();
            let (selected_inputs, _) = manager.composer_resource_inputs(&project, &resource, &cancel).await.unwrap();
            let (negative_inputs, _) = manager.composer_resource_inputs(&other, &resource, &cancel).await.unwrap();
            let mut selected = PhpJsonTranslationResource {
                project: project.clone(),
                resource: resource.clone(),
                root: Some(resource.parent().unwrap().to_owned()),
                target: target.clone(),
                sources: manager.navigation_sources(&source, &target).unwrap(),
                composer: Some(Arc::new(selected_inputs)),
            };
            let negative = PhpJsonTranslationResource {
                project: other.clone(),
                resource: resource.clone(),
                root: None,
                target: target.clone(),
                sources: manager.navigation_sources(&source, &target).unwrap(),
                composer: Some(Arc::new(negative_inputs)),
            };
            let inputs = manager.json_input_snapshot();
            manager.php_json_translation_resource_current(&selected, cancel.clone()).await.unwrap();
            manager.php_json_translation_resource_current(&negative, cancel.clone()).await.unwrap();
            let catalog = manager.snapshot(catalog).unwrap();
            match action {
                "edit" => replace(&manager, composer, 2, MANIFEST, "{}"),
                "restore" => {
                    replace(&manager, composer, 2, MANIFEST, "{}");
                    replace(&manager, composer, 3, "{}", MANIFEST);
                }
                "close" => manager.did_close(composer),
                "open" => manager.did_open(composer, edited_manifest, &LanguageId::new("json"), 1, MANIFEST),
                "unchanged" => {}
                _ => unreachable!(),
            }
            assert_eq!(
                manager.php_json_translation_resource_current(&selected, cancel.clone()).await.is_ok(),
                background || action == "unchanged"
            );
            assert_eq!(
                manager.php_json_translation_resource_current(&negative, cancel).await.is_ok(),
                !background || action == "unchanged"
            );
            let result = if background {
                manager.publish_refreshed_json_catalog(&catalog, &mut selected, &inputs)
            } else {
                manager.open_json_session(&catalog, &mut selected, &inputs).and_then(|session| session.retain())
            };
            let mut wire = tokio::io::BufReader::new(wire);
            if action == "unchanged" {
                result.unwrap();
                let notification = tokio::time::timeout(Duration::from_secs(1), framing::read_message(&mut wire))
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
                let notification: Value = serde_json::from_slice(&notification).unwrap();
                assert_eq!(notification["method"], "textDocument/didOpen");
                assert_eq!(notification["params"]["textDocument"]["text"], PRIVATE);
            } else {
                assert!(result.is_err(), "{action}: retired Composer proof must not forward private catalog text");
                assert_eq!(manager.json_resource_count.load(Ordering::Acquire), 0);
                target.run.peer.notify("test/publicationBarrier", json!({})).unwrap();
                let barrier = tokio::time::timeout(Duration::from_secs(1), framing::read_message(&mut wire))
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
                let barrier: Value = serde_json::from_slice(&barrier).unwrap();
                assert_eq!(barrier["method"], "test/publicationBarrier", "no private open may precede the barrier");
            }
            assert!(!resource.exists());
            target.run.peer.close_write();
            drop(wire);
            manager.shutdown_all().await;
        }
    }
}
