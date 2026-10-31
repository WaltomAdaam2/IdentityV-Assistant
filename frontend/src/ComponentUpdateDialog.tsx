import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import type { ComponentUpdateInfo, ComponentUpdateProgress } from "./bridge";
import DownloadProgress from "./DownloadProgress";
import type { Settings } from "./types";
import { SettingIcon } from "./SettingRow";
import "./updates.css";

export default function ComponentUpdateDialog({ info, error, phase, progress, source, language, continuing, onSource, onClose, onContinue, onCheck, onApply, onDownload }: {
  info: ComponentUpdateInfo | null; error: string; phase: "checking" | "applying" | "idle";
  progress: ComponentUpdateProgress | null;
  source: Settings["app"]["update_source"]; language: Settings["app"]["language"]; continuing: boolean;
  onSource: (source: Settings["app"]["update_source"]) => void; onClose: () => void; onContinue: () => void; onCheck: () => void;
  onApply: () => void; onDownload: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const zh = language === "zh-CN";
  const available = !!info && (!!info.release_url || info.hotfixes.length > 0);
  const busy = phase !== "idle";
  const state = error ? "error" : busy ? phase === "checking" ? "checking" : "installing" : available ? "available" : "current";
  const labels = zh ? {checking:"正在检查组件更新…", installing:"正在应用组件更新…", error:"组件更新未完成", available:"发现组件更新", current:"组件已是最新版本"} : {checking:"Checking component updates…", installing:"Applying component updates…", error:"Component update failed", available:"Component update available", current:"Component is up to date"};
  useEffect(() => { dialog.current?.showModal(); }, []);
  return createPortal(<dialog ref={dialog} className={`update-dialog${available ? " has-release" : ""}`} aria-labelledby="component-update-title" onCancel={(event) => {event.preventDefault(); if (!busy) onClose();}}>
    <button className="update-dialog-close" disabled={busy} aria-label={zh ? "关闭组件更新" : "Close component update"} onClick={onClose}>×</button>
    <header className={`update-dialog-heading phase-${state}`}>
      <span className="update-state-icon" aria-hidden="true">{state === "current" ? <svg viewBox="0 0 24 24"><path d="m5 12 4 4L19 6" /></svg> : state === "error" ? <svg viewBox="0 0 24 24"><path d="M12 5v9m0 4h.01" /></svg> : <SettingIcon name="refresh" />}</span>
      <div><h2 id="component-update-title">{labels[state]}</h2><p>{zh ? "官方登录组件 · 更新前由你确认。" : "Official login component · Updates require your confirmation."}</p></div>
    </header>
    <dl className="update-facts">
      <div><dt>{zh ? "当前版本" : "Installed version"}</dt><dd>{info?.current_version || "—"}{info?.release_url && <><span className="update-version-arrow"> → </span><strong>{info.version}</strong></>}</dd></div>
      <div><dt><label htmlFor="component-update-source">{zh ? "更新源" : "Update source"}</label></dt><dd><select id="component-update-source" disabled={busy} value={source} onChange={(event) => onSource(event.target.value as Settings["app"]["update_source"])}>
        <option value="direct">{zh ? "GitHub 直连（默认）" : "GitHub direct (default)"}</option><option value="proxy">GH-Proxy</option><option value="ghfast">GHFast</option><option value="ghproxy_net">ghproxy.net</option>
      </select></dd></div>
    </dl>
    {available && <section className="update-dialog-notes" aria-label={zh ? "更新日志" : "Release notes"}><h3>{zh ? "更新日志" : "Release notes"}</h3>{info.notes && <p>{info.notes}</p>}{info.hotfixes.map((item) => <p key={`${item.target_module}@${item.target_commit}`}><strong>{item.target_module}</strong> · {item.target_commit.slice(0, 8)}<br />{item.note}</p>)}</section>}
    {info?.hotfixes.length ? <p>{zh ? "应用热更新会暂时停止并恢复助手启动的代理，游戏会保留。" : "Applying hotfixes temporarily stops and restores the assistant's proxy; the game stays open."}</p> : null}
    {info?.release_url && <p>{zh ? "完整新版将从所选源下载。更换组件前需要确认与助手兼容。" : "Download the full release from the selected source. Check assistant compatibility before replacing the component."}</p>}
    {error && <details className="update-error" open><summary>{zh ? "错误详情" : "Error details"}</summary><p role="alert">{error}</p></details>}
    {busy && (progress?.phase === "downloading" ? <div>
      <small>{zh ? "正在下载模块：" : "Downloading module: "}{progress.module}</small>
      <DownloadProgress key={`${progress.module}@${progress.commit}`} downloaded={progress.downloaded} total={progress.total} language={language} />
    </div> : <div className="update-download"><progress aria-label={labels[state]} /><small role="status">{phase === "checking" ? labels.checking : progress?.phase === "applying" ? (zh ? "正在编译并应用模块…" : "Compiling and applying module…") : progress?.phase === "restoring" ? (zh ? "正在恢复登录代理…" : "Restoring login proxy…") : (zh ? "正在准备热更新…" : "Preparing hotfixes…")}</small></div>)}
    <footer className="update-actions">
      <button disabled={busy} onClick={onCheck}>{zh ? "重新检查" : "Check again"}</button>
      <button autoFocus disabled={busy} onClick={continuing ? onContinue : onClose}>{continuing ? (zh ? "继续启动游戏" : "Continue launching") : (zh ? "关闭" : "Close")}</button>
      {info?.release_url && <button className="primary" disabled={busy} onClick={onDownload}>{zh ? "下载官方新版" : "Download official release"}</button>}
      {!!info?.hotfixes.length && <button className="primary" disabled={busy} onClick={onApply}>{zh ? "应用热更新" : "Apply hotfixes"}</button>}
    </footer>
  </dialog>, document.body);
}
