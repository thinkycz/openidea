use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct JsonRefreshKey {
    doc: DocId,
    pub(super) server: ide_core::ServerId,
    generation: u64,
}

pub(crate) struct JsonRefreshPublication<'a>(&'a LspManager);

impl Drop for JsonRefreshPublication<'_> {
    fn drop(&mut self) {
        self.0.json_refresh_publications.fetch_sub(1, Ordering::AcqRel);
        self.0.invalidate_json_preparation();
    }
}

struct RefreshTask {
    manager: std::sync::Weak<LspManager>,
    key: JsonRefreshKey,
    cancel: Arc<CancellationToken>,
}

impl Drop for RefreshTask {
    fn drop(&mut self) {
        if let Some(manager) = self.manager.upgrade() {
            let mut active = manager.json_refreshes.lock();
            if active.get(&self.key).is_some_and(|current| Arc::ptr_eq(current, &self.cancel)) {
                active.remove(&self.key);
            }
        }
    }
}

impl LspManager {
    pub(crate) fn retire_json_resources_for_publication(&self) -> JsonRefreshPublication<'_> {
        self.json_refresh_publications.fetch_add(1, Ordering::AcqRel);
        let publication = JsonRefreshPublication(self);
        let mut docs = self.docs.lock();
        if self.json_resource_count.load(Ordering::Acquire) != 0 {
            let ids: Vec<_> = docs
                .iter()
                .filter(|(_, doc)| {
                    doc.servers
                        .iter()
                        .any(|attached| attached.resource.as_ref().is_some_and(|resource| resource.persistent))
                })
                .map(|(id, _)| *id)
                .collect();
            for doc in ids {
                self.refresh_retained_json_document(&docs, doc);
            }
            self.retire_json_resources_in(&mut docs, |_, _, _, _| true);
        }
        self.invalidate_json_preparation();
        publication
    }

    pub(crate) fn cancel_json_refreshes(&self, affected: impl Fn(&JsonRefreshKey) -> bool) {
        self.json_refreshes.lock().retain(|key, cancel| {
            if affected(key) {
                cancel.cancel();
                false
            } else {
                true
            }
        });
    }

    pub(crate) fn cancel_json_refreshes_for_document(&self, doc: DocId) {
        self.cancel_json_refreshes(|key| key.doc == doc);
    }

    pub(super) fn cancel_json_refreshes_for_document_path(&self, path: &Path) {
        let ids: Vec<_> = self.docs.lock().iter().filter(|(_, doc)| doc.path == path).map(|(id, _)| *id).collect();
        self.cancel_json_refreshes(|key| ids.contains(&key.doc));
    }

    pub(crate) fn refresh_retained_json_document(&self, docs: &HashMap<DocId, DocState>, doc: DocId) {
        let Some(source) = docs.get(&doc) else { return };
        if !Self::json_catalog_path(&source.path, &source.uri, &source.language) {
            return;
        }
        for attached in &source.servers {
            let Some(resource) = &attached.resource else { continue };
            let Some(generation) = attached.opened_gen else { continue };
            if !resource.persistent || resource.cancel.is_cancelled() {
                continue;
            }
            let key = JsonRefreshKey { doc, server: attached.server.id, generation };
            let cancel = Arc::new(CancellationToken::new());
            if let Some(previous) = self.json_refreshes.lock().insert(key, cancel.clone()) {
                previous.cancel();
            }
            let task = RefreshTask { manager: self.me.clone(), key, cancel };
            let project = resource.selector.clone();
            let server = attached.server.clone();
            let mut changed = self.json_preparation_changed.subscribe();
            self.runtime.spawn(async move {
                loop {
                    changed.borrow_and_update();
                    tokio::select! {
                        biased;
                        () = task.cancel.cancelled() => return,
                        result = changed.changed() => { if result.is_err() { return; } continue; },
                        () = tokio::time::sleep(Duration::from_millis(100)) => {},
                    }
                    let Some(manager) = task.manager.upgrade() else { return };
                    let attempt = task.cancel.child_token();
                    let result = tokio::select! {
                        biased;
                        () = task.cancel.cancelled() => { attempt.cancel(); return; },
                        result = changed.changed() => { attempt.cancel(); if result.is_err() { return; } None },
                        result = manager.refresh_json_catalog(key, &project, &server, &attempt) => Some(result),
                    };
                    drop(manager);
                    match result {
                        Some(Ok(true)) => return,
                        Some(Err(error)) => {
                            tracing::debug!(kind = ?error.kind, "Retained JSON refresh awaits current inputs")
                        }
                        Some(Ok(false)) => {}
                        None => continue,
                    }
                    tokio::select! {
                        biased;
                        () = task.cancel.cancelled() => return,
                        result = changed.changed() => { if result.is_err() { return; } },
                    }
                }
            });
        }
    }

    async fn refresh_json_catalog(
        &self,
        key: JsonRefreshKey,
        project: &Path,
        server: &Arc<Server>,
        cancel: &CancellationToken,
    ) -> IdeResult<bool> {
        let _selection = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(IdeError::cancelled()),
            gate = self.php_json_resource_gate.lock() => gate,
        };
        if self.shutting_down.load(Ordering::Acquire) || !self.execution_allowed.load(Ordering::Acquire) {
            return Ok(true);
        }
        if self.json_refresh_publications.load(Ordering::Acquire) != 0 {
            return Ok(false);
        }
        let Some(run) = server.running_gen(key.generation) else { return Ok(true) };
        let target = Target { server: server.clone(), run };
        if server.name != "phpantom" || !supports_resources(&target.caps()) {
            return Ok(true);
        }
        let Ok(catalog) = self.snapshot(key.doc) else { return Ok(true) };
        if !Self::json_catalog_path(&catalog.path, &catalog.uri, &catalog.language)
            || !self.registered_json_candidate(&catalog)
        {
            return Ok(true);
        }
        let sources = {
            let docs = self.docs.lock();
            if docs.get(&key.doc).is_some_and(|doc| doc.servers.iter().any(|attached| attached.server.id == key.server))
            {
                return Ok(true);
            }
            let mut sources: Vec<_> = docs
                .iter()
                .filter(|(_, doc)| {
                    doc.language.as_str() == "php"
                        && doc.servers.iter().any(|attached| {
                            attached.resource.is_none()
                                && attached.server.id == key.server
                                && attached.opened_gen == Some(key.generation)
                        })
                })
                .map(|(id, doc)| (*id, doc.path.clone()))
                .collect();
            sources.sort_by(|left, right| left.1.cmp(&right.1));
            sources
        };
        for (doc, _) in sources {
            if cancel.is_cancelled() {
                return Err(IdeError::cancelled());
            }
            let Ok(source) = self.snapshot(doc) else { continue };
            if source.uri != uri::path_to_uri(&source.path)
                || self.json_resource_manifest(&source, cancel).await.ok().flatten().as_deref() != Some(project)
            {
                continue;
            }
            let mut receipt = self.json_resource_on_target(project, &catalog.path, &source, &target, cancel).await?;
            if receipt.root.is_none() {
                return Ok(false);
            }
            if cancel.is_cancelled() {
                return Err(IdeError::cancelled());
            }
            self.open_json_session(&catalog, &mut receipt)?.retain()?;
            return Ok(true);
        }
        Ok(true)
    }
}
