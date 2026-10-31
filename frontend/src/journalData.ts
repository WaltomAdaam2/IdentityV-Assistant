import { invoke } from "@tauri-apps/api/core";

export interface JournalSettings {
  season: string;
  difficulty: string;
  door: string;
  floor: string;
  display: string;
  collapsed: boolean;
}
export const defaultJournalSettings: JournalSettings = {
  season: "s0", difficulty: "hard", door: "lf", floor: "1", display: "pure", collapsed: false,
};
export interface JournalMap { index: number; category: string; name: string; floors: Record<string, string>; display: Record<string, string> }
export interface JournalPack { id: string; name: string; version: string; season_id: string; season_name: string; difficulty: string; maps: JournalMap[] }
export interface JournalLibrary { directory: string; packs: JournalPack[]; errors: string[] }
export interface MapImage { packId: string; path: string }
export interface BrowserMap { key: string; index: number; category: string; name: string; image: MapImage }

// One card per map number; pure/route packages can describe the same map.
export function matchingMaps(packs: JournalPack[], filters: JournalSettings, recognizedKeys?: string[]): BrowserMap[] {
  const maps = new Map<string, BrowserMap>();
  const order = recognizedKeys ? new Map(recognizedKeys.map((key, index) => [key, index])) : null;
  for (const pack of packs) {
    if (pack.season_id !== filters.season || pack.difficulty !== filters.difficulty) continue;
    for (const map of pack.maps) {
      const key = `${map.category}:${map.index}`;
      if (order ? !order.has(key) : map.category !== filters.door) continue;
      const layers = filters.display === "route" ? map.display : map.floors;
      const path = layers[filters.floor] ?? (filters.floor === "b1f" ? layers["3"] : undefined);
      if (!path || maps.has(key)) continue;
      maps.set(key, { key, index: map.index, category: map.category, name: map.name,
        image: { packId: pack.id, path } });
    }
  }
  return [...maps.values()].sort((a, b) => order ? order.get(a.key)! - order.get(b.key)! : a.index - b.index);
}
export interface RecognitionResult { keys: string[]; preview: string | null; warnings: string[] }
export interface RecognitionProgress { request_id: string; stage: "capturing" | "locating" | "matching"; completed: number; total: number }
export const journalApi = {
  list: () => invoke<JournalLibrary>("journal_list_packs"),
  choose: () => invoke<string[] | null>("journal_choose_pack_source"),
  import: (sourcePath: string) => invoke<JournalLibrary>("journal_import_pack", { sourcePath }),
  image: (image: MapImage) => invoke<string>("journal_read_image", { packId: image.packId, path: image.path }),
  openDirectory: () => invoke<void>("journal_open_directory"),
  recognize: (requestId: string, filters: JournalSettings, windowHandle?: number) => invoke<RecognitionResult>("journal_recognize", {
    requestId, season: filters.season, difficulty: filters.difficulty, display: filters.display, windowHandle,
  }),
  cancel: (requestId: string) => invoke<void>("journal_cancel_recognition", { requestId }),
};

export async function importJournalPacks(sources: string[], importPack = journalApi.import) {
  let library: JournalLibrary | null = null;
  let imported = 0;
  const errors: string[] = [];
  for (const source of new Set(sources)) {
    try { library = await importPack(source); imported++; }
    catch (reason) { errors.push(`${source}: ${String(reason)}`); }
  }
  return { library, imported, errors };
}
