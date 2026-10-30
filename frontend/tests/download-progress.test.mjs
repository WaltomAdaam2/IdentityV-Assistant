import assert from "node:assert/strict";
import test from "node:test";
import { sampleDownload, formatSpeed, formatRemaining } from "../src/transferRate.ts";

test("rolling transfer speed and ETA follow real bytes and decay to zero during a stall", () => {
  const samples = [];
  assert.deepEqual(sampleDownload(samples, 0, 10000, 0), {speed: 0, remaining: null});
  assert.deepEqual(sampleDownload(samples, 2000, 10000, 1000), {speed: 2000, remaining: 4});
  assert.deepEqual(sampleDownload(samples, 4000, 10000, 2000), {speed: 2000, remaining: 3});
  for (let now = 2500; now <= 7500; now += 500) sampleDownload(samples, 4000, 10000, now);
  assert.deepEqual(sampleDownload(samples, 4000, 10000, 8000), {speed: 0, remaining: null});
  assert.ok(samples.length <= 12);
});

test("unknown size, retry and completion never produce invalid or negative estimates", () => {
  const samples = [];
  sampleDownload(samples, 0, null, 0);
  assert.deepEqual(sampleDownload(samples, 1000, null, 1000), {speed: 1000, remaining: null});
  assert.deepEqual(sampleDownload(samples, 0, 2000, 1500), {speed: 0, remaining: null});
  assert.deepEqual(sampleDownload(samples, 2100, 2000, 2500), {speed: 2100, remaining: 0});
});

test("rate and time labels cover both languages without Infinity or NaN", () => {
  assert.equal(formatSpeed(0), "0 B/s");
  assert.equal(formatSpeed(2048), "2.0 KB/s");
  assert.equal(formatSpeed(1572864), "1.5 MB/s");
  assert.equal(formatRemaining(null, true), "剩余时间待估算");
  assert.equal(formatRemaining(59, true), "预计剩余 59 秒");
  assert.equal(formatRemaining(61, false), "About 2m remaining");
  assert.equal(formatRemaining(3700, false), "About 1h 2m remaining");
});
