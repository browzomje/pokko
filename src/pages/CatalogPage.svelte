<script lang="ts">
  import {
    Search,
    BookOpen,
    ChevronLeft,
    ChevronRight,
    LoaderCircle,
  } from "lucide-svelte";
  import Cover from "../Cover.svelte";

  import { flag } from "../lib/presentation";
  import type { Manga, Source, SourceReport } from "../types";
  let {
    query = $bindable(""),
    searchInput = $bindable(),
    catalogKind,
    catalogSources,
    searching,
    searched,
    page,
    hasNext,
    visibleResults,
    reports,
    catalogLink = $bindable(""),
    switchCatalog,
    search,
    openCatalogLink,
    openManga,
  }: {
    query: string;
    searchInput?: HTMLInputElement;
    catalogKind: string;
    catalogSources: Source[];
    searching: boolean;
    searched: boolean;
    page: number;
    hasNext: boolean;
    visibleResults: Manga[];
    reports: SourceReport[];
    catalogLink: string;
    switchCatalog: (kind: string) => void;
    search: (page?: number) => void;
    openCatalogLink: () => void;
    openManga: (manga: Manga) => void;
  } = $props();
</script>

<div class="catalog-switch" aria-label="Tipo di catalogo">
  {#each [{ value: "all", label: "Tutti" }, { value: "manga", label: "Manga" }, { value: "comics", label: "Comics" }] as kind}
    <button
      class:chosen={catalogKind === kind.value}
      aria-pressed={catalogKind === kind.value}
      disabled={searching}
      onclick={() => switchCatalog(kind.value)}>{kind.label}</button
    >
  {/each}
</div>
<form
  class="searchbox"
  onsubmit={(e) => {
    e.preventDefault();
    search();
  }}
>
  <Search size={20} /><input
    bind:this={searchInput}
    bind:value={query}
    placeholder="Cerca per titolo o autore…"
    aria-label="Cerca manga"
  /><kbd>Ctrl K</kbd><button
    class="primary"
    disabled={searching || !catalogSources.length}
    >{#if searching}<LoaderCircle size={16} class="spin" /> Ricerca…{:else}Cerca
      <ChevronRight size={16} />{/if}</button
  >
</form>
{#if catalogKind === "comics"}<details class="catalog-link">
    <summary>Apri una serie da un link</summary>
    <form
      class="input-row"
      onsubmit={(event) => {
        event.preventDefault();
        void openCatalogLink();
      }}
    >
      <input
        aria-label="Link della serie comics"
        bind:value={catalogLink}
        placeholder="https://readcomicsonline.lol/comic/Invincible"
      /><button disabled={!catalogLink.trim()}
        >Apri nell’app <BookOpen size={16} /></button
      >
    </form>
    <p>
      Usa il link di una serie da una fonte attiva. I file vengono scaricati
      dalla coda dell’app.
    </p>
  </details>{/if}
{#if reports.some((r) => r.error)}<details class="search-errors">
    <summary
      >{reports.filter((r) => r.error).length}
      {reports.filter((r) => r.error).length === 1
        ? "fonte non disponibile"
        : "fonti non disponibili"}</summary
    >
    {#each reports.filter((r) => r.error) as report}<p>
        <strong>{report.name}</strong>: {report.error}
      </p>{/each}
  </details>{/if}
<div class="results-heading">
  <span
    >{searched
      ? `${visibleResults.length} risultati · pagina ${page}`
      : ""}</span
  >
</div>
{#if searching && visibleResults.length}<p class="search-progress">
    <LoaderCircle size={14} class="spin" /> Altre fonti e metadati in arrivo… Puoi
    già aprire un risultato.
  </p>{/if}
{#if searching && !visibleResults.length}<div class="empty">
    <LoaderCircle size={32} class="spin" />
    <h2>Ricerca in corso</h2>
    <p>Sto cercando nelle fonti attive…</p>
  </div>{:else if visibleResults.length}<div class="manga-grid">
    {#each visibleResults as manga (`${manga.source_id}:${manga.url}`)}<button
        class="manga-card"
        onclick={() => openManga(manga)}
        ><div class="cover">
          <Cover url={manga.cover} title={manga.title} />
          <span class="cover-action">Apri manga <ChevronRight size={15} /></span
          >
        </div>
        <strong>{manga.title}</strong>{#if manga.authors?.length}<small
            >{manga.authors.join(", ")}</small
          >{/if}<span class="result-source"
          >{flag(manga.language)}
          {manga.source_name || "MangaWorld"} · {(
            manga.language || "it"
          ).toUpperCase()}</span
        ></button
      >{/each}
  </div>
{:else}<div class="empty">
    <div class="empty-icon"><BookOpen size={34} /></div>
    <h2>
      {searched ? "Nessun manga trovato" : "Cerca un titolo"}
    </h2>
    <p>
      {searched
        ? "Prova un titolo diverso o controlla le fonti attive."
        : "Manga e comics dalle tue fonti attive."}
    </p>
  </div>{/if}
{#if searched && !searching}
  <div class="pagination">
    <button disabled={page === 1} onclick={() => search(page - 1)}
      ><ChevronLeft size={16} /> Precedente</button
    ><span>Pagina {page}</span><button
      disabled={!hasNext}
      onclick={() => search(page + 1)}
      >Successiva <ChevronRight size={16} /></button
    >
  </div>
{/if}
