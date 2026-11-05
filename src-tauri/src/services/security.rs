use std::path::{Component, Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("path traversal detected: {0}")]
    Traversal(String),
    #[error("absolute path not allowed: {0}")]
    AbsolutePath(String),
    #[error("path outside workspace root")]
    OutsideRoot,
}

/// Validate that a relative path is safe and within the workspace root.
pub fn validate_path(root: &Path, relative_path: &str) -> Result<PathBuf, SecurityError> {
    // Reject absolute paths
    if Path::new(relative_path).is_absolute() {
        return Err(SecurityError::AbsolutePath(relative_path.to_string()));
    }

    // Reject paths with `..` components
    let path = Path::new(relative_path);
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(SecurityError::Traversal(relative_path.to_string()));
        }
    }

    // Resolve and check it's within root
    let resolved = root.join(relative_path);

    // Try canonicalize; if file doesn't exist, check parent dir
    let canonical_root = root
        .canonicalize()
        .map_err(|_| SecurityError::OutsideRoot)?;

    if let Ok(canonical_resolved) = resolved.canonicalize() {
        if !canonical_resolved.starts_with(&canonical_root) {
            return Err(SecurityError::OutsideRoot);
        }
    } else {
        // File doesn't exist — walk up to find existing ancestor within root
        let mut ancestor = resolved.parent();
        while let Some(dir) = ancestor {
            if let Ok(canonical_dir) = dir.canonicalize() {
                if !canonical_dir.starts_with(&canonical_root) {
                    return Err(SecurityError::OutsideRoot);
                }
                break;
            }
            ancestor = dir.parent();
        }
    }

    Ok(resolved)
}

/// Validate that a repository path is exactly the canonical workspace root.
///
/// Git Sync may operate only on the repository that the user explicitly opened.
/// Canonicalizing both paths prevents `..`, symlink, and case-sensitive spelling
/// differences from bypassing the workspace boundary.
pub fn validate_repo_path(root: &Path, repo_path: &Path) -> Result<PathBuf, SecurityError> {
    let canonical_root = root.canonicalize().map_err(|_| SecurityError::OutsideRoot)?;
    let canonical_repo = repo_path
        .canonicalize()
        .map_err(|_| SecurityError::OutsideRoot)?;

    if canonical_repo != canonical_root {
        return Err(SecurityError::OutsideRoot);
    }

    Ok(canonical_repo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn valid_relative_path() {
        let dir = tempdir().unwrap();
        let result = validate_path(dir.path(), "docs/readme.md");
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_dot_dot() {
        let dir = tempdir().unwrap();
        let result = validate_path(dir.path(), "../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_absolute() {
        let dir = tempdir().unwrap();
        let result = validate_path(dir.path(), "/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_traversal_in_middle() {
        let dir = tempdir().unwrap();
        let result = validate_path(dir.path(), "docs/../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn repo_path_must_match_the_canonical_workspace_root() {
        let workspace = tempdir().unwrap();
        let outside = tempdir().unwrap();

        assert!(matches!(
            validate_repo_path(workspace.path(), outside.path()),
            Err(SecurityError::OutsideRoot)
        ));
        assert_eq!(
            validate_repo_path(workspace.path(), workspace.path()).unwrap(),
            workspace.path().canonicalize().unwrap()
        );
    }
}
