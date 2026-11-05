export type DocumentKind = "markdown" | "mdx" | "mermaid-file";
export type WorkspaceKind = "generic-markdown" | "astro";

export interface DocumentRef {
  relativePath: string;
  kind: DocumentKind;
  collection?: string | null;
  titleHint?: string | null;
  isDraft?: boolean;
}

export interface CollectionInfo {
  name: string;
  relativePath: string;
  documentCount: number;
}

export interface WorkspaceInfo {
  root: string;
  kind: WorkspaceKind;
  collections: CollectionInfo[];
  files: DocumentRef[];
}

export interface ParsedDocument {
  frontmatter: Record<string, unknown>;
  body: string;
  raw: string;
}
