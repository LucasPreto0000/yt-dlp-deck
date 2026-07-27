const COMMANDS: &[&str] = &[
    "check_tools",
    "search_videos",
    "start_download",
    "open_downloads_folder",
    "open_external_url",
    "control_download",
    "get_download_state",
    "get_download_history",
    "clear_download_history",
    "open_download_item",
    "share_download_item",
    "delete_download_item",
    "get_mobile_settings",
    "request_mobile_permissions",
    "choose_download_directory",
    "choose_cookie_file",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .ios_path("ios")
        .build();
}
