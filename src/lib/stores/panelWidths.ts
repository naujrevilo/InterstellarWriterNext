import { writable } from "svelte/store";

/**
 * Persisted, clamped widths (in px) for the two resizable side panels.
 *
 * Storage key: "iwnext.panel-widths"
 * Schema: { sidebar: number, metadata: number }
 *
 * The defaults are intentionally small so a 1500px-wide window still
 * leaves room for the editor + the toolbar's two halves.
 */

const STORAGE_KEY = "iwnext.panel-widths";

export const PANEL_DEFAULTS = {
  sidebar: 272, // ~17rem — comfortable for medium-length file names
  metadata: 272, // ~17rem — wide enough for form fields, not greedy
} as const;

export const PANEL_LIMITS = {
  sidebar: { min: 220, max: 400 },
  metadata: { min: 240, max: 360 },
} as const;

interface PanelWidths {
  sidebar: number;
  metadata: number;
}

function clamp(v: unknown, min: number, max: number): number | null {
  if (typeof v !== "number" || !Number.isFinite(v)) return null;
  return Math.max(min, Math.min(max, v));
}

function loadInitial(): PanelWidths {
  if (typeof localStorage === "undefined") return { ...PANEL_DEFAULTS };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...PANEL_DEFAULTS };
    const parsed = JSON.parse(raw) as Partial<PanelWidths>;
    return {
      sidebar: clamp(parsed.sidebar, ...Object.values(PANEL_LIMITS.sidebar) as [number, number]) ?? PANEL_DEFAULTS.sidebar,
      metadata: clamp(parsed.metadata, ...Object.values(PANEL_LIMITS.metadata) as [number, number]) ?? PANEL_DEFAULTS.metadata,
    };
  } catch {
    return { ...PANEL_DEFAULTS };
  }
}

function persist(state: PanelWidths) {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // ignore quota / private-mode errors
  }
}

function createPanelWidthsStore() {
  const { subscribe, update, set } = writable<PanelWidths>(loadInitial());

  return {
    subscribe,
    setSidebar(w: number) {
      update((s) => {
        const next = {
          ...s,
          sidebar: clamp(w, PANEL_LIMITS.sidebar.min, PANEL_LIMITS.sidebar.max) ?? s.sidebar,
        };
        persist(next);
        return next;
      });
    },
    setMetadata(w: number) {
      update((s) => {
        const next = {
          ...s,
          metadata: clamp(w, PANEL_LIMITS.metadata.min, PANEL_LIMITS.metadata.max) ?? s.metadata,
        };
        persist(next);
        return next;
      });
    },
    reset() {
      const next = { ...PANEL_DEFAULTS };
      persist(next);
      set(next);
    },
  };
}

export const panelWidths = createPanelWidthsStore();
