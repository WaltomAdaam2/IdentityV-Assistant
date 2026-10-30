import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import type { Settings } from "./types";
import { importJournalPacks, journalApi, matchingMaps, type JournalLibrary, type JournalSettings, type MapImage } from "./journalData";
import "./journal.css";

const groups = [
  { key: "difficulty", zh: "难度", en: "Difficulty", options: [["hard", "困难", "Hard"], ["nightmare", "噩梦", "Nightmare"]] },
  { key: "door", zh: "门型", en: "Doors", options: [["lf", "左前", "Left / front"], ["rf", "右前", "Right / front"], ["lr", "左右", "Left / right"], ["triple", "三开", "Three doors"]] },
  { key: "floor", zh: "楼层", en: "Floor", options: [["1", "一楼", "1F"], ["2", "二楼", "2F"], ["b1f", "地下室", "Basement"]] },
  { key: "display", zh: "显示", en: "Display", options: [["pure", "纯净", "Base map"], ["route", "速刷", "Route map"]] },
] as const;
const quickFloors = [["b1f", "地下室", "Basement"], ["1", "1F", "1F"], ["2", "2F", "2F"]] as const;

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

export default function Journal({ language, settings: savedSettings, onSettingsChange }: {
  language: Settings["app"]["language"];
  settings: JournalSettings;
  onSettingsChange: (settings: JournalSettings) => void;
}) {
  const zh = language === "zh-CN";
  const [floor, setFloor] = useState("1");
  const [collapsed, setCollapsed] = useState(false);
  const settings = useMemo(() => ({ ...savedSettings, floor, collapsed }), [savedSettings, floor, collapsed]);
  const [library, setLibrary] = useState<JournalLibrary>({ directory: "", packs: [], errors: [] });
  const [busy, setBusy] = useState(true);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [selectedNumber, setSelectedNumber] = useState<number | null>(null);
  const [preview, setPreview] = useState<"closed" | "open" | "closing">("closed");
  const [detailImage, setDetailImage] = useState({ key: "", url: "", error: "" });
  const enterButton = useRef<HTMLButtonElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const previewDialog = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (settings.door === "td") onSettingsChange({ ...settings, door: "lf" });
  }, [settings, onSettingsChange]);
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
  const selectedMap = maps.find((map) => map.index === selectedNumber);
  const availableFloors = useMemo(() => new Set(quickFloors.filter(([floor]) =>
    !(floor === "b1f" && settings.difficulty === "hard") &&
    matchingMaps(library.packs, { ...settings, floor }).some((map) => map.index === selectedNumber)
  ).map(([floor]) => floor)), [library.packs, settings, selectedNumber]);
  const selectedName = selectedMap?.name || (zh ? `地图 ${selectedNumber}` : `Map ${selectedNumber}`);
  const detailKey = selectedMap ? `${selectedMap.image.packId}:${selectedMap.image.path}` : "";
  const detailUrl = detailImage.key === detailKey ? detailImage.url : "";
  const detailError = detailImage.key === detailKey ? detailImage.error : "";
  useEffect(() => {
    if (!selectedMap) return;
    let active = true;
    setDetailImage({ key: detailKey, url: "", error: "" });
    journalApi.image(selectedMap.image)
      .then((url) => { if (active) setDetailImage({ key: detailKey, url, error: "" }); })
      .catch((reason) => { if (active) setDetailImage({ key: detailKey, url: "", error: String(reason) }); });
    return () => { active = false; };
  }, [detailKey]);
  useEffect(() => {
    if (selectedNumber === null) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault(); event.stopImmediatePropagation();
        if (event.repeat) return;
        if (preview === "open") setPreview("closing");
        else if (preview === "closing") return;
        else setSelectedNumber(null);
      } else if (preview !== "closed" && event.key === "Tab") {
        const buttons = [...(previewDialog.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
        const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
        const next = current < 0 ? (event.shiftKey ? buttons.length - 1 : 0) : (current + (event.shiftKey ? -1 : 1) + buttons.length) % buttons.length;
        event.preventDefault(); buttons[next]?.focus();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [selectedNumber, preview]);
  useEffect(() => {
    if (preview === "closed") return;
    closeButton.current?.focus();
    return () => { enterButton.current?.focus(); };
  }, [preview !== "closed"]);
  useEffect(() => {
    if (preview !== "closing") return;
    const timer = window.setTimeout(() => setPreview("closed"), window.matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 240);
    return () => window.clearTimeout(timer);
  }, [preview]);
  const summary = [seasons.find(([id]) => id === settings.season)?.[1], ...groups.map((group) => group.options.find(([value]) => value === settings[group.key])?.[zh ? 1 : 2])].join(" · ");
  const change = (patch: Partial<JournalSettings>) => {
    if (patch.season || patch.difficulty || patch.door) setSelectedNumber(null);
    const next = { ...settings, ...patch };
    if (next.difficulty === "hard" && ["b1f", "3"].includes(next.floor)) next.floor = "1";
    setFloor(next.floor);
    if (Object.keys(patch).some((key) => key !== "floor")) onSettingsChange({ ...next, floor: "1", collapsed: false });
  };
  const floorSwitch = () => <div className="journal-floor-switch" role="group" aria-label={zh ? "切换地图楼层" : "Switch map floor"}>
    {quickFloors.map(([floor, chinese, english]) => <button key={floor} type="button" disabled={!availableFloors.has(floor)} aria-pressed={settings.floor === floor} className={settings.floor === floor ? "primary" : ""} title={!availableFloors.has(floor) ? (zh ? "当前地图没有此楼层" : "This floor is unavailable for this map") : undefined} onClick={() => change({ floor })}>{zh ? chinese : english}</button>)}
  </div>;
  async function reload() {
    setBusy(true); setError(""); setMessage("");
    try { setLibrary(await journalApi.list()); } catch (reason) { setError(String(reason)); } finally { setBusy(false); }
  }
  async function importPack() {
    setBusy(true); setError(""); setMessage(zh ? "请选择地图包…" : "Choose a map pack…");
    try {
      const sources = await journalApi.choose();
      if (!sources?.length) { setMessage(""); return; }
      setMessage(zh ? `正在导入 ${sources.length} 个地图包…` : `Importing ${sources.length} map packs…`);
      const result = await importJournalPacks(sources);
      if (result.library) setLibrary(result.library);
      setMessage(zh ? `已导入 ${result.imported} 个地图包，失败 ${result.errors.length} 个。` : `Imported ${result.imported} map packs; ${result.errors.length} failed.`);
      setError(result.errors.join("\n"));
    } catch (reason) { setMessage(""); setError(String(reason)); } finally { setBusy(false); }
  }
  return <div className="journal">
    {message && <p role="status">{message}</p>}
    {error && <p className="journal-error" role="alert">{error}</p>}
    {library.errors.length > 0 && <details className="journal-error"><summary>{zh ? "部分地图包读取失败" : "Some map packs could not be read"}</summary>{library.errors.map((item, index) => <p key={index}>{item}</p>)}</details>}
    <div className={`journal-layout${settings.collapsed ? " journal-collapsed" : ""}`}>
      <aside className="panel journal-filters">
        <div className="journal-filter-heading">
          <h2 aria-hidden={settings.collapsed}>{zh ? "地图筛选" : "Map filters"}</h2>
          <button type="button" aria-expanded={!settings.collapsed} aria-controls="journal-filter-body" aria-label={zh ? (settings.collapsed ? "展开地图筛选" : "收起地图筛选") : (settings.collapsed ? "Expand filters" : "Collapse filters")} onClick={() => setCollapsed(!collapsed)}>{settings.collapsed ? "›" : "‹"}</button>
        </div>
        <div id="journal-filter-body" hidden={settings.collapsed}>
          <label className="journal-season">{zh ? "赛季" : "Season"}
            <select value={settings.season} onChange={(event) => change({ season: event.target.value })}>{seasons.map(([id, name]) => <option key={id} value={id}>{name}</option>)}</select>
          </label>
          {groups.map((group) => <fieldset key={group.key}>
            <legend>{zh ? group.zh : group.en}</legend>
            <div className="journal-options">{group.options.filter(([value]) => !(group.key === "floor" && value === "b1f" && settings.difficulty === "hard")).map(([value, chinese, english]) => <button type="button" key={value} aria-pressed={settings[group.key] === value} className={settings[group.key] === value ? "primary" : ""} onClick={() => change({ [group.key]: value })}>{zh ? chinese : english}</button>)}</div>
          </fieldset>)}
          <div className="journal-options journal-tools">
            <button type="button" disabled={busy} onClick={() => void importPack()}>{zh ? "导入地图包" : "Import map packs"}</button>
            <button type="button" disabled={busy} onClick={() => void reload()}>{zh ? "刷新地图" : "Refresh maps"}</button>
            <button type="button" onClick={() => { void journalApi.openDirectory().catch((reason) => setError(String(reason))); }}>{zh ? "打开地图目录" : "Open map folder"}</button>
          </div>
          {library.directory && <p className="journal-directory">{library.directory}</p>}
        </div>
      </aside>
      <section className="panel journal-viewer" aria-label={zh ? "地图浏览" : "Map viewer"} aria-busy={busy}>
        {busy ? <div className="empty journal-empty">{zh ? "正在读取地图包…" : "Loading map packs…"}</div>
          : selectedNumber !== null ? <div className="journal-detail" key={`detail:${selectedNumber}`}>
            <button type="button" className="journal-back" onClick={() => setSelectedNumber(null)}>{zh ? "‹ 返回列表" : "‹ Back to list"}</button>
            {!selectedMap ? <p>{zh ? "当前楼层或显示模式暂无此地图。" : "This map is unavailable for the selected floor or display mode."}</p>
              : detailError ? <p role="alert">{zh ? "图片读取失败：" : "Could not read image: "}{detailError}</p>
              : detailUrl ? <img key={detailKey} src={detailUrl} alt={selectedName} onError={() => setDetailImage({ key: detailKey, url: "", error: zh ? "图片格式无效或已损坏" : "Invalid or damaged image" })} />
              : <p>{zh ? "正在读取地图…" : "Loading map…"}</p>}
          </div>
          : maps.length ? <div className="journal-grid" key={`${settings.season}:${settings.difficulty}:${settings.door}:${settings.floor}:${settings.display}`}>{maps.map((map, index) => {
            const name = map.name || (zh ? `地图 ${map.index}` : `Map ${map.index}`);
            return <button type="button" className="journal-map" style={{ "--map-delay": `${Math.min(index, 7) * 35}ms` } as CSSProperties} key={`${map.key}:${map.image.path}`} onClick={() => setSelectedNumber(map.index)} aria-label={zh ? `查看${name}` : `View ${name}`}><Thumbnail image={map.image} name={name} zh={zh} /><span className="journal-map-name">{name}</span></button>;
          })}</div>
          : <div className="empty journal-empty"><span className="journal-empty-icon" aria-hidden="true">▤</span><h3>{zh ? "当前组合暂无地图" : "No maps in this group"}</h3><p>{zh ? "请导入地图包，或选择其他赛季、难度、门型及楼层。" : "Import a map pack or choose another season, difficulty, door type or floor."}</p></div>}
        <footer className={`journal-footer${selectedNumber !== null ? " journal-detail-footer" : ""}`} aria-live="polite"><div><span>{selectedNumber !== null ? selectedName : zh ? `当前组合共 ${maps.length} 张地图` : `${maps.length} maps in this group`}</span><small>{summary}</small></div>
          {selectedNumber !== null && <div className="journal-detail-actions">{floorSwitch()}<button ref={enterButton} type="button" className="primary" disabled={!detailUrl || !!detailError || !selectedMap} onClick={() => setPreview("open")}>{zh ? "进入地图" : "Enter map"}</button></div>}
        </footer>
      </section>
    </div>
    {preview !== "closed" && createPortal(<div ref={previewDialog} className={`journal-full-preview${preview === "closing" ? " is-closing" : ""}`} role="dialog" aria-modal="true" aria-label={zh ? `${selectedName}全窗口预览` : `${selectedName} full-window preview`}>
      {detailUrl ? <img key={detailKey} src={detailUrl} alt={selectedName} onLoad={({ currentTarget: image }) => image.style.setProperty("--map-aspect-ratio", String(image.naturalWidth / image.naturalHeight))} onError={() => setDetailImage({ key: detailKey, url: "", error: zh ? "图片格式无效或已损坏" : "Invalid or damaged image" })} />
        : <p className="journal-preview-status" role={detailError ? "alert" : "status"}>{detailError || (zh ? "正在读取地图…" : "Loading map…")}</p>}
      <div className="journal-preview-floors">{floorSwitch()}</div>
      <button ref={closeButton} type="button" className="journal-preview-close" aria-label={zh ? "退出全窗口预览" : "Exit full-window preview"} title={zh ? "退出预览（Esc）" : "Exit preview (Esc)"} onClick={() => setPreview("closing")}>×</button>
    </div>, document.body)}
  </div>;
}
