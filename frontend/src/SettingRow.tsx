import { cloneElement, useId, type ReactElement, type ReactNode } from "react";

const icons = {
  theme: "M12 3a9 9 0 1 0 9 9c0-2-2-3-4-3h-2a2 2 0 0 1-2-2V5a2 2 0 0 0-1-2ZM7 10h.01M8 15h.01M13 17h.01",
  language: "M3 5h12M9 3v2M5 5c1 6 5 9 9 11M13 5c-1 6-5 9-10 11M14 21l4-10 4 10M16 17h4",
  power: "M12 2v9M6 5a9 9 0 1 0 12 0",
  tray: "M3 14v6h18v-6M7 10l5 5 5-5M12 3v12",
  window: "M3 4h18v16H3ZM3 8h18",
  keyboard: "M2 5h20v14H2ZM6 9h.01M10 9h.01M14 9h.01M18 9h.01M6 12h.01M10 12h.01M14 12h.01M18 12h.01M7 16h10",
  play: "m7 3 14 9-14 9Z",
  moon: "M20 14A8 8 0 0 1 10 4a9 9 0 1 0 10 10Z",
  monitor: "M3 3h18v14H3ZM12 17v4M8 21h8",
  code: "m8 6-6 6 6 6M16 6l6 6-6 6M14 3l-4 18",
  folder: "M3 5h6l2 3h10v12H3Z",
  position: "M12 2v5M12 17v5M2 12h5M17 12h5M12 7a5 5 0 1 0 0 10 5 5 0 0 0 0-10Z",
  search: "M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14ZM15 15l6 6",
  clock: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18ZM12 7v5l3 2",
  camera: "M3 7h4l2-3h6l2 3h4v13H3ZM12 9a4 4 0 1 0 0 8 4 4 0 0 0 0-8Z",
  memory: "M10 14V4a2 2 0 0 1 4 0v10a4 4 0 1 1-4 0ZM12 9v9",
  file: "M5 2h10l4 4v16H5ZM15 2v5h4M8 11h8M8 15h8M8 19h5",
  refresh: "M20 8a8 8 0 0 0-14-3L3 8M3 3v5h5M4 16a8 8 0 0 0 14 3l3-3M16 16h5v5",
  cloud: "M6 19a5 5 0 0 1-1-10 7 7 0 0 1 13-1 5 5 0 0 1 0 11ZM12 10v7m-3-3 3 3 3-3",
} as const;

export function SettingIcon({ name }: { name: keyof typeof icons }) {
  return <svg className="setting-icon" viewBox="0 0 24 24" aria-hidden="true"><path d={icons[name]} /></svg>;
}

export default function SettingRow({ icon, title, description, language, children }: {
  icon: keyof typeof icons; title: ReactNode; description: ReactNode; language: "zh-CN" | "en-US";
  children: ReactElement<{ type?: string; checked?: boolean; "aria-labelledby"?: string; "aria-describedby"?: string }>;
}) {
  const id = useId();
  const toggle = children.props.type === "checkbox";
  const control = cloneElement(children, { "aria-labelledby": `${id}-title`, "aria-describedby": `${id}-description` });
  return <div className="setting-row">
    <SettingIcon name={icon} />
    <div className="setting-copy"><span id={`${id}-title`} className="setting-title">{title}</span><span id={`${id}-description`} className="setting-description">{description}</span></div>
    <div className={`setting-control${toggle ? " setting-toggle" : ""}`}>
      {toggle && <span aria-hidden="true">{language === "zh-CN" ? (children.props.checked ? "开" : "关") : (children.props.checked ? "On" : "Off")}</span>}
      {control}
    </div>
  </div>;
}
