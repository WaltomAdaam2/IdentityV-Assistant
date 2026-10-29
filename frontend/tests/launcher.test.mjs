import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";

// Run with the Vite dev server and agent-browser installed. Only the native
// desktop interface is replaced; navigation uses the real rendered app.
const binary = process.env.AGENT_BROWSER_BIN || "agent-browser";
let session;
let sequence = 0;
const origin = process.env.LAUNCHER_TEST_ORIGIN || "http://127.0.0.1:1420";
function browser(...args) {
  if (args[0] === "open") session = `idv-launcher-test-${process.pid}-${++sequence}`;
  const result = spawnSync(binary, ["--session", session, ...args], { encoding: "utf8", timeout: 30000 });
  assert.equal(result.status, 0, result.error?.message || result.stderr || result.stdout);
  return result.stdout.trim();
}

test("game launch is placed between tasks and journal", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("wait", "--text", "加页手记");
    const output = browser("eval", "JSON.stringify(Array.from(document.querySelectorAll('.sidebar nav button'), node => node.textContent.trim()))");
    const labels = JSON.parse(JSON.parse(output));
    const tasks = labels.indexOf("任务");
    assert.ok(tasks >= 0);
    assert.deepEqual(labels.slice(tasks, tasks + 3), ["任务", "游戏启动", "加页手记"]);
  } finally {
    browser("close");
  }
});

test("announcements show a retry action when loading fails", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("find", "role", "button", "click", "--name", "游戏启动", "--exact");
    browser("wait", "--text", "活动与公告");
    assert.match(browser("eval", "document.querySelector('.launcher-news').textContent"), /公告暂时无法加载/);
    assert.match(browser("snapshot", "-i"), /重试/);
    browser("eval", "window.__launcherTest.newsFailure = false");
    browser("find", "role", "button", "click", "--name", "重试", "--exact");
    browser("wait", "--text", "庄园活动公告");
    browser("find", "role", "button", "click", "--name", "庄园活动公告", "--exact");
    const output = browser("eval", "JSON.stringify(window.__launcherTest.calls.filter(call => call.command === 'open_game_announcement'))");
    assert.deepEqual(JSON.parse(JSON.parse(output)), [{ command: "open_game_announcement", args: { url: "https://id5.163.com/news/official/test.html" } }]);
  } finally {
    browser("close");
  }
});
