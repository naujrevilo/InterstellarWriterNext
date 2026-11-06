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
    #[error("backup not found")]
    BackupNotFound,
    #[error("only files can be moved to the trash")]
    NotFile,
    #[error("could not move file to the system trash: {0}")]
    Trash(String),
    #[error("invalid document filename: {0}")]
    InvalidDocumentName(String),
    #[error("the destination directory does not exist")]
    MissingDirectory,
    #[error("a document with this filename already exists")]
    AlreadyExists,
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

/// Create a new document without ever replacing an existing file.
pub fn create_file(
    root: &Path,
    directory: &str,
    filename: &str,
    content: &str,
) -> Result<String, FsError> {
    let stem = filename.rsplit_once('.').map(|(stem, _)| stem).unwrap_or("");
    let reserved = stem.split('.').next().unwrap_or("").to_ascii_uppercase();
    if filename.is_empty()
        || filename.starts_with('.')
        || filename.ends_with([' ', '.'])
        || filename.chars().any(|c| c.is_control() || r#"<>:"/\|?*"#.contains(c))
        || matches!(reserved.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (reserved.len() == 4
            && (reserved.starts_with("COM") || reserved.starts_with("LPT"))
            && matches!(reserved.as_bytes()[3], b'1'..=b'9'))
        || !matches!(filename.rsplit_once('.'), Some((stem, "md" | "mdx")) if !stem.is_empty())
    {
        return Err(FsError::InvalidDocumentName(filename.to_string()));
    }

    let parent = validate_path(root, directory)?;
    if !parent.is_dir() {
        return Err(FsError::MissingDirectory);
    }
    let normalized_directory = directory
        .replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("/");
    let relative_path = if normalized_directory.is_empty() {
        filename.to_string()
    } else {
        format!("{normalized_directory}/{filename}")
    };
    let path = validate_path(root, &relative_path)?;
    let mut temp = tempfile::NamedTempFile::new_in(&parent)?;
    use std::io::Write;
    temp.write_all(content.as_bytes())?;
    temp.persist_noclobber(&path).map_err(|error| {
        if error.error.kind() == std::io::ErrorKind::AlreadyExists {
            FsError::AlreadyExists
        } else {
            FsError::Io(error.error)
        }
    })?;
    Ok(relative_path)
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

fn is_backup_name(name: &str) -> bool {
    if name == "latest.bak" {
        return true;
    }

    let Some(date) = name.strip_suffix(".bak") else {
        return false;
    };
    let bytes = date.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

/// List the available backup filenames for a workspace document.
pub fn list_backups(
    backup_root: &Path,
    root: &Path,
    relative_path: &str,
) -> Result<Vec<String>, FsError> {
    validate_path(root, relative_path)?;
    let dir = backup_dir_for(backup_root, root, relative_path);
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    let mut backups = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if is_backup_name(&name) {
            backups.push(name);
        }
    }

    backups.sort_by(|a, b| match (a.as_str(), b.as_str()) {
        ("latest.bak", "latest.bak") => std::cmp::Ordering::Equal,
        ("latest.bak", _) => std::cmp::Ordering::Less,
        (_, "latest.bak") => std::cmp::Ordering::Greater,
        _ => b.cmp(a),
    });
    Ok(backups)
}

/// Restore a backup using the same conflict guard and pre-write backup as a
/// normal save. Backup names are restricted to the names emitted by
/// `list_backups`, never accepted as arbitrary paths.
pub fn restore_backup(
    backup_root: &Path,
    root: &Path,
    relative_path: &str,
    backup_name: &str,
    expected: Option<&str>,
) -> Result<(), FsError> {
    validate_path(root, relative_path)?;
    if !is_backup_name(backup_name) {
        return Err(FsError::BackupNotFound);
    }

    let backup_path = backup_dir_for(backup_root, root, relative_path).join(backup_name);
    let raw = match fs::read_to_string(backup_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(FsError::BackupNotFound);
        }
        Err(error) => return Err(error.into()),
    };

    write_file_guarded(root, relative_path, &raw, expected, Some(backup_root))
}

/// Move a file inside the workspace to the operating system's trash.
pub fn move_to_trash(root: &Path, relative_path: &str) -> Result<(), FsError> {
    let path = validate_path(root, relative_path)?;
    if !fs::metadata(&path)?.is_file() {
        return Err(FsError::NotFile);
    }

    trash::delete(&path).map_err(|error| FsError::Trash(error.to_string()))
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
    fn create_file_never_overwrites_an_existing_document() {
        let workspace = tempdir().unwrap();
        fs::create_dir(workspace.path().join("blog")).unwrap();
        let created = create_file(workspace.path(), "blog", "post.mdx", "first").unwrap();
        assert_eq!(created, "blog/post.mdx");
        assert!(matches!(
            create_file(workspace.path(), "blog", "post.mdx", "second"),
            Err(FsError::AlreadyExists)
        ));
        assert_eq!(fs::read_to_string(workspace.path().join("blog/post.mdx")).unwrap(), "first");
    }

    #[test]
    fn create_file_rejects_unsafe_names_and_directories() {
        let workspace = tempdir().unwrap();
        for name in ["../post.md", "nested\\post.md", "CON.md", "post.txt", ".hidden.md", "post?.md"] {
            assert!(matches!(
                create_file(workspace.path(), "", name, "content"),
                Err(FsError::InvalidDocumentName(_))
            ), "{name}");
        }
        assert!(create_file(workspace.path(), "../outside", "post.md", "").is_err());
        assert!(matches!(
            create_file(workspace.path(), "missing", "post.md", ""),
            Err(FsError::MissingDirectory)
        ));
    }

    #[test]
    fn created_paths_are_normalized_to_scanner_format() {
        let workspace = tempdir().unwrap();
        fs::create_dir(workspace.path().join("notes")).unwrap();
        assert_eq!(
            create_file(workspace.path(), "./notes/", "post.md", "").unwrap(),
            "notes/post.md"
        );
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
    fn lists_latest_and_daily_backups_newest_first() {
        let workspace = tempdir().unwrap();
        let backups = tempdir().unwrap();
        let bdir = backup_dir_for(backups.path(), workspace.path(), "a.md");
        fs::create_dir_all(&bdir).unwrap();
        fs::write(bdir.join("2025-01-02.bak"), "old").unwrap();
        fs::write(bdir.join("2025-01-10.bak"), "newer").unwrap();
        fs::write(bdir.join("latest.bak"), "latest").unwrap();
        fs::write(bdir.join("notes.txt"), "not a backup").unwrap();

        assert_eq!(
            list_backups(backups.path(), workspace.path(), "a.md").unwrap(),
            ["latest.bak", "2025-01-10.bak", "2025-01-02.bak"]
        );
    }

    #[test]
    fn restore_backup_saves_current_version_and_checks_conflicts() {
        let workspace = tempdir().unwrap();
        let backups = tempdir().unwrap();
        fs::write(workspace.path().join("a.md"), "current").unwrap();
        let bdir = backup_dir_for(backups.path(), workspace.path(), "a.md");
        fs::create_dir_all(&bdir).unwrap();
        fs::write(bdir.join("2025-01-02.bak"), "recovered").unwrap();

        restore_backup(
            backups.path(),
            workspace.path(),
            "a.md",
            "2025-01-02.bak",
            Some("current"),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(workspace.path().join("a.md")).unwrap(),
            "recovered"
        );
        assert_eq!(
            fs::read_to_string(bdir.join("latest.bak")).unwrap(),
            "current"
        );

        let conflict = restore_backup(
            backups.path(),
            workspace.path(),
            "a.md",
            "2025-01-02.bak",
            Some("stale"),
        );
        assert!(matches!(conflict, Err(FsError::Conflict)));
        assert_eq!(
            fs::read_to_string(workspace.path().join("a.md")).unwrap(),
            "recovered"
        );
    }

    #[test]
    fn restore_rejects_arbitrary_backup_paths() {
        let workspace = tempdir().unwrap();
        let backups = tempdir().unwrap();
        let result = restore_backup(
            backups.path(),
            workspace.path(),
            "a.md",
            "../outside.bak",
            None,
        );
        assert!(matches!(result, Err(FsError::BackupNotFound)));
    }

    #[test]
    fn moving_a_directory_to_trash_is_rejected() {
        let workspace = tempdir().unwrap();
        assert!(matches!(
            move_to_trash(workspace.path(), "."),
            Err(FsError::NotFile)
        ));
    }

    #[test]
    fn move_to_trash_removes_file_from_workspace() {
        let workspace = tempdir().unwrap();
        let path = workspace.path().join("a.md");
        fs::write(&path, "document").unwrap();

        move_to_trash(workspace.path(), "a.md").unwrap();

        assert!(!path.exists());
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
