import { writable } from "svelte/store";
import { githubTokenConfigured, saveGithubToken } from "../api/tauri";

/**
 * Persisted user preferences. Stored in localStorage so the settings
 * survive app restarts.
 *
 * Secrets do not live here. The GitHub personal access token is kept in
 * the operating-system credential store (Windows Credential Manager,
 * macOS Keychain, Secret Service on Linux) and is only ever handled by
 * the backend; this module asks the backend whether a token is
 * configured.
 */
const STORAGE_KEY = "iwnext.preferences";

/**
 * Field that used to hold the GitHub token in localStorage. It is read
 * once at startup, removed from the persisted payload and moved into the
 * credential store.
 */
const LEGACY_TOKEN_FIELD = "githubToken";

interface PreferencesState {
  editorFont: string;
  uiFont: string;
  /** When true, the editor prompts for a sync after every save. */
  syncOnSave: boolean;
  /**
   * Path to the project's MDX figure/image wrapper component,
   * relative to a content file in `src/content/`. Used by the
   * image-insert button in `.mdx` files: when set, the button
   * inserts an `<Figure />` snippet + a matching `import` at the
   * top of the file (just after the frontmatter), instead of a
   * plain `![alt](path)` markdown image.
   *
   * Empty = always insert plain markdown (works for `.md` and for
   * projects that don't have a wrapper component).
   */
  figureComponentPath: string;
}

const DEFAULTS: PreferencesState = {
  editorFont: "Newsreader",
  uiFont: "Inter",
  syncOnSave: false,
  figureComponentPath: "",
};

/**
 * Legacy token captured at startup, waiting to be moved into the
 * credential store. It is held in memory only and never written back.
 */
let legacyToken: string | null = null;

/** True once the legacy token has been handled (migrated or dropped). */
let legacyTokenHandled = false;

function loadInitial(): PreferencesState {
  if (typeof localStorage === "undefined") return { ...DEFAULTS };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULTS };
    const parsed = JSON.parse(raw) as Record<string, unknown>;

    // Drop the legacy secret from the persisted payload as soon as we
    // see it, even if the migration below fails: a token must not stay
    // in localStorage.
    if (Object.prototype.hasOwnProperty.call(parsed, LEGACY_TOKEN_FIELD)) {
      const legacyValue = parsed[LEGACY_TOKEN_FIELD];
      if (typeof legacyValue === "string") {
        legacyToken = legacyValue;
      }
      delete parsed[LEGACY_TOKEN_FIELD];
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(parsed));
      } catch {
        // ignore quota / private-mode errors
      }
    }

    return {
      editorFont: typeof parsed.editorFont === "string" ? parsed.editorFont : DEFAULTS.editorFont,
      uiFont: typeof parsed.uiFont === "string" ? parsed.uiFont : DEFAULTS.uiFont,
      syncOnSave: typeof parsed.syncOnSave === "boolean" ? parsed.syncOnSave : DEFAULTS.syncOnSave,
      figureComponentPath:
        typeof parsed.figureComponentPath === "string"
          ? parsed.figureComponentPath
          : DEFAULTS.figureComponentPath,
    };
  } catch {
    // A malformed legacy payload may still contain a token. Remove the
    // whole entry rather than leaving an unreadable secret in storage.
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      // ignore storage failures
    }
    return { ...DEFAULTS };
  }
}

function persist(state: PreferencesState) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // ignore quota / private-mode errors
  }
}

/**
 * Move a legacy token from localStorage into the credential store.
 *
 * Runs once at startup. The token itself is never logged nor included
 * in an error message; when the migration fails the app stays usable
 * and the user has to enter the token again.
 *
 * Returns true when a token was migrated.
 */
async function migrateLegacyToken(): Promise<boolean> {
  if (legacyTokenHandled) return false;
  legacyTokenHandled = true;

  const token = legacyToken;
  legacyToken = null;
  if (!token) return false;

  try {
    // A token already stored in the OS credential store wins over the
    // legacy localStorage value, so a stale payload cannot overwrite it.
    if (await githubTokenConfigured()) return false;
    await saveGithubToken(token);
    return true;
  } catch (e: any) {
    console.warn(
      "[preferences] The GitHub token could not be moved into the system credential store. Enter it again in Settings."
    );
    return false;
  }
}

function createPreferencesStore() {
  const { subscribe, update, set } = writable<PreferencesState>(loadInitial());

  function commit<T>(mutator: (s: PreferencesState) => PreferencesState, persistAfter = true) {
    update((s) => {
      const next = mutator(s);
      if (persistAfter) persist(next);
      return next;
    });
  }

  return {
    subscribe,
    setEditorFont(font: string) {
      commit((s) => ({ ...s, editorFont: font }));
    },
    setUiFont(font: string) {
      commit((s) => ({ ...s, uiFont: font }));
    },
    setSyncOnSave(value: boolean) {
      commit((s) => ({ ...s, syncOnSave: value }));
    },
    setFigureComponentPath(path: string) {
      commit((s) => ({ ...s, figureComponentPath: path }));
    },
    /**
     * One-time migration of a legacy localStorage token into the
     * operating-system credential store. Safe to call more than once:
     * only the first call can migrate.
     */
    migrateLegacyToken,
    /** Wipe all preferences. Mostly useful for tests. */
    reset() {
      persist(DEFAULTS);
      set({ ...DEFAULTS });
    },
  };
}

export const preferencesStore = createPreferencesStore();
