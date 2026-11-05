<script lang="ts">
  /**
   * Modal for configuring the GitHub personal access token.
   *
   * Triggered by the SyncButton when no token is set, or directly
   * by the user from the toolbar (we add a "GitHub" icon next to
   * the sync button for that — TODO). For v1 we just expose
   * the token field, the "test connection" call (which is
   * actually just a sync attempt against the current repo) and
   * a "clear token" link.
   *
   * The token is handed to the backend, which stores it in the
   * operating-system credential store. The modal only ever learns
   * *whether* a token is configured, and never pre-fills the field.
   */
  import {
    saveGithubToken,
    clearGithubToken,
  } from "../../api/tauri";
  import { githubSyncStore, githubTokenStore } from "../../stores/github";
  import { notifications } from "../../stores/notifications";

  let {
    open = $bindable(false),
  }: { open: boolean } = $props();

  let token = $state("");
  let hasExisting = $state(false);
  let saving = $state(false);
  let testing = $state(false);

  $effect(() => {
    if (open) {
      // Don't pre-fill the password field for security; we just
      // surface the masked status below.
      token = "";
      void githubTokenStore.refresh().then((configured) => {
        hasExisting = configured;
      });
    }
  });

  async function handleSave() {
    const value = token.trim();
    if (!value) {
      notifications.error("Token cannot be empty");
      return;
    }
    saving = true;
    try {
      await saveGithubToken(value);
      githubTokenStore.setConfigured(true);
      notifications.success("GitHub token saved");
      token = "";
      hasExisting = true;
      open = false;
    } catch (e: any) {
      notifications.error(e?.message ?? "Could not save the GitHub token");
    } finally {
      saving = false;
    }
  }

  async function handleClear() {
    try {
      await clearGithubToken();
      githubTokenStore.setConfigured(false);
      notifications.info("Token cleared");
      hasExisting = false;
    } catch (e: any) {
      notifications.error(e?.message ?? "Could not clear the GitHub token");
    }
  }

  async function handleTestSync() {
    testing = true;
    const result = await githubSyncStore.sync();
    testing = false;
    if (result.kind === "auth_failed" || result.kind === "scope_missing") {
      // Stay in the modal so the user can fix the token.
      return;
    }
    if (result.kind === "success" || result.kind === "up_to_date") {
      open = false;
    }
  }

  function handleCancel() {
    open = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") handleCancel();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class="modal-backdrop">
    <button
      type="button"
      class="modal-backdrop-dismiss"
      aria-label="Close GitHub Sync"
      onclick={handleCancel}
    ></button>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="gh-settings-title"
      tabindex="-1"
    >
      <h2 id="gh-settings-title" class="modal-title">GitHub Sync</h2>
      <p class="modal-subtitle">
        Connect a repository to push your changes with one click.
        We use a personal access token with the <code>repo</code> scope.
      </p>

      <div class="form-row">
        <label for="gh-token" class="form-label">Personal access token</label>
        <input
          id="gh-token"
          type="password"
          bind:value={token}
          placeholder={hasExisting ? "•••••••••• (saved)" : "ghp_..."}
          autocomplete="off"
          spellcheck="false"
          class="form-input"
        />
        <p class="form-hint">
          {#if hasExisting}
            A token is already configured. Type a new one to replace it.
          {:else}
            Get one at GitHub → Settings → Developer settings → Personal access tokens.
          {/if}
        </p>
      </div>

      <div class="form-actions">
        <button type="button" class="btn-secondary" onclick={handleCancel}>
          Close
        </button>
        <button
          type="button"
          class="btn-secondary"
          onclick={handleClear}
          disabled={!hasExisting}
        >
          Clear
        </button>
        <button type="button" class="btn-secondary" onclick={handleTestSync} disabled={!hasExisting || testing}>
          {testing ? "Testing…" : "Test sync"}
        </button>
        <button
          type="button"
          class="btn-primary"
          onclick={handleSave}
          disabled={!token.trim() || saving}
        >
          Save
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
    backdrop-filter: blur(2px);
  }

  .modal-backdrop-dismiss {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: default;
  }

  .modal {
    position: relative;
    z-index: 1;
    width: min(440px, calc(100vw - 32px));
    background: var(--color-paper);
    border: 1px solid var(--color-rule);
    border-radius: 10px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.18);
    padding: 22px 24px 18px;
    font-family: var(--font-ui);
  }

  .modal-title {
    margin: 0 0 4px;
    font-family: var(--font-editor);
    font-size: 18px;
    font-weight: 600;
    color: var(--color-ink);
  }

  .modal-subtitle {
    margin: 0 0 18px;
    font-size: 12px;
    color: var(--color-muted);
    line-height: 1.5;
  }

  .modal-subtitle code {
    font-family: var(--font-mono);
    font-size: 11px;
    background: var(--color-chrome);
    padding: 1px 5px;
    border-radius: 3px;
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 18px;
  }

  .form-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-muted);
  }

  .form-input {
    width: 100%;
    height: 36px;
    padding: 0 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--color-ink);
    background: white;
    border: 1px solid var(--color-rule);
    border-radius: 6px;
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .form-input:focus {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 3px rgba(109, 40, 217, 0.12);
  }

  .form-hint {
    margin: 0;
    font-size: 11px;
    color: var(--color-muted);
    line-height: 1.5;
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn-primary,
  .btn-secondary {
    height: 32px;
    padding: 0 14px;
    border-radius: 6px;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s, color 0.15s;
  }

  .btn-primary {
    color: white;
    background: var(--color-ink);
    border: 1px solid var(--color-ink);
  }
  .btn-primary:hover:not(:disabled) { background: #292524; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

  .btn-secondary {
    color: var(--color-ink);
    background: white;
    border: 1px solid var(--color-rule-strong);
  }
  .btn-secondary:hover:not(:disabled) {
    border-color: var(--color-ink-soft);
    background: #fafaf9;
  }
  .btn-secondary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
