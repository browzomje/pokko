<script lang="ts">
  import { onMount, tick } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { Download, X, ChevronRight } from "lucide-svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Shortcuts from "./components/Shortcuts.svelte";
  import CatalogPage from "./pages/CatalogPage.svelte";
  import SourcesPage from "./pages/SourcesPage.svelte";
  import SettingsPage from "./pages/SettingsPage.svelte";
  import DetailPage from "./pages/DetailPage.svelte";
  import DownloadsPage from "./pages/DownloadsPage.svelte";
  import { api, message } from "./lib/api";
  import { newSource } from "./lib/source-config";
  import { formats } from "./lib/presentation";
  import type { MangaPage } from "./types";

  import { mergeSourceResults } from "./lib/search-progress";
  import { rankSearchResults } from "./lib/search-ranking";
  import SourceLab from "./SourceLab.svelte";
  import { catalogElements, planDownload } from "./download-plan";
  import sourcePresets from "../resources/sources.json";
  import type {
    Manga,
    Detail,
    Settings,
    Job,
    Volume,
    Source,
    SourceReport,
  } from "./types";
  let catalogKind = $state("all");
  let query = $state("");
  let searchInput = $state<HTMLInputElement>();
  let results = $state<Manga[]>([]);
  let searching = $state(false);
  let searched = $state(false);
  let page = $state(1);
  let hasNext = $state(false);
  let lastQuery = $state("");
  let mangaPage = $state<MangaPage | null>(null);
  let explorerRoute = $state("results");
  let active = $state("search");
  let jobs = $state<Job[]>([]);
  let error = $state("");
  let notice = $state("");
  let settings = $state<Settings>({
    base_url: "https://www.mangaworld.mx",
    output: "",
    delay_ms: 500,
    cbz: false,
    export_format: "pdf",
    sources: sourcePresets,
  });
  let hydrated = $state(false);
  let saveStatus = $state("Caricamento preferenze…");
  let saveError = $state("");
  let reports = $state<SourceReport[]>([]);
  let content = $state<HTMLElement>();
  const scrollPositions = new Map<string, number>();
  let displayedSection = "search";
  $effect.pre(() => {
    const section =
      active === "search"
        ? explorerRoute === "detail" && mangaPage
          ? mangaPage.id
          : `search:${catalogKind}`
        : active;
    const viewport = content;
    if (!viewport) return;
    scrollPositions.set(displayedSection, viewport.scrollTop);
    displayedSection = section;
    void tick().then(() => {
      if (displayedSection === section)
        viewport.scrollTop = scrollPositions.get(section) || 0;
    });
  });
  let removed = new Set<string>();
  let lastSaved = "";
  let saveChain: Promise<void> = Promise.resolve();
  let visibleResults = $derived(rankSearchResults(results, lastQuery));
  const languageRank = (language: string) => (language === "en" ? 1 : 0);
  const sourceOrder = (a: Source, b: Source) =>
    languageRank(a.language) - languageRank(b.language) ||
    a.name.localeCompare(b.name, "it");
  let sortedSources = $derived([...settings.sources].sort(sourceOrder));
  let enabledSources = $derived(sortedSources.filter((s) => s.enabled));
  let catalogSources = $derived(
    enabledSources.filter(
      (s) => catalogKind === "all" || s.category === catalogKind,
    ),
  );
  const catalogSnapshots = new Map<
    string,
    {
      query: string;
      results: Manga[];
      searched: boolean;
      page: number;
      hasNext: boolean;
      lastQuery: string;
      reports: SourceReport[];
    }
  >();
  function switchCatalog(kind: string) {
    if (searching && kind !== catalogKind) return;
    if (kind !== catalogKind) {
      catalogSnapshots.set(catalogKind, {
        query,
        results,
        searched,
        page,
        hasNext,
        lastQuery,
        reports,
      });
      const snapshot = catalogSnapshots.get(kind);
      query = snapshot?.query || "";
      results = snapshot?.results || [];
      searched = snapshot?.searched || false;
      page = snapshot?.page || 1;
      hasNext = snapshot?.hasNext || false;
      lastQuery = snapshot?.lastQuery || "";
      reports = snapshot?.reports || [];
      catalogKind = kind;
    }
    active = "search";
    explorerRoute = "results";
  }
  function returnToResults() {
    explorerRoute = "results";
    active = "search";
  }

  $effect(() => {
    const snapshot = JSON.stringify(settings);
    if (!hydrated || snapshot === lastSaved) return;
    saveStatus = "Modifiche in corso…";
    const timer = setTimeout(() => {
      void persistSettings(snapshot).catch(() => {});
    }, 550);
    return () => clearTimeout(timer);
  });
  function persistSettings(snapshot = JSON.stringify(settings), force = false) {
    const task = saveChain
      .catch(() => {})
      .then(async () => {
        if (snapshot === lastSaved && !force) return;
        if (snapshot === JSON.stringify(settings)) {
          saveStatus = "Salvataggio…";
          saveError = "";
        }
        try {
          await api("save_settings", { settings: JSON.parse(snapshot) });
          lastSaved = snapshot;
          if (snapshot === JSON.stringify(settings)) {
            saveStatus = "Tutte le modifiche salvate";
            saveError = "";
          }
        } catch (e) {
          if (snapshot === JSON.stringify(settings)) {
            saveStatus = "Modifiche non salvate";
            saveError = message(e);
          }
          throw e;
        }
      });
    saveChain = task;
    return task;
  }
  let sourceDraft = $state<Source | null>(null);
  function editSource(source: Source) {
    openLab(source);
  }
  function addSource() {
    sourceDraft = newSource();
    active = "lab";
  }
  function goLab() {
    if (!sourceDraft) sourceDraft = newSource();
    active = "lab";
  }
  async function saveSource(source: Source) {
    let previous: Settings | null = null;
    try {
      await api("validate_source", { source });
      const next: Settings = $state.snapshot(settings);
      const index = next.sources.findIndex((s) => s.id === source.id);
      if (index < 0) next.sources.push(structuredClone(source));
      else next.sources[index] = structuredClone(source);
      previous = $state.snapshot(settings);
      settings = next;
      await persistSettings();
      saveStatus = "Tutte le modifiche salvate";
    } catch (e) {
      if (previous) {
        settings = previous;
        await persistSettings(JSON.stringify(previous), true).catch(() => {});
      }
      throw e;
    }
  }
  function openLab(source: Source) {
    sourceDraft = structuredClone($state.snapshot(source));
    active = "lab";
  }
  async function restoreSources() {
    try {
      settings.sources = await api<Source[]>("source_defaults");
    } catch (e) {
      error = message(e);
    }
  }
  async function removeJob(job: Job) {
    try {
      await api("remove_job", { id: job.id });
      removed.add(job.id);
      jobs = jobs.filter((j) => j.id !== job.id);
    } catch (e) {
      error = message(e);
    }
  }

  let queueing = $state(false);
  let shortcuts = $state(false);
  let theme = $state(localStorage.getItem("mw-theme") || "dark");
  let current = $derived(
    active === "search" && explorerRoute === "detail" ? mangaPage : null,
  );
  let exportMode = $state("items");
  let visibleVolumes = $derived(
    catalogElements(current?.detail?.volumes || []).filter(
      (v) =>
        current?.volumeFilter === "all" ||
        (v.title.startsWith("Volume ")
          ? current?.volumeFilter === "collections"
          : current?.volumeFilter === "issues"),
    ) || [],
  );
  let pending = $derived(
    jobs.filter((j) => ["queued", "running", "paused"].includes(j.status))
      .length,
  );
  async function goSearch() {
    explorerRoute = "results";
    active = "search";
    await tick();
    searchInput?.focus({ preventScroll: true });
  }
  let running = $derived(jobs.find((j) => j.status === "running"));
  $effect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("mw-theme", theme);
  });
  async function search(target = 1) {
    if (searching) return;
    if (!catalogSources.length) {
      error = "Attiva almeno una fonte in Fonti del catalogo.";
      return;
    }
    searching = true;
    error = "";
    active = "search";
    explorerRoute = "results";
    const requestId = crypto.randomUUID();
    const searchQuery = target === 1 ? query : lastQuery;
    results = [];
    reports = [];
    hasNext = false;
    page = target;
    searched = true;
    lastQuery = searchQuery;
    let unprogress: (() => void) | undefined;
    try {
      if (isTauri())
        unprogress = await listen<{
          request_id: string;
          source_id: string;
          items: Manga[];
          report: SourceReport;
        }>("catalog-results", ({ payload }) => {
          if (payload.request_id !== requestId) return;
          results = mergeSourceResults(
            results,
            payload.source_id,
            payload.items,
          );
          reports = [
            ...reports.filter((r) => r.id !== payload.source_id),
            payload.report,
          ];
          hasNext = reports.some((report) => report.has_next);
        });
      await persistSettings();
      const r = await api<{
        items: Manga[];
        has_next: boolean;
        reports: SourceReport[];
      }>("search_manga", {
        query: searchQuery,
        requestId,
        page: target,
        sourceIds: catalogSources.map((s) => s.id),
      });
      results = r.items;
      reports = r.reports;
      hasNext = r.has_next;
      page = target;
      searched = true;
    } catch (e) {
      error = message(e);
    } finally {
      unprogress?.();
      searching = false;
    }
  }
  let catalogLink = $state("");
  async function openCatalogLink() {
    try {
      const url = new URL(catalogLink.trim());
      if (url.protocol !== "https:")
        throw new Error("Inserisci un link HTTPS alla serie.");
      const source = catalogSources.find(
        (s) => new URL(s.base_url).hostname === url.hostname,
      );
      if (!source)
        throw new Error(
          "Aggiungi o attiva il dominio in Fonti del catalogo, scegliendo la categoria Comics.",
        );
      url.hash = "";
      await openManga({
        title: decodeURIComponent(
          url.pathname.split("/").filter(Boolean).slice(-1)[0] || "Serie",
        ),
        url: url.toString().replace(/\/$/, ""),
        cover: "",
        source_id: source.id,
        source_name: source.name,
        language: source.language,
      });
    } catch (e) {
      error = message(e);
    }
  }
  async function openManga(manga: Manga) {
    exportMode = "items";
    if (mangaPage?.id !== manga.url) {
      mangaPage = {
        id: manga.url,
        title: manga.title,
        manga,
        detail: null,
        loading: true,
        error: "",
        selected: [],
        expanded: [],
        catalog:
          settings.sources.find((s) => s.id === manga.source_id)?.category ||
          "manga",
        volumeFilter: "all",
      };
    }
    explorerRoute = "detail";
    active = "search";
    if (mangaPage.detail) return;
    await loadDetail(manga);
  }
  async function loadDetail(manga: Manga) {
    const detailPage = mangaPage?.id === manga.url ? mangaPage : null;
    if (!detailPage) return;
    detailPage.loading = true;
    detailPage.error = "";
    try {
      detailPage.detail = await api<Detail>("manga_detail", { manga });
      detailPage.title = detailPage.detail.manga.title;
      detailPage.volumeFilter =
        detailPage.catalog === "comics" &&
        detailPage.detail.volumes.some((v) => v.title.startsWith("Volume "))
          ? "collections"
          : "all";
    } catch (e) {
      detailPage.error = message(e);
    } finally {
      detailPage.loading = false;
    }
  }
  let counting = $state<string[]>([]);
  let formatLabel = $derived(
    formats.find((f) => f.value === settings.export_format)?.label ||
      "File PDF",
  );
  function enqueue(volumes: Volume[]) {
    void queueExport(volumes);
  }
  async function countPages(volume: Volume) {
    if (!current?.detail) return;
    const manga = current.detail.manga;
    for (const chapter of volume.chapters) {
      if (chapter.page_count != null || counting.includes(chapter.url))
        continue;
      counting = [...counting, chapter.url];
      try {
        chapter.page_count = await api<number>("chapter_pages", {
          manga,
          chapter,
        });
      } catch (e) {
        error = message(e);
        break;
      } finally {
        counting = counting.filter((url) => url !== chapter.url);
      }
    }
  }
  async function openFolder(job: Job) {
    try {
      await api("open_job_folder", { id: job.id });
    } catch (e) {
      error = message(e);
    }
  }
  async function archiveCompleted() {
    try {
      await api("archive_completed");
      jobs = jobs.map((j) =>
        j.status === "completed" ? { ...j, archived: true } : j,
      );
    } catch (e) {
      error = message(e);
    }
  }
  async function removeAll() {
    try {
      await api("remove_all_jobs");
      const deleted = jobs.filter((j) => j.status !== "completed");
      for (const job of deleted) removed.add(job.id);
      jobs = jobs.filter((j) => j.status === "completed");
    } catch (e) {
      error = message(e);
    }
  }
  async function queueExport(volumes: Volume[]) {
    const detailPage = current;
    if (
      !detailPage?.detail ||
      queueing ||
      !volumes.length ||
      (isTauri() && !hydrated)
    )
      return;
    const manga = detailPage.detail.manga;
    queueing = true;
    error = "";
    try {
      if (!settings.output.trim()) await chooseFolder();
      if (!settings.output.trim()) return;
      await persistSettings();
      const output = planDownload(volumes, exportMode);
      await api("enqueue", { manga, volumes: output });
      detailPage.selected = detailPage.selected.filter(
        (title) => !volumes.some((v) => v.title === title),
      );
      notice = `${manga.title} · ${output.length} ${output.length === 1 ? "file inviato" : "file inviati"} alla coda in ${formatLabel}. I duplicati vengono saltati.`;
    } catch (e) {
      error = message(e);
    } finally {
      queueing = false;
    }
  }
  async function control(job: Job, action: string) {
    try {
      await api("control_job", { id: job.id, action });
    } catch (e) {
      error = message(e);
    }
  }
  async function chooseFolder() {
    try {
      const path = await open({
        directory: true,
        multiple: false,
        title: "Cartella dei manga",
        defaultPath: settings.output || undefined,
      });
      if (typeof path === "string") settings.output = path;
    } catch (e) {
      error = message(e);
    }
  }
  function keydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (shortcuts) shortcuts = false;
      else if (current) returnToResults();
      error = "";
      notice = "";
      return;
    }
    if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
    const key = e.key.toLowerCase();
    if (e.shiftKey) {
      if (!["f", "t", "h", "l"].includes(key)) return;
      e.preventDefault();
      if (key === "f") active = "sources";
      if (key === "l") goLab();
      if (key === "t") theme = theme === "dark" ? "light" : "dark";
      if (key === "h") shortcuts = !shortcuts;
    } else if (["k", "j", ",", "arrowleft"].includes(key)) {
      e.preventDefault();
      if (key === "k") void goSearch();
      if (key === "j") active = "queue";
      if (key === ",") active = "settings";
      if (key === "arrowleft" && current) returnToResults();
    }
  }
  onMount(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let unremove: (() => void) | undefined;
    if (!isTauri())
      saveStatus = "Anteprima web · preferenze disponibili nell’app desktop";
    if (isTauri())
      (async () => {
        try {
          unlisten = await listen<Job>("job-update", (event) => {
            if (removed.has(event.payload.id)) return;
            const index = jobs.findIndex((j) => j.id === event.payload.id);
            if (index < 0) jobs.push(event.payload);
            else jobs[index] = event.payload;
          });
          unremove = await listen<string>("job-removed", (event) => {
            removed.add(event.payload);
            jobs = jobs.filter((j) => j.id !== event.payload);
          });
          if (disposed) {
            unremove();
            unlisten();
            return;
          }
          const data = await api<{ settings: Settings | null; jobs: Job[] }>(
            "bootstrap",
          );
          if (data.settings) settings = data.settings;
          jobs = data.jobs.filter(
            (j) => j.status !== "cancelled" && !removed.has(j.id),
          );
          lastSaved = JSON.stringify(settings);
          saveStatus = "Tutte le modifiche salvate";
          hydrated = true;
        } catch (e) {
          error = message(e);
        }
      })();
    return () => {
      disposed = true;
      unlisten?.();
      unremove?.();
    };
  });
</script>

<svelte:window onkeydown={keydown} />
<div class="shell">
  <div class="workspace">
    <Sidebar
      {active}
      navigate={(section) => (active = section)}
      {theme}
      onTheme={() => (theme = theme === "dark" ? "light" : "dark")}
      onShortcuts={() => (shortcuts = true)}
      onLab={goLab}
    />
    <div class="main">
      {#if error}<div class="alert error" role="alert">
          {error}<button
            class="icon"
            aria-label="Chiudi errore"
            onclick={() => (error = "")}><X size={16} /></button
          >
        </div>{/if}{#if notice}<div class="alert notice" role="status">
          <span>{notice}</span>{#if active !== "queue"}<button
              onclick={() => (active = "queue")}
              >Apri download <ChevronRight size={15} /></button
            >{/if}<button
            class="icon"
            aria-label="Chiudi messaggio"
            onclick={() => (notice = "")}><X size={16} /></button
          >
        </div>{/if}
      <main bind:this={content}>
        {#if active === "search" && !current}
          <CatalogPage
            bind:query
            bind:searchInput
            {catalogKind}
            {catalogSources}
            {searching}
            {searched}
            {page}
            {hasNext}
            {visibleResults}
            {reports}
            bind:catalogLink
            {switchCatalog}
            {search}
            {openCatalogLink}
            {openManga}
          />
        {:else if active === "queue"}
          <DownloadsPage
            {jobs}
            onSearch={goSearch}
            {removeAll}
            {removeJob}
            {openFolder}
            {control}
            onArchive={archiveCompleted}
          />
        {:else if active === "settings"}
          <SettingsPage bind:settings {saveError} {saveStatus} {chooseFolder} />
        {:else if active === "sources"}
          <SourcesPage
            {sortedSources}
            {saveError}
            {saveStatus}
            {restoreSources}
            {addSource}
            {editSource}
            {openLab}
            removeSource={(id) =>
              (settings.sources = settings.sources.filter((s) => s.id !== id))}
          />
        {:else if active === "lab" && sourceDraft}
          {#key sourceDraft.id}<SourceLab
              bind:source={sourceDraft}
              sources={sortedSources}
              onSelect={openLab}
              onNew={addSource}
              onSave={saveSource}
              onBack={() => (active = "sources")}
            />{/key}
        {:else if current}
          <DetailPage
            {current}
            {settings}
            {formatLabel}
            {visibleVolumes}
            bind:exportMode
            {queueing}
            {counting}
            {returnToResults}
            {loadDetail}
            {countPages}
            {enqueue}
            onSettings={() => (active = "settings")}
          />
        {/if}
      </main>
      <footer>
        <span
          ><span class="dot"></span>{running
            ? `Download: ${running.volume.title}`
            : "Pronto"}
        </span><span
          >{pending} in coda <span class="footer-separator">|</span>
          {formats.find((f) => f.value === settings.export_format)?.label ||
            "File PDF"}
        </span>
      </footer>
    </div>
  </div>
</div>
{#if shortcuts}<Shortcuts onClose={() => (shortcuts = false)} />{/if}
