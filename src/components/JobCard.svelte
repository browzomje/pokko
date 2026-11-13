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
    formatJobMessage,
  } from "../lib/presentation";
  import { t } from "../lib/i18n";
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
    <div class="job-icon" class:has-cover={Boolean(job.manga?.cover)}>
      {#if job.manga?.cover}
        <img
          src={job.manga.cover}
          alt={job.manga.title}
          class="job-cover-img"
          loading="lazy"
        />
        <div class="job-cover-badge {job.status}">
          {#if job.status === "completed"}
            <Check size={12} strokeWidth={3} />
          {:else if job.status === "running"}
            <LoaderCircle class="spin" size={12} strokeWidth={3} />
          {:else}
            <BookOpen size={12} />
          {/if}
        </div>
      {:else if job.status === "completed"}
        <Check size={22} />
      {:else if job.status === "running"}
        <LoaderCircle class="spin" size={22} />
      {:else}
        <BookOpen size={22} />
      {/if}
    </div>
    <div class="job-title">
      <h3>{job.manga.title}</h3>
      <span
        >{flag(job.manga.language)}
        {#if job.manga.source_name || job.manga.source_id}{job.manga
            .source_name || job.manga.source_id} · {/if}{job.volume.title} · {job
          .volume.chapters.length}
        {job.volume.chapters.length === 1
          ? t("downloads.chapter")
          : t("downloads.chapters")}</span
      >
    </div>
    <span class={`status ${job.status}`}
      >{t(`downloads.status.${job.status}`) || labels[job.status]}</span
    >
    <div class="job-actions">
      <button
        class="icon"
        title={t("downloads.openFolder")}
        disabled={!job.output}
        onclick={() => openFolder(job)}><FolderOpen size={18} /></button
      >
      {#if ["running", "queued"].includes(job.status)}<button
          class="icon"
          title={t("downloads.pause")}
          onclick={() => control(job, "pause")}><Pause size={17} /></button
        >{:else if ["paused", "error"].includes(job.status)}<button
          class="icon"
          title={t("downloads.resume")}
          onclick={() => control(job, "resume")}><Play size={17} /></button
        >{/if}<button
        class="icon danger"
        title={t("downloads.cancel")}
        aria-label={`${t("downloads.cancel")} ${job.manga.title} ${job.volume.title}`}
        onclick={() => removeJob(job)}><Trash2 size={19} /></button
      >
    </div>
  </div>
  {#if job.status !== "completed"}<div class="progress-label">
      {#if job.phase === "exporting" || (job.export_done && job.export_done > 0)}
        <span
          >{t("downloads.exportProgress", {
            format: job.settings.export_format.toUpperCase(),
          })} · {job.export_done || job.pages_done}/{job.export_total ||
            job.pages_total} {t("downloads.pages")}</span
        >
      {:else}
        <span
          >{job.chapter || t("downloads.waitingToStart")} · {job.pages_done}/{job.pages_total ||
            "—"} {t("downloads.pages")}</span
        >
      {/if}<strong>{progress(job)}%</strong>
    </div>
    <progress
      value={progress(job)}
      max="100"
      aria-label={`${t("downloads.title")} ${job.volume.title}`}
    ></progress>
    <div class="job-meta">
      <span
        >{job.chapters_done}/{job.volume.chapters.length}
        {job.volume.chapters.length === 1
          ? t("downloads.chapter")
          : t("downloads.chapters")} completati ·
        {job.settings.export_format.toUpperCase()}</span
      ><span>{t("downloads.receivedMb", { mb: (job.bytes / 1024 / 1024).toFixed(1) })}</span>
    </div>
    <div
      class="verification"
      class:verified={job.verification?.state === "verified"}
      class:failure={job.verification?.state === "failed"}
      role="status"
    >
      <ShieldCheck size={15} />
      {#if job.verification?.state === "verified"}{t("downloads.verification.verified", {
          verified: job.verification.verified,
          total: job.verification.total,
        })}
      {:else if job.verification?.state === "repairing"}{t("downloads.verification.repairing", {
          verified: job.verification.verified,
          total: job.verification.total,
        })}
      {:else if job.verification?.state === "failed"}{t("downloads.verification.failed")}
      {:else if job.verification?.state === "checking"}{t("downloads.verification.checking", {
          verified: job.verification.verified,
          total: job.verification.total || "—",
        })}
      {:else}{t("downloads.verification.toVerify")}{/if}
      {#if job.verification?.repaired}<span
          >{t("downloads.verification.repaired", {
            count: job.verification.repaired,
          })}</span
        >{/if}
    </div>
  {/if}
  <p class:failure={job.status === "error"}>{formatJobMessage(job.message)}</p>
  {#if job.output}<div class="output-path" title={job.output}>
      <FolderOpen size={14} /><span>{job.output}</span>
    </div>{/if}
</article>
