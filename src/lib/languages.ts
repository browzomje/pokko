export interface Language {
  code: string;
  name: string;
  nameEn: string;
  flag: string;
}

export const languages: Language[] = [
  { code: "ar", name: "العربية", nameEn: "Arabic", flag: "🇸🇦" },
  { code: "bg", name: "Български", nameEn: "Bulgarian", flag: "🇧🇬" },
  { code: "bn", name: "বাংলা", nameEn: "Bengali", flag: "🇧🇩" },
  { code: "ca", name: "Català", nameEn: "Catalan", flag: "🇪🇸" },
  { code: "cs", name: "Čeština", nameEn: "Czech", flag: "🇨🇿" },
  { code: "da", name: "Dansk", nameEn: "Danish", flag: "🇩🇰" },
  { code: "de", name: "Deutsch", nameEn: "German", flag: "🇩🇪" },
  { code: "el", name: "Ελληνικά", nameEn: "Greek", flag: "🇬🇷" },
  { code: "en", name: "English", nameEn: "English", flag: "🇬🇧" },
  { code: "es", name: "Español", nameEn: "Spanish", flag: "🇪🇸" },
  { code: "fa", name: "فارسی", nameEn: "Persian", flag: "🇮🇷" },
  { code: "fi", name: "Suomi", nameEn: "Finnish", flag: "🇫🇮" },
  { code: "fr", name: "Français", nameEn: "French", flag: "🇫🇷" },
  { code: "he", name: "עברית", nameEn: "Hebrew", flag: "🇮🇱" },
  { code: "hi", name: "हिंदी", nameEn: "Hindi", flag: "🇮🇳" },
  { code: "hr", name: "Hrvatski", nameEn: "Croatian", flag: "🇭🇷" },
  { code: "hu", name: "Magyar", nameEn: "Hungarian", flag: "🇭🇺" },
  { code: "id", name: "Bahasa Indonesia", nameEn: "Indonesian", flag: "🇮🇩" },
  { code: "is", name: "Íslenska", nameEn: "Icelandic", flag: "🇮🇸" },
  { code: "it", name: "Italiano", nameEn: "Italian", flag: "🇮🇹" },
  { code: "ja", name: "日本語", nameEn: "Japanese", flag: "🇯🇵" },
  { code: "ko", name: "한국어", nameEn: "Korean", flag: "🇰🇷" },
  { code: "lt", name: "Lietuvių", nameEn: "Lithuanian", flag: "🇱🇹" },
  { code: "lv", name: "Latviešu", nameEn: "Latvian", flag: "🇱🇻" },
  { code: "ms", name: "Bahasa Melayu", nameEn: "Malay", flag: "🇲🇾" },
  { code: "nl", name: "Nederlands", nameEn: "Dutch", flag: "🇳🇱" },
  { code: "no", name: "Norsk", nameEn: "Norwegian", flag: "🇳🇴" },
  { code: "pl", name: "Polski", nameEn: "Polish", flag: "🇵🇱" },
  { code: "pt", name: "Português", nameEn: "Portuguese", flag: "🇵🇹" },
  { code: "pt-br", name: "Português (Brasil)", nameEn: "Portuguese Brazil", flag: "🇧🇷" },
  { code: "ro", name: "Română", nameEn: "Romanian", flag: "🇷🇴" },
  { code: "ru", name: "Русский", nameEn: "Russian", flag: "🇷🇺" },
  { code: "sk", name: "Slovenčina", nameEn: "Slovak", flag: "🇸🇰" },
  { code: "sl", name: "Slovenščina", nameEn: "Slovenian", flag: "🇸🇮" },
  { code: "sr", name: "Српски", nameEn: "Serbian", flag: "🇷🇸" },
  { code: "sv", name: "Svenska", nameEn: "Swedish", flag: "🇸🇪" },
  { code: "th", name: "ไทย", nameEn: "Thai", flag: "🇹🇭" },
  { code: "tl", name: "Tagalog", nameEn: "Tagalog / Filipino", flag: "🇵🇭" },
  { code: "tr", name: "Türkçe", nameEn: "Turkish", flag: "🇹🇷" },
  { code: "uk", name: "Українська", nameEn: "Ukrainian", flag: "🇺🇦" },
  { code: "ur", name: "اردو", nameEn: "Urdu", flag: "🇵🇰" },
  { code: "vi", name: "Tiếng Việt", nameEn: "Vietnamese", flag: "🇻🇳" },
  { code: "zh", name: "中文 (简体)", nameEn: "Chinese Simplified", flag: "🇨🇳" },
  { code: "zh-tw", name: "中文 (繁體)", nameEn: "Chinese Traditional", flag: "🇹🇼" },
];

export const specialLanguages: Language[] = [
  { code: "both", name: "Entrambe (IT/EN)", nameEn: "Both (Italian/English)", flag: "🇮🇹 🇬🇧" },
];

export const languageAliases: Record<string, string[]> = {
  ar: ["ara", "arb", "arabic", "arabo", "arab", "arabia"],
  bg: ["bul", "bulgarian", "bulgaro"],
  bn: ["ben", "bengali", "bangla"],
  ca: ["cat", "catalan", "català", "catala", "catalano"],
  cs: ["cze", "ces", "czech", "čeština", "cestina", "ceco"],
  da: ["dan", "danish", "dansk", "danese"],
  de: ["ger", "deu", "german", "deutsch", "tedesco"],
  el: ["gre", "ell", "greek", "ελληνικά", "ellinika", "greco"],
  en: ["eng", "english", "inglese", "ingles", "anglais", "inglés", "en-us", "en-gb"],
  es: ["spa", "esp", "spanish", "español", "espanol", "espaniol", "castellano", "spagnolo"],
  fa: ["per", "fas", "persian", "farsi", "persiano"],
  fi: ["fin", "finnish", "suomi", "finlandese"],
  fr: ["fre", "fra", "french", "français", "francais", "francese"],
  he: ["heb", "hebrew", "עברית", "ivrit", "ebraico"],
  hi: ["hin", "hindi", "हिंदी"],
  hr: ["hrv", "croatian", "croato"],
  hu: ["hun", "hungarian", "magyar", "ungherese"],
  id: ["ind", "indonesian", "bahasa indonesia", "indonesiano"],
  is: ["ice", "isl", "icelandic", "íslenska", "islenska", "islandese"],
  it: ["ita", "italian", "italiano", "italiana"],
  ja: ["jpn", "japanese", "日本語", "nihongo", "giapponese"],
  ko: ["kor", "korean", "한국어", "hangul", "coreano"],
  lt: ["lit", "lithuanian", "lituano"],
  lv: ["lav", "latvian", "lettone"],
  ms: ["may", "msa", "malay", "bahasa melayu", "malese"],
  nl: ["dut", "nld", "dutch", "nederlands", "olandese", "neerlandese"],
  no: ["nor", "norwegian", "norsk", "norvegese", "nb", "nob", "nn", "nno"],
  pl: ["pol", "polish", "polski", "polacco"],
  pt: ["por", "portuguese", "português", "portugues", "portoghese"],
  "pt-br": ["por", "pt-br", "ptbr", "brazilian portuguese", "português brasil", "portugues brasil", "portoghese brasiliano", "brasiliano"],
  ro: ["rum", "ron", "romanian", "română", "romana", "rumeno"],
  ru: ["rus", "russian", "русский", "russkiy", "russo"],
  sk: ["slk", "slovak", "slovacco"],
  sl: ["slv", "slovenian", "sloveno"],
  sr: ["srp", "serbian", "serbo"],
  sv: ["swe", "sved", "swedish", "svenska", "svedese"],
  th: ["tha", "thai", "ไทย", "tailandese"],
  tl: ["tgl", "tagalog", "filipino", "filippino"],
  tr: ["tur", "turkish", "türkçe", "turkce", "turco"],
  uk: ["ukr", "ukrainian", "українська", "ukrayinska", "ucraino"],
  ur: ["urd", "urdu"],
  vi: ["vie", "vietnamese", "tiếng việt", "tieng viet", "vietnamita"],
  zh: ["chi", "zho", "cmn", "zh-cn", "chs", "chinese", "mandarin", "simplified chinese", "cinese", "cinese semplificato"],
  "zh-tw": ["chi", "zho", "cmn", "zh-hant", "zh-tw", "cht", "traditional chinese", "cinese tradizionale", "taiwanese mandarin"],
  both: ["both", "entrambe", "tutte", "it-en", "all", "multilingual", "italiano e inglese"],
};

export function normalizeLanguageText(value: string | null | undefined): string {
  return (value || "")
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[’']/g, "")
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .trim();
}

export function getLanguageSearchTerms(code: string): string {
  const all = [...languages, ...specialLanguages];
  const lang = all.find((item) => item.code === code);
  return [
    code,
    code.split("-")[0],
    lang?.nameEn,
    lang?.name,
    ...(languageAliases[code] || []),
    ...(languageAliases[code.split("-")[0]] || []),
  ]
    .filter(Boolean)
    .map((term) => String(term))
    .join(" ");
}

export function getLanguage(code: string): Language | undefined {
  const norm = (code || "").trim().toLowerCase();
  const all = [...languages, ...specialLanguages];
  return all.find((l) => l.code === norm) || all.find((l) => l.code === norm.split("-")[0]);
}

export function getLanguageFlag(code: string): string {
  if (code === "both") return "🇮🇹 🇬🇧";
  return getLanguage(code)?.flag || "";
}

export function getLanguageLabel(lang: Language): string {
  if (lang.code === "both") return `${lang.flag} ${lang.name}`;
  if (lang.nameEn === lang.name) {
    return `${lang.flag} ${lang.name}`;
  }
  return `${lang.flag} ${lang.name} — ${lang.nameEn}`;
}
