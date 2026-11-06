use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceKind {
    GenericMarkdown,
    Astro,
}

/// Top-level payload sent to the frontend after a scan.
/// `camelCase` here keeps it consistent with the TS contract
/// (`src/lib/types/workspace.ts`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub root: String,
    pub kind: WorkspaceKind,
    pub collections: Vec<CollectionInfo>,
    pub files: Vec<crate::domain::document::DocumentRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionInfo {
    pub name: String,
    pub relative_path: String,
    pub document_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::document::{DocumentKind, DocumentRef};

    /// The TS contract uses camelCase. Pin the JSON shape so future
    /// refactors can't silently break the frontend.
    #[test]
    fn collection_info_serializes_as_camel_case() {
        let col = CollectionInfo {
            name: "blog".to_string(),
            relative_path: "src/content/blog".to_string(),
            document_count: 3,
        };

        let json = serde_json::to_value(&col).unwrap();
        let obj = json.as_object().unwrap();

        assert!(obj.contains_key("relativePath"));
        assert!(obj.contains_key("documentCount"));
        assert!(!obj.contains_key("relative_path"));
        assert!(!obj.contains_key("document_count"));
        assert_eq!(obj["documentCount"], 3);
    }

    #[test]
    fn workspace_info_serializes_with_camel_case_files() {
        let info = WorkspaceInfo {
            root: "/tmp".to_string(),
            kind: WorkspaceKind::Astro,
            collections: vec![],
            files: vec![DocumentRef {
                relative_path: "x.md".to_string(),
                kind: DocumentKind::Markdown,
                collection: None,
                title_hint: None,
                is_draft: false,
            }],
        };

        let json = serde_json::to_value(&info).unwrap();
        let obj = json.as_object().unwrap();
        // The nested file's fields should also be camelCase.
        let file = &obj["files"][0];
        assert!(file.as_object().unwrap().contains_key("relativePath"));
    }
}
