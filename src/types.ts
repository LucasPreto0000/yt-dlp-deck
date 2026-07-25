export type SourceMode = "url" | "search";
export type FormatId = "mp4" | "mkv" | "webm" | "best" | "mp3" | "flac" | "wav" | "m4a";
export type QualityId = "best" | "2160p60" | "1440p60" | "1080p60" | "1080p" | "720p" | "480p";
export type CookieId = "none" | "chrome" | "edge" | "firefox" | "file";

export interface ToolStatus {
  ytDlp: boolean;
  ffmpeg: boolean;
  ytDlpVersion?: string;
  ffmpegVersion?: string;
  toolsDir: string;
}

export interface SetupProgress {
  tool: string;
  message: string;
  percent?: number | null;
}

export interface SearchResult {
  id: string;
  title: string;
  duration: string;
  thumbnail: string;
  url: string;
}

export interface DownloadResult {
  success: boolean;
  outputDir: string;
  message: string;
}

export interface DownloadRequest {
  url: string;
  platformFolder: string;
  format: FormatId;
  quality: QualityId;
  cookies: CookieId;
  cookieFile: string | null;
}
