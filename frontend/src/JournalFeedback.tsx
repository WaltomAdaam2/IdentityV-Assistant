import { useState } from "react";
import { createPortal } from "react-dom";
import { journalApi } from "./journalData";
import { useDialog } from "./useDialog";
import "./updates.css";
import "./support.css";

export default function JournalFeedback({requestId, maps, zh, onNotice}: {
  requestId: string; maps: {key: string; name: string}[]; zh: boolean;
  onNotice: (message: string, error?: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  const [verdict, setVerdict] = useState("unsure");
  const [correctKey, setCorrectKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const {dialog, closing, close} = useDialog(open, () => setOpen(false));
  return <>
    <button type="button" onClick={() => {setError(""); setOpen(true);}}>{zh ? "反馈并保存至诊断包" : "Save feedback for diagnostics"}</button>
    {open && createPortal(<dialog ref={dialog} data-closing={closing || undefined} className="update-dialog support-dialog" aria-labelledby="journal-feedback-title" aria-busy={busy}
      onCancel={event => {event.preventDefault(); if (!busy) close();}}>
      <button type="button" className="update-dialog-close" disabled={busy} aria-label={zh ? "关闭反馈" : "Close feedback"} onClick={() => close()}>×</button>
      <header className="update-dialog-heading"><div><h2 id="journal-feedback-title">{zh ? "反馈并保存至诊断包" : "Save feedback for diagnostics"}</h2>
        <p>{zh ? "保存在本机的本次识图记录中，下次导出诊断包时包含；不会自动发送。" : "Saved with this recognition on your computer and included in the next diagnostic export. Nothing is sent automatically."}</p></div></header>
      <form onSubmit={event => {event.preventDefault(); setBusy(true); setError(""); void journalApi.feedback(requestId, verdict, verdict === "incorrect" && correctKey ? correctKey : null)
        .then(() => close(() => onNotice(zh ? "反馈已保存，下次导出诊断包时包含。" : "Feedback saved for your next diagnostic export.")))
        .catch(reason => setError(String(reason))).finally(() => setBusy(false));}}>
        <fieldset className="journal-feedback-options" disabled={busy}><legend>{zh ? "第一候选是否正确？无候选时也可标记不正确。" : "Is the first candidate correct? You can also flag a missing result."}</legend>
          {[["correct", zh ? "正确" : "Correct"], ["incorrect", zh ? "不正确" : "Incorrect"], ["unsure", zh ? "暂不确定" : "Not sure"]].map(([value, label]) => <label key={value}><input type="radio" name="recognition-verdict" value={value} checked={verdict === value} onChange={() => setVerdict(value)} />{label}</label>)}
        </fieldset>
        {verdict === "incorrect" && <label className="journal-feedback-map">{zh ? "正确地图（可选）" : "Correct map (optional)"}<select disabled={busy} value={correctKey} onChange={event => setCorrectKey(event.target.value)}><option value="">{zh ? "暂时不知道" : "I don't know yet"}</option>{maps.map(map => <option key={map.key} value={map.key}>{map.name}</option>)}</select></label>}
        {error && <p role="alert" className="support-hint">{error}</p>}
        <footer className="update-actions"><button type="button" disabled={busy} onClick={() => close()}>{zh ? "取消" : "Cancel"}</button><button type="submit" className="primary" disabled={busy}>{busy ? (zh ? "正在保存…" : "Saving…") : (zh ? "保存反馈" : "Save feedback")}</button></footer>
      </form>
    </dialog>, document.body)}
  </>;
}
