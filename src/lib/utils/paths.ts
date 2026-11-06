/**
 * Path utilities for cross-platform (Windows / macOS / Linux) file paths.
 *
 * The app runs in Tauri so paths can mix forward and backward slashes
 * depending on the OS. We normalize everything to forward slashes for
 * in-memory comparisons, and keep the original separator for the
 * results so they round-trip cleanly to the filesystem.
 */

/** Normalize separators to forward slashes. */
export function toForwardSlashes(p: string): string {
  return p.replace(/\\/g, "/");
}

/** Normalize separators to whatever the current platform prefers. */
export function toNativeSlashes(p: string): string {
  if (typeof navigator !== "undefined" && /Win/i.test(navigator.platform)) {
    return p.replace(/\//g, "\\");
  }
  return p;
}

/**
 * Compute the path from `fromFile` (a relative path inside the
 * workspace) to `toAbsolute` (an absolute filesystem path).
 *
 * - `root`         : workspace root (absolute)
 * - `fromFile`     : the file we're computing relative to (relative to root)
 * - `toAbsolute`   : the target absolute path
 *
 * Returns a forward-slash path, relative, suitable for use in markdown
 * like `![alt](../assets/images/cover.png)`. If `toAbsolute` is outside
 * `root`, returns the absolute path unchanged (caller can decide).
 *
 * Example (Windows):
 *   root       = "F:/projects/blog"
 *   fromFile   = "src/content/blog/post.mdx"
 *   toAbsolute = "F:/projects/blog/src/assets/images/cover.png"
 *   → returns   "../assets/images/cover.png"
 */
export function toRelativePath(
  root: string,
  fromFile: string,
  toAbsolute: string
): string {
  const normRoot = toForwardSlashes(root).replace(/\/+$/, "");
  const normTarget = toForwardSlashes(toAbsolute);

  // Target outside the workspace — fall back to absolute.
  if (!normTarget.toLowerCase().startsWith(normRoot.toLowerCase() + "/")) {
    return normTarget;
  }

  // Get the directory of the source file, relative to root.
  const fileDir = fromFile.split("/").slice(0, -1).join("/");
  const fromDirAbs = fileDir ? `${normRoot}/${fileDir}` : normRoot;

  return computeRelative(fromDirAbs, normTarget);
}

/**
 * Compute a relative forward-slash path from `from` to `to`.
 * Both arguments must be absolute and already normalized to
 * forward slashes.
 */
function computeRelative(from: string, to: string): string {
  const fromParts = from.split("/").filter(Boolean);
  const toParts = to.split("/").filter(Boolean);

  // Find common prefix (case-insensitive on Windows-style paths).
  const lowerFrom = fromParts.map((s) => s.toLowerCase());
  const lowerTo = toParts.map((s) => s.toLowerCase());
  let common = 0;
  while (
    common < lowerFrom.length &&
    common < lowerTo.length - 1 &&
    lowerFrom[common] === lowerTo[common]
  ) {
    common++;
  }

  const ups = fromParts.length - common;
  const downs = toParts.slice(common);

  return [...Array(ups).fill(".."), ...downs].join("/");
}

/** Get the file name (no extension) from a path. */
export function basenameNoExt(p: string): string {
  const name = toForwardSlashes(p).split("/").pop() ?? p;
  return name.replace(/\.[^.]+$/, "");
}
