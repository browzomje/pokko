<script lang="ts">
  import {
    Search,
    ChevronLeft,
    Download,
    Globe,
    FlaskConical,
    Sun,
    Keyboard,
    Settings,
    X,
  } from "lucide-svelte";
  import { t } from "../lib/i18n";

  let { onClose }: { onClose: () => void } = $props();

  let shortcuts = $derived([
    { icon: Search, label: t("shortcuts.explore"), key: "Ctrl K" },
    { icon: ChevronLeft, label: t("shortcuts.back"), key: "Ctrl ← / Esc" },
    { icon: Download, label: t("shortcuts.downloads"), key: "Ctrl J" },
    { icon: Globe, label: t("shortcuts.sources"), key: "Ctrl Shift F" },
    { icon: FlaskConical, label: t("shortcuts.lab"), key: "Ctrl Shift L" },
    { icon: Sun, label: t("shortcuts.theme"), key: "Ctrl Shift T" },
    { icon: Keyboard, label: t("shortcuts.help"), key: "Ctrl Shift H" },
    { icon: Settings, label: t("shortcuts.settings"), key: "Ctrl ," },
    { icon: X, label: t("shortcuts.close"), key: "Esc" },
  ]);
</script>

<div
  class="modal-backdrop"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
>
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    aria-label={t("shortcuts.title")}
    tabindex="-1"
  >
    <div class="modal-heading">
      <h2><Keyboard size={19} /> {t("shortcuts.title")}</h2>
      <button class="icon" aria-label={t("shortcuts.close")} onclick={onClose}>
        <X size={18} />
      </button>
    </div>
    {#each shortcuts as shortcut}
      <div class="shortcut-row">
        <span class="shortcut-label">
          <shortcut.icon size={14} />{shortcut.label}
        </span>
        <kbd>{shortcut.key}</kbd>
      </div>
    {/each}
  </div>
</div>
