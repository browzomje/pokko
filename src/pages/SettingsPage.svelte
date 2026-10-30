<script lang="ts">
  import { onMount } from "svelte";
  import { Pause, FolderOpen, Globe, Languages, FileText, Copy, Trash2, RotateCcw } from "lucide-svelte";
  import { api } from "../lib/api";
  import Select from "../Select.svelte";
  import { formats } from "../lib/presentation";
  import { t, currentLanguage, setLanguage, availableUILanguages, type UILang } from "../lib/i18n";
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
  <!-- Column 1: Destinazione & Lingua -->
  <div class="settings-column">
    <div class="settings-card">
      <h2><FolderOpen size={19} /> {t("settings.destination")}</h2>
      <p>{t("settings.destinationDesc")}</p>
      
      <label for="output">{t("settings.downloadFolder")}</label>
      <div class="input-row">
        <input
          id="output"
          bind:value={settings.output}
          placeholder="Scegli una cartella…"
        />
        <button onclick={chooseFolder}><FolderOpen size={16} /> {t("settings.browse")}</button>
      </div>

      <div class="source-field">
        <span>{t("settings.defaultFormat")}</span>
        <Select
          label={t("settings.defaultFormat")}
          bind:value={settings.export_format}
          options={formats}
        />
      </div>
      <small>
        {t("settings.formatHint")}
      </small>
    </div>

    <div class="settings-card">
      <h2><Languages size={19} /> {t("settings.uiLanguage")}</h2>
      <p>{t("settings.uiLanguageDesc")}</p>

      <div class="source-field">
        <Select
          label={t("settings.uiLanguage")}
          value={currentLanguage()}
          options={availableUILanguages.map(l => ({ value: l.code, label: `${l.flag} ${l.label}` }))}
          onchange={(val) => setLanguage(val as UILang)}
        />
      </div>
    </div>
  </div>

  <!-- Column 2: Ritmo delle richieste + Rete -->
  <div class="settings-column">
    <div class="settings-card">
      <h2><Pause size={19} /> {t("settings.requestRate")}</h2>
      <p>{t("settings.requestRateDesc")}</p>
      <label for="delay">
        {t("settings.interval", {
          sec: settings.delay_ms / 1000,
          unit: settings.delay_ms === 1000 ? t("settings.second") : t("settings.seconds")
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
      <button onclick={() => (settings.delay_ms = 500)}>{t("settings.fastProfile")}</button>
      <small>{t("settings.rateHint")}</small>
    </div>

    <div class="settings-card">
      <h2><Globe size={19} /> {t("settings.network")}</h2>
      <p>{t("settings.networkDesc")}</p>
      <label for="user-agent">{t("settings.userAgent")}</label>
      <div class="input-row">
        <input
          id="user-agent"
          bind:value={settings.user_agent}
          placeholder="{t('settings.default')} (Desktop Chrome)"
          spellcheck="false"
        />
        <button onclick={() => (settings.user_agent = "")} title="{t('settings.default')}">
          {t("settings.default")}
        </button>
      </div>
      <small>{t("settings.networkHint")}</small>
    </div>

    <div class="settings-card">
      <h2><FileText size={19} /> Diagnostica e Log</h2>
      <p>File di log strutturato JSONL per la diagnosi di connettori, rete ed errori.</p>
      
      <label for="log-path-input">Percorso file di log</label>
      <div class="input-row">
        <input id="log-path-input" readonly value={logPath || "Caricamento…"} spellcheck="false" />
        <button onclick={copyLogPath} title="Copia percorso negli appunti">
          <Copy size={15} /> Copia
        </button>
      </div>

      <div class="log-actions-row">
        <button onclick={loadRecentLogs}>
          <RotateCcw size={14} /> Aggiorna log
        </button>
        <button class="danger" onclick={clearLogs}>
          <Trash2 size={14} /> Svuota log
        </button>
      </div>

      <details class="log-details" bind:open={logViewerOpen}>
        <summary>Visualizza log recenti ({recentLogs.length} righe)</summary>
        <pre class="log-viewer">{recentLogs.length ? recentLogs.join("\n") : "Nessun log registrato."}</pre>
      </details>
    </div>
  </div>
</div>
