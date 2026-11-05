use std::path::Path;

use crate::domain::document::DocumentRef;
use crate::domain::workspace::{CollectionInfo, WorkspaceKind};

/// Detect if a workspace is an Astro project.
pub fn detect_kind(root: &Path) -> (WorkspaceKind, Vec<String>) {
    let mut signals = Vec::new();

    // Check for astro.config.*
    for ext in &["mjs", "js", "ts", "cjs"] {
        let name = format!("astro.config.{}", ext);
        if root.join(&name).exists() {
            signals.push(name);
        }
    }

    // Check for src/content/
    if root.join("src/content").exists() {
        signals.push("src/content/".to_string());
    }

    // Check for src/content.config.*
    for ext in &["ts", "js", "mjs"] {
        let name = format!("src/content.config.{}", ext);
        if root.join(&name).exists() {
            signals.push(name);
        }
    }

    // Check package.json for astro dependency
    let pkg_path = root.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_path) {
        if content.contains("\"astro\"") {
            signals.push("package.json contains astro".to_string());
        }
    }

    if signals.len() >= 2 {
        (WorkspaceKind::Astro, signals)
    } else {
        (WorkspaceKind::GenericMarkdown, signals)
    }
}

/// Detect collections for Astro workspaces.
pub fn detect_collections(root: &Path, files: &mut [DocumentRef]) -> Vec<CollectionInfo> {
    let content_dir = root.join("src/content");
    if !content_dir.exists() {
        return Vec::new();
    }

    let mut collections = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&content_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            let relative = path
                .strip_prefix(root)
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();

            // Count documents and assign collection to files
            let mut count = 0;
            for file in files.iter_mut() {
                if file.relative_path.starts_with(&relative) {
                    file.collection = Some(name.clone());
                    count += 1;
                }
            }

            collections.push(CollectionInfo {
                name,
                relative_path: relative,
                document_count: count,
            });
        }
    }

    collections
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detect_generic_markdown() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("readme.md"), "# Hello").unwrap();

        let (kind, signals) = detect_kind(root);
        assert_eq!(kind, WorkspaceKind::GenericMarkdown);
        assert!(signals.is_empty());
    }

    #[test]
    fn detect_astro_workspace() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create Astro signals
        fs::write(root.join("astro.config.mjs"), "").unwrap();
        fs::create_dir_all(root.join("src/content/blog")).unwrap();
        fs::write(
            root.join("package.json"),
            r#"{"dependencies": {"astro": "^4.0.0"}}"#,
        )
        .unwrap();

        let (kind, signals) = detect_kind(root);
        assert_eq!(kind, WorkspaceKind::Astro);
        assert!(signals.len() >= 2);
        assert!(signals.contains(&"astro.config.mjs".to_string()));
        assert!(signals.contains(&"src/content/".to_string()));
    }

    #[test]
    fn detect_collections_assigns_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create collection structure
        fs::create_dir_all(root.join("src/content/blog")).unwrap();
        fs::create_dir_all(root.join("src/content/projects")).unwrap();
        fs::write(root.join("src/content/blog/post1.md"), "---\ntitle: Post 1\n---\nBody").unwrap();
        fs::write(root.join("src/content/projects/proj1.md"), "---\ntitle: Proj 1\n---\nBody").unwrap();

        let mut files = vec![
            DocumentRef {
                relative_path: "src/content/blog/post1.md".to_string(),
                kind: crate::domain::document::DocumentKind::Markdown,
                collection: None,
                title_hint: Some("Post 1".to_string()),
                is_draft: false,
            },
            DocumentRef {
                relative_path: "src/content/projects/proj1.md".to_string(),
                kind: crate::domain::document::DocumentKind::Markdown,
                collection: None,
                title_hint: Some("Proj 1".to_string()),
                is_draft: false,
            },
        ];

        let collections = detect_collections(root, &mut files);

        assert_eq!(collections.len(), 2);
        assert_eq!(files[0].collection, Some("blog".to_string()));
        assert_eq!(files[1].collection, Some("projects".to_string()));

        let blog_col = collections.iter().find(|c| c.name == "blog").unwrap();
        assert_eq!(blog_col.document_count, 1);
    }

    #[test]
    fn no_content_dir_returns_empty() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let mut files = vec![];
        let collections = detect_collections(root, &mut files);
        assert!(collections.is_empty());
    }
}
