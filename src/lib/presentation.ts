import type { Job } from "../types";
import { getLanguageFlag } from "./languages";
import { t } from "./i18n";
import { Images, Archive, Package, FileText } from "lucide-svelte";

export const flag = (language: string) => {
  if (language === "both") return "🇮🇹 🇬🇧";
  return getLanguageFlag(language) || (language === "en" ? "🇬🇧" : "🇮🇹");
};

export const formats = [
  { value: "images", label: "Foto singole", icon: Images },
  { value: "cbz", label: "Archivio CBZ (ZIP)", icon: Archive },
  { value: "cbr", label: "Archivio CBR (RAR)", icon: Package },
  { value: "pdf", label: "File PDF", icon: FileText },
];

export const getFormats = () => [
  { value: "images", label: t("format.images"), icon: Images },
  { value: "cbz", label: t("format.cbz"), icon: Archive },
  { value: "cbr", label: t("format.cbr"), icon: Package },
  { value: "pdf", label: t("format.pdf"), icon: FileText },
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
  "API · disattivata: verifica la connessione prima di abilitarla":
    "sourceNote.md",
  "HTML verificato · capitoli senza gruppi volume": "sourceNote.mf",
  "HTML verificato · capitoli in inglese": "sourceNote.mangaread",
  "Comics occidentali · raccolte e albi · PDF completo nella coda":
    "sourceNote.readcomics",
  "Catalogo manga in lingua inglese · scansioni HD (FMHY)":
    "sourceNote.mangapill",
  "Catalogo manga classico in lingua inglese (FMHY)": "sourceNote.mangatown",
  "Catalogo manga in lingua inglese · scansioni HD": "sourceNote.mangapill",
  "Catalogo manga classico in lingua inglese": "sourceNote.mangatown",
  "HTML verificato WordPress Madara con capitoli in inglese":
    "sourceNote.mangaread",
  "HTML verificato con capitoli in lingua inglese": "sourceNote.mf",
  "Catalogo manga italiano con gruppi volumi e capitoli": "sourceNote.mw",
};

export function formatSourceNote(
  note: string | undefined,
  sourceId?: string,
): string {
  if (sourceId) {
    const key = `sourceNote.${sourceId}`;
    const localized = t(key);
    if (localized && localized !== key) return localized;
  }
  if (!note) return "";
  const key = PRESET_NOTE_KEYS[note.trim()];
  if (key) {
    const localized = t(key);
    if (localized && localized !== key) return localized;
  }
  return note;
}

export function jobProgress(job: Job) {
  if (job.status === "completed") return 100;
  if (!job.volume?.chapters?.length) return 0;
  const raw = Math.round(
    ((job.chapters_done +
      (job.pages_done && job.chapters_done < job.volume.chapters.length
        ? job.pages_done / (job.pages_total || 1)
        : 0)) /
      job.volume.chapters.length) *
      100,
  );
  return Math.min(99, Math.max(0, raw));
}
