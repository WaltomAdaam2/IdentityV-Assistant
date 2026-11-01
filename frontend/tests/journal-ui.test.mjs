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
    evaluate(`window.__launcherTest.emitRecognitionPreview({request_id:window.__launcherTest.recognitionRequest.requestId,preview:'data:image/svg+xml,'+encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="200" height="600"><rect width="200" height="600" fill="tan"/></svg>')})`);
    wait("!!document.querySelector('.journal-matching-preview img')?.complete");
    assert.equal(evaluate("(()=>{const p=document.querySelector('.journal-matching-preview').getBoundingClientRect(),i=document.querySelector('.journal-matching-preview img').getBoundingClientRect();return i.top>=p.top && i.bottom<=p.bottom && i.left>=p.left && i.right<=p.right && Math.abs((i.left+i.right)-(p.left+p.right))<2})()"),true);
    evaluate("window.__launcherTest.finishRecognition({keys:['rf:1','lf:1'],preview:'data:image/png;base64,AA==',warnings:[]})");
    wait("document.querySelectorAll('.journal-map').length===2");
    assert.deepEqual(evaluate("Array.from(document.querySelectorAll('.journal-map-name'),e=>e.textContent)"), ["右前 · 地图 1", "左前 · 地图 1"]);
    assert.equal(evaluate("!!document.querySelector('.journal-detail')"), false);
    assert.equal(evaluate("document.querySelector('.journal-region-preview').open"), false);
    click("反馈并保存至诊断包");
    browser("find", "role", "radio", "check", "--name", "不正确", "--exact");
    browser("select", ".journal-feedback-map select", "lf:1"); click("保存反馈");
    wait("!document.querySelector('#journal-feedback-title')");
    assert.deepEqual(evaluate("window.__launcherTest.calls.findLast(c=>c.command==='journal_save_feedback').args"),{requestId:evaluate("window.__launcherTest.recognitionRequest.requestId"),verdict:"incorrect",correctKey:"lf:1"});
    click("反馈并保存至诊断包");
    assert.equal(evaluate("document.querySelector('input[value=incorrect]').checked"),true); click("关闭反馈");
    wait("!document.querySelector('#journal-feedback-title')");
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
    assert.equal(evaluate("window.__launcherTest.calls.findLast(c=>c.command==='journal_cancel_recognition').args.reason"),"filters_changed");
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

test("diagnostic screenshot selection persists through navigation and a fresh settings load", () => {
  try {
    setup(); click("设置"); wait("!!document.querySelector('.support-panel')");
    browser("find", "role", "checkbox", "check", "--name", "诊断包包含截图", "--exact");
    wait("window.__launcherTest.savedSettings?.app.diagnostic_images === true");
    click("加页手记"); wait("!!document.querySelector('.journal')"); click("设置"); wait("!!document.querySelector('.support-panel')");
    assert.equal(evaluate("Array.from(document.querySelectorAll('.setting-row')).find(e=>e.textContent.includes('诊断包包含截图')).querySelector('input').checked"), true);
    click("日志文件"); click("截图文件");
    assert.equal(evaluate("window.__launcherTest.calls.some(c=>c.command==='open_screenshots_directory')"),true);
    evaluate("sessionStorage.setItem('journal-test-settings',JSON.stringify(window.__launcherTest.savedSettings))");
    browser("reload"); click("设置");
    wait("!!document.querySelector('.support-panel')");
    assert.equal(evaluate("Array.from(document.querySelectorAll('.setting-row')).find(e=>e.textContent.includes('诊断包包含截图')).querySelector('input').checked"), true);
    click("预览诊断包"); browser("wait", "--text", "已选择包含截图");
    assert.equal(evaluate("window.__launcherTest.calls.findLast(c=>c.command==='preview_diagnostics').args.includeImages"), true);
  } finally { browser("close"); }
});

test("auto-entry opens best match once, keeps background focus, and supports a new request or no match", () => {
  try {
    setup();
    assert.equal(evaluate("document.querySelector('.journal-auto-enter input').checked"), false);
    browser("find", "role", "switch", "check", "--name", "自动进入最匹配地图", "--exact");
    wait("window.__launcherTest.savedSettings?.journal.auto_enter === true");
    evaluate("document.hasFocus=()=>false; window.__launcherTest.recognitionShortcut()");
    wait("!!window.__launcherTest.finishRecognition");
    evaluate("window.__launcherTest.finishRecognition({keys:['rf:1','lf:1'],preview:null,warnings:[]})");
    wait("!!document.querySelector('.journal-full-preview:modal .journal-preview-map img')");
    assert.equal(evaluate("document.querySelector('.journal-full-preview img').alt"), "右前 · 地图 1");
    assert.deepEqual(evaluate("window.__launcherTest.calls.filter(c=>/show|unminimize|set_focus/.test(c.command))"), []);
    click("退出全窗口预览"); wait("!document.querySelector('.journal-full-preview')");
    click("2F");
    assert.equal(evaluate("!!document.querySelector('.journal-full-preview')"), false);
    evaluate("window.__launcherTest.recognitionShortcut()");
    wait("window.__launcherTest.calls.filter(c=>c.command==='journal_recognize').length===2");
    evaluate("window.__launcherTest.finishRecognition({keys:['lf:1'],preview:null,warnings:[]})");
    wait("document.querySelector('.journal-full-preview img')?.alt==='左前 · 地图 1'");
    evaluate("window.__launcherTest.recognitionShortcut()");
    wait("window.__launcherTest.calls.filter(c=>c.command==='journal_recognize').length===3");
    evaluate("window.__launcherTest.finishRecognition({keys:[],preview:null,warnings:[]})");
    browser("wait", "--text", "未找到匹配地图");
    assert.equal(evaluate("!!document.querySelector('.journal-full-preview')"), false);
  } finally { browser("close"); }
});

test("map dragging remembers position, clamps edges, and keeps position presets selected until moved", () => {
  try {
    setup(); click("识别一楼地图"); wait("!!window.__launcherTest.finishRecognition");
    evaluate("window.__launcherTest.finishRecognition({keys:['rf:1','lf:1'],preview:null,warnings:[]})");
    click("查看右前 · 地图 1"); wait("!!document.querySelector('.journal-detail img')"); click("进入地图");
    wait("!!document.querySelector('.journal-preview-map img')?.complete");
    assert.equal(evaluate("document.querySelector('.journal-preview-positions button[aria-pressed=true]').textContent"),"居中");
    click("左上"); wait("window.__launcherTest.savedSettings?.journal.preview_position.preset==='top_left'");
    click("左上");
    assert.equal(evaluate("document.querySelector('.journal-preview-positions button[aria-pressed=true]').textContent"),"左上");
    click("居中");
    const rect = evaluate("(()=>{const r=document.querySelector('.journal-preview-map').getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2}})()");
    browser("mouse", "move", String(rect.x), String(rect.y)); browser("mouse", "down");
    browser("mouse", "move", "1900", "1200"); browser("mouse", "up");
    wait("window.__launcherTest.savedSettings?.journal.preview_position.x > 0.5");
    assert.equal(evaluate("document.querySelectorAll('.journal-preview-positions button[aria-pressed=true]').length"),0);
    const position = evaluate("window.__launcherTest.savedSettings.journal.preview_position");
    const bounds = () => evaluate("(()=>{const r=document.querySelector('.journal-preview-map').getBoundingClientRect();return {x:r.x,y:r.y,right:r.right,bottom:r.bottom,width:innerWidth,height:innerHeight}})()");
    const moved = bounds();
    assert.ok(moved.x >= 15.5 && moved.y >= 15.5 && moved.right <= moved.width - 15.5 && moved.bottom <= moved.height - 15.5);
    assert.equal(evaluate("(()=>{const e=document.querySelector('.journal-preview-floors button[aria-pressed=true]'),r=e.getBoundingClientRect();return e.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))})()"), true);
    evaluate(`window.__launcherTest.mapImage='data:image/svg+xml,'+encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="800" height="200"><rect width="800" height="200" fill="tan"/></svg>')`);
    browser("click", ".journal-preview-floors button:last-child");
    wait("document.querySelector('.journal-full-preview img')?.naturalWidth===800");
    assert.deepEqual(evaluate("window.__launcherTest.savedSettings.journal.preview_position"), position);
    browser("click", ".journal-preview-map");
    evaluate("new Promise(resolve=>setTimeout(resolve,350))");
    assert.deepEqual(evaluate("window.__launcherTest.savedSettings.journal.preview_position"), position, "clicking a clamped map does not overwrite the shared preferred position");
    click("退出全窗口预览"); wait("!document.querySelector('.journal-full-preview')");
    click("进入地图"); wait("!!document.querySelector('.journal-preview-map')");
    browser("set", "viewport", "1000", "680");
    const changed = bounds();
    assert.ok(changed.x >= 15.5 && changed.y >= 15.5 && changed.right <= changed.width - 15.5 && changed.bottom <= changed.height - 15.5);
    evaluate("document.querySelector('.journal-preview-map').focus()"); browser("press", "ArrowUp");
    wait(`window.__launcherTest.savedSettings.journal.preview_position.y < ${position.y}`);
    click("右下"); wait("window.__launcherTest.savedSettings.journal.preview_position.preset==='bottom_right'");
    const anchored = bounds();
    assert.ok(Math.abs(anchored.right-anchored.width+16)<1 && Math.abs(anchored.bottom-anchored.height+16)<1);
    click("居中");
    wait("window.__launcherTest.savedSettings.journal.preview_position.x===0.5 && window.__launcherTest.savedSettings.journal.preview_position.y===0.5");
    const reset = bounds();
    assert.ok(Math.abs((reset.x + reset.right) / 2 - reset.width / 2) < 1);
    assert.ok(Math.abs((reset.y + reset.bottom) / 2 - reset.height / 2) < 1);
    evaluate("document.querySelector('.journal-preview-map').focus()"); browser("press", "Shift+ArrowDown");
    wait("window.__launcherTest.savedSettings.journal.preview_position.y > 0.5");
    assert.equal(evaluate("document.querySelectorAll('.journal-preview-positions button[aria-pressed=true]').length"),0);
    const restoredPosition = evaluate("window.__launcherTest.savedSettings.journal.preview_position");
    evaluate("sessionStorage.setItem('journal-test-settings',JSON.stringify(window.__launcherTest.savedSettings))");
    browser("reload");
    evaluate("window.__launcherTest.packs=[{id:'base',season_id:'s0',difficulty:'hard',maps:[{index:1,category:'lf',name:'',floors:{1:'left.png'},display:{}}]}]");
    click("加页手记"); wait("!!document.querySelector('.journal-map')"); click("查看地图 1"); wait("!!document.querySelector('.journal-detail img')"); click("进入地图");
    wait("!!document.querySelector('.journal-preview-map')");
    const reopened = bounds();
    assert.ok(Math.abs((reopened.x + reopened.right) / 2 - reopened.width / 2) < 1);
    assert.ok(Math.abs((reopened.y + reopened.bottom) / 2 / reopened.height - restoredPosition.y) < 0.002);
  } finally { browser("close"); }
});
