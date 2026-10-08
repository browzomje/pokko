import type { Source } from "../types";
export function newSource(): Source {
  return {
    id: `custom-${crypto.randomUUID()}`,
    name: "Nuova fonte",
    base_url: "",
    category: "manga",
    language: "en",
    adapter: "generic",
    enabled: false,
    min_delay_ms: 1000,
    note: "",
    search_path: "",
    result_selector: "",
    chapter_selector: "",
    image_selector: "",
    description_selector: "",
    next_selector: "",
    reader_next_selector: "",
    author_selector: "",
    workflow: [],
  };
}
export type SelectorField =
  | "result_selector"
  | "chapter_selector"
  | "image_selector"
  | "description_selector"
  | "next_selector"
  | "reader_next_selector"
  | "author_selector";
export function fieldForStep(step: string): SelectorField {
  return step === "search"
    ? "result_selector"
    : step === "detail"
      ? "chapter_selector"
      : "image_selector";
}
export function inferSearchPath(address: string, query: string): string | null {
  try {
    const url = new URL(address),
      normalize = (s: string) => s.trim().replace(/\s+/g, " ").toLowerCase();
    if (!query.trim() || url.hash) return null;
    let found = false;
    const pairs = url.search
      .slice(1)
      .split("&")
      .filter(Boolean)
      .map((pair) => {
        const at = pair.indexOf("=");
        if (at < 0) return pair;
        const key = pair.slice(0, at),
          raw = pair.slice(at + 1),
          decoded = decodeURIComponent(raw.replaceAll("+", " "));
        if (normalize(decoded) === normalize(query)) {
          found = true;
          return `${key}={query}`;
        }
        return pair;
      });
    const segments = url.pathname.split("/").map((segment) => {
      if (normalize(decodeURIComponent(segment)) === normalize(query)) {
        found = true;
        return "{query}";
      }
      return segment;
    });
    if (!found) return null;
    return (
      segments.join("/") + (pairs.length ? "?" + pairs.join("&") : "")
    ).replace(/([?&](?:page|paged)=)\d+/, "$1{page}");
  } catch {
    return null;
  }
}

export const stageFields: Record<string, SelectorField[]> = {
  search: ["result_selector", "next_selector"],
  detail: ["chapter_selector", "description_selector", "author_selector"],
  reader: ["image_selector", "reader_next_selector"],
};
export function stageSignature(
  source: Source,
  kind: string,
  html: string,
  url: string,
): string {
  return JSON.stringify([
    kind,
    html,
    url,
    source.base_url,
    source.adapter,
    kind === "search" ? source.search_path : "",
    ...(stageFields[kind] || []).map((key) => source[key] || ""),
  ]);
}
