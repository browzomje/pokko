export type UILang =
  | "ar"
  | "zh"
  | "nl"
  | "en"
  | "fr"
  | "de"
  | "hi"
  | "it"
  | "ja"
  | "ko"
  | "pl"
  | "pt"
  | "ru"
  | "es"
  | "tr";

export interface UILanguageOption {
  code: UILang;
  label: string;
  nameEn: string;
  flag: string;
  dir?: "ltr" | "rtl";
}

export const availableUILanguages: UILanguageOption[] = [
  { code: "ar", label: "العربية", nameEn: "Arabic", flag: "🇸🇦", dir: "rtl" },
  { code: "zh", label: "中文", nameEn: "Chinese", flag: "🇨🇳" },
  { code: "nl", label: "Nederlands", nameEn: "Dutch", flag: "🇳🇱" },
  { code: "en", label: "English", nameEn: "English", flag: "🇬🇧" },
  { code: "fr", label: "Français", nameEn: "French", flag: "🇫🇷" },
  { code: "de", label: "Deutsch", nameEn: "German", flag: "🇩🇪" },
  { code: "hi", label: "हिन्दी", nameEn: "Hindi", flag: "🇮🇳" },
  { code: "it", label: "Italiano", nameEn: "Italian", flag: "🇮🇹" },
  { code: "ja", label: "日本語", nameEn: "Japanese", flag: "🇯🇵" },
  { code: "ko", label: "한국어", nameEn: "Korean", flag: "🇰🇷" },
  { code: "pl", label: "Polski", nameEn: "Polish", flag: "🇵🇱" },
  { code: "pt", label: "Português", nameEn: "Portuguese", flag: "🇵🇹" },
  { code: "ru", label: "Русский", nameEn: "Russian", flag: "🇷🇺" },
  { code: "es", label: "Español", nameEn: "Spanish", flag: "🇪🇸" },
  { code: "tr", label: "Türkçe", nameEn: "Turkish", flag: "🇹🇷" },
];

const supportedCodes = new Set<string>(availableUILanguages.map((l) => l.code));

const localeModules = import.meta.glob<Record<string, string>>("./locales/*.json", {
  eager: true,
  import: "default",
});

function getDictionary(lang: string): Record<string, string> {
  const direct = localeModules[`./locales/${lang}.json`];
  if (direct) return direct;
  return localeModules["./locales/it.json"] || localeModules["./locales/en.json"] || {};
}

const STORAGE_KEY = "pokko-ui-language";

function getSavedLanguage(): UILang {
  if (typeof window !== "undefined") {
    try {
      const saved = localStorage.getItem(STORAGE_KEY) as UILang | null;
      if (saved && supportedCodes.has(saved)) return saved;
      const nav = (navigator.language || "").toLowerCase();
      const prefix = nav.split("-")[0];
      if (supportedCodes.has(prefix)) return prefix as UILang;
    } catch {}
  }
  return "it";
}

function applyHtmlDirection(lang: UILang) {
  if (typeof document !== "undefined") {
    const opt = availableUILanguages.find((l) => l.code === lang);
    document.documentElement.dir = opt?.dir === "rtl" ? "rtl" : "ltr";
    document.documentElement.lang = lang;
  }
}

class I18nManager {
  current = $state<UILang>(getSavedLanguage());

  constructor() {
    applyHtmlDirection(this.current);
  }

  setLanguage(lang: UILang) {
    if (!supportedCodes.has(lang)) return;
    this.current = lang;
    applyHtmlDirection(lang);
    if (typeof window !== "undefined") {
      try {
        localStorage.setItem(STORAGE_KEY, lang);
      } catch {}
    }
  }

  t(key: string, params?: Record<string, string | number>): string {
    const dict = getDictionary(this.current);
    const fallbackIt = localeModules["./locales/it.json"] || {};
    const fallbackEn = localeModules["./locales/en.json"] || {};
    let text = dict[key] || fallbackIt[key] || fallbackEn[key] || key;
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        text = text.replaceAll(`{{${k}}}`, String(v)).replaceAll(`{${k}}`, String(v));
      }
    }
    return text;
  }
}

export const i18n = new I18nManager();
export const t = (key: string, params?: Record<string, string | number>) => i18n.t(key, params);
export const currentLanguage = () => i18n.current;
export const setLanguage = (lang: UILang) => i18n.setLanguage(lang);
