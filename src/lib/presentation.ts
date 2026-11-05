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

export function formatJobMessage(message: string): string {
  if (!message) return "";
  const trimmed = message.trim();

  if (trimmed === "In attesa") return t("downloads.msg.waiting");
  if (trimmed === "In pausa") return t("downloads.msg.paused");
  if (trimmed === "Ripresa richiesta") return t("downloads.msg.resuming");
  if (trimmed === "Estrazione delle pagine…")
    return t("downloads.msg.extractingPages");
  if (trimmed === "Pagina già presente, saltata")
    return t("downloads.msg.pageSkipped");
  if (trimmed === "Sessione ripristinata. Premi Riprendi per continuare")
    return t("downloads.msg.sessionRestored");
  if (
    trimmed === "File modificato o danneggiato: riparazione automatica in coda"
  )
    return t("downloads.msg.corruptQueued");
  if (trimmed === "Coda sospesa dopo un blocco del sito")
    return t("downloads.msg.queueSuspended");

  // "Creazione PDF · pagina 22 di 23"
  const creatingMatch = trimmed.match(
    /^Creazione\s+([A-Za-z0-9]+)\s+·\s+pagina\s+(\d+)\s+di\s+(\d+)$/i,
  );
  if (creatingMatch) {
    return t("downloads.msg.creatingPage", {
      format: creatingMatch[1].toUpperCase(),
      current: creatingMatch[2],
      total: creatingMatch[3],
    });
  }

  // "Creazione PDF · 23 pagine verificate"
  const creatingVerifiedMatch = trimmed.match(
    /^Creazione\s+([A-Za-z0-9]+)\s+·\s+(\d+)\s+pagine\s+verificate$/i,
  );
  if (creatingVerifiedMatch) {
    return t("downloads.msg.creatingVerified", {
      format: creatingVerifiedMatch[1].toUpperCase(),
      pages: creatingVerifiedMatch[2],
    });
  }

  // "Pagina 1 di 23 · tentativo 1/3"
  const pageAttemptMatch = trimmed.match(
    /^Pagina\s+(\d+)\s+di\s+(\d+)\s+·\s+tentativo\s+(\d+)\/(\d+)$/i,
  );
  if (pageAttemptMatch) {
    return t("downloads.msg.pageAttempt", {
      page: pageAttemptMatch[1],
      total: pageAttemptMatch[2],
      attempt: pageAttemptMatch[3],
    });
  }

  // "Completo · 23/23 pagine verificate · 0 riparate"
  const completeMatch = trimmed.match(
    /^Completo\s+·\s+(\d+)\/(\d+)\s+pagine\s+verificate\s+·\s+(\d+)\s+riparate$/i,
  );
  if (completeMatch) {
    return t("downloads.msg.completeVerified", {
      total: completeMatch[2],
      repaired: completeMatch[3],
    });
  }

  // "File già esportato · 23 pagine verificate"
  const alreadyExportedMatch = trimmed.match(
    /^File\s+già\s+esportato\s+·\s+(\d+)\s+pagine\s+verificate$/i,
  );
  if (alreadyExportedMatch) {
    return t("downloads.msg.alreadyExported", {
      pages: alreadyExportedMatch[1],
    });
  }

  return message;
}

export function jobProgress(job: Job): number {
  if (job.status === "completed") return 100;
  if (!job.volume?.chapters?.length) return 0;

  const format = (job.settings?.export_format || "pdf").toLowerCase();
  const isExportFormat = ["pdf", "cbz", "cbr"].includes(format);

  if (!isExportFormat) {
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

  // Two-phase calibration:
  // PDF takes ~25% for page scaling/JPEG encoding, CBZ/CBR take ~15%.
  const exportWeight = format === "pdf" ? 0.25 : 0.15;
  const downloadWeight = 1 - exportWeight; // 0.75 for PDF, 0.85 for CBZ

  const downloadRatio = Math.max(
    0,
    Math.min(
      1,
      (job.chapters_done +
        (job.pages_done && job.chapters_done < job.volume.chapters.length
          ? job.pages_done / (job.pages_total || 1)
          : 0)) /
        job.volume.chapters.length,
    ),
  );

  const isExporting =
    job.phase === "exporting" ||
    (job.export_done != null && job.export_done > 0) ||
    Boolean(job.message?.includes("Creazione")) ||
    Boolean(job.message?.includes("Creating"));

  if (isExporting) {
    const exportTotal = job.export_total || job.pages_total || 1;
    const exportDone = Math.min(job.export_done || 1, exportTotal);
    const exportRatio = Math.max(0, Math.min(1, exportDone / exportTotal));
    const base = downloadWeight * 100;
    const raw = Math.round(base + exportRatio * (99 - base));
    return Math.min(99, Math.max(Math.round(base), raw));
  }

  // During page downloading: scale from 0% up to (downloadWeight * 100)%
  const raw = Math.round(downloadRatio * (downloadWeight * 100));
  return Math.min(Math.round(downloadWeight * 100) - 1, Math.max(0, raw));
}
