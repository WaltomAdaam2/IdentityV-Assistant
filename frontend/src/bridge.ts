import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  CaptureFrame,
  EngineEvent,
  EngineSnapshot,
  InputTestResult,
  Settings,
  TemplateTestResult,
  WindowInfo,
} from "./types";

export const bridge = {
  startEngine: () => invoke<void>("start_engine"),
  stopEngine: () => invoke<void>("stop_engine"),
  restartEngine: () => invoke<void>("restart_engine"),
  sendEngineCommand: (command: Record<string, unknown>) =>
    invoke<void>("send_engine_command", { command }),
  getEngineSnapshot: () => invoke<EngineSnapshot>("get_engine_snapshot"),
  loadSettings: () => invoke<Settings>("load_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  detectWindow: () => invoke<WindowInfo>("detect_window"),
  captureFrame: () => invoke<CaptureFrame>("capture_frame"),
  readDebugFrame: (path: string) => invoke<string>("read_debug_frame", { path }),
  testTemplate: (template: string) =>
    invoke<TemplateTestResult>("test_template", { template }),
  inputTest: (action: Record<string, unknown>) =>
    invoke<InputTestResult>("input_test", { action }),
  onEngineEvent: (handler: (event: EngineEvent) => void): Promise<UnlistenFn> =>
    listen<EngineEvent>("engine-event", ({ payload }) => handler(payload)),
};
