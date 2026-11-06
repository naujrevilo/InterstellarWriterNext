<script lang="ts">
  import { tick } from "svelte";
  import { uiStore } from "../../stores/ui";
  import { availableCommands, shortcutGroups, type Command } from "../../features/commands/registry";

  let { paletteOpen, helpOpen } = $derived($uiStore);

  let query = $state("");
  let active = $state(0);
  let inputEl: HTMLInputElement | null = $state(null);
  let helpEl: HTMLDivElement | null = $state(null);
  let returnFocus: HTMLElement | null = null;

  // Recomputed on every keystroke and on open, so availability is current.
  let results = $derived(paletteOpen ? availableCommands(query) : ([] as Command[]));

  $effect(() => {
    if (paletteOpen) {
      returnFocus = document.activeElement as HTMLElement | null;
      query = "";
      active = 0;
      void tick().then(() => inputEl?.focus());
    }
  });

  $effect(() => {
    if (helpOpen) {
      returnFocus = document.activeElement as HTMLElement | null;
      void tick().then(() => helpEl?.focus());
    }
  });

  $effect(() => {
    // Keep the highlighted row inside the shrinking list.
    if (active >= results.length) active = Math.max(0, results.length - 1);
  });

  function restoreFocus() {
    const el = returnFocus;
    returnFocus = null;
    void tick().then(() => el?.focus?.());
  }

  function closePalette() {
    uiStore.closePalette();
    restoreFocus();
  }

  function closeHelp() {
    uiStore.closeHelp();
    restoreFocus();
  }

  async function runCommand(command: Command | undefined) {
    if (!command) return;
    uiStore.closePalette();
    // Run after the dialog closes so focus-dependent commands act on the editor.
    restoreFocus();
    await command.run();
  }

  function onPaletteKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      closePalette();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      if (results.length) active = (active + 1) % results.length;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (results.length) active = (active - 1 + results.length) % results.length;
    } else if (e.key === "Enter") {
      e.preventDefault();
      void runCommand(results[active]);
    } else if (e.key === "Tab") {
      e.preventDefault();
    }
  }

  function onHelpKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      closeHelp();
    } else if (e.key === "Tab") {
      e.preventDefault();
    }
  }
</script>

{#if paletteOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="overlay" onclick={closePalette}>
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div
      class="panel"
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={onPaletteKeydown}
    >
      <input
        bind:this={inputEl}
        bind:value={query}
        class="search"
        type="text"
        role="combobox"
        aria-expanded="true"
        aria-controls="palette-list"
        aria-activedescendant={results.length ? `palette-opt-${results[active]?.id}` : undefined}
        aria-label="Type a command"
        placeholder="Type a command…"
        autocomplete="off"
        spellcheck="false"
      />
      <ul id="palette-list" class="list" role="listbox" aria-label="Commands">
        {#each results as command, i (command.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <li
            id="palette-opt-{command.id}"
            role="option"
            aria-selected={i === active}
            class="row"
            class:row-active={i === active}
            onmousemove={() => (active = i)}
            onclick={() => runCommand(command)}
          >
            <span>{command.label}</span>
            {#if command.keys}<kbd>{command.keys}</kbd>{/if}
          </li>
        {:else}
          <li class="empty" role="presentation">No matching command</li>
        {/each}
      </ul>
    </div>
  </div>
{/if}

{#if helpOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="overlay" onclick={closeHelp}>
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div
      bind:this={helpEl}
      class="panel help"
      role="dialog"
      aria-modal="true"
      aria-label="Keyboard shortcuts"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={onHelpKeydown}
    >
      <h2 class="help-title">Keyboard shortcuts</h2>
      {#each shortcutGroups() as group (group.title)}
        <section class="group">
          <h3 class="group-title">{group.title}</h3>
          <dl class="shortcuts">
            {#each group.items as item (item.label)}
              <div class="shortcut">
                <dt>{item.label}</dt>
                <dd><kbd>{item.keys}</kbd></dd>
              </div>
            {/each}
          </dl>
        </section>
      {/each}
      <p class="hint">Press Esc to close.</p>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 14vh;
    background: rgba(31, 27, 22, 0.28);
  }

  .panel {
    width: min(34rem, calc(100vw - 2rem));
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    background: var(--color-paper);
    color: var(--color-ink);
    border: 1px solid var(--color-rule-strong);
    border-radius: 10px;
    box-shadow: var(--shadow-elevated), 0 16px 40px rgba(0, 0, 0, 0.16);
    font-family: var(--font-ui);
    overflow: hidden;
  }

  .search {
    border: 0;
    border-bottom: 1px solid var(--color-rule);
    background: transparent;
    padding: var(--space-3) var(--space-4);
    font: inherit;
    font-size: var(--text-base);
    color: inherit;
    outline: none;
  }

  .search:focus-visible {
    box-shadow: inset 0 -2px 0 var(--color-accent);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: var(--space-1);
    overflow-y: auto;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-3);
    border-radius: 6px;
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .row-active {
    background: var(--color-accent-soft);
  }

  .empty {
    padding: var(--space-3);
    font-size: var(--text-sm);
    color: var(--color-ink-soft);
  }

  kbd {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--color-ink-soft);
    background: var(--color-chrome);
    border: 1px solid var(--color-rule);
    border-radius: 4px;
    padding: 1px 6px;
    white-space: nowrap;
  }

  .help {
    padding: var(--space-6);
    overflow-y: auto;
    outline: none;
  }

  .help-title {
    margin: 0 0 var(--space-4);
    font-size: var(--text-xl);
    font-weight: 600;
  }

  .group {
    margin-bottom: var(--space-4);
  }

  .group-title {
    margin: 0 0 var(--space-2);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-ink-soft);
  }

  .shortcuts {
    margin: 0;
  }

  .shortcut {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-1) 0;
    border-bottom: 1px solid var(--color-rule);
    font-size: var(--text-sm);
  }

  .shortcut dt,
  .shortcut dd {
    margin: 0;
  }

  .hint {
    margin: var(--space-2) 0 0;
    font-size: var(--text-xs);
    color: var(--color-ink-soft);
  }
</style>
