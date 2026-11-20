use super::*;

struct ResourceSession<'a> {
    manager: &'a LspManager,
    target: &'a Target,
    path: PathBuf,
    uri: lsp::Uri,
}

impl Drop for ResourceSession<'_> {
    fn drop(&mut self) {
        let _docs = self.manager.docs.lock();
        if self.target.server.release_transient(&self.path, self.target.run.generation) {
            let _ = self.target.run.peer.notify("textDocument/didClose", json!({"textDocument":{"uri":self.uri}}));
        }
    }
}

impl LspManager {
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

    fn open_json_session<'a>(
        &'a self,
        snap: &DocSnap,
        receipt: &'a PhpJsonTranslationResource,
    ) -> IdeResult<ResourceSession<'a>> {
        let target = &receipt.target;
        let docs = self.docs.lock();
        if !docs.values().any(|doc| {
            doc.path == snap.path
                && doc.uri == snap.uri
                && doc.version == snap.version
                && Arc::ptr_eq(&doc.text, &snap.text)
        }) || docs.values().filter(|doc| doc.canonical == snap.path).count() != 1
            || docs.values().any(|doc| doc.canonical == snap.path && Self::navigation_attached(doc, target))
            || !Self::navigation_documents_current(&docs, target, &receipt.sources)
            || self.project_files_revision.load(Ordering::Acquire) != receipt.sources.files_revision
            || !self.execution_allowed.load(Ordering::Acquire)
            || target.server.running_gen(target.run.generation).is_none()
        {
            return Err(IdeError::new(ErrorKind::OutOfSync, "JSON resource session changed before opening"));
        }
        if !target.server.hold_transient(&snap.path, target.run.generation) {
            target.server.release_transient(&snap.path, target.run.generation);
            return Err(IdeError::new(ErrorKind::OutOfSync, "JSON resource already has a temporary owner"));
        }
        let session = ResourceSession { manager: self, target, path: snap.path.clone(), uri: snap.uri.clone() };
        let params = json!({"textDocument":{"uri":snap.uri,"languageId":"json","version":snap.version,"text":snap.text.as_str()}});
        let sent = target.run.peer.notify("textDocument/didOpen", params);
        // Drop must be able to acquire the documents lock after a failed send.
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
        let mut receipts = Vec::new();
        let mut found: Option<References> = None;
        for manifest in manifests {
            self.json_buffer_current(snap)?;
            if self.project_files_revision.load(Ordering::Acquire) != revision {
                return Err(IdeError::new(ErrorKind::OutOfSync, "JSON consuming projects changed during Find Usages"));
            }
            let receipt = match self.json_resource_from_composer(&manifest, &snap.path, cancel.clone()).await {
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
            let locations = {
                let _session = self.open_json_session(snap, &receipt)?;
                self.json_session_references(snap, &receipt, pos, include_declaration, cancel).await?
            };
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
