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
  import { t } from "../lib/i18n";
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
    onSources,
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
    onSources?: () => void;
  } = $props();
</script>

<div class="catalog-switch" aria-label={t("catalog.switchAria")}>
  {#each [{ value: "all", label: t("catalog.all") }, { value: "manga", label: t("catalog.manga") }, { value: "comics", label: t("catalog.comics") }] as kind}
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
    placeholder={t("catalog.searchPlaceholder")}
    aria-label={t("catalog.search")}
  /><kbd>Ctrl K</kbd><button
    class="primary"
    disabled={searching || !catalogSources.length}
    >{#if searching}<LoaderCircle size={16} class="spin" /> {t("catalog.searching")}{:else}{t("catalog.search")}
      <ChevronRight size={16} />{/if}</button
  >
</form>
{#if catalogKind === "comics"}<details class="catalog-link">
    <summary>{t("catalog.openFromLink")}</summary>
    <form
      class="input-row"
      onsubmit={(event) => {
        event.preventDefault();
        void openCatalogLink();
      }}
    >
      <input
        aria-label={t("catalog.seriesLinkAria")}
        bind:value={catalogLink}
        placeholder="https://example.com/comic/series"
      /><button disabled={!catalogLink.trim()}
        >{t("catalog.openInApp")} <BookOpen size={16} /></button
      >
    </form>
    <p>
      {t("catalog.seriesLinkNote")}
    </p>
  </details>{/if}
{#if reports.some((r) => r.error)}<details class="search-errors">
    <summary
      >{reports.filter((r) => r.error).length}
      {reports.filter((r) => r.error).length === 1
        ? t("catalog.sourceUnavailableSingle")
        : t("catalog.sourceUnavailableMultiple")}</summary
    >
    {#each reports.filter((r) => r.error) as report}<p>
        <strong>{report.name}</strong>: {report.error}
      </p>{/each}
  </details>{/if}
<div class="results-heading">
  <span
    >{searched
      ? t("catalog.resultsPage", { count: visibleResults.length, page })
      : ""}</span
  >
</div>
{#if searching && visibleResults.length}<p class="search-progress">
    <LoaderCircle size={14} class="spin" /> {t("catalog.moreMetadata")}
  </p>{/if}
{#if searching && !visibleResults.length}<div class="empty">
    <LoaderCircle size={32} class="spin" />
    <h2>{t("catalog.searching")}</h2>
    <p>{t("catalog.searchingSubtitle")}</p>
  </div>{:else if visibleResults.length}<div class="manga-grid">
    {#each visibleResults as manga (`${manga.source_id}:${manga.url}`)}<button
        class="manga-card"
        onclick={() => openManga(manga)}
        ><div class="cover">
          <Cover url={manga.cover} title={manga.title} />
          <span class="cover-action">{t("catalog.openManga")} <ChevronRight size={15} /></span
          >
        </div>
        <strong>{manga.title}</strong>{#if manga.authors?.length}<small
            >{manga.authors.join(", ")}</small
          >{/if}<span class="result-source"
          >{flag(manga.language)}
          {#if manga.source_name || manga.source_id}{manga.source_name ||
              manga.source_id} · {/if}{(manga.language || "").toUpperCase()}</span
        ></button
      >{/each}
  </div>
{:else if !searched && !catalogSources.length}<div class="empty">
    <div class="empty-icon"><BookOpen size={34} /></div>
    <h2>{t("nav.sources")}</h2>
    <p>{t("sources.emptyImported")}</p>
    {#if onSources}
      <button class="primary" onclick={onSources}>
        {t("nav.sources")} <ChevronRight size={16} />
      </button>
    {/if}
  </div>
{:else}<div class="empty">
    <div class="empty-icon"><BookOpen size={34} /></div>
    <h2>
      {searched ? t("catalog.noResults") : t("catalog.searchPrompt")}
    </h2>
    <p>
      {searched
        ? t("catalog.noResultsHint")
        : t("catalog.activeSourcesSubtitle")}
    </p>
  </div>{/if}
{#if searched && !searching}
  <div class="pagination">
    <button disabled={page === 1} onclick={() => search(page - 1)}
      ><ChevronLeft size={16} /> {t("catalog.prev")}</button
    ><span>{t("catalog.page", { current: page })}</span><button
      disabled={!hasNext}
      onclick={() => search(page + 1)}
      >{t("catalog.next")} <ChevronRight size={16} /></button
    >
  </div>
{/if}
