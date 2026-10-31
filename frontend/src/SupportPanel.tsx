import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "./types";
import "./support.css";

interface RestorePreview { id: string; changes: {field: string; before: unknown; after: unknown}[]; warnings: string[] }
interface BundlePreview { id: string; files: {name: string; size: number}[]; truncated: boolean }

export default function SupportPanel({language, taskActive, withSettings, onRestored}: {
  language: Settings["app"]["language"]; taskActive: boolean;
  withSettings: (action: () => Promise<void>) => Promise<void>;
  onRestored: (settings: Settings) => void;
}) {
  const zh = language === "zh-CN";
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [restore, setRestore] = useState<RestorePreview | null>(null);
  const [bundle, setBundle] = useState<BundlePreview | null>(null);
  const [includeImages, setIncludeImages] = useState(false);
  const choose = (kind: string) => invoke<string | null>("support_choose_path", {kind});
  const run = async (action: () => Promise<void>) => {
    setBusy(true); setError(""); setMessage("");
    try { await withSettings(action); } catch (error) { setError(String(error)); }
    finally { setBusy(false); }
  };
  const formatValue = (value: unknown) => value === undefined ? "—" : JSON.stringify(value);
  return <section className="panel support-panel" aria-busy={busy}>
    <h2>{zh ? "备份与诊断" : "Backup and diagnostics"}</h2>
    <p>{zh ? "设置备份包含助手偏好与任务参数，不包含账号、地图或日志。诊断包独立生成，始终由你选择保存和分享。" : "Settings backups contain preferences and task options, excluding accounts, maps and logs. Diagnostic bundles are separate local files that you choose to save and share."}</p>
    {error && <p role="alert" className="form-error">{error}</p>}
    {message && <p role="status">{message}</p>}
    <div className="support-actions">
      <button disabled={busy} onClick={() => void run(async () => { const path = await choose("export-settings"); if (path) { await invoke("export_settings", {path}); setMessage(zh ? "设置备份已保存。" : "Settings backup saved."); } })}>{zh ? "导出设置" : "Export settings"}</button>
      <button disabled={busy || taskActive} onClick={() => void run(async () => { const path = await choose("import-settings"); if (path) setRestore(await invoke<RestorePreview>("preview_settings_restore", {path})); })}>{zh ? "导入设置并预览" : "Import and preview settings"}</button>
    </div>
    {taskActive && <p>{zh ? "请先停止任务再恢复设置。" : "Stop tasks before restoring settings."}</p>}
    {restore && <div className="support-preview">
      <h3>{zh ? "确认设置变化" : "Review settings changes"}</h3>
      <p>{zh ? "确认后会保留原配置备份，并停止空闲引擎；下次启动使用新设置。" : "Restoring saves a copy of the original settings and stops an idle engine. The next start uses the restored settings."}</p>
      {restore.warnings.map((warning,index) => <p key={index}>{warning}</p>)}
      {restore.changes.length ? <div className="support-table"><table><thead><tr><th>{zh ? "设置" : "Setting"}</th><th>{zh ? "当前" : "Current"}</th><th>{zh ? "恢复后" : "After restore"}</th></tr></thead><tbody>{restore.changes.map(change => <tr key={change.field}><td>{change.field}</td><td>{formatValue(change.before)}</td><td>{formatValue(change.after)}</td></tr>)}</tbody></table></div> : <p>{zh ? "设置没有差异。" : "No settings differences."}</p>}
      <div className="support-actions"><button className="primary" disabled={busy || taskActive || !restore.changes.length} onClick={() => void run(async () => {
        const settings = await invoke<Settings>("restore_settings", {id: restore.id});
        onRestored(settings); setRestore(null); setMessage(zh ? "设置已恢复，原配置备份已保留。" : "Settings restored; the original backup was retained.");
      })}>{zh ? "确认恢复设置" : "Confirm restore"}</button><button disabled={busy} onClick={() => setRestore(null)}>{zh ? "取消" : "Cancel"}</button></div>
    </div>}
    <hr />
    <h3>{zh ? "诊断包" : "Diagnostic bundle"}</h3>
    <p>{zh ? "包含版本、必要运行环境、本次会话日志和脱敏设置。日志最多 4 MiB。排除账号数据、登录凭据及签名密钥；分享前仍请检查内容。" : "Includes versions, necessary environment, up to 4 MiB of current-session logs and redacted settings. Excludes account data, login credentials and signing keys. Review the contents before sharing."}</p>
    <label className="check-label"><input type="checkbox" disabled={busy} checked={includeImages} onChange={event => {setIncludeImages(event.target.checked); setBundle(null);}} />{zh ? "另外包含最近的调试／失败截图（最多 20 张、100 MiB）" : "Also include recent debug/failure screenshots (up to 20 images / 100 MiB)"}</label>
    {includeImages && <p>{zh ? "截图未经文字脱敏，可能显示昵称或聊天。失败截图可能早于失败发生时间。" : "Images are not redacted and may show names or chat. Failure screenshots may predate the failure."}</p>}
    <div className="support-actions"><button disabled={busy} onClick={() => void run(async () => { setBundle(await invoke<BundlePreview>("preview_diagnostics", {includeImages})); })}>{zh ? "预览诊断包" : "Preview diagnostics"}</button></div>
    {bundle && <div className="support-preview">
      <ul>{bundle.files.map(file => <li key={file.name}><span>{file.name}</span><span>{(file.size / 1024).toFixed(1)} KB</span></li>)}</ul>
      {bundle.truncated && <p>{zh ? "日志较多，仅包含限定大小内的会话记录。" : "The log size limit was reached; only bounded session records are included."}</p>}
      <button className="primary" disabled={busy} onClick={() => void run(async () => {const path = await choose("export-diagnostics"); if (path) {await invoke("export_diagnostics", {id: bundle.id, path}); setBundle(null); setMessage(zh ? "诊断包已保存，未上传。" : "Diagnostic bundle saved locally; nothing was uploaded.");}})}>{zh ? "保存诊断 ZIP" : "Save diagnostic ZIP"}</button>
    </div>}
  </section>;
}
