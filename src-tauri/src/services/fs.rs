use std::fs;
use std::path::{Path, PathBuf};

use super::security::{validate_path, SecurityError};

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("security: {0}")]
    Security(#[from] SecurityError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("conflict: the file changed on disk since it was opened")]
    Conflict,
}

/// Number of daily backups kept per document.
const BACKUP_DAYS: usize = 14;

fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Convert days since the Unix epoch to a `YYYY-MM-DD` civil date (UTC).
fn civil_date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    civil_date((secs / 86_400) as i64)
}

/// Directory that holds the backups of one document. It lives outside the
/// workspace so backups never show up in Git Sync.
pub fn backup_dir_for(backup_root: &Path, root: &Path, relative_path: &str) -> PathBuf {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let workspace = format!("{:016x}", fnv1a(&canonical.to_string_lossy()));
    let name: String = relative_path
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect();
    let name = format!("{name}-{:08x}", fnv1a(relative_path) as u32);
    backup_root.join(workspace).join(name)
}

/// Keep the previous version (`latest.bak`) and the first version of each
/// day (`YYYY-MM-DD.bak`), pruning daily copies beyond `BACKUP_DAYS`.
fn backup_existing(path: &Path, dir: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dir)?;
    fs::copy(path, dir.join("latest.bak"))?;

    let daily = dir.join(format!("{}.bak", today()));
    if !daily.exists() {
        fs::copy(path, &daily)?;
    }

    let mut days: Vec<String> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.len() == 14 && n.ends_with(".bak") && n != "latest.bak")
        .collect();
    days.sort();
    while days.len() > BACKUP_DAYS {
        let oldest = days.remove(0);
        let _ = fs::remove_file(dir.join(oldest));
    }
    Ok(())
}

/// Read a file within the workspace root.
pub fn read_file(root: &Path, relative_path: &str) -> Result<String, FsError> {
    let path = validate_path(root, relative_path)?;
    let content = fs::read_to_string(&path)?;
    Ok(content)
}

/// Write a file within the workspace root using atomic temp-file replacement.
pub fn write_file(root: &Path, relative_path: &str, content: &str) -> Result<(), FsError> {
    write_file_guarded(root, relative_path, content, None, None)
}

/// Like `write_file`, but refuses to overwrite a file that no longer matches
/// `expected` (changed by another app or device) and backs up the existing
/// file under `backup_root` first. If the backup fails, nothing is written.
pub fn write_file_guarded(
    root: &Path,
    relative_path: &str,
    content: &str,
    expected: Option<&str>,
    backup_root: Option<&Path>,
) -> Result<(), FsError> {
    let path = validate_path(root, relative_path)?;

    if path.is_file() {
        if let Some(expected) = expected {
            if fs::read_to_string(&path)? != expected {
                return Err(FsError::Conflict);
            }
        }
        if let Some(backup_root) = backup_root {
            backup_existing(&path, &backup_dir_for(backup_root, root, relative_path))?;
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Atomic write: write to temp file, then rename
    let dir = path.parent().unwrap_or(root);
    let mut temp = tempfile::NamedTempFile::new_in(dir)?;
    use std::io::Write;
    temp.write_all(content.as_bytes())?;

    // Persist (rename) atomically
    temp.persist(&path).map_err(|e| FsError::Io(e.error))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn read_write_roundtrip() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create a test file
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/test.md"), "hello").unwrap();

        let content = read_file(root, "docs/test.md").unwrap();
        assert_eq!(content, "hello");

        write_file(root, "docs/test.md", "world").unwrap();
        let content = read_file(root, "docs/test.md").unwrap();
        assert_eq!(content, "world");
    }

    #[test]
    fn write_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        write_file(root, "a/b/c/test.md", "content").unwrap();
        let content = read_file(root, "a/b/c/test.md").unwrap();
        assert_eq!(content, "content");
    }

    #[test]
    fn guarded_write_detects_external_change_without_mutation() {
        let dir = tempdir().unwrap();
        let backups = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "disk").unwrap();

        let result = write_file_guarded(dir.path(), "a.md", "new", Some("opened"), Some(backups.path()));
        assert!(matches!(result, Err(FsError::Conflict)));
        assert_eq!(fs::read_to_string(dir.path().join("a.md")).unwrap(), "disk");
        assert_eq!(fs::read_dir(backups.path()).unwrap().count(), 0);
    }

    #[test]
    fn guarded_write_backs_up_outside_workspace() {
        let dir = tempdir().unwrap();
        let backups = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "old").unwrap();

        write_file_guarded(dir.path(), "a.md", "new", Some("old"), Some(backups.path())).unwrap();
        assert_eq!(fs::read_to_string(dir.path().join("a.md")).unwrap(), "new");

        let bdir = backup_dir_for(backups.path(), dir.path(), "a.md");
        assert_eq!(fs::read_to_string(bdir.join("latest.bak")).unwrap(), "old");
        assert_eq!(fs::read_to_string(bdir.join(format!("{}.bak", today()))).unwrap(), "old");
        assert!(!bdir.starts_with(dir.path()));
    }

    #[test]
    fn prunes_daily_backups() {
        let dir = tempdir().unwrap();
        let bdir = tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "x").unwrap();
        for d in 1..=20 {
            fs::write(bdir.path().join(format!("2020-01-{d:02}.bak")), "o").unwrap();
        }
        backup_existing(&dir.path().join("a.md"), bdir.path()).unwrap();
        let daily = fs::read_dir(bdir.path())
            .unwrap()
            .filter(|e| e.as_ref().unwrap().file_name() != "latest.bak")
            .count();
        assert_eq!(daily, BACKUP_DAYS);
    }

    #[test]
    fn civil_date_is_correct() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(20_397), "2025-11-05");
    }

    #[test]
    fn rejects_outside_root() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let result = read_file(root, "../outside.md");
        assert!(result.is_err());
    }
}
