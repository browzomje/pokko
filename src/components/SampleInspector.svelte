<script lang="ts">
  import { stageFields, type SelectorField } from "../lib/source-config";
  import Select from "../Select.svelte";
  import type { InspectedElement } from "../lab-preview";
  let {
    inspected,
    selectorField = $bindable(),
    generic,
    stage,
    busy,
    onApply,
    onParent,
  }: {
    inspected: InspectedElement | null;
    selectorField: string;
    generic: boolean;
    stage: string;
    busy: string;
    onApply: (exact?: boolean) => void;
    onParent: () => void;
  } = $props();
</script>

{#if inspected}<div class="lab-inspector">
    <strong>Elemento selezionato</strong>
    <p><code>{inspected.selector}</code></p>
    <details>
      <summary>HTML e CSS dell’elemento</summary>
      <pre>{inspected.html}</pre>
      <pre>{inspected.css}</pre>
    </details>
    <div class="lab-controls">
      <button onclick={() => onParent()}>Elemento padre</button>
      {#if generic}<Select
          label="Applica selettore al campo"
          bind:value={selectorField}
          options={[
            { value: "result_selector", label: "Risultati" },
            { value: "chapter_selector", label: "Capitoli" },
            { value: "image_selector", label: "Immagini" },
            { value: "description_selector", label: "Descrizione" },
            { value: "next_selector", label: "Risultati successivi" },
            {
              value: "reader_next_selector",
              label: "Pagina successiva del lettore",
            },
            { value: "author_selector", label: "Autori" },
          ].filter((option) =>
            (stageFields[stage] || []).includes(option.value as SelectorField),
          )}
        /><button disabled={!!busy} onclick={() => onApply()}
          >Usa selettore</button
        ><button disabled={!!busy} onclick={() => onApply(true)}
          >Usa percorso esatto</button
        >{/if}
    </div>
  </div>{/if}
