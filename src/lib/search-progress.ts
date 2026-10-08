import type { Manga } from "../types";
/** Replace one source snapshot, keeping results from every other source. */
export function mergeSourceResults(
  current: Manga[],
  sourceId: string,
  incoming: Manga[],
): Manga[] {
  const unique = new Map<string, Manga>();
  for (const manga of incoming)
    unique.set(manga.url, { ...manga, source_id: sourceId });
  return [
    ...current.filter((manga) => manga.source_id !== sourceId),
    ...unique.values(),
  ];
}
