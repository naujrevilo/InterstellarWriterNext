import { get } from "svelte/store";
import { uiStore } from "../../stores/ui";
import { documentStore, canUndo, canRedo } from "../../stores/document";
import { preferencesStore } from "../../stores/preferences";
import { workspaceStore } from "../../stores/workspace";
import { githubSyncStore, githubTokenStore } from "../../stores/github";

/**
 * Single definition of every command and shortcut. The command palette
 * and the shortcuts reference are both generated from these lists, so
 * the two can never disagree.
 */

export interface Command {
  id: string;
  label: string;
  /** Shortcut shown next to the command, e.g. "Ctrl+Shift+F". */
  keys?: string;
  /** Hidden from the palette when this returns false. */
  when?: () => boolean;
  run: () => void | Promise<void>;
}

export interface ShortcutGroup {
  title: string;
  items: { label: string; keys: string }[];
}

let saveHandler: (() => void | Promise<void>) | null = null;

/** The editor panel owns saving (re-scan, sync on save); it registers here. */
export function setSaveHandler(handler: (() => void | Promise<void>) | null) {
  saveHandler = handler;
}

const hasDocument = () => !!get(workspaceStore).selectedFile;

export const COMMANDS: Command[] = [
  {
    id: "save",
    label: "Save",
    keys: "Ctrl+S",
    when: () => get(documentStore).dirty && saveHandler !== null,
    run: () => saveHandler?.(),
  },
  {
    id: "toggle-autosave",
    label: "Toggle autosave",
    run: () => {
      preferencesStore.setAutosave(!get(preferencesStore).autosave);
    },
  },
  {
    id: "toggle-preview",
    label: "Toggle preview",
    keys: "Ctrl+Shift+P",
    when: hasDocument,
    run: () => uiStore.togglePreview(),
  },
  {
    id: "toggle-focus",
    label: "Toggle focus mode",
    keys: "Ctrl+Shift+F",
    when: hasDocument,
    run: () => uiStore.toggleFocusMode(),
  },
  {
    id: "toggle-files",
    label: "Toggle files sidebar",
    keys: "Ctrl+B",
    run: () => uiStore.toggleLeftSidebar(),
  },
  {
    id: "toggle-metadata",
    label: "Toggle metadata panel",
    keys: "Ctrl+I",
    run: () => uiStore.toggleRightSidebar(),
  },
  {
    id: "undo",
    label: "Undo",
    keys: "Ctrl+Z",
    when: () => get(canUndo),
    run: () => {
      documentStore.undo();
    },
  },
  {
    id: "redo",
    label: "Redo",
    keys: "Ctrl+Y",
    when: () => get(canRedo),
    run: () => {
      documentStore.redo();
    },
  },
  {
    id: "sync-github",
    label: "Sync to GitHub",
    when: () => get(githubTokenStore).configured && !!get(workspaceStore).root,
    run: async () => {
      await githubSyncStore.sync();
    },
  },
  {
    id: "shortcuts",
    label: "Keyboard shortcuts",
    keys: "Ctrl+/",
    run: () => uiStore.openHelp(),
  },
];

/** Shortcuts that act inside the writing surface and are not palette commands. */
const EDITOR_SHORTCUTS = [
  { label: "Bold", keys: "Ctrl+B" },
  { label: "Italic", keys: "Ctrl+I" },
  { label: "Insert image (.md / .mdx)", keys: "Ctrl+Shift+I" },
  { label: "Continue list", keys: "Enter" },
];

export function shortcutGroups(): ShortcutGroup[] {
  const fromCommands = COMMANDS.filter((c) => c.keys).map((c) => ({
    label: c.label,
    keys: c.keys as string,
  }));
  return [
    {
      title: "Anywhere",
      items: [{ label: "Command palette", keys: "Ctrl+K" }, ...fromCommands],
    },
    { title: "Writing surface", items: EDITOR_SHORTCUTS },
    { title: "Focus mode", items: [{ label: "Leave focus mode", keys: "Esc" }] },
  ];
}

/** Commands that can run now, matching `query` (all words, any order). */
export function availableCommands(query: string): Command[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return COMMANDS.filter((c) => (c.when ? c.when() : true)).filter((c) => {
    const label = c.label.toLowerCase();
    return words.every((w) => label.includes(w));
  });
}
