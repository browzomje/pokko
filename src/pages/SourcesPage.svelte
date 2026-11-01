<script lang="ts">
  import {
    Trash2,
    Download,
    Upload,
    Github,
    FolderOpen,
    FolderDown,
    FileCode,
    X,
    Pencil,
    FlaskConical,
    ShieldCheck,
    Plus,
    RotateCcw,
  } from "lucide-svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { api, message as msg } from "../lib/api";
  import { flag, formatSourceNote } from "../lib/presentation";
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
    addSource?: () => void;
    editSource: (source: Source) => void;
    openLab: (source: Source) => void;
    removeSource: (id: string) => void;
  } = $props();

  let importModalOpen = $state(false);
  let githubRepoInput = $state("");
  let actionStatus = $state("");
  let actionError = $state("");

  const builtInIds = ["mw", "mf", "mangaread", "mangapill", "mangatown"];

  let officialSources = $derived(
    sortedSources.filter(
      (s) => s.origin === "official" || builtInIds.includes(s.id),
    ),
  );

  let createdSources = $derived(
    sortedSources.filter(
      (s) =>
        !builtInIds.includes(s.id) &&
        (s.origin === "created" ||
          (!s.origin && !s.id.startsWith("imported-"))),
    ),
  );

  let importedSources = $derived(
    sortedSources.filter(
      (s) =>
        !builtInIds.includes(s.id) &&
        (s.origin === "imported" || s.id.startsWith("imported-")),
    ),
  );

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
        actionError =
          "I file archivio (ZIP, RAR, TAR, CBZ) non sono consentiti per motivi di sicurezza. Importa solo file connettore .json.";
        return;
      }
      const imported = await api<Source>("sources_import_file", {
        path: selected,
      });
      imported.origin = "imported";
      imported.enabled = true;
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
      const importedList = await api<Source[]>("sources_import_folder", {
        path: selected,
      });
      for (const imported of importedList) {
        imported.origin = "imported";
        imported.enabled = true;
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
      actionError =
        "Inserisci l'URL o il percorso repository (es. 'owner/repo').";
      return;
    }
    if (checkArchiveExt(githubRepoInput)) {
      actionError =
        "I file archivio non sono consentiti per motivi di sicurezza.";
      return;
    }
    try {
      actionStatus = "Connessione a GitHub in corso…";
      const importedList = await api<Source[]>("sources_import_github", {
        repo: githubRepoInput.trim(),
      });
      for (const imported of importedList) {
        imported.origin = "imported";
        imported.enabled = true;
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

{#snippet sourceCard(source: Source)}
  <article class="source-card">
    <div class="source-header">
      <label class="checkbox">
        <input type="checkbox" bind:checked={source.enabled} />
        <strong>{source.name}</strong>
      </label>
      <span class="pill">
        {flag(source.language)}
        {t("sources.genericHtml")}
      </span>
      <button
        class="icon danger"
        title={`${t("sources.remove")} ${source.name}`}
        onclick={() => removeSource(source.id)}
      >
        <Trash2 size={18} />
      </button>
    </div>
    <p class="source-note">{formatSourceNote(source.note, source.id)}</p>
    <div class="source-actions">
      <button onclick={() => editSource(source)}>
        <Pencil size={14} /> {t("sources.edit")}
      </button>
      <button onclick={() => openLab(source)}>
        <FlaskConical size={14} /> {t("sources.openLab")}
      </button>
      <button
        onclick={() => exportSource(source)}
        title={t("sources.export")}
      >
        <Download size={14} /> {t("sources.export")}
      </button>
    </div>
  </article>
{/snippet}

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
    <button class="primary" onclick={() => (importModalOpen = true)}>
      <Upload size={15} /> {t("sources.import")}
    </button>
    <button class="danger-btn" onclick={restoreSources}>
      <RotateCcw size={15} /> {t("sources.restore")}
    </button>
  </div>
</div>

{#if importModalOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) importModalOpen = false;
    }}
  >
    <div class="modal-dialog" role="dialog" aria-modal="true" tabindex="-1">
      <div class="modal-header">
        <h3>{t("sources.importDialogTitle")}</h3>
        <button class="icon" onclick={() => (importModalOpen = false)}
          ><X size={18} /></button
        >
      </div>

      <div class="modal-body">
        <div class="import-options">
          <button class="import-option-btn" onclick={importSingleFile}>
            <FileCode size={20} />
            <div>
              <strong>{t("sources.singleFile")}</strong>
              <small>{t("sources.singleFileDesc")}</small>
            </div>
          </button>

          <button class="import-option-btn" onclick={importFolder}>
            <FolderOpen size={20} />
            <div>
              <strong>{t("sources.folder")}</strong>
              <small>{t("sources.folderDesc")}</small>
            </div>
          </button>
        </div>

        <div class="github-import-box">
          <label>
            <span><Github size={15} /> {t("sources.githubImport")}</span>
            <div class="input-row">
              <input
                bind:value={githubRepoInput}
                placeholder={t("sources.githubPlaceholder")}
                spellcheck="false"
              />
              <button class="primary" onclick={importFromGithub}
                >{t("sources.download")}</button
              >
            </div>
          </label>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Section 1: Official Sources -->
<section class="sources-section">
  <div class="sources-section-header">
    <div class="section-title-wrap">
      <ShieldCheck size={18} class="section-icon official-icon" />
      <h2>{t("sources.sectionOfficial")}</h2>
      <span class="count-badge">{officialSources.length}</span>
    </div>
  </div>
  <div class="sources-list">
    {#each officialSources as source (source.id)}
      {@render sourceCard(source)}
    {/each}
  </div>
</section>

<!-- Section 2: Created Connectors (Source Lab) -->
<section class="sources-section">
  <div class="sources-section-header">
    <div class="section-title-wrap">
      <FlaskConical size={18} class="section-icon created-icon" />
      <h2>{t("sources.sectionCreated")}</h2>
      <span class="count-badge">{createdSources.length}</span>
    </div>
    {#if addSource}
      <button class="primary subtle-action-btn" onclick={addSource}>
        <Plus size={14} /> {t("sources.createConnector")}
      </button>
    {/if}
  </div>

  {#if createdSources.length > 0}
    <div class="sources-list">
      {#each createdSources as source (source.id)}
        {@render sourceCard(source)}
      {/each}
    </div>
  {:else}
    <div class="empty-sources-box">
      <FlaskConical size={32} class="empty-icon" />
      <p>{t("sources.emptyCreated")}</p>
      {#if addSource}
        <button class="primary" onclick={addSource}>
          <Plus size={15} /> {t("sources.createConnector")}
        </button>
      {/if}
    </div>
  {/if}
</section>

<!-- Section 3: Imported Connectors -->
<section class="sources-section">
  <div class="sources-section-header">
    <div class="section-title-wrap">
      <FolderDown size={18} class="section-icon imported-icon" />
      <h2>{t("sources.sectionImported")}</h2>
      <span class="count-badge">{importedSources.length}</span>
    </div>
    <button class="primary subtle-action-btn" onclick={() => (importModalOpen = true)}>
      <Upload size={14} /> {t("sources.import")}
    </button>
  </div>

  {#if importedSources.length > 0}
    <div class="sources-list">
      {#each importedSources as source (source.id)}
        {@render sourceCard(source)}
      {/each}
    </div>
  {:else}
    <div class="empty-sources-box">
      <FolderDown size={32} class="empty-icon" />
      <p>{t("sources.emptyImported")}</p>
      <button onclick={() => (importModalOpen = true)}>
        <Upload size={15} /> {t("sources.import")}
      </button>
    </div>
  {/if}
</section>

<style>
  .sources-section {
    margin-bottom: 32px;
  }

  .sources-section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 12px;
    margin-bottom: 16px;
    border-bottom: 1px solid var(--border, #374151);
    flex-wrap: wrap;
    gap: 12px;
  }

  .section-title-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .section-title-wrap h2 {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
    color: var(--fg, #f3f4f6);
  }

  :global(.section-icon.official-icon) {
    color: #60a5fa;
  }

  :global(.section-icon.created-icon) {
    color: #a78bfa;
  }

  :global(.section-icon.imported-icon) {
    color: #34d399;
  }

  .count-badge {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 9999px;
    padding: 1px 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted, #94a3b8);
  }

  .subtle-action-btn {
    font-size: 12px;
    padding: 6px 12px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .empty-sources-box {
    border: 1px dashed var(--border, #374151);
    border-radius: 10px;
    padding: 32px 20px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    background: rgba(255, 255, 255, 0.015);
  }

  .empty-sources-box p {
    color: var(--muted, #94a3b8);
    font-size: 13px;
    margin: 0;
  }

  :global(.empty-icon) {
    opacity: 0.4;
    color: var(--muted, #94a3b8);
  }

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
    white-space: normal;
  }
  .import-option-btn div {
    display: flex;
    flex-direction: column;
    gap: 2px;
    white-space: normal;
  }
  .import-option-btn small {
    color: var(--muted, #94a3b8);
    font-size: 11px;
    white-space: normal;
    line-height: 1.35;
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
