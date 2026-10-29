//! Reviewed text edits: validate every target before the first mutation.
use super::*;
use ide_core::{FileEdit, WorkspaceEdit};
use std::collections::HashSet;

fn stale() -> IdeError {
    IdeError::new(ErrorKind::OutOfSync, "The edit preview is stale. Request a new preview.")
}

fn revision(states: &[(DocId, parking_lot::MutexGuard<'_, DocState>)]) -> IdeResult<String> {
    let mut hash = blake3::Hasher::new();
    for (id, st) in states {
        let record = serde_json::to_vec(&(
            id,
            &st.path,
            st.doc.version(),
            &st.encoding,
            st.has_bom,
            st.doc.line_ending(),
            st.read_only,
            st.closed,
        ))
        .map_err(|e| IdeError::internal(e.to_string()))?;
        hash.update(&(record.len() as u64).to_le_bytes());
        hash.update(&record);
    }
    Ok(hash.finalize().to_hex().to_string())
}

struct Staged {
    path: PathBuf,
    original: Vec<u8>,
    replacement: Vec<u8>,
    text: String,
    open: Option<DocId>,
}

impl Documents {
    /// Identity of all open overlays, including unsaved dependencies. Saving
    /// the same text does not invalidate a preview; editing/opening/closing does.
    pub fn edit_revision(&self) -> IdeResult<String> {
        let registry = self.registry.lock();
        let mut slots: Vec<_> = registry.docs.iter().map(|(id, entry)| (*id, entry.slot.clone())).collect();
        slots.sort_by_key(|(id, _)| id.0);
        let states: Vec<_> = slots.iter().map(|(id, slot)| (*id, slot.lock())).collect();
        revision(&states)
    }

    /// Apply a host-validated text plan. Disk originals are retained before
    /// writes; rollback never overwrites a subsequent external modification.
    pub fn apply_guarded_edits(&self, edit: &WorkspaceEdit, recovery_root: &Path) -> IdeResult<()> {
        let guard = edit.guard.as_ref().ok_or_else(|| IdeError::invalid("Missing edit revision"))?;
        let registry = self.registry.lock();
        let mut slots: Vec<_> = registry.docs.iter().map(|(id, entry)| (*id, entry.slot.clone())).collect();
        slots.sort_by_key(|(id, _)| id.0);
        let mut states: Vec<_> = slots.iter().map(|(id, slot)| (*id, slot.lock())).collect();
        if revision(&states)? != guard.documents_revision {
            return Err(stale());
        }
        let mut seen = HashSet::new();
        #[cfg(unix)]
        let mut identities = HashSet::new();
        let mut staged = Vec::new();
        let mut staged_bytes = 0u64;
        for file in &edit.files {
            let FileEdit::Edit { path, edits } = file else {
                return Err(IdeError::invalid("This reviewed plan supports text edits only"));
            };
            let path = dunce::canonicalize(path)?;
            if !seen.insert(path.clone()) {
                return Err(IdeError::invalid("The edit plan contains duplicate or aliased targets"));
            }
            if path.file_name().is_some_and(|n| n.to_string_lossy().starts_with(".env")) {
                return Err(IdeError::invalid("Environment files cannot be refactoring targets"));
            }
            let metadata = std::fs::metadata(&path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if !identities.insert((metadata.dev(), metadata.ino())) {
                    return Err(IdeError::invalid("The edit plan contains physically aliased targets"));
                }
            }
            staged_bytes = staged_bytes.saturating_add(metadata.len());
            if metadata.len() > 8 * 1024 * 1024 || staged_bytes > 64 * 1024 * 1024 {
                return Err(IdeError::unavailable("The edit plan exceeds the safe staging size limit"));
            }
            if metadata.permissions().readonly() {
                return Err(read_only_error(&path));
            }
            let original = std::fs::read(&path)?;
            if !guard
                .disk_revisions
                .iter()
                .any(|(target, expected)| **target == path && *expected == blake3::hash(&original).to_hex().as_str())
            {
                return Err(stale());
            }
            let open = Self::lookup(&registry, &path);
            let (mut proposal, encoding, bom) = if let Some(id) = open {
                let st = &states.iter().find(|(key, _)| *key == id).ok_or_else(stale)?.1;
                if st.read_only || st.binary || st.uri.is_some() || st.closed {
                    return Err(read_only_error(&path));
                }
                if blake3::hash(&original).to_hex().as_str() != st.disk_revision {
                    return Err(IdeError::new(
                        ErrorKind::AlreadyExists,
                        "A refactoring target changed on disk. Resolve the save conflict before retrying.",
                    ));
                }
                let mut proposal = Document::new(&st.doc.text());
                proposal.set_line_ending(st.doc.line_ending());
                (proposal, st.encoding.clone(), st.has_bom)
            } else {
                let loaded = ide_vfs::load_text(&path)?;
                if loaded.binary
                    || loaded.too_large
                    || loaded.disk_revision != blake3::hash(&original).to_hex().as_str()
                {
                    return Err(IdeError::invalid("The edit target changed or is not writable text"));
                }
                (Document::new(&loaded.text), loaded.encoding, loaded.has_bom)
            };
            // Document's generic LSP mapping clamps positions; reviewed plans
            // must instead reject invalid lines, columns and split surrogates.
            let source = proposal.text();
            let lines: Vec<_> = source.split('\n').collect();
            let mut ranges = Vec::new();
            for e in edits {
                for pos in [e.range.start, e.range.end] {
                    let line =
                        lines.get(pos.line as usize).ok_or_else(|| IdeError::invalid("Edit line is out of range"))?;
                    let mut units = 0;
                    let mut valid = pos.col == 0;
                    for c in line.chars() {
                        units += c.len_utf16() as u32;
                        valid |= units == pos.col;
                    }
                    if !valid {
                        return Err(IdeError::invalid("Edit column is out of range or splits a Unicode character"));
                    }
                }
                ranges.push(e.range);
            }
            ranges.sort_by_key(|r| (r.start, r.end));
            if ranges.windows(2).any(|r| r[1].start < r[0].end || r[1].start == r[0].start) {
                return Err(IdeError::invalid("The edit plan contains overlapping or colliding edits"));
            }
            proposal.apply_edits_detailed(edits)?;
            staged.push(Staged {
                path,
                original,
                replacement: ide_vfs::encode(&proposal.text_for_save(), &encoding, bom)?,
                text: proposal.text(),
                open,
            });
        }
        if staged.is_empty() {
            return Ok(());
        }
        let receipt = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        );
        let recovery = recovery_root.join(receipt);
        std::fs::create_dir_all(&recovery)?;
        let manifest: Vec<_> = staged.iter().enumerate().map(|(i, file)| (i, &file.path)).collect();
        std::fs::write(
            recovery.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).map_err(|e| IdeError::internal(e.to_string()))?,
        )?;
        for (i, file) in staged.iter().enumerate() {
            std::fs::write(recovery.join(format!("{i}.original")), &file.original)?;
        }
        for (i, file) in staged.iter().enumerate() {
            let write = (|| -> IdeResult<()> {
                if std::fs::read(&file.path)? != file.original {
                    return Err(stale());
                }
                ide_vfs::write_atomic_strict(&file.path, &file.replacement)?;
                Ok(())
            })();
            if let Err(error) = write {
                for previous in staged[..i].iter().rev() {
                    if std::fs::read(&previous.path).is_ok_and(|b| b == previous.replacement) {
                        let _ = ide_vfs::write_atomic_strict(&previous.path, &previous.original);
                    }
                }
                return Err(IdeError::new(
                    error.kind,
                    format!("{}. Original files and recovery manifest: {}", error.message, recovery.display()),
                ));
            }
        }
        for file in &staged {
            if let Some(id) = file.open {
                let st = &mut states.iter_mut().find(|(key, _)| *key == id).ok_or_else(stale)?.1;
                self.replace_locked(id, st, &file.text)?;
                st.disk_revision = blake3::hash(&file.replacement).to_hex().to_string();
                st.disk_mtime = ide_vfs::mtime_ms(&std::fs::metadata(&file.path)?);
                st.disk_len = file.replacement.len() as u64;
                self.set_dirty(id, st, false);
                if st.observed {
                    for observer in self.observers() {
                        observer.saved(id, &st.path, &file.text);
                    }
                }
            }
        }
        if let Err(error) = std::fs::remove_dir_all(&recovery) {
            tracing::warn!("Edit succeeded; recovery copies retained at {}: {error}", recovery.display());
        }
        Ok(())
    }
}
