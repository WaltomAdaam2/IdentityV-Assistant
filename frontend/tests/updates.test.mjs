import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";

const binary = process.env.AGENT_BROWSER_BIN || "agent-browser";
const origin = process.env.LAUNCHER_TEST_ORIGIN || "http://127.0.0.1:1420";
let sequence = 0, session;
function browser(...args) {
  if (args[0] === "open") session = `idv-update-test-${process.pid}-${++sequence}`;
  const result = spawnSync(binary, ["--session", session, ...args], { encoding: "utf8", timeout: 30000 });
  assert.equal(result.status, 0, result.error?.message || result.stderr || result.stdout);
  return result.stdout.trim();
}
const evaluate = (code) => JSON.parse(browser("eval", code));
function openSettings() {
  browser("open", `${origin}/tests/launcher-harness.html?desktop`);
  browser("find", "role", "button", "click", "--name", "设置", "--exact");
  browser("wait", "--fn", "document.querySelector('.update-panel button')?.disabled === false");
}

test("update notes render safe Markdown and the source defaults to direct without probes", () => {
  try {
    openSettings();
    assert.deepEqual(evaluate("[...document.querySelector('.update-panel select').options].map(o => [o.value,o.selected])"), [["direct",true],["proxy",false],["ghfast",false],["ghproxy_net",false]]);
    assert.ok(evaluate("document.querySelector('.update-panel select').selectedOptions[0].textContent").includes("GitHub 直连（默认）"));
    const notes = '# Version\n\n## Changes\n\n- **Clear** progress\n- `Safe` code\n\n[Release](https://github.com/WaltomAdaam2/IdentityV-Assistant/releases)\n\n```text\nchecksum\n```\n\n<script>window.__injected = true</script>\n\n[Bad](javascript:alert(1))\n\n![pixel](https://example.invalid/pixel.png)';
    evaluate(`window.__launcherTest.releaseNotes = ${JSON.stringify(notes)}`);
    browser("find", "role", "button", "click", "--name", "手动检查", "--exact");
    browser("wait", "--fn", "document.querySelector('.release-markdown h1')?.textContent === 'Version'");
    assert.deepEqual(evaluate(`(() => {const n=document.querySelector('.release-markdown');return {heading:n.querySelector('h2').textContent,items:n.querySelectorAll('li').length,bold:n.querySelector('strong').textContent,code:n.querySelector('pre code').textContent.trim(),links:n.querySelectorAll('a').length,unsafe:!!n.querySelector('script,img,[href^="javascript:"]')||!!window.__injected,scroll:getComputedStyle(n).overflowY};})()`), {heading:"Changes",items:2,bold:"Clear",code:"checksum",links:1,unsafe:false,scroll:"auto"});
    assert.equal(evaluate("window.__launcherTest.calls.find(c=>c.command==='check_update').args.source"), "direct");
    browser("find", "role", "link", "click", "--name", "Release", "--exact");
    assert.equal(evaluate("window.__launcherTest.calls.find(c=>c.command==='open_release_link').args.url"), "https://github.com/WaltomAdaam2/IdentityV-Assistant/releases");
  } finally { browser("close"); }
});

test("download speed and remaining time sit beside progress, stall safely and reset on retry", () => {
  try {
    openSettings();
    browser("find", "role", "button", "click", "--name", "手动检查", "--exact");
    browser("wait", "--text", "发现新版本");
    browser("find", "role", "button", "click", "--name", "下载更新 · 9.5 MB", "--exact");
    browser("wait", "--fn", "Boolean(document.querySelector('.update-transfer'))");
    evaluate("window.__launcherTest.emitUpdate({downloaded: 2000000})");
    browser("wait", "--fn", "document.querySelector('.update-transfer').textContent.includes('预计剩余')");
    assert.equal(evaluate("document.querySelector('.update-progress-row progress').value"), 20);
    assert.equal(evaluate("document.querySelector('.update-transfer').getBoundingClientRect().left >= document.querySelector('.update-progress-row progress').getBoundingClientRect().right"), true);
    browser("set", "viewport", "520", "760");
    assert.equal(evaluate("document.querySelector('.update-dialog').scrollWidth <= document.querySelector('.update-dialog').clientWidth"), true);
    browser("wait", "--fn", "document.querySelector('.update-transfer').textContent.includes('0 B/s剩余时间待估算')");
    browser("find", "role", "button", "click", "--name", "取消下载", "--exact");
    browser("wait", "--text", "下载已取消");
    assert.equal(evaluate("Boolean(document.querySelector('.update-transfer'))"), false);
    browser("find", "role", "button", "click", "--name", "下载更新 · 9.5 MB", "--exact");
    browser("wait", "--fn", "document.querySelector('.update-transfer')?.textContent.includes('0 B/s剩余时间待估算')");
    assert.equal(evaluate("document.querySelector('.update-progress-row progress').value"), 0);
    evaluate("window.__launcherTest.emitUpdate({total: null, downloaded: 1000000})");
    browser("wait", "--fn", "document.querySelector('.update-progress-row progress')?.hasAttribute('value') === false");
    assert.ok(evaluate("document.querySelector('.update-transfer').textContent").includes("剩余时间待估算"));
  } finally { browser("close"); }
});
