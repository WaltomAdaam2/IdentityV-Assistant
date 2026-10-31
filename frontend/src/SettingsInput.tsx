import { useEffect, useId, useState, type InputHTMLAttributes } from "react";

type NumberProps = Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "onChange" | "type"> & {
  value: number; onValue: (value: number) => void; language: "zh-CN" | "en-US";
};

export function NumberInput({value, onValue, language, ...props}: NumberProps) {
  const [draft, setDraft] = useState(String(value));
  const [error, setError] = useState("");
  const errorId = useId();
  useEffect(() => { setDraft(String(value)); setError(""); }, [value]);
  return <div className="editable-input"><input {...props} type="number" value={draft}
    aria-invalid={!!error} aria-describedby={[props["aria-describedby"], error ? errorId : ""].filter(Boolean).join(" ") || undefined}
    onChange={event => { setDraft(event.target.value); setError(""); }} onBlur={event => {
      const input = event.currentTarget;
      if (!input.value.trim() || !Number.isFinite(input.valueAsNumber) || !input.validity.valid) {
        setError(language === "zh-CN" ? `请输入有效数值${props.min !== undefined ? `，最小 ${props.min}` : ""}${props.max !== undefined ? `，最大 ${props.max}` : ""}${props.step ? `，步长 ${props.step}` : ""}。` : `Enter a valid number${props.min !== undefined ? `, minimum ${props.min}` : ""}${props.max !== undefined ? `, maximum ${props.max}` : ""}${props.step ? `, step ${props.step}` : ""}.`);
        return;
      }
      setDraft(String(input.valueAsNumber));
      setError(""); onValue(input.valueAsNumber);
    }} />{error && <small className="form-error" id={errorId} role="alert">{error}</small>}</div>;
}

export function KeywordsInput({value, onValue, ...props}: Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "onChange"> & {value: string[]; onValue: (value: string[]) => void}) {
  const serialized = value.join("，");
  const [draft, setDraft] = useState(serialized);
  useEffect(() => setDraft(serialized), [serialized]);
  return <input {...props} value={draft} onChange={event => setDraft(event.target.value)} onBlur={() => {
    const next = draft.split(/[,，]/).map(item => item.trim()).filter(Boolean);
    setDraft(next.join("，")); onValue(next);
  }} />;
}
