use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::MobileDownloader;
#[cfg(mobile)]
use mobile::MobileDownloader;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the mobile-downloader APIs.
pub trait MobileDownloaderExt<R: Runtime> {
    fn mobile_downloader(&self) -> &MobileDownloader<R>;
}

impl<R: Runtime, T: Manager<R>> crate::MobileDownloaderExt<R> for T {
    fn mobile_downloader(&self) -> &MobileDownloader<R> {
        self.state::<MobileDownloader<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("mobile-downloader")
        .invoke_handler(tauri::generate_handler![
            commands::check_tools,
            commands::search_videos,
            commands::start_download,
            commands::open_downloads_folder,
            commands::open_external_url
        ])
        .setup(|app, api| {
            #[cfg(mobile)]
            let mobile_downloader = mobile::init(app, api)?;
            #[cfg(desktop)]
            let mobile_downloader = desktop::init(app, api)?;
            app.manage(mobile_downloader);
            Ok(())
        })
        .build()
}
