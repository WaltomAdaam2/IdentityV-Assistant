import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "./types";
import SettingRow, { SettingIcon } from "./SettingRow";
import "./updates.css";
import "./support.css";

interface RestorePreview { id: string; changes: {field: string; before: unknown; after: unknown}[]; warnings: string[] }
interface BundlePreview { id: string; files: {name: string; size: number}[]; truncated: boolean }

const fieldNames: Record<string, [string, string]> = {
  "tasks.enabled": ["启用任务", "Enabled tasks"],
  "app.language": ["界面语言", "Language"], "app.theme": ["界面主题", "Theme"],
  "app.launch_engine_on_start": ["启动应用时启动自动化服务", "Start automation on app launch"],
  "app.launch_on_login": ["开机启动助手", "Launch at sign-in"], "app.minimize_to_tray": ["关闭到托盘", "Close to tray"],
  "app.ui_scale_percent": ["界面缩放（%）", "Interface scale (%)"], "app.engine_hotkey": ["引擎快捷键", "Engine shortcut"],
  "app.developer_mode": ["开发者模式", "Developer mode"], "app.prevent_system_sleep": ["禁止电脑自动休眠", "Prevent system sleep"],
  "app.prevent_display_sleep": ["禁止自动熄屏和闲置锁屏", "Prevent display sleep and idle lock"],
  "app.check_updates_on_start": ["自动检测更新", "Check for updates on startup"],
  "app.update_prereleases": ["接收预发布版本", "Include prereleases"], "app.update_source": ["更新源", "Update source"],
  "engine.memory_protection": ["内存占用保护", "Memory protection"],
  "engine.heartbeat_interval_seconds": ["连接检测间隔（秒）", "Connection check interval (s)"],
  "game_window.executable_path": ["游戏程序路径", "Game executable"], "game_window.resolution": ["游戏分辨率", "Game resolution"],
  "game_window.position": ["窗口位置", "Window position"], "game_window.process_names": ["进程名", "Process names"],
  "game_window.title_contains": ["窗口标题关键词", "Window title keywords"], "game_window.class_names": ["窗口类名", "Window class names"],
  "capture.backend": ["捕获后端", "Capture backend"], "capture.idle_fps": ["空闲帧率", "Idle frame rate"],
  "capture.active_fps": ["任务帧率", "Active frame rate"], "vision.template_threshold": ["模板匹配阈值", "Template threshold"],
  "vision.state_confirmation_count": ["画面确认次数", "Screen confirmation count"],
  "automation.login_platform": ["登录平台", "Login platform"], "automation.login_timeout_seconds": ["登录超时（秒）", "Login timeout (s)"],
  "automation.screenshot_after_click_ms": ["点击后截图等待（毫秒）", "Capture delay after click (ms)"],
  "automation.click_after_detection_ms": ["识别后点击等待（毫秒）", "Click delay after detection (ms)"],
  "automation.click_position_jitter_px": ["点击位置浮动（像素）", "Click position variation (px)"],
  "automation.click_interval_jitter_ms": ["点击间隔浮动（毫秒）", "Click interval variation (ms)"],
  "automation.click_duration_jitter_ms": ["点击时长浮动（毫秒）", "Click duration variation (ms)"],
  "automation.blackjack_infinite_loop": ["黑杰克持续循环", "Repeat Blackjack"],
  "automation.hunter_match_infinite_loop": ["监管者匹配持续循环", "Repeat hunter matches"],
  "automation.hunter_rank_infinite_loop": ["监管者排位持续循环", "Repeat hunter ranked matches"],
  "automation.hide_seek_infinite_loop": ["捉迷藏持续循环", "Repeat Hide and Seek"],
  "automation.camera_turn_scale_percent": ["视角转动比例（%）", "Camera turn scale (%)"],
  "debug.save_debug_screenshots": ["保存调试截图", "Save debug screenshots"],
  "debug.save_failure_screenshots": ["任务失败时保存游戏画面", "Save failure screenshots"], "debug.verbose_logs": ["详细日志", "Verbose logs"],
  "journal.season": ["地图赛季", "Map season"], "journal.difficulty": ["地图难度", "Map difficulty"],
  "journal.door": ["地图门型", "Map door layout"], "journal.floor": ["地图楼层", "Map floor"],
  "journal.display": ["地图显示模式", "Map display"], "journal.collapsed": ["收起地图筛选", "Collapse map filters"],
};

export default function SupportPanel({language, taskActive, withSettings, onRestored, onNotice}: {
  language: Settings["app"]["language"]; taskActive: boolean;
  withSettings: (action: () => Promise<void>) => Promise<void>;
  onRestored: (settings: Settings) => void;
  onNotice: (message: string, error?: boolean, details?: string) => void;
}) {
  const zh = language === "zh-CN";
  const [busy, setBusy] = useState(false);
  const [restore, setRestore] = useState<RestorePreview | null>(null);
  const [bundle, setBundle] = useState<BundlePreview | null>(null);
  const [includeImages, setIncludeImages] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const element = dialog.current;
    if (restore || bundle) element?.showModal();
    return () => element?.close();
  }, [restore, bundle]);
  const closePreview = () => { dialog.current?.close(); setRestore(null); setBundle(null); };
  const choose = (kind: string) => invoke<string | null>("support_choose_path", {kind});
  const run = async (action: () => Promise<void>) => {
    setBusy(true);
    try { await withSettings(action); } catch (error) {
      closePreview();
      onNotice(zh ? "操作未完成，请查看详情后重试。" : "The operation did not finish. Review the details and retry.", true, String(error));
    }
    finally { setBusy(false); }
  };
  const valueNames: Record<string, [string, string]> = {
    light: ["浅色", "Light"], dark: ["深色", "Dark"], system: ["跟随系统", "System"],
    "zh-CN": ["简体中文", "Simplified Chinese"], "en-US": ["English", "English"],
    login_lobby: ["登录进大厅", "Enter lobby"], blackjack: ["黑杰克", "Blackjack"],
    hunter_match: ["监管者匹配", "Hunter match"], hunter_rank: ["监管者排位", "Hunter ranked"], hide_seek: ["捉迷藏", "Hide and Seek"],
    center: ["居中", "Center"], unrestricted: ["无限制", "Unrestricted"], top_left: ["左上", "Top left"],
    top_right: ["右上", "Top right"], bottom_left: ["左下", "Bottom left"], bottom_right: ["右下", "Bottom right"],
  };
  const formatValue = (value: unknown, field: string): string => {
    if (value == null || value === "") return "—";
    if (typeof value === "boolean") return zh ? (value ? "开" : "关") : (value ? "On" : "Off");
    if (Array.isArray(value)) return value.length ? value.map(item => formatValue(item, field)).join(zh ? "、" : ", ") : "—";
    if (typeof value === "number") return new Intl.NumberFormat(language, {maximumFractionDigits: 6}).format(value);
    if (typeof value === "string") return ["settings.app.theme", "settings.app.language", "settings.tasks.enabled", "settings.game_window.position"].includes(field) ? valueNames[value]?.[zh ? 0 : 1] ?? value : value;
    return JSON.stringify(value);
  };
  const bytes = (size: number) => size >= 1048576 ? `${(size / 1048576).toFixed(1)} MiB` : size >= 1024 ? `${(size / 1024).toFixed(1)} KiB` : `${size} B`;
  const modal = (restore || bundle) && createPortal(<dialog ref={dialog} className="update-dialog support-dialog" aria-labelledby="support-title" aria-busy={busy}
    onCancel={event => {event.preventDefault(); if (!busy) closePreview();}}>
    <button className="update-dialog-close" disabled={busy} aria-label={zh ? "关闭预览" : "Close preview"} onClick={closePreview}>×</button>
    <header className="update-dialog-heading">
      <span className="update-state-icon" aria-hidden="true"><SettingIcon name={restore ? "folder" : "debug"} /></span>
      <div><h2 id="support-title">{restore ? (zh ? "应用导入的设置" : "Apply imported settings") : (zh ? "诊断包预览" : "Diagnostic bundle preview")}</h2>
        <p>{restore ? (zh ? "核对变化后再应用。原配置会自动备份。" : "Review the changes before applying. Your current settings will be backed up.") : (zh ? "仅保存到你选择的位置，不会自动上传。" : "Save to a location you choose. Nothing is uploaded automatically.")}</p></div>
    </header>
    {restore && <>
      {restore.warnings.map((warning, index) => <p className="support-hint" key={index}>{warning}</p>)}
      {restore.changes.length ? <>
        <div className="support-table" tabIndex={0} aria-label={zh ? "设置变化" : "Settings changes"}><table><thead><tr><th scope="col">{zh ? "设置" : "Setting"}</th><th scope="col">{zh ? "当前" : "Current"}</th><th scope="col">{zh ? "应用后" : "After apply"}</th></tr></thead><tbody>{restore.changes.map(change => <tr key={change.field}><th scope="row">{fieldNames[change.field.replace(/^settings\./, "")]?.[zh ? 0 : 1] ?? change.field}</th><td>{formatValue(change.before, change.field)}</td><td>{formatValue(change.after, change.field)}</td></tr>)}</tbody></table></div>
        <p className="support-hint">{taskActive ? (zh ? "请先停止任务，再应用设置。" : "Stop active tasks before applying settings.") : (zh ? "应用时会停止空闲的自动化服务，下次启动使用新设置。" : "Applying stops an idle automation service. The next start uses the new settings.")}</p>
      </> : <p className="support-empty">{zh ? "导入的设置与当前一致，无需应用。" : "The imported settings match your current settings. No changes are needed."}</p>}
    </>}
    {bundle && <>
      <div className="support-summary"><strong>{zh ? `${bundle.files.length} 个文件` : `${bundle.files.length} files`}</strong><span>{bytes(bundle.files.reduce((total, file) => total + file.size, 0))}</span></div>
      <p className="support-hint">{includeImages ? (zh ? "已选择包含截图，请在分享前检查昵称或聊天内容。" : "Screenshots were selected. Check for names or chat before sharing.") : (zh ? "未包含截图。" : "Screenshots are not included.")}</p>
      <ul className="support-files" tabIndex={0} aria-label={zh ? "诊断文件清单" : "Diagnostic file list"}>{bundle.files.map(file => <li key={file.name}><span>{file.name}</span><span>{bytes(file.size)}</span></li>)}</ul>
      {bundle.truncated && <p className="support-hint">{zh ? "日志达到大小限制，仅包含限定范围内的会话记录。" : "The log size limit was reached; only bounded session records are included."}</p>}
    </>}
    <footer className="update-actions">
      <button autoFocus disabled={busy} onClick={closePreview}>{restore && !restore.changes.length ? (zh ? "关闭" : "Close") : (zh ? "取消" : "Cancel")}</button>
      {!!restore?.changes.length && <button className="primary" disabled={busy || taskActive} onClick={() => void run(async () => {
        const settings = await invoke<Settings>("restore_settings", {id: restore.id});
        closePreview(); onRestored(settings); onNotice(zh ? "设置已应用，原配置备份已保留。" : "Settings applied; the original backup was retained.");
      })}>{busy ? (zh ? "正在应用…" : "Applying…") : (zh ? "确认应用" : "Confirm apply")}</button>}
      {bundle && <button className="primary" disabled={busy} onClick={() => void run(async () => {
        const path = await choose("export-diagnostics");
        if (path) {await invoke("export_diagnostics", {id: bundle.id, path}); closePreview(); onNotice(zh ? "诊断包已保存，未上传。" : "Diagnostic bundle saved locally; nothing was uploaded.");}
      })}>{busy ? (zh ? "正在保存…" : "Saving…") : (zh ? "保存 ZIP" : "Save ZIP")}</button>}
    </footer>
  </dialog>, document.body);
  return <div className="support-panel" aria-busy={busy}>
    {modal}
    <SettingRow icon="file" title={zh ? "导出设置" : "Export settings"} description={zh ? "备份助手偏好与任务参数，不包含账号、地图或日志。" : "Back up preferences and task options, excluding accounts, maps and logs."} language={language}>
      <button disabled={busy} onClick={() => void run(async () => { const path = await choose("export-settings"); if (path) { await invoke("export_settings", {path}); onNotice(zh ? "设置备份已保存。" : "Settings backup saved."); } })}>{zh ? "导出设置" : "Export settings"}</button>
    </SettingRow>
    <SettingRow icon="folder" title={zh ? "导入设置并预览" : "Import and preview settings"} description={taskActive ? (zh ? "请先停止任务再应用设置。" : "Stop tasks before applying settings.") : (zh ? "预览变化后确认应用，自动保留原配置备份。" : "Review changes before applying; a backup of the original settings is retained.")} language={language}>
      <button disabled={busy || taskActive} onClick={() => void run(async () => { const path = await choose("import-settings"); if (path) setRestore(await invoke<RestorePreview>("preview_settings_restore", {path})); })}>{zh ? "导入设置并预览" : "Import and preview settings"}</button>
    </SettingRow>
    <SettingRow icon="screenshots" title={zh ? "诊断包包含截图" : "Include screenshots in diagnostics"} description={zh ? "可选加入最近的调试／失败截图，最多 20 张、100 MiB。截图可能含昵称或聊天，也可能早于失败发生时间。" : "Optionally include recent debug/failure images, up to 20 images / 100 MiB. Images may contain names or chat and predate the failure."} language={language}>
      <input type="checkbox" disabled={busy} checked={includeImages} onChange={event => {setIncludeImages(event.target.checked); setBundle(null);}} />
    </SettingRow>
    <SettingRow icon="debug" title={zh ? "预览诊断包" : "Preview diagnostics"} description={zh ? "包含版本、必要运行环境、最多 4 MiB 会话日志和脱敏设置；排除账号、凭据和私钥。预览后保存到本机，不自动上传；分享前请检查内容。" : "Includes versions, necessary environment, up to 4 MiB of session logs and redacted settings; excludes accounts, credentials and private keys. Preview and save locally without uploading. Review before sharing."} language={language}>
      <button disabled={busy} onClick={() => void run(async () => { setBundle(await invoke<BundlePreview>("preview_diagnostics", {includeImages})); })}>{zh ? "预览诊断包" : "Preview diagnostics"}</button>
    </SettingRow>
  </div>;
}
