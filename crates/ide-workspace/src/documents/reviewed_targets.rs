//! Resolve reviewed plans against their pre-move files before touching disk.
use super::*;
use ide_core::{FileEdit, WorkspaceEdit};
use std::collections::{HashMap, HashSet};

pub(super) struct Target {
    pub source: PathBuf,
    pub path: PathBuf,
    pub edits: Vec<TextEdit>,
}

pub(super) fn destination(path: &Path) -> IdeResult<PathBuf> {
    let name = path.file_name().ok_or_else(|| IdeError::invalid("A rename destination needs a file name"))?;
    let parent = dunce::canonicalize(path.parent().ok_or_else(|| IdeError::invalid("Missing rename parent"))?)?;
    Ok(parent.join(name))
}

pub(super) fn absent(path: &Path) -> IdeResult<()> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(IdeError::new(ErrorKind::AlreadyExists, "A rename destination already exists")),
        Err(e) => Err(e.into()),
    }
}

pub(super) fn targets(edit: &WorkspaceEdit) -> IdeResult<Vec<Target>> {
    let mut result: Vec<Target> = Vec::new();
    let mut renamed = HashMap::new();
    let mut sources = HashSet::new();
    let mut edited = HashSet::new();
    let mut saw_text = false;
    for file in &edit.files {
        match file {
            FileEdit::Rename { from, to } => {
                if saw_text {
                    return Err(IdeError::invalid("Reviewed renames must precede text edits"));
                }
                if std::fs::symlink_metadata(from)?.file_type().is_symlink() {
                    return Err(IdeError::invalid("Reviewed renames cannot move a symbolic link"));
                }
                let source = dunce::canonicalize(from)?;
                let path = destination(to)?;
                if source == path {
                    continue;
                }
                if source.parent() != path.parent()
                    || !source.extension().is_some_and(|e| e.eq_ignore_ascii_case("php"))
                    || !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("php"))
                    || source.file_name().is_some_and(|n| n.to_string_lossy().starts_with(".env"))
                    || path.file_name().is_some_and(|n| n.to_string_lossy().starts_with(".env"))
                    || !std::fs::metadata(&source)?.is_file()
                {
                    return Err(IdeError::invalid(
                        "Reviewed file renames currently require PHP files in the same directory",
                    ));
                }
                absent(&path)?;
                if !sources.insert(source.clone()) || renamed.insert(path.clone(), result.len()).is_some() {
                    return Err(IdeError::invalid("The edit plan contains colliding file renames"));
                }
                result.push(Target { source, path, edits: Vec::new() });
            }
            FileEdit::Edit { path, edits } => {
                saw_text = true;
                let possible_destination = destination(path)?;
                if let Some(index) = renamed.get(&possible_destination).copied() {
                    if !edited.insert(possible_destination) {
                        return Err(IdeError::invalid("The edit plan contains duplicate targets"));
                    }
                    result[index].edits = edits.clone();
                } else {
                    let path = dunce::canonicalize(path)?;
                    if sources.contains(&path) || !edited.insert(path.clone()) {
                        return Err(IdeError::invalid("The edit plan contains duplicate or obsolete rename targets"));
                    }
                    result.push(Target { source: path.clone(), path, edits: edits.clone() });
                }
            }
            _ => return Err(IdeError::invalid("Unsupported reviewed file operation")),
        }
    }
    Ok(result)
}
