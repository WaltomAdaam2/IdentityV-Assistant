import assert from "node:assert/strict";
import test from "node:test";
import { messageLanguage } from "../src/messageLanguage.ts";

test("presentation selects the configured language while preserving identifiers and raw records", () => {
  const raw = "正在匹配地图 / Matching maps\n助手更新 / Assistant update: checking";
  assert.equal(messageLanguage(raw, "zh-CN"), "正在匹配地图\n助手更新：检查中");
  assert.equal(messageLanguage(raw, "en-US"), "Matching maps\nAssistant update: checking");
  assert.equal(raw, "正在匹配地图 / Matching maps\n助手更新 / Assistant update: checking");
  for (const value of ["C:\\Users\\测试\\maps.png", "https://example.invalid/maps", "2 / 29", "rf:3: image could not be decoded", "请扫码登录", "", "path / another-path"]) {
    assert.equal(messageLanguage(value, "zh-CN"), value);
    assert.equal(messageLanguage(value, "en-US"), value);
  }
  assert.equal(messageLanguage("识图失败 / Recognition failed: request-42", "zh-CN"), "识图失败：request-42");
  assert.equal(messageLanguage("设置失败：无效设置 / Invalid setting: settings.journal.s1_display", "zh-CN"), "设置失败：无效设置：settings.journal.s1_display");
  assert.equal(messageLanguage("无效设置：settings.journal.s1_display / Invalid setting: settings.journal.s1_display", "zh-CN"), "无效设置：settings.journal.s1_display");
});
