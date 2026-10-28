import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { bridge } from "./bridge";
import type { Settings } from "./types";
import "./updates.css";

export interface UpdateInfo {
  version: string;
  current_version: string;
  notes: string;
  date: string | null;
  sha256: string;
  size: number;
}
export interface UpdateStatus {
  phase: "idle" | "checking" | "available" | "current" | "downloading" | "verifying" | "ready" | "installing" | "error" | "cancelled";
  info: UpdateInfo | null;
  downloaded: number;
  total: number | null;
  error: string | null;
  source: "direct" | "proxy";
}
const initialStatus: UpdateStatus = { phase: "idle", info: null, downloaded: 0, total: null, error: null, source: "direct" };
const copy = {
  "zh-CN": {
    title: "软件更新", hint: "获取新功能与修复，安装前由你确认。", automatic: "启动时自动检查更新", source: "下载线路",
    direct: "GitHub 直连", proxy: "国内代理（GH-Proxy）", proxyHint: "代理由第三方提供；下载后仍需通过发布签名与 SHA-256 校验。",
    check: "检查更新", download: "下载更新", install: "安装并重启", cancel: "取消下载", retry: "重新检查", view: "查看更新", dismiss: "关闭更新通知",
    installHint: "安装会停止自动化服务、退出助手并打开更新安装向导。请先结束当前任务；个人配置和地图包会保留。",
    notes: "版本说明", verified: "发布签名与 SHA-256 校验通过", browser: "请在桌面应用中检查和安装更新。", checksum: "SHA-256 校验值",
    phases: { idle: "随时检查最新版本", checking: "正在检查更新…", available: "发现新版本", current: "已是最新版本", downloading: "正在下载更新", verifying: "正在校验下载文件…", ready: "更新已准备好", installing: "正在准备安装，请稍候…", error: "更新未完成", cancelled: "下载已取消" },
  },
  "en-US": {
    title: "Software updates", hint: "Get improvements and fixes. You choose when to install.", automatic: "Check for updates on startup", source: "Download route",
    direct: "GitHub direct", proxy: "China proxy (GH-Proxy)", proxyHint: "This proxy is a third-party service. Every download must pass release signature and SHA-256 verification.",
    check: "Check for updates", download: "Download update", install: "Install and restart", cancel: "Cancel download", retry: "Check again", view: "View update", dismiss: "Dismiss update notice",
    installHint: "Installing stops automation, closes the assistant, and opens the update installer. Finish your current task first. Your settings and map packs are kept.",
    notes: "Release notes", verified: "Release signature and SHA-256 verified", browser: "Check and install updates in the desktop app.", checksum: "SHA-256 checksum",
    phases: { idle: "Check for the latest version", checking: "Checking for updates…", available: "Update available", current: "You're up to date", downloading: "Downloading update", verifying: "Verifying download…", ready: "Ready to install", installing: "Preparing installation…", error: "Update did not finish", cancelled: "Download cancelled" },
  },
};
const megabytes = (bytes: number) => `${(bytes / 1048576).toFixed(1)} MB`;

export default function UpdatePanel({ settings, loaded, expanded, onSettingsChange, onOpen }: {
  settings: Settings; loaded: boolean; expanded: boolean;
  onSettingsChange: (app: Settings["app"]) => void; onOpen: () => void;
}) {
  const [status, setStatus] = useState(initialStatus);
  const [dismissed, setDismissed] = useState(false);
  const [pending, setPending] = useState(false);
  const [listening, setListening] = useState(false);
  const [connectionFailed, setConnectionFailed] = useState(false);
  const [connectionAttempt, setConnectionAttempt] = useState(0);
  const startupHandled = useRef(false);
  const mounted = useRef(true);
  const requestPending = useRef(false);
  const t = copy[settings.app.language];
  const desktop = isTauri();
  const source = settings.app.update_source;
  const busy = pending || ["checking", "downloading", "verifying", "installing"].includes(status.phase);

  const run = useCallback(async (action: () => Promise<unknown>) => {
    if (requestPending.current || !isTauri()) return;
    requestPending.current = true;
    setPending(true);
    try {
      await action();
      const next = await bridge.getUpdateStatus();
      if (mounted.current) setStatus(next);
    } catch (error) {
      if (mounted.current) setStatus((previous) => ({ ...previous, phase: "error", error: String(error) }));
    } finally {
      requestPending.current = false;
      if (mounted.current) setPending(false);
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    if (!desktop) return () => { mounted.current = false; };
    let disposed = false;
    let unlisten: (() => void) | undefined;
    setConnectionFailed(false);
    // Subscribe before reading the snapshot so a running download survives navigation.
    void bridge.onUpdateStatus((next) => { if (!disposed) setStatus(next); }).then(async (stop) => {
      if (disposed) { stop(); return; }
      unlisten = stop;
      const next = await bridge.getUpdateStatus();
      if (!disposed) { setStatus(next); setListening(true); }
    }).catch((error) => {
      if (!disposed) {
        setConnectionFailed(true);
        setStatus((previous) => ({ ...previous, phase: "error", error: String(error) }));
      }
    });
    return () => { disposed = true; mounted.current = false; unlisten?.(); };
  }, [desktop, connectionAttempt]);

  useEffect(() => {
    if (!loaded || !desktop || !listening || startupHandled.current) return;
    startupHandled.current = true;
    if (settings.app.check_updates_on_start) void run(() => bridge.checkUpdate(source));
  }, [loaded, desktop, listening, settings.app.check_updates_on_start, source, run]);

  useEffect(() => { setDismissed(false); }, [status.info?.version]);
  const check = () => {
    setDismissed(false);
    if (connectionFailed) { setConnectionAttempt((attempt) => attempt + 1); return; }
    void run(() => bridge.checkUpdate(source));
  };
  const progress = status.total && status.total > 0 ? Math.min(100, status.downloaded / status.total * 100) : undefined;
  const notice = status.info && !dismissed && ["available", "downloading", "verifying", "ready", "error"].includes(status.phase);
  if (!expanded) return notice ? <div className="update-notice" role="status">
    <span><strong>{t.phases[status.phase]}</strong> · v{status.info!.version}</span>
    <button onClick={onOpen}>{t.view}</button><button aria-label={t.dismiss} onClick={() => setDismissed(true)}>×</button>
  </div> : null;

  return <section className="panel update-panel" aria-labelledby="update-title">
    <header className="update-heading"><div><span className="eyebrow">IDENTITY V ASSISTANT</span><h2 id="update-title">{t.title}</h2><p>{t.hint}</p></div>
      <span className={`update-symbol update-${status.phase}`} aria-hidden="true">{status.phase === "ready" || status.phase === "current" ? "✓" : "↓"}</span>
    </header>
    <div className="update-preferences">
      <label className="check-label"><input type="checkbox" checked={settings.app.check_updates_on_start} onChange={(event) => onSettingsChange({ ...settings.app, check_updates_on_start: event.target.checked })} />{t.automatic}</label>
      <label>{t.source}<select disabled={busy} value={source} onChange={(event) => onSettingsChange({ ...settings.app, update_source: event.target.value as "direct" | "proxy" })}>
        <option value="direct">{t.direct}</option><option value="proxy">{t.proxy}</option>
      </select></label>
    </div>
    {source === "proxy" && <p className="update-hint">{t.proxyHint}</p>}
    <div className={`update-state update-state-${status.phase}`}>
      <strong role="status">{t.phases[status.phase]}</strong>
      {status.info && <div className="update-version"><span>v{status.info.current_version}</span><span aria-hidden="true">→</span><b>v{status.info.version}</b></div>}
      {status.phase === "error" && <p className="update-error" role="alert">{status.error}</p>}
      {status.phase === "downloading" && <div className="update-download">
        <progress max="100" value={progress} aria-label={t.phases.downloading} />
        <small>{megabytes(status.downloaded)}{status.total ? ` / ${megabytes(status.total)}` : ""}{progress !== undefined ? ` · ${Math.floor(progress)}%` : ""}</small>
      </div>}
      {status.phase === "ready" && <p className="update-verified">✓ {t.verified}</p>}
    </div>
    {status.info && <details className="update-notes" open><summary>{t.notes}</summary><p>{status.info.notes || `v${status.info.version}`}</p>
      <details><summary>{t.checksum}</summary><code>{status.info.sha256}</code></details>
    </details>}
    {status.phase === "ready" && <p className="update-hint">{t.installHint}</p>}
    {!desktop && <p className="update-hint">{t.browser}</p>}
    <div className="update-actions">
      <button disabled={busy || !desktop || !loaded || (!listening && !connectionFailed)} onClick={check}>{status.phase === "error" ? t.retry : t.check}</button>
      {status.info && ["available", "cancelled", "error"].includes(status.phase) && <button className="primary" disabled={busy} onClick={() => void run(() => bridge.downloadUpdate(source))}>{t.download}{status.info.size > 0 ? ` · ${megabytes(status.info.size)}` : ""}</button>}
      {status.phase === "downloading" && <button onClick={() => { void bridge.cancelUpdate().catch((error) => setStatus((previous) => ({ ...previous, phase: "error", error: String(error) }))); }}>{t.cancel}</button>}
      {status.phase === "ready" && <button className="primary" disabled={busy} onClick={() => void run(bridge.installUpdate)}>{t.install}</button>}
    </div>
  </section>;
}
