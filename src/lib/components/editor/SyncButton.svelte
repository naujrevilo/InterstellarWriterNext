<script lang="ts">
  /**
   * The "Sync to GitHub" button that lives in the editor toolbar.
   *
   * Reflects the current `githubSyncStore` state with a color and
   * icon. Idle = neutral, syncing = spinner, success = green flash,
   * error/conflict = red flash. Click always triggers a fresh
   * sync attempt; the actual transition is in the store.
   */
  import { githubSyncStore, githubTokenStore, type SyncState } from "../../stores/github";

  let { onRequestToken }: { onRequestToken: () => void } = $props();

  let syncState: SyncState = $state({ kind: "idle" });

  $effect(() => {
    syncState = $githubSyncStore;
  });

  async function handleClick() {
    // The backend owns the token: ask it whether one is configured.
    // A missing token opens the settings dialog instead of failing.
    const configured = await githubTokenStore.refresh();
    if (!configured) {
      onRequestToken();
      return;
    }
    void githubSyncStore.sync();
  }

  // Map state → CSS class + label.
  let visual = $derived.by(() => {
    switch (syncState.kind) {
      case "syncing":
        return { cls: "sync-on", label: "Syncing", icon: "spinner" };
      case "success":
        return { cls: "sync-success", label: "Synced", icon: "check" };
      case "up_to_date":
        return { cls: "sync-success", label: "Up to date", icon: "check" };
      case "auth_failed":
      case "scope_missing":
        return { cls: "sync-error", label: "Auth failed", icon: "alert" };
      case "conflict":
        return { cls: "sync-error", label: "Conflict", icon: "alert" };
      case "error":
        return { cls: "sync-error", label: "Sync error", icon: "alert" };
      default:
        return { cls: "sync-idle", label: "Sync GitHub", icon: "github" };
    }
  });
</script>

<button
  type="button"
  class="icon-btn sync-btn {visual.cls}"
  title="Sync to GitHub"
  aria-label="Sync to GitHub"
  disabled={syncState.kind === "syncing"}
  onclick={handleClick}
>
  {#if visual.icon === "spinner"}
    <svg class="sync-spinner" width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.6" opacity="0.25" />
      <path d="M14 8a6 6 0 0 0-6-6" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    </svg>
  {:else if visual.icon === "check"}
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path d="M3.5 8.5L6.5 11.5L12.5 4.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  {:else if visual.icon === "alert"}
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path d="M8 2L1.5 13.5h13L8 2z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" />
      <line x1="8" y1="6.5" x2="8" y2="9.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
      <circle cx="8" cy="11.5" r="0.7" fill="currentColor" />
    </svg>
  {:else}
    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <path d="M8 0a8 8 0 0 0-2.53 15.59c.4.07.55-.17.55-.39v-1.36c-2.22.48-2.69-1.07-2.69-1.07-.36-.92-.89-1.16-.89-1.16-.73-.5.05-.49.05-.49.81.06 1.23.83 1.23.83.72 1.23 1.88.87 2.34.67.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.13 0 0 .67-.21 2.2.82a7.66 7.66 0 0 1 4 0c1.53-1.03 2.2-.82 2.2-.82.44 1.11.16 1.93.08 2.13.51.56.82 1.28.82 2.15 0 3.07-1.87 3.74-3.65 3.94.29.25.54.74.54 1.48v2.2c0 .22.15.47.55.39A8 8 0 0 0 8 0z" />
    </svg>
  {/if}
  <span class="sync-label">{visual.label}</span>
</button>

<style>
  .sync-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 32px;
    padding: 0 10px;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
    border: 1px solid var(--color-rule);
    border-radius: 6px;
    cursor: pointer;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
  }

  .sync-btn:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .sync-label {
    letter-spacing: 0.01em;
  }

  .sync-idle {
    color: var(--color-ink-soft);
    background: white;
    border-color: var(--color-rule);
  }
  .sync-idle:hover {
    color: var(--color-ink);
    background: #fafaf9;
    border-color: var(--color-rule-strong);
  }

  .sync-on {
    color: var(--color-accent);
    background: var(--color-accent-soft);
    border-color: rgba(109, 40, 217, 0.3);
  }

  .sync-success {
    color: #166534;
    background: #DCFCE7;
    border-color: rgba(22, 101, 52, 0.35);
  }

  .sync-error {
    color: #B91C1C;
    background: rgba(220, 38, 38, 0.08);
    border-color: rgba(220, 38, 38, 0.4);
  }
  .sync-error:hover {
    background: rgba(220, 38, 38, 0.14);
  }

  .sync-spinner {
    animation: sync-spin 0.8s linear infinite;
  }
  @keyframes sync-spin {
    to { transform: rotate(360deg); }
  }
  @media (prefers-reduced-motion: reduce) {
    .sync-spinner { animation: none; }
  }
</style>
