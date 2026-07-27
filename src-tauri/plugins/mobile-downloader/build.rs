const COMMANDS: &[&str] = &[
    "check_tools",
    "search_videos",
    "start_download",
    "open_downloads_folder",
    "open_external_url",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .ios_path("ios")
        .build();
}
