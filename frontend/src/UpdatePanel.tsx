import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { isTauri } from "@tauri-apps/api/core";
import { bridge } from "./bridge";
import type { Settings } from "./types";
import SettingRow, { SettingIcon } from "./SettingRow";
import { version as appVersion } from "../package.json";
import "./updates.css";

export interface UpdateInfo {
  version: string; current_version: string; notes: string; date: string | null; sha256: string; size: number;
}
export interface UpdateStatus {
  phase: "idle" | "checking" | "available" | "current" | "downloading" | "verifying" | "ready" | "installing" | "error" | "cancelled";
  info: UpdateInfo | null; downloaded: number; total: number | null; error: string | null; source: "direct" | "proxy";
}
const initialStatus: UpdateStatus = { phase: "idle", info: null, downloaded: 0, total: null, error: null, source: "direct" };
const skippedVersionKey = "identityv.update.dismissedVersion";
const releaseUrl = "https://github.com/WaltomAdaam2/desktop-releases/releases";
const megabytes = (bytes: number) => `${(bytes / 1048576).toFixed(1)} MB`;
const copy = {
  "zh-CN": {
    currentVersion: "当前版本", channel: "检查范围", stable: "正式版本", preview: "正式及预发布版本", done: "知道了", sourceHint: "默认使用 GitHub 直连；连接不畅时可选择 GH-Proxy。", manualHint: "检查所选通道的新版本，并查看更新内容。", title: "软件更新", prerelease: "接收预发布版本", prereleaseHint: "提前体验新功能，可能存在尚未修复的问题。", source: "更新源", direct: "GitHub 直连", proxy: "GH-Proxy", check: "检查更新", manual: "手动检查", later: "稍后", release: "发布页面", download: "下载更新", install: "安装并重启", cancel: "取消下载", retry: "重试", skip: "此版本下次打开不再提示", notes: "更新日志", empty: "此版本暂无更新日志。", close: "关闭", preferenceError: "无法保存提示偏好，请重试。", verified: "发布签名与 SHA-256 校验通过", installHint: "安装会停止当前任务并关闭助手。配置、日志与地图包将保留。", browser: "请在桌面应用中检查更新。",
    hints: { idle: "准备查询所选更新源。", checking: "正在获取发布信息，请稍候。", current: "检查完成，所选通道暂无可用的新版本。", available: "查看更新内容后下载，安装前仍需你确认。", downloading: "下载后将自动校验发布签名、版本与 SHA-256。", verifying: "正在检查安装包的真实性与完整性。", ready: "安装包已校验，可以开始安装。", installing: "正在关闭助手并启动安装程序。", error: "未能完成更新，请检查网络或更换更新源后重试。", cancelled: "下载已停止。你可以稍后重新下载。" },
    phases: { idle: "软件更新", checking: "正在检查更新…", available: "发现新版本", current: "已是最新版本", downloading: "正在下载更新", verifying: "正在校验文件…", ready: "更新已准备好", installing: "正在准备安装…", error: "更新未完成", cancelled: "下载已取消" },
  },
  "en-US": {
    currentVersion: "Installed version", channel: "Release channel", stable: "Stable releases", preview: "Stable and prereleases", done: "Done", sourceHint: "GitHub direct is the default. Use GH-Proxy if the connection is slow.", manualHint: "Check the selected channel and review release notes.", title: "Software updates", prerelease: "Receive prerelease versions", prereleaseHint: "Try new features early. These versions may contain unresolved issues.", source: "Update source", direct: "GitHub direct", proxy: "GH-Proxy", check: "Check for updates", manual: "Manual check", later: "Later", release: "Release page", download: "Download update", install: "Install and restart", cancel: "Cancel download", retry: "Retry", skip: "Do not show this version on startup again", notes: "Release notes", empty: "No release notes for this version.", close: "Close", preferenceError: "Could not save this preference. Please try again.", verified: "Release signature and SHA-256 verified", installHint: "Installing stops the current task and closes the assistant. Settings, logs, and maps are kept.", browser: "Check for updates in the desktop app.",
    hints: { idle: "Ready to query the selected update source.", checking: "Retrieving release information. Please wait.", current: "Check complete. No newer release is available on this channel.", available: "Review the changes before downloading. Installation needs your confirmation.", downloading: "The download will be checked for its signature, version and SHA-256.", verifying: "Checking the installer authenticity and integrity.", ready: "The installer is verified and ready.", installing: "Closing the assistant and starting the installer.", error: "The update could not finish. Check your connection or choose another source and retry.", cancelled: "Download stopped. You can download it again later." },
    phases: { idle: "Software updates", checking: "Checking for updates…", available: "Update available", current: "You're up to date", downloading: "Downloading update", verifying: "Verifying files…", ready: "Ready to install", installing: "Preparing installation…", error: "Update did not finish", cancelled: "Download cancelled" },
  },
};

export default function UpdatePanel({ settings, loaded, expanded, onSettingsChange }: {
  settings: Settings; loaded: boolean; expanded: boolean;
  onSettingsChange: (app: Settings["app"]) => void; onOpen: () => void;
}) {
  const [status, setStatus] = useState(initialStatus);
  const [checkedPrereleases, setCheckedPrereleases] = useState(settings.app.update_prereleases);
  const [pending, setPending] = useState(false);
  const [listening, setListening] = useState(false);
  const [connectionAttempt, setConnectionAttempt] = useState(0);
  const [connectionFailed, setConnectionFailed] = useState(false);
  const [showPrompt, setShowPrompt] = useState(false);
  const [skipVersion, setSkipVersion] = useState(false);
  const [preferenceError, setPreferenceError] = useState(false);
  const startupHandled = useRef(false);
  const promptCheck = useRef<"startup" | "manual" | null>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const mounted = useRef(true);
  const requestPending = useRef(false);
  const desktop = isTauri();
  const t = copy[settings.app.language];
  const source = settings.app.update_source;
  const busy = pending || ["checking", "downloading", "verifying", "installing"].includes(status.phase);

  const run = useCallback(async (action: () => Promise<unknown>, checking = false) => {
    if (requestPending.current || !isTauri()) return;
    requestPending.current = true; setPending(true);
    let timer: number | undefined;
    try {
      const operation = action().then(() => bridge.getUpdateStatus());
      const next = checking ? await Promise.race([
        operation,
        new Promise<never>((_, reject) => {
          timer = window.setTimeout(() => reject(new Error("更新检查超时，请重试或更换更新源。 / Update check timed out.")), 60_000);
        }),
      ]) : await operation;
      if (mounted.current) setStatus(next);
    } catch (error) {
      if (mounted.current) setStatus((previous) => ({ ...previous, phase: "error", error: String(error) }));
    } finally { window.clearTimeout(timer); requestPending.current = false; if (mounted.current) setPending(false); }
  }, []);

  useEffect(() => {
    mounted.current = true;
    if (!desktop) return () => { mounted.current = false; };
    let disposed = false;
    let unlisten: (() => void) | undefined;
    setConnectionFailed(false);
    void bridge.onUpdateStatus((next) => { if (!disposed) setStatus(next); }).then(async (stop) => {
      if (disposed) { stop(); return; }
      unlisten = stop;
      const next = await bridge.getUpdateStatus();
      if (!disposed) {
        setStatus(next); setListening(true);
        if (connectionAttempt > 0 && promptCheck.current === "manual") void run(() => bridge.checkUpdate(source, settings.app.update_prereleases), true);
      }
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
    promptCheck.current = "startup";
    setCheckedPrereleases(settings.app.update_prereleases);
    void run(() => bridge.checkUpdate(source, settings.app.update_prereleases), true);
  }, [loaded, desktop, listening, source, settings.app.update_prereleases, run]);

  useEffect(() => {
    if (!promptCheck.current || pending) return;
    if (status.phase === "available" && status.info) {
      let skipped = false;
      try { skipped = localStorage.getItem(skippedVersionKey) === status.info.version; } catch { /* Still show updates when storage is unavailable. */ }
      if (promptCheck.current === "manual" || !skipped) { setShowPrompt(true); setSkipVersion(skipped); setPreferenceError(false); }
      promptCheck.current = null;
    } else if (["current", "error"].includes(status.phase)) promptCheck.current = null;
  }, [status, pending]);

  useEffect(() => { if (showPrompt && dialog.current && !dialog.current.open) dialog.current.showModal(); }, [showPrompt]);
  const closePrompt = () => { promptCheck.current = null; dialog.current?.close(); setShowPrompt(false); };
  const savePromptPreference = (checked: boolean) => {
    if (!status.info) return;
    try {
      if (checked) localStorage.setItem(skippedVersionKey, status.info.version);
      else localStorage.removeItem(skippedVersionKey);
      setSkipVersion(checked); setPreferenceError(false);
    } catch { setPreferenceError(true); }
  };
  const check = () => {
    setShowPrompt(true); setPreferenceError(false); promptCheck.current = "manual";
    setCheckedPrereleases(settings.app.update_prereleases);
    setStatus((previous) => ({ ...previous, phase: "checking", info: null, error: null, source }));
    if (connectionFailed) { setConnectionAttempt((attempt) => attempt + 1); return; }
    void run(() => bridge.checkUpdate(source, settings.app.update_prereleases), true);
  };
  const progress = status.total && status.total > 0 ? Math.min(100, status.downloaded / status.total * 100) : undefined;
  const modal = showPrompt ? createPortal(<dialog ref={dialog} className={`update-dialog${status.info ? " has-release" : ""}`} aria-labelledby="update-dialog-title" onCancel={(event) => { event.preventDefault(); closePrompt(); }}>
    <button className="update-dialog-close" aria-label={t.close} onClick={closePrompt}>×</button>
    <header className={`update-dialog-heading phase-${status.phase}`}>
      <span className="update-state-icon" aria-hidden="true">{status.phase === "current" || status.phase === "ready" ? <svg viewBox="0 0 24 24"><path d="m5 12 4 4L19 6" /></svg> : status.phase === "error" ? <svg viewBox="0 0 24 24"><path d="M12 5v9m0 4h.01" /></svg> : <SettingIcon name="refresh" />}</span>
      <div><h2 id="update-dialog-title">{t.phases[status.phase]}</h2><p>{t.hints[status.phase]}</p></div>
    </header>
    <dl className="update-facts">
      <div><dt>{t.currentVersion}</dt><dd>v{status.info?.current_version ?? appVersion}{status.info && <><span className="update-version-arrow"> → </span><strong>v{status.info.version}</strong></>}</dd></div>
      <div><dt>{t.source}</dt><dd>{status.source === "proxy" ? t.proxy : t.direct}</dd></div>
      <div><dt>{t.channel}</dt><dd>{checkedPrereleases ? t.preview : t.stable}</dd></div>
    </dl>
    {status.info && <section className="update-dialog-notes" aria-label={t.notes}><h3>{t.notes}</h3><p>{status.info.notes || t.empty}</p></section>}
    {status.phase === "error" && <details className="update-error"><summary>{settings.app.language === "zh-CN" ? "错误详情" : "Error details"}</summary><p role="alert">{status.error}</p></details>}
    {status.phase === "downloading" && <div className="update-download"><progress max="100" value={progress} aria-label={t.phases.downloading} /><small>{megabytes(status.downloaded)}{status.total ? ` / ${megabytes(status.total)}` : ""}{progress !== undefined ? ` · ${Math.floor(progress)}%` : ""}</small></div>}
    {status.phase === "ready" && <><p className="update-verified">✓ {t.verified}</p><p>{t.installHint}</p></>}
    {status.info && <label className="check-label update-skip"><input type="checkbox" checked={skipVersion} onChange={(event) => savePromptPreference(event.target.checked)} />{t.skip}</label>}
    {preferenceError && <p role="alert" className="update-error">{t.preferenceError}</p>}
    <footer className="update-actions">
      <a href={releaseUrl} target="_blank" rel="noreferrer" onClick={(event) => { if (!desktop) return; event.preventDefault(); void bridge.openRepository().catch((error) => setStatus((previous) => ({ ...previous, phase: "error", error: String(error) }))); }}>{t.release}</a>
      <button autoFocus className={status.phase === "current" ? "primary" : undefined} onClick={closePrompt}>{status.phase === "current" ? t.done : status.info ? t.later : t.close}</button>
      {status.info && ["available", "cancelled", "error"].includes(status.phase) && <button className="primary" disabled={busy} onClick={() => void run(() => bridge.downloadUpdate(source))}>{t.download} · {megabytes(status.info.size)}</button>}
      {status.phase === "downloading" && <button onClick={() => { void bridge.cancelUpdate().catch((error) => setStatus((previous) => ({ ...previous, phase: "error", error: String(error) }))); }}>{t.cancel}</button>}
      {status.phase === "ready" && <button className="primary" disabled={busy} onClick={() => void run(bridge.installUpdate)}>{t.install}</button>}
      {status.phase === "error" && !status.info && <button disabled={busy} onClick={check}>{t.retry}</button>}
    </footer>
  </dialog>, document.body) : null;

  return <>{modal}{expanded && <section className="panel update-panel" aria-labelledby="update-title">
    <h2 id="update-title">{t.title}</h2>
    <SettingRow icon="refresh" title={t.prerelease} description={t.prereleaseHint} language={settings.app.language}><input type="checkbox" role="switch" disabled={busy} checked={settings.app.update_prereleases} onChange={(event) => onSettingsChange({ ...settings.app, update_prereleases: event.target.checked })} /></SettingRow>
    <SettingRow icon="cloud" title={t.source} description={t.sourceHint} language={settings.app.language}><select disabled={busy} value={source} onChange={(event) => onSettingsChange({ ...settings.app, update_source: event.target.value as "direct" | "proxy" })}><option value="direct">{t.direct}</option><option value="proxy">{t.proxy}</option></select></SettingRow>
    <SettingRow icon="updateCheck" title={t.manual} description={t.manualHint} language={settings.app.language}><button disabled={(busy && !["downloading", "verifying", "installing"].includes(status.phase)) || !desktop || !loaded || (!listening && !connectionFailed)} onClick={() => { if (["ready", "downloading", "verifying", "installing"].includes(status.phase)) setShowPrompt(true); else check(); }}>{t.check}</button></SettingRow>
    {!desktop && <p>{t.browser}</p>}
  </section>}</>;
}
