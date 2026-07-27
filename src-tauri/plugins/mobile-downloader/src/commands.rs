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
