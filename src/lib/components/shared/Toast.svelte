<script lang="ts">
  import { notifications } from "../../stores/notifications";

  let items = $derived($notifications);
</script>

{#if items.length > 0}
  <div class="toast-container" aria-live="polite">
    {#each items as item (item.id)}
      <div
        class="toast toast-{item.type}"
        role="alert"
      >
        <span class="toast-message">{item.message}</span>
        <button
          onclick={() => notifications.dismiss(item.id)}
          class="toast-dismiss"
          aria-label="Dismiss"
        >
          &times;
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .toast-container {
    position: fixed;
    bottom: var(--space-4);
    right: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    z-index: 100;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
    border-radius: 7px;
    font-size: 12px;
    font-family: var(--font-ui);
    font-weight: 500;
    color: white;
    background: var(--color-ink);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    pointer-events: auto;
    animation: toast-in 0.18s ease-out;
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .toast {
      animation: none;
    }
  }

  .toast-success {
    background: #166534;
  }

  .toast-error {
    background: var(--color-danger);
  }

  .toast-info {
    background: var(--color-ink);
  }

  .toast-message {
    flex: 1;
  }

  .toast-dismiss {
    background: none;
    border: none;
    color: inherit;
    opacity: 0.7;
    cursor: pointer;
    font-size: var(--text-lg);
    padding: 0;
    line-height: 1;
    transition: opacity 0.15s;
  }

  .toast-dismiss:hover {
    opacity: 1;
  }
</style>
