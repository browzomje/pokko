<script lang="ts">
  import { api, message as msg } from "./lib/api";
  import { onMount, onDestroy } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import {
    ChevronLeft,
    ChevronRight,
    Download,
    Upload,
    LoaderCircle,
    X,
    Search,
    Globe,
    Check,
    SlidersHorizontal,
    Images,
    BookOpen,
    Layers,
    FileCheck,
    RotateCcw,
    ExternalLink,
    Shield,
    MousePointerClick,
    Sparkles,
    Play,
    CheckCircle2,
    FolderOpen,
    Info,
    AlertTriangle,
  } from "lucide-svelte";
  import SourceForm from "./SourceForm.svelte";
  import Select from "./Select.svelte";
  import LanguageSelect from "./lib/components/LanguageSelect.svelte";
  import SampleInspector from "./components/SampleInspector.svelte";
  import SampleBrowser from "./components/SampleBrowser.svelte";
  import LabGuide from "./components/LabGuide.svelte";
  import FieldLabel from "./components/FieldLabel.svelte";
  import {
    fieldForStep,
    inferSearchPath,
    inferSearchPathFromHtml,
    stageSignature,
  } from "./lib/source-config";
  import { flag } from "./lib/presentation";
  import { t } from "./lib/i18n";
  import { getLanguage, getLanguageLabel } from "./lib/languages";
  import type { Inspector, InspectedElement } from "./lab-preview";
  import type { Source } from "./types";

  type Sample = {
    id: string;
    source: Source;
    url: string;
    html: string;
    created: number;
    resources: Record<string, string>;
    warnings: string[];
  };
  type SampleInfo = {
    id: string;
    url: string;
    created: number;
    resources: number;
  };
  type Extraction = {
    items: { title: string; url: string; group?: string }[];
    description: string;
    has_next: boolean;
    next_url?: string;
  };

  let {
    source = $bindable(),
    onSave,
    onBack,
    sources,
    onNew: _onNew,
  }: {
    source: Source;
    onSave: (source: Source) => Promise<void>;
    onBack: () => void;
    sources: Source[];
    onSelect?: (source: Source) => void;
    onNew?: () => void;
  } = $props();

  const builtInIds = ["mw", "mf", "mangaread", "mangapill", "mangatown"];

  let url = $state("");
  let testQuery = $state("");
  let assets = $state(false);
  let offline = $state(true);
  let samples = $state<SampleInfo[]>([]);
  let sample = $state<Sample | null>(null);
  let sampleId = $state("");
  let html = $state("");
  let kind = $state("setup");
  let busy = $state("");
  let progress = $state("");
  let failure = $state("");
  let status = $state(t("lab.ready"));
  let token = $state("");
  let inspector: Inspector | null = null;
  let inspected = $state<InspectedElement | null>(null);
  let selectorField = $state("result_selector");
  let extraction = $state<Extraction | null>(null);
  let tested = $state("");
  let downloaded = $state("");
  let stageSamples = $state<Record<string, Sample>>({});
  let stageUrls = $state<Record<string, string>>({});
  let passed = $state<Record<string, { signature: string; count: number }>>({});
  let validTest = $derived(
    tested === stageSignature(source, kind, html, sample?.url || ""),
  );

  // Guided wizard states
  let skipSearch = $state(false);
  let chaptersConfirmed = $state(false);
  let imagesConfirmed = $state(false);
  let completedStages = $state<string[]>([]);
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  // Yellow visual flash (Vesta style) when step changes or co-pilot detects elements
  let stepFlash = $state(false);
  let copilotFlash = $state(false);
  let stepFlashTimer: ReturnType<typeof setTimeout> | null = null;
  let copilotFlashTimer: ReturnType<typeof setTimeout> | null = null;

  function triggerStepFlash() {
    stepFlash = false;
    if (stepFlashTimer) clearTimeout(stepFlashTimer);
    requestAnimationFrame(() => {
      stepFlash = true;
      stepFlashTimer = setTimeout(() => {
        stepFlash = false;
      }, 1600);
    });
  }

  function triggerCopilotFlash() {
    copilotFlash = false;
    if (copilotFlashTimer) clearTimeout(copilotFlashTimer);
    requestAnimationFrame(() => {
      copilotFlash = true;
      copilotFlashTimer = setTimeout(() => {
        copilotFlash = false;
      }, 1600);
    });
  }

  // Co-pilot states & field test
  let detectedResults = $state<{ title: string; url: string; group?: string }[]>([]);
  let detectedChapters = $state<{ title: string; url: string; group?: string }[]>([]);
  let selectedTestChapterUrl = $state("");
  let generalizationQuery = $state("");
  let generalizationResults = $state<{ title: string; url: string }[] | null>(null);
  let generalizationBusy = $state(false);
  let downloadSuccessInfo = $state<{ pages: number; path: string } | null>(null);
  let unlistenProgress: (() => void) | null = null;

  // Firefox companion status
  let firefoxInstalled = $state(false);
  let firefoxRunning = $state(false);
  let ublockInstalled = $state(false);

  const stepOrder = ["setup", "search", "detail", "reader", "review"];

  let userEditedName = $state(false);

  function normalizeUrl(raw: string): string {
    let val = (raw || "").trim();
    if (!val) return "";
    if (!val.startsWith("http://") && !val.startsWith("https://")) {
      val = "https://" + val;
    }
    return val;
  }

  function isValidSiteUrl(raw: string): boolean {
    const norm = normalizeUrl(raw);
    try {
      const u = new URL(norm);
      return (u.protocol === "https:" || u.protocol === "http:") && u.hostname.includes(".");
    } catch {
      return false;
    }
  }

  function deriveNameFromUrl(raw: string): string {
    try {
      const u = new URL(normalizeUrl(raw));
      let host = u.hostname.replace(/^www\./i, "");
      const domainPart = host.split(".")[0];
      if (domainPart) {
        return domainPart.charAt(0).toUpperCase() + domainPart.slice(1);
      }
    } catch {}
    return "";
  }

  function onUrlInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    source.base_url = val;
    if (!userEditedName) {
      const derived = deriveNameFromUrl(val);
      if (derived) {
        source.name = derived;
      }
    }
  }

  function onNameInput(e: Event) {
    userEditedName = true;
    source.name = (e.target as HTMLInputElement).value;
  }

  let step1Ready = $derived(isValidSiteUrl(source.base_url));

  let setupReady = $derived.by(() => {
    return isValidSiteUrl(source.base_url);
  });

  let searchReady = $derived(
    skipSearch || (!!source.result_selector && (extraction?.items.length || 0) > 0),
  );

  let allPassed = $derived(
    ["detail", "reader"].every((k) => {
      const s = stageSamples[k];
      return (
        !!s &&
        passed[k]?.count > 0 &&
        passed[k].signature === stageSignature(source, k, s.html, s.url)
      );
    }) && (skipSearch || (passed["search"]?.count || 0) > 0),
  );

  // Template pre-compilation
  let templateOptions = $derived.by(() => {
    const builtIns = sources.filter((s) =>
      builtInIds.includes(s.id),
    );
    const customs = sources.filter(
      (s) => s.adapter === "generic" && !builtInIds.includes(s.id) && s.id !== source.id,
    );

    const list: { value: string; label: string; disabled?: boolean }[] = [
      { value: "", label: t("lab.templatePlaceholder") },
    ];

    if (builtIns.length > 0) {
      list.push({ value: "--group-builtin", label: t("lab.builtinGroup"), disabled: true });
      for (const s of builtIns) {
        list.push({ value: s.id, label: s.name });
      }
    }

    if (customs.length > 0) {
      list.push({ value: "--group-custom", label: t("lab.customGroup"), disabled: true });
      for (const s of customs) {
        list.push({ value: s.id, label: s.name });
      }
    }

    return list;
  });

  function loadFromTemplate(templateId: string) {
    if (!templateId || templateId.startsWith("--group")) return;
    const template = sources.find((s) => s.id === templateId);
    if (!template) return;

    source = {
      ...source,
      category: template.category,
      language: template.language === "both" ? "it" : template.language,
      min_delay_ms: template.min_delay_ms || 500,
      search_path: template.search_path || "",
      result_selector: template.result_selector || "",
      chapter_selector: template.chapter_selector || "",
      image_selector: template.image_selector || "",
      description_selector: template.description_selector || "",
      author_selector: template.author_selector || "",
      next_selector: template.next_selector || "",
      reader_next_selector: template.reader_next_selector || "",
      note: `Modello basato su ${template.name}`,
    };

    if (!source.name.trim() || source.name === "Nuova fonte") {
      source.name = `Copia di ${template.name}`;
    }

    status = `${template.name} ✓`;
  }

  // Firefox companion integration with Marionette & uBlock Origin
  async function checkFirefoxStatus() {
    try {
      const s = await api<{ installed: boolean; running: boolean; ublock_installed: boolean }>(
        "lab_firefox_status",
      );
      firefoxInstalled = s.installed;
      firefoxRunning = s.running;
      ublockInstalled = s.ublock_installed;
    } catch {}
  }

  async function launchFirefox(targetUrl?: string) {
    const toOpen = targetUrl || url || source.base_url;
    if (!toOpen || !toOpen.startsWith("http")) return;
    try {
      status = t("lab.startingFirefox");
      const launched = await api<boolean>("lab_firefox_launch", { url: toOpen });
      if (launched) {
        firefoxRunning = true;
        startFirefoxPolling();
        status = t("lab.firefoxActive");
      }
    } catch (e) {
      failure = msg(e);
    }
  }

  async function startFirefoxPicker() {
    try {
      await api("lab_firefox_start_picker");
      status = t("lab.clickElementInPreview");
    } catch (e) {
      failure = msg(e);
    }
  }

  interface LastNav {
    from_url: string;
    from_html: string;
    clicked_selector: string;
    clicked_href: string;
    clicked_text: string;
    search_input_value: string;
    timestamp: number;
  }

  let firefoxLiveUrl = $state("");
  let mangaNavDetected = $state<{ title: string; selector: string; url: string } | null>(null);
  let lastProcessedNavTimestamp = 0;

  function startFirefoxPolling() {
    if (pollInterval) clearInterval(pollInterval);
    pollInterval = setInterval(async () => {
      if (kind === "review" || kind === "setup") return;
      try {
        // Poll current URL from Firefox without automatic trigger
        const current = await api<string>("lab_firefox_get_url");
        if (current && current.startsWith("http")) {
          firefoxLiveUrl = current;
          if (!url) url = current;
        }

        // Try injecting anti-popup & navigation helper
        void api("lab_firefox_inject_picker").catch(() => {});

        // Check if an element was picked in Firefox (inspect mode)
        const picked = await api<string | null>("lab_firefox_get_picked");
        if (picked) {
          await applyPickedSelector(picked);
        }

        // Check transparent last navigation data from user natural clicks in Firefox!
        const lastNav = await api<LastNav | null>("lab_firefox_get_last_nav");
        if (lastNav && lastNav.timestamp && lastNav.timestamp > lastProcessedNavTimestamp) {
          lastProcessedNavTimestamp = lastNav.timestamp;
          await handleFirefoxNavEvent(lastNav, current || url);
        }
      } catch {}
    }, 1500);
  }

  async function handleFirefoxNavEvent(lastNav: LastNav, currentUrl: string) {
    if (!lastNav) return;

    if (lastNav.search_input_value && !testQuery.trim()) {
      testQuery = lastNav.search_input_value;
    }

    if (kind === "search") {
      // User clicked a manga link or navigated
      const isMangaLink =
        lastNav.clicked_href?.includes("/manga/") ||
        lastNav.clicked_href?.includes("/comic/") ||
        lastNav.clicked_href?.includes("/series/") ||
        currentUrl.includes("/manga/") ||
        currentUrl.includes("/series/") ||
        currentUrl.includes("/comic/");

      if (isMangaLink && currentUrl !== source.base_url) {
        mangaNavDetected = {
          title: lastNav.clicked_text || "Manga",
          selector: lastNav.clicked_selector,
          url: currentUrl,
        };
        if (lastNav.clicked_selector && !source.result_selector) {
          source = { ...source, result_selector: lastNav.clicked_selector };
        }
        status = t("lab.mangaOpenedInFirefox", { title: lastNav.clicked_text || "Manga" });
      }
    } else if (kind === "detail") {
      // User is on series page and clicked a chapter in Firefox!
      const isChapterLink =
        lastNav.clicked_href?.includes("/leer/") ||
        lastNav.clicked_href?.includes("/chapter") ||
        lastNav.clicked_href?.includes("/read/") ||
        lastNav.clicked_href?.includes("/ch-") ||
        currentUrl.includes("/leer/") ||
        currentUrl.includes("/chapter") ||
        currentUrl.includes("/read/") ||
        currentUrl.includes("/ch-");

      if (isChapterLink && currentUrl !== lastNav.from_url) {
        if (lastNav.clicked_selector) {
          source = { ...source, chapter_selector: lastNav.clicked_selector };
        }
        chaptersConfirmed = true;
        markStageCompleted("detail");
        selectedTestChapterUrl = currentUrl;
        url = currentUrl;
        status = t("lab.chapterOpenedInFirefox", { title: lastNav.clicked_text || "Capitolo" });
        step("reader");
        setTimeout(() => {
          void onHereReader();
        }, 500);
      }
    }
  }

  async function applyPickedSelector(selector: string) {
    if (!selector) return;
    if (kind === "search") {
      source = { ...source, result_selector: selector };
      if (!sample) await capture();
      await extract();
      status = t("lab.detectedResults", { count: extraction?.items.length || 1 });
      // Keep user on search step, showing the confirmation box so they decide when to proceed!
    } else if (kind === "detail") {
      source = { ...source, chapter_selector: selector };
      if (!sample) await capture();
      await extract();
      status = t("lab.detectedChapters", { count: extraction?.items.length || 0 });
    } else if (kind === "reader") {
      source = { ...source, image_selector: selector };
      if (!sample) await capture();
      await extract();
      status = t("lab.detectedPages", { count: extraction?.items.length || 0 });
    }
  }

  let taskDepth = 0;
  async function task(label: string, work: () => Promise<void>) {
    if (taskDepth > 0) {
      busy = label;
      return await work();
    }
    busy = label;
    taskDepth++;
    failure = "";
    progress = "";
    downloaded = "";
    token = `lab-${crypto.randomUUID()}`;
    try {
      await work();
    } catch (e) {
      failure = msg(e);
    } finally {
      taskDepth--;
      if (taskDepth === 0) {
        busy = "";
        progress = "";
      }
    }
  }

  async function refresh() {
    samples = await api<SampleInfo[]>("lab_samples", { sourceId: source.id });
  }

  function setSample(value: Sample) {
    sample = value;
    stageSamples = { ...stageSamples, [kind]: value };
    sampleId = value.id;
    url = value.url;
    html = value.html;
    inspected = null;
    extraction = null;
    tested = "";
    downloaded = "";
    inspector = null;
  }

  function step(value: string) {
    if (sample) {
      stageSamples = {
        ...stageSamples,
        [kind]: { ...$state.snapshot(sample), html },
      };
      stageUrls = { ...stageUrls, [kind]: url };
    }
    kind = value;
    selectorField = fieldForStep(value);
    extraction = null;
    tested = "";
    inspected = null;
    sample = stageSamples[value] || null;
    html = sample?.html || "";
    sampleId = sample?.id || "";
    url =
      stageUrls[value] ||
      source.workflow?.find((s) => s.kind === value)?.url ||
      (firefoxLiveUrl && firefoxLiveUrl.startsWith("http") ? firefoxLiveUrl : "");
    triggerStepFlash();
  }

  function markStageCompleted(stage: string) {
    if (!completedStages.includes(stage)) {
      completedStages = [...completedStages, stage];
    }
  }

  let lastOpenedBaseUrl = $state(source.base_url);

  // Step 1: Start recording
  async function startRecording() {
    const normalized = normalizeUrl(source.base_url);
    source.base_url = normalized;
    if (!source.name.trim() || source.name.trim() === "Nuova fonte") {
      source.name = deriveNameFromUrl(source.base_url) || "Fonte";
    }

    if (lastOpenedBaseUrl && lastOpenedBaseUrl !== normalized) {
      completedStages = ["setup"];
      stageSamples = {};
      extraction = null;
      sample = null;
      sampleId = "";
      url = normalized;
      html = "";
      chaptersConfirmed = false;
      imagesConfirmed = false;
      source.search_path = "";
      source.result_selector = "";
      source.chapter_selector = "";
      source.image_selector = "";
    } else {
      markStageCompleted("setup");
    }
    lastOpenedBaseUrl = normalized;

    if (firefoxInstalled) {
      void launchFirefox(source.base_url);
    }
    step(skipSearch ? "detail" : "search");
  }

  // Step 2: "Ho cercato"
  async function onSearched() {
    await task(t("lab.iSearched"), async () => {
      try {
        const ffUrl = await api<string>("lab_firefox_get_url");
        if (ffUrl && ffUrl.startsWith("http")) url = ffUrl;
      } catch {}

      if (!url.trim() && firefoxLiveUrl && firefoxLiveUrl.startsWith("http")) {
        url = firefoxLiveUrl;
      }
      if (!url.trim()) throw new Error(t("lab.searchUrlRequired"));

      let lastNav: LastNav | null = null;
      try {
        lastNav = await api<LastNav | null>("lab_firefox_get_last_nav");
      } catch {}

      if (lastNav?.search_input_value && !testQuery.trim()) {
        testQuery = lastNav.search_input_value;
      }

      if (testQuery.trim()) {
        const path = inferSearchPath(url, testQuery);
        if (path) source = { ...source, search_path: path };
      }

      await capture();

      // If search_path was not inferred from URL, infer from HTML form or search input
      if (!source.search_path && (html || lastNav?.from_html)) {
        const inferred = inferSearchPathFromHtml(
          html || lastNav?.from_html || "",
          source.base_url || url,
        );
        if (inferred) {
          source = {
            ...source,
            search_path: inferred.searchPath,
            ...(inferred.resultSelector && !source.result_selector
              ? { result_selector: inferred.resultSelector }
              : {}),
          };
        }
      }

      // Check if current page is already a series page with chapters!
      if (html) {
        try {
          const autoChap = await api<{ found: boolean; selector: string; count: number; sample_titles: string[] }>(
            "lab_detect_chapters",
            { html, url },
          );
          if (autoChap.found && autoChap.count >= 2) {
            if (lastNav?.clicked_selector) {
              source = { ...source, result_selector: lastNav.clicked_selector };
            } else if (url.includes("/manga/")) {
              source = { ...source, result_selector: "a[href*=\"/manga/\"]" };
            }
            source = { ...source, chapter_selector: autoChap.selector };
            markStageCompleted("search");
            stageUrls["detail"] = url;
            step("detail");
            await extract();
            status = t("lab.alreadyOnMangaPage", { count: autoChap.count });
            return;
          }
        } catch {}
      }

      // If on search page, detect search results automatically!
      if (!source.result_selector && html) {
        try {
          const autoRes = await api<{ found: boolean; selector: string; count: number; sample_titles: string[] }>(
            "lab_detect_search_results",
            { html, url },
          );
          if (autoRes.found && autoRes.count >= 1) {
            source = { ...source, result_selector: autoRes.selector };
          }
        } catch {}
      }
      if (!source.result_selector && lastNav?.clicked_selector) {
        source = { ...source, result_selector: lastNav.clicked_selector };
      }

      selectorField = "result_selector";
      await extract();
      status = t("lab.pageAcquiredClickManga");
    });
  }

  // Step 2 shortcut: user already navigated to the manga page in Firefox
  async function onOpenedMangaInFirefox() {
    await task(t("lab.detectingFromFirefox"), async () => {
      try {
        const ffUrl = await api<string>("lab_firefox_get_url");
        if (ffUrl && ffUrl.startsWith("http")) url = ffUrl;
      } catch {}

      if (!url.trim() && firefoxLiveUrl && firefoxLiveUrl.startsWith("http")) {
        url = firefoxLiveUrl;
      }
      if (!url.trim()) throw new Error(t("lab.seriesUrlRequired"));

      let lastNav: LastNav | null = null;
      try {
        lastNav = await api<LastNav | null>("lab_firefox_get_last_nav");
      } catch {}

      if (lastNav?.search_input_value && !testQuery.trim()) {
        testQuery = lastNav.search_input_value;
      }

      // If search_path not set, infer from from_html of the previous search page!
      if (!source.search_path && lastNav?.from_html) {
        const inferred = inferSearchPathFromHtml(
          lastNav.from_html,
          source.base_url || url,
        );
        if (inferred) {
          source = { ...source, search_path: inferred.searchPath };
          if (inferred.resultSelector && !source.result_selector) {
            source = { ...source, result_selector: inferred.resultSelector };
          }
        }
      }

      if (lastNav?.clicked_selector && !source.result_selector) {
        source = { ...source, result_selector: lastNav.clicked_selector };
      } else if (!source.result_selector && url.includes("/manga/")) {
        source = { ...source, result_selector: "a[href*=\"/manga/\"]" };
      }

      markStageCompleted("search");
      stageUrls["detail"] = url;
      step("detail");
      await onHereSeries();
    });
  }

  // Step 3: "Sono qui" on Series page with Heuristic Auto-Detection!
  async function onHereSeries() {
    await task(t("lab.imHereChapters"), async () => {
      try {
        const ffUrl = await api<string>("lab_firefox_get_url");
        if (ffUrl && ffUrl.startsWith("http")) url = ffUrl;
      } catch {}

      if (!url.trim() && firefoxLiveUrl && firefoxLiveUrl.startsWith("http")) {
        url = firefoxLiveUrl;
      }
      if (!url.trim()) throw new Error(t("lab.seriesUrlRequired"));

      // Also try inferring search_path from lastNav if still empty
      if (!source.search_path) {
        try {
          const lastNav = await api<LastNav | null>("lab_firefox_get_last_nav");
          if (lastNav?.from_html) {
            const inferred = inferSearchPathFromHtml(
              lastNav.from_html,
              source.base_url || url,
            );
            if (inferred) {
              source = { ...source, search_path: inferred.searchPath };
              if (inferred.resultSelector && !source.result_selector) {
                source = { ...source, result_selector: inferred.resultSelector };
              }
            }
          }
        } catch {}
      }

      await capture();
      selectorField = "chapter_selector";
      chaptersConfirmed = false;

      // Run automatic heuristic detection for chapters
      if (html) {
        try {
          const auto = await api<{ found: boolean; selector: string; count: number; sample_titles: string[] }>(
            "lab_detect_chapters",
            { html, url },
          );
          if (auto.found && auto.count >= 2) {
            source = { ...source, chapter_selector: auto.selector };
            await extract();
            status = t("lab.detectedChapters", { count: auto.count });
            return;
          }
        } catch {}
      }

      status = t("lab.pageAcquiredClickChapter");
    });
  }

  function confirmChapters() {
    chaptersConfirmed = true;
    markStageCompleted("detail");
    status = t("lab.detectedChapters", { count: extraction?.items.length || 0 });
    setTimeout(() => step("reader"), 600);
  }

  function retryChapters() {
    chaptersConfirmed = false;
    source.chapter_selector = "";
    extraction = null;
    status = t("lab.clickAnotherChapter");
  }

  // Step 4: "Sono qui" on Reader page
  async function onHereReader() {
    await task(t("lab.imHereScans"), async () => {
      try {
        const ffUrl = await api<string>("lab_firefox_get_url");
        if (ffUrl && ffUrl.startsWith("http")) url = ffUrl;
      } catch {}

      if (!url.trim() && firefoxLiveUrl && firefoxLiveUrl.startsWith("http")) {
        url = firefoxLiveUrl;
      }
      if (!url.trim() && selectedTestChapterUrl) {
        url = selectedTestChapterUrl;
      }
      if (!url.trim()) throw new Error(t("lab.readerUrlRequired"));

      await capture();
      selectorField = "image_selector";
      imagesConfirmed = false;

      if (!source.image_selector && html) {
        const detected = autoDetectScanImages(html);
        if (detected) source = { ...source, image_selector: detected };
      }
      await extract();
      status = t("lab.detectedPages", { count: extraction?.items.length || 0 });
    });
  }

  function autoDetectScanImages(htmlContent: string): string {
    const doc = new DOMParser().parseFromString(htmlContent, "text/html");
    const candidates = [
      ".reading-content img",
      ".chapter-content img",
      "#readerarea img",
      ".read_img img",
      "img#gohere",
      "img.page-image",
      "img.pages__img",
      "img.js-page",
      "img.scan",
      ".entry-content img",
      ".page-break img",
      "picture img",
      ".lc-pages img",
      "#lcPages img",
      "main.lc-pages img",
    ];
    for (const sel of candidates) {
      const matches = doc.querySelectorAll(sel);
      if (matches.length >= 2) return sel;
    }
    const lazyImages = doc.querySelectorAll("img[data-src], img[data-original], img[data-lazy-src]");
    if (lazyImages.length >= 2) {
      const first = lazyImages[0];
      if (first.className) {
        const cls = first.className
          .trim()
          .split(/\s+/)
          .find((c) => !/^(lazy|lazyload|dracula|darkreader|img-fluid|img-responsive)/i.test(c));
        if (cls) return `img.${cls}`;
      }
      return "img[data-src]";
    }
    // Generic structural grouping: find repeated images outside nav/header/footer/sidebar
    const classCounts = new Map<string, number>();
    const parentCounts = new Map<string, number>();
    const allImgs = Array.from(doc.querySelectorAll("img"));
    for (const img of allImgs) {
      if (img.closest("header, nav, footer, aside, .sidebar, .comments, .widget")) continue;
      const src = (img.getAttribute("data-src") || img.getAttribute("src") || "").toLowerCase();
      if (!src || /(logo|icon|avatar|banner|ads|pixel|1x1|\.svg)/i.test(src)) continue;
      const classes = (img.className || "")
        .trim()
        .split(/\s+/)
        .filter((c) => c && !/^(lazy|lazyload|dracula|darkreader|img-fluid|img-responsive|__pokko)/i.test(c));
      if (classes[0]) {
        const sel = `img.${classes[0]}`;
        classCounts.set(sel, (classCounts.get(sel) || 0) + 1);
      }
      const parent = img.parentElement;
      if (parent && parent.className) {
        const pCls = parent.className
          .trim()
          .split(/\s+/)
          .find((c) => c && !/^(dracula|darkreader|__pokko|row|col|container)/i.test(c));
        if (pCls) {
          const pSel = `.${pCls} img`;
          parentCounts.set(pSel, (parentCounts.get(pSel) || 0) + 1);
        }
      }
    }
    for (const [sel, count] of classCounts.entries()) {
      if (count >= 3) return sel;
    }
    for (const [sel, count] of parentCounts.entries()) {
      if (count >= 3) return sel;
    }
    return "img";
  }

  function confirmImages() {
    imagesConfirmed = true;
    markStageCompleted("reader");
    status = t("lab.detectedPages", { count: extraction?.items.length || 0 });
    setTimeout(() => step("review"), 600);
  }

  function confirmImagesAndGo() {
    imagesConfirmed = true;
    markStageCompleted("reader");
    status = t("lab.detectedPages", { count: extraction?.items.length || 0 });
    step("review");
  }

  async function confirmSearchResultAndGo(targetUrl?: string) {
    const toOpen = targetUrl || extraction?.items[0]?.url;
    if (!toOpen) return;
    markStageCompleted("search");
    stageUrls["detail"] = toOpen;
    url = toOpen;
    if (firefoxRunning) {
      await api("lab_firefox_navigate", { url: toOpen }).catch(() => {});
    }
    step("detail");
    await onHereSeries();
  }

  async function confirmChaptersAndGo(chapterUrl?: string) {
    chaptersConfirmed = true;
    markStageCompleted("detail");
    const toOpen =
      chapterUrl || extraction?.items[0]?.url || detectedChapters[0]?.url;
    if (toOpen) {
      selectedTestChapterUrl = toOpen;
      stageUrls["reader"] = toOpen;
      url = toOpen;
      if (firefoxRunning) {
        await api("lab_firefox_navigate", { url: toOpen }).catch(() => {});
      }
    }
    step("reader");
    await onHereReader();
  }

  async function runLiveTestDownload() {
    await task("Download di prova", async () => {
      downloadSuccessInfo = null;
      downloaded = "";

      let targetSample = stageSamples["reader"] || sample;
      let targetHtml = targetSample?.html || html;

      if (selectedTestChapterUrl && targetSample?.url !== selectedTestChapterUrl) {
        progress = "Caricamento capitolo selezionato…";
        if (firefoxRunning) {
          await api("lab_firefox_navigate", { url: selectedTestChapterUrl }).catch(() => {});
        }
        const s = await api<Sample>("lab_capture", {
          url: selectedTestChapterUrl,
          source: $state.snapshot(source),
          assets: false,
          token,
        });
        targetSample = s;
        targetHtml = s.html;
      }

      if (!targetSample) {
        throw new Error("Nessun capitolo disponibile per il download di prova.");
      }

      progress = "Download scan in corso…";
      const result = await api<{
        output: string;
        pages: number;
        bytes: number;
      }>("lab_download", {
        request: {
          source: $state.snapshot(source),
          sampleId: targetSample.id,
          html: targetHtml,
          output: "",
          offline: false,
          token,
        },
      });

      downloaded = result.output;
      downloadSuccessInfo = { pages: result.pages, path: result.output };
      status = `Capitolo verificato: ${result.pages} pagine ✓`;
    });
  }

  async function saveAndEnable() {
    source.enabled = true;
    await saveSource();
  }

  async function testGeneralization() {
    if (!generalizationQuery.trim()) return;
    generalizationBusy = true;
    generalizationResults = null;
    failure = "";
    try {
      let searchUrl = "";
      const base = source.base_url.replace(/\/$/, "");
      const q = encodeURIComponent(generalizationQuery.trim());
      if (source.search_path.includes("{query}")) {
        searchUrl = `${base}/${source.search_path.replace(/^\//, "").replace("{query}", q)}`;
      } else {
        searchUrl = `${base}/${source.search_path.replace(/^\//, "")}${q}`;
      }
      if (firefoxRunning) {
        void api("lab_firefox_navigate", { url: searchUrl });
      }
      const s = await api<Sample>("lab_capture", {
        url: searchUrl,
        source: $state.snapshot(source),
        assets: false,
        token: `gen-${crypto.randomUUID()}`,
      });
      const ext = await api<Extraction>("lab_extract", {
        source: $state.snapshot(source),
        html: s.html,
        url: s.url,
        kind: "search",
      });
      generalizationResults = ext.items;
    } catch (e) {
      failure = `Verifica fallita: ${msg(e)}`;
    } finally {
      generalizationBusy = false;
    }
  }

  function retryImages() {
    imagesConfirmed = false;
    source.image_selector = "";
    extraction = null;
    status = t("lab.chooseAnother");
  }

  async function inspectElement(value: InspectedElement) {
    inspected = value;
    if (source.adapter !== "generic" || busy) return;

    if (kind === "reader") {
      if (value.tag !== "img") {
        status = t("lab.clickScanImageNotContainer");
        return;
      }
      source = { ...source, image_selector: value.selector };
      await extract();
      status = t("lab.detectedPages", { count: extraction?.items.length || 0 });
      return;
    }

    if (kind === "detail") {
      source = { ...source, chapter_selector: value.linkSelector || value.selector };
      await extract();
      status = t("lab.detectedChapters", { count: extraction?.items.length || 0 });
      return;
    }

    if (kind === "search") {
      source = { ...source, result_selector: value.linkSelector || value.selector };
      await extract();
      markStageCompleted("search");
      status = t("lab.detectedResults", { count: extraction?.items.length || 0 });
      setTimeout(() => step("detail"), 800);
      return;
    }

    if (selectorField !== fieldForStep(kind)) {
      source = { ...source, [selectorField]: value.selector };
      status = t("lab.fieldUpdated");
    }
  }

  async function capture() {
    await task("Acquisizione", async () => {
      const value = await api<Sample>("lab_capture", {
        url,
        source: $state.snapshot(source),
        assets,
        token,
      });
      setSample(value);
      if (kind === "search" && testQuery.trim()) {
        const path = inferSearchPath(value.url, testQuery);
        if (path) source = { ...source, search_path: path };
      }
      await refresh();
    });
  }

  async function extract() {
    if (!sample) return;
    await task("Estrazione", async () => {
      const signature = stageSignature(source, kind, html, sample?.url || "");
      extraction = await api<Extraction>("lab_extract", {
        source: $state.snapshot(source),
        html,
        url: sample!.url,
        kind,
      });
      tested = signature;
      stageSamples = {
        ...stageSamples,
        [kind]: { ...$state.snapshot(sample!), html },
      };
      passed = {
        ...passed,
        [kind]: { signature, count: extraction.items.length },
      };
      if (kind === "search" && extraction?.items) {
        detectedResults = extraction.items;
      } else if (kind === "detail" && extraction?.items) {
        detectedChapters = extraction.items;
        if (!selectedTestChapterUrl && detectedChapters.length > 0) {
          selectedTestChapterUrl = detectedChapters[0].url;
        }
      }
      if (extraction?.items && extraction.items.length > 0) {
        triggerCopilotFlash();
      }
      const selector = source[selectorField as keyof Source];
      if (typeof selector === "string") inspector?.highlight(selector);
    });
  }

  async function download() {
    if (!sample || kind !== "reader" || !validTest || !extraction?.items.length)
      return;
    await task("Download di prova", async () => {
      const output = await open({
        directory: true,
        multiple: false,
        title: "Cartella del PDF di prova",
      });
      if (typeof output !== "string") return;
      const result = await api<{
        output: string;
        pages: number;
        bytes: number;
      }>("lab_download", {
        request: {
          source: $state.snapshot(source),
          sampleId: sample!.id,
          html,
          output,
          offline,
          token,
        },
      });
      downloaded = result.output;
      status = `PDF verificato: ${result.pages} pagine ✓`;
    });
  }

  async function exportConnector() {
    try {
      const data = JSON.stringify($state.snapshot(source), null, 2);
      const filename = `${(source.name || "connettore").toLowerCase().replace(/[^a-z0-9]/g, "-")}.json`;
      const blob = new Blob([data], { type: "application/json" });
      const a = document.createElement("a");
      a.href = URL.createObjectURL(blob);
      a.download = filename;
      a.click();
      status = t("lab.connectorExported");
    } catch (e) {
      failure = `Errore esportazione: ${msg(e)}`;
    }
  }

  async function importConnector() {
    try {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = ".json,application/json";
      input.onchange = async () => {
        const file = input.files?.[0];
        if (!file) return;
        const name = file.name.toLowerCase();
        if (name.endsWith(".zip") || name.endsWith(".rar") || name.endsWith(".tar") || name.endsWith(".cbz")) {
          failure = "I file archivio (ZIP, RAR, TAR, CBZ) non sono consentiti per motivi di sicurezza. Importa solo file connettore .json.";
          return;
        }
        const text = await file.text();
        const parsed = JSON.parse(text) as Source;
        if (!parsed.base_url || !parsed.name) {
          throw new Error(t("lab.invalidConnectorFile"));
        }
        source = {
          ...parsed,
          id: source.id,
        };
        status = `Connettore '${parsed.name}' importato con successo ✓`;
      };
      input.click();
    } catch (e) {
      failure = `Errore importazione: ${msg(e)}`;
    }
  }

  async function saveSource() {
    await task("Salvataggio", async () => {
      source.base_url = normalizeUrl(source.base_url);
      if (!source.name.trim() || source.name.trim() === "Nuova fonte") {
        throw new Error(
          t("lab.nameRequired") ||
            "Inserisci un nome per la fonte prima di salvare.",
        );
      }
      source.enabled = true;
      source.origin = "created";
      await onSave($state.snapshot(source));
      status = t("lab.sourceSavedSuccess");
    });
  }

  onMount(() => {
    let disposed = false;
    void refresh().catch((e) => (failure = msg(e)));
    void checkFirefoxStatus();

    listen<{ token: string; message: string }>("lab-progress", (event) => {
      if (event.payload?.message) {
        progress = event.payload.message;
      }
    }).then((unlisten) => {
      if (!disposed) {
        unlistenProgress = unlisten;
      } else {
        unlisten();
      }
    });

    return () => {
      disposed = true;
      if (stepFlashTimer) clearTimeout(stepFlashTimer);
      if (copilotFlashTimer) clearTimeout(copilotFlashTimer);
      if (unlistenProgress) unlistenProgress();
      if (pollInterval) clearInterval(pollInterval);
      if (busy) void api("lab_cancel", { token }).catch(() => {});
    };
  });

  onDestroy(() => {
    if (stepFlashTimer) clearTimeout(stepFlashTimer);
    if (copilotFlashTimer) clearTimeout(copilotFlashTimer);
    if (unlistenProgress) unlistenProgress();
    if (pollInterval) clearInterval(pollInterval);
  });
</script>

<div class="lab-workspace">
<!-- Top Row: Left = 5 step tabs, Right = Template select + Save draft button (Zero wasted space above tabs) -->
<div class="lab-topbar">
  <div class="lab-topbar-tabs">
    <LabGuide {kind} onStep={step} {completedStages} />
  </div>
  <div class="lab-topbar-actions">
    <div class="template-selector-wrap">
      <Select
        label={t("lab.templatePlaceholder")}
        value=""
        options={templateOptions}
        onchange={loadFromTemplate}
        disabled={!!busy}
      />
    </div>
    <button class="primary" disabled={!!busy} onclick={saveSource}>
      {source.enabled ? t("lab.save") : t("lab.saveDraft")}
    </button>
  </div>
</div>

<!-- Strictly Horizontal Side-by-Side Layout (Left: Wizard 380px, Right: Interactive Preview flex:1) -->
<div class="lab-layout" class:lab-review={kind === "review"}>
  <!-- Left Column: Single-Path Guided Step Wizard -->
  <section class="lab-config" aria-label="Passo guidato">
    {#if kind === "setup"}
      <div class="wizard-card step-card" class:vesta-flash-yellow={stepFlash}>
        <div class="step-badge">{t("lab.step1Badge")}</div>
        <h2>{t("lab.step1Title")}</h2>

        <div class="step-fields">
          <label>
            <FieldLabel text={t("lab.url")} icon={Globe} help={t("lab.urlHelp")} />
            <input
              type="text"
              class="url-placeholder-input"
              value={source.base_url}
              oninput={onUrlInput}
              placeholder="https://example.com"
              required
            />
          </label>

          <label>
            <FieldLabel text={t("lab.name")} icon={Search} help={t("lab.nameHelp")} />
            <input
              value={source.name}
              oninput={onNameInput}
              placeholder={t("lab.namePlaceholder")}
              required
            />
          </label>

          <div class="source-field">
            <FieldLabel text={t("lab.catalog")} icon={Layers} help={t("lab.catalogHelp")} />
            <Select
              label={t("lab.catalog")}
              bind:value={source.category}
              options={[
                { value: "manga", label: t("catalog.manga") },
                { value: "comics", label: t("catalog.comics") },
              ]}
            />
          </div>

          <div class="source-field">
            <FieldLabel text={t("lab.language")} icon={Globe} help={t("lab.languageHelp")} />
            <LanguageSelect bind:value={source.language} />
          </div>

          {#if firefoxInstalled}
            <div class="firefox-shield-banner">
              <Shield size={16} class="shield-green" />
              <span>{t("lab.firefoxShieldBanner")}</span>
            </div>
          {:else}
            <div class="firefox-warning-banner">
              <AlertTriangle size={16} class="shield-amber" />
              <div class="firefox-warning-content">
                <span>{t("lab.firefoxMissingBanner")}</span>
                <a
                  href="https://www.mozilla.org/firefox/new/"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="firefox-download-link"
                >
                  <Download size={14} />
                  <span>{t("lab.downloadFirefoxBtn")}</span>
                </a>
              </div>
            </div>
          {/if}

          <button
            type="button"
            class="primary start-btn"
            disabled={!step1Ready || !!busy}
            title={!step1Ready ? t("lab.step1DisabledTooltip") : ""}
            onclick={startRecording}
          >
            {t("lab.openSite")}
          </button>
        </div>
      </div>

    {:else if kind === "search"}
      <div class="wizard-card step-card" class:vesta-flash-yellow={stepFlash}>
        <div class="step-badge">{t("lab.step2Badge")}</div>
        <h2>{t("lab.step2CopilotTitle")}</h2>

        <div class="animated-hint">
          <Search size={16} class="pulse-icon" />
          <div class="hint-lines">
            <div>{t("lab.step2Instruction1")}</div>
            <small>{t("lab.step2Instruction2")}</small>
          </div>
        </div>

        {#if mangaNavDetected}
          <div class="confirmation-box copilot-box" class:vesta-flash-yellow={copilotFlash} style="margin-bottom: 0.75rem;">
            <h4>{t("lab.mangaOpenedInFirefox", { title: mangaNavDetected.title || "Manga" })}</h4>
            <div class="confirmation-actions">
              <button class="primary" onclick={onOpenedMangaInFirefox}>
                <Check size={16} /> {t("lab.alreadyOpenedManga")}
              </button>
            </div>
          </div>
        {/if}

        <div class="step-fields">
          <button
            class="primary single-cta"
            disabled={!!busy}
            onclick={onSearched}
          >
            <Search size={16} /> {t("lab.detectResults")}
          </button>

          {#if firefoxRunning}
            <button
              type="button"
              class="subtle-btn picker-action-btn"
              disabled={!!busy}
              onclick={onOpenedMangaInFirefox}
            >
              <BookOpen size={15} /> {t("lab.alreadyOpenedManga")}
            </button>
            <button
              type="button"
              class="subtle-btn picker-action-btn"
              disabled={!!busy}
              onclick={startFirefoxPicker}
            >
              <MousePointerClick size={15} /> {t("lab.pickResultInFirefox")}
            </button>
          {/if}

          <button
            type="button"
            class="text-link-btn"
            onclick={() => {
              skipSearch = true;
              step("detail");
            }}
          >
            {t("lab.skipSearchExplanation")}
          </button>

          {#if source.search_path}
            <div class="status-badge success">
              <span>{t("lab.searchPatternBadge")}</span><code>{source.search_path}</code>
            </div>
          {/if}

          {#if source.result_selector}
            <div class="status-badge success">
              <span>{t("lab.resultSelectorBadge")}</span><code>{source.result_selector}</code>
            </div>
          {/if}

          {#if extraction?.items.length}
            <div class="confirmation-box copilot-box" class:vesta-flash-yellow={copilotFlash}>
              <h4>{t("lab.isValidResult")}</h4>
              <p class="first-title-preview">🎯 <strong>{extraction.items[0]?.title}</strong></p>
              <div class="confirmation-actions">
                <button
                  class="primary"
                  onclick={() => confirmSearchResultAndGo()}
                >
                  <Check size={16} /> {t("lab.openMangaAndGo")}
                </button>
                <button type="button" onclick={startFirefoxPicker}>
                  <RotateCcw size={16} /> {t("lab.pickDifferentResult")}
                </button>
              </div>
            </div>
          {/if}
        </div>
      </div>

    {:else if kind === "detail"}
      <div class="wizard-card step-card" class:vesta-flash-yellow={stepFlash}>
        <div class="step-badge">{t("lab.step3Badge")}</div>
        <h2>{t("lab.step3CopilotTitle")}</h2>

        <div class="animated-hint">
          <BookOpen size={16} class="pulse-icon" />
          <span>{t("lab.step3Instruction")}</span>
        </div>

        <div class="step-fields">
          <button
            class="primary single-cta"
            disabled={!!busy}
            onclick={onHereSeries}
          >
            <BookOpen size={16} /> {t("lab.detectChaptersAuto")}
          </button>

          {#if firefoxRunning}
            <button
              type="button"
              class="subtle-btn picker-action-btn"
              disabled={!!busy}
              onclick={startFirefoxPicker}
            >
              <MousePointerClick size={15} /> {t("lab.pickChapterInFirefox")}
            </button>
          {/if}

          <details class="expand-list-accordion">
            <summary>{t("lab.expandChaptersHint")}</summary>
            <p>{t("lab.expandChaptersDesc")}</p>
          </details>

          {#if source.chapter_selector}
            <div class="status-badge success">
              <span>{t("lab.chapterSelectorBadge")}</span><code>{source.chapter_selector}</code>
            </div>
          {/if}

          {#if extraction?.items.length}
            <div class="confirmation-box copilot-box" class:vesta-flash-yellow={copilotFlash}>
              <h4>{t("lab.areTheseChapters", { count: extraction.items.length })}</h4>
              <p class="first-title-preview">
                📖 <strong>{extraction.items[0]?.title}</strong>
                {#if extraction.items[1]}
                  <span class="sub-chapter">, {extraction.items[1]?.title}</span>
                {/if}
                {#if extraction.items.length > 2}
                  <span class="sub-chapter">…</span>
                {/if}
              </p>
              <div class="confirmation-actions">
                <button
                  class="primary"
                  disabled={chaptersConfirmed}
                  onclick={() => confirmChaptersAndGo()}
                >
                  <Check size={16} /> {t("lab.openChapterAndGo")}
                </button>
                <button type="button" onclick={retryChapters}>
                  <RotateCcw size={16} /> {t("lab.pickDifferentChapter")}
                </button>
              </div>
            </div>
          {/if}
        </div>
      </div>

    {:else if kind === "reader"}
      <div class="wizard-card step-card" class:vesta-flash-yellow={stepFlash}>
        <div class="step-badge">{t("lab.step4Badge")}</div>
        <h2>{t("lab.step4CopilotTitle")}</h2>

        <div class="animated-hint">
          <Images size={16} class="pulse-icon" />
          <span>{t("lab.step4Instruction")}</span>
        </div>

        <div class="step-fields">
          <button
            class="primary single-cta"
            disabled={!!busy}
            onclick={onHereReader}
          >
            <Images size={16} /> {t("lab.detectScansAuto")}
          </button>

          {#if firefoxRunning}
            <button
              type="button"
              class="subtle-btn picker-action-btn"
              disabled={!!busy}
              onclick={startFirefoxPicker}
            >
              <MousePointerClick size={15} /> {t("lab.pickScanInFirefox")}
            </button>
          {/if}

          {#if source.image_selector}
            <div class="status-badge success">
              <span>{t("lab.imageSelectorBadge")}</span><code>{source.image_selector}</code>
            </div>
          {/if}

          {#if extraction?.items.length}
            <div class="confirmation-box copilot-box" class:vesta-flash-yellow={copilotFlash}>
              <h4>{t("lab.isThisScan", { count: extraction.items.length })}</h4>
              {#if extraction.items[0]}
                <div class="first-scan-preview-thumb">
                  <img
                    src={sample?.resources[extraction.items[0].url] || extraction.items[0].url}
                    alt="Scan 1"
                    referrerpolicy="no-referrer"
                    onerror={(e) => {
                      (e.currentTarget as HTMLImageElement).style.display = "none";
                    }}
                  />
                </div>
              {/if}

              {#if extraction.items.length === 1}
                <div class="single-page-alert">
                  <p>{t("lab.singlePageDetected")}</p>
                  <button type="button" class="subtle-btn" onclick={startFirefoxPicker}>
                    {t("lab.clickNextInFirefox")}
                  </button>
                </div>
              {/if}

              <div class="confirmation-actions">
                <button
                  class="primary"
                  disabled={imagesConfirmed}
                  onclick={confirmImagesAndGo}
                >
                  <Check size={16} /> {t("lab.confirmScansAndVerify")}
                </button>
                <button type="button" onclick={retryImages}>
                  <RotateCcw size={16} /> {t("lab.pickDifferentScan")}
                </button>
              </div>
            </div>
          {/if}
        </div>
      </div>

    {:else if kind === "review"}
      <div class="wizard-card step-card" class:vesta-flash-yellow={stepFlash}>
        <div class="step-badge">{t("lab.step5Badge")}</div>
        <h2>{t("lab.step5Title")}</h2>
        <p class="step-instruction">{t("lab.step5Desc")}</p>

        <div class="verify-checklist">
          <div class="verify-item">
            <strong>{t("lab.url")}</strong>
            <span>{source.name} ({source.base_url})</span>
            <Check size={16} class="check-green" />
          </div>

          <div class="verify-item">
            <strong>{t("lab.language")}</strong>
            <span>{getLanguageLabel(getLanguage(source.language) || { code: source.language, name: source.language, nameEn: source.language, flag: flag(source.language) })}</span>
            <Check size={16} class="check-green" />
          </div>

          <div class="verify-item">
            <strong>{t("nav.explore")}</strong>
            <span>{skipSearch ? t("lab.step2Skip") : source.search_path || "✓"}</span>
            <Check size={16} class="check-green" />
          </div>

          <div class="verify-item">
            <strong>{t("lab.summaryChapters")}</strong>
            <span>{source.chapter_selector || "✓"}</span>
            <Check size={16} class="check-green" />
          </div>

          <div class="verify-item">
            <strong>{t("lab.summaryPages")}</strong>
            <span>{source.image_selector || "✓"}</span>
            <Check size={16} class="check-green" />
          </div>
        </div>

        <div class="enable-catalog-box">
          <label class="checkbox enable-checkbox" title={t("sources.enabled")}>
            <input type="checkbox" bind:checked={source.enabled} />
            <strong>{t("lab.enableInCatalog")}</strong>
            <span class="info-pill" title={t("lab.step5Desc")}>
              <Info size={14} />
            </span>
          </label>
        </div>

        <button
          class="primary save-full-btn"
          disabled={!allPassed && source.enabled}
          onclick={saveSource}
        >
          <FileCheck size={16} /> {t("lab.saveConnector")}
        </button>
      </div>
    {/if}

    <!-- Collapsed Advanced Manual Section -->
    <details class="lab-advanced">
      <summary>
        <SlidersHorizontal size={14} /> {t("lab.advanced")}
      </summary>
      <div class="advanced-content">
        <SourceForm
          bind:source
          stage={kind}
          onPick={(field) => {
            selectorField = field;
            status = t("lab.clickElementInPreview");
          }}
        />
        {#if html}
          <div class="advanced-html-wrap">
            <label>
              <span>{t("lab.viewDom")}</span>
              <textarea
                class="lab-code"
                bind:value={html}
                disabled={!!busy}
                spellcheck="false"
              ></textarea>
            </label>
          </div>
        {/if}
      </div>
    </details>
  </section>

  <!-- Right Column: Interactive Sanitized Preview & Live Element Picker -->
  <section class="lab-panel" aria-label="Preview">
    {#if kind === "review"}
      <!-- Step 5 Studio: Prova sul campo (Live Test & Download) & Generalizzazione -->
      <div class="review-stage-studio">
        <!-- Live Test & Download Hero Card -->
        <div class="hero-live-card">
          <div class="hero-live-header">
            <div class="hero-live-title">
              <Sparkles size={22} class="sparkle-icon" />
              <div>
                <h3>{t("lab.step5LiveTestTitle")}</h3>
                <p class="hero-sub">{t("lab.step5LiveTestPrompt")}</p>
              </div>
            </div>
          </div>

          <div class="live-test-form">
            {#if detectedChapters.length > 0}
              <div class="source-field">
                <span>{t("lab.selectChapterToTest")}</span>
                <Select
                  label={t("lab.selectChapterToTest")}
                  bind:value={selectedTestChapterUrl}
                  options={detectedChapters.map((chap) => ({ value: chap.url, label: chap.title }))}
                  disabled={!!busy}
                />
              </div>
            {:else}
              <label class="chapter-picker-label">
                <span>{t("lab.selectChapterToTest")}</span>
                <input bind:value={selectedTestChapterUrl} placeholder="https://..." disabled={!!busy} />
              </label>
            {/if}

            <button
              type="button"
              class="primary live-download-cta"
              disabled={!!busy}
              onclick={runLiveTestDownload}
            >
              {#if busy}
                <LoaderCircle size={18} class="spin" />
                <span>{progress || t("lab.waiting")}</span>
              {:else}
                <Download size={18} />
                <span>{t("lab.downloadTestChapterBtn")}</span>
              {/if}
            </button>
          </div>

          {#if downloadSuccessInfo}
            <div class="download-success-banner">
              <div class="success-banner-heading">
                <CheckCircle2 size={24} class="check-green" />
                <div>
                  <h4>{t("lab.downloadSuccessTitle", { count: downloadSuccessInfo.pages })}</h4>
                  <p>{t("lab.downloadSuccessDesc")}</p>
                </div>
              </div>
              <div class="success-actions">
                <button
                  type="button"
                  class="open-pdf-btn"
                  onclick={() => {
                    if (downloadSuccessInfo?.path) {
                      void api("lab_open_output", { path: downloadSuccessInfo.path }).catch((e) => (failure = msg(e)));
                    }
                  }}
                >
                  <FolderOpen size={16} /> {t("lab.openDownloadedChapter")}
                </button>
                <button type="button" class="primary save-enable-btn" onclick={saveAndEnable}>
                  <Check size={16} /> {t("lab.saveAndEnableBtn")}
                </button>
              </div>
            </div>
          {/if}
        </div>

        <!-- Generalization Verification Card -->
        <div class="generalization-card">
          <div class="generalization-header">
            <h4>🧪 {t("lab.generalizationTitle")}</h4>
            <p>{t("lab.generalizationDesc")}</p>
          </div>

          <div class="gen-search-form">
            <input
              type="text"
              bind:value={generalizationQuery}
              placeholder={t("lab.generalizationPlaceholder")}
              disabled={generalizationBusy}
              onkeydown={(e) => {
                if (e.key === "Enter") void testGeneralization();
              }}
            />
            <button
              type="button"
              class="primary"
              disabled={generalizationBusy || !generalizationQuery.trim()}
              onclick={testGeneralization}
            >
              {#if generalizationBusy}
                <LoaderCircle size={14} class="spin" />
              {:else}
                <Search size={14} /> {t("lab.testGeneralizationBtn")}
              {/if}
            </button>
          </div>

          {#if generalizationResults !== null}
            <div class="gen-results-box">
              {#if generalizationResults.length > 0}
                <div class="status-badge success">
                  <Check size={14} /> {t("lab.detectedResults", { count: generalizationResults.length })}
                </div>
                <div class="lab-result-list">
                  {#each generalizationResults.slice(0, 5) as item}
                    <div class="extraction-item">
                      <strong>{item.title}</strong>
                      <small class="item-url">{item.url}</small>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="status-badge">
                  <span>{t("catalog.noResults")}</span>
                </div>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Share / Export / Import Connector Card moved to Right Studio -->
        <div class="export-import-card">
          <h4>{t("lab.shareConnector")}</h4>
          <p class="export-hint">{t("lab.exportHint")}</p>
          <div class="export-import-buttons">
            <button type="button" onclick={exportConnector}>
              <Download size={15} /> {t("lab.exportConnector")}
            </button>
            <button type="button" onclick={importConnector}>
              <Upload size={15} /> {t("lab.importConnector")}
            </button>
          </div>
        </div>
      </div>
    {:else}
      <!-- Live Status Line (Single status line, no duplicate Firefox buttons) -->
      <div class="status-bar" role="status">
        <span class="status-text">{status}</span>
        {#if busy}
          <span class="status-busy"><LoaderCircle size={14} class="spin" /> {busy}…</span>
        {/if}
      </div>

      <!-- Prominent Detected Elements Preview Card (Shows extracted chapters, results or pages) -->
      {#if extraction && extraction.items.length > 0}
        <div class="extraction-card detected-primary-card" class:vesta-flash-yellow={copilotFlash}>
          <div class="extraction-card-header">
            <div>
              <h4>
                <Check size={16} class="check-green" />
                {#if kind === "detail"}
                  {t("lab.detectedChapters", { count: extraction.items.length })}
                {:else if kind === "reader"}
                  {t("lab.detectedPages", { count: extraction.items.length })}
                {:else}
                  {t("lab.detectedResults", { count: extraction.items.length })}
                {/if}
              </h4>
              {#if kind === "detail" && source.chapter_selector}
                <small class="selector-tag"><code>{source.chapter_selector}</code></small>
              {:else if kind === "search" && source.result_selector}
                <small class="selector-tag"><code>{source.result_selector}</code></small>
              {:else if kind === "reader" && source.image_selector}
                <small class="selector-tag"><code>{source.image_selector}</code></small>
              {/if}
            </div>
          </div>

          {#if kind === "reader"}
            <div class="lab-result-list scan-pages-list">
              {#each extraction.items.slice(0, 8) as item, i}
                <div class="extraction-item scan-list-row">
                  <div class="scan-row-meta">
                    <strong>{item.title || `#${i + 1}`}</strong>
                    <small class="item-url">{item.url}</small>
                  </div>
                </div>
              {/each}
              {#if extraction.items.length > 8}
                <div class="more-items-notice">
                  {t("lab.moreItems", { count: extraction.items.length - 8 })}
                </div>
              {/if}
            </div>
          {:else}
            <div class="lab-result-list">
              {#each extraction.items.slice(0, 10) as item}
                <div class="extraction-item">
                  <strong>{item.title}</strong>
                  {#if item.group}<small>{item.group}</small>{/if}
                  <small class="item-url">{item.url}</small>
                </div>
              {/each}
              {#if extraction.items.length > 10}
                <div class="more-items-notice">
                  {t("lab.moreItems", { count: extraction.items.length - 10 })}
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Firefox Companion Active Banner when nothing is extracted yet or during navigation -->
      {#if firefoxRunning && (!extraction || extraction.items.length === 0)}
        <div class="firefox-companion-card">
          <div class="firefox-companion-header">
            <span class="firefox-pill">
              <span class="live-indicator"></span>
              {t("lab.firefoxActive")}
            </span>
            <small class="firefox-url truncate" title={firefoxLiveUrl || url}>{firefoxLiveUrl || url}</small>
          </div>
          <p class="firefox-hint">{t("lab.firefoxCompanionHint")}</p>
        </div>
      {/if}

      <!-- Single Sanitized Interactive DOM Preview (with Inspector if sample present) -->
      {#if sample && html}
        <details class="preview-dom-accordion" open={!extraction || extraction.items.length === 0}>
          <summary>
            <Layers size={14} /> {t("lab.viewDom")}
          </summary>
          <div class="preview-stage-container">
            <SampleBrowser
              {html}
              {sample}
              onInspect={inspectElement}
              onReady={(value) => (inspector = value)}
            />
          </div>
          <SampleInspector
            {inspected}
            stage={kind}
            bind:selectorField
            generic={source.adapter === "generic"}
            {busy}
            onApply={(exact) => {
              if (!inspected || source.adapter !== "generic") return;
              const sel = exact ? inspected.exact : inspected.selector;
              source = { ...source, [selectorField]: sel };
              inspector?.highlight(sel);
            }}
            onParent={() => inspector?.parent()}
          />
        </details>
      {:else if !firefoxRunning}
        <div class="preview-stage-container">
          <div class="preview-placeholder">
            <Globe size={40} class="placeholder-icon" />
            <p>{t("lab.previewPlaceholder")}</p>
            <p class="placeholder-sub">{t("lab.previewSub")}</p>
          </div>
        </div>
      {/if}
    {/if}

    <!-- Notifications & feedback -->
    {#if failure}<p class="alert error" role="alert">{failure}</p>{/if}
    {#if downloaded}
      <button
        onclick={() =>
          void api("lab_open_output", { path: downloaded }).catch(
            (e) => (failure = msg(e)),
          )}
      >
        {t("lab.openTestPdf")}
      </button>
    {/if}
  </section>
</div>

<!-- Secondary Navigation (Bottom) -->
<div class="wizard-navigation">
  <button
    type="button"
    disabled={kind === "setup" || !!busy}
    onclick={() => {
      const idx = stepOrder.indexOf(kind);
      if (idx > 0) step(stepOrder[idx - 1]);
    }}
  >
    <ChevronLeft size={15} /> {t("lab.prevStep")}
  </button>

  {#if kind !== "review"}
    <button
      type="button"
      class="primary"
      disabled={(kind === "setup" && !setupReady) || (kind === "search" && !searchReady) || (kind === "detail" && !chaptersConfirmed) || (kind === "reader" && !imagesConfirmed) || !!busy}
      onclick={() => {
        const idx = stepOrder.indexOf(kind);
        if (idx < stepOrder.length - 1) step(stepOrder[idx + 1]);
      }}
    >
      {t("lab.nextStep")} <ChevronRight size={15} />
    </button>
  {/if}
</div>
</div>

<style>
  .lab-topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    margin-bottom: 14px;
    flex-wrap: wrap;
  }

  .lab-topbar-tabs {
    display: flex;
    align-items: center;
  }

  .lab-topbar-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .url-placeholder-input::placeholder {
    font-style: italic;
    opacity: 0.55;
  }

  .template-selector-wrap {
    min-width: 210px;
  }

  .step-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .step-badge {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--accent, #818cf8);
  }

  .step-instruction {
    font-size: 13px;
    color: var(--muted, #94a3b8);
    margin: 0;
  }

  .animated-hint {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    background: rgba(129, 140, 248, 0.08);
    border: 1px solid rgba(129, 140, 248, 0.25);
    border-radius: 8px;
    font-size: 13px;
    color: var(--text, #f9fafb);
  }

  :global(.pulse-icon) {
    color: var(--accent, #818cf8);
    animation: pulse 1.8s infinite ease-in-out;
    flex-shrink: 0;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.5; transform: scale(1.15); }
  }

  .firefox-shield-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: rgba(34, 197, 94, 0.08);
    border: 1px solid rgba(34, 197, 94, 0.25);
    border-radius: 6px;
    font-size: 12px;
    color: var(--text, #f9fafb);
  }

  :global(.shield-green) {
    color: #4ade80;
    flex-shrink: 0;
  }

  .firefox-warning-banner {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    background: rgba(245, 158, 11, 0.09);
    border: 1px solid rgba(245, 158, 11, 0.3);
    border-radius: 6px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text, #f9fafb);
  }

  :global(.shield-amber) {
    color: #fbbf24;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .firefox-warning-content {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .firefox-download-link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: #fbbf24;
    font-weight: 600;
    text-decoration: underline;
    font-size: 12px;
  }

  .firefox-download-link:hover {
    color: #fde68a;
  }

  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border, #374151);
    border-radius: 8px;
    margin-bottom: 12px;
    font-size: 13px;
    flex-wrap: wrap;
    gap: 8px;
  }

  .status-text {
    color: var(--accent, #818cf8);
    font-weight: 500;
  }

  .preview-stage-container {
    width: 100%;
    min-height: 480px;
    background: var(--bg, #111827);
    border: 1px solid var(--border, #374151);
    border-radius: 8px;
    overflow: hidden;
  }

  .preview-placeholder {
    height: 480px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--muted, #94a3b8);
    text-align: center;
    padding: 24px;
    gap: 10px;
  }

  :global(.placeholder-icon) {
    color: var(--border, #374151);
  }

  .placeholder-sub {
    font-size: 12px;
    max-width: 360px;
  }

  .step-fields {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 4px;
  }

  .start-btn,
  .single-cta {
    width: 100%;
    padding: 12px;
    font-weight: 600;
    margin-top: 6px;
  }

  .status-badge {
    padding: 8px 10px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border, #374151);
    border-radius: 6px;
    font-size: 12px;
  }

  .status-badge.success {
    background: rgba(34, 197, 94, 0.1);
    border-color: rgba(34, 197, 94, 0.3);
    color: #4ade80;
  }

  .confirmation-box {
    padding: 12px;
    background: rgba(129, 140, 248, 0.08);
    border: 1px solid rgba(129, 140, 248, 0.25);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 4px;
  }

  .confirmation-box h4 {
    margin: 0;
    font-size: 13px;
  }

  .confirmation-actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
  }

  .confirmation-actions button {
    width: 100%;
    white-space: normal;
    height: auto;
    padding: 10px 12px;
    text-align: center;
    justify-content: center;
    line-height: 1.35;
  }

  .enable-catalog-box {
    margin: 6px 0;
  }

  .info-pill {
    display: inline-flex;
    align-items: center;
    color: var(--muted, #94a3b8);
    margin-left: 6px;
    cursor: help;
  }

  .scan-list-row {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
  }

  .scan-row-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .verify-checklist {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 8px 0;
  }

  .verify-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border, #374151);
    border-radius: 6px;
    font-size: 13px;
  }

  .verify-item strong {
    width: 90px;
  }

  .verify-item span {
    flex: 1;
    color: var(--muted, #94a3b8);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.check-green) {
    color: #4ade80;
  }

  .export-import-card {
    padding: 14px;
    border: 1px dashed var(--border, #374151);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.02);
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 8px 0;
  }

  .export-import-card h4 {
    margin: 0;
    font-size: 13px;
  }

  .export-hint {
    font-size: 11px;
    color: var(--muted, #94a3b8);
    line-height: 1.4;
    margin: 0;
  }

  .export-import-buttons {
    display: flex;
    gap: 8px;
  }

  .export-import-buttons button {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    padding: 7px 12px;
  }

  .save-full-btn {
    width: 100%;
    margin-top: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px;
    font-size: 14px;
  }

  .lab-advanced {
    margin-top: 16px;
    border: 1px solid var(--border, #374151);
    border-radius: 8px;
    background: var(--panel, #1f2937);
  }

  .lab-advanced summary {
    padding: 10px 14px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted, #94a3b8);
  }

  .advanced-content {
    padding: 12px 14px;
    border-top: 1px solid var(--border, #374151);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .advanced-html-wrap textarea {
    width: 100%;
    min-height: 200px;
    font-family: monospace;
    font-size: 11px;
  }

  .extraction-card {
    margin-top: 12px;
    padding: 12px;
    border: 1px solid var(--border, #374151);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.02);
  }

  .extraction-card h4 {
    margin: 0 0 8px;
    font-size: 13px;
  }

  .extraction-item {
    padding: 6px 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .item-url {
    color: var(--muted, #94a3b8);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .lab-workspace {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 100%;
  }

  .firefox-companion-card {
    background: rgba(129, 140, 248, 0.08);
    border: 1px solid rgba(129, 140, 248, 0.25);
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 16px;
  }

  .firefox-companion-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .firefox-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--accent, #818cf8);
  }

  .live-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #4ade80;
    box-shadow: 0 0 6px #4ade80;
    animation: pulse-dot 2s infinite ease-in-out;
  }

  @keyframes pulse-dot {
    0%, 100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.5;
      transform: scale(0.85);
    }
  }

  .firefox-url {
    color: var(--muted, #94a3b8);
    font-family: monospace;
    font-size: 11px;
    max-width: 320px;
  }

  .firefox-hint {
    font-size: 13px;
    color: var(--fg, #e2e8f0);
    margin: 0;
    line-height: 1.5;
  }

  .detected-primary-card {
    background: rgba(74, 222, 128, 0.05);
    border: 1px solid rgba(74, 222, 128, 0.3);
    border-radius: 8px;
    padding: 14px;
    margin-bottom: 16px;
  }

  .extraction-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }

  .extraction-card-header h4 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 4px;
  }

  .selector-tag code {
    font-size: 11px;
    color: #4ade80;
    background: rgba(74, 222, 128, 0.1);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .subtle-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    padding: 6px 10px;
  }

  .more-items-notice {
    padding: 8px;
    text-align: center;
    font-size: 12px;
    color: var(--muted, #94a3b8);
    font-style: italic;
  }

  .preview-dom-accordion {
    margin-top: 14px;
    border: 1px solid var(--border, #374151);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.01);
  }

  .preview-dom-accordion summary {
    padding: 10px 14px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted, #94a3b8);
  }

  .hint-lines {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .hint-lines small {
    color: var(--muted, #94a3b8);
    font-size: 11px;
  }

  .picker-action-btn {
    width: 100%;
    justify-content: center;
    padding: 10px;
    font-weight: 500;
  }

  .text-link-btn {
    background: transparent;
    border: none;
    color: var(--muted, #94a3b8);
    font-size: 11px;
    text-decoration: underline;
    cursor: pointer;
    padding: 4px;
    text-align: left;
  }

  .text-link-btn:hover {
    color: var(--accent, #818cf8);
  }

  .copilot-box {
    background: rgba(129, 140, 248, 0.08);
    border: 1px solid rgba(129, 140, 248, 0.25);
    border-radius: 8px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 6px;
  }

  .first-title-preview {
    font-size: 13px;
    color: var(--accent, #818cf8);
    margin: 0;
    word-break: break-word;
  }

  .sub-chapter {
    color: var(--muted, #94a3b8);
    font-weight: normal;
  }

  .expand-list-accordion {
    font-size: 12px;
    color: var(--muted, #94a3b8);
    padding: 6px 0;
  }

  .expand-list-accordion summary {
    cursor: pointer;
    font-weight: 500;
  }

  .expand-list-accordion p {
    margin: 6px 0 0;
    font-size: 11px;
    line-height: 1.4;
  }

  .first-scan-preview-thumb {
    width: 100%;
    max-height: 140px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid var(--border, #374151);
    border-radius: 6px;
    overflow: hidden;
    margin: 4px 0;
  }

  .first-scan-preview-thumb img {
    max-height: 140px;
    max-width: 100%;
    object-fit: contain;
  }

  .single-page-alert {
    background: rgba(234, 179, 8, 0.1);
    border: 1px solid rgba(234, 179, 8, 0.3);
    border-radius: 6px;
    padding: 8px 10px;
    font-size: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .single-page-alert p {
    margin: 0;
  }

  /* Step 5 Studio Styles */
  .review-stage-studio {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 480px;
  }

  .hero-live-card {
    background: rgba(129, 140, 248, 0.05);
    border: 1px solid rgba(129, 140, 248, 0.25);
    border-radius: 10px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .hero-live-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }

  .hero-live-title {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .hero-live-title h3 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }

  .hero-sub {
    margin: 2px 0 0;
    font-size: 13px;
    color: var(--muted, #94a3b8);
  }

  :global(.sparkle-icon) {
    color: var(--accent, #818cf8);
    flex-shrink: 0;
  }

  .live-test-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .chapter-picker-label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
  }

  .chapter-picker-label input {
    width: 100%;
    padding: 10px 12px;
    background: var(--bg, #111827);
    border: 1px solid var(--border, #374151);
    border-radius: 6px;
    color: var(--text, #f9fafb);
    font-size: 13px;
  }

  .live-download-cta {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 14px;
    font-size: 15px;
    font-weight: 600;
    border-radius: 8px;
    margin-top: 4px;
  }

  .download-success-banner {
    background: rgba(74, 222, 128, 0.08);
    border: 1px solid rgba(74, 222, 128, 0.35);
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .success-banner-heading {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .success-banner-heading h4 {
    margin: 0;
    font-size: 14px;
    color: #4ade80;
  }

  .success-banner-heading p {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--muted, #94a3b8);
  }

  .success-actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    margin-top: 6px;
  }

  .open-pdf-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 10px 16px;
    font-size: 13px;
  }

  .save-enable-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 10px 16px;
    font-size: 13px;
    background: #16a34a;
    border-color: #16a34a;
    color: #fff;
  }

  .generalization-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border, #374151);
    border-radius: 10px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .generalization-header h4 {
    margin: 0;
    font-size: 14px;
  }

  .generalization-header p {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--muted, #94a3b8);
  }

  .gen-search-form {
    display: flex;
    gap: 8px;
  }

  .gen-search-form input {
    flex: 1;
    padding: 9px 12px;
    background: var(--bg, #111827);
    border: 1px solid var(--border, #374151);
    border-radius: 6px;
    color: var(--text, #f9fafb);
    font-size: 13px;
  }

  .gen-results-box {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 4px;
  }
</style>
