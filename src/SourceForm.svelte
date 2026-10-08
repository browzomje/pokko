<script lang="ts">
  import {
    TextCursorInput,
    Globe,
    Library,
    Languages,
    Plug,
    Timer,
    StickyNote,
    Search,
    Link,
    BookOpen,
    Image,
    AlignLeft,
    ChevronRight,
    UserRound,
  } from "lucide-svelte";
  import Select from "./Select.svelte";
  import FieldLabel from "./components/FieldLabel.svelte";
  import type { Source } from "./types";
  import { stageFields } from "./lib/source-config";
  import type { SelectorField } from "./lib/source-config";
  let {
    source = $bindable(),
    selectors = true,
    stage = "setup",
    onPick,
  }: {
    source: Source;
    selectors?: boolean;
    stage?: string;
    onPick?: (field: SelectorField) => void;
  } = $props();
  const fields: {
    key: SelectorField;
    text: string;
    icon: typeof Search;
    help: string;
  }[] = [
    {
      key: "result_selector",
      text: "Link dei titoli",
      icon: Link,
      help: "Nella pagina dei risultati, indica i link che aprono la scheda di ogni manga. Clicca una copertina o il suo titolo e seleziona il link padre. Es.: .post-title a.",
    },
    {
      key: "chapter_selector",
      text: "Link dei capitoli",
      icon: BookOpen,
      help: "Nella scheda della serie, indica i link dei capitoli o albi. L’app li segue in ordine numerico e cerca le immagini in ciascun lettore. Es.: .chapter-list a.",
    },
    {
      key: "image_selector",
      text: "Immagini delle scan",
      icon: Image,
      help: "Nella pagina che mostra le scan, indica le immagini del fumetto. Si leggono src, data-src o data-original, nell’ordine del DOM. Es.: .reading-content img. Non selezionare copertine o pubblicità.",
    },
    {
      key: "description_selector",
      text: "Descrizione",
      icon: AlignLeft,
      help: "Facoltativo: testo della trama nella scheda della serie. Es.: .description-summary. Non influenza il download.",
    },
    {
      key: "author_selector",
      text: "Autori",
      icon: UserRound,
      help: "Facoltativo: nomi degli autori nella scheda o nella scheda di ciascun risultato. Consente di dare priorità a titolo + autore nella ricerca. Es.: .author-content a. Se la fonte non li indica, non vengono inventati.",
    },
    {
      key: "next_selector",
      text: "Pagina successiva dei risultati",
      icon: ChevronRight,
      help: "Link “Successiva” della pagina di ricerca. Indica se c’è un’altra pagina del catalogo; il percorso ricerca usa {page} per aprirla. Non è il capitolo successivo.",
    },
    {
      key: "reader_next_selector",
      text: "Pagina successiva delle scan",
      icon: ChevronRight,
      help: "Facoltativo: se il capitolo è diviso su più pagine web, seleziona il link che mostra le altre scan dello stesso capitolo. L’app segue questo link fino alla fine. Non selezionare “Capitolo successivo”, altrimenti uniresti capitoli diversi.",
    },
  ];
</script>

<div class="source-fields">
  {#if stage === "setup"}
    <label
      ><FieldLabel
        text="Nome"
        icon={TextCursorInput}
        help="Nome della fonte mostrato nel catalogo. Puoi sceglierlo liberamente."
      /><input bind:value={source.name} required /></label
    >
    <label
      ><FieldLabel
        text="Indirizzo del sito"
        icon={Globe}
        help="La pagina iniziale del nuovo sito, per esempio https://example.org. Questo indirizzo identifica la fonte: non basta a conoscere come cerca i titoli. Nel passo Ricerca incollerai un URL dei risultati ottenuto realmente sul sito."
      /><input
        bind:value={source.base_url}
        readonly={source.adapter === "mangadex"}
        placeholder="https://example.org"
      /></label
    >
    <div class="source-field">
      <FieldLabel
        text="Catalogo"
        icon={Library}
        help="Manga o Comics: determina la categoria in Esplora."
      /><Select
        label="Catalogo della fonte"
        bind:value={source.category}
        options={[
          { value: "manga", label: "Manga" },
          { value: "comics", label: "Comics" },
        ]}
      />
    </div>
    <div class="source-field">
      <FieldLabel
        text="Lingua"
        icon={Languages}
        help="Lingua delle scan offerte da questa fonte. Per HTML generico è un’etichetta: non traduce né filtra automaticamente le pagine."
      /><Select
        label="Lingua della fonte"
        bind:value={source.language}
        options={[
          { value: "it", label: "🇮🇹 Italiano" },
          { value: "en", label: "🇬🇧 Inglese" },
          ...(source.adapter === "mangadex"
            ? [{ value: "both", label: "🇮🇹 🇬🇧 Entrambe" }]
            : []),
        ]}
      />
    </div>
    <div class="source-field">
      <FieldLabel
        text="Adattatore"
        icon={Plug}
        help="Per un nuovo sito scegli HTML generico e registra i selettori. Gli altri adattatori sono parser già programmati per un sito specifico; MangaDex usa un’API."
      /><Select
        label="Adattatore della fonte"
        bind:value={source.adapter}
        options={[
          { value: "generic", label: "HTML generico" },
          { value: "mangaworld", label: "MangaWorld" },
          { value: "readcomics", label: "ReadComicsOnline" },
          { value: "mangadex", label: "MangaDex API" },
        ]}
        onchange={(value) => {
          if (value === "mangadex") source.base_url = "https://mangadex.org";
          if (value === "readcomics") source.category = "comics";
          if (value !== "mangadex" && source.language === "both")
            source.language = "en";
        }}
      />
    </div>
    <label
      ><FieldLabel
        text="Intervallo minimo (ms)"
        icon={Timer}
        help="Attesa minima tra richieste a questa fonte. 1000 ms = 1 secondo. Si applica anche durante le prove online."
      /><input
        type="number"
        min="500"
        max="120000"
        step="500"
        bind:value={source.min_delay_ms}
      /></label
    >
    <label class="wide"
      ><FieldLabel
        text="Nota"
        icon={StickyNote}
        help="Promemoria personale sulla fonte. Non modifica la ricerca o i download."
      /><input bind:value={source.note} /></label
    >
  {/if}
  {#if selectors && source.adapter === "generic"}
    {#if stage === "search"}
      <label class="wide"
        ><FieldLabel
          text="Modello della ricerca, ricavato dal sito"
          icon={Search}
          help={"Parte dell’URL dopo il dominio. {query} viene sostituito con il titolo cercato; {page} con il numero di pagina. Esegui una ricerca di prova sul sito e registra l’URL: il laboratorio può ricavare questo modello. Es.: /search?q={query}&page={page}."}
        /><input bind:value={source.search_path} spellcheck="false" /></label
      >
    {/if}
    {#each fields.filter( (field) => (stageFields[stage] || []).includes(field.key) ) as field}<label
        ><FieldLabel
          text={field.text}
          icon={field.icon}
          help={field.help}
        /><input
          bind:value={source[field.key]}
          spellcheck="false"
          placeholder="Scegli un elemento nell’anteprima"
        />{#if onPick}<button
            type="button"
            class="source-pick"
            onclick={() => onPick?.(field.key)}>Seleziona dalla pagina</button
          >{/if}</label
      >{/each}
  {/if}
</div>
