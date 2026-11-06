import type { DocumentKind } from "./workspace";

export interface History {
  past: string[];
  future: string[];
}

export interface DocumentState {
  relativePath: string;
  kind: DocumentKind;
  frontmatter: Record<string, unknown>;
  body: string;
  raw: string;
  dirty: boolean;
  pendingMetadata: Record<string, string>;
  history: History;
}

export function createDocumentState(): DocumentState {
  return {
    relativePath: "",
    kind: "markdown",
    frontmatter: {},
    body: "",
    raw: "",
    dirty: false,
    pendingMetadata: {},
    history: { past: [], future: [] },
  };
}
