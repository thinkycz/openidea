//! The pinned PHP engine shipped with the macOS Apple Silicon application.
use ide_language::LanguageServerConfig;
use std::path::{Path, PathBuf};

pub const COMMAND: &str = "phpantom_lsp";
/// The Tools row describes the checked executable shipped with this build.
pub fn version() -> &'static str {
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION
        .get_or_init(|| {
            let provenance: serde_json::Value =
                serde_json::from_str(include_str!("../../../src-tauri/resources/phpantom/provenance.json"))
                    .expect("checked bundled PHPantom provenance");
            provenance["version"].as_str().expect("bundled PHPantom version").to_owned()
        })
        .as_str()
}

pub fn bundled() -> Option<PathBuf> {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return None;
    }
    resolve(
        &std::env::current_exe().ok()?,
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources").as_path(),
    )
}

fn resolve(exe: &Path, dev_resources: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    // A packaged application must never silently borrow a development binary.
    let resources = if dir.file_name()? == "MacOS" && dir.parent()?.file_name()? == "Contents" {
        dir.parent()?.join("Resources")
    } else {
        dev_resources.to_path_buf()
    };
    let path = resources.join("phpantom").join(COMMAND);
    path.is_file().then_some(path)
}

/// Explicit server commands and Tools path overrides have already been applied.
pub fn configure(cfg: &mut LanguageServerConfig) {
    if cfg.name == "phpantom"
        && cfg.command == COMMAND
        && cfg.connect.is_none()
        && let Some(path) = bundled()
    {
        cfg.command = path.to_string_lossy().into_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundle_wins_and_missing_bundle_does_not_use_dev_engine() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("OpenStorm.app/Contents/MacOS/openidea");
        let resources = tmp.path().join("OpenStorm.app/Contents/Resources/phpantom");
        let dev = tmp.path().join("dev");
        std::fs::create_dir_all(dev.join("phpantom")).unwrap();
        std::fs::write(dev.join("phpantom/phpantom_lsp"), "dev").unwrap();
        assert_eq!(resolve(&exe, &dev), None);
        std::fs::create_dir_all(&resources).unwrap();
        std::fs::write(resources.join(COMMAND), "bundle").unwrap();
        assert_eq!(resolve(&exe, &dev), Some(resources.join(COMMAND)));
        assert_eq!(resolve(&tmp.path().join("target/debug/openidea"), &dev), Some(dev.join("phpantom/phpantom_lsp")));
    }
    #[test]
    fn explicit_paths_are_preserved() {
        let cfg = |command: &str| {
            let registry = ide_language::LanguageRegistry::load(None).unwrap();
            let mut cfg = registry.get(&ide_core::LanguageId::new("php")).unwrap().language_servers[0].clone();
            cfg.command = command.into();
            cfg
        };
        let mut custom = cfg("/custom/phpantom");
        configure(&mut custom);
        assert_eq!(custom.command, "/custom/phpantom");
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn bundled_tools_row_reports_the_actual_engine_version() {
        let tmp = tempfile::tempdir().unwrap();
        let registry = ide_language::LanguageRegistry::load(None).unwrap();
        let php = registry.get(&ide_core::LanguageId::new("php")).unwrap();
        let tools =
            crate::Tools::with_paths(tmp.path().to_path_buf(), std::ffi::OsString::new(), crate::Platform::current());
        let row = tools.list(&[php]).into_iter().find(|tool| tool.command == COMMAND).unwrap();
        let path = row.path.as_ref().expect("bundled tool path");
        let output =
            std::process::Command::new(&path.0).arg("--version").current_dir(tmp.path()).env_clear().output().unwrap();
        assert!(output.status.success());
        let actual = String::from_utf8(output.stdout).unwrap();
        let actual = actual.trim().strip_prefix("phpantom_lsp ").unwrap();
        assert_eq!(row.version.as_deref(), Some(actual), "Tools must identify its shipped executable");
    }
}
