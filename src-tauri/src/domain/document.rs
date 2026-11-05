use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentKind {
    Markdown,
    Mdx,
    MermaidFile,
}

impl DocumentKind {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "md" => Some(Self::Markdown),
            "mdx" => Some(Self::Mdx),
            "mmd" | "mermaid" => Some(Self::MermaidFile),
            _ => None,
        }
    }
}

/// `DocumentRef` crosses the IPC boundary into the TypeScript frontend.
/// The TS contract (`src/lib/types/workspace.ts`) uses camelCase, so we
/// serialize on camelCase. Without `rename_all = "camelCase"`, serde would
/// emit `relative_path` / `title_hint` and the frontend would receive
/// `undefined` for those fields — which is exactly what caused the
/// `Cannot read properties of undefined (reading 'split')` crash in Sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRef {
    pub relative_path: String,
    pub kind: DocumentKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_hint: Option<String>,
    /// `true` when the frontmatter says `draft: true`. The scanner peeks
    /// the first 30 lines of the file to detect this; it does NOT load
    /// the full content (the actual frontmatter is parsed lazily when
    /// the file is opened).
    #[serde(default)]
    pub is_draft: bool,
}

impl DocumentRef {
    /// Construct a basic `DocumentRef` (used by tests / ad-hoc callers
    /// that don't need the draft flag).
    pub fn new(relative_path: String, kind: DocumentKind) -> Self {
        Self {
            relative_path,
            kind,
            collection: None,
            title_hint: None,
            is_draft: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedDocument {
    pub frontmatter: serde_yaml::Mapping,
    pub body: String,
    pub raw: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The TS contract in `src/lib/types/workspace.ts` uses camelCase.
    /// This test pins the JSON shape so a future refactor can't silently
    /// regress to snake_case (which made the sidebar blow up with
    /// `Cannot read properties of undefined (reading 'split')`).
    #[test]
    fn document_ref_serializes_as_camel_case() {
        let doc = DocumentRef {
            relative_path: "src/content/blog/post.md".to_string(),
            kind: DocumentKind::Markdown,
            collection: Some("blog".to_string()),
            title_hint: Some("Post".to_string()),
            is_draft: true,
        };

        let json = serde_json::to_value(&doc).unwrap();
        let obj = json.as_object().unwrap();

        // camelCase keys present
        assert!(obj.contains_key("relativePath"));
        assert!(obj.contains_key("titleHint"));
        assert!(obj.contains_key("isDraft"));
        // snake_case keys MUST NOT be present
        assert!(!obj.contains_key("relative_path"));
        assert!(!obj.contains_key("title_hint"));
        assert!(!obj.contains_key("is_draft"));
        // values are correct
        assert_eq!(obj["relativePath"], "src/content/blog/post.md");
        assert_eq!(obj["titleHint"], "Post");
        assert_eq!(obj["isDraft"], true);
        assert_eq!(obj["kind"], "markdown");
        assert_eq!(obj["collection"], "blog");
    }

    #[test]
    fn document_kind_serializes_as_kebab_case() {
        assert_eq!(
            serde_json::to_value(DocumentKind::MermaidFile).unwrap(),
            "mermaid-file"
        );
        assert_eq!(
            serde_json::to_value(DocumentKind::Mdx).unwrap(),
            "mdx"
        );
        assert_eq!(
            serde_json::to_value(DocumentKind::Markdown).unwrap(),
            "markdown"
        );
    }
}
