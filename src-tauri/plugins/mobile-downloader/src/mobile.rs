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

    pub fn read_clipboard(&self) -> crate::Result<ClipboardResponse> {
        self.0
            .run_mobile_plugin("readClipboard", ())
            .map_err(Into::into)
    }

    pub fn control_download(
        &self,
        payload: DownloadControlRequest,
    ) -> crate::Result<DownloadStateResponse> {
        self.0
            .run_mobile_plugin("controlDownload", payload)
            .map_err(Into::into)
    }

    pub fn get_download_state(&self) -> crate::Result<DownloadStateResponse> {
        self.0
            .run_mobile_plugin("getDownloadState", ())
            .map_err(Into::into)
    }

    pub fn get_download_history(&self) -> crate::Result<DownloadHistoryResponse> {
        self.0
            .run_mobile_plugin("getDownloadHistory", ())
            .map_err(Into::into)
    }

    pub fn clear_download_history(&self) -> crate::Result<EmptyResponse> {
        self.0
            .run_mobile_plugin("clearDownloadHistory", ())
            .map_err(Into::into)
    }

    pub fn open_download_item(&self, payload: DownloadItemRequest) -> crate::Result<EmptyResponse> {
        self.0
            .run_mobile_plugin("openDownloadItem", payload)
            .map_err(Into::into)
    }

    pub fn share_download_item(
        &self,
        payload: DownloadItemRequest,
    ) -> crate::Result<EmptyResponse> {
        self.0
            .run_mobile_plugin("shareDownloadItem", payload)
            .map_err(Into::into)
    }

    pub fn delete_download_item(
        &self,
        payload: DownloadItemRequest,
    ) -> crate::Result<EmptyResponse> {
        self.0
            .run_mobile_plugin("deleteDownloadItem", payload)
            .map_err(Into::into)
    }

    pub fn get_mobile_settings(&self) -> crate::Result<MobileSettingsResponse> {
        self.0
            .run_mobile_plugin("getMobileSettings", ())
            .map_err(Into::into)
    }

    pub fn request_mobile_permissions(&self) -> crate::Result<MobileSettingsResponse> {
        self.0
            .run_mobile_plugin("requestMobilePermissions", ())
            .map_err(Into::into)
    }

    pub fn choose_download_directory(&self) -> crate::Result<MobileSettingsResponse> {
        self.0
            .run_mobile_plugin("chooseDownloadDirectory", ())
            .map_err(Into::into)
    }

    pub fn choose_cookie_file(&self) -> crate::Result<CookieFileResponse> {
        self.0
            .run_mobile_plugin("chooseCookieFile", ())
            .map_err(Into::into)
    }

    pub fn delete_cookie_file(&self) -> crate::Result<EmptyResponse> {
        self.0
            .run_mobile_plugin("deleteCookieFile", ())
            .map_err(Into::into)
    }

    pub fn set_immersive_navigation(
        &self,
        payload: ImmersiveNavigationRequest,
    ) -> crate::Result<MobileSettingsResponse> {
        self.0
            .run_mobile_plugin("setImmersiveNavigation", payload)
            .map_err(Into::into)
    }
}
