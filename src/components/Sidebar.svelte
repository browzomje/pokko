<script lang="ts">
  import {
    Search,
    Download,
    Globe,
    FlaskConical,
    Keyboard,
    Sun,
    Moon,
    Settings,
    Languages,
    Check,
  } from "lucide-svelte";
  import {
    t,
    currentLanguage,
    setLanguage,
    availableUILanguages,
    type UILang,
  } from "../lib/i18n";

  let {
    active,
    navigate,
    theme,
    onTheme,
    onShortcuts,
    onLab,
    downloadStats = { isDownloading: false, progress: 0, current: 0, total: 0 },
  }: {
    active: string;
    navigate: (section: string) => void;
    theme: string;
    onTheme: () => void;
    onShortcuts: () => void;
    onLab: () => void;
    downloadStats?: {
      isDownloading: boolean;
      progress: number;
      current: number;
      total: number;
    };
  } = $props();

  let langMenuOpen = $state(false);
  let langContainer = $state<HTMLDivElement>();

  function toggleLanguageMenu() {
    langMenuOpen = !langMenuOpen;
  }

  function chooseLanguage(code: UILang) {
    setLanguage(code);
    langMenuOpen = false;
  }
</script>

<svelte:window
  onpointerdown={(event) => {
    if (
      langMenuOpen &&
      langContainer &&
      !langContainer.contains(event.target as Node)
    ) {
      langMenuOpen = false;
    }
  }}
  onkeydown={(event) => {
    if (event.key === "Escape" && langMenuOpen) {
      langMenuOpen = false;
    }
  }}
/>

<aside class="sidebar">
  <div class="pokko-brand">
    <img src="/pokko.svg" alt="" width="32" height="32" />
    <strong>pokko</strong>
  </div>

  <div class="section-label">{t("nav.workspace")}</div>
  <button class:chosen={active === "search"} onclick={() => navigate("search")}>
    <Search size={18} />
    {t("nav.explore")} <kbd>Ctrl K</kbd>
  </button>
  <button class:chosen={active === "queue"} onclick={() => navigate("queue")}>
    <div class="sidebar-icon-wrapper">
      {#if downloadStats?.isDownloading}
        <svg
          class="progress-ring"
          viewBox="0 0 24 24"
          width="24"
          height="24"
          aria-hidden="true"
        >
          <circle class="ring-bg" cx="12" cy="12" r="9" stroke-width="2.5" />
          <circle
            class="ring-bar"
            cx="12"
            cy="12"
            r="9"
            stroke-width="2.5"
            stroke-dasharray={2 * Math.PI * 9}
            stroke-dashoffset={2 *
              Math.PI *
              9 *
              (1 - Math.max(0, Math.min(100, downloadStats.progress)) / 100)}
          />
        </svg>
        <Download size={13} class="icon-downloading" />
      {:else}
        <Download size={18} />
      {/if}
    </div>
    <span>{t("nav.downloads")}</span>
    {#if downloadStats?.isDownloading}
      <span
        class="download-badge"
        title={`${downloadStats.current}/${downloadStats.total} · ${downloadStats.progress}%`}
      >
        {downloadStats.current}/{downloadStats.total}
      </span>
    {:else}
      <kbd>Ctrl J</kbd>
    {/if}
  </button>
  <button
    class:chosen={active === "sources"}
    onclick={() => navigate("sources")}
  >
    <Globe size={18} />
    {t("nav.sources")} <kbd>Ctrl ⇧ F</kbd>
  </button>
  <button class:chosen={active === "lab"} onclick={onLab}>
    <FlaskConical size={18} />
    {t("nav.lab")} <kbd>Ctrl ⇧ L</kbd>
  </button>

  <div class="sidebar-bottom">
    <button
      class:chosen={active === "settings"}
      onclick={() => navigate("settings")}
    >
      <Settings size={18} />
      {t("nav.settings")} <kbd>Ctrl ,</kbd>
    </button>

    <div class="sidebar-divider"></div>

    <div class="section-label">{t("nav.quickActions")}</div>
    <div class="quick-actions-row">
      <button
        class="quick-action-btn"
        onclick={onShortcuts}
        title="{t('nav.shortcuts')} (Ctrl+Shift+H)"
      >
        <Keyboard size={17} />
      </button>
      <button
        class="quick-action-btn"
        onclick={onTheme}
        title="{t('nav.theme')} (Ctrl+Shift+T)"
      >
        {#if theme === "dark"}<Sun size={17} />{:else}<Moon size={17} />{/if}
      </button>

      <div class="lang-dropdown-wrapper" bind:this={langContainer}>
        <button
          type="button"
          class="quick-action-btn lang-toggle-btn"
          class:active={langMenuOpen}
          onclick={toggleLanguageMenu}
          title="{t('nav.language')}: {currentLanguage().toUpperCase()}"
          aria-haspopup="listbox"
          aria-expanded={langMenuOpen}
        >
          <Languages size={15} />
          <span>{currentLanguage().toUpperCase()}</span>
        </button>

        {#if langMenuOpen}
          <div
            class="lang-popup-menu"
            role="listbox"
            aria-label={t("settings.uiLanguage")}
          >
            <div class="lang-popup-header">{t("settings.uiLanguage")}</div>
            <div class="lang-popup-list">
              {#each availableUILanguages as l}
                <button
                  type="button"
                  role="option"
                  aria-selected={currentLanguage() === l.code}
                  class="lang-popup-item"
                  class:chosen={currentLanguage() === l.code}
                  onclick={() => chooseLanguage(l.code)}
                >
                  <span class="lang-item-flag">{l.flag}</span>
                  <span class="lang-item-label">{l.label}</span>
                  <span class="lang-item-code">{l.code.toUpperCase()}</span>
                  {#if currentLanguage() === l.code}
                    <Check size={14} class="lang-item-check" />
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
</aside>

<style>
  .pokko-brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px 20px;
    font-size: 22px;
    letter-spacing: -0.5px;
  }
  .sidebar-divider {
    height: 1px;
    background: var(--border);
    margin: 12px 12px 14px;
    opacity: 0.6;
  }
  .quick-actions-row {
    display: flex;
    gap: 8px;
    padding: 0 10px 10px;
  }
  .quick-action-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 9px 8px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--panel);
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .quick-action-btn:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .lang-dropdown-wrapper {
    position: relative;
    flex: 1.2;
    display: flex;
  }
  .lang-dropdown-wrapper .lang-toggle-btn {
    width: 100%;
  }
  .lang-toggle-btn.active {
    border-color: var(--accent, #818cf8);
    color: var(--text, #f9fafb);
    background: var(--surface, #1e293b);
  }
  .lang-popup-menu {
    position: absolute;
    bottom: calc(100% + 8px);
    right: 0;
    width: 220px;
    background: var(--panel, #1e293b);
    border: 1px solid var(--border, #334155);
    border-radius: 8px;
    box-shadow:
      0 12px 28px rgba(0, 0, 0, 0.5),
      0 4px 10px rgba(0, 0, 0, 0.3);
    z-index: 100;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: popupFadeIn 0.15s cubic-bezier(0.16, 1, 0.3, 1);
  }
  @keyframes popupFadeIn {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .lang-popup-header {
    padding: 8px 12px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--muted, #94a3b8);
    border-bottom: 1px solid var(--border, #334155);
    background: rgba(0, 0, 0, 0.15);
  }
  .lang-popup-list {
    max-height: 270px;
    overflow-y: auto;
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .lang-popup-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--text, #e2e8f0);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    text-align: left;
    transition: background 0.12s ease;
  }
  .lang-popup-item:hover {
    background: rgba(255, 255, 255, 0.08);
  }
  .lang-popup-item.chosen {
    background: rgba(99, 102, 241, 0.15);
    color: var(--accent, #818cf8);
    font-weight: 600;
  }
  .lang-item-flag {
    font-size: 14px;
    flex-shrink: 0;
  }
  .lang-item-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lang-item-code {
    font-size: 10px;
    color: var(--muted, #94a3b8);
    font-family: monospace;
    flex-shrink: 0;
  }
  :global(.lang-item-check) {
    color: var(--accent, #818cf8);
    flex-shrink: 0;
  }
  .sidebar-icon-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    margin-right: 2px;
    flex-shrink: 0;
  }
  .progress-ring {
    position: absolute;
    top: -2px;
    left: -2px;
    width: 24px;
    height: 24px;
    transform: rotate(-90deg);
    pointer-events: none;
  }
  .ring-bg {
    fill: none;
    stroke: color-mix(in srgb, var(--accent, #6366f1) 20%, transparent);
  }
  .ring-bar {
    fill: none;
    stroke: var(--accent, #6366f1);
    stroke-linecap: round;
    transition: stroke-dashoffset 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }
  .sidebar-icon-wrapper :global(.icon-downloading) {
    color: var(--accent, #6366f1);
  }
  .download-badge {
    margin-left: auto;
    font-size: 10px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    padding: 1px 7px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent, #6366f1) 18%, transparent);
    color: var(--accent, #6366f1);
    border: 1px solid
      color-mix(in srgb, var(--accent, #6366f1) 35%, transparent);
  }
</style>
