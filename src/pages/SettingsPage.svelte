<script lang="ts">
  import { onMount } from "svelte";
  import {
    Pause,
    FolderOpen,
    Globe,
    Languages,
    FileText,
    Copy,
    Trash2,
    RotateCcw,
    Pencil,
    Zap,
  } from "lucide-svelte";
  import { api } from "../lib/api";
  import Select from "../Select.svelte";
  import { getFormats } from "../lib/presentation";
  import {
    t,
    currentLanguage,
    setLanguage,
    availableUILanguages,
    type UILang,
  } from "../lib/i18n";
  import type { Settings } from "../types";

  let {
    settings = $bindable(),
    saveError,
    saveStatus: _saveStatus,
    chooseFolder,
  }: {
    settings: Settings;
    saveError: string;
    saveStatus: string;
    chooseFolder: () => void;
  } = $props();

  let logPath = $state("");
  let recentLogs = $state<string[]>([]);
  let logViewerOpen = $state(false);

  const UA_FIREFOX =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:135.0) Gecko/20100101 Firefox/135.0";
  const UA_SAFARI =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.3 Safari/605.1.15";
  const UA_CHROMIUM_LINUX =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

  let uaPresets = $derived([
    { value: "", label: t("settings.uaDefaultChrome") },
    { value: UA_FIREFOX, label: "Firefox Desktop" },
    { value: UA_SAFARI, label: "Safari macOS" },
    { value: UA_CHROMIUM_LINUX, label: "Chromium Linux" },
  ]);

  let customUaMode = $state(
    Boolean(
      settings.user_agent &&
        !["", UA_FIREFOX, UA_SAFARI, UA_CHROMIUM_LINUX].includes(
          settings.user_agent,
        ),
    ),
  );

  async function loadLogPath() {
    try {
      logPath = await api<string>("get_log_path");
    } catch {}
  }

  async function loadRecentLogs() {
    try {
      recentLogs = await api<string[]>("read_recent_logs", { lines: 100 });
    } catch {}
  }

  async function clearLogs() {
    try {
      await api("clear_logs");
      recentLogs = [];
    } catch {}
  }

  async function copyLogPath() {
    if (logPath && typeof navigator !== "undefined") {
      await navigator.clipboard.writeText(logPath);
    }
  }

  onMount(() => {
    void loadLogPath();
    void loadRecentLogs();
  });
</script>

<div class="page-heading">
  <div>
    <h1>{t("settings.title")}</h1>
    {#if saveError}
      <div class="save-status failure" role="status">
        <span>{saveError}</span>
      </div>
    {/if}
  </div>
</div>

<div class="settings-grid">
  <!-- Column 1: Destinazione, Lingua & Rete -->
  <div class="settings-column">
    <div class="settings-card">
      <h2><FolderOpen size={19} /> {t("settings.destination")}</h2>

      <label for="output">{t("settings.downloadFolder")}</label>
      <div class="input-row">
        <input
          id="output"
          bind:value={settings.output}
          placeholder={t("settings.chooseFolderPlaceholder")}
        />
        <button onclick={chooseFolder}
          ><FolderOpen size={16} /> {t("settings.browse")}</button
        >
      </div>

      <div class="source-field">
        <span>{t("settings.defaultFormat")}</span>
        <Select
          label={t("settings.defaultFormat")}
          bind:value={settings.export_format}
          options={getFormats()}
        />
      </div>
      <small>
        {t("settings.formatHint")}
      </small>
    </div>

    <div class="settings-card">
      <h2><Languages size={19} /> {t("settings.uiLanguage")}</h2>

      <div class="source-field">
        <Select
          label={t("settings.uiLanguage")}
          value={currentLanguage()}
          options={availableUILanguages.map((l) => ({
            value: l.code,
            label: `${l.flag} ${l.label}`,
          }))}
          onchange={(val) => setLanguage(val as UILang)}
        />
      </div>
    </div>

    <div class="settings-card">
      <h2><Globe size={19} /> {t("settings.userAgent")}</h2>
      {#if !customUaMode}
        <div class="input-row">
          <Select
            label={t("settings.userAgent")}
            bind:value={settings.user_agent}
            options={uaPresets}
          />
          <button
            type="button"
            class="icon"
            title={t("settings.uaCustomToggle")}
            onclick={() => (customUaMode = true)}
          >
            <Pencil size={16} />
          </button>
        </div>
      {:else}
        <div class="input-row">
          <input
            id="user-agent"
            bind:value={settings.user_agent}
            placeholder="{t('settings.default')} (Chrome Desktop)"
            spellcheck="false"
          />
          <button
            type="button"
            class="icon"
            title={t("settings.uaPresetToggle")}
            onclick={() => {
              customUaMode = false;
              if (
                settings.user_agent &&
                !["", UA_FIREFOX, UA_SAFARI, UA_CHROMIUM_LINUX].includes(
                  settings.user_agent,
                )
              ) {
                settings.user_agent = "";
              }
            }}
          >
            <RotateCcw size={16} />
          </button>
        </div>
      {/if}
      <small>{t("settings.networkHint")}</small>
    </div>
  </div>

  <!-- Column 2: Ritmo delle richieste + Diagnostica -->
  <div class="settings-column">
    <div class="settings-card">
      <h2><Pause size={19} /> {t("settings.requestRate")}</h2>
      <label for="delay">
        {t("settings.interval", {
          sec: settings.delay_ms / 1000,
          unit:
            settings.delay_ms === 1000
              ? t("settings.second")
              : t("settings.seconds"),
        })}
      </label>
      <input
        id="delay"
        type="range"
        min="500"
        max="10000"
        step="500"
        bind:value={settings.delay_ms}
      />
      <button onclick={() => (settings.delay_ms = 500)}
        ><Zap size={15} /> {t("settings.fastProfile")}</button
      >
      <small>{t("settings.rateHint")}</small>
    </div>

    <div class="settings-card">
      <h2><FileText size={19} /> {t("settings.diagnosticsTitle")}</h2>

      <label for="log-path-input">{t("settings.logPath")}</label>
      <div class="input-row">
        <input
          id="log-path-input"
          readonly
          value={logPath || t("settings.loadingPreferences")}
          spellcheck="false"
        />
        <button onclick={copyLogPath} title={t("settings.copy")}>
          <Copy size={15} /> {t("settings.copy")}
        </button>
      </div>

      <div class="log-actions-row">
        <button onclick={loadRecentLogs}>
          <RotateCcw size={14} /> {t("settings.refreshLog")}
        </button>
        <button class="danger" onclick={clearLogs}>
          <Trash2 size={14} /> {t("settings.clearLog")}
        </button>
      </div>

      <details class="log-details" bind:open={logViewerOpen}>
        <summary>{t("settings.viewLogs", { count: recentLogs.length })}</summary
        >
        <pre class="log-viewer">{recentLogs.length
            ? recentLogs.join("\n")
            : t("settings.noLogs")}</pre>
      </details>
    </div>
  </div>
</div>

<style>
  .input-row :global(.modern-select) {
    flex: 1;
    min-width: 0;
  }
</style>
