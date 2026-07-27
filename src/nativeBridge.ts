import { addPluginListener, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const mobileCommands: Record<string, string> = {
  get_tool_status: "check_tools",
  search_videos: "search_videos",
  start_download: "start_download",
  open_downloads_folder: "open_downloads_folder",
  open_external_url: "open_external_url",
  control_download: "control_download",
  get_download_state: "get_download_state",
  get_download_history: "get_download_history",
  clear_download_history: "clear_download_history",
  open_download_item: "open_download_item",
  share_download_item: "share_download_item",
  delete_download_item: "delete_download_item",
  get_mobile_settings: "get_mobile_settings",
  request_mobile_permissions: "request_mobile_permissions",
  choose_download_directory: "choose_download_directory",
  choose_cookie_file: "choose_cookie_file",
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

export async function listenMobilePluginEvent<T>(
  event: "download-state" | "shared-url",
  callback: (payload: T) => void,
): Promise<() => void> {
  if (!isAndroidRuntime) return () => undefined;
  const listener = await addPluginListener<T>("mobile-downloader", event, callback);
  return () => {
    void listener.unregister();
  };
}
