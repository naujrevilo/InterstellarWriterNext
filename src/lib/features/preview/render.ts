import { Marked } from "marked";

const marked = new Marked();

const SAFE_HTML_TAGS = new Set([
  "a", "article", "aside", "b", "blockquote", "br", "caption", "code",
  "del", "details", "div", "em", "figcaption", "figure", "h1", "h2",
  "h3", "h4", "h5", "h6", "hr", "i", "img", "kbd", "li", "main",
  "mark", "ol", "p", "pre", "q", "s", "section", "small", "span",
  "strong", "sub", "summary", "sup", "table", "tbody", "td", "tfoot",
  "th", "thead", "tr", "u", "ul",
]);
const SAFE_HTML_ATTRIBUTES = new Set([
  "alt", "class", "colspan", "decoding", "height", "id", "loading", "rowspan",
  "scope", "target", "title", "width",
]);
const SAFE_SVG_TAGS = new Set([
  "svg", "g", "path", "rect", "circle", "ellipse", "line", "polyline", "polygon",
  "text", "tspan", "defs", "marker", "style", "use", "symbol", "clippath",
]);

function isSafeUrl(value: string, allowDataImage = false): boolean {
  const normalized = value.replace(/[\u0000-\u0020]+/g, "").toLowerCase();
  return (
    normalized.startsWith("https://") ||
    normalized.startsWith("http://") ||
    normalized.startsWith("mailto:") ||
    normalized.startsWith("#") ||
    normalized.startsWith("/") ||
    normalized.startsWith("./") ||
    normalized.startsWith("../") ||
    (allowDataImage && /^data:image\/(png|gif|jpe?g|webp|avif|svg\+xml);/i.test(value))
  );
}

/** Sanitize untrusted markdown HTML before it reaches Svelte's {@html}. */
export function sanitizeMarkdownHtml(html: string): string {
  const document = new DOMParser().parseFromString(html, "text/html");
  for (const element of Array.from(document.body.querySelectorAll("*"))) {
    const tag = element.tagName.toLowerCase();
    if (!SAFE_HTML_TAGS.has(tag)) {
      element.remove();
      continue;
    }
    for (const attribute of Array.from(element.attributes)) {
      const name = attribute.name.toLowerCase();
      const isUrl = name === "href" || name === "src";
      if (
        name.startsWith("on") ||
        name === "style" ||
        (!SAFE_HTML_ATTRIBUTES.has(name) && !isUrl) ||
        (isUrl && !isSafeUrl(attribute.value, name === "src"))
      ) {
        element.removeAttribute(attribute.name);
      }
    }
    if (tag === "a" && element.getAttribute("target") === "_blank") {
      element.setAttribute("rel", "noopener noreferrer");
    }
  }
  return document.body.innerHTML;
}

/** Keep Mermaid's safe SVG drawing vocabulary, but remove scriptable hooks. */
export function sanitizeMermaidSvg(svg: string): string {
  const document = new DOMParser().parseFromString(svg, "text/html");
  for (const element of Array.from(document.body.querySelectorAll("*"))) {
    const tag = element.tagName.toLowerCase();
    if (!SAFE_SVG_TAGS.has(tag)) {
      element.remove();
      continue;
    }
    for (const attribute of Array.from(element.attributes)) {
      const name = attribute.name.toLowerCase();
      const value = attribute.value.trim();
      const isLink = name === "href" || name === "xlink:href";
      if (
        name.startsWith("on") ||
        (isLink && !value.startsWith("#")) ||
        (name === "style" && /url\s*\(\s*(?!#)/i.test(value))
      ) {
        element.removeAttribute(attribute.name);
      }
    }
  }
  return document.body.innerHTML;
}

/**
 * Render markdown to HTML.
 * Handles basic markdown syntax.
 */
export function renderMarkdown(source: string): string {
  try {
    return sanitizeMarkdownHtml(marked.parse(source, { async: false }) as string);
  } catch {
    return `<p>${escapeHtml(source)}</p>`;
  }
}

/**
 * Escape HTML special characters.
 */
export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

/**
 * Extract mermaid code blocks from markdown source.
 * Returns an array of { id, code } objects.
 */
export function extractMermaidBlocks(source: string): Array<{
  id: string;
  code: string;
}> {
  const blocks: Array<{ id: string; code: string }> = [];
  const regex = /```mermaid\n([\s\S]*?)```/g;
  let match;
  let index = 0;

  while ((match = regex.exec(source)) !== null) {
    blocks.push({
      id: `mermaid-${index}`,
      code: match[1].trim(),
    });
    index++;
  }

  return blocks;
}

/**
 * Replace mermaid blocks in markdown with placeholder divs.
 */
export function replaceMermaidBlocks(
  source: string,
  rendered: Map<string, string>
): string {
  const regex = /```mermaid\n([\s\S]*?)```/g;
  let index = 0;

  return source.replace(regex, (_, code) => {
    const id = `mermaid-${index}`;
    const html = rendered.get(id);
    index++;

    if (html) {
      return `<div class="mermaid-rendered" data-mermaid-id="${id}">${html}</div>`;
    }

    return `<div class="mermaid-placeholder">
      <span>Mermaid diagram</span>
      <code>${escapeHtml(code.trim())}</code>
    </div>`;
  });
}

/**
 * Pretty up fenced code blocks emitted by marked.
 *
 * `marked` (after our sanitizer) outputs:
 *   <pre><code class="language-rust">fn main() ...</code></pre>
 *
 * We wrap each block in a shell that adds:
 *   - A header showing the language label (or "text" when missing)
 *   - A copy button that copies the raw code to the clipboard
 *
 * The actual code is stored on the button as `data-code-b64` (base64
 * of the UTF-8 bytes), so we never have to worry about attribute
 * escaping — the click handler in Preview.svelte reads it back with
 * atob() and writes the decoded string to the clipboard.
 *
 * IMPORTANT: this runs AFTER the sanitizer. The wrapper HTML is
 * generated by us (not user input), so it's safe to add the
 * `<button>` and `data-*` attributes that the sanitizer would
 * otherwise strip.
 */
export function enhanceCodeBlocks(html: string): string {
  return html.replace(
    /<pre><code(?:\s+class="language-([a-zA-Z0-9_+\-.]+)")?\s*>([\s\S]*?)<\/code><\/pre>/g,
    (_match, lang: string | undefined, escapedCode: string) => {
      const langLabel = (lang || "text").toLowerCase();
      // marked outputs HTML-escaped code. Decode once so the data
      // attribute holds the original (re-typed) source.
      const decoded = decodeHtmlEntities(escapedCode);
      const b64 = base64EncodeUtf8(decoded);
      return (
        `<div class="code-block" data-lang="${escapeHtml(langLabel)}">` +
        `<div class="code-block-header">` +
        `<span class="code-block-lang">${escapeHtml(langLabel)}</span>` +
        `<button type="button" class="code-block-copy" data-code-b64="${b64}" aria-label="Copy code">` +
        `<svg class="code-block-copy-icon" width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">` +
        `<rect x="5" y="5" width="9" height="9" rx="1.4" stroke="currentColor" stroke-width="1.3"/>` +
        `<path d="M3 11V3.5A1.5 1.5 0 0 1 4.5 2H11" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>` +
        `</svg>` +
        `<svg class="code-block-check-icon" width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">` +
        `<path d="M3.5 8.5L6.5 11.5L12.5 4.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>` +
        `</svg>` +
        `<span class="code-block-copy-label">Copy</span>` +
        `</button>` +
        `</div>` +
        `<pre class="code-block-pre"><code class="language-${escapeHtml(langLabel)}">${escapedCode}</code></pre>` +
        `</div>`
      );
    }
  );
}

function decodeHtmlEntities(s: string): string {
  return s
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#039;/g, "'")
    .replace(/&#x27;/g, "'");
}

function base64EncodeUtf8(s: string): string {
  const bytes = new TextEncoder().encode(s);
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

/**
 * Sanitize MDX/JSX components to safe placeholders.
 * Does NOT execute components — renders them as visible placeholders.
 */
export function sanitizeMdxComponents(html: string): string {
  // Self-closing components: <Component prop="value" />
  html = html.replace(
    /<([A-Z][a-zA-Z0-9]*)([^>]*?)\/>/g,
    (_, name, attrs) => {
      const directive = extractDirectives(attrs);
      return `<div class="mdx-placeholder">
        <span class="mdx-name">Component: ${escapeHtml(name)}</span>
        ${directive ? `<span class="mdx-directive">${escapeHtml(directive)}</span>` : ""}
      </div>`;
    }
  );

  // Opening/closing components: <Component>...</Component>
  html = html.replace(
    /<([A-Z][a-zA-Z0-9]*)([^>]*)>([\s\S]*?)<\/\1>/g,
    (_, name, attrs, children) => {
      const directive = extractDirectives(attrs);
      return `<div class="mdx-placeholder">
        <span class="mdx-name">Component: ${escapeHtml(name)}</span>
        ${directive ? `<span class="mdx-directive">${escapeHtml(directive)}</span>` : ""}
        ${children.trim() ? `<span class="mdx-children">${escapeHtml(children.trim())}</span>` : ""}
      </div>`;
    }
  );

  return html;
}

/**
 * Extract Astro directives (client:*, transition:*, etc.) from attribute string.
 */
function extractDirectives(attrs: string): string | null {
  const directives = attrs.match(
    /(?:client|transition|animate|class:list)(?::[a-zA-Z]+)?="[^"]*"/g
  );
  return directives?.join(" ") ?? null;
}
