<script lang="ts">
  import { previewDocument, installInspector } from "../lab-preview";
  import type {
    PreviewSample,
    InspectedElement,
    Inspector,
  } from "../lab-preview";
  let {
    html,
    sample,
    onInspect,
    onReady,
  }: {
    html: string;
    sample: PreviewSample;
    onInspect: (value: InspectedElement) => void;
    onReady: (value: Inspector | null) => void;
  } = $props();
  let frame = $state<HTMLIFrameElement>();
  let preview = $derived(previewDocument(html, sample));
</script>

<iframe
  bind:this={frame}
  class="lab-frame"
  title="Browser dei campioni: clicca sugli elementi"
  sandbox="allow-same-origin"
  srcdoc={preview}
  onload={() => frame && onReady(installInspector(frame, onInspect))}
></iframe>
