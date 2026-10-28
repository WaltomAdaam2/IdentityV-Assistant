export type EngineState = "stopped" | "starting" | "idle" | "running" | "paused" | "stopping" | "error";

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
  stage?: string;
  role?: "hunter" | "survivor" | "bad_child" | null;
  completed_cycles?: number;
  infinite_loop?: boolean;
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
  journal?: import("./journalData").JournalSettings;
  tasks: { enabled: string[] };
  app: {
    language: "zh-CN" | "en-US";
    theme: "system" | "light" | "dark";
    launch_engine_on_start: boolean;
    developer_mode: boolean;
    prevent_system_sleep: boolean;
    prevent_display_sleep: boolean;
    check_updates_on_start: boolean;
    update_source: "direct" | "proxy";
  };
  engine: {
    memory_protection: boolean;
    heartbeat_interval_seconds: number;
  };
  game_window: {
    resolution: "2560x1440" | "1920x1080" | "1600x900" | "1280x720";
    position: "center" | "unrestricted" | "top_left" | "top_right" | "bottom_left" | "bottom_right";
    process_names: string[];
    title_contains: string[];
    class_names: string[];
  };
  capture: { backend: string; idle_fps: number; active_fps: number };
  vision: { template_threshold: number; state_confirmation_count: number };
  automation: {
    login_platform: "android" | "ios";
    login_timeout_seconds: number;
    screenshot_after_click_ms: number;
    click_after_detection_ms: number;
    click_position_jitter_px: number;
    click_interval_jitter_ms: number;
    click_duration_jitter_ms: number;
    blackjack_infinite_loop: boolean;
    hunter_match_infinite_loop: boolean;
    hunter_rank_infinite_loop: boolean;
    hide_seek_infinite_loop: boolean;
    camera_turn_scale_percent: number;
  };
  debug: { save_debug_screenshots: boolean; verbose_logs: boolean };
}

export interface TaskItem {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  status: "idle" | "queued" | "running" | "paused";
}
