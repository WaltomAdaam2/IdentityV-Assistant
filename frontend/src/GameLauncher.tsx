import { useEffect, useRef, useState } from "react";
import { bridge, type Announcement, type LauncherStatus, type ComponentUpdateInfo } from "./bridge";
import ComponentUpdateDialog from "./ComponentUpdateDialog";
import type { Settings } from "./types";
import Icon from "./Icon";
import "./launcher.css";

export default function GameLauncher({ language, appSettings, onAppSettingsChange, executablePath, onExecutableChange }: {
  language: Settings["app"]["language"];
  executablePath: string;
  onExecutableChange: (path: string) => void;
  appSettings: Settings["app"];
  onAppSettingsChange: (app: Settings["app"]) => void;
}) {
  const zh = language === "zh-CN";
  const [news, setNews] = useState<Announcement[]>([]);
  const [newsState, setNewsState] = useState<"loading" | "ready" | "error">("loading");
  const [retry, setRetry] = useState(0);
  const [linkError, setLinkError] = useState(false);
  const [gamePath, setGamePath] = useState("");
  const [locating, setLocating] = useState(true);
  const [choosing, setChoosing] = useState(false);
  const [gameError, setGameError] = useState("");
  const [status, setStatus] = useState<LauncherStatus | null>(null);
  const [statusError, setStatusError] = useState("");
  const [launchError, setLaunchError] = useState("");
  const [launching, setLaunching] = useState(false);
  const [configuring, setConfiguring] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const [setup, setSetup] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<ComponentUpdateInfo | null>(null);
  const [updateError, setUpdateError] = useState("");
  const [updatePhase, setUpdatePhase] = useState<"checking" | "applying" | "idle">("idle");
  const [updateOpen, setUpdateOpen] = useState(false);
  const continueAfterUpdate = useRef(false);
  const starting = launching || (updatePhase === "checking" && continueAfterUpdate.current) || status?.phase === "starting";
  const installing = configuring || status?.phase === "configuring";
  const busy = starting || installing || updatePhase !== "idle" || status?.phase === "stopping";
  const proxyPhase = cancelling ? "stopping" : updatePhase !== "idle" ? "updating" : statusError ? "error" : status?.proxy_phase || status?.phase || "checking";
  const proxyLabels: Record<string, [string, string]> = {
    idle: ["登录代理未启动", "Login proxy stopped"],
    starting: ["正在启动代理", "Starting proxy"],
    configuring: ["正在配置组件", "Configuring component"],
    initializing: ["代理初始化中", "Initializing proxy"],
    updating: ["检查组件更新", "Checking component updates"],
    restarting: ["代理重启中", "Restarting proxy"],
    checking: ["正在检查代理", "Checking proxy"],
    ready: ["登录代理已就绪", "Login proxy ready"],
    stopping: ["代理停止中", "Stopping proxy"],
    error: ["登录代理不可用", "Login proxy unavailable"],
  };
  const proxyLabel = !status?.component_path && proxyPhase === "idle"
    ? (zh ? "请先配置官方登录组件" : "Set up the official login component")
    : (proxyLabels[proxyPhase] || proxyLabels.checking)[zh ? 0 : 1];
  const proxyDot = proxyPhase === "ready" ? "running" : proxyPhase === "error" ? "error" : proxyPhase === "idle" ? "stopped" : "starting";
  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const next = await bridge.getLauncherStatus();
        if (active) { setStatus(next); setStatusError(""); }
      } catch (error) { if (active) setStatusError(String(error)); }
      finally { if (active) timer = setTimeout(() => void poll(), 1000); }
    };
    void poll();
    return () => { active = false; clearTimeout(timer); };
  }, []);
  const launch = async () => {
    if (starting || installing) {
      setCancelling(true);
      try { await bridge.cancelGameLaunch(); }
      catch (error) { setLaunchError(String(error)); }
      finally { setCancelling(false); }
      return;
    }
    if (!status?.component_path) { setSetup(true); return; }
    await checkUpdates(true);
  };
  const startGame = async () => {
    setLaunching(true);
    setLaunchError("");
    setStatus((previous) => previous && { ...previous, phase: "starting", proxy_phase: "starting", message: null });
    let message = "";
    try { await bridge.launchGame(gamePath); }
    catch (error) { message = String(error); }
    try { setStatus(await bridge.getLauncherStatus()); }
    catch (error) { setStatusError(String(error)); }
    setLaunchError(message);
    setLaunching(false);
  };
  const checkUpdates = async (forLaunch: boolean) => {
    continueAfterUpdate.current = forLaunch;
    setUpdatePhase("checking"); setUpdateError(""); setUpdateInfo(null); setLaunchError("");
    if (!forLaunch) setUpdateOpen(true);
    try {
      const info = await bridge.checkLoginUpdates(appSettings.update_source);
      setUpdateInfo(info);
      if (info.release_url || info.hotfixes.length || !forLaunch) setUpdateOpen(true);
      else { setUpdatePhase("idle"); continueAfterUpdate.current = false; await startGame(); }
    } catch (error) {
      if (String(error).includes("取消")) { continueAfterUpdate.current = false; setLaunchError(String(error)); }
      else { setUpdateError(String(error)); setUpdateOpen(true); }
    } finally { setUpdatePhase("idle"); }
  };
  const closeUpdate = (resume = false) => {
    setUpdateOpen(false);
    const pending = continueAfterUpdate.current;
    continueAfterUpdate.current = false;
    if (pending && resume) void startGame();
  };
  const applyHotfixes = async () => {
    setUpdatePhase("applying"); setUpdateError("");
    try {
      await bridge.applyLoginHotfixes(appSettings.update_source);
      setUpdateInfo((info) => info && {...info, hotfixes: []});
      if (continueAfterUpdate.current) closeUpdate(true);
    } catch (error) { setUpdateInfo(null); setUpdateError(String(error)); }
    finally { setUpdatePhase("idle"); }
  };
  const configureComponent = async (download: boolean) => {
    setConfiguring(true);
    setLaunchError("");
    try { await bridge.configureLoginComponent(download); }
    catch (error) { setLaunchError(String(error)); }
    finally {
      try { setStatus(await bridge.getLauncherStatus()); }
      catch (error) { setStatusError(String(error)); }
      setConfiguring(false);
    }
  };
  const chooseComponent = async () => {
    setChoosing(true);
    setLaunchError("");
    try {
      const selected = await bridge.chooseLoginComponent();
      if (selected) {
        await bridge.setLoginComponent(selected);
        setStatus(await bridge.getLauncherStatus());
      }
    } catch (error) { setLaunchError(String(error)); }
    finally { setChoosing(false); }
  };
  useEffect(() => {
    let active = true;
    setLocating(true);
    setGameError("");
    bridge.locateGame(executablePath).then((path) => {
      if (active) setGamePath(path);
    }).catch((error) => {
      if (active) { setGamePath(""); setGameError(String(error)); }
    }).finally(() => { if (active) setLocating(false); });
    return () => { active = false; };
  }, [executablePath]);
  const chooseGame = async () => {
    setChoosing(true);
    try {
      const selected = await bridge.chooseGameExecutable();
      if (selected) {
        const path = await bridge.locateGame(selected);
        setGamePath(path);
        setGameError("");
        onExecutableChange(path);
      }
    } catch (error) { setGameError(String(error)); }
    finally { setChoosing(false); }
  };
  useEffect(() => {
    let active = true;
    setNewsState("loading");
    bridge.getGameAnnouncements().then((items) => {
      if (active) { setNews(items); setNewsState("ready"); }
    }).catch(() => { if (active) setNewsState("error"); });
    return () => { active = false; };
  }, [retry]);
  const openNews = async (url: string) => {
    setLinkError(false);
    try { await bridge.openGameAnnouncement(url); }
    catch { setLinkError(true); }
  };
  return <section className="game-launcher" aria-label={zh ? "游戏启动" : "Game launch"}>
    {updateOpen && <ComponentUpdateDialog info={updateInfo} error={updateError} phase={updatePhase} source={appSettings.update_source} language={language} continuing={continueAfterUpdate.current}
      onSource={(source) => onAppSettingsChange({...appSettings, update_source: source})} onClose={() => closeUpdate()} onContinue={() => closeUpdate(true)} onCheck={() => void checkUpdates(continueAfterUpdate.current)} onApply={() => void applyHotfixes()}
      onDownload={() => void bridge.openLoginUpdate(appSettings.update_source).catch((error) => setUpdateError(String(error)))} />}
    <img className="launcher-logo" src="/launcher/logo.png" alt={zh ? "第五人格" : "Identity V"} draggable={false} />
    <div className="engine-status launcher-proxy-status launcher-status" role="status" aria-live="polite" title={status?.message || undefined}>
      <span className={`dot ${proxyDot}`} aria-hidden="true" /><span className="launcher-proxy-label" key={proxyPhase}>{proxyLabel}</span>
    </div>
    <div className="launcher-bottom">
      <section className="launcher-news" aria-labelledby="launcher-news-title">
        <div className="launcher-news-heading">
          <h2 id="launcher-news-title">{zh ? "活动与公告" : "News and events"}</h2>
          <button onClick={() => void openNews("https://id5.163.com/news/official/")}>{zh ? "更多" : "More"}</button>
        </div>
        {newsState === "loading" && <p role="status">{zh ? "正在加载公告…" : "Loading announcements…"}</p>}
        {newsState === "error" && <div role="status"><p>{zh ? "公告暂时无法加载，请检查网络后重试。" : "Announcements are unavailable. Check your connection and retry."}</p><button className="launcher-news-retry" onClick={() => setRetry((value) => value + 1)}>{zh ? "重试" : "Retry"}</button></div>}
        {newsState === "ready" && (news.length ? <ul>{news.map((item) => <li key={item.url}>
          <button title={item.title} onClick={() => void openNews(item.url)}>{item.title}</button>
          <time>{new Date(item.release_time * 1000).toLocaleDateString(language, { month: "2-digit", day: "2-digit", timeZone: "Asia/Shanghai" })}</time>
        </li>)}</ul> : <p>{zh ? "暂无公告" : "No announcements yet"}</p>)}
        {linkError && <p role="alert">{zh ? "无法打开公告，请检查默认浏览器。" : "Could not open the announcement. Check your default browser."}</p>}
      </section>
      <div className="launcher-actions">
        {setup && <section className="launcher-setup" aria-labelledby="launcher-setup-title" onKeyDown={(event) => { if (event.key === "Escape") setSetup(false); }}>
          <div className="launcher-setup-heading"><h2 id="launcher-setup-title">{zh ? "启动设置" : "Launch settings"}</h2><button onClick={() => setSetup(false)} aria-label={zh ? "关闭启动设置" : "Close launch settings"}>×</button></div>
          <p>{zh ? "第五人格游戏程序" : "Identity V executable"}</p>
          <div className="launcher-setup-path" title={gamePath}>{gamePath || (zh ? "未找到游戏" : "Game not found")}</div>
          <button disabled={choosing || locating || busy} onClick={() => void chooseGame()}>{zh ? "选择游戏程序" : "Choose game executable"}</button>
          <p>{zh ? "官方登录组件 · v6.3.2-stable" : "Official login component · v6.3.2-stable"}</p>
          <div className="launcher-setup-path" title={status?.component_path || undefined}>{status?.component_path || (zh ? "尚未设置" : "Not configured")}</div>
          <div className="launcher-setup-buttons"><button disabled={choosing || busy || status?.phase === "ready"} onClick={() => void chooseComponent()}>{zh ? "选择登录组件" : "Choose component"}</button><button disabled={choosing || busy || status?.phase === "ready"} onClick={() => void configureComponent(true)}>{zh ? "官方下载" : "Official download"}</button><button disabled={choosing || busy || status?.phase === "ready"} onClick={() => void configureComponent(false)}>{zh ? "自动配置" : "Auto configure"}</button></div>
          <p className="launcher-setup-note">{zh ? "自动配置会扫描已安装的组件。未找到时，官方下载会按设置中的下载源下载到助手程序目录并自动配置；也可手动选择包含“点我启动工具.bat”的文件夹。首次使用需本人登录，之后由组件保留账号。" : "Auto configure scans installed components. Official download uses your selected download source, extracts into the assistant directory and configures it. You can also select the folder containing the startup BAT. Sign in yourself on first use; the component remembers accounts."}</p>
          <p className="launcher-setup-note">{zh ? "组件会安装登录所需证书。退出助手会关闭自有代理；已打开的游戏会保留。" : "The component installs a certificate for login. Exiting the assistant closes its proxy and leaves the game open."}</p>
          <button disabled={busy || !status?.component_path} onClick={() => void checkUpdates(false)}>{zh ? "检查组件更新" : "Check component updates"}</button>
        </section>}
        <p className="launcher-game-path" title={gamePath} role="status">{locating ? (zh ? "正在定位游戏…" : "Locating game…") : gamePath || (zh ? "请选择游戏程序" : "Choose the game executable")}</p>
        {gameError && <p className="launcher-game-error" role="alert">{gameError}</p>}
        {(launchError || statusError || (!busy && status?.message)) && <p className="launcher-game-error" role="alert">{launchError || statusError || status?.message}</p>}
        <div className="launcher-launch-controls">
          <button className="launcher-start" disabled={cancelling || status?.phase === "stopping" || (!starting && !installing && (choosing || locating || !gamePath || !status || !!statusError))} onClick={() => void launch()}>{cancelling ? (zh ? "正在取消…" : "Cancelling…") : installing ? (zh ? "取消配置" : "Cancel setup") : starting ? (zh ? "取消启动" : "Cancel launch") : (zh ? "启动游戏" : "Launch game")}</button>
          <button className="launcher-choose" onClick={() => setSetup((open) => !open)} aria-expanded={setup} aria-label={zh ? "启动设置" : "Launch settings"} title={zh ? "启动设置" : "Launch settings"}><Icon name="settings" /></button>
        </div>
      </div>
    </div>
  </section>;
}
