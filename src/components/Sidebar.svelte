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
  }: {
    active: string;
    navigate: (section: string) => void;
    theme: string;
    onTheme: () => void;
    onShortcuts: () => void;
    onLab: () => void;
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
    if (langMenuOpen && langContainer && !langContainer.contains(event.target as Node)) {
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
    <Search size={18} /> {t("nav.explore")} <kbd>Ctrl K</kbd>
  </button>
  <button class:chosen={active === "queue"} onclick={() => navigate("queue")}>
    <Download size={18} /> {t("nav.downloads")} <kbd>Ctrl J</kbd>
  </button>
  <button class:chosen={active === "sources"} onclick={() => navigate("sources")}>
    <Globe size={18} /> {t("nav.sources")} <kbd>Ctrl ⇧ F</kbd>
  </button>
  <button class:chosen={active === "lab"} onclick={onLab}>
    <FlaskConical size={18} /> {t("nav.lab")} <kbd>Ctrl ⇧ L</kbd>
  </button>

  <div class="sidebar-bottom">
    <button class:chosen={active === "settings"} onclick={() => navigate("settings")}>
      <Settings size={18} /> {t("nav.settings")} <kbd>Ctrl ,</kbd>
    </button>

    <div class="sidebar-divider"></div>

    <div class="section-label">{t("nav.quickActions")}</div>
    <div class="quick-actions-row">
      <button class="quick-action-btn" onclick={onShortcuts} title="{t('nav.shortcuts')} (Ctrl+Shift+H)">
        <Keyboard size={17} />
      </button>
      <button class="quick-action-btn" onclick={onTheme} title="{t('nav.theme')} (Ctrl+Shift+T)">
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
          <div class="lang-popup-menu" role="listbox" aria-label={t("settings.uiLanguage")}>
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
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.5), 0 4px 10px rgba(0, 0, 0, 0.3);
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
</style>
