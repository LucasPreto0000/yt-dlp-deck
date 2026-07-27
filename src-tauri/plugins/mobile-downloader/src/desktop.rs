use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<MobileDownloader<R>> {
    Ok(MobileDownloader(app.clone()))
}

/// Access to the mobile-downloader APIs.
pub struct MobileDownloader<R: Runtime>(AppHandle<R>);

impl<R: Runtime> MobileDownloader<R> {
    pub fn check_tools(&self) -> crate::Result<ToolStatus> {
        Err(crate::Error::Unsupported)
    }

    pub fn search_videos(&self, _payload: SearchRequest) -> crate::Result<SearchResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn start_download(&self, _payload: StartDownloadRequest) -> crate::Result<DownloadResult> {
        Err(crate::Error::Unsupported)
    }

    pub fn open_downloads_folder(
        &self,
        _payload: OpenDownloadsRequest,
    ) -> crate::Result<EmptyResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn open_external_url(&self, _payload: OpenUrlRequest) -> crate::Result<EmptyResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn read_clipboard(&self) -> crate::Result<ClipboardResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn control_download(
        &self,
        _payload: DownloadControlRequest,
    ) -> crate::Result<DownloadStateResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn get_download_state(&self) -> crate::Result<DownloadStateResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn get_download_history(&self) -> crate::Result<DownloadHistoryResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn clear_download_history(&self) -> crate::Result<EmptyResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn open_download_item(
        &self,
        _payload: DownloadItemRequest,
    ) -> crate::Result<EmptyResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn share_download_item(
        &self,
        _payload: DownloadItemRequest,
    ) -> crate::Result<EmptyResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn delete_download_item(
        &self,
        _payload: DownloadItemRequest,
    ) -> crate::Result<EmptyResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn get_mobile_settings(&self) -> crate::Result<MobileSettingsResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn request_mobile_permissions(&self) -> crate::Result<MobileSettingsResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn choose_download_directory(&self) -> crate::Result<MobileSettingsResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn choose_cookie_file(&self) -> crate::Result<CookieFileResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn delete_cookie_file(&self) -> crate::Result<EmptyResponse> {
        Err(crate::Error::Unsupported)
    }

    pub fn set_immersive_navigation(
        &self,
        _payload: ImmersiveNavigationRequest,
    ) -> crate::Result<MobileSettingsResponse> {
        Err(crate::Error::Unsupported)
    }
}
