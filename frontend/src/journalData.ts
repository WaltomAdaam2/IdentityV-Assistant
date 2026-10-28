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
export interface BrowserMap { key: string; index: number; name: string; image: MapImage }

// One card per map number; pure/route packages can describe the same map.
export function matchingMaps(packs: JournalPack[], filters: JournalSettings): BrowserMap[] {
  const maps = new Map<number, BrowserMap>();
  for (const pack of packs) {
    if (pack.season_id !== filters.season || pack.difficulty !== filters.difficulty) continue;
    for (const map of pack.maps) {
      if (map.category !== filters.door) continue;
      const layers = filters.display === "route" ? map.display : map.floors;
      const path = layers[filters.floor] ?? (filters.floor === "b1f" ? layers["3"] : undefined);
      if (!path || maps.has(map.index)) continue;
      maps.set(map.index, { key: `${pack.id}:${map.index}`, index: map.index, name: map.name,
        image: { packId: pack.id, path } });
    }
  }
  return [...maps.values()].sort((a, b) => a.index - b.index);
}
export const journalApi = {
  list: () => invoke<JournalLibrary>("journal_list_packs"),
  choose: (kind: "folder" | "zip") => invoke<string | null>("journal_choose_pack_source", { kind }),
  import: (sourcePath: string) => invoke<JournalLibrary>("journal_import_pack", { sourcePath }),
  image: (image: MapImage) => invoke<string>("journal_read_image", { packId: image.packId, path: image.path }),
  openDirectory: () => invoke<void>("journal_open_directory"),
};
