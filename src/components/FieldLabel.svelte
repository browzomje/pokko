<script lang="ts">
  import { Info } from "lucide-svelte";
  let {
    text,
    help,
    icon: Icon = Info,
  }: { text: string; help: string; icon?: typeof Info } = $props();
</script>

<span class="field-caption"
  ><Icon size={12} aria-hidden="true" /><span>{text}</span><span
    class="field-help"
    ><button
      type="button"
      class="field-info"
      aria-label={`Informazioni: ${text || "abilitazione fonte"}`}
      title={help}><Info size={12} /></button
    ><span class="help-tooltip" role="tooltip">{help}</span></span
  ></span
>
