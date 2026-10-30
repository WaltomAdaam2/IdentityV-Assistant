export interface DownloadSample { time: number; bytes: number }

// Keep a five-second window, including the sample just before its boundary.
// Sampling continues without new chunks so a stalled download reaches 0 B/s.
export function sampleDownload(samples: DownloadSample[], bytes: number, total: number | null, now: number) {
  if (samples.length && (bytes < samples[samples.length - 1].bytes || now < samples[samples.length - 1].time)) samples.length = 0;
  samples.push({ time: now, bytes });
  while (samples.length > 2 && samples[1].time <= now - 5000) samples.shift();
  const elapsed = (now - samples[0].time) / 1000;
  const speed = elapsed > 0 ? Math.max(0, bytes - samples[0].bytes) / elapsed : 0;
  const remaining = total !== null && total > 0 && speed > 0 ? Math.ceil(Math.max(0, total - bytes) / speed) : null;
  return { speed, remaining };
}

export function formatSpeed(speed: number) {
  if (speed >= 1048576) return `${(speed / 1048576).toFixed(1)} MB/s`;
  if (speed >= 1024) return `${(speed / 1024).toFixed(1)} KB/s`;
  return `${Math.round(speed)} B/s`;
}

export function formatRemaining(seconds: number | null, zh: boolean) {
  if (seconds === null) return zh ? "剩余时间待估算" : "Time remaining unknown";
  const minutes = Math.ceil(seconds / 60);
  const time = seconds < 60 ? `${seconds}${zh ? " 秒" : "s"}` : minutes < 60 ? `${minutes}${zh ? " 分钟" : "m"}` : `${Math.floor(minutes / 60)}${zh ? " 小时 " : "h "}${minutes % 60}${zh ? " 分钟" : "m"}`;
  return zh ? `预计剩余 ${time}` : `About ${time} remaining`;
}
