/**
 * Astro-aware pre-processing for the preview.
 *
 * The preview is plain markdown rendered with `marked` (no JSX runtime,
 * no Astro components). This module transforms the source body so the
 * markdown renderer sees a stream it can handle, while preserving the
 * Astro semantics the user wrote:
 *
 *   1. Top-level `import` statements are stripped. They are metadata,
 *      not content. Astro's runtime would handle them; the preview
 *      ignores them.
 *
 *   2. `<Image />` / `<Picture />` (Astro built-ins) are normalized
 *      to plain `<img />` so `marked` passes them through as inline
 *      HTML. Astro directives like `client:load` are dropped (they'd
 *      be no-ops in the preview anyway).
 *
 *   3. Local image paths (markdown + JSX) are resolved to `data:`
 *      URLs via the Rust side (`resolve_image` command) and the URLs
 *      in the body are replaced in place. The webview cannot read the
 *      filesystem directly, so we have to inline the bytes.
 *
 * This is the closest we can get to "Astro preview" without running
 * the actual Astro build. It covers the common case of:
 *
 *   import { Image } from "astro:assets";
 *   import cover from "../assets/cover.png";
 *
 *   <Image src={cover} alt="..." />
 *   <Image src="../assets/cover.png" alt="..." />
 *   ![Alt text](../assets/cover.png)
 *
 * — and renders all three correctly.
 */

import { resolveImage } from "../../api/tauri";

/* -------------------------------------------------------------------------- */
/* Small utilities                                                             */
/* -------------------------------------------------------------------------- */

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

/* -------------------------------------------------------------------------- */
/* Step 1: strip imports                                                       */
/* -------------------------------------------------------------------------- */

/**
 * Remove top-level `import` statements (multi-line, with or without
 * semicolons). Conservative: only matches a line that starts with
 * `import` and ends with a closing quote + optional `;`.
 */
export function stripImports(source: string): string {
  // import x from "y";
  // import { a, b } from "y";
  // import * as x from "y";
  // import "y";
  // import x from "y"; // with a trailing comment
  return source.replace(
    /^[ \t]*import(?:[ \t]+(?:[^"'`;]+[ \t]+from[ \t]+)?)?[ \t]*(?:["'`]([^"'`]+)["'`])(?:[ \t]*;)?[ \t]*(?:\r?\n|$)/gm,
    ""
  );
}

/* -------------------------------------------------------------------------- */
/* Step 2: normalize Astro components → plain HTML                            */
/* -------------------------------------------------------------------------- */

/**
 * Pull `src="..."` and `alt="..."` from a JSX-style attribute blob
 * (the bit between `<Image` and `/>` or `>`).
 */
function extractAttr(attrs: string, name: string): string | null {
  const re = new RegExp(`\\b${name}=["']([^"']*)["']`, "i");
  const m = re.exec(attrs);
  return m ? m[1] : null;
}

/**
 * Convert `<Image src="…" alt="…" width="…" … />` to
 * `<img src="…" alt="…" width="…" … />`. Strips Astro directives.
 */
export function normalizeImageComponent(source: string): string {
  // Self-closing: <Image ... />
  let out = source.replace(
    /<[Ii]mage\b([^>]*?)\/>/g,
    (_, attrs) => jsxImageToImg(attrs)
  );
  // With children: <Image ...>...</Image> — uncommon, but Astro supports it.
  out = out.replace(
    /<[Ii]mage\b([^>]*?)>([\s\S]*?)<\/[Ii]mage>/g,
    (_, attrs, children) => `${jsxImageToImg(attrs)}${children}`
  );
  return out;
}

/**
 * Convert `<Picture>` to a single `<img>` using its `src` attribute.
 * The full `<picture><source></picture>` flow needs build-time
 * optimization, but the preview just needs to show *something*.
 */
export function normalizePictureComponent(source: string): string {
  return source.replace(
    /<[Pp]icture\b([^>]*?)>([\s\S]*?)<\/[Pp]icture>/g,
    (_, attrs) => {
      const src = extractAttr(attrs, "src");
      return `<img src="${src ?? ""}" alt="" class="astro-picture" />`;
    }
  );
}

function jsxImageToImg(attrs: string): string {
  const src = extractAttr(attrs, "src") ?? "";
  const alt = extractAttr(attrs, "alt") ?? "";
  const width = extractAttr(attrs, "width");
  const height = extractAttr(attrs, "height");
  const cls = extractAttr(attrs, "class");

  const parts = [`src="${src}"`, `alt="${alt}"`];
  if (width) parts.push(`width="${width}"`);
  if (height) parts.push(`height="${height}"`);
  if (cls) parts.push(`class="${cls}"`);
  return `<img ${parts.join(" ")} />`;
}

/* -------------------------------------------------------------------------- */
/* Step 3: extract and resolve image paths                                    */
/* -------------------------------------------------------------------------- */

/** Every relative or absolute image reference in the body, in order. */
export function extractImagePaths(source: string): string[] {
  const paths = new Set<string>();
  const reList: RegExp[] = [
    // Markdown: ![alt](path) or ![alt](path "title")
    /!\[[^\]]*\]\(([^)\s]+)(?:\s+["'][^"']*["'])?\)/g,
    // <img src="…">  (already normalized by step 2 for Astro)
    /<img\b[^>]*?\bsrc=["']([^"']+)["'][^>]*>/g,
    // <source src="…"> inside <picture>
    /<source\b[^>]*?\bsrc=["']([^"']+)["'][^>]*>/g,
  ];

  for (const re of reList) {
    let m: RegExpExecArray | null;
    while ((m = re.exec(source)) !== null) {
      const p = m[1];
      if (!p) continue;
      // Skip obviously external / data URLs / non-files
      if (
        p.startsWith("data:") ||
        p.startsWith("http://") ||
        p.startsWith("https://") ||
        p.startsWith("//")
      ) {
        continue;
      }
      paths.add(p);
    }
  }

  return Array.from(paths);
}

/* -------------------------------------------------------------------------- */
/* Frontmatter cover / hero image                                            */
/* -------------------------------------------------------------------------- */

/**
 * Common frontmatter keys that hold the post's cover/hero image.
 * Checked in order; first match wins.
 */
const COVER_FM_KEYS = [
  "image",
  "cover",
  "thumbnail",
  "hero",
  "banner",
  "ogImage",
  "og_image",
  "socialImage",
  "social_image",
] as const;

/**
 * Pick the cover image path out of a frontmatter object, if any.
 * Returns the raw value as stored (relative or absolute path),
 * or null when the frontmatter has no cover-like field.
 */
export function extractCoverFromFrontmatter(
  fm: Record<string, unknown>
): string | null {
  if (!fm || typeof fm !== "object") return null;
  for (const key of COVER_FM_KEYS) {
    const v = (fm as Record<string, unknown>)[key];
    if (typeof v === "string" && v.trim().length > 0) return v;
  }
  return null;
}

/**
 * Build a `<figure class="post-hero">` block prepended to the body.
 * If the cover path is in `resolved` (i.e. we already fetched its
 * bytes), use the data URL. Otherwise the raw path goes in and the
 * browser will show a broken-image marker — that's intentional, we
 * don't want a half-baked preview when the file is missing.
 */
export function buildHeroBlock(
  fm: Record<string, unknown> | null | undefined,
  resolved: Map<string, string>
): string {
  const raw = extractCoverFromFrontmatter(fm ?? {});
  if (!raw) return "";

  // Skip obviously external / data URLs from the resolver side.
  const isExternal =
    raw.startsWith("data:") ||
    raw.startsWith("http://") ||
    raw.startsWith("https://") ||
    raw.startsWith("//");
  const src = (!isExternal && resolved.get(raw)) || raw;

  const title = typeof fm?.title === "string" ? fm.title : "";
  const alt = title || "Cover image";

  return (
    `<figure class="post-hero">` +
    `<img src="${src}" alt="${escapeHtml(alt)}" loading="eager" decoding="async" />` +
    `</figure>\n\n`
  );
}

/**
 * Walk the body (and the cover image in the frontmatter, if any),
 * ask the Rust side to resolve every local image path to a `data:`
 * URL, and return a map `original → dataUrl`. Failed resolutions
 * are omitted; the caller decides how to render those (we just
 * leave the path untouched so the broken-image marker shows).
 */
export async function resolveAllImages(
  root: string,
  body: string,
  fromFile: string,
  frontmatter?: Record<string, unknown> | null
): Promise<Map<string, string>> {
  const fromBody = extractImagePaths(body);
  const fromFm = extractCoverFromFrontmatter(frontmatter ?? {});
  const allPaths = fromFm ? Array.from(new Set([...fromBody, fromFm])) : fromBody;
  if (allPaths.length === 0) return new Map();

  const out = new Map<string, string>();
  // Resolve in parallel; small fanout is fine.
  await Promise.all(
    allPaths.map(async (p) => {
      try {
        const { dataUrl } = await resolveImage(fromFile, p);
        out.set(p, dataUrl);
      } catch {
        // leave it out
      }
    })
  );
  return out;
}

/** Replace every occurrence of `original` in `body` with `dataUrl`. */
export function applyResolvedImages(
  body: string,
  resolved: Map<string, string>
): string {
  if (resolved.size === 0) return body;
  let out = body;
  for (const [original, dataUrl] of resolved) {
    // Escape regex special chars in the original (paths are mostly
    // safe, but be defensive).
    const safe = original.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    out = out.replace(new RegExp(safe, "g"), dataUrl);
  }
  return out;
}

/* -------------------------------------------------------------------------- */
/* Public entry point                                                          */
/* -------------------------------------------------------------------------- */

/**
 * Full pre-processing pipeline. Async because image resolution
 * round-trips to the backend.
 *
 * Returns the body with:
 *   - imports stripped
 *   - `<Image>` / `<Picture>` normalized to `<img>`
 *   - local image paths replaced by `data:` URLs
 *   - a hero `<figure>` prepended when the frontmatter has a cover
 *
 * The result is safe to feed to `marked` — it'll render markdown
 * around the now-inlined HTML, and the inline HTML will pass through.
 */
export async function preprocessForPreview(
  root: string,
  body: string,
  fromFile: string,
  frontmatter?: Record<string, unknown> | null
): Promise<string> {
  let processed = body ?? "";
  processed = stripImports(processed);
  processed = normalizeImageComponent(processed);
  processed = normalizePictureComponent(processed);
  const resolved = await resolveAllImages(root, processed, fromFile, frontmatter);
  processed = applyResolvedImages(processed, resolved);

  // Hero block (only for .mdx, caller decides via the frontmatter arg).
  const hero = buildHeroBlock(frontmatter, resolved);
  if (hero) processed = hero + processed;

  return processed;
}
