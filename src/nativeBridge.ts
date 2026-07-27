import { addPluginListener, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const mobileCommands: Record<string, string> = {
  get_tool_status: "check_tools",
  search_videos: "search_videos",
  start_download: "start_download",
  open_downloads_folder: "open_downloads_folder",
  open_external_url: "open_external_url",
};

export const isAndroidRuntime =
  typeof navigator !== "undefined" && /Android/i.test(navigator.userAgent);

export function appInvoke<T>(
  command: keyof typeof mobileCommands,
  args?: Record<string, unknown>,
): Promise<T> {
  const target = isAndroidRuntime
    ? `plugin:mobile-downloader|${mobileCommands[command]}`
    : command;
  return invoke<T>(target, args);
}

export async function listenDownloadOutput(
  callback: (line: string) => void,
): Promise<() => void> {
  if (isAndroidRuntime) {
    const listener = await addPluginListener<{ line?: string }>(
      "mobile-downloader",
      "download-output",
      (payload) => callback(String(payload?.line || "")),
    );
    return () => {
      void listener.unregister();
    };
  }
  return listen<string>("download-output", ({ payload }) => callback(String(payload || "")));
}
