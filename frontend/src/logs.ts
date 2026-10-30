import type { EngineEvent } from "./types";

export type LogLevel = "debug" | "info" | "warning" | "error";
export interface LogEntry {
  id: number;
  at: string;
  level: LogLevel;
  message: string;
  source: string;
  important?: boolean;
  detailedOnly?: boolean;
}

export function appendLogBatch(current: LogEntry[], pending: LogEntry[]): LogEntry[] {
  return pending.slice().reverse().concat(current).slice(0, 99_000);
}

export function appendNormalLogBatch(current: LogEntry[], pending: LogEntry[]): LogEntry[] {
  if (!pending.length) return current;
  const oldestId = pending[pending.length - 1].id - 99_000 + 1;
  const retained = current.length && current[current.length - 1].id < oldestId
    ? current.filter((entry) => entry.id >= oldestId)
    : current;
  const additions = filterLogs(pending, "all", "", false).filter((entry) => entry.id >= oldestId);
  return additions.length ? appendLogBatch(retained, additions) : retained;
}

export function filterLogs(entries: LogEntry[], level: "all" | LogLevel, query: string, verbose: boolean): LogEntry[] {
  const search = query.toLocaleLowerCase();
  return entries.filter((entry) => {
    // The launcher outcome summarizes these upstream startup instructions.
    // Keep unknown messages (including sign-in prompts) and real failures visible.
    const componentDetail = entry.source === "idv-login" && (
      (entry.level === "info" && /^(正在初始化官方登录组件|本地配置已是最新|云端配置未变化|初始化内置浏览器|证书已存在且有效|由助手托管：跳过桌面快捷方式|mitmproxy .*模式已(?:启动|就绪)|已注册 idvlogin:\/\/ URI Scheme|当前使用进程代理模式，请通过桌面快捷方式启动游戏|如需设置全局代理，可在管理页面切换|拦截成功，您现在可以打开游戏了|登入账号且已经··进入游戏··后)/.test(entry.message)) ||
      (entry.level === "warning" && entry.message === "如果您在之前已经打开了游戏，请关闭游戏后重新打开，否则工具不会生效！")
    );
    return (verbose || (!componentDetail && !entry.detailedOnly && (entry.important || entry.level === "warning" || entry.level === "error"))) &&
    (level === "all" || entry.level === level) &&
    (!search || entry.message.toLocaleLowerCase().includes(search));
  });
}

export function progressLog(event: EngineEvent, stages: Record<string, string>, previous: Map<string, string>): string | null {
  if (event.type !== "task_progress" || !event.task_id) return null;
  const stage = event.stage ?? "running";
  const signature = `${stage}:${event.role ?? ""}:${event.completed_cycles ?? 0}`;
  if (previous.get(event.task_id) === signature) return null;
  previous.set(event.task_id, signature);
  return stages[stage] ?? null;
}

export function isWindowDiagnostic(event: EngineEvent): boolean {
  return event.type !== "error" && event.level !== "error" &&
    (event.type === "window_health" || event.type === "window_diagnostics");
}
