import assert from "node:assert/strict";
import test from "node:test";
import { matchingMaps, defaultJournalSettings } from "../src/journalData.ts";

const packs = [{ id: "base", season_id: "s0", difficulty: "hard", maps: [
  { index: 1, category: "lf", name: "", floors: { 1: "left.png", 2: "upper.png" }, display: {} },
  { index: 1, category: "rf", name: "", floors: { 1: "right.png" }, display: {} },
] }, { id: "route", season_id: "s0", difficulty: "hard", maps: [
  { index: 1, category: "rf", name: "", floors: { 1: "right-base.png" }, display: { 1: "right-route.png" } },
] }];
test("recognition keeps cross-door identities and order while ordinary browsing keeps its door filter", () => {
  assert.deepEqual(matchingMaps(packs, defaultJournalSettings).map(map => map.key), ["lf:1"]);
  assert.deepEqual(matchingMaps(packs, defaultJournalSettings, ["rf:1", "lf:1"]).map(map => map.key), ["rf:1", "lf:1"]);
  assert.deepEqual(matchingMaps(packs, defaultJournalSettings, []), []);
});
test("candidate floor and route views never fall back to an unavailable image", () => {
  const keys = ["lf:1", "rf:1"];
  assert.deepEqual(matchingMaps(packs, { ...defaultJournalSettings, display: "route" }, keys).map(map => map.image.path), ["right-route.png"]);
  assert.deepEqual(matchingMaps(packs, { ...defaultJournalSettings, floor: "2" }, keys).map(map => map.image.path), ["upper.png"]);
  assert.deepEqual(matchingMaps(packs, { ...defaultJournalSettings, season: "s1" }, keys), []);
});
