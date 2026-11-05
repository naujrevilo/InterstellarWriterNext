use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use walkdir::WalkDir;

use crate::domain::document::{DocumentKind, DocumentRef};

/// Hard cap on how many files we return from a single scan.
/// If a workspace has more than this, we stop walking and return what we have.
/// The frontend can still re-scan a sub-folder if it needs more.
pub const MAX_FILES_PER_SCAN: usize = 5_000;
/// Hard cap on directory depth from the workspace root.
const MAX_DEPTH: usize = 16;
/// How many leading lines of a file we peek to detect `draft: true` in
/// the frontmatter. 30 is enough for any reasonable frontmatter and
/// keeps the I/O per file tiny.
const DRAFT_PEEK_LINES: usize = 30;
/// Maximum bytes read per file during the draft peek (defensive bound).
const DRAFT_PEEK_BYTES: u64 = 8 * 1024;

const EXCLUDE_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "dist",
    "build",
    ".astro",
    ".next",
    "target",
    ".cache",
    ".turbo",
    ".svelte-kit",
    ".vercel",
    "out",
    "coverage",
    ".output",
    ".vinxi",
    ".nuxt",
    ".angular",
    ".gradle",
    ".idea",
    ".vscode",
];

/// Peek at the first few lines of a file and return `true` if the YAML
/// frontmatter says `draft: true`. Reads at most `DRAFT_PEEK_LINES` lines
/// and at most `DRAFT_PEEK_BYTES` bytes — bounded I/O so it never hangs.
///
/// Returns `false` for:
/// - files without frontmatter
/// - files where reading fails (silently — we'd rather show no badge than
///   a wrong one)
/// - files where `draft:` is missing or set to anything other than `true`
pub fn peek_is_draft(path: &Path) -> bool {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut reader = BufReader::new(file.take(DRAFT_PEEK_BYTES));
    let mut line = String::new();
    let mut in_frontmatter = false;
    let mut saw_first_delim = false;

    for _ in 0..DRAFT_PEEK_LINES {
        line.clear();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed == "---" {
            if saw_first_delim && in_frontmatter {
                // Closing delimiter — no draft flag in header
                return false;
            }
            saw_first_delim = true;
            in_frontmatter = true;
            continue;
        }
        if !in_frontmatter {
            // Outside frontmatter (or no frontmatter at all) — keep scanning
            // until we see the opening `---` or hit EOF.
            if saw_first_delim {
                // We saw a leading `---` then content not in frontmatter —
                // unusual; keep scanning.
                continue;
            }
            continue;
        }
        // We're inside the frontmatter. Look for `draft:`.
        if let Some(rest) = trimmed.strip_prefix("draft:") {
            let value = rest.trim().to_ascii_lowercase();
            return value == "true" || value == "yes" || value == "1";
        }
    }
    false
}

/// Scan a directory for supported document files.
///
/// This must stay fast. We:
/// - DO walk the tree with walkdir
/// - DO read up to ~8KB of each .md / .mdx file just to detect
///   `draft: true` in the frontmatter (for the sidebar indicator).
///   The full frontmatter is still parsed lazily on file open.
/// - DO NOT read the entire file or parse the body.
pub fn scan(root: &Path) -> Vec<DocumentRef> {
    let mut files: Vec<DocumentRef> = Vec::with_capacity(256);

    let walker = WalkDir::new(root)
        .follow_links(false)
        .max_depth(MAX_DEPTH)
        .max_open(64)
        .into_iter()
        .filter_entry(|e| {
            if !e.file_type().is_dir() {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            !EXCLUDE_DIRS.iter().any(|ex| name == *ex)
        });

    for entry in walker.filter_map(|e| e.ok()) {
        if files.len() >= MAX_FILES_PER_SCAN {
            break;
        }

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let ext = match path.extension().and_then(|s| s.to_str()) {
            Some(e) => e,
            None => continue,
        };
        let kind = match DocumentKind::from_extension(ext) {
            Some(k) => k,
            None => continue,
        };

        let relative = match path.strip_prefix(root) {
            Ok(p) => p.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };

        let title_hint = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string());

        // Peek for `draft: true` only on markdown / mdx — standalone
        // mermaid files have no frontmatter.
        let is_draft = matches!(kind, DocumentKind::Markdown | DocumentKind::Mdx)
            && peek_is_draft(path);

        files.push(DocumentRef {
            relative_path: relative,
            kind,
            collection: None, // filled by astro adapter if applicable
            title_hint,
            is_draft,
        });
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Instant;
    use tempfile::tempdir;

    #[test]
    fn scan_finds_markdown_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("readme.md"), "# Hello").unwrap();
        fs::write(root.join("page.mdx"), "---\ntitle: Page\n---\nBody").unwrap();
        fs::write(root.join("diagram.mmd"), "graph TD\n  A-->B").unwrap();
        fs::write(root.join("flow.mermaid"), "graph LR\n  X-->Y").unwrap();
        fs::write(root.join("ignore.txt"), "not supported").unwrap();

        let files = scan(root);
        assert_eq!(files.len(), 4);

        let kinds: Vec<_> = files.iter().map(|f| &f.kind).collect();
        assert!(kinds.contains(&&DocumentKind::Markdown));
        assert!(kinds.contains(&&DocumentKind::Mdx));
        assert!(kinds.contains(&&DocumentKind::MermaidFile));
    }

    #[test]
    fn scan_excludes_node_modules() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("valid.md"), "content").unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::write(root.join("node_modules/pkg/readme.md"), "ignored").unwrap();

        let files = scan(root);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "valid.md");
    }

    #[test]
    fn scan_fallback_title_from_filename() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("my-document.md"), "No frontmatter here").unwrap();

        let files = scan(root);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].title_hint, Some("my-document".to_string()));
    }

    #[test]
    fn scan_nested_directories() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("docs/guides")).unwrap();
        fs::write(root.join("docs/guide.md"), "content").unwrap();
        fs::write(root.join("docs/guides/advanced.md"), "advanced").unwrap();

        let files = scan(root);
        assert_eq!(files.len(), 2);
    }

    /// Lock the regression that caused the "Opening..." infinite spinner:
    /// the scan used to read the contents of every file. On a workspace with
    /// many files, that was unacceptably slow. The scan must stay fast even
    /// when there are hundreds of files.
    #[test]
    fn scan_is_fast_on_many_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        for i in 0..500 {
            let sub = root.join(format!("group-{}", i / 50));
            fs::create_dir_all(&sub).unwrap();
            fs::write(
                sub.join(format!("post-{}.md", i)),
                format!("---\ntitle: Post {}\n---\n\nBody {}", i, i),
            )
            .unwrap();
        }

        let start = Instant::now();
        let files = scan(root);
        let elapsed = start.elapsed();

        assert_eq!(files.len(), 500);
        assert!(
            elapsed.as_millis() < 1000,
            "scan took {}ms — it must NOT read file contents",
            elapsed.as_millis()
        );
    }

    /// Scan must not blow up on binary / non-UTF8 files.
    #[test]
    fn scan_skips_binary_files_safely() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("post.md"), "---\ntitle: Real\n---\nBody").unwrap();
        let bytes: &[u8] = &[0xFF, 0xFE, 0x00, 0x01, 0x80, 0x90];
        fs::write(root.join("garbage.md"), bytes).unwrap();

        let files = scan(root);
        // Both files are listed (we don't read content), but the scan must
        // not error or hang on the binary one.
        assert_eq!(files.len(), 2);
    }

    /// Symlinks (especially recursive ones) must not be followed.
    /// This prevents infinite walks on real-world workspaces.
    #[cfg(unix)]
    #[test]
    fn scan_does_not_follow_symlinks() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("a.md"), "real").unwrap();
        // A symlink to the root itself — would loop forever if followed.
        symlink(root, root.join("loop")).unwrap();

        let start = Instant::now();
        let files = scan(root);
        let elapsed = start.elapsed();

        // We only expect the one real file; the symlinked dir must be skipped.
        assert_eq!(files.len(), 1);
        assert!(elapsed.as_secs() < 2);
    }

    /// Scan must respect the file cap and stop gracefully on huge trees.
    #[test]
    fn scan_caps_at_max_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create 2x the cap, all in one shallow tree.
        for i in 0..(MAX_FILES_PER_SCAN * 2) {
            fs::write(root.join(format!("post-{}.md", i)), "x").unwrap();
        }

        let start = Instant::now();
        let files = scan(root);
        let elapsed = start.elapsed();

        assert_eq!(files.len(), MAX_FILES_PER_SCAN);
        assert!(elapsed.as_millis() < 2000);
    }

    /// Lock the behavior of the bounded frontmatter peek that surfaces
    /// the `isDraft` flag in the sidebar.
    #[test]
    fn peek_is_draft_detects_draft_true() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::write(
            root.join("draft.md"),
            "---\ntitle: A\ndraft: true\n---\nbody",
        )
        .unwrap();
        fs::write(
            root.join("published.md"),
            "---\ntitle: B\ndraft: false\n---\nbody",
        )
        .unwrap();
        fs::write(root.join("no-fm.md"), "no frontmatter here").unwrap();
        fs::write(
            root.join("missing.md"),
            "---\ntitle: C\n---\nbody",
        )
        .unwrap();
        fs::write(
            root.join("yes.md"),
            "---\ndraft: yes\n---\nbody",
        )
        .unwrap();

        assert!(peek_is_draft(&root.join("draft.md")));
        assert!(!peek_is_draft(&root.join("published.md")));
        assert!(!peek_is_draft(&root.join("no-fm.md")));
        assert!(!peek_is_draft(&root.join("missing.md")));
        assert!(peek_is_draft(&root.join("yes.md")));
    }

    /// The end-to-end `scan` must propagate the draft flag onto `DocumentRef`.
    #[test]
    fn scan_propagates_draft_flag() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::write(
            root.join("draft.md"),
            "---\ndraft: true\n---\nbody",
        )
        .unwrap();
        fs::write(
            root.join("published.md"),
            "---\ndraft: false\n---\nbody",
        )
        .unwrap();
        fs::write(root.join("plain.md"), "no frontmatter").unwrap();

        let files = scan(root);
        let by_name: std::collections::HashMap<_, _> =
            files.iter().map(|f| (f.relative_path.clone(), f)).collect();

        assert!(by_name["draft.md"].is_draft);
        assert!(!by_name["published.md"].is_draft);
        assert!(!by_name["plain.md"].is_draft);
    }

    /// 500 markdown files with frontmatter must still scan in well under 1s.
    /// The bounded peek adds I/O; this test catches any regression.
    #[test]
    fn scan_with_draft_peek_stays_fast() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        for i in 0..500 {
            let draft = i % 3 == 0;
            let content = format!(
                "---\ntitle: Post {}\ndraft: {}\n---\n\nBody {}",
                i, draft, i
            );
            fs::write(root.join(format!("post-{}.md", i)), content).unwrap();
        }

        let start = Instant::now();
        let files = scan(root);
        let elapsed = start.elapsed();

        assert_eq!(files.len(), 500);
        assert!(
            elapsed.as_millis() < 2000,
            "scan with draft peek took {}ms",
            elapsed.as_millis()
        );

        // Every 3rd file should be a draft
        let drafts = files.iter().filter(|f| f.is_draft).count();
        assert_eq!(drafts, 167);
    }
}
