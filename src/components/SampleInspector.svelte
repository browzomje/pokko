<script lang="ts">
  import { stageFields, type SelectorField } from "../lib/source-config";
  import Select from "../Select.svelte";
  import { t } from "../lib/i18n";
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
    <strong>{t("lab.selectedElement")}</strong>
    <p><code>{inspected.selector}</code></p>
    <details>
      <summary>{t("lab.htmlAndCss")}</summary>
      <pre>{inspected.html}</pre>
      <pre>{inspected.css}</pre>
    </details>
    <div class="lab-controls">
      <button onclick={() => onParent()}>{t("lab.parentElement")}</button>
      {#if generic}<Select
          label={t("lab.applySelectorToField")}
          bind:value={selectorField}
          options={[
            { value: "result_selector", label: t("lab.selectorResults") },
            { value: "chapter_selector", label: t("lab.selectorChapters") },
            { value: "image_selector", label: t("lab.selectorImages") },
            { value: "description_selector", label: t("lab.selectorDescription") },
            { value: "next_selector", label: t("lab.selectorNext") },
            {
              value: "reader_next_selector",
              label: t("lab.selectorReaderNext"),
            },
            { value: "author_selector", label: t("lab.selectorAuthors") },
          ].filter((option) =>
            (stageFields[stage] || []).includes(option.value as SelectorField),
          )}
        /><button disabled={!!busy} onclick={() => onApply()}
          >{t("lab.useSelector")}</button
        ><button disabled={!!busy} onclick={() => onApply(true)}
          >{t("lab.useExactPath")}</button
        >{/if}
    </div>
  </div>{/if}
