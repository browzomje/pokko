<script lang="ts">
  import { Plus, Trash2, Download, Upload, Github, FolderOpen, FileCode, X } from "lucide-svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { api, message as msg } from "../lib/api";
  import { flag } from "../lib/presentation";
  import { t } from "../lib/i18n";
  import type { Source } from "../types";

  let {
    sortedSources,
    saveError,
    saveStatus: _saveStatus,
    restoreSources,
    addSource,
    editSource,
    openLab,
    removeSource,
  }: {
    sortedSources: Source[];
    saveError: string;
    saveStatus: string;
    restoreSources: () => void;
    addSource: () => void;
    editSource: (source: Source) => void;
    openLab: (source: Source) => void;
    removeSource: (id: string) => void;
  } = $props();

  let importModalOpen = $state(false);
  let githubRepoInput = $state("");
  let actionStatus = $state("");
  let actionError = $state("");

  function checkArchiveExt(filename: string): boolean {
    const lower = filename.toLowerCase();
    return [".zip", ".rar", ".7z", ".tar", ".gz", ".cbz", ".cbr"].some((ext) =>
      lower.endsWith(ext),
    );
  }

  async function importSingleFile() {
    actionError = "";
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Connettore JSON", extensions: ["json"] }],
      });
      if (typeof selected !== "string") return;
      if (checkArchiveExt(selected)) {
        actionError = "I file archivio (ZIP, RAR, TAR, CBZ) non sono consentiti per motivi di sicurezza. Importa solo file connettore .json.";
        return;
      }
      const imported = await api<Source>("sources_import_file", { path: selected });
      const index = sortedSources.findIndex((s) => s.id === imported.id);
      if (index >= 0) sortedSources[index] = imported;
      else sortedSources.push(imported);
      actionStatus = `Connettore '${imported.name}' importato con successo!`;
      importModalOpen = false;
    } catch (e) {
      actionError = msg(e);
    }
  }

  async function importFolder() {
    actionError = "";
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Seleziona cartella con file connettori .json",
      });
      if (typeof selected !== "string") return;
      const importedList = await api<Source[]>("sources_import_folder", { path: selected });
      for (const imported of importedList) {
        const index = sortedSources.findIndex((s) => s.id === imported.id);
        if (index >= 0) sortedSources[index] = imported;
        else sortedSources.push(imported);
      }
      actionStatus = `Importati ${importedList.length} connettori con successo!`;
      importModalOpen = false;
    } catch (e) {
      actionError = msg(e);
    }
  }

  async function importFromGithub() {
    actionError = "";
    if (!githubRepoInput.trim()) {
      actionError = "Inserisci l'URL o il percorso repository (es. 'owner/repo').";
      return;
    }
    if (checkArchiveExt(githubRepoInput)) {
      actionError = "I file archivio non sono consentiti per motivi di sicurezza.";
      return;
    }
    try {
      actionStatus = "Connessione a GitHub in corso…";
      const importedList = await api<Source[]>("sources_import_github", { repo: githubRepoInput.trim() });
      for (const imported of importedList) {
        const index = sortedSources.findIndex((s) => s.id === imported.id);
        if (index >= 0) sortedSources[index] = imported;
        else sortedSources.push(imported);
      }
      actionStatus = `Importati ${importedList.length} connettori da GitHub con successo!`;
      importModalOpen = false;
      githubRepoInput = "";
    } catch (e) {
      actionError = msg(e);
    }
  }

  async function exportSource(source: Source) {
    actionError = "";
    try {
      const filename = `${(source.name || "connettore").toLowerCase().replace(/[^a-z0-9]/g, "-")}.json`;
      const selected = await save({
        defaultPath: filename,
        filters: [{ name: "Connettore JSON", extensions: ["json"] }],
      });
      if (!selected) return;
      if (checkArchiveExt(selected)) {
        actionError = "È consentito esportare solo in formato .json.";
        return;
      }
      await api("sources_export_file", { source, path: selected });
      actionStatus = `Connettore '${source.name}' esportato con successo!`;
    } catch (e) {
      actionError = msg(e);
    }
  }
</script>

<div class="page-heading">
  <div>
    <h1>{t("sources.title")}</h1>
    {#if saveError || actionError}
      <div class="save-status failure" role="status">
        <span>{saveError || actionError}</span>
      </div>
    {:else if actionStatus}
      <div class="save-status" role="status">
        <span>{actionStatus}</span>
      </div>
    {/if}
  </div>
  <div class="page-heading-actions">
    <button onclick={() => (importModalOpen = true)}>
      <Upload size={15} /> Importa connettori
    </button>
    <button onclick={restoreSources}>{t("sources.restore")}</button>
    <button class="primary" onclick={addSource}>
      <Plus size={16} /> {t("sources.add")}
    </button>
  </div>
</div>

{#if importModalOpen}
  <div class="modal-backdrop" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) importModalOpen = false; }}>
    <div class="modal-dialog" role="dialog" aria-modal="true" tabindex="-1">
      <div class="modal-header">
        <h3>Importa connettori</h3>
        <button class="icon" onclick={() => (importModalOpen = false)}><X size={18} /></button>
      </div>

      <div class="modal-body">
        <p class="import-help">
          Importa connettori configurati per nuovi siti. Sono consentiti solo file <code>.json</code> con URL sicuri (HTTPS). Gli archivi ZIP/RAR/CBZ sono bloccati per sicurezza.
        </p>

        <div class="import-options">
          <button class="import-option-btn" onclick={importSingleFile}>
            <FileCode size={20} />
            <div>
              <strong>File singolo .json</strong>
              <small>Carica un connettore salvato in locale</small>
            </div>
          </button>

          <button class="import-option-btn" onclick={importFolder}>
            <FolderOpen size={20} />
            <div>
              <strong>Cartella locale</strong>
              <small>Scansiona una cartella con più file .json</small>
            </div>
          </button>
        </div>

        <div class="github-import-box">
          <label>
            <span><Github size={15} /> Importa da repository GitHub</span>
            <div class="input-row">
              <input
                bind:value={githubRepoInput}
                placeholder="es. browzomje/pokko o URL completo"
                spellcheck="false"
              />
              <button class="primary" onclick={importFromGithub}>Scarica</button>
            </div>
          </label>
        </div>
      </div>
    </div>
  </div>
{/if}

<div class="sources-list">
  {#each sortedSources as source (source.id)}
    <article class="source-card">
      <div class="source-header">
        <label class="checkbox">
          <input type="checkbox" bind:checked={source.enabled} />
          <strong>{source.name}</strong>
        </label>
        <span class="pill">
          {flag(source.language)}
          {source.adapter === "mangadex"
            ? "API"
            : source.adapter === "readcomics"
              ? "Comics"
              : source.adapter === "mangaworld"
                ? "MangaWorld"
                : "HTML generico"}
        </span>
        <button
          class="icon danger"
          title={`${t("sources.remove")} ${source.name}`}
          onclick={() => removeSource(source.id)}
        >
          <Trash2 size={18} />
        </button>
      </div>
      <p class="source-note">{source.note}</p>
      <div class="source-actions">
        {#if source.adapter === "generic" && !["mw", "md", "mf", "mangaread", "readcomics"].includes(source.id)}
          <button onclick={() => editSource(source)}>{t("sources.edit")}</button>
          <button onclick={() => openLab(source)}>{t("sources.openLab")}</button>
          <button onclick={() => exportSource(source)} title="Esporta file .json">
            <Download size={14} /> Esporta
          </button>
        {:else}
          <button onclick={() => editSource(source)}>{t("sources.configure")}</button>
          <button onclick={() => exportSource(source)} title="Esporta file .json">
            <Download size={14} /> Esporta
          </button>
        {/if}
      </div>
    </article>
  {/each}
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    padding: 16px;
  }
  .modal-dialog {
    background: var(--panel, #1f2937);
    border: 1px solid var(--border, #374151);
    border-radius: 12px;
    width: 100%;
    max-width: 520px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border, #374151);
  }
  .modal-header h3 {
    margin: 0;
    font-size: 16px;
  }
  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .import-help {
    font-size: 12px;
    color: var(--muted, #94a3b8);
    line-height: 1.5;
    margin: 0;
  }
  .import-options {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .import-option-btn {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px;
    border: 1px solid var(--border, #374151);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.03);
    text-align: left;
    cursor: pointer;
  }
  .import-option-btn div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .import-option-btn small {
    color: var(--muted, #94a3b8);
    font-size: 11px;
  }
  .github-import-box {
    padding-top: 10px;
    border-top: 1px solid var(--border, #374151);
  }
  .github-import-box label > span {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 500;
    margin-bottom: 8px;
  }
</style>
