use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
const FFMPEG_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupProgress {
    tool: String,
    message: String,
    percent: Option<u8>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolStatus {
    yt_dlp: bool,
    ffmpeg: bool,
    yt_dlp_version: Option<String>,
    ffmpeg_version: Option<String>,
    tools_dir: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchResult {
    id: String,
    title: String,
    duration: String,
    thumbnail: String,
    url: String,
}

type SearchCache = Mutex<HashMap<String, (Instant, Vec<SearchResult>)>>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DownloadRequest {
    url: String,
    platform_folder: String,
    format: String,
    quality: Option<String>,
    cookies: String,
    cookie_file: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadResult {
    success: bool,
    output_dir: String,
    message: String,
}

fn app_tools_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let _ = app;
    std::env::current_exe()
        .map_err(|e| format!("Não foi possível localizar o aplicativo: {e}"))?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "A pasta do aplicativo não pôde ser identificada.".to_owned())
}

fn find_tool(app: &AppHandle, file_name: &str) -> Result<Option<PathBuf>, String> {
    let candidate = app_tools_dir(app)?.join(file_name);
    Ok(candidate.is_file().then_some(candidate))
}

fn require_tool(app: &AppHandle, file_name: &str) -> Result<PathBuf, String> {
    find_tool(app, file_name)?.ok_or_else(|| {
        format!(
            "{file_name} não foi encontrado. Coloque-o na mesma pasta do aplicativo e tente novamente."
        )
    })
}

fn downloads_root(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .download_dir()
        .map(|path| path.join("YT-DLP Deck"))
        .map_err(|e| format!("Não foi possível localizar a pasta Downloads: {e}"))
}

fn emit_setup(app: &AppHandle, tool: &str, message: &str, percent: Option<u8>) {
    let _ = app.emit(
        "setup-progress",
        SetupProgress {
            tool: tool.to_owned(),
            message: message.to_owned(),
            percent,
        },
    );
}

async fn copy_local_tool_if_available(destination: &Path, file_name: &str) -> bool {
    let mut candidates = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        candidates.push(current.join(file_name));
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            candidates.push(parent.join(file_name));
        }
    }

    for source in candidates {
        if source.is_file() && fs::copy(&source, destination).await.is_ok() {
            return true;
        }
    }
    false
}

async fn download_file(
    app: &AppHandle,
    tool: &str,
    url: &str,
    destination: &Path,
) -> Result<(), String> {
    let temporary = destination.with_extension("download");
    let response = reqwest::Client::new()
        .get(url)
        .header("User-Agent", "YT-DLP-Deck/1.0")
        .send()
        .await
        .map_err(|e| format!("Falha ao conectar para baixar {tool}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("O servidor recusou o download de {tool}: {e}"))?;
    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut file = fs::File::create(&temporary)
        .await
        .map_err(|e| format!("Falha ao criar o arquivo de {tool}: {e}"))?;
    let mut received = 0_u64;
    let mut last_percent = 255_u8;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download de {tool} interrompido: {e}"))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| format!("Falha ao salvar {tool}: {e}"))?;
        received += chunk.len() as u64;
        let percent = total.map(|size| ((received * 100 / size).min(100)) as u8);
        if let Some(value) = percent {
            if value != last_percent {
                last_percent = value;
                emit_setup(
                    app,
                    tool,
                    &format!("Baixando {tool}… {value}%"),
                    Some(value),
                );
            }
        }
    }
    tokio::io::AsyncWriteExt::flush(&mut file)
        .await
        .map_err(|e| format!("Falha ao finalizar {tool}: {e}"))?;
    drop(file);
    fs::rename(&temporary, destination)
        .await
        .map_err(|e| format!("Falha ao instalar {tool}: {e}"))
}

fn extract_ffmpeg_archive(archive_path: &Path, output_dir: &Path) -> Result<(), String> {
    let file =
        File::open(archive_path).map_err(|e| format!("Falha ao abrir o pacote do FFmpeg: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Pacote do FFmpeg inválido: {e}"))?;
    let wanted = ["ffmpeg.exe", "ffprobe.exe"];
    let mut extracted = 0;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| format!("Falha ao ler o pacote do FFmpeg: {e}"))?;
        let normalized = entry.name().replace('\\', "/");
        let Some(file_name) = wanted
            .iter()
            .find(|name| normalized.ends_with(&format!("/bin/{name}")))
        else {
            continue;
        };
        let output = output_dir.join(file_name);
        let mut target =
            File::create(&output).map_err(|e| format!("Falha ao criar {file_name}: {e}"))?;
        io::copy(&mut entry, &mut target)
            .map_err(|e| format!("Falha ao extrair {file_name}: {e}"))?;
        target
            .flush()
            .map_err(|e| format!("Falha ao finalizar {file_name}: {e}"))?;
        extracted += 1;
    }

    if extracted == 0 {
        return Err("O pacote baixado não continha ffmpeg.exe.".to_owned());
    }
    Ok(())
}

async fn ensure_tools_internal(app: &AppHandle) -> Result<ToolStatus, String> {
    let directory = app_tools_dir(app)?;
    fs::create_dir_all(&directory)
        .await
        .map_err(|e| format!("Falha ao criar a pasta das ferramentas: {e}"))?;
    let yt_dlp = directory.join("yt-dlp.exe");
    let ffmpeg = directory.join("ffmpeg.exe");

    if !yt_dlp.is_file() {
        emit_setup(app, "yt-dlp", "Preparando yt-dlp…", Some(0));
        if !copy_local_tool_if_available(&yt_dlp, "yt-dlp.exe").await {
            download_file(app, "yt-dlp", YTDLP_URL, &yt_dlp).await?;
        }
        emit_setup(app, "yt-dlp", "yt-dlp instalado", Some(100));
    }

    if !ffmpeg.is_file() {
        emit_setup(app, "ffmpeg", "Preparando FFmpeg…", Some(0));
        if !copy_local_tool_if_available(&ffmpeg, "ffmpeg.exe").await {
            let archive = directory.join("ffmpeg.zip");
            download_file(app, "ffmpeg", FFMPEG_URL, &archive).await?;
            emit_setup(app, "ffmpeg", "Extraindo FFmpeg…", None);
            let archive_for_task = archive.clone();
            let directory_for_task = directory.clone();
            tokio::task::spawn_blocking(move || {
                extract_ffmpeg_archive(&archive_for_task, &directory_for_task)
            })
            .await
            .map_err(|e| format!("Falha interna ao extrair FFmpeg: {e}"))??;
            let _ = fs::remove_file(&archive).await;
        }
        emit_setup(app, "ffmpeg", "FFmpeg instalado", Some(100));
    }

    get_tool_status_internal(app).await
}

async fn get_tool_status_internal(app: &AppHandle) -> Result<ToolStatus, String> {
    let yt_dlp_path = find_tool(app, "yt-dlp.exe")?;
    let ffmpeg_path = find_tool(app, "ffmpeg.exe")?;
    let yt_dlp = yt_dlp_path.is_some();
    let ffmpeg = ffmpeg_path.is_some();
    let directory = yt_dlp_path
        .as_ref()
        .or(ffmpeg_path.as_ref())
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .unwrap_or(std::env::current_dir().unwrap_or_default());
    Ok(ToolStatus {
        yt_dlp,
        ffmpeg,
        yt_dlp_version: None,
        ffmpeg_version: None,
        tools_dir: directory.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
async fn ensure_tools(app: AppHandle) -> Result<ToolStatus, String> {
    ensure_tools_internal(&app).await
}

#[tauri::command]
async fn get_tool_status(app: AppHandle) -> Result<ToolStatus, String> {
    get_tool_status_internal(&app).await
}

#[tauri::command]
async fn search_videos(app: AppHandle, query: String) -> Result<Vec<SearchResult>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    static SEARCH_CACHE: OnceLock<SearchCache> = OnceLock::new();
    let cache = SEARCH_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let cache_key = query.to_lowercase();
    if let Ok(entries) = cache.lock() {
        if let Some((created, results)) = entries.get(&cache_key) {
            if created.elapsed() < Duration::from_secs(300) {
                return Ok(results.clone());
            }
        }
    }

    let executable = require_tool(&app, "yt-dlp.exe")?;
    let mut command = Command::new(executable);
    command.args([
        "--no-config",
        "--flat-playlist",
        "--lazy-playlist",
        "--playlist-end",
        "5",
        "--dump-json",
        "--no-warnings",
        &format!("ytsearch5:{query}"),
    ]);
    hide_console(&mut command);
    let output = command
        .output()
        .await
        .map_err(|e| format!("Não foi possível executar a busca: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }

    let mut results = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(item) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let id = item["id"].as_str().unwrap_or_default().to_owned();
        let url = item["webpage_url"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| (!id.is_empty()).then(|| format!("https://www.youtube.com/watch?v={id}")))
            .unwrap_or_default();
        results.push(SearchResult {
            id,
            title: item["title"].as_str().unwrap_or("Sem título").to_owned(),
            duration: item["duration_string"]
                .as_str()
                .unwrap_or("duração indisponível")
                .to_owned(),
            thumbnail: item["thumbnail"]
                .as_str()
                .or_else(|| {
                    item["thumbnails"]
                        .as_array()
                        .and_then(|thumbnails| thumbnails.last())
                        .and_then(|thumbnail| thumbnail["url"].as_str())
                })
                .unwrap_or_default()
                .to_owned(),
            url,
        });
    }
    if let Ok(mut entries) = cache.lock() {
        entries.retain(|_, (created, _)| created.elapsed() < Duration::from_secs(300));
        if entries.len() >= 24 {
            entries.clear();
        }
        entries.insert(cache_key, (Instant::now(), results.clone()));
    }
    Ok(results)
}

fn safe_folder_name(input: &str) -> Result<String, String> {
    let value = input.trim();
    if value.is_empty() || value == "." || value == ".." {
        return Err("Escolha um nome válido para a pasta da plataforma.".to_owned());
    }
    let sanitized: String = value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            ch if ch.is_control() => '_',
            _ => ch,
        })
        .collect();
    let sanitized = sanitized.trim_end_matches([' ', '.']).to_owned();
    if sanitized.is_empty() {
        Err("Escolha um nome válido para a pasta da plataforma.".to_owned())
    } else {
        Ok(sanitized)
    }
}

fn quality_selector(quality: Option<&str>) -> Option<&'static str> {
    match quality {
        Some("best") => Some("bv*+ba/b"),
        Some("2160p60") => Some("bv*[height<=2160][fps>=60]+ba/bv*[height<=2160]+ba/b"),
        Some("1440p60") => Some("bv*[height<=1440][fps>=60]+ba/bv*[height<=1440]+ba/b"),
        Some("1080p60") => Some("bv*[height<=1080][fps>=60]+ba/bv*[height<=1080]+ba/b"),
        Some("1080p") => Some("bv*[height<=1080]+ba/b"),
        Some("720p") => Some("bv*[height<=720]+ba/b"),
        Some("480p") => Some("bv*[height<=480]+ba/b"),
        _ => None,
    }
}

fn build_download_args(
    request: &DownloadRequest,
    output_template: &Path,
    ffmpeg: &Path,
) -> Vec<String> {
    let mut args = vec![
        request.url.trim().to_owned(),
        "-o".to_owned(),
        output_template.to_string_lossy().into_owned(),
        "--no-playlist".to_owned(),
        "--add-metadata".to_owned(),
        "--newline".to_owned(),
        "--concurrent-fragments".to_owned(),
        "4".to_owned(),
        "--ffmpeg-location".to_owned(),
        ffmpeg.to_string_lossy().into_owned(),
    ];

    match request.cookies.as_str() {
        "chrome" | "edge" | "firefox" => {
            args.extend(["--cookies-from-browser".to_owned(), request.cookies.clone()]);
        }
        "file" => {
            if let Some(path) = request
                .cookie_file
                .as_deref()
                .filter(|path| !path.trim().is_empty())
            {
                args.extend(["--cookies".to_owned(), path.to_owned()]);
            }
        }
        _ => {}
    }

    match request.format.as_str() {
        "mp3" | "flac" | "wav" | "m4a" => args.extend([
            "-x".to_owned(),
            "--audio-format".to_owned(),
            request.format.clone(),
            "--audio-quality".to_owned(),
            "0".to_owned(),
        ]),
        format => {
            if let Some(selector) = quality_selector(request.quality.as_deref()) {
                args.extend(["-f".to_owned(), selector.to_owned()]);
            }
            let container = if format == "best" { "mkv" } else { format };
            if matches!(container, "mp4" | "mkv" | "webm") {
                args.extend(["--merge-output-format".to_owned(), container.to_owned()]);
            }
        }
    }
    args
}

#[tauri::command]
async fn start_download(
    app: AppHandle,
    request: DownloadRequest,
) -> Result<DownloadResult, String> {
    if request.url.trim().is_empty() {
        return Err("Informe uma URL antes de iniciar o download.".to_owned());
    }
    let status = get_tool_status_internal(&app).await?;
    if !status.yt_dlp || !status.ffmpeg {
        return Err("yt-dlp e FFmpeg precisam estar instalados.".to_owned());
    }

    let platform = safe_folder_name(&request.platform_folder)?;
    let output_dir = downloads_root(&app)?.join(platform);
    fs::create_dir_all(&output_dir)
        .await
        .map_err(|e| format!("Falha ao criar a pasta de destino: {e}"))?;
    let output_template = output_dir.join("%(uploader)s - %(title)s [%(id)s].%(ext)s");
    let executable = require_tool(&app, "yt-dlp.exe")?;
    let ffmpeg = require_tool(&app, "ffmpeg.exe")?;
    let args = build_download_args(&request, &output_template, &ffmpeg);

    let mut command = Command::new(executable);
    command
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_console(&mut command);
    let mut child = command
        .spawn()
        .map_err(|e| format!("Não foi possível iniciar o yt-dlp: {e}"))?;
    let stdout = child.stdout.take().ok_or("Saída do yt-dlp indisponível")?;
    let stderr = child.stderr.take().ok_or("Erros do yt-dlp indisponíveis")?;
    let out_app = app.clone();
    let err_app = app.clone();

    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = out_app.emit("download-output", line);
        }
    });
    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = err_app.emit("download-output", line);
        }
    });

    let exit = child
        .wait()
        .await
        .map_err(|e| format!("Falha enquanto o yt-dlp estava executando: {e}"))?;
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    let output_dir_text = output_dir.to_string_lossy().into_owned();

    let history_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao localizar o histórico: {e}"))?;
    let _ = fs::create_dir_all(&history_dir).await;
    let history_line = format!(
        "Plataforma: {} | Formato: {} | URL: {}\n",
        request.platform_folder,
        request.format,
        request.url.trim()
    );
    let history_path = history_dir.join("historico_downloads.txt");
    if let Ok(mut history) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path)
    {
        let _ = history.write_all(history_line.as_bytes());
    }

    if exit.success() {
        Ok(DownloadResult {
            success: true,
            output_dir: output_dir_text,
            message: "Download concluído com sucesso.".to_owned(),
        })
    } else {
        Err(format!(
            "O yt-dlp terminou com erro (código {}). Consulte o log exibido no aplicativo.",
            exit.code().unwrap_or(-1)
        ))
    }
}

#[tauri::command]
async fn open_downloads_folder(
    app: AppHandle,
    platform_folder: Option<String>,
) -> Result<(), String> {
    let mut destination = downloads_root(&app)?;
    if let Some(folder) = platform_folder
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        destination.push(safe_folder_name(folder)?);
    }
    fs::create_dir_all(&destination)
        .await
        .map_err(|e| format!("Falha ao criar a pasta: {e}"))?;
    let mut command = Command::new("explorer.exe");
    command.arg(&destination);
    hide_console(&mut command);
    command
        .spawn()
        .map_err(|e| format!("Não foi possível abrir a pasta: {e}"))?;
    Ok(())
}

#[cfg(windows)]
fn hide_console(command: &mut Command) {
    command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_command: &mut Command) {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ensure_tools,
            get_tool_status,
            search_videos,
            start_download,
            open_downloads_folder
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o YT-DLP Deck");
}
