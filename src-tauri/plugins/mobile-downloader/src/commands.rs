use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::MobileDownloaderExt;
use crate::Result;

#[command]
pub(crate) async fn check_tools<R: Runtime>(app: AppHandle<R>) -> Result<ToolStatus> {
    app.mobile_downloader().check_tools()
}

#[command]
pub(crate) async fn search_videos<R: Runtime>(
    app: AppHandle<R>,
    query: String,
) -> Result<Vec<SearchResult>> {
    app.mobile_downloader()
        .search_videos(SearchRequest { query })
        .map(|response| response.items)
}

#[command]
pub(crate) async fn start_download<R: Runtime>(
    app: AppHandle<R>,
    request: DownloadRequest,
) -> Result<DownloadResult> {
    app.mobile_downloader()
        .start_download(StartDownloadRequest { request })
}

#[command]
pub(crate) async fn open_downloads_folder<R: Runtime>(
    app: AppHandle<R>,
    platform_folder: Option<String>,
) -> Result<()> {
    app.mobile_downloader()
        .open_downloads_folder(OpenDownloadsRequest { platform_folder })
        .map(|_| ())
}

#[command]
pub(crate) async fn open_external_url<R: Runtime>(app: AppHandle<R>, url: String) -> Result<()> {
    app.mobile_downloader()
        .open_external_url(OpenUrlRequest { url })
        .map(|_| ())
}

#[command]
pub(crate) async fn read_clipboard<R: Runtime>(app: AppHandle<R>) -> Result<ClipboardResponse> {
    app.mobile_downloader().read_clipboard()
}

#[command]
pub(crate) async fn control_download<R: Runtime>(
    app: AppHandle<R>,
    action: String,
) -> Result<DownloadStateResponse> {
    app.mobile_downloader()
        .control_download(DownloadControlRequest { action })
}

#[command]
pub(crate) async fn get_download_state<R: Runtime>(
    app: AppHandle<R>,
) -> Result<DownloadStateResponse> {
    app.mobile_downloader().get_download_state()
}

#[command]
pub(crate) async fn get_download_history<R: Runtime>(
    app: AppHandle<R>,
) -> Result<DownloadHistoryResponse> {
    app.mobile_downloader().get_download_history()
}

#[command]
pub(crate) async fn clear_download_history<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    app.mobile_downloader().clear_download_history().map(|_| ())
}

#[command]
pub(crate) async fn open_download_item<R: Runtime>(app: AppHandle<R>, id: String) -> Result<()> {
    app.mobile_downloader()
        .open_download_item(DownloadItemRequest { id })
        .map(|_| ())
}

#[command]
pub(crate) async fn share_download_item<R: Runtime>(app: AppHandle<R>, id: String) -> Result<()> {
    app.mobile_downloader()
        .share_download_item(DownloadItemRequest { id })
        .map(|_| ())
}

#[command]
pub(crate) async fn delete_download_item<R: Runtime>(app: AppHandle<R>, id: String) -> Result<()> {
    app.mobile_downloader()
        .delete_download_item(DownloadItemRequest { id })
        .map(|_| ())
}

#[command]
pub(crate) async fn get_mobile_settings<R: Runtime>(
    app: AppHandle<R>,
) -> Result<MobileSettingsResponse> {
    app.mobile_downloader().get_mobile_settings()
}

#[command]
pub(crate) async fn request_mobile_permissions<R: Runtime>(
    app: AppHandle<R>,
) -> Result<MobileSettingsResponse> {
    app.mobile_downloader().request_mobile_permissions()
}

#[command]
pub(crate) async fn choose_download_directory<R: Runtime>(
    app: AppHandle<R>,
) -> Result<MobileSettingsResponse> {
    app.mobile_downloader().choose_download_directory()
}

#[command]
pub(crate) async fn choose_cookie_file<R: Runtime>(
    app: AppHandle<R>,
) -> Result<CookieFileResponse> {
    app.mobile_downloader().choose_cookie_file()
}
