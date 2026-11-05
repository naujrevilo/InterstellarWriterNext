use std::fs;
use std::path::Path;

use super::security::{validate_path, SecurityError};

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("security: {0}")]
    Security(#[from] SecurityError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Read a file within the workspace root.
pub fn read_file(root: &Path, relative_path: &str) -> Result<String, FsError> {
    let path = validate_path(root, relative_path)?;
    let content = fs::read_to_string(&path)?;
    Ok(content)
}

/// Write a file within the workspace root using atomic temp-file replacement.
pub fn write_file(root: &Path, relative_path: &str, content: &str) -> Result<(), FsError> {
    let path = validate_path(root, relative_path)?;

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
    fn rejects_outside_root() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let result = read_file(root, "../outside.md");
        assert!(result.is_err());
    }
}
