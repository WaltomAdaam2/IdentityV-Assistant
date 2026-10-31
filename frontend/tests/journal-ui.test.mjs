import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";

const binary = process.env.AGENT_BROWSER_BIN || "agent-browser";
const origin = process.env.LAUNCHER_TEST_ORIGIN || "http://127.0.0.1:1420";
let session, sequence = 0;
function browser(...args) {
  if (args[0] === "open") session = `idv-journal-${process.pid}-${++sequence}`;
  const result = spawnSync(binary, ["--session", session, ...args], { encoding: "utf8", timeout: 30000 });
  assert.equal(result.status, 0, result.error?.message || result.stderr || result.stdout);
  return result.stdout.trim();
}
const evaluate = code => JSON.parse(browser("eval", code));
const click = name => browser("find", "role", "button", "click", "--name", name, "--exact");
const wait = code => browser("wait", "--fn", code);
function setup() {
  browser("open", `${origin}/tests/launcher-harness.html?desktop`);
  evaluate(`window.__launcherTest.packs=[{id:'base',season_id:'s0',difficulty:'hard',maps:[
    {index:1,category:'lf',name:'',floors:{1:'left.png'},display:{}},
    {index:1,category:'rf',name:'',floors:{1:'right.png',2:'right-upper.png'},display:{}}]}]`);
  click("加页手记");
  wait("!!document.querySelector('.journal') && !document.querySelector('.journal-viewer[aria-busy=true]')");
}
test("recognition updates existing grid, keeps same-number doors distinct, and opens only on click", () => {
  try {
    setup(); click("识别一楼地图");
    wait("!!window.__launcherTest.finishRecognition");
    evaluate("window.__launcherTest.emitRecognitionProgress({request_id:window.__launcherTest.recognitionRequest.requestId,stage:'matching',completed:2,total:8})");
    browser("wait", "--text", "正在匹配地图… 2 / 8");
    evaluate("window.__launcherTest.finishRecognition({keys:['rf:1','lf:1'],preview:'data:image/png;base64,AA==',warnings:[]})");
    wait("document.querySelectorAll('.journal-map').length===2");
    assert.deepEqual(evaluate("Array.from(document.querySelectorAll('.journal-map-name'),e=>e.textContent)"), ["右前 · 地图 1", "左前 · 地图 1"]);
    assert.equal(evaluate("!!document.querySelector('.journal-detail')"), false);
    assert.equal(evaluate("document.querySelector('.journal-region-preview').open"), false);
    click("查看右前 · 地图 1"); click("2F");
    wait("window.__launcherTest.calls.some(c=>c.command==='journal_read_image' && c.args.path==='right-upper.png')");
    assert.equal(evaluate("!!document.querySelector('.journal-detail')"), true);
    click("‹ 返回列表"); click("一楼"); click("返回全部地图");
    wait("document.querySelectorAll('.journal-map').length===1");
    assert.equal(evaluate("document.querySelector('.journal-map-name').textContent"), "地图 1");
    click("查看地图 1");
    wait("!!document.querySelector('.journal-detail img')"); click("进入地图");
    wait("!!document.querySelector('.journal-full-preview:modal')");
    evaluate("document.hasFocus=()=>false; window.__launcherTest.recognitionShortcut()");
    wait("window.__launcherTest.calls.filter(c=>c.command==='journal_recognize').length===2");
    wait("!document.querySelector('.journal-full-preview')");
    assert.equal(evaluate("window.__launcherTest.recognitionRequest.windowHandle"), 123);
  } finally { browser("close"); }
});
test("filter changes and page leave cancel recognition and discard late results", () => {
  try {
    setup(); click("识别一楼地图"); wait("!!window.__launcherTest.finishRecognition");
    evaluate("window.__lateRecognition=window.__launcherTest.finishRecognition");
    click("噩梦");
    wait("window.__launcherTest.calls.some(c=>c.command==='journal_cancel_recognition')");
    evaluate("window.__lateRecognition({keys:['rf:1'],preview:null,warnings:[]})");
    assert.equal(evaluate("!!document.querySelector('.journal-recognition')"), false);
    click("困难"); click("识别一楼地图");
    wait("window.__launcherTest.calls.filter(c=>c.command==='journal_recognize').length===2");
    click("概览");
    wait("window.__launcherTest.calls.filter(c=>c.command==='journal_cancel_recognition').length===2");
    evaluate("window.__launcherTest.finishRecognition({keys:['rf:1'],preview:null,warnings:[]})");
    click("加页手记"); wait("!!document.querySelector('.journal')");
    assert.equal(evaluate("!!document.querySelector('.journal-recognition')"), false);
  } finally { browser("close"); }
});
test("background shortcut requests the original game window and never calls native focus actions", () => {
  try {
    setup(); click("设置"); wait("!!document.querySelector('.settings-form')");
    evaluate("document.hasFocus=()=>false; window.__launcherTest.calls=[]; window.__launcherTest.recognitionShortcut()");
    wait("!!window.__launcherTest.recognitionRequest");
    assert.equal(evaluate("window.__launcherTest.recognitionRequest.windowHandle"), 123);
    assert.equal(evaluate("document.activeElement.matches('.topbar h1')"), false);
    assert.deepEqual(evaluate("window.__launcherTest.calls.filter(c=>/show|unminimize|set_focus/.test(c.command))"), []);
    evaluate("window.__launcherTest.finishRecognition({keys:[],preview:null,warnings:[]})");
    browser("wait", "--text", "未找到匹配地图，请在一楼打开地图后重试。");
    assert.equal(evaluate("document.querySelectorAll('.journal-map').length"), 0);
  } finally { browser("close"); }
});
