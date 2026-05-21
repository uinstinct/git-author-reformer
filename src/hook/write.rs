//! Atomic file writer for the commit-msg hook.
//!
//! `atomic_write_executable` follows RESEARCH §Pattern 1: write to a temp file
//! in the same directory, set permissions, then rename atomically.
//!
//! ORDER IS LOAD-BEARING: set permissions BEFORE rename — Pitfall §3.

use std::fs;
use std::io::Write;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Write `contents` to `target` atomically with mode 0755 on Unix.
///
/// Steps (RESEARCH §Pattern 1):
/// 1. Compute `tmp = target.with_extension("tmp.<pid>")`.
/// 2. Create tmp, write bytes, `sync_all`.
/// 3. On Unix: set mode 0755 on tmp (BEFORE rename — Pitfall §3).
/// 4. Rename tmp → target atomically (same filesystem).
pub(crate) fn atomic_write_executable(target: &Path, contents: &str) -> std::io::Result<()> {
    let tmp = target.with_extension(format!("tmp.{}", std::process::id()));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
    }
    let mut perms = fs::metadata(&tmp)?.permissions();
    #[cfg(unix)]
    perms.set_mode(0o755);
    fs::set_permissions(&tmp, perms)?;
    fs::rename(&tmp, target)?;
    Ok(())
}

/// Delete the hook file. Called by `remove_strip` when the resulting list is empty (HOOK-10).
pub(crate) fn delete_hook(path: &Path) -> std::io::Result<()> {
    fs::remove_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn atomic_write_executable_creates_file_with_contents() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("commit-msg");
        atomic_write_executable(&target, "hello").unwrap();
        let read_back = fs::read_to_string(&target).unwrap();
        assert_eq!(read_back, "hello");
    }

    #[test]
    fn atomic_write_executable_overwrites_existing_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("commit-msg");
        fs::write(&target, "old content").unwrap();
        atomic_write_executable(&target, "new content").unwrap();
        let read_back = fs::read_to_string(&target).unwrap();
        assert_eq!(read_back, "new content");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_executable_sets_mode_0755_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("commit-msg");
        atomic_write_executable(&target, "#!/bin/sh\n").unwrap();
        let mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o755, "expected mode 0755, got {mode:o}");
    }

    #[test]
    fn atomic_write_executable_cleans_up_tmp_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("commit-msg");
        atomic_write_executable(&target, "data").unwrap();
        // The .tmp.<pid> sibling must not exist after success.
        let tmp = target.with_extension(format!("tmp.{}", std::process::id()));
        assert!(!tmp.exists(), "tmp file must be cleaned up after rename");
    }

    #[test]
    fn atomic_write_executable_emits_lf_line_endings() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("commit-msg");
        atomic_write_executable(&target, "line1\nline2\n").unwrap();
        let bytes = fs::read(&target).unwrap();
        let content = String::from_utf8(bytes).unwrap();
        assert!(
            !content.contains("\r\n"),
            "output must not contain CRLF (Pitfall §4)"
        );
    }

    #[test]
    fn delete_hook_removes_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("commit-msg");
        fs::write(&path, "content").unwrap();
        delete_hook(&path).unwrap();
        assert!(!path.exists(), "file must be removed after delete_hook");
    }

    #[test]
    fn delete_hook_returns_err_on_missing_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("nonexistent");
        let result = delete_hook(&path);
        assert!(
            result.is_err(),
            "delete_hook on missing file must return Err"
        );
    }
}
