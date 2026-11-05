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
  import { t } from "../lib/i18n";
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
      ? `${volume.chapters.reduce((n, c) => n + (c.page_count || 0), 0)} ${t("downloads.pages")}`
      : t("detail.pagesPending");
  }
</script>

<button class="back-link detail-back" onclick={returnToResults}
  ><ChevronLeft size={16} /> {t("detail.backToResults")}</button
>
{#if current.loading}<div class="empty">
    <LoaderCircle size={32} class="spin" />
    <h2>{t("detail.loadingVolumes")}</h2>
    <p>{t("detail.loadingChapters")}</p>
  </div>{:else if current.error}<div class="empty">
    <h2>{t("detail.cannotLoad")}</h2>
    <p>{current.error}</p>
    <button onclick={() => loadDetail(current!.manga)}>{t("detail.retry")}</button>
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
        )} {t("detail.chapters").toUpperCase()}
        {#if !hasUngrouped}
          · {current.detail.volumes.length}
          {current.catalog === "comics" ? "COMICS" : t("detail.volumes").toUpperCase()}{/if}
      </div>
      <h1>{current.title}</h1>
      <p class="description">{current.detail.description}</p>
    </div>
  </div>
  {#if current.catalog === "comics" && current.detail.volumes.some( (v) => v.title.startsWith("Volume ") )}<div
      class="queue-filters"
      aria-label="Raccolte e albi comics"
    >
      {#each [{ value: "collections", label: t("detail.collections") }, { value: "issues", label: t("detail.issues") }, { value: "all", label: t("detail.allElements") }] as filter}<button
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
      >{settings.output || t("detail.destinationNotice")}</span
    ><button onclick={onSettings}>{formatLabel} · {t("detail.edit")}</button>
  </div>
  {#if hasUngrouped}<p class="grouping-note">
      {t("detail.ungroupedNote", {
        count: current.detail.volumes
          .filter(unknownVolume)
          .reduce((n, v) => n + v.chapters.length, 0),
      })}
    </p>{/if}
  <div class="export-choice">
    <span>{t("detail.organizeFiles")}</span><Select
      label={t("detail.organizeFiles")}
      bind:value={exportMode}
      options={[
        {
          value: "items",
          label: hasUngrouped
            ? t("detail.modeAvailable")
            : t("detail.onePerVolume"),
        },
        {
          value: "chapters",
          label: t("detail.onePerChapter"),
          disabled: !canSplit,
        },
        {
          value: "combined",
          label: t("detail.singleFile"),
        },
      ]}
    />
    <small
      >{exportMode === "combined"
        ? t("detail.combinedDesc", { format: formatLabel })
        : t("detail.separateDesc", { format: formatLabel })}</small
    >
  </div>
  {#if !canSplit && current.catalog === "comics"}<p class="grouping-note">
      {t("detail.comicsSingleReaderNote")}
    </p>{/if}
  <div class="output-preview" aria-live="polite">
    <strong
      >{selection.length
        ? t("detail.selectionPreview", { selection: selection.length, output: output.length })
        : t("detail.selectionPrompt")}</strong
    >{#if output.length}<ul>
        {#each output.slice(0, 3) as file}<li>
            {file.title} · {file.chapters.length}
            {file.chapters.length === 1 ? t("downloads.chapter") : t("downloads.chapters")}
          </li>{/each}
      </ul>
      {#if output.length > 3}<small>{t("detail.moreFiles", { count: output.length - 3 })}</small
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
      /> {t("detail.selectAll")}</label
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
        {queueing ? "…" : t("detail.downloadSelected")} ({current.selected
          .length})</button
      ><button
        disabled={queueing || !visibleVolumes.length}
        onclick={() => enqueue(visibleVolumes)}
        ><Download size={16} /> {t("detail.downloadAll")}</button
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
                ? `${volume.chapters.length} ${t("downloads.chapters")} · `
                : ""}{pageTotal(volume)}</span
            ></label
          >{#if volume.chapters.length > 1}<button
              class="icon"
              aria-label={t("detail.showChapters", { title: volume.title })}
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
            title={t("detail.countPages")}
            disabled={volume.chapters.some((c) => counting.includes(c.url))}
            onclick={() => countPages(volume)}><Search size={17} /></button
          ><button
            class="icon"
            aria-label={t("detail.downloadVolume", { title: volume.title })}
            title={t("detail.downloadVolume", { title: volume.title })}
            disabled={queueing}
            onclick={() => enqueue([volume])}
            ><ArrowDownToLine size={19} /></button
          >
        </div>
        {#if current.expanded.includes(volume.title)}<div class="chapter-list">
            {#each volume.chapters as chapter}<div>
                <span>{chapter.title}</span><span
                  >{chapter.page_count != null
                    ? `${chapter.page_count} ${t("downloads.pages")}`
                    : `— ${t("downloads.pages")}`}</span
                >
                <button
                  class="icon"
                  title={t("detail.countPages")}
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
