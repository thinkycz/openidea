//! Atomic no-clobber moves for reviewed transactions.
use ide_core::{IdeError, IdeResult};
use std::path::Path;

/// Move a path atomically, refusing a destination which exists at syscall time.
/// Unsupported platforms fail before mutating either path.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn rename_exclusive(from: &Path, to: &Path) -> IdeResult<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let from = CString::new(from.as_os_str().as_bytes()).map_err(|_| IdeError::invalid("Invalid source path"))?;
    let to = CString::new(to.as_os_str().as_bytes()).map_err(|_| IdeError::invalid("Invalid destination path"))?;
    // SAFETY: both C strings remain alive for the syscall and are NUL-terminated.
    #[cfg(target_os = "macos")]
    let result =
        unsafe { libc::renameatx_np(libc::AT_FDCWD, from.as_ptr(), libc::AT_FDCWD, to.as_ptr(), libc::RENAME_EXCL) };
    // SAFETY: same valid C strings; RENAME_NOREPLACE prevents destination replacement.
    #[cfg(target_os = "linux")]
    let result =
        unsafe { libc::renameat2(libc::AT_FDCWD, from.as_ptr(), libc::AT_FDCWD, to.as_ptr(), libc::RENAME_NOREPLACE) };
    if result == 0 { Ok(()) } else { Err(std::io::Error::last_os_error().into()) }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn rename_exclusive(_from: &Path, _to: &Path) -> IdeResult<()> {
    Err(IdeError::unavailable("Guarded file moves are unavailable on this platform"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn exclusive_rename_never_replaces_an_existing_destination() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("from.php");
        let to = tmp.path().join("to.php");
        std::fs::write(&from, b"source").unwrap();
        std::fs::write(&to, b"external").unwrap();
        assert!(rename_exclusive(&from, &to).is_err());
        assert_eq!(std::fs::read(&from).unwrap(), b"source");
        assert_eq!(std::fs::read(&to).unwrap(), b"external");
        std::fs::remove_file(&to).unwrap();
        rename_exclusive(&from, &to).unwrap();
        assert!(!from.exists());
        assert_eq!(std::fs::read(&to).unwrap(), b"source");
    }
}
