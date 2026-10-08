<script lang="ts">
  import JobCard from "../components/JobCard.svelte";
  import { Search, Check, ArrowDownToLine, Trash2 } from "lucide-svelte";

  import type { Job } from "../types";
  let {
    jobs,
    onSearch: goSearch,
    removeAll,
    removeJob,
    openFolder,
    control,
    onArchive,
  }: {
    jobs: Job[];
    onSearch: () => void;
    removeAll: () => void;
    removeJob: (job: Job) => void;
    openFolder: (job: Job) => void;
    control: (job: Job, action: string) => void;
    onArchive: () => void;
  } = $props();
  let queueFilter = $state("active");
  let showArchived = $state(false);
  let pending = $derived(jobs.filter((j) => j.status !== "completed").length);
  let completed = $derived(
    jobs.filter((j) => j.status === "completed" && !j.archived).length,
  );
  let archived = $derived(
    jobs.filter((j) => j.status === "completed" && j.archived).length,
  );
  let visibleJobs = $derived(
    jobs.filter((j) =>
      queueFilter === "active"
        ? j.status !== "completed"
        : j.status === "completed" && (showArchived || !j.archived),
    ),
  );
</script>

<div class="page-heading">
  <div>
    <div class="eyebrow">ATTIVITÀ</div>
    <h1>Downloads</h1>
    <p>Metti in pausa un download per lasciare proseguire il successivo.</p>
  </div>
  <button
    class="danger"
    disabled={!jobs.some((j) => j.status !== "completed")}
    onclick={removeAll}
    ><Trash2 size={17} /> Svuota download non completati</button
  >
  <span class="pill"
    >{completed} completati{#if archived}
      · {archived} archiviati{/if} · {pending} in coda</span
  >
</div>
{#if !jobs.length}<div class="empty">
    <div class="empty-icon"><ArrowDownToLine size={34} /></div>
    <h2>La coda è pronta</h2>
    <p>Apri un manga e aggiungi i volumi che vuoi conservare.</p>
    <button class="primary" onclick={goSearch}
      ><Search size={16} /> Cerca un manga</button
    >
  </div>{/if}
{#if jobs.length}<div class="queue-filters" aria-label="Filtra download">
    {#each [{ value: "active", label: "Coda", count: pending }, { value: "completed", label: "Cronologia", count: completed }] as filter}
      <button
        class:chosen={queueFilter === filter.value}
        aria-pressed={queueFilter === filter.value}
        onclick={() => (queueFilter = filter.value)}
        >{filter.label}<span class="count">{filter.count}</span></button
      >
    {/each}
  </div>
  {#if !visibleJobs.length}<div class="empty">
      <Check size={28} />
      <h2>Nessun download in questa sezione</h2>
      <button
        onclick={() => {
          queueFilter = "completed";
          showArchived = true;
        }}>Mostra cronologia completa</button
      >
    </div>{/if}{/if}
{#if queueFilter === "completed"}<div class="lab-controls">
    <button disabled={!completed} onclick={onArchive}
      ><Trash2 size={16} /> Pulisci cronologia</button
    ><label class="checkbox"
      ><input type="checkbox" bind:checked={showArchived} /> Mostra archiviati</label
    ><small
      >I completati passano qui automaticamente. Pulire archivia le voci e
      conserva i file e le verifiche.</small
    >
  </div>{/if}
<div class="queue-list">
  {#each visibleJobs as job (job.id)}<JobCard
      {job}
      {openFolder}
      {control}
      {removeJob}
    />{/each}
</div>
