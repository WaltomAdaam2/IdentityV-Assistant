import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import type { Settings } from "./types";
import type { RecognitionView } from "./useJournalRecognition";
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

export default function Journal({ language, settings: savedSettings, onSettingsChange, onNotice, recognition, onRecognize, onCancelRecognition, shortcut }: {
  language: Settings["app"]["language"];
  settings: JournalSettings;
  onSettingsChange: (settings: JournalSettings) => void;
  onNotice: (message: string, error?: boolean) => void;
  recognition: RecognitionView;
  onRecognize: () => void;
  onCancelRecognition: () => void;
  shortcut: string;
}) {
  const zh = language === "zh-CN";
  const [floor, setFloor] = useState("1");
  const [collapsed, setCollapsed] = useState(false);
  const settings = useMemo(() => ({ ...savedSettings, floor, collapsed }), [savedSettings, floor, collapsed]);
  const [library, setLibrary] = useState<JournalLibrary>({ directory: "", packs: [], errors: [] });
  const [busy, setBusy] = useState(true);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const recognizing = recognition.status === "busy";
  const recognizedKeys = recognition.status === "idle" ? undefined : recognition.result?.keys ?? [];
  const [preview, setPreview] = useState<"closed" | "open" | "closing">("closed");
  const [detailImage, setDetailImage] = useState({ key: "", url: "", error: "" });
  const enterButton = useRef<HTMLButtonElement>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  const previewDialog = useRef<HTMLDialogElement>(null);
  const backButton = useRef<HTMLButtonElement>(null);
  const mapButtons = useRef(new Map<string, HTMLButtonElement>());
  const lastSelected = useRef<string | null>(null);
  useEffect(() => {
    if (recognizing) { setSelectedKey(null); setFloor("1"); setPreview("closed"); }
  }, [recognizing]);
  useEffect(() => {
    if (!document.hasFocus()) return;
    if (selectedKey !== null) { lastSelected.current = selectedKey; backButton.current?.focus(); }
    else if (lastSelected.current !== null) { mapButtons.current.get(lastSelected.current)?.focus(); lastSelected.current = null; }
  }, [selectedKey]);
  useEffect(() => {
    if (settings.door === "td") onSettingsChange({ ...settings, door: "lf" });
  }, [settings, onSettingsChange]);
  useEffect(() => {
    let active = true;
    journalApi.list().then((data) => { if (active) setLibrary(data); })
      .catch((reason) => { if (active) onNotice(String(reason), true); })
      .finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, []);
  const seasons = useMemo(() => {
    const names = new Map(library.packs.map((pack) => [pack.season_id, pack.season_name || pack.season_id]));
    if (!names.has(settings.season)) names.set(settings.season, settings.season === "s0" ? (zh ? "S0 厄运之女" : "S0 Lady of Misfortune") : settings.season);
    return [...names];
  }, [library.packs, settings.season, zh]);
  const maps = useMemo(() => matchingMaps(library.packs, settings, recognizedKeys), [library.packs, settings, recognizedKeys]);
  const selectedMap = maps.find((map) => map.key === selectedKey);
  const availableFloors = useMemo(() => new Set(quickFloors.filter(([floor]) =>
    !(floor === "b1f" && settings.difficulty === "hard") &&
    matchingMaps(library.packs, { ...settings, floor }, recognizedKeys).some((map) => map.key === selectedKey)
  ).map(([floor]) => floor)), [library.packs, settings, selectedKey, recognizedKeys]);
  const mapName = (map: typeof maps[number]) => {
    const name = map.name || (zh ? `地图 ${map.index}` : `Map ${map.index}`);
    const door = groups[1].options.find(([value]) => value === map.category)?.[zh ? 1 : 2] || map.category;
    return recognizedKeys ? `${door} · ${name}` : name;
  };
  const selectedName = selectedMap ? mapName(selectedMap) : (zh ? "地图" : "Map");
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
    if (selectedKey === null) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented && !event.repeat && preview === "closed" && !document.querySelector(":modal")) {
        event.preventDefault(); setSelectedKey(null);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selectedKey, preview]);
  useEffect(() => {
    if (preview === "closed") return;
    const dialog = previewDialog.current;
    if (dialog && !dialog.open) dialog.showModal();
    closeButton.current?.focus();
    return () => { dialog?.close(); if (document.hasFocus()) enterButton.current?.focus(); };
  }, [preview !== "closed"]);
  useEffect(() => {
    if (preview !== "closing") return;
    const timer = window.setTimeout(() => setPreview("closed"), window.matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 240);
    return () => window.clearTimeout(timer);
  }, [preview]);
  const summary = [seasons.find(([id]) => id === settings.season)?.[1], ...groups.filter(group => !recognizedKeys || group.key !== "door").map((group) => group.options.find(([value]) => value === settings[group.key])?.[zh ? 1 : 2])].join(" · ");
  const change = (patch: Partial<JournalSettings>) => {
    if (recognizing || Object.keys(patch).some(key => key !== "floor")) onCancelRecognition();
    if (patch.season || patch.difficulty || patch.door) setSelectedKey(null);
    const next = { ...settings, ...patch };
    if (next.difficulty === "hard" && ["b1f", "3"].includes(next.floor)) next.floor = "1";
    setFloor(next.floor);
    if (Object.keys(patch).some((key) => key !== "floor")) onSettingsChange({ ...next, floor: "1", collapsed: false });
  };
  const floorSwitch = () => <div className="journal-floor-switch" role="group" aria-label={zh ? "切换地图楼层" : "Switch map floor"}>
    {quickFloors.map(([floor, chinese, english]) => <button key={floor} type="button" disabled={!availableFloors.has(floor)} aria-pressed={settings.floor === floor} className={settings.floor === floor ? "primary" : ""} title={!availableFloors.has(floor) ? (zh ? "当前地图没有此楼层" : "This floor is unavailable for this map") : undefined} onClick={() => change({ floor })}>{zh ? chinese : english}</button>)}
  </div>;
  async function reload() {
    onCancelRecognition(); setSelectedKey(null);
    setBusy(true);
    try { setLibrary(await journalApi.list()); } catch (reason) { onNotice(String(reason), true); } finally { setBusy(false); }
  }
  async function importPack() {
    onCancelRecognition(); setSelectedKey(null);
    setBusy(true);
    try {
      const sources = await journalApi.choose();
      if (!sources?.length) return;
      const result = await importJournalPacks(sources);
      if (result.library) setLibrary(result.library);
      const summary = zh ? `已导入 ${result.imported} 个地图包，失败 ${result.errors.length} 个。` : `Imported ${result.imported} map packs; ${result.errors.length} failed.`;
      onNotice([summary, ...result.errors].join("\n"), result.errors.length > 0);
    } catch (reason) { onNotice(String(reason), true); } finally { setBusy(false); }
  }
  return <div className="journal">
    {library.errors.length > 0 && <details className="journal-error"><summary>{zh ? "部分地图包读取失败" : "Some map packs could not be read"}</summary>{library.errors.map((item, index) => <p key={index}>{item}</p>)}</details>}
    <div className={`journal-layout${settings.collapsed ? " journal-collapsed" : ""}`}>
      <aside className="panel journal-filters">
        <div className="journal-filter-heading">
          <h2 aria-hidden={settings.collapsed}>{zh ? "地图筛选" : "Map filters"}</h2>
          <button type="button" aria-expanded={!settings.collapsed} aria-controls="journal-filter-body" aria-label={zh ? (settings.collapsed ? "展开地图筛选" : "收起地图筛选") : (settings.collapsed ? "Expand filters" : "Collapse filters")} onClick={() => setCollapsed(!collapsed)}>{settings.collapsed ? "›" : "‹"}</button>
        </div>
        <div id="journal-filter-body" ref={element => {if (element) element.inert = settings.collapsed;}} aria-hidden={settings.collapsed}><div className="journal-filter-content">
          <label className="journal-season">{zh ? "赛季" : "Season"}
            <select value={settings.season} onChange={(event) => change({ season: event.target.value })}>{seasons.map(([id, name]) => <option key={id} value={id}>{name}</option>)}</select>
          </label>
          {groups.map((group) => <fieldset key={group.key}>
            <legend>{zh ? group.zh : group.en}</legend>
            <div className="journal-options">{group.options.filter(([value]) => !(group.key === "floor" && value === "b1f" && settings.difficulty === "hard")).map(([value, chinese, english]) => <button type="button" key={value} aria-pressed={settings[group.key] === value} className={settings[group.key] === value ? "primary" : ""} onClick={() => change({ [group.key]: value })}>{zh ? chinese : english}</button>)}</div>
          </fieldset>)}
          <div className="journal-options journal-tools">
            <button type="button" className="primary" disabled={busy || recognizing} onClick={onRecognize}>{zh ? "识别一楼地图" : "Recognize 1F map"}</button>
            {recognizing && <button type="button" onClick={onCancelRecognition}>{zh ? "取消识别" : "Cancel recognition"}</button>}
          </div>
          <p className="journal-recognition-hint">{zh ? `游戏前台按 ${shortcut} 也可识别，不切走游戏焦点。仅识别当前赛季、难度的一楼。` : `Press ${shortcut} in the game without losing focus. Matches 1F in the selected season and difficulty.`}</p>
          <div className="journal-options journal-tools">
            <button type="button" disabled={busy} onClick={() => void importPack()}>{zh ? "导入地图包" : "Import map packs"}</button>
            <button type="button" disabled={busy} onClick={() => void reload()}>{zh ? "刷新地图" : "Refresh maps"}</button>
            <button type="button" onClick={() => { void journalApi.openDirectory().catch((reason) => onNotice(String(reason), true)); }}>{zh ? "打开地图目录" : "Open map folder"}</button>
          </div>
          {library.directory && <p className="journal-directory">{library.directory}</p>}
        </div></div>
      </aside>
      <section className="panel journal-viewer" aria-label={zh ? "地图浏览" : "Map viewer"} aria-busy={busy || recognizing}>
        {recognition.status !== "idle" && <div className="journal-recognition">
          <div className="journal-recognition-heading"><h2>{zh ? "一楼识图结果" : "First-floor matches"}</h2>
            <div className="journal-options">{recognizing
              ? <button type="button" onClick={onCancelRecognition}>{zh ? "取消识别" : "Cancel recognition"}</button>
              : <><button type="button" onClick={onRecognize}>{zh ? "重新识别" : "Recognize again"}</button><button type="button" onClick={() => { setSelectedKey(null); onCancelRecognition(); }}>{zh ? "返回全部地图" : "Show all maps"}</button></>}
            </div>
          </div>
          <p role="status">{recognizing
            ? ({ capturing: zh ? "正在截图…" : "Capturing…", locating: zh ? "正在定位地图…" : "Locating the map…", matching: zh ? "正在匹配地图…" : "Matching maps…" }[recognition.progress?.stage ?? "capturing"])
              + (recognition.progress?.total ? ` ${recognition.progress.completed} / ${recognition.progress.total}` : "")
            : recognition.error || (recognition.result?.keys.length
              ? (recognition.result.keys.length > 1 ? (zh ? "存在相似地图，请点击候选核对；也可继续探索后重新识别。" : "Similar maps remain. Choose one to check, or explore further and recognize again.") : (zh ? "点击候选查看地图；楼层按钮只切换看图。" : "Choose the candidate to view it; floor buttons only change the view."))
              : (zh ? "未找到匹配地图，请在一楼打开地图后重试。" : "No matching map. Open the first-floor map in the game and try again."))}</p>
          {!!recognition.result?.warnings.length && <details><summary>{zh ? "部分识别图片不可用" : "Some recognition images are unavailable"}</summary>{recognition.result.warnings.map((warning, index) => <p key={index}>{warning}</p>)}</details>}
          {recognition.result?.preview && <details className="journal-region-preview"><summary>{zh ? "查看识别区域" : "View recognized region"}</summary><img src={recognition.result.preview} alt={zh ? "本次识别使用的游戏地图区域" : "Game map region used for this recognition"} /></details>}
        </div>}
        {busy ? <div className="empty journal-empty">{zh ? "正在读取地图包…" : "Loading map packs…"}</div>
          : selectedKey !== null ? <div className="journal-detail" key={`detail:${selectedKey}`}>
            <button ref={backButton} type="button" className="journal-back" onClick={() => setSelectedKey(null)}>{zh ? "‹ 返回列表" : "‹ Back to list"}</button>
            {!selectedMap ? <p>{zh ? "当前楼层或显示模式暂无此地图。" : "This map is unavailable for the selected floor or display mode."}</p>
              : detailError ? <p role="alert">{zh ? "图片读取失败：" : "Could not read image: "}{detailError}</p>
              : detailUrl ? <img key={detailKey} src={detailUrl} alt={selectedName} onError={() => setDetailImage({ key: detailKey, url: "", error: zh ? "图片格式无效或已损坏" : "Invalid or damaged image" })} />
              : <p>{zh ? "正在读取地图…" : "Loading map…"}</p>}
          </div>
          : maps.length ? <div className="journal-grid" key={`${settings.season}:${settings.difficulty}:${settings.door}:${settings.floor}:${settings.display}`}>{maps.map((map, index) => {
            const name = mapName(map);
            return <button ref={element => {if (element) mapButtons.current.set(map.key, element); else mapButtons.current.delete(map.key);}} type="button" className="journal-map" style={{ "--map-delay": `${Math.min(index, 7) * 35}ms` } as CSSProperties} key={`${map.key}:${map.image.path}`} onClick={() => setSelectedKey(map.key)} aria-label={zh ? `查看${name}` : `View ${name}`}><Thumbnail image={map.image} name={name} zh={zh} /><span className="journal-map-name">{name}</span></button>;
          })}</div>
          : recognizedKeys ? <div className="empty journal-empty"><p>{recognizing ? (zh ? "正在读取游戏地图，请稍候。" : "Reading the game map. Please wait.") : (zh ? "没有可显示的候选，可重新识别或返回全部地图。" : "No candidates to display. Recognize again or show all maps.")}</p></div>
          : <div className="empty journal-empty"><span className="journal-empty-icon" aria-hidden="true">▤</span><h3>{zh ? "当前组合暂无地图" : "No maps in this group"}</h3><p>{zh ? "请导入地图包，或选择其他赛季、难度、门型及楼层。" : "Import a map pack or choose another season, difficulty, door type or floor."}</p></div>}
        <footer className={`journal-footer${selectedKey !== null ? " journal-detail-footer" : ""}`} aria-live="polite"><div><span>{selectedKey !== null ? selectedName : zh ? `当前组合共 ${maps.length} 张地图` : `${maps.length} maps in this group`}</span><small>{summary}</small></div>
          {selectedKey !== null && <div className="journal-detail-actions">{floorSwitch()}<button ref={enterButton} type="button" className="primary" disabled={!detailUrl || !!detailError || !selectedMap} onClick={() => setPreview("open")}>{zh ? "进入地图" : "Enter map"}</button></div>}
        </footer>
      </section>
    </div>
    {preview !== "closed" && createPortal(<dialog ref={previewDialog} className={`journal-full-preview${preview === "closing" ? " is-closing" : ""}`} aria-label={zh ? `${selectedName}全窗口预览` : `${selectedName} full-window preview`} onCancel={event => {event.preventDefault(); setPreview("closing");}}>
      {detailUrl ? <img key={detailKey} src={detailUrl} alt={selectedName} onLoad={({ currentTarget: image }) => image.style.setProperty("--map-aspect-ratio", String(image.naturalWidth / image.naturalHeight))} onError={() => setDetailImage({ key: detailKey, url: "", error: zh ? "图片格式无效或已损坏" : "Invalid or damaged image" })} />
        : <p className="journal-preview-status" role={detailError ? "alert" : "status"}>{detailError || (zh ? "正在读取地图…" : "Loading map…")}</p>}
      <div className="journal-preview-floors">{floorSwitch()}</div>
      <button ref={closeButton} type="button" className="journal-preview-close" aria-label={zh ? "退出全窗口预览" : "Exit full-window preview"} title={zh ? "退出预览（Esc）" : "Exit preview (Esc)"} onClick={() => setPreview("closing")}>×</button>
    </dialog>, document.body)}
  </div>;
}
