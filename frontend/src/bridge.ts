import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { UpdateInfo, UpdateStatus } from "./UpdatePanel";

import type {
  DiagnosticRequest,
  EngineEvent,
  EngineSnapshot,
  Settings,
} from "./types";

export interface Announcement {
  title: string;
  url: string;
  release_time: number;
}

export interface LauncherStatus {
  phase: "idle" | "starting" | "configuring" | "ready" | "error" | "stopping";
  proxy_phase?: "idle" | "starting" | "configuring" | "initializing" | "updating" | "restarting" | "checking" | "ready" | "stopping" | "error";
  component_path: string | null;
  message: string | null;
}

export interface ComponentUpdateInfo {
  current_version: string;
  version: string;
  notes: string;
  release_url: string | null;
  hotfixes: { target_module: string; target_commit: string; note: string }[];
}

export interface ComponentUpdateProgress {
  phase: "preparing" | "downloading" | "applying" | "restoring";
  downloaded: number;
  total: number | null;
  module: string;
  commit: string;
}

export const bridge = {
  minimizeToTaskbar: () => invoke<void>("minimize_to_taskbar"),
  checkLoginUpdates: (source: Settings["app"]["update_source"]) => invoke<ComponentUpdateInfo>("check_login_updates", {source}),
  applyLoginHotfixes: (source: Settings["app"]["update_source"]) => invoke<void>("apply_login_hotfixes", {source}),
  onComponentUpdateProgress: (handler: (progress: ComponentUpdateProgress) => void): Promise<UnlistenFn> =>
    listen<ComponentUpdateProgress>("component-update-progress", ({ payload }) => handler(payload)),
  openLoginUpdate: (source: Settings["app"]["update_source"]) => invoke<void>("open_login_update", {source}),
  resetWindowSize: () => invoke<void>("reset_window_size"),
  chooseGameExecutable: () => invoke<string | null>("choose_game_executable"),
  chooseLoginComponent: () => invoke<string | null>("choose_login_component"),
  openLogsDirectory: () => invoke<void>("open_logs_directory"),
  launchGame: (executablePath: string) => invoke<void>("launch_game", { executablePath }),
  locateGame: (executablePath = "") => invoke<string>("locate_game", { executablePath }),
  getLauncherStatus: () => invoke<LauncherStatus>("get_launcher_status"),
  setLoginComponent: (executablePath: string) => invoke<void>("set_login_component", { executablePath }),
  configureLoginComponent: (download: boolean) => invoke<void>("configure_login_component", { download }),
  cancelGameLaunch: () => invoke<void>("cancel_game_launch"),
  getGameAnnouncements: () => invoke<Announcement[]>("get_game_announcements"),
  openGameAnnouncement: (url: string) => invoke<void>("open_game_announcement", { url }),
  checkUpdate: (source: Settings["app"]["update_source"], prereleases = false) => invoke<UpdateInfo | null>("check_update", { source, prereleases }),
  downloadUpdate: (source: Settings["app"]["update_source"]) => invoke<void>("download_update", { source }),
  cancelUpdate: () => invoke<void>("cancel_update"),
  installUpdate: () => invoke<void>("install_update"),
  getUpdateStatus: () => invoke<UpdateStatus>("get_update_status"),
  onUpdateStatus: (handler: (status: UpdateStatus) => void): Promise<UnlistenFn> =>
    listen<UpdateStatus>("update-status", ({ payload }) => handler(payload)),
  openRepository: () => invoke<void>("open_project_repository"),
  openReleaseLink: (url: string) => invoke<void>("open_release_link", { url }),
  startEngine: () => invoke<void>("start_engine"),
  stopEngine: () => invoke<void>("stop_engine"),
  restartEngine: () => invoke<void>("restart_engine"),
  sendEngineCommand: (command: Record<string, unknown>) =>
    invoke<void>("send_engine_command", { command }),
  getEngineSnapshot: () => invoke<EngineSnapshot>("get_engine_snapshot"),
  loadSettings: () => invoke<Settings>("load_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  setPowerPrevention: (systemSleep: boolean, displaySleep: boolean) =>
    invoke<void>("set_power_prevention", { systemSleep, displaySleep }),
  appendUiLog: (entry: Record<string, unknown>) => invoke<void>("append_ui_log", { entry }),
  detectWindow: () => invoke<DiagnosticRequest>("detect_window"),
  taskPreflight: () => invoke<DiagnosticRequest>("task_preflight"),
  captureFrame: () => invoke<DiagnosticRequest>("capture_frame"),
  readDebugFrame: (path: string) => invoke<string>("read_debug_frame", { path }),
  testTemplate: (template: string) =>
    invoke<DiagnosticRequest>("test_template", { template }),
  inputTest: (action: Record<string, unknown>) =>
    invoke<DiagnosticRequest>("input_test", { action }),
  onEngineEvent: (handler: (event: EngineEvent) => void): Promise<UnlistenFn> =>
    listen<EngineEvent>("engine-event", ({ payload }) => handler(payload)),
};
