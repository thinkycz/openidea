//! Reviewed text edits and PHP file renames: validate every target before the first mutation.
use super::*;
use ide_core::WorkspaceEdit;
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
    before_path: PathBuf,
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
    source: PathBuf,
    path: PathBuf,
    original: Vec<u8>,
    replacement: Vec<u8>,
    text: String,
    open: Option<DocId>,
    dirty: bool,
}

/// Files changed by one reviewed apply/undo/redo operation.
#[derive(Debug, Default)]
pub struct ReviewedChanges {
    pub created: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
}
impl ReviewedChanges {
    fn staged(files: &[Staged]) -> Self {
        let mut changed = Self::default();
        for file in files {
            if file.source == file.path {
                changed.modified.push(file.path.clone());
            } else {
                changed.created.push(file.path.clone());
                changed.removed.push(file.source.clone());
            }
        }
        changed
    }
}

impl Documents {
    fn validate_move_mode(&self, source: &Path, path: &Path, text: &str) -> IdeResult<()> {
        if source == path {
            return Ok(());
        }
        let language = (self.detect_language)(source, text.lines().next());
        let target_language = (self.detect_language)(path, text.lines().next());
        let defaults = self.file_defaults();
        let editorconfig = |p: &Path| defaults.editorconfig.then(|| self.editorconfig.resolve(p)).flatten();
        if language != target_language
            || (self.detect_ui_mode)(source, &language) != (self.detect_ui_mode)(path, &target_language)
            || editorconfig(source) != editorconfig(path)
            || defaults.native_to_ascii_for(source) != defaults.native_to_ascii_for(path)
        {
            return Err(IdeError::invalid(
                "Reviewed renames cannot change file type or filename-specific editor settings",
            ));
        }
        Ok(())
    }

    /// Pre-move sources and final targets used to seal and validate a reviewed plan.
    pub fn reviewed_edit_targets(edit: &WorkspaceEdit) -> IdeResult<Vec<(PathBuf, PathBuf)>> {
        Ok(reviewed_targets::targets(edit)?.into_iter().map(|t| (t.source, t.path)).collect())
    }

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
    pub fn apply_guarded_edits(&self, edit: &WorkspaceEdit, recovery_root: &Path) -> IdeResult<ReviewedChanges> {
        let _operation = self.operations.lock();
        let guard = edit.guard.as_ref().ok_or_else(|| IdeError::invalid("Missing edit revision"))?;
        // Reuse upstream's preview fingerprints, under the common operation lock,
        // before locking registry/buffers: fingerprinting reads those buffers too.
        self.validate_preconditions(&edit.preconditions)?;
        let mut history = self.reviewed_history.lock();
        let mut registry = self.registry.lock();
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
        for target in reviewed_targets::targets(edit)? {
            let source = target.source;
            let path = target.path;
            let edits = &target.edits;
            if source != path && Self::lookup(&registry, &path).is_some() {
                return Err(IdeError::invalid("A rename destination is already open in the editor"));
            }
            if !seen.insert(path.clone()) {
                return Err(IdeError::invalid("The edit plan contains duplicate or aliased targets"));
            }
            if path.file_name().is_some_and(|n| n.to_string_lossy().starts_with(".env")) {
                return Err(IdeError::invalid("Environment files cannot be refactoring targets"));
            }
            let metadata = std::fs::metadata(&source)?;
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
                return Err(read_only_error(&source));
            }
            let original = std::fs::read(&source)?;
            if !guard
                .disk_revisions
                .iter()
                .any(|(target, expected)| **target == source && *expected == blake3::hash(&original).to_hex().as_str())
            {
                return Err(stale());
            }
            let open = Self::lookup(&registry, &source);
            if let Some(expected) = target.version {
                match open.and_then(|id| states.iter().find(|(key, _)| *key == id)) {
                    Some((_, state)) if i64::from(state.doc.version()) == i64::from(expected) => {}
                    // A closed source can be versioned by the engine; the sealed
                    // disk witness above must already match its original bytes.
                    None => {}
                    _ => return Err(stale()),
                }
            }
            let (mut proposal, encoding, bom) = if let Some(id) = open {
                let st = &states.iter().find(|(key, _)| *key == id).ok_or_else(stale)?.1;
                if st.read_only || st.binary || st.uri.is_some() || st.closed {
                    return Err(read_only_error(&source));
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
                let loaded = ide_vfs::load_text(&source)?;
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
            let source_text = proposal.text();
            self.validate_move_mode(&source, &path, &source_text)?;
            let before_dirty = open.is_some_and(|id| states.iter().any(|(key, st)| *key == id && st.dirty));
            let before = Side { disk: original.clone(), text: source_text.clone(), dirty: before_dirty };
            let lines: Vec<_> = source_text.split('\n').collect();
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
                before_path: source.clone(),
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
                source,
                path,
                original,
                replacement: ide_vfs::encode(&proposal.text_for_save(), &encoding, bom)?,
                text: proposal.text(),
                open,
                dirty: false,
            });
        }
        if staged.is_empty() {
            return Ok(ReviewedChanges::default());
        }
        let group = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        );
        self.commit_reviewed(
            &staged,
            &mut registry,
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
        let changes = ReviewedChanges::staged(&staged);
        if !changes.created.is_empty() || !changes.removed.is_empty() {
            self.sink.emit(IdeEvent::FsChanged {
                ignore_rules: Vec::new(),
                created: changes.created.iter().cloned().map(Into::into).collect(),
                modified: changes.modified.iter().cloned().map(Into::into).collect(),
                removed: changes.removed.iter().cloned().map(Into::into).collect(),
            });
        }
        Ok(changes)
    }

    pub fn reviewed_edit_paths(&self, group: &str, redo: bool) -> IdeResult<Vec<PathBuf>> {
        let history = self.reviewed_history.lock();
        let index = if redo { Some(history.cursor) } else { history.cursor.checked_sub(1) };
        let entry = index
            .and_then(|i| history.entries.get(i))
            .filter(|e| e.group == group)
            .ok_or_else(|| IdeError::invalid("This reviewed edit is no longer next in project history"))?;
        Ok(entry.files.iter().flat_map(|f| [f.before_path.clone(), f.path.clone()]).collect())
    }

    /// Undo/redo exactly one reviewed transaction. Validate every disk and open
    /// overlay before writing. Never merge history into subsequent user edits.
    pub fn restore_reviewed_edit(&self, group: &str, redo: bool, recovery_root: &Path) -> IdeResult<ReviewedChanges> {
        let _operation = self.operations.lock();
        let mut history = self.reviewed_history.lock();
        let index = if redo { Some(history.cursor) } else { history.cursor.checked_sub(1) };
        let entry = index
            .and_then(|i| history.entries.get(i))
            .filter(|e| e.group == group)
            .ok_or_else(|| IdeError::invalid("This reviewed edit is no longer next in project history"))?;
        let mut registry = self.registry.lock();
        let mut slots: Vec<_> = registry.docs.iter().map(|(id, entry)| (*id, entry.slot.clone())).collect();
        slots.sort_by_key(|(id, _)| id.0);
        let mut states: Vec<_> = slots.iter().map(|(id, slot)| (*id, slot.lock())).collect();
        let mut staged = Vec::new();
        #[cfg(unix)]
        let mut identities = HashSet::new();
        for file in &entry.files {
            let (expected, target) = if redo { (&file.before, &file.after) } else { (&file.after, &file.before) };
            let (source, path) = if redo { (&file.before_path, &file.path) } else { (&file.path, &file.before_path) };
            self.validate_move_mode(source, path, &expected.text)?;
            if source != path {
                if reviewed_targets::destination(path)? != *path {
                    return Err(stale());
                }
                reviewed_targets::absent(path)?;
                if Self::lookup(&registry, path).is_some() {
                    return Err(stale());
                }
            }
            if dunce::canonicalize(source)? != *source || std::fs::read(source)? != expected.disk {
                return Err(IdeError::new(
                    ErrorKind::OutOfSync,
                    "A refactoring target changed on disk. Undo/redo was cancelled without changes.",
                ));
            }
            let metadata = std::fs::metadata(source)?;
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
                return Err(read_only_error(source));
            }
            let open = Self::lookup(&registry, source);
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
                source: source.clone(),
                path: path.clone(),
                original: expected.disk.clone(),
                replacement: target.disk.clone(),
                text: target.text.clone(),
                open,
                dirty: target.dirty,
            });
        }
        self.commit_reviewed(
            &staged,
            &mut registry,
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
        let changes = ReviewedChanges::staged(&staged);
        if !changes.created.is_empty() || !changes.removed.is_empty() {
            self.sink.emit(IdeEvent::FsChanged {
                ignore_rules: Vec::new(),
                created: changes.created.iter().cloned().map(Into::into).collect(),
                modified: changes.modified.iter().cloned().map(Into::into).collect(),
                removed: changes.removed.iter().cloned().map(Into::into).collect(),
            });
        }
        Ok(changes)
    }

    fn commit_reviewed(
        &self,
        staged: &[Staged],
        registry: &mut Registry,
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
        let manifest: Vec<_> = staged.iter().enumerate().map(|(i, file)| (i, &file.source, &file.path)).collect();
        std::fs::write(
            recovery.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).map_err(|e| IdeError::internal(e.to_string()))?,
        )?;
        for (i, file) in staged.iter().enumerate() {
            std::fs::write(recovery.join(format!("{i}.original")), &file.original)?;
        }
        for (i, file) in staged.iter().enumerate() {
            let mut touched = false;
            let write = (|| -> IdeResult<()> {
                if std::fs::symlink_metadata(&file.source)?.file_type().is_symlink()
                    || dunce::canonicalize(&file.source)? != file.source
                    || std::fs::read(&file.source)? != file.original
                {
                    return Err(stale());
                }
                if file.source != file.path {
                    ide_vfs::rename_exclusive(&file.source, &file.path)?;
                    touched = true;
                    if std::fs::read(&file.path)? != file.original {
                        return Err(stale());
                    }
                }
                if file.replacement != file.original {
                    touched = true;
                    ide_vfs::write_atomic_strict(&file.path, &file.replacement)?;
                }
                Ok(())
            })();
            if let Err(error) = write {
                for (index, previous) in staged[..=i].iter().enumerate().rev() {
                    if index == i && !touched {
                        continue;
                    }
                    if let Ok(bytes) = std::fs::read(&previous.path) {
                        if previous.replacement != previous.original && bytes == previous.replacement {
                            let _ = ide_vfs::write_atomic_strict(&previous.path, &previous.original);
                        }
                        if previous.source != previous.path
                            && std::fs::read(&previous.path).is_ok_and(|b| b == previous.original)
                        {
                            let _ = ide_vfs::rename_exclusive(&previous.path, &previous.source);
                        }
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
                if file.source != file.path {
                    let from = st.path.clone();
                    if st.observed {
                        for observer in self.observers() {
                            observer.closed(id, &from);
                        }
                    }
                    let old_keys = std::mem::take(&mut registry.docs.get_mut(&id).ok_or_else(stale)?.keys);
                    for key in old_keys {
                        registry.by_key.remove(&key);
                    }
                    registry.by_key.insert(file.path.clone(), id);
                    registry.docs.get_mut(&id).ok_or_else(stale)?.keys = vec![file.path.clone()];
                    st.path = file.path.clone();
                    if st.observed {
                        let info = Self::info(id, st);
                        for observer in self.observers() {
                            observer.opened(&info);
                        }
                    }
                    self.sink.emit(IdeEvent::DocMoved { doc: id, from: from.into(), to: file.path.clone().into() });
                }
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
                    workspace_edit: true,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn guarded_pair(docs: &Documents, root: &Path) -> (DocId, WorkspaceEdit) {
        use ide_core::{FileEdit, Pos, Range, WorkspaceEditGuard};
        let paths = [root.join("one.php"), root.join("two.php")];
        for path in &paths {
            std::fs::write(path, "<?php old();\n").unwrap();
        }
        let doc = docs.open(&paths[0]).unwrap();
        let edit = WorkspaceEdit {
            files: paths
                .iter()
                .map(|path| FileEdit::Edit {
                    path: path.clone().into(),
                    version: (path == &paths[0]).then_some(doc.version as i32),
                    edits: vec![TextEdit { range: Range::new(Pos::new(0, 6), Pos::new(0, 9)), new_text: "new".into() }],
                })
                .collect(),
            guard: Some(WorkspaceEditGuard {
                documents_revision: docs.edit_revision().unwrap(),
                disk_revisions: paths
                    .iter()
                    .map(|path| {
                        (
                            dunce::canonicalize(path).unwrap().into(),
                            blake3::hash(&std::fs::read(path).unwrap()).to_hex().to_string(),
                        )
                    })
                    .collect(),
                ..Default::default()
            }),
            ..Default::default()
        };
        (doc.id, edit)
    }

    #[test]
    fn upstream_constraints_are_checked_before_any_reviewed_write() {
        use ide_core::FileEdit;
        for invalid in ["version", "negative version", "fingerprint"] {
            let tmp = tempfile::tempdir().unwrap();
            let docs = Documents::new(Arc::new(|_| {}), Box::new(|_, _| LanguageId::new("php")));
            let (doc, mut edit) = guarded_pair(&docs, tmp.path());
            docs.prepare_workspace_edit(&mut edit).unwrap();
            if invalid == "fingerprint" {
                edit.preconditions[1].hash = "outdated".into();
            } else {
                let FileEdit::Edit { version, .. } = &mut edit.files[0] else { panic!() };
                *version = Some(if invalid == "version" { i32::MAX } else { -1 });
            }
            let before = docs.snapshot(doc).unwrap();
            let error = docs.apply_guarded_edits(&edit, &tmp.path().join("recovery")).unwrap_err();
            assert_eq!(error.kind, ErrorKind::OutOfSync, "{invalid}");
            for name in ["one.php", "two.php"] {
                assert_eq!(std::fs::read_to_string(tmp.path().join(name)).unwrap(), "<?php old();\n");
            }
            assert_eq!(docs.snapshot(doc).unwrap().text, before.text);
            assert_eq!(docs.snapshot(doc).unwrap().version, before.version);
            assert!(docs.reviewed_history.lock().entries.is_empty());
            assert!(!tmp.path().join("recovery").exists());
        }
    }

    #[test]
    fn valid_upstream_constraints_keep_reviewed_apply_undo_and_redo() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = Documents::new(Arc::new(|_| {}), Box::new(|_, _| LanguageId::new("php")));
        let (doc, mut edit) = guarded_pair(&docs, tmp.path());
        docs.prepare_workspace_edit(&mut edit).unwrap();
        let recovery = tmp.path().join("recovery");
        docs.apply_guarded_edits(&edit, &recovery).unwrap();
        let group = docs.reviewed_history.lock().entries[0].group.clone();
        assert_eq!(docs.snapshot(doc).unwrap().text, "<?php new();\n");
        docs.restore_reviewed_edit(&group, false, &recovery).unwrap();
        assert_eq!(docs.snapshot(doc).unwrap().text, "<?php old();\n");
        docs.restore_reviewed_edit(&group, true, &recovery).unwrap();
        for name in ["one.php", "two.php"] {
            assert_eq!(std::fs::read_to_string(tmp.path().join(name)).unwrap(), "<?php new();\n");
        }
    }

    #[test]
    fn generic_workspace_paths_cannot_strip_a_reviewed_guard() {
        let tmp = tempfile::tempdir().unwrap();
        let docs = Documents::new(Arc::new(|_| {}), Box::new(|_, _| LanguageId::new("php")));
        let (_, edit) = guarded_pair(&docs, tmp.path());
        for agent in [false, true] {
            let error = if agent { docs.apply_agent_workspace_edit(&edit) } else { docs.apply_workspace_edit(&edit) }
                .unwrap_err();
            assert!(error.message.contains("guarded project"));
            for name in ["one.php", "two.php"] {
                assert_eq!(std::fs::read_to_string(tmp.path().join(name)).unwrap(), "<?php old();\n");
            }
        }
    }

    #[test]
    fn reviewed_move_commit_rolls_back_prior_writes_without_touching_a_late_destination() {
        for destination_bytes in [b"external".as_slice(), b"New".as_slice()] {
            let tmp = tempfile::tempdir().unwrap();
            let first = tmp.path().join("first.php");
            let first_destination = tmp.path().join("first-moved.php");
            let source = tmp.path().join("source.php");
            let destination = tmp.path().join("destination.php");
            std::fs::write(&first, b"Old").unwrap();
            std::fs::write(&source, b"Old").unwrap();
            // The destination appeared after staging, including a coincidental match
            // with this plan's replacement. Rollback must never claim that file.
            std::fs::write(&destination, destination_bytes).unwrap();
            let staged = vec![
                Staged {
                    source: first.clone(),
                    path: first_destination.clone(),
                    original: b"Old".to_vec(),
                    replacement: b"New".to_vec(),
                    text: "New".into(),
                    open: None,
                    dirty: false,
                },
                Staged {
                    source: source.clone(),
                    path: destination.clone(),
                    original: b"Old".to_vec(),
                    replacement: b"New".to_vec(),
                    text: "New".into(),
                    open: None,
                    dirty: false,
                },
            ];
            let docs = Documents::new(Arc::new(|_| {}), Box::new(|_, _| LanguageId::plain_text()));
            let recovery = tmp.path().join("recovery");
            let error = docs
                .commit_reviewed(
                    &staged,
                    &mut docs.registry.lock(),
                    &mut [],
                    &recovery,
                    ide_core::event::ReviewedEditHistory {
                        group: "late".into(),
                        action: ide_core::event::ReviewedEditAction::Apply,
                    },
                )
                .unwrap_err();
            assert!(error.message.contains("recovery manifest"));
            assert_eq!(std::fs::read(&first).unwrap(), b"Old");
            assert!(!first_destination.exists());
            assert_eq!(std::fs::read(&source).unwrap(), b"Old");
            assert_eq!(std::fs::read(&destination).unwrap(), destination_bytes);
            let receipt = std::fs::read_dir(&recovery).unwrap().next().unwrap().unwrap().path();
            assert_eq!(std::fs::read(receipt.join("0.original")).unwrap(), b"Old");
            assert!(receipt.join("manifest.json").exists());
        }
    }
}
