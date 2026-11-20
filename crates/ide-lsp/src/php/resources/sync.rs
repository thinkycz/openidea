use super::*;

struct Catalog {
    doc: DocId,
    snap: DocSnap,
}

struct Preparation {
    catalogs_revision: u64,
    resources: Vec<(ide_core::ServerId, u64)>,
}

struct CatalogProof {
    doc: DocId,
    server: ide_core::ServerId,
    witness: Option<Arc<JsonResourceWitness>>,
}

impl LspManager {
    pub(crate) fn invalidate_json_preparation(&self) {
        if self.json_preparation_changed.receiver_count() != 0 {
            self.json_preparation_changed.send_modify(|revision| *revision = revision.wrapping_add(1));
        }
    }

    pub(crate) fn record_json_catalog_change(&self) {
        self.json_catalog_revision.fetch_add(1, Ordering::AcqRel);
        self.invalidate_json_preparation();
    }

    pub(crate) fn json_catalog_path(path: &Path, uri: &lsp::Uri, language: &LanguageId) -> bool {
        language.as_str() == "json"
            && path.is_absolute()
            && path.extension().is_some_and(|ext| ext == "json")
            && path.file_name().is_some_and(|name| name != "composer.json")
            && *uri == uri::path_to_uri(path)
    }

    fn json_catalogs(&self) -> Vec<Catalog> {
        let ids: Vec<_> = self
            .docs
            .lock()
            .iter()
            .filter(|(_, doc)| Self::json_catalog_path(&doc.path, &doc.uri, &doc.language))
            .map(|(id, _)| *id)
            .collect();
        let mut catalogs: Vec<_> = ids
            .into_iter()
            .filter_map(|doc| {
                self.snapshot(doc)
                    .ok()
                    .filter(|snap| self.registered_json_candidate(snap))
                    .map(|snap| Catalog { doc, snap })
            })
            .collect();
        catalogs.sort_by(|left, right| left.snap.path.cmp(&right.snap.path));
        catalogs
    }

    fn json_catalogs_current(&self, before: &[Catalog]) -> bool {
        let after = self.json_catalogs();
        before.len() == after.len()
            && before.iter().zip(&after).all(|(before, after)| {
                before.doc == after.doc
                    && before.snap.uri == after.snap.uri
                    && before.snap.version == after.snap.version
                    && Arc::ptr_eq(&before.snap.text, &after.snap.text)
            })
    }

    async fn attached_json_current(
        &self,
        catalog: &Catalog,
        target: &Target,
        composer: &Path,
        cancel: &CancellationToken,
    ) -> IdeResult<Option<Arc<JsonResourceWitness>>> {
        let revision = target.server.resource_revision.load(Ordering::Acquire);
        let witness = self
            .docs
            .lock()
            .get(&catalog.doc)
            .and_then(|doc| {
                doc.servers.iter().find(|attached| {
                    attached.server.id == target.server.id && attached.opened_gen == Some(target.run.generation)
                })
            })
            .and_then(|attached| attached.resource.as_ref())
            .filter(|resource| !resource.cancel.is_cancelled())
            .map(|resource| resource.witness.clone());
        let Some(witness) = witness else {
            return Ok(None);
        };
        if witness.project != composer {
            self.retire_json_resources_in(&mut self.docs.lock(), |doc, _, _, attached| {
                doc == catalog.doc && attached.server.id == target.server.id
            });
            return Ok(None);
        }
        let valid = if let Some(composer) = &witness.composer {
            self.composer_inputs_current(&witness.project, &witness.resource, composer, cancel).await
        } else {
            resource_authority(&witness.project, &witness.resource, cancel).await
        };
        if let Err(error) = valid {
            self.retire_json_resources_in(&mut self.docs.lock(), |doc, _, _, attached| {
                doc == catalog.doc && attached.server.id == target.server.id
            });
            return Err(error);
        }
        if cancel.is_cancelled()
            || revision != target.server.resource_revision.load(Ordering::Acquire)
            || target.server.running_gen(target.run.generation).is_none()
            || !self.execution_allowed.load(Ordering::Acquire)
        {
            return Err(IdeError::cancelled());
        }
        Ok(Some(witness))
    }

    pub(crate) async fn sync_php_json_resources(
        &self,
        source: &mut DocSnap,
        targets: &[Target],
        cancel: &CancellationToken,
    ) -> IdeResult<()> {
        if source.language.as_str() != "php"
            || !targets.iter().any(|target| target.server.name == "phpantom" && supports_resources(&target.caps()))
        {
            return Ok(());
        }
        let mut changed = self.json_preparation_changed.subscribe();
        let prepared = tokio::select! {
            biased;
            () = cancel.cancelled() => Err(IdeError::cancelled()),
            _ = changed.changed() => Err(IdeError::cancelled()),
            result = self.prepare_php_json_resources(source, targets, cancel) => result,
        }?;
        if let Some(prepared) = prepared {
            source.catalogs_revision = Some(prepared.catalogs_revision);
            source.prepared_resources = prepared.resources;
        }
        Ok(())
    }

    async fn prepare_php_json_resources(
        &self,
        source: &DocSnap,
        targets: &[Target],
        cancel: &CancellationToken,
    ) -> IdeResult<Option<Preparation>> {
        if source.language.as_str() != "php" || source.uri != uri::path_to_uri(&source.path) {
            return Ok(None);
        }
        let selected: Vec<_> = targets
            .iter()
            .filter(|target| target.server.name == "phpantom" && supports_resources(&target.caps()))
            .collect();
        if selected.is_empty() {
            return Ok(None);
        }
        let revision = self.project_files_revision.load(Ordering::Acquire);
        let catalogs_revision = self.json_catalog_revision.load(Ordering::Acquire);
        let catalogs = self.json_catalogs();
        if catalogs_revision != self.json_catalog_revision.load(Ordering::Acquire) {
            return Err(IdeError::cancelled());
        }
        if catalogs.is_empty() {
            return self.finish_json_preparation(&selected, &catalogs, &[], revision, catalogs_revision).map(Some);
        }
        let path = source.path.clone();
        let boundary = self.project_root_for(&path);
        let worker_cancel = cancel.clone();
        let composer = tokio::select! {biased;()=cancel.cancelled()=>return Err(IdeError::cancelled()),result=tokio::task::spawn_blocking(move||{
            physical_path(&path,false)?;
            for directory in path.ancestors().skip(1).take_while(|directory|directory.starts_with(&boundary)) {
                if worker_cancel.is_cancelled(){return Err(IdeError::cancelled());}
                let manifest=directory.join("composer.json");
                if std::fs::symlink_metadata(&manifest).is_ok(){return Ok(Some(manifest));}
            }
            Ok(None)
        })=>result.map_err(|error|IdeError::internal(error.to_string()))??};
        let Some(composer) = composer else {
            self.retire_json_resources_in(&mut self.docs.lock(), |doc, _, _, attached| {
                catalogs.iter().any(|catalog| catalog.doc == doc)
                    && selected.iter().any(|target| target.server.id == attached.server.id)
            });
            let proofs: Vec<_> = selected
                .iter()
                .flat_map(|target| {
                    catalogs.iter().map(move |catalog| CatalogProof {
                        doc: catalog.doc,
                        server: target.server.id,
                        witness: None,
                    })
                })
                .collect();
            return self.finish_json_preparation(&selected, &catalogs, &proofs, revision, catalogs_revision).map(Some);
        };
        let mut proofs = Vec::new();
        let mut complete = true;
        for target in &selected {
            for catalog in &catalogs {
                let witness = self.attached_json_current(catalog, target, &composer, cancel).await?;
                complete &= witness.is_some();
                proofs.push(CatalogProof { doc: catalog.doc, server: target.server.id, witness });
            }
        }
        if complete {
            if !self.json_catalogs_current(&catalogs)
                || self.project_files_revision.load(Ordering::Acquire) != revision
                || self.json_catalog_revision.load(Ordering::Acquire) != catalogs_revision
            {
                return Err(IdeError::cancelled());
            }
            for target in &selected {
                self.navigation_sources(source, target)?;
            }
            return self.finish_json_preparation(&selected, &catalogs, &proofs, revision, catalogs_revision).map(Some);
        }
        let _selection = tokio::select! {biased;()=cancel.cancelled()=>return Err(IdeError::cancelled()),gate=self.php_json_resource_gate.lock()=>gate};
        if !self.json_catalogs_current(&catalogs)
            || self.project_files_revision.load(Ordering::Acquire) != revision
            || self.json_catalog_revision.load(Ordering::Acquire) != catalogs_revision
        {
            return Err(IdeError::cancelled());
        }
        let mut opened = Vec::new();
        proofs.clear();
        for target in &selected {
            for catalog in &catalogs {
                if let Some(witness) = self.attached_json_current(catalog, target, &composer, cancel).await? {
                    proofs.push(CatalogProof { doc: catalog.doc, server: target.server.id, witness: Some(witness) });
                    continue;
                }
                let mut receipt =
                    match self.json_resource_on_target(&composer, &catalog.snap.path, source, target, cancel).await {
                        Ok(receipt) => receipt,
                        Err(error) if error.kind == ErrorKind::InvalidInput => {
                            proofs.push(CatalogProof { doc: catalog.doc, server: target.server.id, witness: None });
                            continue;
                        }
                        Err(error) => return Err(error),
                    };
                if cancel.is_cancelled() {
                    return Err(IdeError::cancelled());
                }
                let witness = if receipt.root.is_some() {
                    let session = self.open_json_session(&catalog.snap, &mut receipt)?;
                    let witness = session.witness.clone();
                    opened.push(session);
                    Some(witness)
                } else {
                    None
                };
                proofs.push(CatalogProof { doc: catalog.doc, server: target.server.id, witness });
            }
        }
        if cancel.is_cancelled()
            || !self.json_catalogs_current(&catalogs)
            || self.project_files_revision.load(Ordering::Acquire) != revision
            || self.json_catalog_revision.load(Ordering::Acquire) != catalogs_revision
        {
            return Err(IdeError::cancelled());
        }
        for target in &selected {
            self.navigation_sources(source, target)?;
        }
        let prepared = self.finish_json_preparation(&selected, &catalogs, &proofs, revision, catalogs_revision)?;
        for session in opened {
            session.retain()?;
        }
        Ok(Some(prepared))
    }

    fn finish_json_preparation(
        &self,
        targets: &[&Target],
        catalogs: &[Catalog],
        proofs: &[CatalogProof],
        files_revision: u64,
        catalogs_revision: u64,
    ) -> IdeResult<Preparation> {
        let docs = self.docs.lock();
        if self.project_files_revision.load(Ordering::Acquire) != files_revision
            || self.json_catalog_revision.load(Ordering::Acquire) != catalogs_revision
            || !self.execution_allowed.load(Ordering::Acquire)
            || targets.iter().any(|target| target.server.running_gen(target.run.generation).is_none())
            || catalogs.iter().any(|catalog| {
                !docs.get(&catalog.doc).is_some_and(|doc| {
                    doc.uri == catalog.snap.uri
                        && doc.version == catalog.snap.version
                        && Arc::ptr_eq(&doc.text, &catalog.snap.text)
                })
            })
            || proofs.iter().any(|proof| {
                let current = docs
                    .get(&proof.doc)
                    .and_then(|doc| doc.servers.iter().find(|attached| attached.server.id == proof.server))
                    .and_then(|attached| attached.resource.as_ref());
                match (&proof.witness, current) {
                    (Some(expected), Some(current)) => {
                        current.cancel.is_cancelled() || !Arc::ptr_eq(expected, &current.witness)
                    }
                    (None, None) => false,
                    _ => true,
                }
            })
        {
            return Err(IdeError::cancelled());
        }
        Ok(Preparation {
            catalogs_revision,
            resources: targets
                .iter()
                .map(|target| (target.server.id, target.server.resource_revision.load(Ordering::Acquire)))
                .collect(),
        })
    }
}
