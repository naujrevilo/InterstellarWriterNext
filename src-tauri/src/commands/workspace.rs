use std::path::PathBuf;
use std::sync::Mutex;

use tauri::State;

use crate::domain::workspace::{WorkspaceInfo, WorkspaceKind};
use crate::services::{astro, scanner};

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WorkspaceStateError {
    #[error("no workspace is open")]
    NotOpen,
}

/// The only root accepted by document commands. The frontend may choose a
/// folder once, but subsequent operations are constrained to this canonical
/// directory rather than a caller-provided filesystem path.
#[derive(Default)]
pub struct WorkspaceState(Mutex<Option<PathBuf>>);

impl WorkspaceState {
    pub fn root(&self) -> Result<PathBuf, WorkspaceStateError> {
        self.0
            .lock()
            .ok()
            .and_then(|root| root.clone())
            .ok_or(WorkspaceStateError::NotOpen)
    }

    fn set_root(&self, root: PathBuf) {
        if let Ok(mut current) = self.0.lock() {
            *current = Some(root);
        }
    }
}

impl serde::Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[tauri::command]
pub fn detect_workspace_kind(
    state: State<'_, WorkspaceState>,
) -> Result<serde_json::Value, CommandError> {
    let path = state.root().map_err(|_| {
        CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no workspace is open",
        ))
    })?;
    let (kind, signals) = astro::detect_kind(&path);

    Ok(serde_json::json!({
        "kind": match kind {
            WorkspaceKind::Astro => "astro",
            WorkspaceKind::GenericMarkdown => "generic-markdown",
        },
        "signals": signals,
    }))
}

pub fn scan_workspace(root: String) -> Result<WorkspaceInfo, CommandError> {
    let path = PathBuf::from(&root).canonicalize()?;

    if !path.is_dir() {
        return Err(CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "workspace root does not exist",
        )));
    }

    let (kind, _) = astro::detect_kind(&path);
    let mut files = scanner::scan(&path);

    let collections = if kind == WorkspaceKind::Astro {
        let cols = astro::detect_collections(&path, &mut files);
        // For Astro workspaces, only files that live inside a `src/content/**`
        // collection are part of the content. Root-level .md files (README,
        // AGENTS.md, design notes, etc.) are project meta and don't belong
        // in the writer surface. Standalone mermaid files (`.mmd` /
        // `.mermaid`) are kept regardless of location.
        files.retain(|f| {
            f.collection.is_some()
                || matches!(
                    f.kind,
                    crate::domain::document::DocumentKind::MermaidFile
                )
        });
        cols
    } else {
        Vec::new()
    };

    Ok(WorkspaceInfo {
        root: path.to_string_lossy().to_string(),
        kind,
        collections,
        files,
    })
}

#[tauri::command]
pub fn open_workspace(
    root: String,
    state: State<'_, WorkspaceState>,
) -> Result<WorkspaceInfo, CommandError> {
    let info = scan_workspace(root)?;
    state.set_root(PathBuf::from(&info.root));
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn workspace_state_requires_an_opened_root() {
        let state = WorkspaceState::default();
        assert!(matches!(state.root(), Err(WorkspaceStateError::NotOpen)));
    }

    #[test]
    fn astro_scan_filters_to_content_collections_only() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Astro signals
        fs::write(root.join("astro.config.mjs"), "").unwrap();
        fs::create_dir_all(root.join("src/content/blog")).unwrap();

        // A blog post — should appear
        fs::write(
            root.join("src/content/blog/post.md"),
            "---\ntitle: Post\n---\nBody",
        )
        .unwrap();

        // Root-level markdown — must NOT appear in an Astro workspace
        fs::write(root.join("README.md"), "# README").unwrap();
        fs::write(root.join("AGENTS.md"), "# Agents").unwrap();
        fs::write(root.join("CHANGELOG.md"), "# Changelog").unwrap();

        // A standalone mermaid — must appear (kind is MermaidFile)
        fs::create_dir_all(root.join("docs/diagrams")).unwrap();
        fs::write(
            root.join("docs/diagrams/flow.mmd"),
            "graph TD\nA-->B",
        )
        .unwrap();

        let info = scan_workspace(root.to_string_lossy().to_string()).unwrap();

        assert_eq!(info.kind, WorkspaceKind::Astro);
        // 1 blog post + 1 standalone mermaid
        assert_eq!(info.files.len(), 2);
        for f in &info.files {
            // No root-level markdown should survive
            assert!(
                f.collection.is_some()
                    || matches!(f.kind, crate::domain::document::DocumentKind::MermaidFile),
                "unexpected file survived the filter: {:?}",
                f.relative_path
            );
            assert!(
                !f.relative_path.eq_ignore_ascii_case("README.md")
                    && !f.relative_path.eq_ignore_ascii_case("AGENTS.md")
                    && !f.relative_path.eq_ignore_ascii_case("CHANGELOG.md"),
                "root-level markdown must not appear: {}",
                f.relative_path
            );
        }
    }

    #[test]
    fn generic_markdown_scan_keeps_everything() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("README.md"), "# readme").unwrap();
        fs::write(root.join("notes.md"), "notes").unwrap();
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("nested/deep.md"), "deep").unwrap();

        let info = scan_workspace(root.to_string_lossy().to_string()).unwrap();

        assert_eq!(info.kind, WorkspaceKind::GenericMarkdown);
        assert_eq!(info.files.len(), 3);
    }
}
