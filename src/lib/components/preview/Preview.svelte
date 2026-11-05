<script lang="ts">
  import { onMount, tick } from "svelte";
  import {
    renderMarkdown,
    extractMermaidBlocks,
    sanitizeMermaidSvg,
    sanitizeMdxComponents,
    enhanceCodeBlocks,
  } from "../../features/preview/render";
  import {
    preprocessForPreview,
  } from "../../features/preview/astroAwareness";
  import { highlightCodeBlocks } from "../../features/preview/highlight";
  import { lintBody, type LintIssue } from "../../features/preview/linter";
  import { workspaceStore } from "../../stores/workspace";
  import { documentStore } from "../../stores/document";
  import type { DocumentKind } from "../../types/workspace";

  let {
    body = "",
    kind = "markdown",
  }: {
    body?: string;
    kind?: DocumentKind;
  } = $props();

  let mermaidModule: any = $state(null);
  let mermaidReady = $state(false);
  let mermaidError = $state<string | null>(null);
  let renderedBlocks = $state(new Map<string, string>());
  let containerEl: HTMLDivElement | undefined = $state();

  // Pre-processed body (Astro-aware) — imports stripped, Image/Picture
  // normalized, local image paths inlined as data: URLs.
  let preprocessedBody = $state("");
  let preprocessError = $state<string | null>(null);

  /**
   * Lint issues for the current body. Re-derived on every
   * preprocessedBody change so the banner updates as the user types.
   * We lint the preprocessed body so the line numbers match what
   * the user sees (imports stripped, Image normalized, etc.).
   */
  let lintIssues: LintIssue[] = $derived(lintBody(preprocessedBody));

  let { root } = $derived($workspaceStore);
  let { relativePath, frontmatter, kind: docKind } = $derived($documentStore);

  // Try to initialize mermaid once. If it fails (offline, blocked CSP,
  // bundle too large), the preview still works — mermaid blocks just
  // become a styled placeholder.
  onMount(async () => {
    try {
      const mod = await import("mermaid");
      const mermaid = mod.default;
      mermaid.initialize({
        startOnLoad: false,
        theme: "neutral",
        securityLevel: "loose",
        fontFamily: "'Inter', system-ui, sans-serif",
      });
      mermaidModule = mermaid;
      mermaidReady = true;
    } catch (e: any) {
      console.warn("[Preview] Mermaid not available:", e?.message ?? e);
      mermaidError = e?.message ?? "Mermaid could not be loaded";
      mermaidReady = false;
    }
  });

  // Run the Astro-aware pre-processor whenever the body, frontmatter,
  // workspace root, or current file changes. We only do this when the
  // workspace is open — otherwise we just pass the body through.
  // We pass the frontmatter so the cover/hero image (frontmatter.image)
  // is included in the resolver pass and the hero block can be built.
  // Hero is only injected for .mdx (Astro) — generic markdown posts
  // don't follow that convention.
  $effect(() => {
    if (!body || !relativePath || !root) {
      preprocessedBody = body ?? "";
      return;
    }
    const fm = docKind === "mdx" ? frontmatter : null;
    let cancelled = false;
    preprocessForPreview(root, body, relativePath, fm)
      .then((processed) => {
        if (cancelled) return;
        preprocessedBody = processed;
        preprocessError = null;
      })
      .catch((e) => {
        if (cancelled) return;
        console.warn("[Preview] Astro preprocess failed:", e);
        // Fall back to raw body — preview still works, just without
        // resolved images / stripped imports.
        preprocessedBody = body;
        preprocessError = e?.message ?? String(e);
      });
    return () => {
      cancelled = true;
    };
  });

  // Render mermaid blocks when the *preprocessed* body or mermaid
  // availability changes.
  $effect(() => {
    if (mermaidReady && mermaidModule && preprocessedBody) {
      renderMermaidBlocks(preprocessedBody);
    }
  });

  async function renderMermaidBlocks(source: string) {
    const blocks = extractMermaidBlocks(source);
    if (blocks.length === 0) {
      renderedBlocks = new Map();
      return;
    }
    const next = new Map<string, string>();
    for (const block of blocks) {
      try {
        const { svg } = await mermaidModule.render(
          `mermaid-${block.id}-${Date.now()}`,
          block.code
        );
        next.set(block.id, sanitizeMermaidSvg(svg));
      } catch (err) {
        console.warn("[Preview] Mermaid render failed for", block.id, err);
        // Leave this block out of `next` so the placeholder is shown.
      }
    }
    renderedBlocks = next;
  }

  /**
   * Compute the final HTML.
   * Order matters:
   *   1. The body has already been pre-processed for Astro awareness
   *      (imports stripped, Image/Picture normalized, images inlined).
   *   2. Replace mermaid blocks with markers (or rendered SVGs).
   *   3. Render the (now mermaid-free) markdown as HTML.
   *   4. Substitute markers back with their rendered SVG or placeholder.
   *   5. Any remaining unknown <Component> tags become placeholders.
   */
  let finalHtml = $derived.by(() => {
    let processed = preprocessedBody ?? "";
    const blocks = extractMermaidBlocks(processed);
    const markers: Map<string, string> = new Map();

    for (const block of blocks) {
      const marker = `<!--MMAID-${block.id}-${block.id}-->`;
      const svg = renderedBlocks.get(block.id);
      if (svg) {
        markers.set(marker, `<div class="mermaid-rendered">${svg}</div>`);
      } else {
        markers.set(
          marker,
          `<div class="mermaid-placeholder">
            <div class="mermaid-placeholder-header">
              <span class="mermaid-placeholder-icon">◇</span>
              <span>Mermaid diagram</span>
              ${mermaidError ? '<span class="mermaid-placeholder-hint">(renderer unavailable)</span>' : ""}
            </div>
            <pre class="mermaid-placeholder-code"><code>${escapeForPlaceholder(block.code)}</code></pre>
          </div>`
        );
      }
      processed = processed.replace(/```mermaid\n[\s\S]*?```/, marker);
    }

    let html = renderMarkdown(processed);

    for (const [marker, replacement] of markers) {
      // marked may wrap the marker in a <p> — strip that too
      html = html.replace(`<p>${marker}</p>`, replacement);
      html = html.replace(marker, replacement);
    }

    // Wrap each remaining <pre><code> in a shell with a language
    // label and a copy button. Runs AFTER the sanitizer so the
    // chrome (button + data-*) is preserved.
    html = enhanceCodeBlocks(html);

    // Astro built-ins (Image, Picture) are already normalized to <img>
    // by the pre-processor. Any other PascalCase component that survived
    // — <Layout>, <MyComponent>, etc. — becomes a styled placeholder so
    // the preview shows structure instead of raw HTML noise.
    if (kind === "mdx") {
      html = sanitizeMdxComponents(html);
    }

    return html;
  });

  /**
   * After the preview HTML updates, run Prism over every code block
   * to tokenize it. We `await tick()` so Svelte's `{@html}` has
   * actually flushed the new DOM into `containerEl` before we
   * query for code blocks; without this the highlight can run on
   * the previous render's DOM and miss the new blocks.
   *
   * `highlightCodeBlocks` is idempotent: blocks that already contain
   * `.token` spans are skipped, so re-renders are safe and cheap.
   */
  $effect(() => {
    // Touch `finalHtml` so the effect re-runs when the preview
    // content changes (typing, switching files, etc.).
    void finalHtml;
    tick().then(() => {
      if (containerEl) highlightCodeBlocks(containerEl);
    });
  });

  /**
   * Wire a delegated click handler on the preview container once it
   * mounts. We use event delegation so we don't have to attach
   * `onclick` to each block (the HTML is generated by
   * `enhanceCodeBlocks` and rendered via `{@html}`, so per-button
   * handlers aren't possible).
   */
  onMount(() => {
    const el = containerEl;
    if (!el) return;
    const handler = (e: Event) => {
      const target = e.target as HTMLElement | null;
      if (!target) return;
      const btn = target.closest<HTMLButtonElement>(".code-block-copy");
      if (!btn) return;
      const b64 = btn.getAttribute("data-code-b64");
      if (!b64) return;

      try {
        const code = base64DecodeUtf8(b64);
        const label = btn.querySelector(".code-block-copy-label");
        const restore = () => {
          btn.classList.remove("copied");
          if (label) label.textContent = "Copy";
        };
        navigator.clipboard
          .writeText(code)
          .then(() => {
            btn.classList.add("copied");
            if (label) label.textContent = "Copied";
            setTimeout(restore, 1500);
          })
          .catch((err) => {
            console.warn("[Preview] Copy failed:", err);
            // Best-effort fallback: still flash the button so the user
            // knows something happened, even if the clipboard write
            // was blocked (e.g. insecure context).
            btn.classList.add("copied");
            if (label) label.textContent = "Copy failed";
            setTimeout(restore, 1500);
          });
      } catch (err) {
        console.warn("[Preview] Copy decode failed:", err);
      }
    };
    el.addEventListener("click", handler);
    return () => el.removeEventListener("click", handler);
  });

  function base64DecodeUtf8(b64: string): string {
    const binary = atob(b64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return new TextDecoder().decode(bytes);
  }

  function escapeForPlaceholder(text: string): string {
    return text
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }
</script>

<div class="preview" bind:this={containerEl}>
  <div class="preview-content editor-prose">
    {#if body && body.trim().length > 0}
      {#if lintIssues.length > 0}
        <aside class="lint-banner" aria-label="MDX lint issues">
          <header class="lint-banner-head">
            <span class="lint-banner-icon" aria-hidden="true">⚠</span>
            <span class="lint-banner-title">
              {lintIssues.length}
              {lintIssues.length === 1 ? "issue" : "issues"} found — the build may fail.
            </span>
          </header>
          <ul class="lint-banner-list">
            {#each lintIssues as issue (issue.line + ":" + issue.column + ":" + issue.rule)}
              <li class="lint-banner-item lint-banner-item-{issue.severity}">
                <span class="lint-banner-loc">L{issue.line}:{issue.column}</span>
                <span class="lint-banner-msg">{issue.message}</span>
                <span class="lint-banner-rule">{issue.rule}</span>
              </li>
            {/each}
          </ul>
          <p class="lint-banner-hint">
            The preview is permissive by design; these are the patterns that break the MDX parser at build time.
          </p>
        </aside>
      {/if}
      {@html finalHtml}
    {:else}
      <div class="preview-empty">
        <p>Nothing to preview yet.</p>
        <p class="preview-empty-hint">Start writing in the editor to see the rendered output here.</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .preview {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .preview-content {
    flex: 1;
    overflow-y: auto;
    padding: 6vh 24px 12vh;
    background: var(--color-paper);
    user-select: text;
  }

  .preview-empty {
    text-align: center;
    color: var(--color-muted);
    font-family: var(--font-editor);
    padding: 60px 24px;
  }

  .preview-empty p {
    margin: 0 0 8px;
    font-size: 15px;
  }

  .preview-empty-hint {
    font-style: italic;
    font-size: 13px;
    opacity: 0.7;
  }

  /* ----- Lint banner (MDX/JSX issues that may break the build) ----- */
  .preview-content :global(.lint-banner) {
    margin: 0 0 1.5rem;
    padding: 12px 14px 10px;
    background: rgba(245, 158, 11, 0.08);
    border: 1px solid rgba(245, 158, 11, 0.45);
    border-left: 3px solid #B45309;
    border-radius: 6px;
    font-family: var(--font-ui);
    color: #78350F;
  }

  .preview-content :global(.lint-banner-head) {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }

  .preview-content :global(.lint-banner-icon) {
    font-size: 16px;
    color: #B45309;
  }

  .preview-content :global(.lint-banner-title) {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  .preview-content :global(.lint-banner-list) {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .preview-content :global(.lint-banner-item) {
    display: grid;
    grid-template-columns: 64px 1fr auto;
    gap: 10px;
    align-items: baseline;
    padding: 4px 0;
    font-size: 11.5px;
    line-height: 1.45;
    border-top: 1px solid rgba(245, 158, 11, 0.2);
  }

  .preview-content :global(.lint-banner-item:first-child) {
    border-top: none;
  }

  .preview-content :global(.lint-banner-loc) {
    font-family: var(--font-mono);
    font-weight: 600;
    color: #B45309;
  }

  .preview-content :global(.lint-banner-msg) {
    color: #422006;
  }

  .preview-content :global(.lint-banner-rule) {
    font-family: var(--font-mono);
    font-size: 10px;
    color: #92400E;
    background: rgba(245, 158, 11, 0.15);
    padding: 1px 6px;
    border-radius: 3px;
  }

  .preview-content :global(.lint-banner-item-error .lint-banner-rule) {
    background: rgba(220, 38, 38, 0.12);
    color: #7F1D1D;
  }

  .preview-content :global(.lint-banner-hint) {
    margin: 8px 0 0;
    font-size: 10.5px;
    color: #92400E;
    font-style: italic;
    opacity: 0.85;
  }

  /* ----- Typography (scoped to the preview so global CSS can't override) ----- */
  .preview-content :global(.editor-prose),
  .preview-content :global(h1),
  .preview-content :global(h2),
  .preview-content :global(h3),
  .preview-content :global(h4) {
    font-family: var(--font-editor);
    color: var(--color-ink);
  }

  .preview-content :global(h1) {
    font-size: 2.25rem;
    font-weight: 700;
    line-height: 1.15;
    margin: 2.5rem 0 1rem;
    letter-spacing: -0.025em;
  }

  .preview-content :global(h2) {
    font-size: 1.75rem;
    font-weight: 700;
    line-height: 1.25;
    margin: 2.25rem 0 0.75rem;
    letter-spacing: -0.02em;
    border-bottom: 1px solid var(--color-rule);
    padding-bottom: 0.4rem;
  }

  .preview-content :global(h3) {
    font-size: 1.4rem;
    font-weight: 600;
    line-height: 1.3;
    margin: 1.75rem 0 0.5rem;
    letter-spacing: -0.015em;
  }

  .preview-content :global(h4) {
    font-size: 1.15rem;
    font-weight: 600;
    line-height: 1.4;
    margin: 1.5rem 0 0.5rem;
  }

  .preview-content :global(p) {
    margin: 0 0 1rem;
    line-height: 1.75;
  }

  .preview-content :global(ul),
  .preview-content :global(ol) {
    margin: 0 0 1rem;
    padding-left: 1.75rem;
    line-height: 1.75;
  }

  /* Tailwind's preflight resets list-style to "none" on every ol/ul.
     Re-apply the browser default so the preview actually shows
     bullets (•) and numbers (1., 2., 3., …). */
  .preview-content :global(ul) {
    list-style: disc;
  }
  .preview-content :global(ol) {
    list-style: decimal;
  }
  .preview-content :global(li) {
    list-style: inherit;
  }

  .preview-content :global(li) {
    margin-bottom: 0.25rem;
  }

  .preview-content :global(strong) {
    font-weight: 700;
  }

  .preview-content :global(em) {
    font-style: italic;
  }

  .preview-content :global(a) {
    color: var(--color-accent);
    text-decoration: underline;
    text-underline-offset: 3px;
    text-decoration-thickness: 1px;
  }

  .preview-content :global(a):hover {
    color: #5b21b6;
  }

  .preview-content :global(blockquote) {
    border-left: 3px solid var(--color-accent);
    margin: 1.25rem 0;
    padding: 0.25rem 0 0.25rem 1.25rem;
    color: var(--color-ink-soft);
    font-style: italic;
    background: var(--color-chrome);
    border-radius: 0 6px 6px 0;
  }

  .preview-content :global(code) {
    font-family: var(--font-mono);
    font-size: 0.875em;
    background: var(--color-chrome);
    padding: 0.1em 0.35em;
    border-radius: 4px;
    color: var(--color-ink);
  }

  /* ----- Code blocks -----
     The wrapper (.code-block) holds the chrome (header + copy button).
     The inner <pre> is transparent so the wrapper's rounded corners
     and background are visible. The styles below apply to both
     wrapped and unwrapped <pre> elements so the legacy look still
     works for raw HTML or stylesheet-only previews. */
  .preview-content :global(pre) {
    font-family: var(--font-mono);
    font-size: 0.85em;
    line-height: 1.6;
    background: #1F1B16;
    color: #F5F0EA;
    padding: 1rem 1.25rem;
    border-radius: 6px;
    overflow-x: auto;
    margin: 1rem 0;
  }

  .preview-content :global(pre code) {
    background: none;
    padding: 0;
    color: inherit;
    font-size: inherit;
  }

  /* Wrapped code block: header + pre in a single rounded card. */
  .preview-content :global(.code-block) {
    margin: 1.25rem 0;
    border-radius: 8px;
    overflow: hidden;
    background: #1F1B16;
    border: 1px solid rgba(255, 255, 255, 0.06);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
  }

  .preview-content :global(.code-block-header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 5px 10px 5px 14px;
    background: rgba(255, 255, 255, 0.04);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .preview-content :global(.code-block-lang) {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 600;
    color: rgba(245, 240, 234, 0.55);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .preview-content :global(.code-block-copy) {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: transparent;
    border: none;
    color: rgba(245, 240, 234, 0.5);
    cursor: pointer;
    font-family: var(--font-ui);
    font-size: 11px;
    font-weight: 500;
    padding: 3px 7px;
    border-radius: 4px;
    transition: color 0.12s, background 0.12s;
  }

  .preview-content :global(.code-block-copy:hover) {
    color: rgba(245, 240, 234, 0.9);
    background: rgba(255, 255, 255, 0.07);
  }

  .preview-content :global(.code-block-check-icon) {
    display: none;
  }

  .preview-content :global(.code-block-copy.copied) {
    color: #4ADE80;
  }

  .preview-content :global(.code-block-copy.copied .code-block-copy-icon) {
    display: none;
  }

  .preview-content :global(.code-block-copy.copied .code-block-check-icon) {
    display: inline-block;
  }

  /* Inner <pre> is reset so the wrapper's chrome shows through. */
  .preview-content :global(.code-block .code-block-pre) {
    margin: 0;
    background: transparent;
    border: none;
    border-radius: 0;
    padding: 1rem 1.25rem;
  }

  .preview-content :global(hr) {
    border: none;
    border-top: 1px solid var(--color-rule);
    margin: 2rem 0;
  }

  .preview-content :global(img) {
    max-width: 100%;
    height: auto;
    border-radius: 6px;
  }

  /* Hero image auto-injected when frontmatter has a cover-like field. */
  .preview-content :global(.post-hero) {
    margin: -2rem 0 2.75rem;
    text-align: center;
  }

  .preview-content :global(.post-hero img) {
    max-width: 100%;
    max-height: 60vh;
    height: auto;
    border-radius: 10px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04), 0 12px 32px rgba(0, 0, 0, 0.08);
  }

  .preview-content :global(table) {
    width: 100%;
    border-collapse: collapse;
    margin: 1rem 0;
    font-size: 0.9em;
  }

  .preview-content :global(th),
  .preview-content :global(td) {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--color-rule);
    text-align: left;
  }

  .preview-content :global(th) {
    background: var(--color-chrome);
    font-weight: 600;
  }

  /* Mermaid */
  :global(.mermaid-rendered) {
    display: flex;
    justify-content: center;
    margin: 24px 0;
    background: white;
    border: 1px solid var(--color-rule);
    border-radius: 8px;
    padding: 16px;
  }

  :global(.mermaid-rendered svg) {
    max-width: 100%;
    height: auto;
  }

  :global(.mermaid-placeholder) {
    margin: 24px 0;
    background: linear-gradient(180deg, white 0%, var(--color-chrome) 100%);
    border: 1px dashed var(--color-rule-strong);
    border-radius: 8px;
    padding: 14px 18px;
  }

  :global(.mermaid-placeholder-header) {
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: var(--font-ui);
    font-size: 11px;
    font-weight: 600;
    color: var(--color-ink-soft);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    margin-bottom: 10px;
  }

  :global(.mermaid-placeholder-icon) {
    color: var(--color-accent);
    font-size: 14px;
  }

  :global(.mermaid-placeholder-hint) {
    color: var(--color-muted);
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0;
    font-size: 11px;
    margin-left: auto;
  }

  :global(.mermaid-placeholder-code) {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.6;
    color: var(--color-ink);
    background: transparent;
    padding: 0;
    overflow-x: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }

  /* MDX placeholders */
  :global(.mdx-placeholder) {
    background: var(--color-accent-soft);
    border: 1px solid rgba(109, 40, 217, 0.3);
    border-radius: 6px;
    padding: 8px 12px;
    margin: 12px 0;
    font-family: var(--font-ui);
    font-size: 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  :global(.mdx-name) {
    font-weight: 600;
    color: var(--color-accent);
  }

  :global(.mdx-directive) {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-ink-soft);
    background: rgba(0, 0, 0, 0.05);
    padding: 1px 4px;
    border-radius: 2px;
  }

  :global(.mdx-children) {
    color: var(--color-ink-soft);
    font-style: italic;
  }
</style>
