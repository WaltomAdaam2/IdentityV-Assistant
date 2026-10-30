import Markdown from "react-markdown";
import { useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { bridge } from "./bridge";

// Release content is remote data: no raw HTML, embedded images, or custom protocols.
export default function ReleaseNotes({ children, language }: { children: string; language: "zh-CN" | "en-US" }) {
  const [error, setError] = useState("");
  return <div className="release-markdown" tabIndex={0}>
    <Markdown skipHtml urlTransform={(url) => /^https?:\/\//i.test(url) ? url : ""} components={{
      a: ({ href, children }) => href ? <a href={href} target="_blank" rel="noreferrer" onClick={(event) => {
        if (!isTauri()) return;
        event.preventDefault(); setError("");
        void bridge.openReleaseLink(href).catch((reason) => setError(String(reason)));
      }}>{children}</a> : <span>{children}</span>,
      img: ({ alt }) => <span>{alt}</span>,
    }}>{children}</Markdown>
    {error && <p role="alert">{language === "zh-CN" ? "无法打开链接：" : "Could not open link: "}{error}</p>}
  </div>;
}
