import type { FormatId } from "../../types";

export const errorText = (error: unknown) => (error instanceof Error ? error.message : String(error));
export const isAudio = (format: FormatId | null) =>
  format === "mp3" || format === "flac" || format === "wav" || format === "m4a";
export const isImage = (format: FormatId | null) => format === "image";
export const supportsImageCover = (platform: string | null) =>
  platform === "TikTok" || platform === "Instagram";
