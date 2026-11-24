import type { DownloadTelemetry } from "./uiTypes";

export function getConsoleLineTone(line: string) {
  const normalized = line.toLowerCase();
  if (normalized.includes("error") || normalized.includes("erro:") || normalized.includes("failed")) {
    return "error";
  }
  if (normalized.includes("warning") || normalized.includes("aviso")) return "warning";
  if (normalized.includes("[download]") || normalized.includes("%")) return "progress";
  if (
    normalized.includes("[info]") ||
    normalized.includes("[sistema]") ||
    normalized.includes("destination")
  ) {
    return "info";
  }
  return "default";
}

export function readDownloadTelemetry(lines: string[]): DownloadTelemetry {
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const line = lines[index];
    if (!line.includes("[download]")) continue;
    const size = line.match(/\bof\s+~?([0-9.]+\s*[KMGT]i?B)/i)?.[1] || "Calculando";
    const speed = line.match(/\bat\s+([0-9.]+\s*[KMGT]i?B\/s)/i)?.[1] || "Calculando";
    const eta = line.match(/\bETA\s+([0-9:]+)/i)?.[1] || "—";
    return { size, speed, eta };
  }
  return { size: "Calculando", speed: "Calculando", eta: "—" };
}
