export type EngineState = "stopped" | "starting" | "idle" | "running" | "stopping" | "error";

export interface EngineSnapshot {
  status?: EngineState | string;
  pid?: number | null;
  engineVersion?: string | null;
  taskStatus?: string;
  lastHeartbeat?: string | null;
  heartbeatAgeSeconds?: number | null;
  lastError?: string | null;
  [key: string]: unknown;
}

export interface EngineEvent {
  type?: string;
  level?: "debug" | "info" | "warning" | "error";
  message?: string;
  timestamp?: string;
  task_id?: string;
  engine_status?: string;
  task_status?: string;
  source?: string;
  state?: string;
  previous_state?: string;
  confidence?: number;
  found?: boolean;
  hwnd?: number | null;
  title?: string | null;
  process_id?: number | null;
  client_rect?: { left?: number; top?: number; width?: number; height?: number } | null;
  dpi_scale?: number | null;
  capture_backend?: string | null;
  capture_backend_configured?: string | null;
  process_name?: string | null;
  class_name?: string | null;
  is_minimized?: boolean | null;
  is_foreground?: boolean | null;
  is_responding?: boolean | null;
  window_status?: string;
  match_reasons?: string[] | null;
  path?: string;
  width?: number;
  height?: number;
  captured_at?: string | null;
  capture_ms?: number | null;
  file_size?: number | null;
  region?: { left?: number; top?: number; width?: number; height?: number } | null;
  template?: string;
  bbox?: [number, number, number, number] | null;
  threshold?: number;
  action?: string;
  input?: string;
  target?: Record<string, unknown> | null;
  dry_run?: boolean;
  enabled?: boolean;
  from?: string;
  to?: string;
  reason?: string;
  code?: string;
  recoverable?: boolean;
  request_id?: string | null;
  capture_last_error?: string | null;
  details?: Record<string, unknown>;
  data?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface WindowInfo {
  found?: boolean;
  hwnd?: number | null;
  title?: string | null;
  process_id?: number | null;
  client_rect?: { left?: number; top?: number; width?: number; height?: number } | null;
  dpi_scale?: number | null;
  capture_backend?: string | null;
  capture_backend_configured?: string | null;
  process_name?: string | null;
  class_name?: string | null;
  is_minimized?: boolean | null;
  is_foreground?: boolean | null;
  is_responding?: boolean | null;
  window_status?: string;
  match_reasons?: string[] | null;
  request_id?: string | null;
  capture_last_error?: string | null;
  [key: string]: unknown;
}

export interface CaptureFrame {
  path?: string;
  imageUrl?: string;
  width?: number;
  height?: number;
  state?: string;
  source?: string;
  captured_at?: string | null;
  capture_ms?: number | null;
  file_size?: number | null;
  region?: { left?: number; top?: number; width?: number; height?: number } | null;
  hwnd?: number | null;
  request_id?: string | null;
  [key: string]: unknown;
}

export interface TemplateTestResult {
  found?: boolean;
  confidence?: number;
  bbox?: [number, number, number, number] | null;
  template?: string;
  threshold?: number;
  [key: string]: unknown;
}

export interface InputTestResult {
  accepted?: boolean;
  message?: string;
  [key: string]: unknown;
}

export interface DiagnosticRequest {
  requested: boolean;
  request_id: string;
}

export interface Settings {
  app: {
    language: "zh-CN" | "en-US";
    theme: "system" | "light" | "dark";
    launch_engine_on_start: boolean;
    developer_mode: boolean;
  };
  engine: {
    dry_run: boolean;
    memory_protection: boolean;
    heartbeat_interval_seconds: number;
    mock_step_delay_seconds: number;
  };
  game_window: {
    process_names: string[];
    title_contains: string[];
    class_names: string[];
    background_mode: boolean;
    resolution: "1280x720" | "1600x900" | "1920x1080" | "2560x1400";
    position: "free" | "center" | "left_top" | "right_top" | "left_bottom" | "right_bottom";
    restore_after_task: boolean;
  };
  capture: { backend: string; idle_fps: number; active_fps: number };
  vision: { template_threshold: number; state_confirmation_count: number };
  debug: { save_debug_screenshots: boolean; verbose_logs: boolean };
}

export interface TaskItem {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  status: "idle" | "queued" | "running" | "paused";
}
