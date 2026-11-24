import type { MobileDownloadRecord } from "../../types";

export type ToastTone = "info" | "success" | "error";
export type ToolId = "yt-dlp" | "ffmpeg";
export const TOOL_UPDATE_CACHE_MS = 60_000;

export interface ToastItem {
  id: number;
  tone: ToastTone;
  title: string;
  message: string;
}

export interface DesktopHistoryRecord {
  id: string;
  title: string;
  url: string;
  status: "completed" | "failed";
  percent: number;
  message: string;
  outputDir: string;
  createdAt: number;
}

export type HistoryRecord = Pick<
  MobileDownloadRecord,
  "id" | "title" | "url" | "status" | "percent" | "message" | "outputDir" | "createdAt"
> & {
  fileUri?: string | null;
};

export type DownloadVisualStatus =
  | "idle"
  | "queued"
  | "running"
  | "paused"
  | "processing"
  | "saving"
  | "completed"
  | "failed"
  | "cancelled";

export interface DownloadTelemetry {
  size: string;
  speed: string;
  eta: string;
}
