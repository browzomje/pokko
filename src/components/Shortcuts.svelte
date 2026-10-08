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
  let { onClose }: { onClose: () => void } = $props();
  const shortcuts = [
    { icon: Search, label: "Esplora / cerca", key: "Ctrl K" },
    { icon: ChevronLeft, label: "Torna ai risultati", key: "Ctrl ← / Esc" },
    { icon: Download, label: "Downloads", key: "Ctrl J" },
    { icon: Globe, label: "Fonti del catalogo", key: "Ctrl Shift F" },
    { icon: FlaskConical, label: "Laboratorio fonti", key: "Ctrl Shift L" },
    { icon: Sun, label: "Cambia tema", key: "Ctrl Shift T" },
    { icon: Keyboard, label: "Scorciatoie", key: "Ctrl Shift H" },
    { icon: Settings, label: "Impostazioni", key: "Ctrl ," },
    { icon: X, label: "Chiudi finestra o messaggi", key: "Esc" },
  ];
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
    aria-label="Scorciatoie da tastiera"
    tabindex="-1"
  >
    <div class="modal-heading">
      <h2><Keyboard size={19} /> Scorciatoie da tastiera</h2>
      <button class="icon" aria-label="Chiudi scorciatoie" onclick={onClose}
        ><X size={18} /></button
      >
    </div>
    {#each shortcuts as shortcut}<div class="shortcut-row">
        <span class="shortcut-label"
          ><shortcut.icon size={14} />{shortcut.label}</span
        ><kbd>{shortcut.key}</kbd>
      </div>{/each}
  </div>
</div>
