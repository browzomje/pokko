import type { Volume } from "./types";

export const unknownVolume = (volume: Volume) =>
  ["Capitoli", "Capitoli senza volume"].includes(volume.title);

/** Preserve source order and real volume groups; expose ungrouped chapters directly. */
export function catalogElements(volumes: Volume[]): Volume[] {
  const elements = volumes.flatMap((volume) =>
    unknownVolume(volume)
      ? volume.chapters.map((chapter) => ({
          title: chapter.title,
          chapters: [chapter],
        }))
      : [volume],
  );
  const counts = new Map<string, number>();
  for (const element of elements)
    counts.set(element.title, (counts.get(element.title) || 0) + 1);
  return elements.map((element, index) =>
    counts.get(element.title)! > 1
      ? { ...element, title: `${index + 1} - ${element.title}` }
      : element,
  );
}
function selectionId(volumes: Volume[]): string {
  let hash = 2166136261;
  for (const char of JSON.stringify(
    volumes.map((v) => [v.title, v.chapters.map((c) => c.url)]),
  )) {
    hash = Math.imul(hash ^ char.charCodeAt(0), 16777619);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}
/** Each returned group is exactly one queue job and one exported file. */
export function planDownload(volumes: Volume[], mode: string): Volume[] {
  if (!volumes.length) return [];
  if (mode === "combined" && volumes.length > 1)
    return [
      {
        title: `Selezione ${selectionId(volumes)} - ${volumes[0].title} - ${volumes[volumes.length - 1].title}`,
        chapters: volumes.flatMap((v) => v.chapters),
      },
    ];
  if (mode === "chapters")
    return volumes.flatMap((volume) =>
      volume.chapters.map((chapter, index) => ({
        title:
          volume.chapters.length === 1
            ? volume.title
            : `${volume.title} - ${index + 1} - ${chapter.title}`,
        chapters: [chapter],
      })),
    );
  return volumes;
}
