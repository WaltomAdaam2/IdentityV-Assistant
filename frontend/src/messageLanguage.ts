// Backend diagnostics retain both translations. Only presentation selects one;
// raw records, paths, identifiers and third-party messages stay intact.
export function messageLanguage(message: string, language: string): string {
  return message.split("\n").map(line => {
    const separator = line.indexOf(" / ");
    if (separator < 0) return line;
    const chinese = line.slice(0, separator), english = line.slice(separator + 3);
    if (!/[\u3400-\u9fff]/u.test(chinese) || !/^[A-Za-z]/.test(english)) return line;
    if (language !== "zh-CN") return english;
    const suffix = english.match(/: (.+)$/)?.[1];
    const states: Record<string, string> = { checking: "检查中", current: "已是最新版本", available: "发现新版本", downloading: "下载中", ready: "已准备好", error: "失败", idle: "空闲" };
    return suffix && !chinese.includes(suffix) ? `${chinese}：${states[suffix] ?? suffix}` : chinese;
  }).join("\n");
}
