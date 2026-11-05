/**
 * Helpers for managing the `import` block at the top of an MDX file.
 *
 * In MDX, all `import` statements must come before any other content
 * (after the optional frontmatter, but before the first heading).
 * When the user clicks "Insert image" in an `.mdx` file with a
 * wrapper component configured, we need to:
 *
 *   1. Make sure the wrapper's import is present.
 *   2. Don't add it twice if it's already there.
 *   3. Insert it in the canonical place (after the frontmatter, at
 *      the top of the body).
 *
 * The exact import statement is derived from the configured path
 * (the file's basename without extension becomes the component name,
 * PascalCased). Example: `../../components/Figure.astro` →
 * `import Figure from "../../components/Figure.astro";`.
 */

export interface ImportInsertionResult {
  /** The (possibly modified) body. */
  body: string;
  /** Whether the import was newly added. */
  inserted: boolean;
  /** True if the import was already present (no change made). */
  alreadyPresent: boolean;
}

/** "figure.astro" → "Figure", "image-with-caption.tsx" → "ImageWithCaption" */
export function componentNameFromPath(filePath: string): string {
  const base = filePath
    .split(/[\\/]/)
    .pop() ?? filePath;
  const stem = base.replace(/\.[^.]+$/, "");
  // PascalCase: capitalize first letter of each "word" (split on
  // `-`, `_`, or whitespace, then join with no separator).
  return stem
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map((w) => w[0].toUpperCase() + w.slice(1))
    .join("");
}

/**
 * Find the offset where new `import` statements should be inserted.
 * Returns:
 *   - The position right after the closing `---` of the frontmatter,
 *     if one is present at the very start of the file.
 *   - Otherwise, the position right after the leading whitespace /
 *     blank lines (i.e. position 0 in practice).
 *   - If the file already has imports at the top, the position is
 *     right after the LAST import, so the new import is appended
 *     to the import block (no scattered placement).
 *
 * Always returns a position that is at the start of a new line
 * (after a newline character).
 */
export function findImportInsertionPoint(body: string): number {
  const lines = body.split("\n");
  let i = 0;

  // Skip a leading BOM if present.
  if (lines[0]?.startsWith("\uFEFF")) {
    i = 1;
  }

  // 1. Frontmatter: starts with `---` on its own line, ends with
  //    `---` on its own line. If found, insert after the closing
  //    fence plus one blank line.
  if (lines[i]?.trim() === "---") {
    let end = i + 1;
    while (end < lines.length && lines[end].trim() !== "---") end++;
    if (end < lines.length) {
      let insertAt = end + 1;
      // Skip the blank line(s) right after the frontmatter so we
      // insert at the first non-blank line, then back up to the blank
      // line that precedes it (so we keep one blank line for
      // readability).
      while (
        insertAt < lines.length &&
        lines[insertAt].trim() === "" &&
        insertAt < lines.length - 1 &&
        lines[insertAt + 1].trim() !== ""
      ) {
        insertAt++;
      }
      return charOffsetOfLine(lines, insertAt);
    }
  }

  // 2. Already-present imports: skip them so we append to the
  //    import block instead of creating a second one.
  while (i < lines.length && /^import\s+/.test(lines[i])) {
    i++;
  }

  // 3. If we walked past some imports, insert right after them. If
  //    not, insert at position 0.
  return charOffsetOfLine(lines, i);
}

function charOffsetOfLine(lines: string[], lineIndex: number): number {
  let offset = 0;
  for (let i = 0; i < lineIndex && i < lines.length; i++) {
    offset += lines[i].length + 1; // +1 for the newline
  }
  return offset;
}

/**
 * Ensure `importStatement` is present at the top of the MDX body.
 * Returns the (possibly modified) body and whether a change was
 * made. Idempotent — calling twice with the same statement is a
 * no-op the second time.
 */
export function ensureImport(
  body: string,
  importStatement: string
): ImportInsertionResult {
  const trimmed = importStatement.trim();
  if (!trimmed) return { body, inserted: false, alreadyPresent: false };

  // Naive but effective: search for the same import line anywhere.
  // We compare on the canonical single-line form so `Figure` and
  // `figure.astro` paths and exact text both match.
  if (body.includes(trimmed)) {
    return { body, inserted: false, alreadyPresent: true };
  }

  const insertAt = findImportInsertionPoint(body);
  const before = body.slice(0, insertAt);
  const after = body.slice(insertAt);
  // Ensure blank line separation: if the chunk before doesn't end in
  // a blank line, add one. Same for the chunk after.
  const needsLeadingNl = before.length > 0 && !before.endsWith("\n\n");
  const needsTrailingNl = after.length > 0 && !after.startsWith("\n\n");

  const insertion =
    (needsLeadingNl ? "\n" : "") +
    trimmed +
    (needsTrailingNl ? "\n" : "");

  return {
    body: before + insertion + after,
    inserted: true,
    alreadyPresent: false,
  };
}
