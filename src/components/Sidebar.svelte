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
  } from "lucide-svelte";
  import { t, currentLanguage, setLanguage } from "../lib/i18n";

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

  function toggleLanguage() {
    setLanguage(currentLanguage() === "it" ? "en" : "it");
  }
</script>

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
      <button class="quick-action-btn lang-toggle-btn" onclick={toggleLanguage} title="{t('nav.language')}: {currentLanguage().toUpperCase()}">
        <Languages size={15} />
        <span>{currentLanguage().toUpperCase()}</span>
      </button>
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
  .lang-toggle-btn {
    flex: 1.2;
  }
</style>
