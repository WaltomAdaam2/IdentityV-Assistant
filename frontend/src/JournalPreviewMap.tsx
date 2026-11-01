import { useLayoutEffect, useRef, useState, type PointerEvent } from "react";
import { previewPlacement, type JournalSettings } from "./journalData";

type Position = JournalSettings["preview_position"];

export default function JournalPreviewMap({ src, name, zh, position, onPositionChange, onError }: {
  src: string; name: string; zh: boolean; position: Position;
  onPositionChange: (position: Position) => void; onError: () => void;
}) {
  const element = useRef<HTMLDivElement>(null);
  const drag = useRef<{ id: number; x: number; y: number; moved: boolean } | null>(null);
  const [draft, setDraft] = useState<Position | null>(null);
  const latestPosition = useRef(position);
  const [view, setView] = useState({ width: 0, height: 0 });
  const [aspect, setAspect] = useState(1);
  const placement = previewPlacement(view, aspect, draft ?? position);
  useLayoutEffect(() => {
    const parent = element.current?.parentElement;
    if (!parent) return;
    const resize = () => setView({ width: parent.clientWidth, height: parent.clientHeight });
    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(parent);
    return () => observer.disconnect();
  }, []);
  const move = (x: number, y: number) => {
    const next = previewPlacement(view, aspect, { x, y });
    const center = { x: (next.x + next.width / 2) / view.width, y: (next.y + next.height / 2) / view.height };
    latestPosition.current = center;
    setDraft(center);
    return center;
  };
  const finish = (event: PointerEvent<HTMLDivElement>) => {
    if (drag.current?.id !== event.pointerId) return;
    const moved = drag.current.moved;
    drag.current = null;
    if (moved) onPositionChange(latestPosition.current);
    setDraft(null);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
  };
  return <div ref={element} className={`journal-preview-map${draft ? " is-dragging" : ""}`} tabIndex={0} role="group"
    aria-label={zh ? `${name}，拖动或使用方向键移动地图` : `${name}. Drag or use arrow keys to move the map`}
    style={{ width: placement.width, height: placement.height, transform: `translate(${placement.x}px, ${placement.y}px)` }}
    onPointerDown={event => {
      if (event.button !== 0 || !event.isPrimary || !view.width || !view.height) return;
      event.preventDefault();
      event.currentTarget.focus({ preventScroll: true });
      event.currentTarget.setPointerCapture(event.pointerId);
      drag.current = { id: event.pointerId, x: event.clientX, y: event.clientY, moved: false };
      latestPosition.current = { x: (placement.x + placement.width / 2) / view.width, y: (placement.y + placement.height / 2) / view.height };
      setDraft(latestPosition.current);
    }}
    onPointerMove={event => {
      const previous = drag.current;
      const bounds = element.current?.parentElement?.getBoundingClientRect();
      if (!previous || previous.id !== event.pointerId || !bounds?.width || !bounds.height) return;
      // Client coordinates include interface zoom; saved coordinates are viewport-relative.
      const before = latestPosition.current;
      const next = move(before.x + (event.clientX - previous.x) / bounds.width,
        before.y + (event.clientY - previous.y) / bounds.height);
      previous.moved ||= Math.abs(next.x - before.x) > 1e-9 || Math.abs(next.y - before.y) > 1e-9;
      previous.x = event.clientX; previous.y = event.clientY;
    }}
    onPointerUp={finish} onPointerCancel={finish} onLostPointerCapture={finish}
    onKeyDown={event => {
      const delta = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[event.key];
      if (!delta || !view.width || !view.height) return;
      event.preventDefault();
      const step = event.shiftKey ? 32 : 8;
      const center = { x: (placement.x + placement.width / 2) / view.width, y: (placement.y + placement.height / 2) / view.height };
      const next = move(center.x + delta[0] * step / view.width, center.y + delta[1] * step / view.height);
      if (Math.abs(next.x - center.x) > 1e-9 || Math.abs(next.y - center.y) > 1e-9) onPositionChange(next);
      setDraft(null);
    }}>
    <img src={src} alt={name} draggable={false} onLoad={({ currentTarget: image }) => setAspect(image.naturalWidth / image.naturalHeight)} onError={onError} />
  </div>;
}
