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

test("launcher displays the located game and persists a manually selected executable", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("find", "role", "button", "click", "--name", "游戏启动", "--exact");
    browser("wait", "--text", "活动与公告");
    assert.match(browser("eval", "document.querySelector('.launcher-actions').textContent"), /C:.*dwrg\.exe/);
    browser("find", "role", "button", "click", "--name", "启动设置", "--exact");
    browser("find", "role", "button", "click", "--name", "选择游戏程序", "--exact");
    browser("wait", "--text", "D:\\Games\\IdentityV\\dwrg.exe");
    browser("wait", "--fn", "window.__launcherTest.calls.some(call => call.command === 'save_settings' && call.args.settings.game_window.executable_path === 'D:\\\\Games\\\\IdentityV\\\\dwrg.exe')");
    browser("eval", "window.__launcherTest.selectedGamePath = 'D:\\\\Games\\\\Fever.exe'");
    browser("find", "role", "button", "click", "--name", "选择游戏程序", "--exact");
    browser("wait", "--text", "不能选择发烧平台");
    browser("eval", "window.__launcherTest.selectedGamePath = 'D:\\\\Games\\\\IdentityV\\\\dwrg.exe'");
    browser("find", "role", "button", "click", "--name", "选择游戏程序", "--exact");
    assert.equal(JSON.parse(browser("eval", "Boolean(document.querySelector('.launcher-game-error'))")), false);
    browser("eval", "window.__launcherTest.locationFailure = true");
    browser("find", "role", "button", "click", "--name", "任务", "--exact");
    browser("wait", "--fn", "!document.querySelector('.game-launcher')");
    browser("find", "role", "button", "click", "--name", "游戏启动", "--exact");
    browser("wait", "--text", "未找到第五人格");
    browser("eval", "window.__launcherTest.locationFailure = false");
    browser("find", "role", "button", "click", "--name", "启动设置", "--exact");
    browser("find", "role", "button", "click", "--name", "选择游戏程序", "--exact");
    assert.match(browser("eval", "document.querySelector('.launcher-game-path').textContent"), /D:.*dwrg\.exe/);
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

test("launch waits for the component and exposes success and recoverable failure", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("find", "role", "button", "click", "--name", "游戏启动", "--exact");
    browser("wait", "--text", "活动与公告");
    assert.equal(JSON.parse(browser("eval", "document.querySelector('.launcher-start').disabled")), false);
    browser("find", "role", "button", "click", "--name", "启动游戏", "--exact");
    browser("wait", "--text", "正在启动");
    assert.equal(JSON.parse(browser("eval", "document.querySelector('.launcher-start').disabled")), true);
    browser("eval", "window.__launcherTest.finishLaunch()");
    browser("wait", "--text", "登录代理已就绪");
    browser("wait", "--fn", "!document.querySelector('.launcher-start').disabled");
    browser("find", "role", "button", "click", "--name", "启动游戏", "--exact");
    browser("eval", "window.__launcherTest.finishLaunch('登录组件退出，请重试')");
    browser("wait", "--text", "登录组件退出，请重试");
    assert.equal(JSON.parse(browser("eval", "document.querySelector('.launcher-start').disabled")), false);
  } finally { browser("close"); }
});

test("missing login component opens setup and accepts an installed component", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("eval", "window.__launcherTest.launcherStatus.component_path = null");
    browser("find", "role", "button", "click", "--name", "游戏启动", "--exact");
    browser("wait", "--text", "请先配置官方登录组件");
    browser("find", "role", "button", "click", "--name", "启动游戏", "--exact");
    browser("wait", "--text", "首次使用需本人登录");
    assert.equal(JSON.parse(browser("eval", "window.__launcherTest.calls.some(call => call.command === 'launch_game')")), false);
    browser("find", "role", "button", "click", "--name", "官方下载", "--exact");
    assert.equal(JSON.parse(browser("eval", "window.__launcherTest.calls.some(call => call.command === 'open_login_component_download')")), true);
    browser("find", "role", "button", "click", "--name", "选择登录组件", "--exact");
    browser("wait", "--text", "D:\\idv-login\\python-embed\\python.exe");
    assert.equal(JSON.parse(browser("eval", "window.__launcherTest.calls.some(call => call.command === 'set_login_component' && call.args.executablePath === 'D:\\\\idv-login\\\\python-embed\\\\python.exe')")), true);
    browser("find", "role", "button", "click", "--name", "关闭启动设置", "--exact");
    assert.equal(JSON.parse(browser("eval", "Boolean(document.querySelector('.launcher-setup'))")), false);
  } finally { browser("close"); }
});
