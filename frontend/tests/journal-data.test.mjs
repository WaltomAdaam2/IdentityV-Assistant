import assert from "node:assert/strict";
import test from "node:test";
import { matchingMaps, defaultJournalSettings, previewPlacement } from "../src/journalData.ts";

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

test("S1 ignores S0 door and route preferences and keeps B3 distinct from S0 basement", () => {
  const seasonal = [{ id: "s1_hard", season_id: "s1", difficulty: "hard", maps: [
    { index: 1, category: "td", name: "", floors: { 1: "b1.png", 2: "b2.png", 3: "b3.png" }, display: {} },
  ] }];
  const filters = { ...defaultJournalSettings, season: "s1", display: "route" };
  assert.deepEqual(matchingMaps(seasonal, filters).map(map => map.image.path), ["b1.png"]);
  assert.deepEqual(matchingMaps(seasonal, { ...filters, floor: "3" }).map(map => map.image.path), ["b3.png"]);
  assert.deepEqual(matchingMaps(seasonal, { ...filters, floor: "b1f" }), []);
  assert.deepEqual(matchingMaps(seasonal, { ...filters, difficulty: "nightmare" }), []);
});

test("recognition shows only the first four ranked identities without limiting normal browsing", () => {
  const many = [{ ...packs[0], maps: Array.from({ length: 6 }, (_, i) => ({ ...packs[0].maps[0], index: i + 1 })) }];
  const ranked = ["lf:6", "lf:2", "lf:5", "lf:1", "lf:4", "lf:3"];
  assert.deepEqual(matchingMaps(many, defaultJournalSettings, ranked).map(map => map.key), ranked.slice(0, 4));
  assert.equal(matchingMaps(many, defaultJournalSettings).length, 6);
  assert.equal(ranked.length, 6, "full results remain available to diagnostics");
});

test("map placement keeps shared relative center and clamps different shapes to safe edges", () => {
  const view = { width: 1000, height: 680 };
  const centered = previewPlacement(view, 1, { x: 0.5, y: 0.5 });
  assert.equal(centered.x + centered.width / 2, 500);
  assert.equal(centered.y + centered.height / 2, 340);
  for (const aspect of [0.1, 0.5, 1, 2, 10]) {
    for (const position of [{ x: 0, y: 0 }, { x: 1, y: 1 }, { x: 0.8, y: 0.3 }]) {
      const placed = previewPlacement(view, aspect, position);
      assert.ok(placed.x >= 16 && placed.y >= 16);
      assert.ok(placed.x + placed.width <= view.width - 16);
      assert.ok(placed.y + placed.height <= view.height - 16);
      assert.ok(Math.abs(placed.width / placed.height - aspect) < 1e-10);
    }
  }
  const position = { x: 0.65, y: 0.5 };
  for (const size of [view, { width: 1600, height: 900 }]) {
    const placed = previewPlacement(size, 0.5, position);
    assert.equal((placed.x + placed.width / 2) / size.width, position.x);
  }
});
