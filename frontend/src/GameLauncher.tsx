import { useEffect, useState } from "react";
import { bridge, type Announcement, type LauncherStatus } from "./bridge";
import type { Settings } from "./types";
import Icon from "./Icon";
import "./launcher.css";

export default function GameLauncher({ language, executablePath, onExecutableChange }: {
  language: Settings["app"]["language"];
  executablePath: string;
  onExecutableChange: (path: string) => void;
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
  const [setup, setSetup] = useState(false);
  const busy = launching || status?.phase === "starting" || status?.phase === "stopping";
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
    if (!status?.component_path) { setSetup(true); return; }
    setLaunching(true);
    setLaunchError("");
    try {
      await bridge.launchGame(gamePath);
      setStatus(await bridge.getLauncherStatus());
    } catch (error) { setLaunchError(String(error)); }
    finally { setLaunching(false); }
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
    <img className="launcher-logo" src="/launcher/logo.png" alt={zh ? "第五人格" : "Identity V"} draggable={false} />
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
          <div className="launcher-setup-buttons"><button disabled={choosing || busy || status?.phase === "ready"} onClick={() => void chooseComponent()}>{zh ? "选择登录组件" : "Choose component"}</button><button onClick={() => void bridge.openLoginComponentDownload().catch((error) => setLaunchError(String(error)))}>{zh ? "官方下载" : "Official download"}</button></div>
          <p className="launcher-setup-note">{zh ? "安装后选择 python-embed 文件夹中的 python.exe。首次使用需本人登录，之后由组件保留账号。" : "After installation, choose python.exe in python-embed. Sign in yourself on first use; the component remembers accounts."}</p>
          <p className="launcher-setup-note">{zh ? "组件会安装登录所需证书。退出助手会关闭自有代理；已打开的游戏会保留。" : "The component installs a certificate for login. Exiting the assistant closes its proxy and leaves the game open."}</p>
        </section>}
        <p className="launcher-game-path" title={gamePath} role="status">{locating ? (zh ? "正在定位游戏…" : "Locating game…") : gamePath || (zh ? "请选择游戏程序" : "Choose the game executable")}</p>
        {gameError && <p className="launcher-game-error" role="alert">{gameError}</p>}
        <p className="launcher-status" role="status">{busy ? (zh ? "正在启动登录组件…" : "Starting login component…") : status?.phase === "ready" ? (zh ? "登录代理已就绪" : "Login proxy ready") : !status?.component_path ? (zh ? "请先配置官方登录组件" : "Set up the official login component") : (zh ? "直接启动第五人格" : "Launch Identity V directly")}</p>
        {(launchError || statusError || status?.message) && <p className="launcher-game-error" role="alert">{launchError || statusError || status?.message}</p>}
        <div className="launcher-launch-controls">
          <button className="launcher-start" disabled={busy || choosing || locating || !gamePath || !status || !!statusError} onClick={() => void launch()}>{busy ? (zh ? "正在启动…" : "Starting…") : (zh ? "启动游戏" : "Launch game")}</button>
          <button className="launcher-choose" onClick={() => setSetup((open) => !open)} aria-expanded={setup} aria-label={zh ? "启动设置" : "Launch settings"} title={zh ? "启动设置" : "Launch settings"}><Icon name="settings" /></button>
        </div>
      </div>
    </div>
  </section>;
}
