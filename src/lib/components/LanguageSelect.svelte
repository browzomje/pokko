<script lang="ts">
  import { tick } from "svelte";
  import { ChevronDown, Check, Search } from "lucide-svelte";
  import {
    languages,
    specialLanguages,
    getLanguage,
    getLanguageSearchTerms,
    getLanguageLabel,
    normalizeLanguageText,
    type Language,
  } from "../languages";

  let {
    value = $bindable("it"),
    onchange,
    includeBoth = false,
    disabled = false,
    placeholder = "Cerca lingua…",
    label = "Lingua",
  }: {
    value?: string;
    onchange?: (value: string) => void;
    includeBoth?: boolean;
    disabled?: boolean;
    placeholder?: string;
    label?: string;
  } = $props();

  let expanded = $state(false);
  let searchQuery = $state("");
  let highlightedIndex = $state(0);
  let trigger = $state<HTMLButtonElement>();
  let searchInput = $state<HTMLInputElement>();
  let root = $state<HTMLDivElement>();
  let dropdown = $state<HTMLDivElement>();
  let position = $state({ left: 0, top: 0, width: 0, maxHeight: 320 });

  let availableLanguages = $derived.by(() => {
    const list = [...languages];
    if (includeBoth || value === "both") {
      list.unshift(...specialLanguages);
    }
    return list;
  });

  let selectedLanguage = $derived(getLanguage(value));

  let displayLabel = $derived.by(() => {
    if (value === "both") return "🇮🇹 🇬🇧 Entrambe (IT/EN)";
    if (!selectedLanguage) return value ? `🌐 ${value}` : "Scegli lingua…";
    return getLanguageLabel(selectedLanguage);
  });

  let filteredLanguages = $derived.by(() => {
    const query = normalizeLanguageText(searchQuery);
    if (!query) return availableLanguages;
    return availableLanguages.filter((lang) => {
      const searchTerms = normalizeLanguageText(getLanguageSearchTerms(lang.code));
      const codeMatch = lang.code.toLowerCase().includes(query);
      const nameMatch = normalizeLanguageText(lang.name).includes(query);
      const nameEnMatch = normalizeLanguageText(lang.nameEn).includes(query);
      return codeMatch || nameMatch || nameEnMatch || searchTerms.includes(query);
    });
  });

  $effect(() => {
    if (filteredLanguages.length > 0) {
      highlightedIndex = Math.min(highlightedIndex, filteredLanguages.length - 1);
      if (highlightedIndex < 0) highlightedIndex = 0;
    }
  });

  function close() {
    expanded = false;
    searchQuery = "";
  }

  async function open() {
    if (disabled || expanded) return;
    const rect = trigger!.getBoundingClientRect();
    const width = Math.min(Math.max(rect.width, 280), window.innerWidth - 16);
    const below = window.innerHeight - rect.bottom - 12;
    const height = Math.min(340, window.innerHeight - 24);
    position = {
      left: Math.min(rect.left, window.innerWidth - width - 8),
      top: below >= 240 ? rect.bottom + 6 : Math.max(8, rect.top - height - 6),
      width,
      maxHeight: height,
    };
    expanded = true;
    searchQuery = "";

    // Set highlighted index to selected option
    const selIndex = filteredLanguages.findIndex((l) => l.code === value);
    highlightedIndex = selIndex >= 0 ? selIndex : 0;

    await tick();
    searchInput?.focus();
    scrollToHighlighted();
  }

  function toggle() {
    if (expanded) close();
    else open();
  }

  function selectLanguage(lang: Language) {
    value = lang.code;
    onchange?.(lang.code);
    close();
    trigger?.focus();
  }

  function scrollToHighlighted() {
    if (dropdown) {
      const item = dropdown.querySelector(`[data-lang-index="${highlightedIndex}"]`);
      if (item instanceof HTMLElement) {
        item.scrollIntoView({ block: "nearest" });
      }
    }
  }

  function handleTriggerKey(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " " || event.key === "ArrowDown") {
      event.preventDefault();
      open();
    }
  }

  function handleSearchKey(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      trigger?.focus();
      return;
    }
    if (event.key === "Tab") {
      close();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (highlightedIndex < filteredLanguages.length - 1) {
        highlightedIndex++;
        scrollToHighlighted();
      }
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (highlightedIndex > 0) {
        highlightedIndex--;
        scrollToHighlighted();
      }
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const current = filteredLanguages[highlightedIndex];
      if (current) selectLanguage(current);
    }
  }
</script>

<svelte:window
  onpointerdown={(event) => {
    if (root && !root.contains(event.target as Node) && dropdown && !dropdown.contains(event.target as Node)) {
      close();
    }
  }}
  onresize={close}
/>

<div class="language-select-container" bind:this={root}>
  <button
    type="button"
    bind:this={trigger}
    class="select-button language-trigger"
    aria-label={label}
    aria-haspopup="listbox"
    aria-expanded={expanded}
    {disabled}
    onclick={toggle}
    onkeydown={handleTriggerKey}
  >
    <span class="language-label-text">{displayLabel}</span>
    <ChevronDown size={15} class={expanded ? "rotated" : ""} />
  </button>

  {#if expanded}
    <div
      bind:this={dropdown}
      class="select-options language-dropdown"
      role="listbox"
      aria-label={label}
      style="left:{position.left}px;top:{position.top}px;width:{position.width}px;max-height:{position.maxHeight}px"
    >
      <div class="language-search-wrapper">
        <Search size={14} class="language-search-icon" />
        <input
          bind:this={searchInput}
          type="text"
          class="language-search-input"
          bind:value={searchQuery}
          {placeholder}
          onkeydown={handleSearchKey}
          autocomplete="off"
          spellcheck="false"
        />
      </div>

      <div class="language-list">
        {#if filteredLanguages.length === 0}
          <div class="language-empty">Nessuna lingua trovata</div>
        {:else}
          {#each filteredLanguages as lang, index}
            <button
              type="button"
              role="option"
              data-lang-index={index}
              aria-selected={value === lang.code}
              class="language-option"
              class:highlighted={highlightedIndex === index}
              onmouseenter={() => (highlightedIndex = index)}
              onclick={() => selectLanguage(lang)}
            >
              <span class="language-option-flag">{lang.flag}</span>
              <span class="language-option-name">
                <strong class="language-native-name">{lang.name}</strong>
                {#if lang.nameEn && lang.nameEn !== lang.name}
                  <span class="language-en-name">{lang.nameEn}</span>
                {/if}
              </span>
              <span class="language-code-pill">{lang.code}</span>
              {#if value === lang.code}
                <Check size={14} class="language-check-icon" />
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .language-select-container {
    min-width: 0;
    position: relative;
    width: 100%;
  }

  .language-trigger {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .language-label-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .language-dropdown {
    display: flex;
    flex-direction: column;
    padding: 6px;
    z-index: 2000;
  }

  .language-search-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    margin-bottom: 6px;
    padding: 0 4px;
  }

  :global(.language-search-icon) {
    position: absolute;
    left: 14px;
    color: var(--muted, #94a3b8);
    pointer-events: none;
  }

  .language-search-input {
    width: 100%;
    height: 36px;
    padding: 6px 12px 6px 34px;
    background: var(--bg, #111827);
    border: 1px solid var(--border, #374151);
    border-radius: 7px;
    color: var(--text, #f9fafb);
    font-size: 13px;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .language-search-input:focus {
    border-color: var(--accent, #818cf8);
    box-shadow: 0 0 0 2px rgba(129, 140, 248, 0.2);
  }

  .language-list {
    overflow-y: auto;
    max-height: 260px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .language-option {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border: none;
    background: transparent;
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    font-size: 13px;
    color: inherit;
    transition: background-color 0.1s ease;
  }

  .language-option:hover,
  .language-option.highlighted {
    background: rgba(129, 140, 248, 0.12);
  }

  .language-option[aria-selected="true"] {
    background: rgba(129, 140, 248, 0.2);
    color: var(--accent, #818cf8);
    font-weight: 500;
  }

  .language-option-flag {
    font-size: 16px;
    flex-shrink: 0;
    line-height: 1;
  }

  .language-option-name {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
    overflow: hidden;
  }

  .language-native-name {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .language-en-name {
    font-size: 11px;
    color: var(--muted, #94a3b8);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .language-code-pill {
    font-size: 10px;
    font-family: monospace;
    text-transform: uppercase;
    background: rgba(255, 255, 255, 0.08);
    padding: 2px 5px;
    border-radius: 4px;
    color: var(--muted, #94a3b8);
    flex-shrink: 0;
  }

  :global(.language-check-icon) {
    color: var(--accent, #818cf8);
    flex-shrink: 0;
  }

  .language-empty {
    padding: 16px 12px;
    text-align: center;
    color: var(--muted, #94a3b8);
    font-size: 13px;
  }
</style>
