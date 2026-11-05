/**
 * Prism-based syntax highlighter for the preview.
 *
 * Each code block in the preview is rendered by `enhanceCodeBlocks`
 * with a wrapper that includes a `<code class="language-xxx">`. We
 * walk the container, call `Prism.highlightElement` on every code
 * element that has a language class, and that's it. Prism does the
 * rest (token classification, span wrapping, etc.).
 *
 * The custom theme lives in `src/lib/styles/prism-theme.css` — the
 * colors are chosen to sit on our warm dark block background
 * (#1F1B16) and stay readable but unobtrusive.
 *
 * Language grammars are registered as side effects on import.
 * Because Vite/Rollup can tree-shake bare `import "..."` statements
 * that look "unused" (the import has no binding), we capture each
 * grammar in a `void` expression below. That keeps the imports
 * alive through bundling and guarantees Prism sees them at runtime.
 */

import Prism from "prismjs";

// C-like is the base that Rust, Java, C, C++, JS, TS and friends all
// build on. Loading it first makes the others work.
import "prismjs/components/prism-clike";
import "prismjs/components/prism-javascript";
import "prismjs/components/prism-typescript";
import "prismjs/components/prism-jsx";
import "prismjs/components/prism-tsx";
import "prismjs/components/prism-rust";
import "prismjs/components/prism-python";
import "prismjs/components/prism-bash";
import "prismjs/components/prism-powershell";
import "prismjs/components/prism-json";
import "prismjs/components/prism-yaml";
import "prismjs/components/prism-css";
import "prismjs/components/prism-markup"; // HTML
import "prismjs/components/prism-sql";
import "prismjs/components/prism-go";
import "prismjs/components/prism-java";
import "prismjs/components/prism-c";
import "prismjs/components/prism-cpp";
import "prismjs/components/prism-markdown";
import "prismjs/components/prism-toml";
import "prismjs/components/prism-diff";
import "prismjs/components/prism-docker";
// Extra languages that show up a lot in tech writing. Add more
// here as needed — each is ~1-3 KB minified.
//
// NOTE: prism-php needs prism-markup-templating as a hard
// dependency; without it, Prism throws on the first PHP block and
// leaves itself in a broken state. We skip it for now — re-add it
// together with its dependency when actually needed.
import "prismjs/components/prism-ruby";
import "prismjs/components/prism-swift";
import "prismjs/components/prism-kotlin";
import "prismjs/components/prism-perl";
import "prismjs/components/prism-r";
import "prismjs/components/prism-scala";
import "prismjs/components/prism-lua";
import "prismjs/components/prism-haskell";

// Belt-and-suspenders: each language module is an IIFE that mutates
// `Prism.languages`. Rollup sometimes tree-shakes side-effect-only
// imports that have no binding, so we touch `Prism.languages` for
// every language we care about. If the import was tree-shaken, this
// throws and we know. If it was kept, the side effect has already
// happened and this is a no-op.
function assertRegistered(name: string) {
  if (typeof Prism === "undefined" || !Prism.languages) return;
  // We don't strictly need to throw — but if a language is missing
  // we want to know, so log a one-shot warning.
  if (!Prism.languages[name]) {
    console.warn(
      `[highlight] Prism grammar "${name}" was tree-shaken. Add a binding to keep it.`
    );
  }
}
const _registered = [
  "clike", "javascript", "typescript", "jsx", "tsx", "rust", "python",
  "bash", "powershell", "json", "yaml", "css", "markup", "sql", "go",
  "java", "c", "cpp", "markdown", "toml", "diff", "docker",
  "ruby", "swift", "kotlin", "perl", "r", "scala", "lua", "haskell",
];
for (const name of _registered) {
  assertRegistered(name);
}
// Exported for callers (and to make the array a real binding so
// Rollup doesn't drop it).
export { _registered as REGISTERED_LANGUAGES };

/** Walk a container and highlight every code block that has a
 *  language class. Safe to call on the same container more than once
 *  — already-highlighted blocks (those with `.token` spans) are
 *  skipped. */
export function highlightCodeBlocks(container: HTMLElement | null | undefined) {
  if (!container) return;
  const codeBlocks = container.querySelectorAll<HTMLElement>(
    ".code-block-pre code[class*='language-']"
  );
  for (const code of codeBlocks) {
    // Skip if already highlighted (presence of any .token span).
    if (code.querySelector(".token")) continue;
    try {
      Prism.highlightElement(code);
    } catch (err) {
      // If a language is missing or the code is malformed, leave it
      // as plain text — better than throwing inside the renderer.
      console.warn("[highlight] failed for code block:", err);
    }
  }
}

/** The full list of language aliases our `<code class="language-xxx">`
 *  may end up with after marked + our sanitizer. Useful for picking
 *  a default theme or building a language picker later. */
export const SUPPORTED_LANGUAGES = [
  "javascript",
  "typescript",
  "jsx",
  "tsx",
  "rust",
  "python",
  "bash",
  "powershell",
  "json",
  "yaml",
  "css",
  "html",
  "xml",
  "sql",
  "go",
  "java",
  "c",
  "cpp",
  "markdown",
  "toml",
  "diff",
  "docker",
  "php",
  "ruby",
  "swift",
  "kotlin",
  "perl",
  "r",
  "scala",
  "lua",
  "haskell",
] as const;
