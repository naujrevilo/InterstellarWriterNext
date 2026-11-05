/**
 * GitHub sync store.
 *
 * Tracks the state of a sync attempt so any UI piece (button,
 * toast, modal) can react. States are mutually exclusive — once a
 * sync finishes (success or failure), the store settles back to
 * `idle` after a short delay so the user can sync again.
 *
 * The personal access token itself never reaches the frontend: it lives
 * in the operating-system credential store and is used by the backend.
 * This store only knows *whether* a token is configured.
 */

import { writable, get } from "svelte/store";
import {
  syncToGithub,
  githubTokenConfigured as queryTokenConfigured,
  type SyncOutcome,
} from "../api/tauri";
import { workspaceStore } from "./workspace";
import { documentStore } from "./document";
import { notifications } from "./notifications";

export type SyncState =
  | { kind: "idle" }
  | { kind: "syncing" }
  | { kind: "success"; message: string }
  | { kind: "up_to_date" }
  | { kind: "auth_failed"; reason: string }
  | { kind: "scope_missing"; reason: string }
  | { kind: "conflict"; reason: string }
  | { kind: "error"; reason: string };

/** Whether the credential store currently holds a GitHub token. */
export interface TokenState {
  configured: boolean;
  loading: boolean;
}

function createTokenStore() {
  const { subscribe, update } = writable<TokenState>({
    configured: false,
    loading: false,
  });

  let lastKnown = false;

  return {
    subscribe,

    /**
     * Ask the backend whether a token is stored. The answer is a
     * boolean — the secret never leaves the backend.
     *
     * Returns the current state so callers can branch on it.
     */
    async refresh(): Promise<boolean> {
      update((s) => ({ ...s, loading: true }));
      try {
        const configured = await queryTokenConfigured();
        lastKnown = configured;
        update(() => ({ configured, loading: false }));
        return configured;
      } catch (e: any) {
        console.warn(
          "[github] Could not read the GitHub token state:",
          e?.message ?? e
        );
        update((s) => ({ ...s, loading: false }));
        return lastKnown;
      }
    },

    /** Record a state the backend already confirmed. */
    setConfigured(configured: boolean) {
      lastKnown = configured;
      update(() => ({ configured, loading: false }));
    },
  };
}

/** Configured / not-configured state of the stored GitHub token. */
export const githubTokenStore = createTokenStore();

function createGitHubStore() {
  const { subscribe, set } = writable<SyncState>({ kind: "idle" });

  let successTimer: ReturnType<typeof setTimeout> | null = null;
  const RESET_DELAY_MS = 2500;

  function clearTimer() {
    if (successTimer) {
      clearTimeout(successTimer);
      successTimer = null;
    }
  }

  function scheduleReset() {
    clearTimer();
    successTimer = setTimeout(() => set({ kind: "idle" }), RESET_DELAY_MS);
  }

  return {
    subscribe,

    /**
     * Run a sync. Returns the final state so callers can branch on
     * it (e.g. open the settings dialog on `auth_failed`).
     */
    async sync(): Promise<SyncState> {
      if (!get(githubTokenStore).configured) {
        const reason = "No GitHub token configured. Open Settings to add one.";
        set({ kind: "auth_failed", reason });
        return { kind: "auth_failed", reason };
      }

      const ws = get(workspaceStore);
      if (!ws.root) {
        const reason = "No workspace open.";
        set({ kind: "error", reason });
        return { kind: "error", reason };
      }

      set({ kind: "syncing" });
      clearTimer();

      const doc = get(documentStore);
      const fileName = doc.relativePath
        ? doc.relativePath.split(/[\\/]/).pop()
        : null;
      const commitMessage = fileName
        ? `Update ${fileName}`
        : "Update content";

      try {
        const outcome: SyncOutcome = await syncToGithub(
          ws.root,
          commitMessage
        );
        return applyOutcome(outcome, set, scheduleReset, notifications);
      } catch (err: any) {
        const reason = err?.message ?? String(err);
        set({ kind: "error", reason });
        return { kind: "error", reason };
      }
    },

    /** Reset the store back to idle (e.g. when closing the workspace). */
    reset() {
      clearTimer();
      set({ kind: "idle" });
    },
  };
}

function applyOutcome(
  outcome: SyncOutcome,
  set: (s: SyncState) => void,
  scheduleReset: () => void,
  notifications: typeof import("./notifications").notifications
): SyncState {
  switch (outcome.kind) {
    case "synced":
      set({ kind: "success", message: outcome.message });
      notifications.success("Synced to GitHub");
      scheduleReset();
      return { kind: "success", message: outcome.message };
    case "no_changes":
      set({ kind: "up_to_date" });
      notifications.info("Up to date — nothing to commit");
      scheduleReset();
      return { kind: "up_to_date" };
    case "auth_failed":
      set({ kind: "auth_failed", reason: outcome.reason });
      notifications.error(`Auth failed: ${outcome.reason}`);
      return { kind: "auth_failed", reason: outcome.reason };
    case "scope_missing":
      set({ kind: "scope_missing", reason: outcome.reason });
      notifications.error(`Token scope missing: ${outcome.reason}`);
      return { kind: "scope_missing", reason: outcome.reason };
    case "conflict":
      set({ kind: "conflict", reason: outcome.reason });
      notifications.error(`Conflict: ${outcome.reason}`);
      return { kind: "conflict", reason: outcome.reason };
    case "failed":
      set({ kind: "error", reason: outcome.reason });
      notifications.error(`Sync failed: ${outcome.reason}`);
      return { kind: "error", reason: outcome.reason };
  }
}

export const githubSyncStore = createGitHubStore();
