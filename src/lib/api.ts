import { invoke, isTauri } from "@tauri-apps/api/core";
export async function api<T>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (!isTauri())
    throw new Error(
      "Apri l’app desktop per collegarti alle fonti e salvare le modifiche.",
    );
  return invoke<T>(command, args);
}
export const message = (e: unknown) =>
  e instanceof Error ? e.message : String(e);
