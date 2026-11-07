import { messageLanguage } from "./messageLanguage";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "./types";
import SettingRow, { SettingIcon } from "./SettingRow";
import { useDialog } from "./useDialog";
import "./updates.css";
import "./support.css";

interface RestorePreview { id: string; changes: {field: string; before: unknown; after: unknown}[]; warnings: string[] }
interface BundlePreview { id: string; files: {name: string; size: number}[]; truncated: boolean; omissions?: {group: string; reason: string; files?: string[]}[] }
interface DiagnosticSession { id: string; started: string | null; last_record: string; legacy: boolean; current: boolean; log_records: number; reports: number; images: number }
interface SessionCatalog { current: string; sessions: DiagnosticSession[]; warnings?: {group: string; reason: string}[] }

const fieldNames: Record<string, [string, string]> = {
  "tasks.enabled": ["启用任务", "Enabled tasks"],
  "app.language": ["界面语言", "Language"], "app.theme": ["界面主题", "Theme"],
  "app.launch_engine_on_start": ["启动应用时启动自动化服务", "Start automation on app launch"],
  "app.launch_on_login": ["开机启动助手", "Launch at sign-in"], "app.minimize_to_tray": ["关闭到托盘", "Close to tray"],
  "app.ui_scale_percent": ["界面缩放（%）", "Interface scale (%)"], "app.engine_hotkey": ["引擎快捷键", "Engine shortcut"],
  "app.recognition_hotkey": ["地图识别快捷键", "Map recognition shortcut"],
  "app.floor_hotkey": ["楼层切换快捷键", "Floor switching shortcut"],
  "app.diagnostic_images": ["诊断包包含截图", "Include screenshots in diagnostics"],
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
  "journal.auto_difficulty": ["S0 自动判断难度", "S0 automatic difficulty"],
  "journal.s1_display": ["S1 地图显示模式", "S1 map display"],
  "journal.door": ["地图门型", "Map door layout"], "journal.floor": ["地图楼层", "Map floor"],
  "journal.display": ["地图显示模式", "Map display"], "journal.collapsed": ["收起地图筛选", "Collapse map filters"],
  "journal.auto_enter": ["自动进入最匹配地图", "Automatically open the best match"],
  "journal.preview_position.x": ["地图预览水平位置", "Map preview horizontal position"],
  "journal.preview_position.y": ["地图预览垂直位置", "Map preview vertical position"],
  "journal.preview_position.preset": ["地图预设位置", "Map position preset"],
};

export default function SupportPanel({language, taskActive, withSettings, onRestored, onNotice, includeImages, onIncludeImagesChange}: {
  language: Settings["app"]["language"]; taskActive: boolean;
  withSettings: (action: () => Promise<void>) => Promise<void>;
  onRestored: (settings: Settings) => void;
  onNotice: (message: string, error?: boolean, details?: string) => void;
  includeImages: boolean;
  onIncludeImagesChange: (value: boolean) => void;
}) {
  const zh = language === "zh-CN";
  const [busy, setBusy] = useState(false);
  const [restore, setRestore] = useState<RestorePreview | null>(null);
  const [bundle, setBundle] = useState<BundlePreview | null>(null);
  const [sessions, setSessions] = useState<SessionCatalog | null>(null);
  const [selectedSessions, setSelectedSessions] = useState<string[]>([]);
  const [diagnosticError, setDiagnosticError] = useState("");
  const modalHeading = useRef<HTMLHeadingElement>(null);
  const preparedId = useRef<string | null>(null);
  preparedId.current = bundle?.id ?? null;
  const discard = (id: string) => invoke("discard_diagnostics", {id});
  const {dialog, closing, close} = useDialog(Boolean(restore || sessions), () => {
    if (preparedId.current) void discard(preparedId.current).catch(error => onNotice(zh ? "临时诊断包未能释放。" : "Could not release the temporary bundle.", true, String(error)));
    setRestore(null); setBundle(null); setSessions(null); setSelectedSessions([]); setDiagnosticError("");
  });
  useEffect(() => {
    if (sessions) modalHeading.current?.focus();
  }, [Boolean(sessions), Boolean(bundle)]);
  useEffect(() => () => {
    if (preparedId.current) void discard(preparedId.current).catch(error => onNotice(zh ? "临时诊断包未能释放。" : "Could not release the temporary bundle.", true, String(error)));
  }, []);
  const closePreview = () => close();
  const choose = (kind: string) => invoke<string | null>("support_choose_path", {kind});
  const run = async (action: () => Promise<void>) => {
    setBusy(true);
    try { await withSettings(action); } catch (error) {
      const notify = () => onNotice(zh ? "操作未完成，请查看详情后重试。" : "The operation did not finish. Review the details and retry.", true, String(error));
      if (sessions) setDiagnosticError(String(error));
      else if (restore) close(notify); else notify();
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
  const sessionTime = (value: string) => new Intl.DateTimeFormat(language, {year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false}).format(new Date(value));
  const modal = (restore || sessions) && createPortal(<dialog ref={dialog} data-closing={closing || undefined} className="update-dialog support-dialog" aria-labelledby="support-title" aria-busy={busy}
    onCancel={event => {event.preventDefault(); if (!busy) closePreview();}}>
    <button className="update-dialog-close" disabled={busy} aria-label={zh ? "关闭预览" : "Close preview"} onClick={closePreview}>×</button>
    <header className="update-dialog-heading">
      <span className="update-state-icon" aria-hidden="true"><SettingIcon name={restore ? "folder" : "debug"} /></span>
      <div><h2 id="support-title" ref={modalHeading} tabIndex={-1}>{restore ? (zh ? "应用导入的设置" : "Apply imported settings") : bundle ? (zh ? "诊断包预览" : "Diagnostic bundle preview") : (zh ? "选择诊断会话" : "Choose diagnostic sessions")}</h2>
        <p>{restore ? (zh ? "核对变化后再应用。原配置会自动备份。" : "Review the changes before applying. Your current settings will be backed up.") : (zh ? "仅保存到你选择的位置，不会自动上传。" : "Save to a location you choose. Nothing is uploaded automatically.")}</p></div>
    </header>
    {sessions && !bundle && <>
      <p className="support-hint">{zh ? "默认包含当前会话，也可勾选要追加的历史会话。" : "The current session is included. Select any historical sessions to add."}</p>
      <div className="support-sessions" role="group" aria-label={zh ? "诊断会话" : "Diagnostic sessions"}>
        {sessions.sessions.map(session => <label key={session.id} className="support-session">
          <input type="checkbox" name="diagnostic-session" value={session.id} checked={session.current || selectedSessions.includes(session.id)} disabled={busy || session.current}
            onChange={event => {setSelectedSessions(current => event.target.checked ? [...current, session.id] : current.filter(id => id !== session.id)); setDiagnosticError("");}} />
          <span><strong>{session.current ? (zh ? "当前会话" : "Current session") : session.legacy ? (zh ? "旧版日志" : "Legacy logs") : (zh ? "历史会话" : "Previous session")}</strong>
            <span>{session.legacy ? session.id.slice("legacy:".length) : sessionTime(session.started ?? session.last_record)}</span>
            <small>{session.legacy ? (zh ? "无法区分启动会话 · 仅文字记录" : "Cannot separate app launches · Text only") : `${zh ? "日志" : "Logs"} ${session.log_records} · ${zh ? "识图报告" : "Recognition reports"} ${session.reports} · ${includeImages && session.images ? (zh ? `${session.images} 张图片可用` : `${session.images} images available`) : (zh ? "仅文字记录" : "Text only")}`}</small>
          </span>
        </label>)}
      </div>
      <p className="support-hint">{includeImages ? (zh ? "包含仍在三天保留期内的图片和模板；日志与识图文字报告保留七天。" : "Includes available images and templates from the past 3 days. Logs and recognition reports are retained for 7 days.") : (zh ? "截图开关已关闭，仅导出日志和文字报告。" : "Images are disabled. Only logs and text reports will be exported.")}</p>
      <p className="support-hint">{selectedSessions.length ? (zh ? `已追加 ${selectedSessions.length} 个历史记录，不限制总大小。` : `${selectedSessions.length} historical records added. No total size limit.`) : (zh ? "未选择历史会话，仅导出当前会话，最多 100 MB。" : "No history selected. Export the current session only, up to 100 MB.")}</p>
      {!!sessions.warnings?.length && <p className="support-hint">{zh ? `${sessions.warnings.length} 份报告不可读或无法确定所属会话，将在预览中列为缺失。` : `${sessions.warnings.length} reports are unreadable or lack session information. They will be listed as missing in the preview.`}</p>}
    </>}
    {restore && <>
      {restore.warnings.map((warning, index) => <p className="support-hint" key={index}>{messageLanguage(warning, language)}</p>)}
      {restore.changes.length ? <>
        <div className="support-table" tabIndex={0} aria-label={zh ? "设置变化" : "Settings changes"}><table><thead><tr><th scope="col">{zh ? "设置" : "Setting"}</th><th scope="col">{zh ? "当前" : "Current"}</th><th scope="col">{zh ? "应用后" : "After apply"}</th></tr></thead><tbody>{restore.changes.map(change => <tr key={change.field}><th scope="row">{fieldNames[change.field.replace(/^settings\./, "")]?.[zh ? 0 : 1] ?? change.field}</th><td>{formatValue(change.before, change.field)}</td><td>{formatValue(change.after, change.field)}</td></tr>)}</tbody></table></div>
        <p className="support-hint">{taskActive ? (zh ? "请先停止任务，再应用设置。" : "Stop active tasks before applying settings.") : (zh ? "应用时会停止空闲的自动化服务，下次启动使用新设置。" : "Applying stops an idle automation service. The next start uses the new settings.")}</p>
      </> : <p className="support-empty">{zh ? "导入的设置与当前一致，无需应用。" : "The imported settings match your current settings. No changes are needed."}</p>}
    </>}
    {bundle && <>
      <div className="support-summary"><strong>{zh ? `${bundle.files.length} 个文件` : `${bundle.files.length} files`}</strong><span>{bytes(bundle.files.reduce((total, file) => total + file.size, 0))}</span></div>
      <p className="support-hint">{includeImages ? (zh ? "已选择包含截图，请在分享前检查昵称或聊天内容。" : "Screenshots were selected. Check for names or chat before sharing.") : (zh ? "未包含截图。" : "Screenshots are not included.")}</p>
      <ul className="support-files" tabIndex={0} aria-label={zh ? "诊断文件清单" : "Diagnostic file list"}>{bundle.files.map(file => <li key={file.name}><span>{file.name}</span><span>{bytes(file.size)}</span></li>)}</ul>
      {bundle.truncated && <p className="support-hint">{zh ? "部分记录未纳入，请查看下方清单；本机原文件仍按保留期限保存。" : "Some records were omitted. See the list below; local originals follow the retention period."}</p>}
      {!!bundle.omissions?.length && <details className="support-hint"><summary>{zh ? `未包含的记录（${bundle.omissions.length} 组）` : `Omitted records (${bundle.omissions.length} groups)`}</summary><ul>{bundle.omissions.map((item, index) => <li key={index}>{item.group} · {item.reason === "size_limit" ? (zh ? "达到 100 MB 上限" : "100 MB limit reached") : item.reason === "images_expired" ? (zh ? "图片已超过三天保留期" : "Images exceeded the 3-day retention period") : item.reason === "unreadable_report" ? (zh ? "报告不可读或会话归属不明，未猜测关联图片" : "Unreadable report or unknown session; no images associated by guesswork") : (zh ? "文件缺失、不可读或无效" : "Missing, unreadable or invalid file")}</li>)}</ul></details>}
      <p className="support-hint">{zh ? "环境与脱敏设置为本次预览时的状态；历史会话保留其当时的日志和识图报告。" : "Environment and redacted settings reflect this preview. Historical sessions retain their original logs and recognition reports."}</p>
    </>}
    {diagnosticError && <p className="support-error" role="alert">{zh ? "未能完成，请重试或返回调整会话范围。" : "Could not finish. Retry or adjust the selected sessions."}<br />{messageLanguage(diagnosticError, language)}</p>}
    <footer className="update-actions">
      {bundle && <button disabled={busy} onClick={() => void run(async () => {await discard(bundle.id); setBundle(null); setDiagnosticError("");})}>{zh ? "返回选择" : "Back to selection"}</button>}
      <button autoFocus disabled={busy} onClick={closePreview}>{restore && !restore.changes.length ? (zh ? "关闭" : "Close") : (zh ? "取消" : "Cancel")}</button>
      {sessions && !bundle && <button className="primary" disabled={busy} onClick={() => void run(async () => {
        setDiagnosticError(""); setBundle(await invoke<BundlePreview>("preview_diagnostics", {includeImages, historicalSessions: selectedSessions}));
      })}>{busy ? (zh ? "正在整理…" : "Preparing…") : (zh ? "预览" : "Preview")}</button>}
      {!!restore?.changes.length && <button className="primary" disabled={busy || taskActive} onClick={() => void run(async () => {
        const settings = await invoke<Settings>("restore_settings", {id: restore.id});
        onRestored(settings); close(() => onNotice(zh ? "设置已应用，原配置备份已保留。" : "Settings applied; the original backup was retained."));
      })}>{busy ? (zh ? "正在应用…" : "Applying…") : (zh ? "确认应用" : "Confirm apply")}</button>}
      {bundle && <button className="primary" disabled={busy} onClick={() => void run(async () => {
        const path = await choose("export-diagnostics");
        if (path) {await invoke("export_diagnostics", {id: bundle.id, path}); close(() => onNotice(zh ? "诊断包已保存，未上传。" : "Diagnostic bundle saved locally; nothing was uploaded."));}
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
    <SettingRow icon="screenshots" title={zh ? "诊断包包含截图" : "Include screenshots in diagnostics"} description={zh ? "加入所选会话的原图、裁剪图、处理图、模板及调试／失败截图。图片在本机保留 3 天；可能含昵称或聊天，分享前请检查。" : "Include selected sessions’ captures, crops, processed images, templates and debug/failure screenshots. Local images are retained for 3 days. Review names or chat before sharing."} language={language}>
      <input type="checkbox" disabled={busy} checked={includeImages} onChange={event => {onIncludeImagesChange(event.target.checked); setBundle(null);}} />
    </SettingRow>
    <SettingRow icon="debug" title={zh ? "导出诊断包" : "Export diagnostics"} description={zh ? "默认仅包含当前会话，可追加保留期内的历史会话。先选择、再预览，确认后保存到本机；排除账号、凭据和私钥。" : "Includes the current session by default, with optional retained history. Choose sessions, preview, then save locally. Accounts, credentials and private keys are excluded."} language={language}>
      <button disabled={busy} onClick={() => void run(async () => {setSelectedSessions([]); setDiagnosticError(""); setSessions(await invoke<SessionCatalog>("list_diagnostic_sessions"));})}>{zh ? "导出诊断包" : "Export diagnostics"}</button>
    </SettingRow>
  </div>;
}
