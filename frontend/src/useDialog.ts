import { useCallback, useEffect, useRef, useState } from "react";

/** Keep native modality and focus restoration until the closing motion finishes. */
export function useDialog(open: boolean, onClosed: () => void) {
  const dialog = useRef<HTMLDialogElement>(null);
  const callback = useRef(onClosed);
  callback.current = onClosed;
  const timer = useRef<number>();
  const [closing, setClosing] = useState(false);
  useEffect(() => {
    const element = dialog.current;
    if (!open || !element) return;
    element.inert = false;
    setClosing(false);
    if (!element.open) element.showModal();
    return () => {
      window.clearTimeout(timer.current);
      timer.current = undefined;
      element.close();
    };
  }, [open]);
  const close = useCallback((after?: () => void) => {
    if (timer.current !== undefined) return;
    const finish = () => {
      dialog.current?.close();
      timer.current = undefined;
      setClosing(false);
      callback.current();
      after?.();
    };
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) { finish(); return; }
    if (dialog.current) dialog.current.inert = true;
    setClosing(true);
    timer.current = window.setTimeout(finish, 180);
  }, []);
  return { dialog, closing, close };
}
