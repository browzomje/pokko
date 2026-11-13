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
  import type { Manga, MangaPage, Volume, Chapter, Settings } from "../types";
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
  function isChapterSelected(volume: Volume, chapter: Chapter): boolean {
    if (current.selected.includes(volume.title)) return true;
    return (current.selectedChapters || []).includes(chapter.url);
  }

  function isVolumeFullySelected(volume: Volume): boolean {
    if (current.selected.includes(volume.title)) return true;
    if (!volume.chapters.length) return false;
    const chSet = new Set(current.selectedChapters || []);
    return volume.chapters.every((c) => chSet.has(c.url));
  }

  function isVolumeIndeterminate(volume: Volume): boolean {
    if (isVolumeFullySelected(volume)) return false;
    const chSet = new Set(current.selectedChapters || []);
    return volume.chapters.some((c) => chSet.has(c.url));
  }

  function toggleVolume(volume: Volume) {
    if (!current.selectedChapters) current.selectedChapters = [];
    const chUrls = volume.chapters.map((c) => c.url);
    const fullySelected = isVolumeFullySelected(volume);

    if (fullySelected || isVolumeIndeterminate(volume)) {
      current.selected = current.selected.filter((t) => t !== volume.title);
      current.selectedChapters = current.selectedChapters.filter(
        (u) => !chUrls.includes(u),
      );
    } else {
      if (!current.selected.includes(volume.title)) {
        current.selected = [...current.selected, volume.title];
      }
      const existing = new Set(current.selectedChapters);
      for (const u of chUrls) existing.add(u);
      current.selectedChapters = Array.from(existing);
    }
  }

  function toggleChapter(volume: Volume, chapter: Chapter) {
    if (!current.selectedChapters) current.selectedChapters = [];
    const selected = isChapterSelected(volume, chapter);
    const chUrls = volume.chapters.map((c) => c.url);

    if (selected) {
      current.selected = current.selected.filter((t) => t !== volume.title);
      const baseList =
        current.selectedChapters.length > 0 ? current.selectedChapters : chUrls;
      current.selectedChapters = baseList.filter((u) => u !== chapter.url);
    } else {
      current.selectedChapters = [...current.selectedChapters, chapter.url];
      const set = new Set(current.selectedChapters);
      if (volume.chapters.every((c) => set.has(c.url))) {
        if (!current.selected.includes(volume.title)) {
          current.selected = [...current.selected, volume.title];
        }
      }
    }
  }

  let selection = $derived.by(() => {
    const selectedChapterSet = new Set(current.selectedChapters || []);
    const selectedVolumeSet = new Set(current.selected || []);

    return visibleVolumes.flatMap((volume) => {
      if (selectedVolumeSet.has(volume.title)) {
        return [volume];
      }
      const selectedChs = volume.chapters.filter((c) =>
        selectedChapterSet.has(c.url),
      );
      if (selectedChs.length === volume.chapters.length) {
        return [volume];
      }
      if (selectedChs.length > 0) {
        return [
          {
            title:
              selectedChs.length === 1
                ? `${volume.title} - ${selectedChs[0].title}`
                : `${volume.title} (${selectedChs.length} ${t("downloads.chapters")})`,
            chapters: selectedChs,
          },
        ];
      }
      return [];
    });
  });

  let totalSelectedChaptersCount = $derived.by(() => {
    let count = 0;
    const selectedChapterSet = new Set(current.selectedChapters || []);
    for (const v of visibleVolumes) {
      if (current.selected.includes(v.title)) {
        count += v.chapters.length;
      } else {
        count += v.chapters.filter((c) => selectedChapterSet.has(c.url)).length;
      }
    }
    return count;
  });

  let totalVisibleChaptersCount = $derived.by(() => {
    return visibleVolumes.reduce((acc, v) => acc + v.chapters.length, 0);
  });

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
        {#if current.detail.manga.source_name || current.detail.manga.source_id}{current
            .detail.manga.source_name || current.detail.manga.source_id} · {/if}{current.detail.volumes.reduce(
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
        use:partialSelection={totalSelectedChaptersCount > 0 &&
          totalSelectedChaptersCount < totalVisibleChaptersCount}
        checked={totalVisibleChaptersCount > 0 &&
          totalSelectedChaptersCount === totalVisibleChaptersCount}
        onchange={() => {
          if (current?.detail) {
            if (totalSelectedChaptersCount === totalVisibleChaptersCount) {
              current.selected = [];
              current.selectedChapters = [];
            } else {
              current.selected = visibleVolumes.map((v) => v.title);
              current.selectedChapters = visibleVolumes.flatMap((v) =>
                v.chapters.map((c) => c.url),
              );
            }
          }
        }}
      /> {t("detail.selectAll")}</label
    >
    <div>
      <button
        class="primary"
        disabled={totalSelectedChaptersCount === 0 || queueing}
        onclick={() => enqueue(selection)}
        ><Download size={16} />
        {queueing ? "…" : t("detail.downloadSelected")} ({totalSelectedChaptersCount})</button
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
        class:selected={isVolumeFullySelected(volume) || isVolumeIndeterminate(volume)}
      >
        <div class="volume-main">
          <label class="volume-select"
            ><input
              type="checkbox"
              use:partialSelection={isVolumeIndeterminate(volume)}
              checked={isVolumeFullySelected(volume)}
              onchange={() => toggleVolume(volume)}
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
            {#each volume.chapters as chapter}<div
                class="chapter-row"
                class:selected={isChapterSelected(volume, chapter)}
              >
                <label class="chapter-select">
                  <input
                    type="checkbox"
                    checked={isChapterSelected(volume, chapter)}
                    onchange={() => toggleChapter(volume, chapter)}
                  />
                  <span>{chapter.title}</span>
                </label>
                <span
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
                  title={t("detail.downloadVolume", { title: chapter.title })}
                  aria-label={t("detail.downloadVolume", { title: chapter.title })}
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
