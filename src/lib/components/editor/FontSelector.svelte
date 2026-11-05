<script lang="ts">
  import { preferencesStore } from "../../stores/preferences";

  let { editorFont } = $derived($preferencesStore);

  const FONTS = [
    { name: "Newsreader", label: "Newsreader", style: "Georgia, serif" },
    { name: "Inter", label: "Inter", style: "system-ui, sans-serif" },
    { name: "JetBrains Mono", label: "JetBrains Mono", style: "monospace" },
  ];

  function setFont(name: string) {
    preferencesStore.setEditorFont(name);
  }
</script>

<div class="font-selector" role="radiogroup" aria-label="Editor font">
  {#each FONTS as font (font.name)}
    <button
      onclick={() => setFont(font.name)}
      class="font-pill"
      class:font-pill-active={editorFont === font.name}
      role="radio"
      aria-checked={editorFont === font.name}
      style="font-family: '{font.name}', {font.style}"
      title={font.label}
    >
      {font.label}
    </button>
  {/each}
</div>

<style>
  .font-selector {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    background: rgba(0, 0, 0, 0.04);
    border-radius: 6px;
  }

  .font-pill {
    height: 22px;
    padding: 0 9px;
    font-size: 11px;
    font-weight: 500;
    color: var(--color-muted);
    background: transparent;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: color 0.15s, background 0.15s;
    letter-spacing: -0.005em;
  }

  .font-pill:hover {
    color: var(--color-ink);
  }

  .font-pill-active {
    background: white;
    color: var(--color-ink);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
  }
</style>
