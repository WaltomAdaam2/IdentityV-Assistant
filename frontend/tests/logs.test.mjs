import assert from "node:assert/strict";
import test from "node:test";
import { appendLogBatch, appendNormalLogBatch, filterLogs, isWindowDiagnostic, progressLog } from "../src/logs.ts";

const entry = (id, patch = {}) => ({ id, at: "12:00:00", level: "info", message: "准备匹配", source: "hide_seek", important: true, ...patch });

test("batch append preserves order and newest 99,000 entries", () => {
  const current = Array.from({ length: 99_000 }, (_, index) => entry(99_000 - index));
  const pending = [entry(99_001), entry(99_002)];
  const merged = appendLogBatch(current, pending);
  assert.equal(merged.length, 99_000);
  assert.deepEqual(merged.slice(0, 3).map((value) => value.id), [99_002, 99_001, 99_000]);
  assert.equal(merged.at(-1).id, 3);
  assert.equal(current[0].id, 99_000);
  assert.deepEqual(pending.map((value) => value.id), [99_001, 99_002]);
});

test("normal logs hide window diagnostics even when marked important or warning", () => {
  const ready = { type: "window_health", source: "window", level: "info", message: "Game window is ready" };
  const unresponsive = { ...ready, level: "warning", message: "Game window is not responding" };
  const entries = [entry(1), entry(2, { ...ready, detailedOnly: isWindowDiagnostic(ready) }), entry(3, { ...unresponsive, detailedOnly: isWindowDiagnostic(unresponsive) }), entry(4, { level: "error", message: "任务执行失败" })];
  assert.deepEqual(filterLogs(entries, "all", "", false).map((value) => value.id), [1, 4]);
  assert.equal(filterLogs(entries, "all", "", true).length, 4);
  assert.deepEqual(filterLogs(entries, "warning", "NOT RESPONDING", true).map((value) => value.id), [3]);
});

test("normal cache evicts at the full history boundary even for telemetry-only batches", () => {
  const normal = [entry(2), entry(1)];
  const telemetry = Array.from({ length: 98_998 }, (_, index) => entry(index + 3, { important: false }));
  const full = appendLogBatch(normal, telemetry);
  const cached = appendNormalLogBatch(normal, telemetry);
  assert.strictEqual(cached, normal);
  const next = [entry(99_001, { important: false })];
  const updated = appendNormalLogBatch(cached, next);
  assert.deepEqual(updated.map((value) => value.id), [2]);
  assert.deepEqual(updated, filterLogs(appendLogBatch(full, next), "all", "", false));
  assert.strictEqual(appendNormalLogBatch(updated, []), updated);
  const postClear = [entry(150_000)];
  assert.strictEqual(appendNormalLogBatch(postClear, [entry(150_001, { important: false })]), postClear);
});

test("a batch larger than retention cannot leave discarded normal entries cached", () => {
  const pending = Array.from({ length: 99_002 }, (_, index) => entry(index + 1, { important: index === 0 || index === 99_001 }));
  assert.deepEqual(appendNormalLogBatch([], pending), filterLogs(appendLogBatch([], pending), "all", "", false));
});

test("actual window errors remain visible in normal logs", () => {
  const events = [
    { type: "log", source: "window", level: "error", message: "Could not apply game window style" },
    { type: "log", source: "window", level: "error", message: "Could not restore game window style" },
    { type: "window_diagnostics", source: "window", level: "error", message: "Window diagnostics failed" },
    { type: "error", source: "window", message: "Window operation failed" },
  ];
  assert.ok(events.every((event) => !isWindowDiagnostic(event)));
  const entries = events.map((event, index) => entry(index, { ...event, level: "error", detailedOnly: isWindowDiagnostic(event) }));
  assert.equal(filterLogs(entries, "all", "", false).length, events.length);
});

test("stage messages are emitted once per transition, and again for later cycles", () => {
  const previous = new Map();
  const stages = { start_game: "准备匹配", v_fixed: "成功固定", living_no_v: "固定消失", continue: "进入结算画面" };
  const event = { type: "task_progress", task_id: "hide_seek", completed_cycles: 0 };
  assert.equal(progressLog({ ...event, stage: "start_game" }, stages, previous), "准备匹配");
  assert.equal(progressLog({ ...event, stage: "start_game" }, stages, previous), null);
  assert.equal(progressLog({ ...event, stage: "v_fixed" }, stages, previous), "成功固定");
  assert.equal(progressLog({ ...event, stage: "living_no_v" }, stages, previous), "固定消失");
  assert.equal(progressLog({ ...event, stage: "continue" }, stages, previous), "进入结算画面");
  assert.equal(progressLog({ ...event, stage: "continue", completed_cycles: 1 }, stages, previous), "进入结算画面");
  assert.equal(progressLog({ ...event, type: "recognition_evidence", stage: "v_fixed" }, stages, previous), null);
  previous.delete("hide_seek");
  assert.equal(progressLog({ ...event, stage: "continue" }, stages, previous), "进入结算画面");
});
