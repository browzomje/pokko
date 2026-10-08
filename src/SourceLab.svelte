<script lang="ts">
  import { api, message as msg } from "./lib/api";
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import {
    ChevronLeft,
    ChevronRight,
    Download,
    LoaderCircle,
    X,
  } from "lucide-svelte";
  import SourceForm from "./SourceForm.svelte";
  import Select from "./Select.svelte";
  import SampleInspector from "./components/SampleInspector.svelte";
  import SampleBrowser from "./components/SampleBrowser.svelte";
  import LabGuide from "./components/LabGuide.svelte";
  import FieldLabel from "./components/FieldLabel.svelte";
  import { Search, Globe, MousePointer, Images } from "lucide-svelte";
  import {
    fieldForStep,
    inferSearchPath,
    stageSignature,
  } from "./lib/source-config";
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
    onSelect,
    onNew,
  }: {
    source: Source;
    onSave: (source: Source) => Promise<void>;
    onBack: () => void;
    sources: Source[];
    onSelect: (source: Source) => void;
    onNew: () => void;
  } = $props();
  let url = $state("");
  let testQuery = $state("");
  let assets = $state(true);
  let offline = $state(true);
  let samples = $state<SampleInfo[]>([]);
  let sample = $state<Sample | null>(null);
  let sampleId = $state("");
  let html = $state("");
  let kind = $state("setup");
  let pane = $state("preview");
  let busy = $state("");
  let progress = $state("");
  let failure = $state("");
  let status = $state("");
  let token = $state("");
  let inspector: Inspector | null = null;
  let recording = $state(true);
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
  let apiVerified = $state("");
  let learnedPath = $derived(inferSearchPath(url, testQuery));
  const stepOrder = ["setup", "search", "detail", "reader", "review"];
  let setupReady = $derived.by(() => {
    try {
      return (
        !!source.name.trim() && new URL(source.base_url).protocol === "https:"
      );
    } catch {
      return false;
    }
  });
  let allPassed = $derived(
    ["search", "detail", "reader"].every((k) => {
      const s = stageSamples[k];
      return (
        !!s &&
        passed[k]?.count > 0 &&
        passed[k].signature === stageSignature(source, k, s.html, s.url)
      );
    }),
  );
  function prepareObservedSearch() {
    if (!learnedPath) {
      failure =
        "L’URL non contiene il titolo di prova. Esegui davvero quella ricerca sul sito, poi copia l’URL completo dei risultati. Se il sito usa un modulo POST o JavaScript, serve un adattatore dedicato.";
      return false;
    }
    const origin = new URL(url).origin;
    if (source.base_url && new URL(source.base_url).origin !== origin) {
      failure =
        "La pagina appartiene a un altro sito. Torna al passo Sito e correggi l’indirizzo, oppure usa una pagina della fonte selezionata.";
      return false;
    }
    source = { ...source, base_url: origin, search_path: learnedPath };
    return true;
  }

  async function task(label: string, work: () => Promise<void>) {
    if (busy) return;
    busy = label;
    failure = "";
    status = "";
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
    status =
      "Campione locale caricato. Le prove di estrazione e l’anteprima funzionano senza rete.";
  }
  function prepareSearchUrl() {
    try {
      const path = (
        source.adapter === "mangaworld"
          ? "/archive?keyword={query}&page={page}"
          : source.search_path
      )
        .replaceAll("{query}", encodeURIComponent(testQuery.trim()))
        .replaceAll("{page}", "1");
      url = new URL(path, source.base_url).href;
      kind = "search";
      selectorField = "result_selector";
    } catch (e) {
      failure = msg(e);
    }
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
      source.workflow?.find((step) => step.kind === value)?.url ||
      "";
  }
  function record(selector: string) {
    if (!sample) return;
    source = {
      ...source,
      [fieldForStep(kind)]: selector,
      workflow: [
        ...(source.workflow || []).filter((s) => s.kind !== kind),
        { kind, url: sample.url, selector },
      ],
    };
    if (kind === "search") {
      const path = inferSearchPath(sample.url, testQuery);
      if (path) source = { ...source, search_path: path };
    }
  }
  async function inspectElement(value: InspectedElement) {
    inspected = value;
    if (!recording || source.adapter !== "generic" || busy) return;
    if (selectorField !== fieldForStep(kind)) {
      source = { ...source, [selectorField]: value.selector };
      status = "Campo registrato. Ripeti Prova estrazione per verificarlo.";
      return;
    }
    if (kind === "reader") {
      if (value.tag !== "img") {
        status =
          "Clicca una scan, non un contenitore o un link al capitolo successivo.";
        return;
      }
      record(value.selector);
      status = "Selettore immagini registrato. Controlla con Prova estrazione.";
      return;
    }
    if (!value.href || !sample) {
      status =
        "Clicca un link alla serie o al capitolo. Usa Elemento padre per selezionarlo.";
      return;
    }
    const target = new URL(value.href, sample.url);
    if (!["https:", "http:"].includes(target.protocol)) {
      failure = "Il link deve essere una pagina HTTP/HTTPS.";
      return;
    }
    record(value.linkSelector || value.selector);
    await extract();
    if (!extraction?.items.length || !validTest) return;
    const nextKind = kind === "search" ? "detail" : "reader";
    step(nextKind);
    url = target.href;
    const local = samples.find((s) => s.url === url);
    if (local) {
      sampleId = local.id;
      await load();
    } else {
      sample = null;
      html = "";
      if (!offline) {
        await capture();
        return;
      }
      status =
        "Passaggio registrato. Premi Acquisisci per salvare la pagina successiva; oppure importa il suo campione offline.";
    }
  }
  async function capture() {
    await task("Acquisizione", async () => {
      if (
        kind === "search" &&
        source.adapter === "generic" &&
        !prepareObservedSearch()
      )
        return;
      const value = await api<Sample>("lab_capture", {
        url,
        source: $state.snapshot(source),
        assets,
        token,
      });
      setSample(value);
      stageSamples = { ...stageSamples, [kind]: value };
      if (kind === "search") {
        const path = inferSearchPath(value.url, testQuery);
        if (path) source = { ...source, search_path: path };
      }
      await refresh();
    });
  }
  async function load() {
    if (!sampleId) return;
    await task("Caricamento", async () =>
      setSample(await api<Sample>("lab_load", { id: sampleId })),
    );
  }
  async function importSample() {
    await task("Importazione", async () => {
      const path = await open({
        multiple: false,
        filters: [
          { name: "Campione JSON o HTML", extensions: ["json", "html", "htm"] },
        ],
      });
      if (typeof path !== "string") return;
      setSample(
        await api<Sample>("lab_import", {
          path,
          url,
          source: $state.snapshot(source),
        }),
      );
      await refresh();
    });
  }
  async function exportSample() {
    if (!sample) return;
    await task("Esportazione", async () => {
      const path = await save({
        defaultPath: `campione-${sample!.id}.json`,
        filters: [{ name: "Campione offline", extensions: ["json"] }],
      });
      if (!path) return;
      await api("lab_export", { id: sample!.id, path });
      status = "Campione esportato con HTML, configurazione e risorse salvate.";
    });
  }
  async function removeSample() {
    if (!sample) return;
    await task("Eliminazione", async () => {
      await api("lab_delete", { id: sample!.id });
      sample = null;
      sampleId = "";
      html = "";
      extraction = null;
      inspected = null;
      await refresh();
      status = "Campione eliminato.";
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
      status = `${extraction.items.length} elementi estratti${extraction.has_next ? " · pagina successiva presente" : ""}.`;
      const selector = source[selectorField as keyof Source];
      if (typeof selector === "string") highlight(selector);
    });
  }
  function highlight(selector: string) {
    inspector?.highlight(selector);
  }
  function applySelector(exact = false) {
    if (!inspected || source.adapter !== "generic") return;
    const selector = exact ? inspected.exact : inspected.selector;
    source = { ...source, [selectorField]: selector };
    highlight(selector);
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
      status = `PDF verificato: ${result.pages} pagine · ${(result.bytes / 1048576).toFixed(1)} MB\n${result.output}`;
    });
  }
  async function saveSource() {
    await task("Salvataggio", async () => {
      if (
        source.enabled &&
        source.adapter === "mangadex" &&
        apiVerified !== source.base_url
      )
        throw new Error("Verifica prima la connessione API dal passo Sito.");
      if (source.enabled && source.adapter === "generic" && !allPassed)
        throw new Error(
          "Verifica prima ricerca, serie e pagine, oppure salva una bozza disabilitata.",
        );
      await onSave($state.snapshot(source));
      status = "Fonte salvata. Puoi continuare le prove.";
    });
  }
  onMount(() => {
    let disposed = false;
    void refresh().catch((e) => (failure = msg(e)));
    const listener = isTauri()
      ? listen<{ token: string; message: string }>("lab-progress", (e) => {
          if (!disposed && e.payload.token === token)
            progress = e.payload.message;
        })
      : Promise.resolve(() => {});
    return () => {
      disposed = true;
      void listener.then((unlisten) => unlisten());
      if (busy) void api("lab_cancel", { token }).catch(() => {});
    };
  });
</script>

<div class="lab-heading">
  <div>
    <button class="back-link" disabled={!!busy} onclick={onBack}
      ><ChevronLeft size={16} /> Fonti del catalogo</button
    >
    <h1>Laboratorio · {source.name}</h1>
  </div>
  <button class="primary" disabled={!!busy} onclick={saveSource}
    >{source.enabled ? "Salva fonte" : "Salva bozza"}</button
  >
</div>
<div class="lab-controls">
  <Select
    label="Fonte in laboratorio"
    value={source.id}
    options={[
      { value: source.id, label: source.name },
      ...sources
        .filter((s) => s.id !== source.id)
        .map((s) => ({ value: s.id, label: s.name })),
    ]}
    onchange={(id) => {
      const selected = sources.find((s) => s.id === id);
      if (selected) onSelect(selected);
    }}
  /><button onclick={onNew} disabled={!!busy}>Nuova fonte</button>
</div>
<LabGuide {kind} onStep={step} />
<div class="lab-layout" class:lab-review={kind === "review"}>
  <section class="lab-config" aria-label="Configurazione del connettore">
    <fieldset disabled={!!busy} class="lab-fieldset">
      <SourceForm
        bind:source
        stage={kind}
        onPick={(field) => {
          selectorField = field;
          status =
            "Clicca l’elemento corrispondente nell’anteprima: il campo scelto verrà compilato.";
        }}
      />
    </fieldset>
    {#if source.adapter !== "generic"}<p class="lab-help">
        {source.adapter === "mangadex"
          ? "MangaDex usa un’API, quindi non richiede selettori HTML. Usa Verifica connessione API prima di abilitarla nel passo finale."
          : "Questo adattatore usa un parser dedicato. Per scrivere i tuoi selettori, scegli HTML generico."}
      </p>{/if}
  </section>
  <section class="lab-panel" aria-label="Prove e campioni">
    {#if kind === "setup"}
      <div class="wizard-card">
        <h2>Un connettore si costruisce da tre pagine reali</h2>
        <ol>
          <li>
            <strong>Ricerca:</strong> risultati di un titolo cercato sul sito.
          </li>
          <li>
            <strong>Serie:</strong> pagina con l’elenco di capitoli o albi.
          </li>
          <li><strong>Pagine:</strong> lettore con le scan di un capitolo.</li>
        </ol>
        <p>
          Non esiste un percorso di ricerca universale: nel prossimo passo lo
          ricaveremo dall’URL che copierai dal sito.
        </p>
        <button
          class="primary"
          disabled={!setupReady}
          onclick={() => step("search")}
          >Continua alla ricerca <ChevronRight size={16} /></button
        >
      </div>
      {#if source.adapter === "mangadex"}<button
          disabled={!!busy}
          onclick={() =>
            void task("Test connessione", async () => {
              status = await api<string>("test_source_connection", {
                source: $state.snapshot(source),
              });
              apiVerified = source.base_url;
            })}>Verifica connessione API</button
        >{/if}
    {:else if kind === "review"}
      <div class="wizard-card">
        <h2>Verifica del connettore</h2>
        {#each ["search", "detail", "reader"] as test}<div class="wizard-check">
            <strong
              >{test === "search"
                ? "Ricerca"
                : test === "detail"
                  ? "Serie e capitoli"
                  : "Immagini"}</strong
            ><span
              >{stageSamples[test] &&
              passed[test]?.count > 0 &&
              passed[test].signature ===
                stageSignature(
                  source,
                  test,
                  stageSamples[test].html,
                  stageSamples[test].url,
                )
                ? `${passed[test].count} elementi verificati`
                : "Da verificare"}</span
            ><button onclick={() => step(test)}>Apri passo</button>
          </div>{/each}<label class="checkbox"
          ><input
            type="checkbox"
            bind:checked={source.enabled}
            disabled={!source.enabled &&
              ((source.adapter === "generic" && !allPassed) ||
                (source.adapter === "mangadex" &&
                  apiVerified !== source.base_url))}
          /> Abilita nelle ricerche del catalogo</label
        >
        <p>
          Salva conserva il connettore. Le prove PDF restano nella cartella
          scelta. Le pagine campione vengono riusate offline.
        </p>
        <button class="primary" onclick={saveSource}
          >Salva {source.enabled ? "fonte" : "bozza disabilitata"}</button
        >
      </div>
    {:else}
      {#if kind === "search"}<div class="wizard-card">
          <strong>Apri il sito e cerca un titolo di prova</strong>
          <p>
            Copia l’indirizzo della pagina dei risultati. Il titolo qui sotto
            deve essere lo stesso digitato sul sito.
          </p>
          <label
            ><FieldLabel
              text="Titolo che hai cercato sul sito"
              icon={Search}
              help="Serve a riconoscere dove si trova il titolo nell’URL. Non stiamo indovinando l’indirizzo della ricerca."
            /><input
              bind:value={testQuery}
              placeholder="Es. 20th century boys"
            /></label
          >
          {#if learnedPath}<p class="query-model">
              Modello rilevato: <code>{learnedPath}</code>
            </p>{:else}<p class="lab-help">
              Incolla l’URL dei risultati per ricavare il modello. Nessun
              percorso viene inventato.
            </p>{/if}
          {#if source.search_path}<details>
              <summary>Prova il modello già configurato</summary>
              <p>
                Questo percorso è salvato nella fonte, non viene ricavato dal
                solo dominio: <code>{source.search_path}</code>
              </p>
              <button disabled={!testQuery.trim()} onclick={prepareSearchUrl}
                >Genera URL dal modello salvato</button
              >
            </details>{/if}
        </div>{/if}
      <label
        ><FieldLabel
          text={kind === "search"
            ? "URL dei risultati copiato dal sito"
            : kind === "detail"
              ? "URL della serie con l’elenco capitoli"
              : "URL del lettore con le scan"}
          icon={Globe}
          help="Questo è l’indirizzo esatto della pagina da leggere. Verrà compilato anche quando segui il link scelto nel passo precedente."
        />
        <div class="lab-url">
          <input
            type="url"
            bind:value={url}
            disabled={!!busy}
            placeholder={kind === "search"
              ? "Incolla l’URL dopo la ricerca sul sito"
              : kind === "detail"
                ? "Incolla il link della serie"
                : "Incolla il link del capitolo"}
          /><button
            disabled={!!busy ||
              !url.trim() ||
              source.adapter === "mangadex" ||
              (kind === "search" && !testQuery.trim())}
            onclick={capture}>Acquisisci pagina</button
          >
        </div></label
      >
      <div class="lab-controls">
        <label class="checkbox"
          ><input
            type="checkbox"
            bind:checked={assets}
            disabled={!!busy}
          /><FieldLabel
            text="Salva anche CSS e immagini"
            icon={Images}
            help="Salva l’aspetto della pagina e le scan insieme all’HTML, così anteprima e PDF di prova possono funzionare offline. L’acquisizione rispetta i limiti della fonte."
          /></label
        ><button disabled={!!busy} onclick={importSample}
          >Importa HTML / campione</button
        >
      </div>
      <p class="lab-help">
        L’acquisizione legge una singola pagina e, se richiesto, le sue risorse
        (fino a 300, massimo 64 MB). Salva separatamente ricerca, dettaglio e
        lettore. Gli script del sito non vengono eseguiti; le risorse mancanti
        non vengono caricate dall’anteprima.
      </p>
      {#if samples.length}<div class="lab-controls">
          <Select
            label="Campioni salvati"
            bind:value={sampleId}
            options={[
              { value: "", label: "Scegli un campione" },
              ...samples.map((s) => ({
                value: s.id,
                label: `${new Date(s.created * 1000).toLocaleString("it-IT")} · ${s.url} · ${s.resources} risorse`,
              })),
            ]}
          /><button disabled={!!busy || !sampleId} onclick={load}>Carica</button
          >
        </div>{/if}
      {#if sample}
        <p class="lab-help">
          Campione: {sample.url} · {Object.keys(sample.resources).length} risorse
          locali
        </p>
        <div class="lab-actions">
          <button
            disabled={!!busy}
            onclick={() => {
              if (sample) {
                source = {
                  ...structuredClone($state.snapshot(sample.source)),
                  id: source.id,
                };
                status =
                  "Configurazione del campione caricata nella bozza. Premi Salva fonte per conservarla.";
              }
            }}>Carica configurazione del campione</button
          >
          <button disabled={!!busy} onclick={exportSample}
            >Esporta campione offline</button
          ><button disabled={!!busy} onclick={removeSample}
            >Elimina campione</button
          >
        </div>
        <div class="lab-controls">
          <label class="checkbox"
            ><input
              type="checkbox"
              bind:checked={recording}
              disabled={source.adapter !== "generic" || !!busy}
            /><MousePointer size={15} /> Registra percorso</label
          ><label class="checkbox"
            ><input type="checkbox" bind:checked={offline} /><FieldLabel
              text="Naviga nei campioni offline"
              icon={Globe}
              help="Durante la registrazione apre pagine già salvate. Se manca un campione, premi Acquisisci. Disattiva per acquisire automaticamente le pagine dei link cliccati. La stessa opzione controlla il PDF di prova."
            /></label
          ><small
            >Ricerca → link serie → link capitolo → immagine. Ogni clic salva il
            selettore; Acquisisci apre il prossimo indirizzo. I campioni già
            salvati si aprono senza rete.</small
          >
        </div>
        {#if source.workflow?.length}<details open class="lab-help">
            <summary
              >Percorso registrato · {source.workflow.length}
              {source.workflow.length === 1 ? "passo" : "passi"}</summary
            >
            <ol>
              {#each source.workflow as action}<li>
                  <strong
                    >{action.kind === "search"
                      ? "Ricerca"
                      : action.kind === "detail"
                        ? "Serie"
                        : "Lettore"}</strong
                  >
                  · <code>{action.selector}</code><br />{action.url}
                </li>{/each}
            </ol>
            <button onclick={() => (source = { ...source, workflow: [] })}
              >Azzera registrazione</button
            >
          </details>{/if}
        {#if sample.warnings.length}<details class="lab-help lab-warning">
            <summary>{sample.warnings.length} avvisi sull’acquisizione</summary
            >{#each sample.warnings as warning}<p>{warning}</p>{/each}
          </details>{/if}
        {#if extraction?.next_url}<div class="lab-help">
            Il lettore continua su <code>{extraction.next_url}</code>.
            <button
              disabled={!!busy}
              onclick={() => {
                url = extraction?.next_url || "";
                void capture();
              }}>Salva pagina successiva</button
            > Il test PDF parte dal campione attuale: carica il primo campione per
            provare tutta la sequenza.
          </div>{/if}
        <div class="lab-tabs">
          {#each [{ value: "preview", label: "Anteprima / ispeziona" }, { value: "html", label: "HTML" }] as tab}<button
              class:chosen={pane === tab.value}
              onclick={() => (pane = tab.value)}>{tab.label}</button
            >{/each}
        </div>
        {#if pane === "preview"}<SampleBrowser
            {html}
            {sample}
            onInspect={inspectElement}
            onReady={(value) => (inspector = value)}
          />
        {:else}<label
            >HTML di lavoro (le modifiche valgono per questa prova)<textarea
              class="lab-code"
              bind:value={html}
              disabled={!!busy}
              spellcheck="false"></textarea></label
          >{/if}
        <SampleInspector
          {inspected}
          stage={kind}
          bind:selectorField
          generic={source.adapter === "generic"}
          {busy}
          onApply={applySelector}
          onParent={() => inspector?.parent()}
        />
        <div class="lab-controls">
          <button
            class="primary"
            disabled={!!busy || source.adapter === "mangadex"}
            onclick={extract}>Prova estrazione</button
          >
        </div>
        {#if extraction && validTest}
          <p class="lab-help">
            {extraction.items.length} elementi{extraction.has_next
              ? " · pagina successiva trovata"
              : ""}
          </p>
          {#if extraction.description}<p class="description">
              {extraction.description}
            </p>{/if}
          <div class="lab-result-list">
            {#each extraction.items as item}<div>
                <strong>{item.title}</strong>{#if item.group}<small
                    >{item.group}</small
                  >{/if}<small>{item.url}</small>{#if kind !== "reader"}<button
                    disabled={!!busy}
                    onclick={() => {
                      step(kind === "search" ? "detail" : "reader");
                      url = item.url;
                    }}>Usa URL per il prossimo campione</button
                  >{/if}
              </div>{/each}
          </div>
          {#if kind === "reader"}
            <div class="lab-image-preview">
              {#each extraction.items.slice(0, 6) as item}{#if sample.resources[item.url]?.startsWith("data:image/")}<img
                    src={sample.resources[item.url]}
                    alt={item.title}
                  />{/if}{/each}
            </div>
            <div class="lab-controls">
              <label class="checkbox"
                ><input
                  type="checkbox"
                  bind:checked={offline}
                  disabled={!!busy}
                /> Test download offline</label
              ><button
                class="primary"
                disabled={!!busy || !extraction.items.length}
                onclick={download}
                ><Download size={16} /> Scarica PDF di prova</button
              >
            </div>
            <p class="lab-help">
              Il test crea un PDF con tutte le immagini estratte, nella cartella
              scelta, e ne verifica il numero di pagine. Offline usa solo le
              immagini salvate; disattivando l’opzione scarica dalla rete quelle
              mancanti.
            </p>
          {/if}
        {:else if extraction}<p class="lab-help">
            Configurazione modificata: ripeti l’estrazione per aggiornare il
            risultato.
          </p>{/if}
      {/if}
    {/if}
    {#if busy}<div class="lab-controls" role="status">
        <LoaderCircle size={16} class="spin" />
        {busy}… {#if busy === "Acquisizione" || busy === "Download di prova"}<button
            onclick={() =>
              void api("lab_cancel", { token }).catch(
                (e) => (failure = msg(e)),
              )}><X size={16} /> Annulla</button
          >{/if}
      </div>{/if}
    {#if progress}<p class="lab-status" role="status">{progress}</p>{/if}
    {#if failure}<p class="alert error" role="alert">{failure}</p>{/if}
    {#if status}<p class="lab-status" role="status">{status}</p>{/if}
    {#if downloaded}<button
        onclick={() =>
          void api("lab_open_output", { path: downloaded }).catch(
            (e) => (failure = msg(e)),
          )}>Apri PDF di prova</button
      >{/if}
  </section>
</div>

<div class="wizard-navigation">
  <button
    disabled={kind === "setup" || !!busy}
    onclick={() => step(stepOrder[stepOrder.indexOf(kind) - 1])}
    ><ChevronLeft size={15} /> Passo precedente</button
  ><button
    disabled={kind === "review" || !!busy || (kind === "setup" && !setupReady)}
    onclick={() => step(stepOrder[stepOrder.indexOf(kind) + 1])}
    >Passo successivo <ChevronRight size={15} /></button
  >
</div>
