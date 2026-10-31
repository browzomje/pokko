import type { Job } from "../types";
import { getLanguageFlag } from "./languages";
import { t } from "./i18n";

export const flag = (language: string) => {
  if (language === "both") return "🇮🇹 🇬🇧";
  return getLanguageFlag(language) || (language === "en" ? "🇬🇧" : "🇮🇹");
};

export const formats = [
  { value: "images", label: "Foto singole" },
  { value: "cbz", label: "Archivio CBZ (ZIP)" },
  { value: "cbr", label: "Archivio CBR (RAR)" },
  { value: "pdf", label: "File PDF" },
];

export const getFormats = () => [
  { value: "images", label: t("format.images") },
  { value: "cbz", label: t("format.cbz") },
  { value: "cbr", label: t("format.cbr") },
  { value: "pdf", label: t("format.pdf") },
];

export const jobLabels: Record<string, string> = {
  queued: "In coda",
  running: "In corso",
  paused: "In pausa",
  completed: "Completato",
  error: "Errore",
  cancelled: "Annullato",
};

const PRESET_NOTE_KEYS: Record<string, string> = {
  "Verificato · catalogo e volumi": "sourceNote.mw",
  "API · disattivata: verifica la connessione prima di abilitarla": "sourceNote.md",
  "HTML verificato · capitoli senza gruppi volume": "sourceNote.mf",
  "HTML verificato · capitoli in inglese": "sourceNote.mangaread",
  "Comics occidentali · raccolte e albi · PDF completo nella coda": "sourceNote.readcomics",
  "Berserk completo in inglese · scansioni HD e capitoli ordinati": "sourceNote.readberserk",
};

export function formatSourceNote(note: string | undefined): string {
  if (!note) return "";
  const key = PRESET_NOTE_KEYS[note.trim()];
  if (key) {
    return t(key);
  }
  return note;
}

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
