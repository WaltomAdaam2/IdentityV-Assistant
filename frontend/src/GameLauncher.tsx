import { useEffect, useState } from "react";
import { bridge, type Announcement } from "./bridge";
import type { Settings } from "./types";
import "./launcher.css";

export default function GameLauncher({ language }: { language: Settings["app"]["language"] }) {
  const zh = language === "zh-CN";
  const [news, setNews] = useState<Announcement[]>([]);
  const [newsState, setNewsState] = useState<"loading" | "ready" | "error">("loading");
  const [retry, setRetry] = useState(0);
  const [linkError, setLinkError] = useState(false);
  useEffect(() => {
    let active = true;
    setNewsState("loading");
    bridge.getGameAnnouncements().then((items) => {
      if (active) { setNews(items); setNewsState("ready"); }
    }).catch(() => { if (active) setNewsState("error"); });
    return () => { active = false; };
  }, [retry]);
  const openNews = async (url: string) => {
    setLinkError(false);
    try { await bridge.openGameAnnouncement(url); }
    catch { setLinkError(true); }
  };
  return <section className="game-launcher" aria-label={zh ? "游戏启动" : "Game launch"}>
    <img className="launcher-logo" src="/launcher/logo.png" alt={zh ? "第五人格" : "Identity V"} draggable={false} />
    <div className="launcher-bottom">
      <section className="launcher-news" aria-labelledby="launcher-news-title">
        <div className="launcher-news-heading">
          <h2 id="launcher-news-title">{zh ? "活动与公告" : "News and events"}</h2>
          <button onClick={() => void openNews("https://id5.163.com/news/official/")}>{zh ? "更多" : "More"}</button>
        </div>
        {newsState === "loading" && <p role="status">{zh ? "正在加载公告…" : "Loading announcements…"}</p>}
        {newsState === "error" && <div role="status"><p>{zh ? "公告暂时无法加载，请检查网络后重试。" : "Announcements are unavailable. Check your connection and retry."}</p><button className="launcher-news-retry" onClick={() => setRetry((value) => value + 1)}>{zh ? "重试" : "Retry"}</button></div>}
        {newsState === "ready" && (news.length ? <ul>{news.map((item) => <li key={item.url}>
          <button title={item.title} onClick={() => void openNews(item.url)}>{item.title}</button>
          <time>{new Date(item.release_time * 1000).toLocaleDateString(language, { month: "2-digit", day: "2-digit", timeZone: "Asia/Shanghai" })}</time>
        </li>)}</ul> : <p>{zh ? "暂无公告" : "No announcements yet"}</p>)}
        {linkError && <p role="alert">{zh ? "无法打开公告，请检查默认浏览器。" : "Could not open the announcement. Check your default browser."}</p>}
      </section>
      <div className="launcher-actions">
        <button className="launcher-start" disabled>{zh ? "启动游戏" : "Launch game"}</button>
      </div>
    </div>
  </section>;
}
