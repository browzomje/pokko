<script lang="ts">
  import { Plus, Trash2, Globe } from "lucide-svelte";

  import { flag } from "../lib/presentation";
  import type { Source } from "../types";
  let {
    sortedSources,
    saveError,
    saveStatus,
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
</script>

<div class="page-heading">
  <div>
    <div class="eyebrow">CATALOGO</div>
    <h1>Fonti del catalogo</h1>
    <p>
      Gestisci le fonti per manga e comics. Modifica la configurazione o prova i
      connettori nel laboratorio.
    </p>
    <div class="save-status" class:failure={!!saveError} role="status">
      {saveStatus}{#if saveError}<span>{saveError}</span>{/if}
    </div>
  </div>
</div>
<div class="sources-heading">
  <div>
    <h2><Globe size={19} /> Fonti disponibili</h2>
    <p>
      Attiva solo le fonti che vuoi cercare. I domini non verificati restano
      disattivati.
    </p>
  </div>
  <div>
    <button onclick={restoreSources}>Ripristina elenco</button><button
      class="primary"
      onclick={addSource}><Plus size={16} /> Aggiungi fonte</button
    >
  </div>
</div>
<div class="sources-list">
  {#each sortedSources as source (source.id)}<article class="source-card">
      <div class="source-header">
        <label class="checkbox"
          ><input type="checkbox" bind:checked={source.enabled} /><strong
            >{source.name}</strong
          ></label
        ><span class="pill"
          >{flag(source.language)}
          {source.adapter === "mangadex"
            ? "API"
            : source.adapter === "readcomics"
              ? "Comics"
              : source.adapter === "mangaworld"
                ? "MangaWorld"
                : "HTML generico"}</span
        ><button
          class="icon danger"
          title={`Rimuovi fonte ${source.name}`}
          onclick={() => removeSource(source.id)}><Trash2 size={18} /></button
        >
      </div>
      <p class="source-note">{source.note}</p>
      <div class="source-actions">
        <button onclick={() => editSource(source)}>Modifica</button>
        <button onclick={() => openLab(source)}>Apri laboratorio</button>
      </div>
    </article>{/each}
</div>
