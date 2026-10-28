import { useEffect, useMemo, useRef, useState } from "react";
import type { Settings, WindowInfo } from "./types";
import { journalApi, matchingMaps, type JournalLibrary, type JournalSettings, type MapImage } from "./journalData";
import "./journal.css";

const groups = [
  { key: "difficulty", zh: "难度", en: "Difficulty", options: [["hard", "困难", "Hard"], ["nightmare", "噩梦", "Nightmare"]] },
  { key: "door", zh: "门型", en: "Doors", options: [["lf", "左前", "Left / front"], ["rf", "右前", "Right / front"], ["lr", "左右", "Left / right"], ["triple", "三开", "Three doors"], ["td", "侧门", "Side door"]] },
  { key: "floor", zh: "楼层", en: "Floor", options: [["1", "一楼", "1F"], ["2", "二楼", "2F"], ["b1f", "地下室", "Basement"]] },
  { key: "display", zh: "显示", en: "Display", options: [["pure", "纯净", "Base map"], ["route", "速刷", "Route map"]] },
] as const;

function Thumbnail({ image, name, zh }: { image: MapImage; name: string; zh: boolean }) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);
  const [url, setUrl] = useState("");
  const [error, setError] = useState("");
  useEffect(() => {
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) { setVisible(true); observer.disconnect(); }
    }, { rootMargin: "150px" });
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    if (!visible) return;
    let active = true;
    setUrl(""); setError("");
    journalApi.image(image).then((src) => { if (active) setUrl(src); }).catch((reason) => { if (active) setError(String(reason)); });
    return () => { active = false; };
  }, [visible, image.packId, image.path]);
  return <div ref={ref} className="journal-thumbnail">
    {error ? <p role="alert">{zh ? "图片读取失败" : "Could not read image"}<small>{error}</small></p>
      : url ? <img src={url} alt={name} onError={() => setError(zh ? "图片格式无效或已损坏" : "Invalid or damaged image")} />
      : <span>{zh ? "正在读取地图…" : "Loading map…"}</span>}
  </div>;
}

export default function Journal({ language, windowInfo, settings, onSettingsChange }: {
  language: Settings["app"]["language"];
  windowInfo: WindowInfo | null;
  settings: JournalSettings;
  onSettingsChange: (settings: JournalSettings) => void;
}) {
  const zh = language === "zh-CN";
  const [library, setLibrary] = useState<JournalLibrary>({ directory: "", packs: [], errors: [] });
  const [busy, setBusy] = useState(true);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    journalApi.list().then((data) => { if (active) setLibrary(data); })
      .catch((reason) => { if (active) setError(String(reason)); })
      .finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, []);
  const seasons = useMemo(() => {
    const names = new Map(library.packs.map((pack) => [pack.season_id, pack.season_name || pack.season_id]));
    if (!names.has(settings.season)) names.set(settings.season, settings.season === "s0" ? (zh ? "S0 厄运之女" : "S0 Lady of Misfortune") : settings.season);
    return [...names];
  }, [library.packs, settings.season, zh]);
  const maps = useMemo(() => matchingMaps(library.packs, settings), [library.packs, settings]);
  const summary = [seasons.find(([id]) => id === settings.season)?.[1], ...groups.map((group) => group.options.find(([value]) => value === settings[group.key])?.[zh ? 1 : 2])].join(" · ");
  const change = (patch: Partial<JournalSettings>) => {
    const next = { ...settings, ...patch };
    if (next.difficulty === "hard" && ["b1f", "3"].includes(next.floor)) next.floor = "1";
    onSettingsChange(next);
  };
  async function reload() {
    setBusy(true); setError(""); setMessage("");
    try { setLibrary(await journalApi.list()); } catch (reason) { setError(String(reason)); } finally { setBusy(false); }
  }
  async function importPack() {
    setBusy(true); setError(""); setMessage(zh ? "请选择地图包文件夹…" : "Choose a map pack folder…");
    try {
      const folder = await journalApi.choose();
      if (!folder) { setMessage(""); return; }
      setMessage(zh ? "正在导入地图包…" : "Importing map pack…");
      setLibrary(await journalApi.import(folder));
      setMessage(zh ? "地图包导入成功，请选择对应赛季、难度和门型。" : "Map pack imported. Select its season, difficulty and doors.");
    } catch (reason) { setMessage(""); setError(String(reason)); } finally { setBusy(false); }
  }
  return <div className="journal">
    <div className="journal-status">
      <span className={`tag ${windowInfo?.found ? "running" : ""}`}>{windowInfo?.found ? (zh ? "已检测到第五人格窗口" : "Identity V window detected") : (zh ? "尚未检测到游戏窗口" : "Game window not detected")}</span>
      <p>{zh ? "选择地图，查看各楼层布局与路线。" : "Browse map layouts and routes by floor."}</p>
    </div>
    {message && <p role="status">{message}</p>}
    {error && <p className="journal-error" role="alert">{error}</p>}
    {library.errors.length > 0 && <details className="journal-error"><summary>{zh ? "部分地图包读取失败" : "Some map packs could not be read"}</summary>{library.errors.map((item, index) => <p key={index}>{item}</p>)}</details>}
    <div className={`journal-layout${settings.collapsed ? " journal-collapsed" : ""}`}>
      <aside className="panel journal-filters">
        <div className="journal-filter-heading">
          {!settings.collapsed && <h2>{zh ? "地图筛选" : "Map filters"}</h2>}
          <button type="button" aria-expanded={!settings.collapsed} aria-controls="journal-filter-body" aria-label={zh ? (settings.collapsed ? "展开地图筛选" : "收起地图筛选") : (settings.collapsed ? "Expand filters" : "Collapse filters")} onClick={() => change({ collapsed: !settings.collapsed })}>{settings.collapsed ? "›" : "‹"}</button>
        </div>
        <div id="journal-filter-body" hidden={settings.collapsed}>
          <label className="journal-season">{zh ? "赛季" : "Season"}
            <select value={settings.season} onChange={(event) => change({ season: event.target.value })}>{seasons.map(([id, name]) => <option key={id} value={id}>{name}</option>)}</select>
          </label>
          {groups.map((group) => <fieldset key={group.key}>
            <legend>{zh ? group.zh : group.en}</legend>
            <div className="journal-options">{group.options.filter(([value]) => !(group.key === "floor" && value === "b1f" && settings.difficulty === "hard")).map(([value, chinese, english]) => <button type="button" key={value} aria-pressed={settings[group.key] === value} className={settings[group.key] === value ? "primary" : ""} onClick={() => change({ [group.key]: value })}>{zh ? chinese : english}</button>)}</div>
          </fieldset>)}
          <p>{zh ? "纯净显示底图，速刷显示路线图。" : "Base maps show the layout; route maps show a route."}</p>
          <div className="journal-options journal-tools">
            <button type="button" disabled={busy} onClick={() => void importPack()}>{zh ? "导入地图包" : "Import map pack"}</button>
            <button type="button" disabled={busy} onClick={() => void reload()}>{zh ? "刷新地图" : "Refresh maps"}</button>
            <button type="button" onClick={() => { void journalApi.openDirectory().catch((reason) => setError(String(reason))); }}>{zh ? "打开地图目录" : "Open map folder"}</button>
          </div>
          {library.directory && <p className="journal-directory">{library.directory}</p>}
        </div>
      </aside>
      <section className="panel journal-viewer" aria-label={zh ? "地图浏览" : "Map viewer"} aria-busy={busy}>
        {busy ? <div className="empty journal-empty">{zh ? "正在读取地图包…" : "Loading map packs…"}</div>
          : maps.length ? <div className="journal-grid">{maps.map((map) => {
            const name = map.name || (zh ? `地图 ${map.index}` : `Map ${map.index}`);
            return <figure className="journal-map" key={`${map.key}:${map.image.path}`}><Thumbnail image={map.image} name={name} zh={zh} /><figcaption>{name}</figcaption></figure>;
          })}</div>
          : <div className="empty journal-empty"><span className="journal-empty-icon" aria-hidden="true">▤</span><h3>{zh ? "当前组合暂无地图" : "No maps in this group"}</h3><p>{zh ? "请导入地图包，或选择其他赛季、难度、门型及楼层。" : "Import a map pack or choose another season, difficulty, door type or floor."}</p></div>}
        <footer className="journal-footer" aria-live="polite"><span>{zh ? `当前组合共 ${maps.length} 张地图` : `${maps.length} maps in this group`}</span><small>{summary}</small></footer>
      </section>
    </div>
  </div>;
}
