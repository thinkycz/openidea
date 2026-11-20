use super::*;

pub(super) struct ResourceSession<'a> {
    manager: &'a LspManager,
    target: Target,
    doc: DocId,
    cancel: CancellationToken,
    retained: bool,
    pub(super) witness: Arc<JsonResourceWitness>,
}

impl ResourceSession<'_> {
    pub(super) fn retain(mut self) -> IdeResult<()> {
        let mut docs = self.manager.docs.lock();
        let resource = docs
            .get_mut(&self.doc)
            .and_then(|doc| {
                doc.servers.iter_mut().find(|attached| {
                    attached.server.id == self.target.server.id
                        && attached.opened_gen == Some(self.target.run.generation)
                })
            })
            .and_then(|attached| attached.resource.as_mut())
            .ok_or_else(IdeError::cancelled)?;
        if resource.cancel.is_cancelled() || self.cancel.is_cancelled() {
            return Err(IdeError::cancelled());
        }
        resource.persistent = true;
        self.retained = true;
        Ok(())
    }

    fn finish(self, receipt: &mut PhpJsonTranslationResource) -> IdeResult<()> {
        let mut docs = self.manager.docs.lock();
        if self.cancel.is_cancelled()
            || !LspManager::navigation_documents_current(&docs, &self.target, &receipt.sources)
            || self.manager.project_files_revision.load(Ordering::Acquire) != receipt.sources.files_revision
            || self.target.server.running_gen(self.target.run.generation).is_none()
            || !self.manager.execution_allowed.load(Ordering::Acquire)
        {
            return Err(IdeError::cancelled());
        }
        self.manager.retire_json_resources_in(&mut docs, |doc, _, _, attached| {
            doc == self.doc
                && attached.server.id == self.target.server.id
                && attached.opened_gen == Some(self.target.run.generation)
        });
        // Account only for this operation's own overlay; never refresh PHP or disk inputs.
        receipt.sources.sources.retain(|source| source.doc != self.doc);
        receipt.sources.resources_revision = self.target.server.resource_revision.load(Ordering::Acquire);
        drop(docs);
        Ok(())
    }
}

impl Drop for ResourceSession<'_> {
    fn drop(&mut self) {
        if self.retained {
            return;
        }
        self.manager.retire_json_resources_in(&mut self.manager.docs.lock(), |doc, _, _, attached| {
            doc == self.doc
                && attached.server.id == self.target.server.id
                && attached.opened_gen == Some(self.target.run.generation)
        });
    }
}

impl LspManager {
    pub(crate) fn retire_json_resources_in(
        &self,
        docs: &mut HashMap<DocId, DocState>,
        affected: impl Fn(DocId, &Path, &Path, &Attached) -> bool,
    ) {
        if self.json_resource_count.load(Ordering::Acquire) == 0 {
            return;
        }
        for (id, doc) in docs.iter_mut() {
            let (path, canonical, uri) = (&doc.path, &doc.canonical, &doc.uri);
            doc.servers.retain(|attached| {
                let Some(resource) = &attached.resource else { return true };
                if !affected(*id, path, canonical, attached) {
                    return true;
                }
                resource.cancel.cancel();
                if let Some(generation) = attached.opened_gen
                    && attached.server.release_transient(path, generation)
                    && let Some(run) = attached.server.running_gen(generation)
                {
                    let _ = run.peer.notify("textDocument/didClose", json!({"textDocument":{"uri":uri}}));
                }
                attached.server.resource_revision.fetch_add(1, Ordering::AcqRel);
                self.json_resource_count.fetch_sub(1, Ordering::AcqRel);
                false
            });
        }
    }

    pub(crate) fn retire_json_resources(&self) {
        self.invalidate_json_preparation();
        if self.json_resource_count.load(Ordering::Acquire) != 0 {
            self.retire_json_resources_in(&mut self.docs.lock(), |_, _, _, _| true);
        }
    }

    pub(crate) fn retire_json_resources_for_server(&self, server: ide_core::ServerId) {
        if self.json_resource_count.load(Ordering::Acquire) != 0 {
            self.retire_json_resources_in(&mut self.docs.lock(), |_, _, _, attached| attached.server.id == server);
        }
    }

    pub(crate) fn retire_json_resources_for_source(
        &self,
        docs: &mut HashMap<DocId, DocState>,
        doc: DocId,
        path: &Path,
        canonical: &Path,
        servers: &[ide_core::ServerId],
    ) {
        self.retire_json_resources_in(docs, |id, _, resource_path, attached| {
            id == doc
                || resource_path == canonical
                || servers.contains(&attached.server.id)
                || attached
                    .resource
                    .as_ref()
                    .is_some_and(|resource| resource.selector == path || resource.selector == canonical)
        });
    }

    pub(crate) fn retire_json_resources_for_document(&self, docs: &mut HashMap<DocId, DocState>, doc: DocId) {
        if docs.get(&doc).is_some_and(|source| Self::json_catalog_path(&source.path, &source.uri, &source.language)) {
            self.record_json_catalog_change();
        } else if docs.get(&doc).is_some_and(|source| {
            source.language.as_str() == "php" || source.path.file_name().is_some_and(|name| name == "composer.json")
        }) {
            self.invalidate_json_preparation();
        }
        if self.json_resource_count.load(Ordering::Acquire) == 0 {
            return;
        }
        let Some(source) = docs.get(&doc) else { return };
        let (path, canonical) = (source.path.clone(), source.canonical.clone());
        let servers: Vec<_> = source
            .servers
            .iter()
            .filter(|attached| attached.resource.is_none())
            .map(|attached| attached.server.id)
            .collect();
        self.retire_json_resources_for_source(docs, doc, &path, &canonical, &servers);
    }

    pub(crate) fn registered_json_candidate(&self, snap: &DocSnap) -> bool {
        snap.language.as_str() == "json"
            && snap.path.extension().is_some_and(|ext| ext == "json")
            && !snap.servers.iter().any(|server| server.name == "phpantom")
    }

    fn json_buffer_current(&self, snap: &DocSnap) -> IdeResult<()> {
        let docs = self.docs.lock();
        if !docs.values().any(|doc| {
            doc.path == snap.path
                && doc.uri == snap.uri
                && doc.version == snap.version
                && doc.language == snap.language
                && Arc::ptr_eq(&doc.text, &snap.text)
        }) {
            return Err(IdeError::new(ErrorKind::OutOfSync, "JSON catalog buffer changed during Find Usages"));
        }
        Ok(())
    }

    pub(super) fn open_json_session<'a>(
        &'a self,
        snap: &DocSnap,
        receipt: &mut PhpJsonTranslationResource,
    ) -> IdeResult<ResourceSession<'a>> {
        let target = &receipt.target;
        let mut docs = self.docs.lock();
        let doc_id = docs.iter().find_map(|(id, doc)| {
            (doc.path == snap.path
                && doc.uri == snap.uri
                && doc.version == snap.version
                && Arc::ptr_eq(&doc.text, &snap.text))
            .then_some(*id)
        });
        if doc_id.is_none()
            || snap.uri != uri::path_to_uri(&snap.path)
            || docs.values().filter(|doc| doc.canonical == snap.path).count() != 1
            || docs.values().any(|doc| {
                doc.canonical == snap.path && doc.servers.iter().any(|attached| attached.server.id == target.server.id)
            })
            || !Self::navigation_documents_current(&docs, target, &receipt.sources)
            || self.project_files_revision.load(Ordering::Acquire) != receipt.sources.files_revision
            || !self.execution_allowed.load(Ordering::Acquire)
            || self.shutting_down.load(Ordering::Acquire)
            || !supports_resources(&target.caps())
            || target.server.running_gen(target.run.generation).is_none()
        {
            return Err(IdeError::new(ErrorKind::OutOfSync, "JSON resource session changed before opening"));
        }
        if !target.server.hold_transient(&snap.path, target.run.generation) {
            target.server.release_transient(&snap.path, target.run.generation);
            return Err(IdeError::new(ErrorKind::OutOfSync, "JSON resource already has a temporary owner"));
        }
        let doc_id = doc_id.expect("current JSON resource document");
        let cancel = CancellationToken::new();
        let witness = Arc::new(JsonResourceWitness {
            project: receipt.project.clone(),
            resource: receipt.resource.clone(),
            composer: receipt.composer.clone(),
        });
        docs.get_mut(&doc_id).expect("current JSON resource document").servers.push(Attached {
            server: target.server.clone(),
            opened_gen: Some(target.run.generation),
            resource: Some(JsonResourceAttachment {
                selector: receipt.project.clone(),
                cancel: cancel.clone(),
                persistent: false,
                witness: witness.clone(),
            }),
        });
        self.json_resource_count.fetch_add(1, Ordering::AcqRel);
        receipt.sources.resources_revision = target.server.resource_revision.fetch_add(1, Ordering::AcqRel) + 1;
        receipt.sources.sources.push(NavigationSource {
            doc: doc_id,
            version: snap.version,
            uri: snap.uri.clone(),
            language: snap.language.clone(),
            text: snap.text.clone(),
        });
        let session =
            ResourceSession { manager: self, target: target.clone(), doc: doc_id, cancel, retained: false, witness };
        let params = json!({"textDocument":{"uri":snap.uri,"languageId":"json","version":snap.version,"text":snap.text.as_str()}});
        let sent = target.run.peer.notify("textDocument/didOpen", params);
        drop(docs);
        sent.map_err(|error| rpc_err(&target.server.name, "textDocument/didOpen", error))?;
        Ok(session)
    }

    /// Explicit project-wide Find Usages only. Custom catalogs are sent after
    /// admission and closed before another consuming context may select them.
    pub(crate) async fn registered_json_references(
        &self,
        snap: &DocSnap,
        pos: Pos,
        include_declaration: bool,
        cancel: &CancellationToken,
    ) -> IdeResult<Option<References>> {
        let _selection = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(IdeError::cancelled()),
            gate = self.php_json_resource_gate.lock() => gate,
        };
        self.retire_json_resources_in(&mut self.docs.lock(), |_, path, _, attached| {
            path == snap.path && attached.resource.as_ref().is_some_and(|resource| resource.persistent)
        });
        self.json_buffer_current(snap)?;
        if !self.execution_allowed.load(Ordering::Acquire) {
            return Ok(None);
        }
        let revision = self.project_files_revision.load(Ordering::Acquire);
        let mut manifests = vec![self.project_root.join("composer.json")];
        manifests.extend(self.content_roots.read().iter().map(|root| root.join("composer.json")));
        manifests.sort();
        manifests.dedup();
        let worker_cancel = cancel.clone();
        let (manifests, mut skipped) = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(IdeError::cancelled()),
            result = tokio::task::spawn_blocking(move || {
                let mut found = Vec::new();
                let mut skipped = false;
                for path in manifests {
                    if worker_cancel.is_cancelled() { break; }
                    match std::fs::symlink_metadata(&path) {
                        Ok(_) => found.push(path),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
                        Err(_) => skipped = true,
                    }
                }
                (found, skipped)
            }) => result.map_err(|error| IdeError::internal(error.to_string()))?,
        };
        let mut receipts: Vec<PhpJsonTranslationResource> = Vec::new();
        let mut found: Option<References> = None;
        for manifest in manifests {
            self.json_buffer_current(snap)?;
            if self.project_files_revision.load(Ordering::Acquire) != revision {
                return Err(IdeError::new(ErrorKind::OutOfSync, "JSON consuming projects changed during Find Usages"));
            }
            let mut receipt = match self.json_resource_from_composer(&manifest, &snap.path, cancel.clone()).await {
                Ok(receipt) => receipt,
                Err(error) if matches!(error.kind, ErrorKind::Unavailable | ErrorKind::InvalidInput) => {
                    skipped = true;
                    continue;
                }
                Err(error) => return Err(error),
            };
            if receipt.root.is_none() {
                receipts.push(receipt);
                continue;
            }
            self.php_json_translation_resource_current(&receipt, cancel.clone()).await?;
            self.json_buffer_current(snap)?;
            if !receipt.target.caps().has("referencesProvider") {
                return Err(unsupported(std::slice::from_ref(&receipt.target.server), "find references"));
            }
            let session = self.open_json_session(snap, &mut receipt)?;
            let locations =
                self.json_session_references(snap, &receipt, pos, include_declaration, cancel, &session.cancel).await?;
            session.finish(&mut receipt)?;
            for previous in &mut receipts {
                if previous.target.server.id == receipt.target.server.id
                    && previous.target.run.generation == receipt.target.run.generation
                {
                    previous.sources.resources_revision = receipt.sources.resources_revision;
                }
            }
            let result = found.get_or_insert_with(References::default);
            if locations.is_empty() && receipt.target.server.indexing() {
                result.incomplete = Some(ReferencesIncomplete::Indexing);
            }
            result.locations.extend(locations);
            receipts.push(receipt);
        }
        self.json_buffer_current(snap)?;
        for receipt in &receipts {
            self.php_json_translation_resource_current(receipt, cancel.clone()).await?;
        }
        for receipt in &receipts {
            self.laravel_query_current(&receipt.target, &receipt.sources)?;
        }
        self.json_buffer_current(snap)?;
        if self.project_files_revision.load(Ordering::Acquire) != revision {
            return Err(IdeError::new(ErrorKind::OutOfSync, "JSON consuming projects changed during Find Usages"));
        }
        if let Some(found) = &mut found {
            if skipped {
                found.incomplete = found.incomplete.or(Some(ReferencesIncomplete::ServerFailed));
            }
            convert::dedup_locations(&mut found.locations);
        }
        Ok(found)
    }

    async fn json_session_references(
        &self,
        snap: &DocSnap,
        receipt: &PhpJsonTranslationResource,
        pos: Pos,
        include_declaration: bool,
        cancel: &CancellationToken,
        session_cancel: &CancellationToken,
    ) -> IdeResult<Vec<Location>> {
        let target = &receipt.target;
        let server_cancel = target.server.start_token();
        let deadline = tokio::time::Instant::now() + LONG_TIMEOUT;
        let mut params = snap.pos_params(target, pos);
        params["context"] = json!({"includeDeclaration":include_declaration});
        loop {
            self.php_json_translation_resource_current(receipt, cancel.clone()).await?;
            self.json_buffer_current(snap)?;
            let response = tokio::select! {
                biased;
                () = cancel.cancelled() => return Err(IdeError::cancelled()),
                () = server_cancel.cancelled() => return Err(IdeError::cancelled()),
                () = session_cancel.cancelled() => return Err(IdeError::cancelled()),
                response = tokio::time::timeout_at(deadline, target.run.peer.request::<_, Option<Vec<lsp::Location>>>(
                    "textDocument/references", &params, Some(cancel.clone())
                )) => response.unwrap_or(Err(RpcError::Timeout)),
            };
            self.php_json_translation_resource_current(receipt, cancel.clone()).await?;
            self.json_buffer_current(snap)?;
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
                response => {
                    let locations =
                        response.map_err(|error| rpc_err(&target.server.name, "textDocument/references", error))?;
                    return Ok(locations
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|location| self.conv(target.enc()).location(location))
                        .collect());
                }
            }
        }
    }
}
