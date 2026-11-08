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

test("live recognition shows full capture, resolved metadata and crop, retaining evidence on failure", () => {
  try {
    setup();
    click("识别一楼地图");
    wait("!!window.__launcherTest.recognitionRequest");
    const id = evaluate("window.__launcherTest.recognitionRequest.requestId");
    assert.match(evaluate("document.querySelector('.journal-footer small').textContent"), /难度识别中.*楼层识别中/);
    const full = 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="800" height="450"><rect width="800" height="450" fill="tan"/></svg>');
    evaluate(`window.__launcherTest.emitRecognitionPreview(${JSON.stringify({request_id:id,preview:full,preview_kind:"full"})})`);
    wait("document.querySelector('.journal-matching-preview figcaption')?.textContent==='已截图，正在分析…'");
    evaluate(`window.__launcherTest.emitRecognitionProgress(${JSON.stringify({request_id:id,stage:"locating",completed:0,total:0,season:"s0",difficulty:"nightmare",floor:"1"})})`);
    wait("document.querySelector('.journal-footer small').textContent.includes('噩梦 · 一楼')");
    assert.equal(evaluate("window.__launcherTest.calls.some(c=>c.command==='journal_cancel_recognition')"), false);
    evaluate(`window.__launcherTest.emitRecognitionPreview(${JSON.stringify({request_id:id,preview:full,preview_kind:"region"})})`);
    wait("document.querySelector('.journal-matching-preview figcaption')?.textContent==='正在核对地图区域'");
    evaluate("window.__launcherTest.failRecognition('无法定位地图 / Could not locate map')");
    wait("document.querySelector('.journal-recognition [role=status]').textContent.includes('无法定位地图')");
    assert.equal(evaluate("!!document.querySelector('.journal-matching-preview img')"), true);
    assert.match(evaluate("document.querySelector('.journal-footer small').textContent"), /噩梦 · 一楼/);
    click("重新识别");
    wait("window.__launcherTest.calls.filter(c=>c.command==='journal_recognize').length===2");
    assert.equal(evaluate("!!document.querySelector('.journal-matching-preview img')"), false);
    evaluate(`window.__launcherTest.emitRecognitionPreview(${JSON.stringify({request_id:id,preview:full,preview_kind:"region"})})`);
    assert.equal(evaluate("!!document.querySelector('.journal-matching-preview img')"), false);
    evaluate(`window.__launcherTest.emitRecognitionPreview({request_id:window.__launcherTest.recognitionRequest.requestId,preview:${JSON.stringify(full)},preview_kind:'full'})`);
    evaluate("window.__launcherTest.finishRecognition({keys:[],difficulty:'nightmare',floor:'1',preview:null,warnings:[]})");
    wait("document.querySelector('.journal-recognition [role=status]').textContent.includes('未找到匹配地图')");
    assert.equal(evaluate("!!document.querySelector('.journal-matching-preview img')"), true);
  } finally { browser("close"); }
});

test("S0 Auto and manual choices reach recognition without changing the other mode", () => {
  try {
    setup();
    assert.equal(evaluate("document.querySelector('.journal').textContent.includes('自动判断识图难度；当前浏览')"), false);
    assert.equal(evaluate("Array.from(document.querySelectorAll('button')).find(e=>e.textContent==='自动').getAttribute('aria-pressed')"), "true");
    for (const [label, difficulty] of [["自动", "auto"], ["困难", "hard"], ["噩梦", "nightmare"], ["自动", "auto"]]) {
      click(label); click("识别一楼地图");
      wait(`window.__launcherTest.recognitionRequest?.difficulty==='${difficulty}'`);
      evaluate(`window.__launcherTest.finishRecognition({difficulty:'${difficulty === "auto" ? "nightmare" : difficulty}',keys:[],preview:null,warnings:[]})`);
      wait("!document.querySelector('.journal-viewer[aria-busy=true]')");
      assert.equal(evaluate(`Array.from(document.querySelectorAll('button')).find(e=>e.textContent==='${label}').getAttribute('aria-pressed')`), "true");
    }
  } finally { browser("close"); }
});

test("S1 preserves S0 view preferences, uses selected difficulty, and cycles B1 through B3", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html?desktop`);
    evaluate(`window.__launcherTest.packs=[
      {id:'s0',season_id:'s0',season_name:'S0 厄运之女',difficulty:'hard',maps:[{index:1,category:'lf',floors:{1:'s0.png'},display:{1:'route.png'}}]},
      ...['hard','nightmare'].map(difficulty=>({id:'s1_'+difficulty,season_id:'s1',season_name:'S1 磷光症',difficulty,maps:[{index:1,category:'td',floors:{1:'b1.png',2:'b2.png',3:'b3.png'},display:{}}]}))]`);
    click("加页手记"); wait("!!document.querySelector('.journal') && !document.querySelector('.journal-viewer[aria-busy=true]')");
    click("速刷"); wait("window.__launcherTest.savedSettings?.journal.display==='route'");
    browser("select", ".journal-season select", "s1");
    wait("!!Array.from(document.querySelectorAll('button')).find(e=>e.textContent==='识别当前楼层地图')");
    assert.deepEqual(evaluate("Array.from(document.querySelectorAll('.journal-filters legend'),e=>e.textContent)"), ["难度","楼层","显示"]);
    assert.equal(evaluate("Array.from(document.querySelectorAll('button')).find(e=>e.textContent==='速刷').disabled"), true);
    assert.equal(evaluate("Array.from(document.querySelectorAll('button')).find(e=>e.textContent==='纯净').getAttribute('aria-pressed')"), "true");
    click("纯净");
    assert.equal(evaluate("window.__launcherTest.savedSettings.journal.display"), "route");
    click("噩梦"); click("识别当前楼层地图"); wait("!!window.__launcherTest.finishRecognition");
    assert.deepEqual(evaluate("(({season,difficulty,display})=>({season,difficulty,display}))(window.__launcherTest.recognitionRequest)"), {season:"s1",difficulty:"nightmare",display:"pure"});
    evaluate("window.__launcherTest.emitRecognitionProgress({request_id:window.__launcherTest.recognitionRequest.requestId,stage:'floor',completed:0,total:0})");
    browser("wait", "--text", "正在判断地图楼层…");
    evaluate("window.__launcherTest.finishRecognition({keys:['td:1'],difficulty:'nightmare',floor:'2',preview:null,warnings:[]})");
    wait("document.querySelectorAll('.journal-map').length===1");
    wait("window.__launcherTest.calls.filter(c=>c.command==='journal_read_image').at(-1)?.args.path==='b2.png'");
    browser("wait", "--text", "地下2层识图结果");
    assert.equal(evaluate("document.querySelector('.journal-map-name').textContent"), "地图 1");
    browser("press", "F6");
    assert.equal(evaluate("window.__launcherTest.calls.filter(c=>c.command==='journal_read_image').at(-1)?.args.path"), "b2.png");
    click("查看地图 1"); wait("!!document.querySelector('.journal-detail img')");
    browser("press", "F6");
    assert.equal(evaluate("window.__launcherTest.calls.filter(c=>c.command==='journal_read_image').at(-1)?.args.path"), "b2.png");
    click("进入地图");
    wait("!!document.querySelector('.journal-full-preview:modal img')");
    for (const [key,label] of [["3","地下3层"],["1","地下1层"],["2","地下2层"]]) {
      browser("press", "F6");
      wait(`document.querySelector('.journal-preview-floors button[aria-pressed=true]')?.textContent==='${label}'`);
      wait(`window.__launcherTest.calls.filter(c=>c.command==='journal_read_image').at(-1)?.args.path==='b${key}.png'`);
    }
    assert.deepEqual(evaluate("Array.from(document.querySelectorAll('.journal-preview-floors button'),e=>e.textContent)"), ["地下1层","地下2层","地下3层"]);
    click("退出全窗口预览"); wait("!document.querySelector('.journal-full-preview')");
    browser("find", "role", "switch", "check", "--name", "自动进入最匹配地图", "--exact");
    click("识别当前楼层地图");
    wait("window.__launcherTest.calls.filter(c=>c.command==='journal_recognize').length===2");
    evaluate("window.__launcherTest.finishRecognition({keys:['td:1'],difficulty:'nightmare',floor:'3',preview:null,warnings:[]})");
    wait("document.querySelector('.journal-full-preview:modal .journal-preview-floors button[aria-pressed=true]')?.textContent==='地下3层'");
    wait("window.__launcherTest.calls.filter(c=>c.command==='journal_read_image').at(-1)?.args.path==='b3.png'");
    click("退出全窗口预览"); wait("!document.querySelector('.journal-full-preview')");
    browser("select", ".journal-season select", "s0");
    wait("Array.from(document.querySelectorAll('button')).find(e=>e.textContent==='速刷')?.getAttribute('aria-pressed')==='true'");
    assert.equal(evaluate("window.__launcherTest.savedSettings.journal.display"), "route");
    assert.equal(evaluate("window.__launcherTest.savedSettings.journal.door"), "lf");
  } finally { browser("close"); }
});
test("imported S1 route images enable route view and preserve separate season preferences", () => {
  try {
    setup();
    evaluate("window.__launcherTest.packs=[...window.__launcherTest.packs,{id:'s1_base',season_id:'s1',difficulty:'hard',maps:[{index:1,category:'td',floors:{1:'base.png'},display:{}}]}]");
    click("刷新地图");
    wait("!!document.querySelector('.journal-season option[value=s1]')");
    browser("select", ".journal-season select", "s1");
    wait("Array.from(document.querySelectorAll('button')).find(e=>e.textContent==='速刷')?.disabled");
    browser("wait", "--text", "导入此难度的速刷地图包后可用。");
    evaluate("window.__launcherTest.packs=[...window.__launcherTest.packs,{id:'s1_route',season_id:'s1',difficulty:'hard',maps:[{index:1,category:'td',floors:{1:'base.png'},display:{1:'route.png'}}]}]");
    click("刷新地图");
    wait("Array.from(document.querySelectorAll('button')).find(e=>e.textContent==='速刷')?.disabled===false");
    click("速刷");
    wait("window.__launcherTest.savedSettings?.journal.s1_display==='route'");
    wait("window.__launcherTest.calls.filter(c=>c.command==='journal_read_image').at(-1)?.args.path==='route.png'");
    click("识别当前楼层地图");
    wait("window.__launcherTest.recognitionRequest?.display==='route'");
    evaluate("window.__launcherTest.finishRecognition({difficulty:'hard',floor:'1',keys:['td:1'],preview:null,warnings:[]})");
    wait("!document.querySelector('.journal-viewer[aria-busy=true]')");
    browser("select", ".journal-season select", "s0");
    wait("Array.from(document.querySelectorAll('button')).find(e=>e.textContent==='纯净')?.getAttribute('aria-pressed')==='true'");
    browser("select", ".journal-season select", "s1");
    wait("Array.from(document.querySelectorAll('button')).find(e=>e.textContent==='速刷')?.getAttribute('aria-pressed')==='true'");
  } finally { browser("close"); }
});

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
    assert.equal(evaluate("Array.from(document.querySelectorAll('button')).some(e=>e.textContent.includes('反馈'))"),false);
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
    click("导出诊断包"); click("预览"); browser("wait", "--text", "已选择包含截图");
    assert.equal(evaluate("window.__launcherTest.calls.findLast(c=>c.command==='preview_diagnostics').args.includeImages"), true);
  } finally { browser("close"); }
});

test("diagnostics choose sessions before preview, preserve the snapshot, and reset history on reopen", () => {
  try {
    setup(); click("设置"); wait("!!document.querySelector('.support-panel')");
    browser("set", "viewport", "1000", "680");
    click("导出诊断包"); wait("!!document.querySelector('.support-sessions')");
    assert.equal(evaluate("window.__launcherTest.calls.some(c=>c.command==='preview_diagnostics')"),false);
    assert.deepEqual(evaluate("Array.from(document.querySelectorAll('.support-session input'),e=>({checked:e.checked,disabled:e.disabled}))"),[{checked:true,disabled:true},{checked:false,disabled:false},{checked:false,disabled:false},{checked:false,disabled:false}]);
    assert.equal(evaluate("document.activeElement.id"),"support-title");
    browser("check", ".support-session input[value=recent]"); browser("check", ".support-session input[value=older]");
    browser("wait", "--text", "不限制总大小");
    evaluate("window.__launcherTest.deferPreview=true"); click("预览");
    wait("!!window.__launcherTest.finishPreview"); browser("press", "Escape");
    assert.equal(evaluate("!!document.querySelector('.support-dialog:modal')"),true);
    assert.equal(evaluate("document.querySelector('.support-dialog .primary').disabled"),true);
    evaluate("window.__launcherTest.finishPreview()"); wait("!!document.querySelector('.support-files')");
    assert.deepEqual(evaluate("window.__launcherTest.calls.findLast(c=>c.command==='preview_diagnostics').args"),{includeImages:false,historicalSessions:["recent","older"]});
    assert.equal(evaluate("document.activeElement.id"),"support-title");
    assert.equal(evaluate("(()=>{const e=document.querySelector('.support-dialog .update-actions').getBoundingClientRect();return e.right<=innerWidth && e.bottom<=innerHeight})()"),true);
    click("返回选择"); wait("!!document.querySelector('.support-sessions')");
    assert.equal(evaluate("window.__launcherTest.calls.some(c=>c.command==='discard_diagnostics')"),true);
    assert.equal(evaluate("document.querySelector('.support-session input[value=recent]').checked"),true);
    click("取消"); wait("!document.querySelector('.support-dialog')");
    click("导出诊断包"); wait("!!document.querySelector('.support-sessions')");
    assert.equal(evaluate("document.querySelectorAll('.support-session input:checked').length"),1);
    evaluate("window.__launcherTest.deferPreview=false;window.__launcherTest.previewFailure=true"); click("预览");
    wait("!!document.querySelector('.support-error')");
    assert.equal(evaluate("!!document.querySelector('.support-sessions')"),true);
    evaluate("window.__launcherTest.previewFailure=false"); click("预览"); wait("!!document.querySelector('.support-files')");
    assert.deepEqual(evaluate("window.__launcherTest.calls.findLast(c=>c.command==='preview_diagnostics').args.historicalSessions"),[]);
    evaluate("window.__launcherTest.exportSupportFailure=true"); click("保存 ZIP"); wait("!!document.querySelector('.support-error')");
    assert.equal(evaluate("!!document.querySelector('.support-files')"),true);
    evaluate("window.__launcherTest.exportSupportFailure=false"); click("保存 ZIP"); wait("!document.querySelector('.support-dialog')");
    assert.equal(evaluate("window.__launcherTest.calls.findLast(c=>c.command==='export_diagnostics').args.id"),"bundle-fixture");
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

test("repeated P cancels the helper, waits for release, and only starts the latest request", () => {
  try {
    setup(); click("识别一楼地图"); wait("!!window.__launcherTest.finishRecognition");
    evaluate("window.__finishOld=window.__launcherTest.finishRecognition; window.__launcherTest.recognitionShortcut(); window.__launcherTest.recognitionShortcut()");
    wait("window.__launcherTest.calls.filter(c=>c.command==='journal_cancel_recognition').length>=1");
    assert.equal(evaluate("window.__launcherTest.calls.filter(c=>c.command==='journal_recognize').length"),1);
    evaluate("window.__finishOld({keys:['lf:1'],preview:null,warnings:[]})");
    wait("window.__launcherTest.calls.filter(c=>c.command==='journal_recognize').length===2");
    assert.equal(evaluate("document.querySelectorAll('.journal-map').length"),0);
    evaluate("window.__launcherTest.finishRecognition({difficulty:'hard',keys:['rf:1'],preview:null,warnings:[]})");
    wait("document.querySelectorAll('.journal-map').length===1");
    assert.equal(evaluate("document.querySelector('.journal-map-name').textContent"),"右前 · 地图 1");
    click("查看右前 · 地图 1"); browser("press","F6");
    wait("window.__launcherTest.calls.some(c=>c.command==='journal_read_image' && c.args.path==='right-upper.png')");
    browser("press","F6");
    wait("document.querySelector('.journal-floor-switch button[aria-pressed=true]').textContent==='1F'");
  } finally { browser("close"); }
});

test("detected difficulty persists without cancelling results or automatic entry", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html?desktop`);
    evaluate("window.__launcherTest.packs=[{id:'nightmare',season_id:'s0',difficulty:'nightmare',maps:[{index:1,category:'lf',name:'',floors:{1:'n1.png',2:'n2.png',b1f:'nb.png'},display:{}}]}]");
    click("加页手记"); wait("!!document.querySelector('.journal') && !document.querySelector('.journal-viewer[aria-busy=true]')");
    browser("find","role","switch","check","--name","自动进入最匹配地图","--exact");
    click("识别一楼地图"); wait("!!window.__launcherTest.finishRecognition");
    evaluate("window.__launcherTest.finishRecognition({difficulty:'nightmare',keys:['lf:1'],preview:null,warnings:[]})");
    wait("!!document.querySelector('.journal-full-preview:modal')");
    wait("window.__launcherTest.savedSettings?.journal.difficulty==='nightmare'");
    browser("press","F6"); wait("window.__launcherTest.calls.at(-1).command!=='unused' && [...document.querySelectorAll('.journal-full-preview .journal-floor-switch button')].find(b=>b.getAttribute('aria-pressed')==='true').textContent==='2F'");
    browser("press","F6"); wait("[...document.querySelectorAll('.journal-full-preview .journal-floor-switch button')].find(b=>b.getAttribute('aria-pressed')==='true').textContent==='地下室'");
    assert.equal(evaluate("window.__launcherTest.calls.some(c=>c.command==='journal_cancel_recognition')"),false);
  } finally { browser("close"); }
});


test("floor shortcut settings persist, exclude conflicts and only cycle inside preview", () => {
  try {
    setup(); click("设置"); wait("!!document.querySelector('.settings-form')");
    assert.deepEqual(evaluate("Array.from(document.querySelectorAll('.setting-row')).filter(e=>/快捷键/.test(e.textContent)).map(e=>e.querySelector('select').value)"), ["F3","P","F6"]);
    evaluate("window.__floorSelect=Array.from(document.querySelectorAll('.setting-row')).find(e=>e.textContent.includes('楼层切换快捷键')).querySelector('select'); window.__floorSelect.id='test-floor-key'");
    assert.deepEqual(evaluate("Array.from(window.__floorSelect.options).filter(e=>e.disabled).map(e=>e.value)"), ["P","F3"]);
    browser("select", "#test-floor-key", "K");
    wait("window.__launcherTest.savedSettings?.app.floor_hotkey==='K'");
    evaluate("sessionStorage.setItem('journal-test-settings',JSON.stringify(window.__launcherTest.savedSettings))");
    browser("reload");
    evaluate("window.__launcherTest.packs=[{id:'base',season_id:'s0',difficulty:'hard',maps:[{index:1,category:'lf',floors:{1:'one.png',2:'two.png'},display:{}}]}]");
    click("设置"); wait("!!document.querySelector('.settings-form')");
    assert.equal(evaluate("Array.from(document.querySelectorAll('.setting-row')).find(e=>e.textContent.includes('楼层切换快捷键')).querySelector('select').value"), "K");
    click("加页手记"); wait("!!document.querySelector('.journal-map')");
    browser("wait", "--text", "进入地图后按 K 切换楼层。");
    click("查看地图 1"); wait("!!document.querySelector('.journal-detail img')");
    browser("press", "k");
    assert.equal(evaluate("document.querySelector('.journal-floor-switch button[aria-pressed=true]').textContent"), "1F");
    click("进入地图"); wait("!!document.querySelector('.journal-full-preview:modal img')");
    browser("press", "F6");
    assert.equal(evaluate("document.querySelector('.journal-preview-floors button[aria-pressed=true]').textContent"), "1F");
    browser("press", "k"); wait("document.querySelector('.journal-preview-floors button[aria-pressed=true]').textContent==='2F'");
    browser("press", "Control+k");
    assert.equal(evaluate("document.querySelector('.journal-preview-floors button[aria-pressed=true]').textContent"), "2F");
    browser("press", "k"); wait("document.querySelector('.journal-preview-floors button[aria-pressed=true]').textContent==='1F'");
  } finally { browser("close"); }
});
