import type { Source } from "../types";
export function newSource(): Source {
  return {
    id: `custom-${crypto.randomUUID()}`,
    name: "",
    base_url: "",
    category: "manga",
    language: "en",
    adapter: "generic",
    enabled: false,
    origin: "created",
    min_delay_ms: 500,
    note: "",
    search_path: "",
    result_selector: "",
    chapter_selector: "",
    image_selector: "",
    description_selector: "",
    next_selector: "",
    reader_next_selector: "",
    author_selector: "",
    chapter_url_suffix: "",
    volume_selector: "",
    volume_title_selector: "",
    chapter_title_selector: "",
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
  | "author_selector"
  | "chapter_url_suffix"
  | "volume_selector"
  | "volume_title_selector"
  | "chapter_title_selector";
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

export function inferSearchPathFromHtml(
  html: string,
  _baseUrl: string,
): { searchPath: string; resultSelector?: string } | null {
  if (!html || typeof html !== "string") return null;
  try {
    const doc = new DOMParser().parseFromString(html, "text/html");

    // 1. Scan for <form> with GET method containing search input
    const forms = doc.querySelectorAll("form");
    for (const form of forms) {
      const method = (form.getAttribute("method") || "get").toLowerCase();
      if (method === "post") continue;

      const inputs = form.querySelectorAll("input");
      let searchParam: string | null = null;
      for (const inp of inputs) {
        const itype = (inp.getAttribute("type") || "text").toLowerCase();
        const name = (inp.getAttribute("name") || "").toLowerCase();
        const idClass = `${inp.id} ${inp.className}`.toLowerCase();

        const isSearchField =
          itype === "search" ||
          [
            "q",
            "s",
            "search",
            "keyword",
            "term",
            "query",
            "key",
            "title",
            "text",
            "name",
            "k",
          ].some(
            (k) =>
              name === k ||
              name.includes(`_${k}`) ||
              name.includes(`${k}_`),
          ) ||
          idClass.includes("search") ||
          idClass.includes("buscar");

        if (isSearchField) {
          searchParam = inp.getAttribute("name") || "q";
          break;
        }
      }

      if (searchParam) {
        const rawAction = (form.getAttribute("action") || "/").trim();
        let action = rawAction;
        if (!action) action = "/";
        else if (action.startsWith("http://") || action.startsWith("https://")) {
          try {
            const u = new URL(action);
            action = u.pathname + (u.search || "");
          } catch {}
        } else if (!action.startsWith("/")) {
          action = `/${action}`;
        }

        const sep = action.includes("?") ? "&" : "?";
        const searchPath = `${action}${sep}${searchParam}={query}`;

        let suggestedSel: string | undefined;
        if (doc.querySelector("a.lc-card-name, a.lc-sr-item")) {
          suggestedSel = "a.lc-card-name, a.lc-sr-item";
        }

        return { searchPath, resultSelector: suggestedSel };
      }
    }

    // 2. Scan for standalone <input type="search"> or search input without <form>
    const searchInputs = doc.querySelectorAll(
      "input[type='search'], input[data-search-input], input[name='q'], input[name='s'], input[name='search'], input[class*='search'], input[id*='search']",
    );
    if (searchInputs.length > 0) {
      const inp = searchInputs[0];
      const param = inp.getAttribute("name") || "q";

      for (const attr of ["data-action", "data-url", "data-search-url", "action"]) {
        const val = inp.getAttribute(attr);
        if (val && val.trim()) {
          const path = val.startsWith("/") ? val : `/${val}`;
          const sep = path.includes("?") ? "&" : "?";
          return {
            searchPath: `${path}${sep}${param}={query}`,
            resultSelector: "a.lc-card-name, a.lc-sr-item",
          };
        }
      }

      // Check navbar links
      const links = doc.querySelectorAll("a[href]");
      for (const a of links) {
        const href = (a.getAttribute("href") || "").toLowerCase();
        if (
          href === "/manga/" ||
          href === "/manga" ||
          href === "/catalogo/" ||
          href === "/catalog/" ||
          href === "/series/"
        ) {
          const raw = a.getAttribute("href")!;
          const clean = raw.endsWith("/") ? raw : `${raw}/`;
          const path = clean.startsWith("/") ? clean : `/${clean}`;
          return {
            searchPath: `${path}?${param}={query}`,
            resultSelector: "a.lc-card-name, a.lc-sr-item",
          };
        }
        if (href.includes("/search") || href.includes("/buscar")) {
          const raw = a.getAttribute("href")!;
          const path = raw.startsWith("/") ? raw : `/${raw}`;
          const sep = path.includes("?") ? "&" : "?";
          return { searchPath: `${path}${sep}${param}={query}` };
        }
      }

      // Check if html contains hints
      if (
        html.includes("/manga/?") ||
        html.includes("/search/?q=") ||
        html.includes("data-live-search")
      ) {
        return {
          searchPath: "/manga/?q={query}",
          resultSelector: "a.lc-card-name, a.lc-sr-item",
        };
      }

      return { searchPath: `/?s={query}` };
    }
  } catch {}
  return null;
}

export const stageFields: Record<string, SelectorField[]> = {
  search: ["result_selector", "next_selector"],
  detail: [
    "chapter_selector",
    "description_selector",
    "author_selector",
    "volume_selector",
    "volume_title_selector",
    "chapter_title_selector",
  ],
  reader: ["image_selector", "reader_next_selector", "chapter_url_suffix"],
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
