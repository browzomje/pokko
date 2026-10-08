<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { BookOpen } from "lucide-svelte";
  let { url, title }: { url: string; title: string } = $props();
  let src = $state("");
  let failed = $state(false);
  $effect(() => {
    const target = url;
    src = "";
    failed = false;
    let alive = true;
    const load = async () => {
      try {
        const value =
          target.includes("uploads.mangadex.org") && isTauri()
            ? await invoke<string>("manga_cover", { url: target })
            : target;
        if (alive) src = value;
      } catch {
        if (alive) failed = true;
      }
    };
    void load();
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
