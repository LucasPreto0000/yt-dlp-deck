use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_mobile_downloader);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<MobileDownloader<R>> {
    #[cfg(target_os = "android")]
    let handle =
        api.register_android_plugin("com.ytdlpdeck.mobiledownloader", "MobileDownloaderPlugin")?;
    #[cfg(target_os = "ios")]
    let handle = api.register_ios_plugin(init_plugin_mobile_downloader)?;
    Ok(MobileDownloader(handle))
}

/// Access to the mobile-downloader APIs.
pub struct MobileDownloader<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> MobileDownloader<R> {
    pub fn check_tools(&self) -> crate::Result<ToolStatus> {
        self.0
            .run_mobile_plugin("checkTools", ())
            .map_err(Into::into)
    }

    pub fn search_videos(&self, payload: SearchRequest) -> crate::Result<SearchResponse> {
        self.0
            .run_mobile_plugin("searchVideos", payload)
            .map_err(Into::into)
    }

    pub fn start_download(&self, payload: StartDownloadRequest) -> crate::Result<DownloadResult> {
        self.0
            .run_mobile_plugin("startDownload", payload)
            .map_err(Into::into)
    }

    pub fn open_downloads_folder(
        &self,
        payload: OpenDownloadsRequest,
    ) -> crate::Result<EmptyResponse> {
        self.0
            .run_mobile_plugin("openDownloadsFolder", payload)
            .map_err(Into::into)
    }

    pub fn open_external_url(&self, payload: OpenUrlRequest) -> crate::Result<EmptyResponse> {
        self.0
            .run_mobile_plugin("openExternalUrl", payload)
            .map_err(Into::into)
    }
}
