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
  return entries.filter((entry) =>
    (verbose || (!entry.detailedOnly && (entry.important || entry.level === "warning" || entry.level === "error"))) &&
    (level === "all" || entry.level === level) &&
    (!search || entry.message.toLocaleLowerCase().includes(search)),
  );
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
