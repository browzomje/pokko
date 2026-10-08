import type { Job } from "../types";
export const flag = (language: string) =>
  language === "en" ? "🇬🇧" : language === "both" ? "🇮🇹 🇬🇧" : "🇮🇹";
export const formats = [
  { value: "images", label: "Foto singole" },
  { value: "cbz", label: "Archivio CBZ (ZIP)" },
  { value: "cbr", label: "Archivio CBR (RAR)" },
  { value: "pdf", label: "File PDF" },
];
export const jobLabels: Record<string, string> = {
  queued: "In coda",
  running: "In corso",
  paused: "In pausa",
  completed: "Completato",
  error: "Errore",
  cancelled: "Annullato",
};
export function jobProgress(job: Job) {
  if (job.status === "completed") return 100;
  return Math.min(
    100,
    Math.round(
      ((job.chapters_done +
        (job.pages_total && job.chapters_done < job.volume.chapters.length
          ? job.pages_done / job.pages_total
          : 0)) /
        job.volume.chapters.length) *
        100,
    ),
  );
}
