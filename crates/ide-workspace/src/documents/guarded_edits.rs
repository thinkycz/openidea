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

#[derive(Clone)]
struct Side {
    disk: Vec<u8>,
    text: String,
    dirty: bool,
}
#[derive(Clone)]
struct HistoryFile {
    path: PathBuf,
    before: Side,
    after: Side,
    encoding: String,
    bom: bool,
    line_ending: LineEnding,
}
struct Entry {
    group: String,
    files: Vec<HistoryFile>,
}
#[derive(Default)]
pub(super) struct ReviewedHistory {
    entries: Vec<Entry>,
    cursor: usize,
}

struct Staged {
    path: PathBuf,
    original: Vec<u8>,
    replacement: Vec<u8>,
    text: String,
    open: Option<DocId>,
    dirty: bool,
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
        let mut history = self.reviewed_history.lock();
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
        let mut history_files = Vec::new();
        let mut history_bytes = 0usize;
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
            let before_dirty = open.is_some_and(|id| states.iter().any(|(key, st)| *key == id && st.dirty));
            let before = Side { disk: original.clone(), text: source.clone(), dirty: before_dirty };
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
            history_files.push(HistoryFile {
                path: path.clone(),
                before,
                after: Side {
                    disk: ide_vfs::encode(&proposal.text_for_save(), &encoding, bom)?,
                    text: proposal.text(),
                    dirty: false,
                },
                encoding: encoding.clone(),
                bom,
                line_ending: proposal.line_ending(),
            });
            let retained = history_files.last().unwrap();
            history_bytes = history_bytes
                .saturating_add(retained.before.disk.len())
                .saturating_add(retained.after.disk.len())
                .saturating_add(retained.before.text.len())
                .saturating_add(retained.after.text.len());
            if history_bytes > 64 * 1024 * 1024 {
                return Err(IdeError::unavailable("The edit plan exceeds the safe history size limit"));
            }
            staged.push(Staged {
                path,
                original,
                replacement: ide_vfs::encode(&proposal.text_for_save(), &encoding, bom)?,
                text: proposal.text(),
                open,
                dirty: false,
            });
        }
        if staged.is_empty() {
            return Ok(());
        }
        let group = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        );
        self.commit_reviewed(
            &staged,
            &mut states,
            recovery_root,
            ide_core::event::ReviewedEditHistory {
                group: group.clone(),
                action: ide_core::event::ReviewedEditAction::Apply,
            },
        )?;
        let cursor = history.cursor;
        history.entries.truncate(cursor);
        history.entries.push(Entry { group, files: history_files });
        // Bound resident originals; Local History remains available independently.
        while history.entries.len() > 16
            || (history.entries.len() > 1
                && history
                    .entries
                    .iter()
                    .flat_map(|e| &e.files)
                    .map(|f| f.before.disk.len() + f.after.disk.len() + f.before.text.len() + f.after.text.len())
                    .sum::<usize>()
                    > 64 * 1024 * 1024)
        {
            history.entries.remove(0);
        }
        history.cursor = history.entries.len();
        Ok(())
    }

    pub fn reviewed_edit_paths(&self, group: &str, redo: bool) -> IdeResult<Vec<PathBuf>> {
        let history = self.reviewed_history.lock();
        let index = if redo { Some(history.cursor) } else { history.cursor.checked_sub(1) };
        let entry = index
            .and_then(|i| history.entries.get(i))
            .filter(|e| e.group == group)
            .ok_or_else(|| IdeError::invalid("This reviewed edit is no longer next in project history"))?;
        Ok(entry.files.iter().map(|f| f.path.clone()).collect())
    }

    /// Undo/redo exactly one reviewed transaction. Validate every disk and open
    /// overlay before writing. Never merge history into subsequent user edits.
    pub fn restore_reviewed_edit(&self, group: &str, redo: bool, recovery_root: &Path) -> IdeResult<Vec<PathBuf>> {
        let mut history = self.reviewed_history.lock();
        let index = if redo { Some(history.cursor) } else { history.cursor.checked_sub(1) };
        let entry = index
            .and_then(|i| history.entries.get(i))
            .filter(|e| e.group == group)
            .ok_or_else(|| IdeError::invalid("This reviewed edit is no longer next in project history"))?;
        let registry = self.registry.lock();
        let mut slots: Vec<_> = registry.docs.iter().map(|(id, entry)| (*id, entry.slot.clone())).collect();
        slots.sort_by_key(|(id, _)| id.0);
        let mut states: Vec<_> = slots.iter().map(|(id, slot)| (*id, slot.lock())).collect();
        let mut staged = Vec::new();
        #[cfg(unix)]
        let mut identities = HashSet::new();
        for file in &entry.files {
            let (expected, target) = if redo { (&file.before, &file.after) } else { (&file.after, &file.before) };
            if dunce::canonicalize(&file.path)? != file.path || std::fs::read(&file.path)? != expected.disk {
                return Err(IdeError::new(
                    ErrorKind::OutOfSync,
                    "A refactoring target changed on disk. Undo/redo was cancelled without changes.",
                ));
            }
            let metadata = std::fs::metadata(&file.path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if !identities.insert((metadata.dev(), metadata.ino())) {
                    return Err(IdeError::invalid(
                        "Refactoring targets are now physically aliased. Undo/redo was cancelled.",
                    ));
                }
            }
            if metadata.permissions().readonly() {
                return Err(read_only_error(&file.path));
            }
            let open = Self::lookup(&registry, &file.path);
            if let Some(id) = open {
                let st = &states.iter().find(|(key, _)| *key == id).ok_or_else(stale)?.1;
                if st.closed
                    || st.read_only
                    || st.binary
                    || st.uri.is_some()
                    || st.doc.text() != expected.text
                    || st.encoding != file.encoding
                    || st.has_bom != file.bom
                    || st.doc.line_ending() != file.line_ending
                    || st.disk_revision != blake3::hash(&expected.disk).to_hex().as_str()
                {
                    return Err(IdeError::new(
                        ErrorKind::OutOfSync,
                        "A refactoring target changed in the editor. Undo/redo was cancelled without changes.",
                    ));
                }
            } else if expected.dirty || target.dirty {
                return Err(IdeError::invalid(format!(
                    "Reopen {} before undo/redo to restore its unsaved buffer",
                    file.path.display()
                )));
            }
            staged.push(Staged {
                path: file.path.clone(),
                original: expected.disk.clone(),
                replacement: target.disk.clone(),
                text: target.text.clone(),
                open,
                dirty: target.dirty,
            });
        }
        self.commit_reviewed(
            &staged,
            &mut states,
            recovery_root,
            ide_core::event::ReviewedEditHistory {
                group: group.into(),
                action: if redo {
                    ide_core::event::ReviewedEditAction::Redo
                } else {
                    ide_core::event::ReviewedEditAction::Undo
                },
            },
        )?;
        if redo {
            history.cursor += 1;
        } else {
            history.cursor -= 1;
        }
        Ok(staged.into_iter().map(|f| f.path).collect())
    }

    fn commit_reviewed(
        &self,
        staged: &[Staged],
        states: &mut [(DocId, parking_lot::MutexGuard<'_, DocState>)],
        recovery_root: &Path,
        history: ide_core::event::ReviewedEditHistory,
    ) -> IdeResult<()> {
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
        for file in staged {
            if let Some(id) = file.open {
                let st = &mut states.iter_mut().find(|(key, _)| *key == id).ok_or_else(stale)?.1;
                let set = st.doc.diff(&file.text);
                let base_version = st.doc.version();
                let (applied, edits) = st.doc.apply_detailed(base_version, &set)?;
                self.notify_changed(id, st, &applied, &edits);
                self.sink.emit(IdeEvent::DocPatched {
                    doc: id,
                    base_version,
                    version: st.doc.version(),
                    changes: set.changes,
                    history: Some(history.clone()),
                });
                st.disk_revision = blake3::hash(&file.replacement).to_hex().to_string();
                st.disk_mtime = ide_vfs::mtime_ms(&std::fs::metadata(&file.path)?);
                st.disk_len = file.replacement.len() as u64;
                self.set_dirty(id, st, file.dirty);
                if st.observed && !file.dirty {
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
