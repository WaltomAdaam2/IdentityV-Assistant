import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { UpdateInfo, UpdateStatus } from "./UpdatePanel";
import type {
  DiagnosticRequest,
  EngineEvent,
  EngineSnapshot,
  Settings,
} from "./types";

export const bridge = {
  chooseGameExecutable: () => invoke<string | null>("choose_game_executable"),
  openLogsDirectory: () => invoke<void>("open_logs_directory"),
  launchGame: (executablePath: string) => invoke<void>("launch_game", { executablePath }),
  checkUpdate: (source: "direct" | "proxy", prereleases = false) => invoke<UpdateInfo | null>("check_update", { source, prereleases }),
  downloadUpdate: (source: "direct" | "proxy") => invoke<void>("download_update", { source }),
  cancelUpdate: () => invoke<void>("cancel_update"),
  installUpdate: () => invoke<void>("install_update"),
  getUpdateStatus: () => invoke<UpdateStatus>("get_update_status"),
  onUpdateStatus: (handler: (status: UpdateStatus) => void): Promise<UnlistenFn> =>
    listen<UpdateStatus>("update-status", ({ payload }) => handler(payload)),
  openRepository: () => invoke<void>("open_project_repository"),
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
  captureFrame: () => invoke<DiagnosticRequest>("capture_frame"),
  readDebugFrame: (path: string) => invoke<string>("read_debug_frame", { path }),
  testTemplate: (template: string) =>
    invoke<DiagnosticRequest>("test_template", { template }),
  inputTest: (action: Record<string, unknown>) =>
    invoke<DiagnosticRequest>("input_test", { action }),
  onEngineEvent: (handler: (event: EngineEvent) => void): Promise<UnlistenFn> =>
    listen<EngineEvent>("engine-event", ({ payload }) => handler(payload)),
};
