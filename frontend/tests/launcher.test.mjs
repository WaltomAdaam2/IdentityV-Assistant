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

test("journal remembers map options silently but resets the temporary floor on reentry", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("find", "role", "button", "click", "--name", "加页手记", "--exact");
    browser("wait", "--fn", "Boolean(document.querySelector('#journal-filter-body'))");
    assert.equal(JSON.parse(browser("eval", "document.querySelector('#journal-filter-body').hidden")), false);
    browser("find", "role", "button", "click", "--name", "三开", "--exact");
    browser("wait", "--fn", "window.__launcherTest.calls.some(c => c.command === 'save_settings' && c.args.settings.journal?.door === 'triple')");
    assert.equal(JSON.parse(browser("eval", "Boolean(document.querySelector('.settings-saved-toast'))")), false);
    browser("eval", "window.__launcherTest.calls = []");
    browser("find", "role", "button", "click", "--name", "二楼", "--exact");
    browser("wait", "500");
    assert.equal(JSON.parse(browser("eval", "window.__launcherTest.calls.some(c => c.command === 'save_settings')")), false);
    browser("find", "role", "button", "click", "--name", "收起地图筛选", "--exact");
    assert.equal(JSON.parse(browser("eval", "document.querySelector('#journal-filter-body').hidden")), true);
    browser("find", "role", "button", "click", "--name", "任务", "--exact");
    browser("wait", "--fn", "!document.querySelector('.journal')");
    browser("find", "role", "button", "click", "--name", "加页手记", "--exact");
    browser("wait", "--text", "地图筛选");
    assert.equal(JSON.parse(browser("eval", "document.querySelector('#journal-filter-body').hidden")), false);
    assert.equal(JSON.parse(browser("eval", "Array.from(document.querySelectorAll('.journal-options button')).find(b => b.textContent === '一楼').getAttribute('aria-pressed')")), "true");
    assert.equal(JSON.parse(browser("eval", "Array.from(document.querySelectorAll('.journal-options button')).find(b => b.textContent === '三开').getAttribute('aria-pressed')")), "true");
  } finally { browser("close"); }
});

test("developer mode reveals retained component details without losing normal launch results", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("find", "role", "button", "click", "--name", "运行日志", "--exact");
    browser("wait", "--fn", "Boolean(document.querySelector('.logs-panel'))");
    browser("eval", "(async () => { await window.__launcherTest.emitLog({source:'idv-login',message:'证书已存在且有效，跳过生成。'}); await window.__launcherTest.emitLog({source:'launcher',message:'登录代理检查通过，已启动游戏'}); })()");
    browser("wait", "--text", "登录代理检查通过，已启动游戏");
    assert.equal(JSON.parse(browser("eval", "document.querySelector('.log-table').textContent.includes('证书已存在')")), false);
    browser("find", "role", "button", "click", "--name", "设置", "--exact");
    browser("eval", "Array.from(document.querySelectorAll('.setting-row')).find(row => row.textContent.includes('开发者模式')).querySelector('input').click()");
    browser("find", "role", "button", "click", "--name", "运行日志", "--exact");
    browser("wait", "--text", "证书已存在且有效，跳过生成。");
    browser("wait", "--text", "登录代理检查通过，已启动游戏");
  } finally { browser("close"); }
});

test("normal logs show startup and a concise game launch sequence", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("find", "role", "button", "click", "--name", "运行日志", "--exact");
    browser("wait", "--fn", "Boolean(document.querySelector('.log-table'))");
    assert.equal(JSON.parse(browser("eval", "document.querySelectorAll('.log-row').length")), 2);
    browser("find", "role", "button", "click", "--name", "游戏启动", "--exact");
    browser("wait", "--text", "活动与公告");
    browser("find", "role", "button", "click", "--name", "启动游戏", "--exact");
    browser("wait", "--text", "取消启动");
    browser("eval", "window.__launcherTest.emitLog({source:'launcher',message:'登录代理检查通过，已启动游戏'}); window.__launcherTest.finishLaunch()");
    browser("find", "role", "button", "click", "--name", "运行日志", "--exact");
    browser("wait", "--text", "登录代理检查通过，已启动游戏");
    assert.equal(JSON.parse(browser("eval", "document.querySelectorAll('.log-row').length")), 5);
  } finally { browser("close"); }
});

test("notices fade after a second and hovering during fade rescues them", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("wait", "--text", "游戏启动");
    const result = JSON.parse(JSON.parse(browser("eval", `(async () => {
      const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
      await window.__launcherTest.emitLog({type:'error', code:'WINDOW_NOT_FOUND', message:'停止请求已发送', recoverable:true});
      await delay(1200);
      const notice = document.querySelector('.notice');
      const fading = !!notice && Number(getComputedStyle(notice).opacity) < 1;
      notice?.dispatchEvent(new MouseEvent('mouseover', {bubbles:true, relatedTarget:document.body}));
      await delay(900);
      const held = !!document.querySelector('.notice') && Number(getComputedStyle(notice).opacity) === 1;
      notice?.dispatchEvent(new MouseEvent('mouseout', {bubbles:true, relatedTarget:document.body}));
      await delay(1800);
      return JSON.stringify({fading, held, removed:!document.querySelector('.notice')});
    })()`)));
    assert.deepEqual(result, {fading:true, held:true, removed:true});
  } finally { browser("close"); }
});

test("component updates check at game launch only and share the dialog source choice", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("wait", "--text", "游戏启动");
    assert.equal(JSON.parse(browser("eval", "window.__launcherTest.calls.some(c => c.command === 'check_login_updates')")), false);
    browser("eval", "window.__launcherTest.hasHotfix = true");
    browser("find", "role", "button", "click", "--name", "游戏启动", "--exact");
    browser("wait", "--text", "活动与公告");
    assert.equal(JSON.parse(browser("eval", "window.__launcherTest.calls.some(c => c.command === 'check_login_updates')")), false);
    browser("find", "role", "button", "click", "--name", "启动游戏", "--exact");
    browser("wait", "--text", "测试热更新");
    assert.equal(JSON.parse(browser("eval", "window.__launcherTest.calls.some(c => c.command === 'launch_game')")), false);
    assert.equal(JSON.parse(browser("eval", "document.querySelector('dialog[open]').classList.contains('update-dialog')")), true);
    browser("select", "#component-update-source", "auto");
    browser("find", "role", "button", "click", "--name", "应用热更新", "--exact");
    browser("wait", "--fn", "window.__launcherTest.calls.some(c => c.command === 'apply_login_hotfixes' && c.args.source === 'auto')");
    browser("wait", "--text", "取消启动");
    browser("eval", "window.__launcherTest.finishLaunch()");
    browser("wait", "--text", "登录代理已就绪");
    browser("find", "role", "button", "click", "--name", "启动设置", "--exact");
    browser("find", "role", "button", "click", "--name", "检查组件更新", "--exact");
    browser("wait", "--text", "组件已是最新版本");
    browser("find", "role", "button", "click", "--name", "关闭组件更新", "--exact");
    assert.equal(JSON.parse(browser("eval", "Boolean(document.querySelector('.launcher-setup input[type=checkbox]'))")), false);
    browser("find", "role", "button", "click", "--name", "关闭启动设置", "--exact");
    browser("eval", "window.__launcherTest.calls = []; window.__launcherTest.hasHotfix = true");
    browser("find", "role", "button", "click", "--name", "启动游戏", "--exact");
    browser("wait", "--text", "测试热更新");
    assert.equal(JSON.parse(browser("eval", "window.__launcherTest.calls.some(c => c.command === 'check_login_updates')")), true);
  } finally { browser("close"); }
});

test("title bar minimize explicitly requests the taskbar", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("find", "role", "button", "click", "--name", "最小化", "--exact");
    browser("wait", "--fn", "window.__launcherTest.calls.some(c => c.command === 'minimize_to_taskbar')");
    assert.equal(JSON.parse(browser("eval", "window.__launcherTest.calls.some(c => c.command === 'plugin:window|minimize')")), false);
  } finally { browser("close"); }
});

test("launcher fits the window with a long error banner and open launch settings", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("set", "viewport", "1000", "680");
    browser("find", "role", "button", "click", "--name", "游戏启动", "--exact");
    browser("wait", "--text", "活动与公告");
    browser("eval", "window.__launcherTest.saveFailure = true");
    browser("eval", "new MutationObserver((_, observer) => { const notice = document.querySelector('.notice'); if (notice) { notice.dispatchEvent(new MouseEvent('mouseover', {bubbles:true, relatedTarget:document.body})); observer.disconnect(); } }).observe(document.body, {childList:true, subtree:true})");
    browser("find", "role", "button", "click", "--name", "启动设置", "--exact");
    browser("find", "role", "button", "click", "--name", "选择游戏程序", "--exact");
    browser("wait", "--text", "Hotkey already registered");
    const layout = JSON.parse(JSON.parse(browser("eval", "JSON.stringify((() => {const w=document.querySelector('.workspace'), p=document.querySelector('.launcher-setup').getBoundingClientRect(), r=w.getBoundingClientRect(), b=document.querySelector('.launcher-start').getBoundingClientRect(); return {overflow:w.scrollHeight-w.clientHeight, top:p.top-r.top, bottom:r.bottom-p.bottom, button:r.bottom-b.bottom};})())")));
    assert.ok(layout.overflow <= 1 && layout.top >= 0 && layout.bottom >= 0 && layout.button >= 0, JSON.stringify(layout));
    assert.match(browser("eval", "getComputedStyle(document.querySelector('.workspace-launcher')).backgroundImage"), /background\.jpg/);
    const panel = JSON.parse(JSON.parse(browser("eval", "JSON.stringify((() => { const p=document.querySelector('.launcher-setup'); return {overflow:p.scrollHeight-p.clientHeight, bottom:p.getBoundingClientRect().bottom, button:document.querySelector('.launcher-start').getBoundingClientRect().top}; })())")));
    assert.ok(panel.overflow <= 1 && panel.bottom < panel.button, JSON.stringify(panel));
  } finally { browser("close"); }
});

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

test("proxy pill uses existing status styling and replaces a stale error during startup", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("eval", "Object.assign(window.__launcherTest.launcherStatus, {phase:'error', proxy_phase:'error', message:'旧的代理错误'})");
    browser("find", "role", "button", "click", "--name", "游戏启动", "--exact");
    browser("wait", "--text", "旧的代理错误");
    browser("find", "role", "button", "click", "--name", "启动游戏", "--exact");
    browser("wait", "--text", "取消启动");
    assert.equal(JSON.parse(browser("eval", "Boolean(document.querySelector('.launcher-game-error'))")), false);
    for (const [phase, label] of [['initializing','代理初始化中'],['updating','检查组件更新'],['restarting','代理重启中'],['checking','正在检查代理'],['ready','登录代理已就绪'],['stopping','代理停止中']]) {
      browser("eval", `Object.assign(window.__launcherTest.launcherStatus, {proxy_phase:'${phase}', message:null})`);
      browser("wait", "--text", label);
      assert.equal(JSON.parse(browser("eval", "document.querySelector('.launcher-proxy-status').classList.contains('engine-status')")), true);
    }
    const rect = JSON.parse(JSON.parse(browser("eval", "JSON.stringify((() => {const p=document.querySelector('.launcher-proxy-status').getBoundingClientRect(), r=document.querySelector('.game-launcher').getBoundingClientRect();return {top:p.top-r.top,right:r.right-p.right};})())")));
    assert.ok(rect.top >= 0 && rect.top < 50 && rect.right >= 0 && rect.right < 50);
    browser("eval", "Object.assign(window.__launcherTest.launcherStatus, {proxy_phase:'checking', message:null})");
    browser("wait", "--text", "正在检查代理");
    browser("find", "role", "button", "click", "--name", "启动设置", "--exact");
    const pill = JSON.parse(JSON.parse(browser("eval", "JSON.stringify((() => {const p=document.querySelector('.launcher-proxy-status'), r=p.getBoundingClientRect(), s=document.querySelector('.launcher-setup').getBoundingClientRect();return {gap:s.top-r.bottom, visible:!!document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)?.closest('.launcher-proxy-status'), pulse:getComputedStyle(p.querySelector('.dot')).animationName, change:getComputedStyle(p.querySelector('.launcher-proxy-label')).animationName};})())")));
    assert.ok(pill.visible && pill.gap >= 8, JSON.stringify(pill));
    assert.equal(pill.pulse, "status-breathe");
    assert.equal(pill.change, "launcher-status-change");
  } finally { browser("close"); }
});

test("long news stays within half the launcher and controls share hover movement", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("set", "viewport", "1000", "680");
    browser("eval", "window.__launcherTest.newsFailure = false; window.__launcherTest.newsTitle = '第五人格超长活动公告'.repeat(25)");
    browser("find", "role", "button", "click", "--name", "游戏启动", "--exact");
    browser("wait", "--text", "活动与公告");
    const layout = JSON.parse(JSON.parse(browser("eval", "JSON.stringify((() => { const panel=document.querySelector('.launcher-news'); const row=panel.querySelector('li'); const button=row.querySelector('button'); return { panel:panel.getBoundingClientRect().width, page:document.querySelector('.game-launcher').getBoundingClientRect().width, right:panel.getBoundingClientRect().right, rowRight:row.getBoundingClientRect().right, clipped:button.scrollWidth>button.clientWidth, height:document.querySelector('.launcher-start').getBoundingClientRect().height }; })())")));
    assert.ok(layout.panel <= layout.page / 2, JSON.stringify(layout));
    assert.ok(layout.rowRight <= layout.right, JSON.stringify(layout));
    assert.equal(layout.clipped, true);
    assert.ok(layout.height <= 48);
    browser("hover", ".launcher-start");
    browser("wait", "--fn", "document.getAnimations().every(a => a.playState !== 'running')");
    const tops = () => JSON.parse(JSON.parse(browser("eval", "JSON.stringify(Array.from(document.querySelectorAll('.launcher-launch-controls > button'), b => b.getBoundingClientRect().top))")));
    assert.ok(Math.abs(tops()[0] - tops()[1]) < 0.5);
    browser("mouse", "down", "left");
    browser("wait", "--fn", "document.getAnimations().every(a => a.playState !== 'running')");
    assert.ok(tops()[0] > tops()[1]);
    browser("mouse", "up", "left");
    browser("eval", "document.querySelector('.launcher-start').focus()");
    assert.equal(JSON.parse(browser("eval", "getComputedStyle(document.querySelector('.launcher-start')).outlineWidth")), "0px");
  } finally { browser("close"); }
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
    assert.equal(JSON.parse(browser("eval", "document.querySelector('.launcher-start').disabled")), false);
    assert.match(browser("eval", "document.querySelector('.launcher-start').textContent"), /取消启动/);
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
    browser("eval", "window.__launcherTest.scanMissing = true");
    browser("find", "role", "button", "click", "--name", "自动配置", "--exact");
    browser("wait", "--text", "未找到官方组件");
    browser("find", "role", "button", "click", "--name", "官方下载", "--exact");
    browser("wait", "--text", "C:\\Assistant\\components\\idv-login\\v6.3.2-stable");
    assert.equal(JSON.parse(browser("eval", "window.__launcherTest.calls.some(call => call.command === 'configure_login_component' && call.args.download === true)")), true);
    browser("eval", "window.__launcherTest.scanMissing = false");
    browser("find", "role", "button", "click", "--name", "自动配置", "--exact");
    browser("wait", "--text", "D:\\idv-login");
    browser("find", "role", "button", "click", "--name", "选择登录组件", "--exact");
    browser("wait", "--text", "D:\\idv-login\\python-embed\\python.exe");
    assert.equal(JSON.parse(browser("eval", "window.__launcherTest.calls.some(call => call.command === 'set_login_component' && call.args.executablePath === 'D:\\\\idv-login\\\\python-embed\\\\python.exe')")), true);
    browser("find", "role", "button", "click", "--name", "关闭启动设置", "--exact");
    assert.equal(JSON.parse(browser("eval", "Boolean(document.querySelector('.launcher-setup'))")), false);
  } finally { browser("close"); }
});

test("a second click cancels launch and later health failure removes ready status", () => {
  try {
    browser("open", `${origin}/tests/launcher-harness.html`);
    browser("find", "role", "button", "click", "--name", "游戏启动", "--exact");
    browser("wait", "--text", "活动与公告");
    browser("find", "role", "button", "click", "--name", "启动游戏", "--exact");
    browser("find", "role", "button", "click", "--name", "取消启动", "--exact");
    browser("wait", "--text", "操作已取消");
    assert.equal(JSON.parse(browser("eval", "window.__launcherTest.calls.some(call => call.command === 'cancel_game_launch')")), true);
    browser("find", "role", "button", "click", "--name", "启动游戏", "--exact");
    browser("eval", "window.__launcherTest.finishLaunch()");
    browser("wait", "--text", "登录代理已就绪");
    browser("eval", "Object.assign(window.__launcherTest.launcherStatus, {phase:'error', proxy_phase:'error', message:'登录代理不可用'})");
    browser("wait", "--text", "登录代理不可用");
    assert.doesNotMatch(browser("eval", "document.querySelector('.launcher-status').textContent"), /已就绪/);
  } finally { browser("close"); }
});
