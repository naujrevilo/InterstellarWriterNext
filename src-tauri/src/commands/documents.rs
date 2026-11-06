use std::path::{Path, PathBuf};

use base64::Engine;
use tauri::State;

use crate::commands::workspace::WorkspaceState;
use crate::domain::document::{DocumentKind, DocumentRef, ParsedDocument};
use crate::domain::frontmatter;
use crate::domain::workspace::WorkspaceKind;
use crate::services::astro;
use crate::services::fs as app_fs;
use crate::services::security::SecurityError;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("security: {0}")]
    Security(#[from] SecurityError),
    #[error("frontmatter: {0}")]
    Frontmatter(#[from] frontmatter::FrontmatterError),
    #[error("fs: {0}")]
    Fs(#[from] app_fs::FsError),
    #[error("image not found: {0}")]
    ImageNotFound(String),
    #[error("unsupported image type: {0}")]
    UnsupportedImageType(String),
    #[error("select an existing content collection")]
    InvalidCollection,
    #[error("a document title is required")]
    MissingTitle,
}

impl serde::Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

fn opened_root(state: &WorkspaceState) -> Result<PathBuf, CommandError> {
    state.root().map_err(|_| {
        CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no workspace is open",
        ))
    })
}

#[tauri::command]
pub fn read_document(
    state: State<'_, WorkspaceState>,
    relative_path: String,
) -> Result<serde_json::Value, CommandError> {
    let root_path = opened_root(&state)?;
    let raw = app_fs::read_file(&root_path, &relative_path)?;
    Ok(serde_json::json!({ "raw": raw }))
}

#[tauri::command]
pub fn create_document(
    state: State<'_, WorkspaceState>,
    title: String,
    filename: String,
    collection: Option<String>,
    directory: String,
) -> Result<DocumentRef, CommandError> {
    let root = opened_root(&state)?;
    create_document_in_root(&root, &title, &filename, collection.as_deref(), &directory)
}

fn create_document_in_root(
    root: &Path,
    title: &str,
    filename: &str,
    collection: Option<&str>,
    directory: &str,
) -> Result<DocumentRef, CommandError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(CommandError::MissingTitle);
    }

    let (kind, _) = astro::detect_kind(root);
    let (destination, collection) = if kind == WorkspaceKind::Astro {
        let collections = astro::detect_collections(root, &mut []);
        let selected = collections
            .into_iter()
            .find(|item| Some(item.name.as_str()) == collection)
            .ok_or(CommandError::InvalidCollection)?;
        (selected.relative_path, Some(selected.name))
    } else {
        (directory.to_string(), None)
    };

    let mut frontmatter = serde_yaml::Mapping::new();
    frontmatter.insert("title".into(), title.into());
    let raw = frontmatter::serialize(&frontmatter, "");
    let relative_path = app_fs::create_file(root, &destination, filename, &raw)?;
    let document_kind = if filename.ends_with(".mdx") {
        DocumentKind::Mdx
    } else {
        DocumentKind::Markdown
    };
    Ok(DocumentRef {
        relative_path,
        kind: document_kind,
        collection,
        title_hint: Some(title.to_string()),
        is_draft: false,
    })
}

#[tauri::command]
pub fn write_document(
    app: tauri::AppHandle,
    state: State<'_, WorkspaceState>,
    relative_path: String,
    raw: String,
    expected_raw: Option<String>,
) -> Result<serde_json::Value, CommandError> {
    use tauri::Manager;
    let root_path = opened_root(&state)?;
    let backup_root = app
        .path()
        .app_local_data_dir()
        .map(|p| p.join("backups"))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    app_fs::write_file_guarded(
        &root_path,
        &relative_path,
        &raw,
        expected_raw.as_deref(),
        Some(&backup_root),
    )?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub fn list_document_backups(
    app: tauri::AppHandle,
    state: State<'_, WorkspaceState>,
    relative_path: String,
) -> Result<Vec<String>, CommandError> {
    use tauri::Manager;
    let root_path = opened_root(&state)?;
    let backup_root = app
        .path()
        .app_local_data_dir()
        .map(|p| p.join("backups"))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    Ok(app_fs::list_backups(
        &backup_root,
        &root_path,
        &relative_path,
    )?)
}

#[tauri::command]
pub fn restore_document_backup(
    app: tauri::AppHandle,
    state: State<'_, WorkspaceState>,
    relative_path: String,
    backup_name: String,
    expected_raw: Option<String>,
) -> Result<serde_json::Value, CommandError> {
    use tauri::Manager;
    let root_path = opened_root(&state)?;
    let backup_root = app
        .path()
        .app_local_data_dir()
        .map(|p| p.join("backups"))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    app_fs::restore_backup(
        &backup_root,
        &root_path,
        &relative_path,
        &backup_name,
        expected_raw.as_deref(),
    )?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub fn move_document_to_trash(
    state: State<'_, WorkspaceState>,
    relative_path: String,
) -> Result<serde_json::Value, CommandError> {
    let root_path = opened_root(&state)?;
    app_fs::move_to_trash(&root_path, &relative_path)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub fn parse_document(raw: String, kind: DocumentKind) -> Result<ParsedDocument, CommandError> {
    match kind {
        DocumentKind::MermaidFile => Ok(ParsedDocument {
            frontmatter: serde_yaml::Mapping::new(),
            body: raw.clone(),
            raw,
        }),
        _ => Ok(frontmatter::parse(&raw)?),
    }
}

#[tauri::command]
pub fn serialize_document(
    frontmatter: serde_yaml::Mapping,
    body: String,
    kind: DocumentKind,
) -> Result<serde_json::Value, CommandError> {
    let raw = match kind {
        DocumentKind::MermaidFile => body,
        _ => frontmatter::serialize(&frontmatter, &body),
    };
    Ok(serde_json::json!({ "raw": raw }))
}

/// Resolve an image referenced from a markdown/MDX file to a
/// `data:` URL the preview can render directly.
///
/// `image_path` can be either:
///   - absolute (starts with a drive letter on Windows or `/` on Unix)
///   - relative to the directory of `from_file`
///   - relative to the workspace root (rare, but tolerated)
///
/// The resolved path is always validated to be inside `root` before
/// we read a single byte from disk. This command is the only way
/// the preview can show local images, so it is the perimeter.
#[tauri::command]
pub fn resolve_image(
    state: State<'_, WorkspaceState>,
    from_file: String,
    image_path: String,
) -> Result<serde_json::Value, CommandError> {
    let root_path = opened_root(&state)?;
    resolve_image_from_root(&root_path, &from_file, &image_path)
}

fn resolve_image_from_root(
    root_path: &Path,
    from_file: &str,
    image_path: &str,
) -> Result<serde_json::Value, CommandError> {
    let canonical_root = root_path
        .canonicalize()
        .map_err(|_| SecurityError::OutsideRoot)?;

    let resolved = resolve_image_path(root_path, from_file, image_path)?;

    let canonical_resolved = resolved
        .canonicalize()
        .map_err(|_| CommandError::ImageNotFound(resolved.display().to_string()))?;

    if !canonical_resolved.starts_with(&canonical_root) {
        return Err(SecurityError::OutsideRoot.into());
    }

    let bytes = std::fs::read(&canonical_resolved)?;
    let mime = mime_for_path(&canonical_resolved);
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    Ok(serde_json::json!({
        "dataUrl": format!("data:{};base64,{}", mime, b64),
        "mime": mime,
        "size": bytes.len(),
    }))
}

/// Resolve a (possibly relative) image path to an absolute filesystem
/// path. Does NOT touch the disk for existence checks.
fn resolve_image_path(
    root: &Path,
    from_file: &str,
    image_path: &str,
) -> Result<PathBuf, CommandError> {
    let p = Path::new(image_path);

    if p.is_absolute() {
        // Absolute path — caller is on the hook for the workspace rule.
        // The caller (resolve_image) will canonicalize + check containment.
        return Ok(p.to_path_buf());
    }

    // Relative path: try the file's directory first, then root.
    let from_file_path = if from_file.is_empty() {
        root.to_path_buf()
    } else {
        root.join(from_file)
    };

    if let Some(parent) = from_file_path.parent() {
        let candidate = normalize_path(&parent.join(image_path));
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // Fall back to workspace root.
    let candidate = normalize_path(&root.join(image_path));
    if candidate.exists() {
        return Ok(candidate);
    }

    Err(CommandError::ImageNotFound(image_path.to_string()))
}

/// Collapse `.` and `..` components without touching the disk.
fn normalize_path(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

fn mime_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn creates_a_document_in_the_selected_astro_collection() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("astro.config.mjs"), "").unwrap();
        fs::create_dir_all(dir.path().join("src/content/blog")).unwrap();
        fs::create_dir_all(dir.path().join("src/content/projects")).unwrap();

        let doc = create_document_in_root(
            dir.path(), "A: title", "a-title.mdx", Some("blog"), "../ignored",
        )
        .unwrap();
        assert_eq!(doc.relative_path, "src/content/blog/a-title.mdx");
        assert_eq!(doc.kind, DocumentKind::Mdx);
        assert_eq!(doc.collection.as_deref(), Some("blog"));
        let raw = fs::read_to_string(dir.path().join(&doc.relative_path)).unwrap();
        assert_eq!(frontmatter::parse(&raw).unwrap().frontmatter["title"], "A: title");
        assert!(matches!(
            create_document_in_root(dir.path(), "Test", "other.md", Some("../blog"), ""),
            Err(CommandError::InvalidCollection)
        ));
    }

    #[test]
    fn creates_in_generic_workspaces_and_rejects_empty_titles() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("notes")).unwrap();
        let doc = create_document_in_root(dir.path(), "Title", "note.md", None, "notes").unwrap();
        assert_eq!(doc.relative_path, "notes/note.md");
        assert_eq!(doc.kind, DocumentKind::Markdown);
        assert!(matches!(
            create_document_in_root(dir.path(), "  ", "empty.md", None, ""),
            Err(CommandError::MissingTitle)
        ));
    }

    #[test]
    fn mime_detection_works() {
        assert_eq!(mime_for_path(Path::new("/x/y.png")), "image/png");
        assert_eq!(mime_for_path(Path::new("/x/y.JPG")), "image/jpeg");
        assert_eq!(mime_for_path(Path::new("/x/y.svg")), "image/svg+xml");
        assert_eq!(mime_for_path(Path::new("/x/y")), "application/octet-stream");
    }

    #[test]
    fn resolve_relative_to_file_dir() {
        let dir = tempdir().unwrap();
        // Realistic structure: cover lives in src/assets/, post is deeper.
        let assets = dir.path().join("src").join("assets");
        fs::create_dir_all(&assets).unwrap();
        fs::write(assets.join("cover.png"), b"\x89PNG fake").unwrap();

        // From src/content/blog/post.mdx, "../../assets/cover.png" → src/assets/cover.png
        let abs = resolve_image_path(
            dir.path(),
            "src/content/blog/post.mdx",
            "../../assets/cover.png",
        )
        .unwrap();
        let s = abs.to_string_lossy().replace('\\', "/");
        assert!(s.ends_with("src/assets/cover.png"), "got: {}", s);
    }

    #[test]
    fn normalize_works() {
        let p = normalize_path(Path::new("a/b/../c/./d"));
        assert_eq!(p.to_string_lossy().replace('\\', "/"), "a/c/d");
    }

    #[test]
    fn resolve_falls_back_to_root() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("favicon.ico"), b"ico").unwrap();

        let abs =
            resolve_image_path(dir.path(), "src/content/blog/post.mdx", "favicon.ico").unwrap();
        assert!(abs.is_absolute());
    }

    #[test]
    fn resolve_image_outside_root_is_rejected() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        fs::write(outside.path().join("secret.png"), b"x").unwrap();

        let outside_path = outside.path().join("secret.png");
        let result = resolve_image_from_root(
            dir.path(),
            "src/post.mdx",
            &outside_path.to_string_lossy(),
        );
        assert!(result.is_err());
    }
}
