<script lang="ts">
  import {
    Search,
    Download,
    Globe,
    FlaskConical,
    Keyboard,
    Sun,
    Moon,
    Settings,
  } from "lucide-svelte";
  let {
    active,
    navigate,
    theme,
    onTheme,
    onShortcuts,
    onLab,
  }: {
    active: string;
    navigate: (section: string) => void;
    theme: string;
    onTheme: () => void;
    onShortcuts: () => void;
    onLab: () => void;
  } = $props();
</script>

<aside class="sidebar">
  <div class="pokko-brand"><img src="/pokko.svg" alt="" width="32" height="32" /><strong>pokko</strong></div>
  <div class="section-label">WORKSPACE</div>
  <button class:chosen={active === "search"} onclick={() => navigate("search")}
    ><Search size={18} /> Esplora <kbd>Ctrl K</kbd></button
  >
  <button class:chosen={active === "queue"} onclick={() => navigate("queue")}
    ><Download size={18} /> Downloads <kbd>Ctrl J</kbd></button
  >
  <button
    class:chosen={active === "sources"}
    onclick={() => navigate("sources")}
    ><Globe size={18} /> Fonti del catalogo <kbd>Ctrl ⇧ F</kbd></button
  >
  <button class:chosen={active === "lab"} onclick={onLab}
    ><FlaskConical size={18} /> Laboratorio fonti <kbd>Ctrl ⇧ L</kbd></button
  >
  <div class="sidebar-bottom">
    <button onclick={onShortcuts}
      ><Keyboard size={18} /> Scorciatoie <kbd>Ctrl ⇧ H</kbd></button
    >
    <button onclick={onTheme}
      >{#if theme === "dark"}<Sun size={18} />{:else}<Moon size={18} />{/if} Tema
      <kbd>Ctrl ⇧ T</kbd></button
    >
    <button
      class:chosen={active === "settings"}
      onclick={() => navigate("settings")}
      ><Settings size={18} /> Impostazioni <kbd>Ctrl ,</kbd></button
    >
  </div>
</aside>

<style>
  .pokko-brand { display: flex; align-items: center; gap: 10px; padding: 8px 12px 20px; font-size: 22px; letter-spacing: -0.5px; }
</style>
