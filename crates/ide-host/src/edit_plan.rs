//! Revision witnesses for reviewed semantic changes, captured off the UI thread.
use crate::Project;
use ide_core::{FileEdit, IdeError, IdeResult, WorkspaceEdit, WorkspaceEditGuard};
use std::path::{Path, PathBuf};

fn relevant(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    if name.starts_with(".env") {
        return false;
    }
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("php" | "phtml" | "php3" | "php4" | "php5" | "phps" | "phpt" | "inc")
    ) || path.parent().is_some_and(|p| p.file_name().is_some_and(|n| n == ".openidea"))
        || matches!(name, "composer.json" | "composer.lock" | ".phpantom.toml" | ".phpantom.json" | ".editorconfig")
}

fn collect(root: &Path, files: &mut Vec<PathBuf>) -> IdeResult<()> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let ty = entry.file_type()?;
        if ty.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some(".git" | "node_modules" | "target" | "artifacts" | ".history")
            ) {
                collect(&path, files)?;
            }
        } else if relevant(&path) {
            // Include symlink identity and bytes; edits themselves are checked
            // against canonical editable roots before application.
            files.push(path);
        }
    }
    Ok(())
}

impl Project {
    pub(crate) fn edit_guard(&self) -> IdeResult<WorkspaceEditGuard> {
        let settings = self.settings.effective();
        let mut hash = blake3::Hasher::new();
        hash.update(&serde_json::to_vec(&settings).map_err(|e| IdeError::internal(e.to_string()))?);
        let mut roots = vec![self.root.clone()];
        roots.extend(crate::project::content_roots(&settings, &self.root));
        roots.sort();
        roots.dedup();
        let mut files = Vec::new();
        for root in roots {
            collect(&root, &mut files)?;
        }
        files.sort();
        files.dedup();
        let mut disk_revisions = Vec::new();
        for path in files {
            let name = path.to_string_lossy();
            hash.update(&(name.len() as u64).to_le_bytes());
            hash.update(name.as_bytes());
            let mut source = blake3::Hasher::new();
            source.update_reader(std::fs::File::open(&path)?)?;
            let digest = source.finalize();
            hash.update(digest.as_bytes());
            disk_revisions.push((dunce::canonicalize(&path)?.into(), digest.to_hex().to_string()));
        }
        Ok(WorkspaceEditGuard {
            project_revision: hash.finalize().to_hex().to_string(),
            documents_revision: self.documents.edit_revision()?,
            disk_revisions,
            engine_generations: self.lsp.edit_generations(),
        })
    }

    pub(crate) fn seal_edit_guard(
        &self,
        mut guard: WorkspaceEditGuard,
        edit: &WorkspaceEdit,
    ) -> IdeResult<WorkspaceEditGuard> {
        let mut targets = Vec::new();
        for file in &edit.files {
            let FileEdit::Edit { path, .. } = file else {
                return Err(IdeError::invalid("Unsupported reviewed file operation"));
            };
            targets.push(dunce::canonicalize(path)?);
        }
        guard.disk_revisions.retain(|(path, _)| targets.iter().any(|target| **path == *target));
        guard.engine_generations = self.lsp.edit_generations();
        Ok(guard)
    }

    pub(crate) fn validate_edit_guard(&self, expected: &WorkspaceEditGuard) -> IdeResult<()> {
        let actual = self.edit_guard()?;
        if actual.project_revision != expected.project_revision
            || actual.documents_revision != expected.documents_revision
        {
            return Err(IdeError::new(
                ide_core::error::ErrorKind::OutOfSync,
                "The project or a document changed after this preview. Request a new preview.",
            ));
        }
        Ok(())
    }

    pub(crate) fn apply_reviewed_edit(&self, edit: &WorkspaceEdit) -> IdeResult<()> {
        let expected = edit.guard.as_ref().ok_or_else(|| IdeError::invalid("Missing edit guard"))?;
        self.validate_edit_guard(expected)?;
        if expected.engine_generations != self.lsp.edit_generations() {
            return Err(IdeError::invalid("The PHP server restarted after this preview. Request a new preview."));
        }
        let settings = self.settings.effective();
        let mut roots = vec![dunce::canonicalize(&self.root)?];
        for root in crate::project::content_roots(&settings, &self.root) {
            roots.push(dunce::canonicalize(root)?);
        }
        for file in &edit.files {
            let FileEdit::Edit { path, .. } = file else {
                return Err(IdeError::invalid("Unsupported reviewed file operation"));
            };
            let canonical = dunce::canonicalize(path)?;
            if !roots.iter().any(|root| canonical.starts_with(root)) {
                return Err(IdeError::invalid("An edit target is outside this project's editable roots"));
            }
        }
        self.documents.apply_guarded_edits(edit, &self.cache_dir.join("edit-recovery"))?;
        let changed: Vec<_> = edit
            .files
            .iter()
            .filter_map(|f| match f {
                FileEdit::Edit { path, .. } => Some(path.to_path_buf()),
                _ => None,
            })
            .collect();
        self.lsp.php_files_changed(&[], &changed, &[]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Host, HostConfig};
    use std::sync::Arc;
    #[tokio::test(flavor = "multi_thread")]
    async fn project_plan_rejects_disk_config_overlay_and_outside_root_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        std::fs::create_dir(&root).unwrap();
        let main = root.join("main.php");
        std::fs::write(&main, "<?php function total() {}\n").unwrap();
        std::fs::write(root.join("composer.json"), "{}").unwrap();
        let host = Host::new(HostConfig {
            project_environment: false,
            config_dir: tmp.path().join("config"),
            data_dir: tmp.path().join("data"),
            languages: Arc::new(ide_language::LanguageRegistry::load(None).unwrap()),
            syntax: Arc::default(),
            sink_for: Box::new(|_| Arc::new(|_| {})),
            owner: |w| w,
            runtime: tokio::runtime::Handle::current(),
        });
        let project = host.open_project("main", &root).unwrap();
        let guard = project.edit_guard().unwrap();
        std::fs::write(root.join("composer.json"), "{\"require\":{\"php\":\"^8.4\"}}").unwrap();
        assert!(project.validate_edit_guard(&guard).is_err());
        let guard = project.edit_guard().unwrap();
        std::fs::write(&main, "<?php function other() {}\n").unwrap();
        assert!(project.validate_edit_guard(&guard).is_err());
        let doc = project.documents.open(&main).unwrap();
        let guard = project.edit_guard().unwrap();
        project.documents.replace_all(doc.id, "<?php function unsaved() {}\n").unwrap();
        assert!(project.validate_edit_guard(&guard).is_err());
        let outside = tmp.path().join("outside.php");
        std::fs::write(&outside, "<?php").unwrap();
        let plan = WorkspaceEdit {
            files: vec![FileEdit::Edit { path: outside.clone().into(), edits: vec![] }],
            guard: Some(project.edit_guard().unwrap()),
        };
        assert!(project.apply_reviewed_edit(&plan).unwrap_err().message.contains("outside"));
        assert_eq!(std::fs::read_to_string(outside).unwrap(), "<?php");
        host.close_project("main");
    }
}
