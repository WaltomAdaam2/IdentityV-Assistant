import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { journalApi, type JournalSettings, type RecognitionProgress, type RecognitionResult } from "./journalData";

export interface RecognitionView {
  status: "idle" | "busy" | "done" | "error";
  progress: RecognitionProgress | null;
  result: RecognitionResult | null;
  error: string;
  preview: string | null;
}
const empty: RecognitionView = { status: "idle", progress: null, result: null, error: "", preview: null };

export function useJournalRecognition(filters: JournalSettings, active: boolean, ready: boolean,
  onShortcut: () => boolean, onError: (message: string) => void) {
  const [view, setView] = useState<RecognitionView>(empty);
  const request = useRef<string | null>(null);
  const latest = useRef({ filters, ready, onShortcut, onError });
  latest.current = { filters, ready, onShortcut, onError };
  const cancel = useCallback((reason = "user_cancelled") => {
    const id = request.current;
    request.current = null;
    if (id) void journalApi.cancel(id, reason).catch(() => {});
    setView(empty);
  }, []);
  const start = useCallback(async (windowHandle?: number) => {
    if (request.current || !latest.current.ready) return;
    const id = crypto.randomUUID();
    request.current = id;
    setView({ ...empty, status: "busy", progress: { request_id: id, stage: "capturing", completed: 0, total: 0 } });
    try {
      const result = await journalApi.recognize(id, latest.current.filters, windowHandle);
      if (request.current === id) setView({ ...empty, status: "done", result: {...result, request_id: id} });
    } catch (reason) {
      if (request.current === id) {
        const error = String(reason);
        setView({ ...empty, status: "error", error });
        latest.current.onError(error);
      }
    } finally {
      if (request.current === id) request.current = null;
    }
  }, []);
  useEffect(() => { cancel("filters_changed"); }, [filters.season, filters.difficulty, filters.display, cancel]);
  useEffect(() => { if (!active) cancel("page_left"); }, [active, cancel]);
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    const subscriptions = [
      listen<{request_id: string; preview: string}>("journal-recognition-preview", ({ payload }) => {
        if (!disposed && request.current === payload.request_id) setView(value => ({ ...value, preview: payload.preview }));
      }),
      listen<RecognitionProgress>("journal-recognition-progress", ({ payload }) => {
        if (!disposed && request.current === payload.request_id) setView(value => ({ ...value, progress: payload }));
      }),
      listen<{ hwnd: number }>("journal-recognition-shortcut", ({ payload }) => {
        if (!disposed && !request.current && latest.current.ready && latest.current.onShortcut()) void start(payload.hwnd);
      }),
    ];
    subscriptions.forEach(subscription => { void subscription.catch(reason => {
      if (!disposed) latest.current.onError(String(reason));
    }); });
    return () => {
      disposed = true;
      const id = request.current;
      request.current = null;
      if (id) void journalApi.cancel(id, "view_disposed").catch(() => {});
      subscriptions.forEach(subscription => { void subscription.then(unlisten => unlisten()).catch(() => {}); });
    };
  }, [start]);
  return { view, start, cancel };
}
