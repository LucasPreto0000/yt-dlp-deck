use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub yt_dlp: bool,
    pub ffmpeg: bool,
    pub yt_dlp_version: Option<String>,
    pub ffmpeg_version: Option<String>,
    pub tools_dir: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub duration: String,
    pub thumbnail: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub items: Vec<SearchResult>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub url: String,
    pub platform_folder: String,
    pub format: String,
    pub quality: Option<String>,
    pub cookies: String,
    pub cookie_file: Option<String>,
    #[serde(default)]
    pub wifi_only: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDownloadRequest {
    pub request: DownloadRequest,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    pub success: bool,
    pub output_dir: String,
    pub message: String,
    pub job_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadControlRequest {
    pub action: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRecord {
    pub id: String,
    pub title: String,
    pub url: String,
    pub status: String,
    pub percent: f64,
    pub message: String,
    pub output_dir: String,
    pub file_uri: Option<String>,
    pub file_name: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub console: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadHistoryResponse {
    pub items: Vec<DownloadRecord>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStateResponse {
    pub active: bool,
    pub paused: bool,
    pub cancelled: bool,
    pub current: Option<DownloadRecord>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileSettingsResponse {
    pub shared_url: Option<String>,
    pub download_directory: String,
    pub notifications_granted: bool,
    pub storage_granted: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieFileResponse {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItemRequest {
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDownloadsRequest {
    pub platform_folder: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenUrlRequest {
    pub url: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyResponse {
    pub ok: bool,
}
