<script lang="ts">
  import { BookOpen } from "lucide-svelte";
  let { url, title }: { url: string; title: string } = $props();
  let src = $state("");
  let failed = $state(false);
  $effect(() => {
    const target = url;
    src = "";
    failed = false;
    let alive = true;
    src = target;
    return () => {
      alive = false;
    };
  });
</script>

{#if src && !failed}<img
    {src}
    alt={`Copertina di ${title}`}
    loading="lazy"
    referrerpolicy="no-referrer"
    onerror={() => (failed = true)}
  />{:else}<div class="cover-placeholder">
    <BookOpen size={35} /><span>{title}</span>
  </div>{/if}
