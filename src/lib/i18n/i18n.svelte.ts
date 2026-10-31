import it from "./locales/it.json";
import en from "./locales/en.json";

export type UILang = "it" | "en";

export interface UILanguageOption {
  code: UILang;
  label: string;
  flag: string;
}

export const availableUILanguages: UILanguageOption[] = [
  { code: "it", label: "Italiano", flag: "🇮🇹" },
  { code: "en", label: "English", flag: "🇬🇧" },
];

const dictionaries: Record<UILang, Record<string, string>> = {
  it,
  en,
};

const STORAGE_KEY = "pokko-ui-language";

function getSavedLanguage(): UILang {
  if (typeof window !== "undefined") {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved === "it" || saved === "en") return saved;
      const nav = navigator.language?.toLowerCase() || "";
      if (nav.startsWith("it")) return "it";
      if (nav.startsWith("en")) return "en";
    } catch {}
  }
  return "it";
}

class I18nManager {
  current = $state<UILang>(getSavedLanguage());

  setLanguage(lang: UILang) {
    this.current = lang;
    if (typeof window !== "undefined") {
      try {
        localStorage.setItem(STORAGE_KEY, lang);
      } catch {}
    }
  }

  t(key: string, params?: Record<string, string | number>): string {
    const dict = dictionaries[this.current] || dictionaries.it;
    let text = dict[key] || dictionaries.it[key] || dictionaries.en[key] || key;
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        text = text.replaceAll(`{{${k}}}`, String(v));
      }
    }
    return text;
  }
}

export const i18n = new I18nManager();
export const t = (key: string, params?: Record<string, string | number>) => i18n.t(key, params);
export const currentLanguage = () => i18n.current;
export const setLanguage = (lang: UILang) => i18n.setLanguage(lang);
