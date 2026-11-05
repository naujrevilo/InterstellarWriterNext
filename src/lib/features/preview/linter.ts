/**
 * MDX body linter for the preview.
 *
 * The preview's sanitizer is deliberately permissive — it lets almost
 * anything through so users can iterate freely. The cost is that
 * common MDX/JSX mistakes (the kind that break the Astro build) sail
 * through and only surface in CI.
 *
 * This linter catches the patterns that bite us in practice. It runs
 * over the raw body (before markdown rendering) and returns a list
 * of issues the Preview can surface as a banner. It's intentionally
 * regex-based and small — the alternative is pulling in a full
 * MDX parser, which is heavy and would just duplicate what Astro
 * does at build time.
 *
 * Detected patterns:
 *   1. `attribute-with-no-name`  — `="..." "` after a valid attribute
 *      (the exact bug that broke the inventory post: a stray quote
 *      starting a phantom attribute).
 *   2. `duplicate-attribute`     — same attribute name twice on the
 *      same tag.
 *   3. `unclosed-jsx-tag`         — opening `<Foo ...>` without a
 *      matching `/>` or `</Foo>` on a later line (heuristic — false
 *      positives possible with multi-line tags).
 *   4. `absolute-asset-path`      — `<Image src="/abs/path/...">` or
 *      `src="C:\..."`. Asset paths must be relative; absolute paths
 *      don't ship in the bundle.
 *
 * Each issue carries a `line` and `column` (1-based) so the editor
 * could scroll to the offending spot later. For now we just show
 * them in a banner — scroll-on-click is a future improvement.
 */

export interface LintIssue {
  line: number;
  column: number;
  rule: string;
  severity: "warning" | "error";
  message: string;
}

const PATTERNS: Array<{
  rule: string;
  severity: LintIssue["severity"];
  regex: RegExp;
  build: (m: RegExpExecArray) => string;
}> = [
  {
    rule: "attribute-with-no-name",
    severity: "error",
    // Match: name="value" followed by whitespace and a stray quote that
    // would open a new attribute. The name+value is captured in $1.
    // Example: `alt="foo" "` — the trailing `"` starts a phantom attr.
    regex: /(\b[a-zA-Z][\w-]*)\s*=\s*["'][^"']*["']\s+["']\s*$/g,
    build: (m) =>
      `Atributo sin nombre después de "${m[1]}" — probablemente quedó una comilla suelta.`,
  },
  {
    rule: "duplicate-attribute",
    severity: "warning",
    // Match each `name="..."` occurrence on a line; we then de-dup
    // in the loop below. Run per-line.
    regex: /\b([a-zA-Z][\w-]*)\s*=\s*["'][^"']*["']/g,
    build: (m) => `Atributo duplicado: "${m[1]}".`,
  },
  {
    rule: "absolute-asset-path",
    severity: "error",
    // Catch Windows absolute paths and Unix absolute paths used as
    // values for likely-image attributes. We only flag the value
    // starts with `C:\` / `C:/` (Windows drive) or `/`.
    regex:
      /\b(src|poster|image|href)\s*=\s*["'](?:[A-Za-z]:[\\/]|\/(?!\/))[^"']*["']/g,
    build: (m) =>
      `Path absoluto en atributo "${m[1]}" — los assets deben usar paths relativos al archivo.`,
  },
];

export function lintBody(body: string): LintIssue[] {
  const issues: LintIssue[] = [];
  const lines = body.split(/\r?\n/);

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineNo = i + 1;

    for (const p of PATTERNS) {
      if (p.rule === "duplicate-attribute") {
        // Special handling: collect per-line attribute names, then
        // flag every occurrence after the first.
        const seen = new Set<string>();
        for (const m of line.matchAll(p.regex)) {
          const name = m[1].toLowerCase();
          if (seen.has(name)) {
            issues.push({
              line: lineNo,
              column: (m.index ?? 0) + 1,
              rule: p.rule,
              severity: p.severity,
              message: p.build(m),
            });
          }
          seen.add(name);
        }
      } else {
        let m: RegExpExecArray | null;
        // Reset lastIndex per call (regex with /g keeps state).
        const re = new RegExp(p.regex.source, p.regex.flags);
        while ((m = re.exec(line)) !== null) {
          issues.push({
            line: lineNo,
            column: (m.index ?? 0) + 1,
            rule: p.rule,
            severity: p.severity,
            message: p.build(m),
          });
        }
      }
    }
  }

  // Multi-line check: unclosed JSX tags. Heuristic — we look for
  // `<Component` that doesn't end with `/>` on the same line, and
  // check whether the body has a matching `</Component>` later.
  // False positives are possible (e.g. JSX comments or markdown
  // that mentions `<Component>`); we only flag when no closing
  // tag exists at all.
  const unclosed = findUnclosedTags(body);
  for (const t of unclosed) {
    issues.push({
      line: t.line,
      column: t.column,
      rule: "unclosed-jsx-tag",
      severity: "error",
      message: `Tag JSX "<${t.name}>" parece no estar cerrado. Falta "/>" o "</${t.name}>" en alguna línea posterior.`,
    });
  }

  return issues;
}

interface UnclosedTag {
  name: string;
  line: number;
  column: number;
}

/**
 * Very small linter pass: scan for `<Foo ...>` that is NOT self-closed
 * on the same line, and never has a matching `</Foo>` later in the
 * body. We do not attempt to handle nested same-name tags perfectly
 * — false positives are acceptable as long as the most common
 * mistakes (forgotten `/>` on a one-liner) are caught.
 */
function findUnclosedTags(body: string): UnclosedTag[] {
  const openRe = /<([A-Z][\w]*)\b[^>]*?(?<!\/)>/g;
  const result: UnclosedTag[] = [];

  let m: RegExpExecArray | null;
  while ((m = openRe.exec(body)) !== null) {
    const name = m[1];
    // Build a per-tag close regex. (We can't use \1 here because
    // the closing tag's regex is independent of the opening one.)
    const closeRe = new RegExp(`<\\/${name}\\s*>`, "g");
    const after = body.slice(m.index);
    const closes = [...after.matchAll(closeRe)];
    if (closes.length === 0) {
      const line = body.slice(0, m.index).split(/\r?\n/).length;
      const col = m.index - body.lastIndexOf("\n", m.index - 1);
      result.push({ name, line, column: col > 0 ? col : 1 });
    }
  }
  return result;
}
