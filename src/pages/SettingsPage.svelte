<script lang="ts">
  import { Pause, FolderOpen } from "lucide-svelte";

  import Select from "../Select.svelte";
  import { formats } from "../lib/presentation";
  import type { Settings } from "../types";
  let {
    settings = $bindable(),
    saveError,
    saveStatus,
    chooseFolder,
  }: {
    settings: Settings;
    saveError: string;
    saveStatus: string;
    chooseFolder: () => void;
  } = $props();
</script>

<div class="page-heading">
  <div>
    <div class="eyebrow">PREFERENZE</div>
    <h1>Impostazioni</h1>
    <p>Le modifiche vengono salvate automaticamente.</p>
    <div class="save-status" class:failure={!!saveError} role="status">
      {saveStatus}{#if saveError}<span>{saveError}</span>{/if}
    </div>
  </div>
</div>
<div class="settings-card">
  <h2><FolderOpen size={19} /> Destinazione</h2>
  <p>Le scan vengono ordinate per manga, volume, capitolo e pagina.</p>
  <label for="output">Cartella di download</label>
  <div class="input-row">
    <input
      id="output"
      bind:value={settings.output}
      placeholder="Scegli una cartella…"
    /><button onclick={chooseFolder}><FolderOpen size={16} /> Sfoglia</button>
  </div>
  <div class="source-field">
    <span>Formato predefinito</span><Select
      label="Formato di esportazione"
      bind:value={settings.export_format}
      options={formats}
    />
  </div>
  <small
    >Ogni download usa questo formato senza chiedere conferma. CBZ, CBR e PDF
    usano immagini temporanee, rimosse dopo l’esportazione riuscita.</small
  >
</div>
<div class="settings-card">
  <h2><Pause size={19} /> Ritmo delle richieste</h2>
  <p>
    La ricerca usa fino a tre fonti indipendenti in parallelo. I download
    rispettano l’intervallo di ogni dominio.
  </p>
  <label for="delay"
    >Intervallo tra richieste: {settings.delay_ms / 1000}
    {settings.delay_ms === 1000 ? "secondo" : "secondi"}</label
  ><input
    id="delay"
    type="range"
    min="500"
    max="10000"
    step="500"
    bind:value={settings.delay_ms}
  /><button onclick={() => (settings.delay_ms = 500)}
    >Profilo rapido · 0,5 s</button
  ><small
    >Le immagini su CDN separati usano 0,5 s; catalogo e lettori rispettano
    anche il minimo della fonte. Le pause riducono il carico sul sito, ma non
    garantiscono l’assenza di blocchi. Su HTTP 403 o 429 la coda si sospende.</small
  >
</div>
