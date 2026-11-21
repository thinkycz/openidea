use super::*;

pub(super) fn supported(caps: &ServerCaps) -> bool {
    providers::supported(caps)
        && caps.raw.pointer("/experimental/phpWorkbenchProviderSourceSync").and_then(Value::as_bool) == Some(true)
}

impl LspManager {
    // One PHP peer can serve several live Composer projects. Revalidate their
    // existing ownership before withdrawing a previously admitted provider copy.
    pub(super) async fn reconcile_provider_sources(
        &self,
        target: &Target,
        project: &Path,
        cancel: &CancellationToken,
    ) -> IdeResult<()> {
        if !supported(&target.caps()) {
            self.retire_json_resources_in(&mut self.docs.lock(), |_, _, _, attached| {
                attached.server.id == target.server.id
                    && attached.resource.as_ref().is_some_and(|resource| resource.provider)
            });
            return Ok(());
        }
        let (owners, consumers) = {
            let docs = self.docs.lock();
            let mut owners = std::collections::BTreeMap::<_, Vec<_>>::new();
            let mut consumers = Vec::new();
            for (id, doc) in docs.iter() {
                for attached in &doc.servers {
                    if attached.server.id != target.server.id || attached.opened_gen != Some(target.run.generation) {
                        continue;
                    }
                    match &attached.resource {
                        Some(resource) if resource.provider && resource.selector != project => {
                            owners.entry(resource.selector.clone()).or_default().push((
                                *id,
                                doc.path.clone(),
                                resource.witness.clone(),
                            ));
                        }
                        None if doc.language.as_str() == "php"
                            && doc.notebook.is_none()
                            && doc.uri == uri::path_to_uri(&doc.path) =>
                        {
                            consumers.push((*id, doc.path.clone()))
                        }
                        _ => {}
                    }
                }
            }
            (owners, consumers)
        };
        if owners.is_empty() {
            return Ok(());
        }
        let mut decisions = Vec::new();
        let mut checked = Vec::new();
        for (selector, resources) in owners {
            let mut source = None;
            for (doc, path) in &consumers {
                if !selector.parent().is_some_and(|root| path.starts_with(root)) {
                    continue;
                }
                match self.json_resource_manifest(path, cancel).await {
                    Ok(Some(manifest)) if manifest == selector => {
                        if let Ok(current) = self.snapshot(*doc)
                            && current.path == *path
                            && current.language.as_str() == "php"
                        {
                            source = Some(current);
                            break;
                        }
                    }
                    _ if cancel.is_cancelled() => return Err(IdeError::cancelled()),
                    _ => {}
                }
            }
            let receipt = if let Some(source) = source {
                let probe = source.path.with_extension("json");
                if self.composer_resource_inputs(&selector, &probe, cancel).await.is_ok() {
                    let receipt =
                        self.query_composer_resource_on_target(&selector, &probe, &source, target, cancel).await?;
                    if receipt.provider_sources.as_ref().is_none_or(|sources| sources.limited) {
                        return Err(IdeError::cancelled());
                    }
                    Some(receipt)
                } else {
                    None
                }
            } else {
                None
            };
            for (doc, path, witness) in resources {
                // A temporarily unresolved registered class still needs its preferred buffer.
                let keep =
                    receipt.as_ref().and_then(|receipt| receipt.provider_sources.as_ref()).is_some_and(|sources| {
                        sources.providers.iter().any(|provider| provider.path == path)
                            || witness.provider_sources.as_ref().is_some_and(|previous| {
                                previous.providers.iter().any(|provider| {
                                    provider.path == path
                                        && sources
                                            .unresolved
                                            .iter()
                                            .any(|name| name.eq_ignore_ascii_case(&provider.class_name))
                                })
                            })
                    });
                decisions.push((doc, witness, keep));
            }
            if let Some(receipt) = receipt {
                checked.push(receipt);
            }
        }
        if cancel.is_cancelled() {
            return Err(IdeError::cancelled());
        }
        for receipt in &checked {
            self.php_json_translation_resource_current(receipt, cancel.clone()).await?;
        }
        self.retire_json_resources_in(&mut self.docs.lock(), |doc, _, _, attached| {
            attached.server.id == target.server.id
                && attached.resource.as_ref().is_some_and(|resource| {
                    resource.provider
                        && decisions.iter().any(|(owner, witness, keep)| {
                            *owner == doc && !keep && Arc::ptr_eq(witness, &resource.witness)
                        })
                })
        });
        Ok(())
    }

    fn provider_source_candidates(&self, receipt: &PhpJsonTranslationResource) -> IdeResult<Vec<DocSnap>> {
        let Some(sources) = receipt.provider_sources.as_ref() else { return Ok(Vec::new()) };
        let docs = self.docs.lock();
        let mut count = 0usize;
        let mut bytes = 0usize;
        for doc in docs.values() {
            if doc.servers.iter().any(|attached| {
                attached.server.id == receipt.target.server.id
                    && attached.resource.as_ref().is_some_and(|resource| resource.provider)
            }) {
                count += 1;
                bytes = bytes.saturating_add(doc.text.as_str().len());
            }
        }
        let mut paths = HashSet::new();
        let mut ids = Vec::new();
        for source in &sources.providers {
            if !paths.insert(&source.path) {
                continue;
            }
            let matches: Vec<_> =
                docs.iter().filter(|(_, doc)| doc.path == source.path || doc.canonical == source.path).collect();
            let [(id, doc)] = matches.as_slice() else {
                if matches.is_empty() {
                    continue;
                }
                return Err(IdeError::invalid("Provider source has ambiguous editor owners"));
            };
            if doc.path != source.path
                || doc.uri != uri::path_to_uri(&source.path)
                || doc.language.as_str() != "php"
                || doc.notebook.is_some()
                || doc.text.as_str().len() as u64 > providers::MAX_BYTES
            {
                return Err(IdeError::invalid("Provider source requires one bounded physical PHP buffer"));
            }
            if doc.servers.iter().any(|attached| attached.server.id == receipt.target.server.id) {
                continue;
            }
            count += 1;
            bytes = bytes.saturating_add(doc.text.as_str().len());
            ids.push(**id);
        }
        if count > providers::MAX_PROVIDERS || bytes > providers::MAX_TOTAL_BYTES {
            return Err(IdeError::invalid("Provider buffers exceed the synchronization budget"));
        }
        drop(docs);
        ids.into_iter().map(|id| self.snapshot(id)).collect()
    }

    /// Only navigation opts into sharing verified provider dependencies. Public
    /// metadata lookups remain content-free, and unrelated buffers stay private.
    pub(super) async fn synchronize_provider_sources(
        &self,
        mut receipt: PhpJsonTranslationResource,
        cancel: &CancellationToken,
    ) -> IdeResult<PhpJsonTranslationResource> {
        if !supported(&receipt.target.caps()) {
            return Ok(receipt);
        }
        let inputs = self.json_input_snapshot();
        for round in 0..=8 {
            self.php_json_translation_resource_current(&receipt, cancel.clone()).await?;
            let candidates = self.provider_source_candidates(&receipt)?;
            if candidates.is_empty() {
                return Ok(receipt);
            }
            if round == 8 {
                return Err(IdeError::unavailable("Provider dependency discovery exceeds the synchronization bound"));
            }
            for source in candidates {
                let session = self.open_provider_session(&source, &mut receipt, &inputs)?;
                receipt.provider_sessions.push(session);
            }
            // This operation deliberately changed the selected server's inputs.
            // Re-query its graph and source digests before admitting any catalog.
            receipt.provider_sources = None;
            receipt = self.query_json_translation_resource(receipt, cancel.clone()).await?;
        }
        unreachable!("bounded provider synchronization")
    }
}
