export type MetadataWidget =
  | "text"
  | "textarea"
  | "datetime"
  | "path"
  | "string-array"
  | "checkbox";

export interface MetadataFieldDef {
  key: string;
  label: string;
  widget: MetadataWidget;
  required?: boolean;
}

export const BLOG_FIELDS: MetadataFieldDef[] = [
  { key: "title", label: "Title", widget: "text", required: true },
  { key: "description", label: "Description", widget: "textarea" },
  { key: "date", label: "Date", widget: "datetime" },
  { key: "image", label: "Image", widget: "path" },
  { key: "tags", label: "Tags", widget: "string-array" },
  { key: "categories", label: "Categories", widget: "string-array" },
  { key: "draft", label: "Draft", widget: "checkbox" },
];

export const APP_LANDINGS_FIELDS: MetadataFieldDef[] = [
  { key: "title", label: "Title", widget: "text", required: true },
  { key: "description", label: "Description", widget: "textarea" },
  { key: "longDescription", label: "Long Description", widget: "textarea" },
  { key: "technologies", label: "Technologies", widget: "string-array" },
  { key: "status", label: "Status", widget: "text" },
  { key: "primaryCtaLabel", label: "Primary CTA Label", widget: "text" },
  { key: "primaryCtaUrl", label: "Primary CTA URL", widget: "text" },
  { key: "secondaryCtaLabel", label: "Secondary CTA Label", widget: "text" },
  { key: "secondaryCtaUrl", label: "Secondary CTA URL", widget: "text" },
  { key: "repoUrl", label: "Repo URL", widget: "text" },
  { key: "image", label: "Image", widget: "path" },
  { key: "pathSlug", label: "Path Slug", widget: "text" },
];

export const PROJECTS_FIELDS: MetadataFieldDef[] = [
  { key: "title", label: "Title", widget: "text", required: true },
  { key: "description", label: "Description", widget: "textarea" },
  { key: "longDescription", label: "Long Description", widget: "textarea" },
  { key: "publishDate", label: "Publish Date", widget: "datetime" },
  { key: "technologies", label: "Technologies", widget: "string-array" },
  { key: "category", label: "Category", widget: "text" },
  { key: "status", label: "Status", widget: "text" },
  { key: "repoUrl", label: "Repo URL", widget: "text" },
  { key: "demoUrl", label: "Demo URL", widget: "text" },
  { key: "image", label: "Image", widget: "path" },
  { key: "featured", label: "Featured", widget: "checkbox" },
];

export const DOCS_FIELDS: MetadataFieldDef[] = [
  { key: "title", label: "Title", widget: "text", required: true },
  { key: "description", label: "Description", widget: "textarea" },
  { key: "sidebar", label: "Sidebar Label", widget: "text" },
  { key: "slug", label: "Slug", widget: "text" },
  { key: "draft", label: "Draft", widget: "checkbox" },
];

export function getFieldsForCollection(collection: string | null): MetadataFieldDef[] {
  switch (collection) {
    case "blog":
      return BLOG_FIELDS;
    case "app-landings":
      return APP_LANDINGS_FIELDS;
    case "projects":
      return PROJECTS_FIELDS;
    case "docs":
      return DOCS_FIELDS;
    default:
      return [
        { key: "title", label: "Title", widget: "text" },
        { key: "description", label: "Description", widget: "textarea" },
        { key: "date", label: "Date", widget: "datetime" },
        { key: "draft", label: "Draft", widget: "checkbox" },
        { key: "tags", label: "Tags", widget: "string-array" },
      ];
  }
}
