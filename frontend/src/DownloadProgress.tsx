import { useEffect, useRef, useState } from "react";
import { formatRemaining, formatSpeed, sampleDownload, type DownloadSample } from "./transferRate";

export default function DownloadProgress({ downloaded, total, language }: { downloaded: number; total: number | null; language: "zh-CN" | "en-US" }) {
  const latest = useRef({ downloaded, total });
  latest.current = { downloaded, total };
  const [rate, setRate] = useState({ speed: 0, remaining: null as number | null });
  useEffect(() => {
    const samples: DownloadSample[] = [{ time: performance.now(), bytes: latest.current.downloaded }];
    const timer = window.setInterval(() => {
      const current = latest.current;
      setRate(sampleDownload(samples, current.downloaded, current.total, performance.now()));
    }, 500);
    return () => window.clearInterval(timer);
  }, []);
  const progress = total && total > 0 ? Math.min(100, downloaded / total * 100) : undefined;
  const mb = (bytes: number) => `${(bytes / 1048576).toFixed(1)} MB`;
  return <div className="update-download">
    <div className="update-progress-row">
      <progress max="100" value={progress} aria-label={language === "zh-CN" ? "正在下载更新" : "Downloading update"} />
      <small className="update-transfer"><span>{formatSpeed(rate.speed)}</span><span>{formatRemaining(rate.remaining, language === "zh-CN")}</span></small>
    </div>
    <small>{mb(downloaded)}{total ? ` / ${mb(total)}` : ""}{progress !== undefined ? ` · ${Math.floor(progress)}%` : ""}</small>
  </div>;
}
