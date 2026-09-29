import { useCallback, useEffect, useMemo, useState } from "react";
import { bridge } from "./bridge";
import type { CaptureFrame, EngineEvent, EngineSnapshot, Settings, TaskItem, TemplateTestResult, WindowInfo } from "./types";

type Page = "dashboard" | "tasks" | "runtime" | "debug" | "logs" | "settings" | "about";
type LogLevel = "debug" | "info" | "warning" | "error";

interface LogEntry {
  id: number;
  at: string;
  level: LogLevel;
  message: string;
  source: string;
}

const nav: Array<{ id: Page; icon: string; label: string }> = [
  { id: "dashboard", icon: "⌂", label: "概览" },
  { id: "tasks", icon: "✓", label: "任务" },
  { id: "runtime", icon: "⌘", label: "运行时" },
  { id: "debug", icon: "⌕", label: "调试" },
  { id: "logs", icon: "≡", label: "日志" },
  { id: "settings", icon: "⚙", label: "设置" },
  { id: "about", icon: "i", label: "关于" },
];

const initialTasks: TaskItem[] = [
  { id: "mock_weekly", name: "模拟周常流程", description: "不操作真实窗口的可验证任务流程", enabled: true, status: "idle" },
  { id: "weekly", name: "周常任务", description: "执行已实现的周常任务状态流", enabled: false, status: "idle" },
];

const defaults: Settings = {
  app: { language: "zh-CN", theme: "system", launch_engine_on_start: false },
  engine: { dry_run: true, heartbeat_interval_seconds: 2, mock_step_delay_seconds: 1 },
  game_window: { process_names: ["dwrg.exe", "IdentityV.exe"], title_contains: ["第五人格", "Identity V"], class_names: [] },
  capture: { backend: "auto", idle_fps: 3, active_fps: 10 },
  vision: { template_threshold: 0.85, state_confirmation_count: 2 },
  debug: { save_debug_screenshots: false, verbose_logs: false },
};

function now() {
  return new Date().toLocaleTimeString("zh-CN", { hour12: false });
}

function asMessage(value: unknown) {
  return value instanceof Error ? value.message : String(value);
}

function engineState(snapshot: EngineSnapshot) {
  return snapshot.status ?? "stopped";
}

function stateLabel(state: string) {
  const labels: Record<string, string> = {
    stopped: "已停止",
    starting: "启动中",
    running: "运行中",
    stopping: "停止中",
    error: "异常",
  };
  return labels[state] ?? state;
}

function statusLabel(status: TaskItem["status"]) {
  return { idle: "空闲", queued: "等待中", running: "执行中", paused: "已暂停" }[status];
}

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [snapshot, setSnapshot] = useState<EngineSnapshot>({ state: "stopped", queue_size: 0 });
  const [tasks, setTasks] = useState(initialTasks);
  const [settings, setSettings] = useState<Settings>(defaults);
  const [logs, setLogs] = useState<LogEntry[]>([
    { id: 1, at: now(), level: "info", message: "控制台已就绪，等待引擎连接。", source: "ui" },
  ]);
  const [notice, setNotice] = useState<string | null>(null);
  const [windowInfo, setWindowInfo] = useState<WindowInfo | null>(null);
  const [frame, setFrame] = useState<CaptureFrame | null>(null);
  const [templateMatch, setTemplateMatch] = useState<TemplateTestResult | null>(null);
  const [lastPlannedAction, setLastPlannedAction] = useState<EngineEvent | null>(null);
  const [developerInputEnabled, setDeveloperInputEnabled] = useState(false);
  const [runtimeEvent, setRuntimeEvent] = useState<EngineEvent | null>(null);

  const appendLog = useCallback((level: LogLevel, message: string, source = "engine") => {
    setLogs((current) => [{ id: Date.now() + Math.random(), at: now(), level, message, source }, ...current].slice(0, 500));
  }, []);

  const run = useCallback(async (label: string, action: () => Promise<void>) => {
    try {
      await action();
      appendLog("info", `${label}请求已发送。`, "ui");
      setNotice(`${label}请求已发送`);
    } catch (error) {
      const message = `${label}失败：${asMessage(error)}`;
      appendLog("error", message, "ui");
      setNotice(message);
    }
  }, [appendLog]);

  const refreshSnapshot = useCallback(async () => {
    try {
      const next = await bridge.getEngineSnapshot();
      setSnapshot(next);
    } catch (error) {
      appendLog("debug", `无法读取运行时快照：${asMessage(error)}`, "ui");
    }
  }, [appendLog]);

  useEffect(() => {
    void refreshSnapshot();
    void bridge.loadSettings().then((loaded) => setSettings({
      ...defaults,
      ...loaded,
      app: { ...defaults.app, ...loaded.app },
      engine: { ...defaults.engine, ...loaded.engine },
      game_window: { ...defaults.game_window, ...loaded.game_window },
      capture: { ...defaults.capture, ...loaded.capture },
      vision: { ...defaults.vision, ...loaded.vision },
      debug: { ...defaults.debug, ...loaded.debug },
    })).catch(() => undefined);
    let dispose: (() => void) | undefined;
    void bridge.onEngineEvent((event: EngineEvent) => {
      const level: LogLevel = event.type === "error" ? "error" : event.level && ["debug", "info", "warning", "error"].includes(event.level) ? event.level : "info";
      const eventMessage = event.message ?? (event.type === "task_state" ? `任务状态：${event.state ?? "未知"}` : event.type === "state_transition" ? `状态迁移：${event.from ?? "?"} → ${event.to ?? "?"}` : event.type === "task_finished" ? `任务结束：${event.reason ?? "未知原因"}` : event.type ?? "收到引擎事件");
      appendLog(level, eventMessage, event.source ?? event.type ?? "engine");
      if (event.type === "heartbeat") {
        setSnapshot((current) => ({ ...current, status: event.engine_status ?? current.status, taskStatus: event.task_status ?? current.taskStatus }));
        if (event.task_status === "paused" || event.task_status === "running") setTasks((current) => current.map((task) => task.status === "running" || task.status === "paused" ? { ...task, status: event.task_status as TaskItem["status"] } : task));
      }
      if (event.type === "window_diagnostics") setWindowInfo(event);
      if (event.type === "debug_frame") {
        const nextFrame = { path: event.path, width: event.width, height: event.height, state: event.state, source: event.source };
        setFrame(nextFrame);
        if (event.path) void bridge.readDebugFrame(event.path).then((imageUrl) => setFrame({ ...nextFrame, imageUrl })).catch((error) => appendLog("warning", `无法读取调试画面：${asMessage(error)}`, "debug"));
      }
      if (event.type === "template_match") setTemplateMatch({ template: event.template, found: event.found, confidence: event.confidence, bbox: event.bbox, threshold: event.threshold });
      if (event.type === "planned_action") { setLastPlannedAction(event); if (event.action === "input_test_enabled") setDeveloperInputEnabled(Boolean(event.enabled)); }
      if (event.type === "task_state" || event.type === "state_transition" || event.type === "task_finished") setRuntimeEvent(event);
      if (event.type === "task_state") setTasks((current) => current.map((task) => task.status === "running" || task.status === "paused" ? { ...task, status: "running" } : task));
      if (event.type === "task_finished") { setTasks((current) => current.map((task) => ({ ...task, status: "idle" }))); setSnapshot((current) => ({ ...current, taskStatus: "idle" })); }
      if (event.type === "error" && !event.recoverable) setSnapshot((current) => ({ ...current, status: "error" }));
    }).then((unlisten) => { dispose = unlisten; }).catch(() => undefined);
    const timer = window.setInterval(() => void refreshSnapshot(), 5_000);
    return () => { window.clearInterval(timer); dispose?.(); };
  }, [appendLog, refreshSnapshot]);

  useEffect(() => {
    document.documentElement.dataset.theme = settings.app.theme;
  }, [settings.app.theme]);

  const actions = {
    start: () => run("启动引擎", async () => { await bridge.startEngine(); await refreshSnapshot(); }),
    stop: () => run("停止引擎", async () => { await bridge.stopEngine(); await refreshSnapshot(); }),
    restart: () => run("重启引擎", async () => { await bridge.restartEngine(); await refreshSnapshot(); }),
  };

  const content = {
    dashboard: <Dashboard snapshot={snapshot} tasks={tasks} onStart={actions.start} onStop={actions.stop} onRestart={actions.restart} onNavigate={setPage} />,
    tasks: <Tasks tasks={tasks} onChange={setTasks} onCommand={(label, command) => run(label, () => bridge.sendEngineCommand(command))} />,
    runtime: <Runtime snapshot={snapshot} runtimeEvent={runtimeEvent} onRefresh={refreshSnapshot} onCommand={(label, command) => run(label, () => bridge.sendEngineCommand(command))} />,
    debug: <Debug windowInfo={windowInfo} frame={frame} templateMatch={templateMatch} lastPlannedAction={lastPlannedAction} developerInputEnabled={developerInputEnabled} appendLog={appendLog} setNotice={setNotice} />,
    logs: <Logs entries={logs} onClear={() => setLogs([])} />,
    settings: <SettingsPage settings={settings} setSettings={setSettings} onSave={() => run("保存设置", () => bridge.saveSettings(settings))} />,
    about: <About />,
  }[page];

  return (
    <main className="shell">
      <aside className="sidebar">
        <div className="brand"><span className="brand-mark">V</span><span>第五人格助手</span></div>
        <nav>{nav.map((item) => <button key={item.id} className={page === item.id ? "nav-item active" : "nav-item"} onClick={() => setPage(item.id)}><span>{item.icon}</span>{item.label}</button>)}</nav>
        <div className="sidebar-footer">本地控制台<br /><span>v0.1.0</span></div>
      </aside>
      <section className="workspace">
        <header className="topbar">
          <div><h1>{nav.find((item) => item.id === page)?.label}</h1><p>IdentityV Assistant</p></div>
          <div className="engine-status"><span className={`dot ${engineState(snapshot)}`} />引擎{stateLabel(engineState(snapshot))}</div>
        </header>
        {notice && <div className="notice"><span>{notice}</span><button aria-label="关闭提示" onClick={() => setNotice(null)}>×</button></div>}
        <div className="page-content">{content}</div>
      </section>
    </main>
  );
}

function Dashboard({ snapshot, tasks, onStart, onStop, onRestart, onNavigate }: { snapshot: EngineSnapshot; tasks: TaskItem[]; onStart: () => void; onStop: () => void; onRestart: () => void; onNavigate: (page: Page) => void }) {
  const state = engineState(snapshot);
  return <>
    <div className="hero-card">
      <div><span className="eyebrow">引擎状态</span><h2>{stateLabel(state)}</h2><p>{snapshot.taskStatus && snapshot.taskStatus !== "idle" ? `任务状态：${snapshot.taskStatus}` : "引擎尚未执行任务。"}</p></div>
      <div className="button-row"><button className="primary" disabled={state === "running" || state === "starting"} onClick={onStart}>启动引擎</button><button disabled={state === "stopped" || state === "stopping"} onClick={onStop}>停止</button><button onClick={onRestart}>重启</button></div>
    </div>
    <div className="stat-grid">
      <Stat label="引擎进程" value={snapshot.pid ? `#${snapshot.pid}` : "未启动"} />
      <Stat label="心跳延迟" value={snapshot.heartbeatAgeSeconds == null ? "—" : `${snapshot.heartbeatAgeSeconds.toFixed(1)} 秒`} />
      <Stat label="启用任务" value={String(tasks.filter((task) => task.enabled).length)} />
      <Stat label="引擎版本" value={snapshot.engineVersion ?? "—"} />
    </div>
    <div className="two-column">
      <section className="panel"><div className="panel-title"><div><h3>任务概况</h3><p>当前配置中的自动任务</p></div><button className="link-button" onClick={() => onNavigate("tasks")}>管理任务</button></div>
        <div className="task-summary">{tasks.map((task) => <div key={task.id} className="summary-row"><span className={`task-indicator ${task.status}`} /><span>{task.name}</span><small>{statusLabel(task.status)}</small></div>)}</div>
      </section>
      <section className="panel"><div className="panel-title"><div><h3>快速开始</h3><p>连接游戏并检查环境</p></div></div>
        <ol className="steps"><li>在桌面启动第五人格。</li><li>在“设置”确认窗口标题。</li><li>启动引擎后，从“任务”开始执行。</li></ol>
      </section>
    </div>
  </>;
}

function Stat({ label, value }: { label: string; value: string }) { return <section className="stat"><span>{label}</span><strong>{value}</strong></section>; }

function Tasks({ tasks, onChange, onCommand }: { tasks: TaskItem[]; onChange: (tasks: TaskItem[]) => void; onCommand: (label: string, command: Record<string, unknown>) => void }) {
  const update = (id: string, patch: Partial<TaskItem>) => onChange(tasks.map((task) => task.id === id ? { ...task, ...patch } : task));
  const firstEnabled = tasks.find((task) => task.enabled);
  const startFirst = () => { if (!firstEnabled) return; update(firstEnabled.id, { status: "running" }); onCommand("启动已启用任务", { type: "start_task", task: firstEnabled.id }); };
  return <><div className="section-heading"><div><h2>自动任务</h2><p>任务由引擎调度。当前版本每次执行一个已启用任务。</p></div><button className="primary" disabled={!firstEnabled} onClick={startFirst}>启动已启用任务</button></div>
    <div className="task-list">{tasks.map((task) => <article className="task-card" key={task.id}><label className="switch"><input type="checkbox" checked={task.enabled} onChange={(event) => update(task.id, { enabled: event.target.checked })} /><span /></label><div className="task-copy"><h3>{task.name}</h3><p>{task.description}</p></div><span className={`tag ${task.status}`}>{statusLabel(task.status)}</span><button onClick={() => { const action = task.status === "running" ? "pause" : task.status === "paused" ? "resume" : "start"; update(task.id, { status: action === "pause" ? "paused" : "running" }); onCommand(action === "pause" ? "暂停任务" : action === "resume" ? "恢复任务" : "启动任务", action === "pause" ? { type: "pause_task" } : action === "resume" ? { type: "resume_task" } : { type: "start_task", task: task.id }); }}>{task.status === "running" ? "暂停" : task.status === "paused" ? "恢复" : "执行"}</button></article>)}</div>
  </>;
}

function Runtime({ snapshot, runtimeEvent, onRefresh, onCommand }: { snapshot: EngineSnapshot; runtimeEvent: EngineEvent | null; onRefresh: () => Promise<void>; onCommand: (label: string, command: Record<string, unknown>) => void }) {
  const [command, setCommand] = useState('{ "type": "ping" }');
  const [error, setError] = useState<string | null>(null);
  const send = () => { try { const parsed = JSON.parse(command) as Record<string, unknown>; setError(null); onCommand("发送运行时命令", parsed); } catch { setError("请输入有效的 JSON 对象。"); } };
  return <div className="runtime-grid"><section className="panel"><div className="panel-title"><div><h2>引擎快照</h2><p>每 5 秒自动刷新</p></div><button onClick={() => void onRefresh()}>立即刷新</button></div><pre className="code-block">{JSON.stringify(snapshot, null, 2)}</pre>{runtimeEvent && <><h3 className="event-title">最新任务事件</h3><pre className="code-block">{JSON.stringify(runtimeEvent, null, 2)}</pre></>}</section><section className="panel"><div className="panel-title"><div><h2>调度命令</h2><p>直接向本地引擎发送 JSON 指令</p></div></div><textarea className="command-input" value={command} onChange={(event) => setCommand(event.target.value)} spellCheck={false} />{error && <p className="form-error">{error}</p>}<button className="primary" onClick={send}>发送命令</button></section></div>;
}

function Debug({ windowInfo, frame, templateMatch, lastPlannedAction, developerInputEnabled, appendLog, setNotice }: { windowInfo: WindowInfo | null; frame: CaptureFrame | null; templateMatch: TemplateTestResult | null; lastPlannedAction: EngineEvent | null; developerInputEnabled: boolean; appendLog: (level: LogLevel, message: string, source?: string) => void; setNotice: (value: string) => void }) {
  const [templateName, setTemplateName] = useState("test/mock_template.pgm");
  const [busy, setBusy] = useState<string | null>(null);
  const request = async (label: string, action: () => Promise<unknown>) => { setBusy(label); try { await action(); appendLog("info", `${label}请求已发送，等待引擎事件。`, "debug"); setNotice(`${label}请求已发送，等待引擎事件`); } catch (error) { const message = `${label}失败：${asMessage(error)}`; appendLog("error", message, "debug"); setNotice(message); } finally { setBusy(null); } };
  const sendInput = async (label: string, action: Record<string, unknown>) => request(label, async () => { if (!developerInputEnabled) await bridge.inputTest({ type: "set_enabled", enabled: true }); await bridge.inputTest(action); });
  return <div className="debug-grid"><section className="panel"><h2>游戏窗口</h2><p>检查当前桌面上的目标窗口。</p><button className="primary" disabled={Boolean(busy)} onClick={() => void request("检测窗口", bridge.detectWindow)}>检测窗口</button>{windowInfo && <dl className="key-values"><dt>状态</dt><dd>{windowInfo.found ? "已找到" : "未找到"}</dd><dt>标题</dt><dd>{windowInfo.title || "—"}</dd><dt>区域</dt><dd>{windowInfo.client_rect ? `${windowInfo.client_rect.width ?? "?"} × ${windowInfo.client_rect.height ?? "?"}` : "—"}</dd><dt>捕获后端</dt><dd>{windowInfo.capture_backend ?? "—"}</dd></dl>}</section>
    <section className="panel"><h2>画面预览</h2><p>获取最近一帧，文件保存在引擎工作目录的 debug 文件夹。</p><button disabled={Boolean(busy)} onClick={() => void request("捕获画面", bridge.captureFrame)}>捕获画面</button><div className="preview">{frame?.imageUrl ? <img src={frame.imageUrl} alt="最近捕获的游戏画面" /> : frame ? <span>{frame.width} × {frame.height}<br />{frame.source ?? "unknown"}<br /><small>{frame.path}</small></span> : <span>尚未捕获画面</span>}</div></section>
    <section className="panel"><h2>模板识别</h2><p>填写 assets/templates 下的相对文件路径，结果会通过引擎事件显示。</p><label>模板路径<input value={templateName} onChange={(event) => setTemplateName(event.target.value)} placeholder="例如：lobby/start.png" /></label><button disabled={Boolean(busy) || !templateName.trim()} onClick={() => void request("测试模板", () => bridge.testTemplate(templateName.trim()))}>测试模板</button>{templateMatch && <dl className="key-values"><dt>模板</dt><dd>{templateMatch.template ?? "—"}</dd><dt>结果</dt><dd>{templateMatch.found ? "已匹配" : "未匹配"}</dd><dt>置信度</dt><dd>{templateMatch.confidence?.toFixed(3) ?? "—"}</dd></dl>}</section>
    <section className="panel"><h2>输入测试</h2><p>输入测试默认关闭。首次操作会先启用开发者输入模式；当前：{developerInputEnabled ? "已启用" : "未启用"}。</p><div className="button-row"><button disabled={Boolean(busy)} onClick={() => void sendInput("点击测试", { type: "click", x: 100, y: 100 })}>测试点击</button><button disabled={Boolean(busy)} onClick={() => void sendInput("按键测试", { type: "press", key: "esc" })}>测试 Esc</button></div>{lastPlannedAction && <dl className="key-values"><dt>最近计划操作</dt><dd>{lastPlannedAction.action ?? "—"}</dd><dt>模式</dt><dd>{lastPlannedAction.dry_run ? "模拟" : "真实输入"}</dd></dl>}</section></div>;
}

function Logs({ entries, onClear }: { entries: LogEntry[]; onClear: () => void }) {
  const [level, setLevel] = useState<"all" | LogLevel>("all");
  const [query, setQuery] = useState("");
  const shown = useMemo(() => entries.filter((entry) => (level === "all" || entry.level === level) && entry.message.toLocaleLowerCase().includes(query.toLocaleLowerCase())), [entries, level, query]);
  return <section className="panel logs-panel"><div className="log-toolbar"><div className="filters"><select value={level} onChange={(event) => setLevel(event.target.value as "all" | LogLevel)}><option value="all">全部级别</option><option value="debug">调试</option><option value="info">信息</option><option value="warning">警告</option><option value="error">错误</option></select><input placeholder="筛选日志" value={query} onChange={(event) => setQuery(event.target.value)} /></div><button onClick={onClear}>清空本页日志</button></div><div className="log-table"><div className="log-head"><span>时间</span><span>级别</span><span>来源</span><span>消息</span></div>{shown.length ? shown.map((entry) => <div className="log-row" key={entry.id}><time>{entry.at}</time><span><b className={`level ${entry.level}`}>{entry.level}</b></span><span>{entry.source}</span><span>{entry.message}</span></div>) : <p className="empty">没有匹配的日志。</p>}</div></section>;
}

function SettingsPage({ settings, setSettings, onSave }: { settings: Settings; setSettings: (value: Settings) => void; onSave: () => void }) {
  const updateApp = (patch: Partial<Settings["app"]>) => setSettings({ ...settings, app: { ...settings.app, ...patch } });
  const updateEngine = (patch: Partial<Settings["engine"]>) => setSettings({ ...settings, engine: { ...settings.engine, ...patch } });
  const updateWindow = (patch: Partial<Settings["game_window"]>) => setSettings({ ...settings, game_window: { ...settings.game_window, ...patch } });
  const updateCapture = (patch: Partial<Settings["capture"]>) => setSettings({ ...settings, capture: { ...settings.capture, ...patch } });
  const updateVision = (patch: Partial<Settings["vision"]>) => setSettings({ ...settings, vision: { ...settings.vision, ...patch } });
  const updateDebug = (patch: Partial<Settings["debug"]>) => setSettings({ ...settings, debug: { ...settings.debug, ...patch } });
  const keywords = (value: string) => value.split(/[,，]/).map((item) => item.trim()).filter(Boolean);
  return <section className="settings-form">
    <section className="panel"><h2>常规</h2><p>界面语言已预留扩展结构，当前内容以简体中文为主。</p>
      <label>界面主题<select value={settings.app.theme} onChange={(event) => updateApp({ theme: event.target.value as Settings["app"]["theme"] })}><option value="system">跟随系统</option><option value="light">浅色</option><option value="dark">深色</option></select></label>
      <label>界面语言<select value={settings.app.language} onChange={(event) => updateApp({ language: event.target.value as Settings["app"]["language"] })}><option value="zh-CN">简体中文</option><option value="en-US">English（预留）</option></select></label>
      <label className="check-label"><input type="checkbox" checked={settings.app.launch_engine_on_start} onChange={(event) => updateApp({ launch_engine_on_start: event.target.checked })} />启动应用时启动引擎</label>
    </section>
    <section className="panel"><h2>游戏窗口</h2><p>通过进程名、标题关键词和窗口类名定位游戏窗口。</p>
      <label>窗口标题关键词<input value={settings.game_window.title_contains.join("，")} onChange={(event) => updateWindow({ title_contains: keywords(event.target.value) })} placeholder="第五人格，Identity V" /></label>
      <label>进程名<input value={settings.game_window.process_names.join("，")} onChange={(event) => updateWindow({ process_names: keywords(event.target.value) })} placeholder="dwrg.exe，IdentityV.exe" /></label>
    </section>
    <section className="panel"><h2>引擎与捕获</h2><p>默认以安全的模拟模式运行，确认识别与输入链路后再关闭。</p>
      <label>心跳间隔（秒）<input type="number" min="0.5" step="0.5" value={settings.engine.heartbeat_interval_seconds} onChange={(event) => updateEngine({ heartbeat_interval_seconds: Number(event.target.value) })} /></label>
      <label>捕获后端<select value={settings.capture.backend} onChange={(event) => updateCapture({ backend: event.target.value })}><option value="auto">自动选择</option><option value="dxcam">DXcam</option><option value="mss">MSS</option></select></label>
      <label>空闲帧率<input type="number" min="1" max="60" value={settings.capture.idle_fps} onChange={(event) => updateCapture({ idle_fps: Number(event.target.value) })} /></label>
      <label className="check-label"><input type="checkbox" checked={settings.engine.dry_run} onChange={(event) => updateEngine({ dry_run: event.target.checked })} />模拟模式 <small>跳过真实输入操作</small></label>
    </section>
    <section className="panel"><h2>识别与诊断</h2>
      <label>模板匹配阈值<input type="number" min="0" max="1" step="0.01" value={settings.vision.template_threshold} onChange={(event) => updateVision({ template_threshold: Number(event.target.value) })} /></label>
      <label className="check-label"><input type="checkbox" checked={settings.debug.verbose_logs} onChange={(event) => updateDebug({ verbose_logs: event.target.checked })} />记录详细调试日志</label>
      <label className="check-label"><input type="checkbox" checked={settings.debug.save_debug_screenshots} onChange={(event) => updateDebug({ save_debug_screenshots: event.target.checked })} />保存调试截图</label>
    </section>
    <div className="save-bar"><button className="primary" onClick={onSave}>保存设置</button></div>
  </section>;
}

function About() { return <section className="panel about"><span className="large-mark">V</span><h2>第五人格助手</h2><p>本地运行的任务控制台，用于连接游戏窗口、调度引擎任务和查看诊断信息。</p><dl className="key-values"><dt>版本</dt><dd>0.1.0</dd><dt>界面</dt><dd>React + TypeScript</dd><dt>运行环境</dt><dd>Tauri</dd></dl></section>; }
