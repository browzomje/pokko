<script lang="ts">
  import { api, message as msg } from "./lib/api";
  import { onMount, onDestroy } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
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

  const builtInIds = ["mw", "md", "mf", "mangaread", "readcomics"];

  let url = $state("");
  let testQuery = $state("");
  let assets = $state(true);
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
      ["mw", "mf", "readcomics", "mangaread"].includes(s.id),
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
      min_delay_ms: template.min_delay_ms || 1000,
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
      status = "Avvio di Firefox con uBlock Origin attivo…";
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

  function startFirefoxPolling() {
    if (pollInterval) clearInterval(pollInterval);
    pollInterval = setInterval(async () => {
      if (kind === "review" || kind === "setup") return;
      try {
        // Poll current URL from Firefox
        const current = await api<string>("lab_firefox_get_url");
        if (current && current.startsWith("http") && current !== url) {
          url = current;
          if (kind === "search" && (current.includes("/search") || current.includes("?q=") || current.includes("?s="))) {
            void capture();
          } else if (kind === "detail") {
            void onHereSeries();
          } else if (kind === "reader") {
            void onHereReader();
          }
        }

        // Try injecting picker
        void api("lab_firefox_inject_picker").catch(() => {});

        // Check if an element was picked in Firefox
        const picked = await api<string | null>("lab_firefox_get_picked");
        if (picked) {
          applyPickedSelector(picked);
        }
      } catch {}
    }, 1500);
  }

  function applyPickedSelector(selector: string) {
    if (kind === "search") {
      source = { ...source, result_selector: selector };
      markStageCompleted("search");
      status = t("lab.detectedResults", { count: 1 });
      setTimeout(() => step("detail"), 800);
    } else if (kind === "detail") {
      source = { ...source, chapter_selector: selector };
      void extract();
    } else if (kind === "reader") {
      source = { ...source, image_selector: selector };
      void extract();
    }
  }

  async function task(label: string, work: () => Promise<void>) {
    if (busy) return;
    busy = label;
    failure = "";
    progress = "";
    downloaded = "";
    token = `lab-${crypto.randomUUID()}`;
    try {
      await work();
    } catch (e) {
      failure = msg(e);
    } finally {
      busy = "";
      progress = "";
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
      "";
  }

  function markStageCompleted(stage: string) {
    if (!completedStages.includes(stage)) {
      completedStages = [...completedStages, stage];
    }
  }

  // Step 1: Start recording
  async function startRecording() {
    source.base_url = normalizeUrl(source.base_url);
    if (!source.name.trim() || source.name.trim() === "Nuova fonte") {
      source.name = deriveNameFromUrl(source.base_url) || "Fonte";
    }
    markStageCompleted("setup");
    if (firefoxInstalled) {
      void launchFirefox(source.base_url);
    }
    step(skipSearch ? "detail" : "search");
  }

  // Step 2: "Ho cercato"
  async function onSearched() {
    await task("Acquisizione", async () => {
      try {
        const ffUrl = await api<string>("lab_firefox_get_url");
        if (ffUrl && ffUrl.startsWith("http")) url = ffUrl;
      } catch {}

      if (!url.trim()) throw new Error("Incolla o apri l'URL della ricerca sul sito.");

      if (testQuery.trim()) {
        const path = inferSearchPath(url, testQuery);
        if (path) source = { ...source, search_path: path };
      }

      await capture();
      selectorField = "result_selector";
      status = "Pagina acquisita. Clicca sul titolo di un manga per registrarlo.";
    });
  }

  // Step 3: "Sono qui" on Series page with Heuristic Auto-Detection!
  async function onHereSeries() {
    await task("Acquisizione serie", async () => {
      try {
        const ffUrl = await api<string>("lab_firefox_get_url");
        if (ffUrl && ffUrl.startsWith("http")) url = ffUrl;
      } catch {}

      if (!url.trim()) throw new Error("Incolla o apri l'URL della scheda serie sul sito.");

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

      status = "Pagina acquisita. Clicca su uno dei capitoli nell'anteprima per registrarlo.";
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
    status = "Clicca su un altro capitolo per cambiare selettore.";
  }

  // Step 4: "Sono qui" on Reader page
  async function onHereReader() {
    await task("Acquisizione lettore", async () => {
      try {
        const ffUrl = await api<string>("lab_firefox_get_url");
        if (ffUrl && ffUrl.startsWith("http")) url = ffUrl;
      } catch {}

      if (!url.trim()) throw new Error("Incolla o apri l'URL del lettore con le scan.");

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
      "img.scan",
      ".entry-content img",
      ".page-break img",
    ];
    for (const sel of candidates) {
      const matches = doc.querySelectorAll(sel);
      if (matches.length >= 2) return sel;
    }
    const lazyImages = doc.querySelectorAll("img[data-src], img[data-original]");
    if (lazyImages.length >= 2) {
      const first = lazyImages[0];
      if (first.className) {
        const cls = first.className.trim().split(/\s+/)[0];
        if (cls) return `img.${cls}`;
      }
      return "img[data-src]";
    }
    return "img";
  }

  function confirmImages() {
    imagesConfirmed = true;
    markStageCompleted("reader");
    status = t("lab.detectedPages", { count: extraction?.items.length || 0 });
    setTimeout(() => step("review"), 600);
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
        status = "Clicca un'immagine della scan, non un contenitore.";
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
      status = "Campo aggiornato ✓";
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
      status = "Connettore esportato con successo ✓";
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
          throw new Error("File connettore non valido: mancano nome o indirizzo del sito.");
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
        throw new Error(t("lab.nameRequired") || "Inserisci un nome per la fonte prima di salvare.");
      }
      await onSave($state.snapshot(source));
      status = "Fonte salvata con successo.";
    });
  }

  onMount(() => {
    let disposed = false;
    void refresh().catch((e) => (failure = msg(e)));
    void checkFirefoxStatus();

    return () => {
      disposed = true;
      if (pollInterval) clearInterval(pollInterval);
      if (busy) void api("lab_cancel", { token }).catch(() => {});
    };
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });
</script>

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
      <div class="wizard-card step-card">
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
      <div class="wizard-card step-card">
        <div class="step-badge">{t("lab.step2Badge")}</div>
        <h2>{t("lab.step2Title")}</h2>

        <div class="animated-hint">
          <Search size={16} class="pulse-icon" />
          <span>{t("lab.step2Hint")}</span>
        </div>

        <label class="checkbox skip-search-toggle">
          <input
            type="checkbox"
            bind:checked={skipSearch}
            onchange={() => {
              if (skipSearch) step("detail");
            }}
          />
          <strong>{t("lab.step2Skip")}</strong>
        </label>

        {#if !skipSearch}
          <div class="step-fields">
            <label>
              <FieldLabel
                text="Titolo cercato sul sito (facoltativo)"
                icon={Search}
                help="Aiuta a identificare il termine di ricerca nell'URL."
              />
              <input
                bind:value={testQuery}
                placeholder="Es. Berserk o Invincible"
              />
            </label>

            <button
              class="primary single-cta"
              disabled={!!busy}
              onclick={onSearched}
            >
              {t("lab.iSearched")}
            </button>

            {#if source.search_path}
              <div class="status-badge success">
                Modello ricerca: <code>{source.search_path}</code>
              </div>
            {/if}

            {#if source.result_selector}
              <div class="status-badge success">
                Selettore titoli: <code>{source.result_selector}</code>
              </div>
            {/if}
          </div>
        {/if}
      </div>

    {:else if kind === "detail"}
      <div class="wizard-card step-card">
        <div class="step-badge">{t("lab.step3Badge")}</div>
        <h2>{t("lab.step3Title")}</h2>

        <div class="animated-hint">
          <BookOpen size={16} class="pulse-icon" />
          <span>{t("lab.step3Hint")}</span>
        </div>

        <div class="step-fields">
          <button
            class="primary single-cta"
            disabled={!!busy}
            onclick={onHereSeries}
          >
            {t("lab.imHereChapters")}
          </button>

          {#if source.chapter_selector}
            <div class="status-badge success">
              Selettore capitoli: <code>{source.chapter_selector}</code>
            </div>
          {/if}

          {#if extraction?.items.length}
            <div class="confirmation-box">
              <h4>
                Trovati {extraction.items.length} capitoli. Confermi la lista?
              </h4>
              <div class="confirmation-actions">
                <button
                  class="primary"
                  disabled={chaptersConfirmed}
                  onclick={confirmChapters}
                >
                  <Check size={16} /> {t("lab.confirmChapters")}
                </button>
                <button onclick={retryChapters}>
                  <RotateCcw size={16} /> {t("lab.retry")}
                </button>
              </div>
            </div>
          {/if}
        </div>
      </div>

    {:else if kind === "reader"}
      <div class="wizard-card step-card">
        <div class="step-badge">{t("lab.step4Badge")}</div>
        <h2>{t("lab.step4Title")}</h2>

        <div class="animated-hint">
          <Images size={16} class="pulse-icon" />
          <span>{t("lab.step4Hint")}</span>
        </div>

        <div class="step-fields">
          <button
            class="primary single-cta"
            disabled={!!busy}
            onclick={onHereReader}
          >
            {t("lab.imHereScans")}
          </button>

          {#if source.image_selector}
            <div class="status-badge success">
              Selettore immagini: <code>{source.image_selector}</code>
            </div>
          {/if}

          {#if extraction?.items.length}
            <div class="confirmation-box">
              <h4>
                Rilevate {extraction.items.length} immagini scan. Confermi la prima pagina?
              </h4>
              <div class="confirmation-actions">
                <button
                  class="primary"
                  disabled={imagesConfirmed}
                  onclick={confirmImages}
                >
                  <Check size={16} /> {t("lab.confirmImages")}
                </button>
                <button onclick={retryImages}>
                  <RotateCcw size={16} /> {t("lab.chooseAnother")}
                </button>
              </div>
            </div>
          {/if}
        </div>
      </div>

    {:else if kind === "review"}
      <div class="wizard-card step-card">
        <div class="step-badge">{t("lab.step5Badge")}</div>
        <h2>{t("lab.step5Title")}</h2>
        <p class="step-instruction">{t("lab.step5Desc")}</p>

        <div class="verify-checklist">
          <div class="verify-item">
            <strong>Sito</strong>
            <span>{source.name} ({source.base_url})</span>
            <Check size={16} class="check-green" />
          </div>

          <div class="verify-item">
            <strong>Lingua</strong>
            <span>{flag(source.language)} {getLanguageLabel(getLanguage(source.language) || { code: source.language, name: source.language, nameEn: source.language, flag: flag(source.language) })}</span>
            <Check size={16} class="check-green" />
          </div>

          <div class="verify-item">
            <strong>Ricerca</strong>
            <span>{skipSearch ? "Serie singola" : source.search_path || "Configurata"}</span>
            <Check size={16} class="check-green" />
          </div>

          <div class="verify-item">
            <strong>Capitoli</strong>
            <span>{source.chapter_selector || "Configurati"}</span>
            <Check size={16} class="check-green" />
          </div>

          <div class="verify-item">
            <strong>Pagine scan</strong>
            <span>{source.image_selector || "Configurate"}</span>
            <Check size={16} class="check-green" />
          </div>
        </div>

        <label class="checkbox enable-checkbox">
          <input type="checkbox" bind:checked={source.enabled} />
          <strong>{t("lab.enableInCatalog")}</strong>
        </label>

        <div class="export-import-card">
          <h4>Condividi il connettore</h4>
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
            status = "Clicca l'elemento nell'anteprima.";
          }}
        />
        {#if html}
          <div class="advanced-html-wrap">
            <label>
              <span>HTML pagina corrente (modificabile per prove)</span>
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
  <section class="lab-panel" aria-label="Anteprima e ispezione">
    {#if kind !== "review"}
      <!-- Live Status Line (Single status line, no duplicate Firefox buttons) -->
      <div class="status-bar" role="status">
        <span class="status-text">{status}</span>
        {#if busy}
          <span class="status-busy"><LoaderCircle size={14} class="spin" /> {busy}…</span>
        {/if}
      </div>

      <!-- Single Sanitized Interactive DOM Preview (NO double window/webview) -->
      <div class="preview-stage-container">
        {#if html && sample}
          <SampleBrowser
            {html}
            {sample}
            onInspect={inspectElement}
            onReady={(value) => (inspector = value)}
          />
        {:else}
          <div class="preview-placeholder">
            <Globe size={40} class="placeholder-icon" />
            <p>{t("lab.previewPlaceholder")}</p>
            <p class="placeholder-sub">{t("lab.previewSub")}</p>
          </div>
        {/if}
      </div>

      {#if sample}
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

        {#if extraction && validTest}
          <div class="extraction-card">
            <h4>Elementi rilevati: {extraction.items.length}</h4>

            {#if kind === "reader" && extraction.items.length}
              <div class="lab-image-preview">
                {#each extraction.items.slice(0, 8) as item}
                  {#if sample.resources[item.url]?.startsWith("data:image/")}
                    <img src={sample.resources[item.url]} alt={item.title} />
                  {/if}
                {/each}
              </div>
              <button
                class="primary test-pdf-btn"
                disabled={!!busy}
                onclick={download}
              >
                <Download size={15} /> {t("lab.testPdf")}
              </button>
            {:else}
              <div class="lab-result-list">
                {#each extraction.items.slice(0, 10) as item}
                  <div class="extraction-item">
                    <strong>{item.title}</strong>
                    {#if item.group}<small>{item.group}</small>{/if}
                    <small class="item-url">{item.url}</small>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
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
        Apri PDF di prova
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
    gap: 8px;
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

  .test-pdf-btn {
    margin-top: 10px;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
</style>
