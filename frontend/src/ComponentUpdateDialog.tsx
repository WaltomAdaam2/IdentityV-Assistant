import { useRef } from "react";
import { createPortal } from "react-dom";
import { useDialog } from "./useDialog";
import type { ComponentUpdateInfo, ComponentUpdateProgress } from "./bridge";
import DownloadProgress from "./DownloadProgress";
import type { Settings } from "./types";
import { SettingIcon } from "./SettingRow";
import "./updates.css";

export default function ComponentUpdateDialog({ info, error, phase, progress, source, language, continuing, skipUpdates, preferenceError, onSkipUpdates, onSource, onClose, onContinue, onCheck, onInstall, cancelling, onCancel }: {
  info: ComponentUpdateInfo | null; error: string; phase: "checking" | "applying" | "idle";
  progress: ComponentUpdateProgress | null;
  source: Settings["app"]["update_source"]; language: Settings["app"]["language"]; continuing: boolean;
  skipUpdates: boolean; preferenceError: boolean; onSkipUpdates: (checked: boolean) => void;
  onSource: (source: Settings["app"]["update_source"]) => void; onClose: () => void; onContinue: () => void; onCheck: () => void;
  onInstall: () => void; cancelling: boolean; onCancel: () => void;
}) {
  const continueOnClose = useRef(false);
  const {dialog, closing, close} = useDialog(true, () => continueOnClose.current ? onContinue() : onClose());
  const zh = language === "zh-CN";
  const available = !!info?.available;
  const busy = phase !== "idle";
  const state = error ? "error" : busy ? phase === "checking" ? "checking" : "installing" : available ? "available" : "current";
  const labels = zh ? {checking:"正在检查组件更新…", installing:"正在应用组件更新…", error:"组件更新未完成", available:"发现组件更新", current:"组件已是最新版本"} : {checking:"Checking component updates…", installing:"Applying component updates…", error:"Component update failed", available:"Component update available", current:"Component is up to date"};
  return createPortal(<dialog ref={dialog} data-closing={closing || undefined} className={`update-dialog${available ? " has-release" : ""}`} aria-labelledby="component-update-title" onCancel={(event) => {event.preventDefault(); if (phase !== "applying") close();}}>
    <button className="update-dialog-close" disabled={phase === "applying"} aria-label={zh ? "关闭组件更新" : "Close component update"} onClick={() => close()}>×</button>
    <header className={`update-dialog-heading phase-${state}`}>
      <span className="update-state-icon" aria-hidden="true">{state === "current" ? <svg viewBox="0 0 24 24"><path d="m5 12 4 4L19 6" /></svg> : state === "error" ? <svg viewBox="0 0 24 24"><path d="M12 5v9m0 4h.01" /></svg> : <SettingIcon name="refresh" />}</span>
      <div><h2 id="component-update-title">{labels[state]}</h2><p>{zh ? "官方登录组件 · 更新前由你确认。" : "Official login component · Updates require your confirmation."}</p></div>
    </header>
    <dl className="update-facts">
      <div><dt>{zh ? "当前版本" : "Installed version"}</dt><dd>{info?.current_version || "—"}{info?.available && <><span className="update-version-arrow"> → </span><strong>{info.version}</strong></>}</dd></div>
      <div><dt><label htmlFor="component-update-source">{zh ? "更新源" : "Update source"}</label></dt><dd><select id="component-update-source" disabled={busy} value={source} onChange={(event) => onSource(event.target.value as Settings["app"]["update_source"])}>
        <option value="direct">{zh ? "GitHub 直连（默认）" : "GitHub direct (default)"}</option><option value="proxy">GH-Proxy</option><option value="ghfast">GHFast</option><option value="ghproxy_net">ghproxy.net</option>
      </select></dd></div>
    </dl>
    {available && <section className="update-dialog-notes" aria-label={zh ? "更新日志" : "Release notes"}><h3>{zh ? "更新日志" : "Release notes"}</h3>{info.notes && <p>{info.notes}</p>}</section>}
    {available && <p>{zh ? "将下载、校验并配置官方完整组件。账号数据保留；失败时尝试恢复原组件配置。" : "Downloads, verifies and configures the official full component. Account data is preserved; failures attempt to restore the previous configuration."}</p>}
    {error && <details className="update-error" open><summary>{zh ? "错误详情" : "Error details"}</summary><p role="alert">{error}</p></details>}
    {available && <label className="check-label update-skip"><input type="checkbox" disabled={busy} checked={skipUpdates} onChange={event => onSkipUpdates(event.target.checked)} />{zh ? "不再提醒本批组件更新" : "Don't remind me about these component updates"}</label>}
    {available && <p className="update-hint">{zh ? "新版本仍会提醒，也可以随时手动检查。" : "New versions will still prompt. You can check manually at any time."}</p>}
    {preferenceError && <p role="alert" className="update-error">{zh ? "提醒偏好未保存，请重试。" : "Could not save this preference. Try again."}</p>}
    {busy && (progress?.phase === "downloading" ? <div>
      <small>{zh ? "正在下载组件：" : "Downloading component: "}{progress.file}</small>
      <DownloadProgress key={progress.file} downloaded={progress.downloaded} total={progress.total} language={language} />
    </div> : <div className="update-download"><progress aria-label={labels[state]} /><small role="status">{phase === "checking" ? labels.checking : progress?.phase === "applying" ? (zh ? "正在配置组件…" : "Configuring component…") : progress?.phase === "restoring" ? (zh ? "正在恢复登录代理…" : "Restoring login proxy…") : (zh ? "正在解包并校验组件…" : "Extracting and verifying component…")}</small></div>)}
    <footer className="update-actions">
      <button disabled={busy} onClick={onCheck}>{zh ? "重新检查" : "Check again"}</button>
      <button autoFocus disabled={busy} onClick={() => {continueOnClose.current = continuing; close();}}>{continuing ? (zh ? "继续启动游戏" : "Continue launching") : (zh ? "关闭" : "Close")}</button>
      {info?.available && <button className="primary" disabled={busy} onClick={onInstall}>{zh ? "更新组件" : "Update component"}</button>}
      {phase === "applying" && progress?.cancellable !== false && <button disabled={cancelling} onClick={onCancel}>{cancelling ? (zh ? "正在取消…" : "Cancelling…") : (zh ? "取消下载" : "Cancel download")}</button>}
    </footer>
  </dialog>, document.body);
}
