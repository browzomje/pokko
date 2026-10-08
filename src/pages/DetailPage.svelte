<script lang="ts">
  import {
    Search,
    Download,
    BookOpen,
    FolderOpen,
    ChevronLeft,
    ChevronRight,
    ArrowDownToLine,
    LoaderCircle,
  } from "lucide-svelte";
  import Cover from "../Cover.svelte";
  import Select from "../Select.svelte";
  import { flag } from "../lib/presentation";
  import type { Manga, MangaPage, Volume, Settings } from "../types";
  import { unknownVolume, planDownload } from "../download-plan";
  let {
    current,
    settings,
    formatLabel,
    visibleVolumes,
    exportMode = $bindable("items"),
    queueing,
    counting,
    returnToResults,
    loadDetail,
    countPages,
    enqueue,
    onSettings,
  }: {
    current: MangaPage;
    settings: Settings;
    formatLabel: string;
    visibleVolumes: Volume[];
    exportMode: string;
    queueing: boolean;
    counting: string[];
    returnToResults: () => void;
    loadDetail: (manga: Manga) => void;
    countPages: (volume: Volume) => void;
    enqueue: (volumes: Volume[]) => void;
    onSettings: () => void;
  } = $props();
  let selection = $derived(
    visibleVolumes.filter((v) => current.selected.includes(v.title)),
  );
  let output = $derived(planDownload(selection, exportMode));
  let canSplit = $derived(visibleVolumes.some((v) => v.chapters.length > 1));
  $effect(() => {
    if (exportMode === "chapters" && !canSplit) exportMode = "items";
  });
  let hasUngrouped = $derived(
    current.detail?.volumes.some(unknownVolume) || false,
  );
  function partialSelection(node: HTMLInputElement, value: boolean) {
    node.indeterminate = value;
    return {
      update(next: boolean) {
        node.indeterminate = next;
      },
    };
  }
  function toggle(volume: Volume) {
    current.selected = current.selected.includes(volume.title)
      ? current.selected.filter((x) => x !== volume.title)
      : [...current.selected, volume.title];
  }
  function pageTotal(volume: Volume) {
    return volume.chapters.every((c) => c.page_count != null)
      ? `${volume.chapters.reduce((n, c) => n + (c.page_count || 0), 0)} pagine`
      : "Pagine da verificare";
  }
</script>

<button class="back-link detail-back" onclick={returnToResults}
  ><ChevronLeft size={16} /> Torna ai risultati</button
>
{#if current.loading}<div class="empty">
    <LoaderCircle size={32} class="spin" />
    <h2>Caricamento dei volumi</h2>
    <p>Sto leggendo l’elenco dei capitoli…</p>
  </div>{:else if current.error}<div class="empty">
    <h2>Impossibile caricare il manga</h2>
    <p>{current.error}</p>
    <button onclick={() => loadDetail(current!.manga)}>Riprova</button>
  </div>{:else if current.detail}
  <div class="detail-heading">
    <div class="detail-cover">
      <Cover
        url={current.detail.manga.cover}
        title={current.detail.manga.title}
      />
    </div>
    <div>
      <div class="eyebrow">
        {flag(current.detail.manga.language)}
        {current.detail.manga.source_name || "MangaWorld"} · {current.detail.volumes.reduce(
          (n, v) => n + v.chapters.length,
          0,
        )} CAPITOLI
        {#if !hasUngrouped}
          · {current.detail.volumes.length}
          {current.catalog === "comics" ? "ELEMENTI" : "VOLUMI"}{/if}
      </div>
      <h1>{current.title}</h1>
      <p class="description">{current.detail.description}</p>
    </div>
  </div>
  {#if current.catalog === "comics" && current.detail.volumes.some( (v) => v.title.startsWith("Volume ") )}<div
      class="queue-filters"
      aria-label="Raccolte e albi comics"
    >
      {#each [{ value: "collections", label: "Volumi / raccolte" }, { value: "issues", label: "Singoli albi" }, { value: "all", label: "Tutti gli elementi" }] as filter}<button
          class:chosen={current.volumeFilter === filter.value}
          aria-pressed={current.volumeFilter === filter.value}
          onclick={() => {
            if (current) {
              current.volumeFilter = filter.value;
              current.selected = [];
            }
          }}
          >{filter.label}<span class="count"
            >{current.detail.volumes.filter(
              (v) =>
                filter.value === "all" ||
                v.title.startsWith("Volume ") ===
                  (filter.value === "collections"),
            ).length}</span
          ></button
        >{/each}
    </div>{/if}
  <div class="download-destination">
    <FolderOpen size={16} /><span title={settings.output}
      >{settings.output || "Scegli una cartella al primo download"}</span
    ><button onclick={onSettings}>{formatLabel} · Modifica</button>
  </div>
  {#if hasUngrouped}<p class="grouping-note">
      La fonte non indica i volumi per {current.detail.volumes
        .filter(unknownVolume)
        .reduce((n, v) => n + v.chapters.length, 0)} capitoli. Sono elencati singolarmente.
    </p>{/if}
  <div class="export-choice">
    <span>Organizza i file</span><Select
      label="Organizzazione dei download"
      bind:value={exportMode}
      options={[
        {
          value: "items",
          label: hasUngrouped
            ? "Un file per capitolo / volume disponibile"
            : "Un file per volume / albo",
        },
        {
          value: "chapters",
          label: "Un file per capitolo",
          disabled: !canSplit,
        },
        {
          value: "combined",
          label: "Un unico file per la selezione",
        },
      ]}
    />
    <small
      >{exportMode === "combined"
        ? "I capitoli selezionati saranno uniti, nell’ordine mostrato."
        : "Ogni elemento verrà salvato separatamente."} Formato: {formatLabel}.</small
    >
  </div>
  {#if !canSplit && current.catalog === "comics"}<p class="grouping-note">
      Ogni elemento ha un solo lettore nella fonte. Le raccolte non espongono i
      singoli capitoli: puoi scaricarle intere o unirle.
    </p>{/if}
  <div class="output-preview" aria-live="polite">
    <strong
      >{selection.length
        ? `${selection.length} elementi → ${output.length} file`
        : "Seleziona gli elementi per vedere i file che verranno creati"}</strong
    >{#if output.length}<ul>
        {#each output.slice(0, 3) as file}<li>
            {file.title} · {file.chapters.length}
            {file.chapters.length === 1 ? "capitolo" : "capitoli"}
          </li>{/each}
      </ul>
      {#if output.length > 3}<small>… e altri {output.length - 3} file</small
        >{/if}{/if}
  </div>
  <div class="volume-toolbar">
    <label class="checkbox"
      ><input
        type="checkbox"
        use:partialSelection={current.selected.length > 0 &&
          current.selected.length < visibleVolumes.length}
        checked={visibleVolumes.length > 0 &&
          current.selected.length === visibleVolumes.length}
        onchange={() => {
          if (current?.detail)
            current.selected =
              current.selected.length === visibleVolumes.length
                ? []
                : visibleVolumes.map((v) => v.title);
        }}
      /> Seleziona tutti</label
    >
    <div>
      <button
        class="primary"
        disabled={!current.selected.length || queueing}
        onclick={() =>
          enqueue(
            visibleVolumes.filter((v) => current!.selected.includes(v.title)),
          )}
        ><Download size={16} />
        {queueing ? "Aggiunta…" : "Scarica selezionati"} ({current.selected
          .length})</button
      ><button
        disabled={queueing || !visibleVolumes.length}
        onclick={() => enqueue(visibleVolumes)}
        ><Download size={16} /> Scarica tutti</button
      >
    </div>
  </div>
  <div class="volume-list">
    {#each visibleVolumes as volume}<div
        class="volume-row"
        class:selected={current.selected.includes(volume.title)}
      >
        <div class="volume-main">
          <label class="volume-select"
            ><input
              type="checkbox"
              checked={current.selected.includes(volume.title)}
              onchange={() => toggle(volume)}
            /><BookOpen size={19} /><strong>{volume.title}</strong><span
              >{volume.chapters.length > 1
                ? `${volume.chapters.length} capitoli · `
                : ""}{pageTotal(volume)}</span
            ></label
          >{#if volume.chapters.length > 1}<button
              class="icon"
              aria-label={`Mostra capitoli di ${volume.title}`}
              aria-expanded={current.expanded.includes(volume.title)}
              onclick={() =>
                (current!.expanded = current!.expanded.includes(volume.title)
                  ? current!.expanded.filter((t) => t !== volume.title)
                  : [...current!.expanded, volume.title])}
              ><ChevronRight
                size={18}
                class={current.expanded.includes(volume.title) ? "rotated" : ""}
              /></button
            >{/if}<button
            class="icon"
            title="Conta le pagine senza scaricare le scan"
            disabled={volume.chapters.some((c) => counting.includes(c.url))}
            onclick={() => countPages(volume)}><Search size={17} /></button
          ><button
            class="icon"
            aria-label={`Scarica ${volume.title}`}
            title={`Scarica ${volume.title}`}
            disabled={queueing}
            onclick={() => enqueue([volume])}
            ><ArrowDownToLine size={19} /></button
          >
        </div>
        {#if current.expanded.includes(volume.title)}<div class="chapter-list">
            {#each volume.chapters as chapter}<div>
                <span>{chapter.title}</span><span
                  >{chapter.page_count != null
                    ? `${chapter.page_count} pagine`
                    : "— pagine"}</span
                >
                <button
                  class="icon"
                  title="Conta pagine"
                  disabled={counting.includes(chapter.url)}
                  onclick={() =>
                    countPages({
                      title: volume.title,
                      chapters: [chapter],
                    })}
                  >{#if counting.includes(chapter.url)}<LoaderCircle
                      size={16}
                      class="spin"
                    />{:else}<Search size={16} />{/if}</button
                >
                <button
                  class="icon"
                  disabled={queueing}
                  title={`Scarica ${chapter.title}`}
                  onclick={() =>
                    enqueue([
                      {
                        title: `${volume.title} - ${chapter.title}`,
                        chapters: [chapter],
                      },
                    ])}><Download size={17} /></button
                >
              </div>{/each}
          </div>{/if}
      </div>{/each}
  </div>{/if}
