import type { Manga } from "../types";
const normalize = (text: string) =>
  text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
/** Prefer matches across both the title and author metadata supplied by the source. */
export function rankSearchResults(items: Manga[], query: string): Manga[] {
  const tokens = normalize(query)
    .split(/[^\p{L}\p{N}]+/u)
    .filter(Boolean);
  const score = (m: Manga) => {
    const title = normalize(m.title),
      authors = normalize((m.authors || []).join(" "));
    return (
      tokens.reduce(
        (sum, t) => sum + (title.includes(t) ? 3 : authors.includes(t) ? 2 : 0),
        0,
      ) +
      (tokens.every((t) => title.includes(t) || authors.includes(t))
        ? tokens.length * 5
        : 0)
    );
  };
  return [...items].sort(
    (a, b) =>
      score(b) - score(a) ||
      (a.language === "en" ? 1 : 0) - (b.language === "en" ? 1 : 0) ||
      a.title.localeCompare(b.title) ||
      (a.source_id || "").localeCompare(b.source_id || "") ||
      (a.url || "").localeCompare(b.url || ""),
  );
}
