import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DiagnosticRequest,
  EngineEvent,
  EngineSnapshot,
  Settings,
} from "./types";

export const bridge = {
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
