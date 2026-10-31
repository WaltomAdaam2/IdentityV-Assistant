import assert from "node:assert/strict";
import {spawnSync} from "node:child_process";
import test from "node:test";

const binary = process.env.AGENT_BROWSER_BIN || "agent-browser";
const origin = process.env.LAUNCHER_TEST_ORIGIN || "http://127.0.0.1:1420";
let session, sequence = 0;
function browser(...args) {
  if (args[0] === "open") session = `idv-consistency-${process.pid}-${++sequence}`;
  const result = spawnSync(binary, ["--session", session, ...args], {encoding: "utf8", timeout: 30000});
  assert.equal(result.status, 0, result.error?.message || result.stderr || result.stdout);
  return result.stdout.trim();
}
const evaluate = code => JSON.parse(browser("eval", code));
const click = name => browser("find", "role", "button", "click", "--name", name, "--exact");
const wait = code => browser("wait", "--fn", code);

test("sidebar navigation retains its focus while content navigation focuses the new page", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    click("任务");
    wait("document.querySelector('.topbar h1')?.textContent === '任务'");
    assert.equal(evaluate("document.activeElement.matches('.nav-item.active')"), true);
    browser("press", "Tab");
    assert.equal(evaluate("document.activeElement.textContent"), "游戏启动");
    browser("press", "Enter");
    wait("Boolean(document.querySelector('.game-launcher'))");
    assert.equal(evaluate("document.activeElement.matches('.nav-item.active')"), true);
    click("概览");
    click("管理任务");
    wait("document.querySelector('.topbar h1')?.textContent === '任务'");
    assert.equal(evaluate("document.activeElement.matches('.topbar h1')"), true);
  } finally { browser("close"); }
});

test("settings failures remain actionable and raw inputs commit only after valid blur", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html?loadFail`);
    click("设置"); browser("wait", "--text", "Fixture: configuration unreadable");
    assert.equal(evaluate("!!document.querySelector('.settings-form input')"), false);
    evaluate("window.__launcherTest.loadFailure=false"); click("重新加载");
    browser("wait", "--text", "识别与诊断");
    browser("fill", 'input[placeholder="第五人格，Identity V"]', "dark,");
    assert.equal(evaluate("document.querySelector('input[placeholder=\"第五人格，Identity V\"]').value"), "dark,");
    browser("press", "Tab");
    wait("window.__launcherTest.savedSettings?.game_window.title_contains[0] === 'dark'");
    const result = evaluate(`(async () => {
      const input = document.querySelector('input[min="0.5"]');
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set;
      input.focus(); setter.call(input,''); input.dispatchEvent(new Event('input',{bubbles:true}));
      await new Promise(r=>setTimeout(r,300));
      const blank = input.value === '';
      input.blur(); await new Promise(r=>setTimeout(r,50));
      return {blank, invalid:input.getAttribute('aria-invalid'), saved:window.__launcherTest.savedSettings.engine.heartbeat_interval_seconds};
    })()`);
    assert.deepEqual(result, {blank:true, invalid:"true", saved:2});
    browser("fill", 'input[min="0.5"]', "3");
    browser("press", "Tab");
    wait("window.__launcherTest.savedSettings.engine.heartbeat_interval_seconds===3");
    evaluate("window.__launcherTest.saveFailure=true");
    browser("select", 'select:has(option[value="en-US"])', "en-US");
    browser("wait", "--text", "Settings not saved");
    assert.equal(evaluate("document.documentElement.lang"), "en-US");
    evaluate("window.__launcherTest.saveFailure=false"); click("Retry save");
    wait("!document.querySelector('.settings-save-error') && window.__launcherTest.savedSettings.app.language==='en-US'");
    evaluate("window.__launcherTest.snapshotFailure=true");
    browser("wait", "--text", "Status unavailable");
    evaluate("window.__launcherTest.snapshotFailure=false"); click("Reconnect");
    wait("!document.querySelector('.persistent-feedback')");
  } finally { browser("close"); }
});

test("passed retry waits for explicit start and task stopping is global", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    evaluate("window.__launcherTest.failPreflight=true"); click("任务");
    browser("wait", "--text", "启动已启用任务"); click("启动已启用任务");
    browser("wait", "--text", "无法获取游戏画面"); click("重新检查");
    browser("wait", "--text", "启动原任务");
    assert.equal(evaluate("window.__launcherTest.calls.filter(c=>c.command==='send_engine_command'&&c.args.command.type==='start_tasks').length"),1);
    evaluate("window.__launcherTest.failPreflight=false"); click("启动原任务");
    wait("!document.querySelector('.preflight-dialog') && window.__launcherTest.calls.filter(c=>c.command==='send_engine_command'&&c.args.command.type==='start_tasks').length===2");
    assert.equal(evaluate("document.querySelector('.section-heading .primary').disabled"),true);
    evaluate("document.querySelector('.section-heading .primary').click()");
    assert.equal(evaluate("window.__launcherTest.calls.filter(c=>c.command==='send_engine_command'&&c.args.command.type==='start_tasks').length"),2);
    evaluate("window.__launcherTest.emitLog({type:'task_started',task_id:'login_lobby'})");
    click("停止任务与队列");
    assert.equal(evaluate("window.__launcherTest.calls.at(-1).args?.command?.type === 'stop_task' || window.__launcherTest.calls.some(c=>c.command==='send_engine_command'&&c.args.command.type==='stop_task')"),true);
    assert.equal(evaluate("document.querySelectorAll('.task-stop-button').length"),0);
    click("概览"); browser("wait", "--text", "重启"); click("重启");
    browser("wait", "--text", "重启自动化服务？");
    assert.equal(evaluate("window.__launcherTest.calls.some(c=>c.command==='restart_engine')"),false);
    click("取消"); wait("!document.querySelector('#restart-title')");
    evaluate("window.__launcherTest.deferRestart=true");
    click("重启"); click("确认重启");
    wait("window.__launcherTest.calls.filter(c=>c.command==='restart_engine').length===1");
    click("停止");
    assert.equal(evaluate("window.__launcherTest.calls.some(c=>c.command==='stop_engine')"),false);
    evaluate("window.__launcherTest.finishRestart()");
    wait("window.__launcherTest.calls.filter(c=>c.command==='stop_engine').length===1");
  } finally { browser("close"); }
});

test("download blocks dismissal until cancellation and launcher popover restores focus", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html?desktop`);
    click("设置"); wait("document.querySelector('.update-panel button')?.disabled===false");
    click("手动检查"); browser("wait", "--text", "发现新版本"); click("下载更新 · 9.5 MB");
    browser("wait", "--text", "正在下载更新"); browser("press", "Escape");
    assert.equal(evaluate("document.querySelector('.update-dialog')?.matches(':modal') && document.querySelector('.update-dialog-close').disabled"),true);
    click("取消下载"); browser("wait", "--text", "下载已取消");
    browser("press", "Escape"); wait("!document.querySelector('.update-dialog')");
    click("游戏启动"); browser("wait", "--text", "活动与公告"); click("启动设置");
    wait("document.activeElement?.getAttribute('aria-label')==='关闭启动设置'");
    browser("press", "Escape"); wait("!document.querySelector('.launcher-setup')");
    assert.equal(evaluate("document.activeElement?.getAttribute('aria-label')"),"启动设置");
    click("启动设置");
    evaluate("document.querySelector('.launcher-choose').click(); setTimeout(()=>document.querySelector('.launcher-choose').click(),60)");
    wait("document.querySelector('.launcher-setup')?.classList.contains('is-visible') && !document.querySelector('.launcher-setup').inert");
  } finally { browser("close"); }
});

test("map previews use native modality and restore list focus with symmetric filters", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    evaluate("window.__launcherTest.packs=[{id:'fixture',name:'Fixture',version:'1',season_id:'s0',season_name:'S0',difficulty:'hard',maps:[{index:1,category:'lf',name:'地图 1',floors:{'1':'map.png'},display:{}}]}]");
    click("加页手记"); wait("!!document.querySelector('.journal-map')"); click("查看地图 1");
    wait("document.activeElement?.classList.contains('journal-back')");
    click("进入地图"); wait("document.querySelector('.journal-full-preview')?.matches(':modal')");
    browser("press", "Escape"); wait("!document.querySelector('.journal-full-preview')");
    assert.equal(evaluate("document.activeElement?.textContent"),"进入地图");
    browser("press", "Escape"); wait("document.activeElement?.classList.contains('journal-map')");
    click("收起地图筛选"); wait("document.querySelector('#journal-filter-body').inert");
    const motion = evaluate("({width:getComputedStyle(document.querySelector('.journal-layout')).transitionDuration,body:getComputedStyle(document.querySelector('#journal-filter-body')).transitionDuration})");
    assert.equal(motion.width,"0.22s"); assert.equal(motion.body,"0.22s, 0.22s");
    click("展开地图筛选"); wait("!document.querySelector('#journal-filter-body').inert");
  } finally { browser("close"); }
});
