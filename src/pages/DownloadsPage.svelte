<script lang="ts">
  import JobCard from "../components/JobCard.svelte";
  import { ArrowDownToLine, Trash2 } from "lucide-svelte";
  import { t } from "../lib/i18n";

  import type { Job } from "../types";
  let {
    jobs,
    onSearch: _goSearch,
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
    <h1>{t("downloads.title")}</h1>
  </div>
  <div class="page-heading-actions">
    <span class="pill">
      {completed} {t("downloads.completed")}{#if archived} · {archived} {t("downloads.archived")}{/if} · {pending} {t("downloads.queued")}
    </span>
    <button
      class="danger"
      disabled={!jobs.some((j) => j.status !== "completed")}
      onclick={removeAll}
    >
      <Trash2 size={16} /> {t("downloads.clearIncomplete")}
    </button>
  </div>
</div>

{#if !jobs.length}
  <div class="empty">
    <div class="empty-icon"><ArrowDownToLine size={34} /></div>
    <h2>{t("downloads.emptyTitle")}</h2>
    <p>{t("downloads.emptyDesc")}</p>
  </div>
{:else}
  <div class="downloads-filter-bar">
    <div class="lab-tabs">
      <button
        class:chosen={queueFilter === "active"}
        onclick={() => (queueFilter = "active")}
      >
        In corso ({pending})
      </button>
      <button
        class:chosen={queueFilter === "completed"}
        onclick={() => (queueFilter = "completed")}
      >
        Completati ({completed})
      </button>
    </div>

    {#if queueFilter === "completed" && completed > 0}
      <button class="action-btn" onclick={onArchive}>
        {t("downloads.clearCompleted")}
      </button>
    {/if}
  </div>

  <div class="queue-list">
    {#each visibleJobs as job (job.id)}
      <JobCard
        {job}
        {control}
        {removeJob}
        {openFolder}
      />
    {/each}
  </div>
{/if}
