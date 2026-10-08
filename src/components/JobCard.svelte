<script lang="ts">
  import {
    Check,
    LoaderCircle,
    BookOpen,
    FolderOpen,
    Pause,
    Play,
    Trash2,
    ShieldCheck,
  } from "lucide-svelte";
  import {
    flag,
    jobLabels as labels,
    jobProgress as progress,
  } from "../lib/presentation";
  import type { Job } from "../types";
  let {
    job,
    openFolder,
    control,
    removeJob,
  }: {
    job: Job;
    openFolder: (job: Job) => void;
    control: (job: Job, action: string) => void;
    removeJob: (job: Job) => void;
  } = $props();
</script>

<article class="job" class:finished={job.status === "completed"}>
  <div class="job-top">
    <div class="job-icon">
      {#if job.status === "completed"}<Check
          size={22}
        />{:else if job.status === "running"}<LoaderCircle
          class="spin"
          size={22}
        />{:else}<BookOpen size={22} />{/if}
    </div>
    <div class="job-title">
      <h3>{job.manga.title}</h3>
      <span
        >{flag(job.manga.language)}
        {job.manga.source_name || "MangaWorld"} · {job.volume.title} · {job
          .volume.chapters.length}
        {job.volume.chapters.length === 1 ? "capitolo" : "capitoli"}</span
      >
    </div>
    <span class={`status ${job.status}`}>{labels[job.status]}</span>
    <div class="job-actions">
      <button
        class="icon"
        title="Apri cartella dei file"
        disabled={!job.output}
        onclick={() => openFolder(job)}><FolderOpen size={18} /></button
      >
      {#if ["running", "queued"].includes(job.status)}<button
          class="icon"
          title="Pausa"
          onclick={() => control(job, "pause")}><Pause size={17} /></button
        >{:else if ["paused", "error"].includes(job.status)}<button
          class="icon"
          title="Riprendi / riprova"
          onclick={() => control(job, "resume")}><Play size={17} /></button
        >{/if}<button
        class="icon danger"
        title="Elimina dalla coda"
        aria-label={`Elimina ${job.manga.title} ${job.volume.title} dalla coda`}
        onclick={() => removeJob(job)}><Trash2 size={19} /></button
      >
    </div>
  </div>
  {#if job.status !== "completed"}<div class="progress-label">
      <span
        >{job.chapter || "In attesa di iniziare"} · {job.pages_done}/{job.pages_total ||
          "—"} pagine</span
      ><strong>{progress(job)}%</strong>
    </div>
    <progress
      value={progress(job)}
      max="100"
      aria-label={`Progresso ${job.volume.title}`}
    ></progress>
    <div class="job-meta">
      <span
        >{job.chapters_done}/{job.volume.chapters.length}
        {job.volume.chapters.length === 1 ? "capitolo" : "capitoli"} completati ·
        {job.settings.export_format.toUpperCase()}</span
      ><span>{(job.bytes / 1024 / 1024).toFixed(1)} MB ricevuti</span>
    </div>
    <div
      class="verification"
      class:verified={job.verification?.state === "verified"}
      class:failure={job.verification?.state === "failed"}
      role="status"
    >
      <ShieldCheck size={15} />
      {#if job.verification?.state === "verified"}Verificato · {job.verification
          .verified}/{job.verification.total} pagine
      {:else if job.verification?.state === "repairing"}Riparazione automatica · {job
          .verification.verified}/{job.verification.total} pagine
      {:else if job.verification?.state === "failed"}Verifica incompleta ·
        riprendi per riprovare
      {:else if job.verification?.state === "checking"}Verifica pagine · {job
          .verification.verified}/{job.verification.total || "—"}
      {:else}Da verificare{/if}
      {#if job.verification?.repaired}<span
          >· {job.verification.repaired} riparate</span
        >{/if}
    </div>
  {/if}
  <p class:failure={job.status === "error"}>{job.message}</p>
  {#if job.output}<div class="output-path" title={job.output}>
      <FolderOpen size={14} /><span>{job.output}</span>
    </div>{/if}
</article>
