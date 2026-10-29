import { cloneElement, useId, type ReactElement, type ReactNode } from "react";

import Icon, { type IconName } from "./Icon";

export const SettingIcon = Icon;

export default function SettingRow({ icon, title, description, language, children }: {
  icon: IconName; title: ReactNode; description: ReactNode; language: "zh-CN" | "en-US";
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
