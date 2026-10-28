// Run: node frontend/check-journal.mjs
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = (await readFile(new URL("./src/journalData.ts", import.meta.url), "utf8"))
  .replace('import { invoke } from "@tauri-apps/api/core";', 'const invoke = () => { throw new Error("Unexpected backend invocation"); };');
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ES2022, target: ts.ScriptTarget.ES2022 } });
const { matchingMaps, importJournalPacks, defaultJournalSettings: defaults } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const pack = (id, maps, extra = {}) => ({ id, name: id, version: "1", season_id: "s0", season_name: "S0", difficulty: "hard", maps, ...extra });
const map = (index, floors = {}, display = {}, category = "lf") => ({ index, name: "", category, floors, display });
const packs = [pack("base", [map(2, { "1": "two.png" }), map(1, { "1": "one.png", "2": "up.png" })]), pack("route", [map(1, {}, { "1": "route.png" })])];
assert.deepEqual(matchingMaps(packs, defaults).map((m) => m.index), [1, 2]);
assert.equal(matchingMaps(packs, { ...defaults, display: "route" }).length, 1);
assert.deepEqual(matchingMaps(packs, { ...defaults, display: "route" })[0].image, { packId: "route", path: "route.png" });
assert.equal(matchingMaps(packs, { ...defaults, floor: "2" }).length, 1);
assert.equal(matchingMaps(packs, { ...defaults, floor: "2", display: "route" }).length, 0);
for (const filter of [{ season: "s1" }, { difficulty: "nightmare" }, { door: "rf" }]) assert.equal(matchingMaps(packs, { ...defaults, ...filter }).length, 0);
assert.equal(matchingMaps([...packs, pack("duplicate", [map(1, { "1": "duplicate.png" })])], defaults).length, 2);
const basement = [pack("basement", [map(1, { "3": "basement.png" })], { difficulty: "nightmare" })];
assert.equal(matchingMaps(basement, { ...defaults, difficulty: "nightmare", floor: "b1f" })[0].image.path, "basement.png");
assert.equal(matchingMaps([], defaults).length, 0);
console.log("Journal filter checks passed: split packages, sorted maps, exact counts, missing layers, aliases, and filters.");
const importedSources = [];
const library = { directory: "local", packs, errors: [] };
const batch = await importJournalPacks(["folder", "broken.zip", "route.zip", "folder"], async (source) => {
  importedSources.push(source);
  if (source === "broken.zip") throw new Error("Invalid ZIP");
  return library;
});
assert.deepEqual(importedSources, ["folder", "broken.zip", "route.zip"]);
assert.equal(batch.imported, 2);
assert.equal(batch.library, library);
assert.deepEqual(batch.errors, ["broken.zip: Error: Invalid ZIP"]);
assert.deepEqual(await importJournalPacks([]), { library: null, imported: 0, errors: [] });
console.log("Journal batch import checks passed: mixed sources, deduplication, partial failure and empty selection.");
