<script lang="ts">
  let {
    open = false,
    backups = [],
    loading = false,
    restoring = false,
    onClose,
    onRestore,
  }: {
    open?: boolean;
    backups?: string[];
    loading?: boolean;
    restoring?: boolean;
    onClose: () => void;
    onRestore: (backupName: string) => void | Promise<void>;
  } = $props();

  let dialog: HTMLDialogElement | undefined = $state();

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  });

  function backupLabel(name: string): string {
    return name === "latest.bak" ? "Latest backup" : name.replace(/\.bak$/, "");
  }
</script>

<dialog
  bind:this={dialog}
  class="recovery-dialog"
  aria-labelledby="recovery-title"
  aria-describedby="recovery-description"
  onclose={() => {
    if (open) onClose();
  }}
>
  <header class="recovery-header">
    <div>
      <h2 id="recovery-title">Restore a backup</h2>
      <p id="recovery-description">
        The current file is backed up before a restore.
      </p>
    </div>
    <button
      type="button"
      class="close-button"
      onclick={() => dialog?.close()}
      aria-label="Close backup recovery"
    >
      ×
    </button>
  </header>

  {#if loading}
    <p class="recovery-status" role="status">Looking for backups…</p>
  {:else if backups.length === 0}
    <p class="recovery-status">No backups are available for this document yet.</p>
  {:else}
    <ul class="backup-list">
      {#each backups as backup (backup)}
        <li>
          <span>{backupLabel(backup)}</span>
          <button
            type="button"
            class="restore-button"
            disabled={restoring}
            onclick={() => onRestore(backup)}
          >
            {restoring ? "Restoring…" : "Restore"}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</dialog>

<style>
  .recovery-dialog {
    width: min(440px, calc(100vw - 32px));
    max-height: min(80vh, 560px);
    padding: 22px;
    color: var(--color-ink);
    background: var(--color-paper);
    border: 1px solid var(--color-rule);
    border-radius: 12px;
    box-shadow: 0 18px 60px rgba(31, 27, 22, 0.2);
    font-family: var(--font-ui);
  }

  .recovery-dialog::backdrop {
    background: rgba(31, 27, 22, 0.28);
    backdrop-filter: blur(2px);
  }

  .recovery-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 18px;
  }

  h2 {
    margin: 0 0 5px;
    font-family: var(--font-editor);
    font-size: 22px;
    font-weight: 500;
  }

  .recovery-header p,
  .recovery-status {
    margin: 0;
    color: var(--color-muted);
    font-size: 13px;
    line-height: 1.5;
  }

  .close-button {
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: 6px;
    color: var(--color-muted);
    background: transparent;
    cursor: pointer;
    font-size: 20px;
  }

  .close-button:hover {
    color: var(--color-ink);
    background: rgba(0, 0, 0, 0.05);
  }

  .backup-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 50vh;
    overflow-y: auto;
    padding: 0;
    margin: 0;
    list-style: none;
  }

  .backup-list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 9px 10px;
    border: 1px solid var(--color-rule);
    border-radius: 7px;
    font-size: 13px;
  }

  .restore-button {
    flex: 0 0 auto;
    padding: 6px 11px;
    border: 1px solid var(--color-rule-strong);
    border-radius: 6px;
    color: var(--color-ink);
    background: white;
    cursor: pointer;
    font: inherit;
  }

  .restore-button:hover:not(:disabled) {
    color: var(--color-accent);
    border-color: var(--color-accent);
  }

  .restore-button:disabled {
    cursor: wait;
    opacity: 0.55;
  }
</style>
