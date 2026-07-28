use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
    sync::RwLock,
};

const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
const FFMPEG_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
const FFMPEG_VERSION_URL: &str =
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip.ver";
const FFMPEG_CHECKSUM_URL: &str =
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip.sha256";
const FFMPEG_GIT_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-git-essentials.7z";
const FFMPEG_GIT_VERSION_URL: &str =
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-git-essentials.7z.ver";
const FFMPEG_GIT_CHECKSUM_URL: &str =
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-git-essentials.7z.sha256";
const MAX_TOOL_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
const FFMPEG_TRANSACTION_FILE: &str = "ffmpeg-update.transaction";
static TOOL_OPERATION_LOCK: OnceLock<RwLock<()>> = OnceLock::new();

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolUpdateResult {
    tool: String,
    status: String,
    updated: bool,
    previous_version: Option<String>,
    current_version: Option<String>,
    message: String,
}

#[derive(Deserialize, Serialize)]
struct FfmpegUpdateTransaction {
    original_files: Vec<String>,
}

#[derive(Clone, Copy)]
struct FfmpegUpdateSource {
    archive_url: &'static str,
    version_url: &'static str,
    checksum_url: &'static str,
    archive_extension: &'static str,
    git_build: bool,
}

fn tool_operation_lock() -> &'static RwLock<()> {
    TOOL_OPERATION_LOCK.get_or_init(|| RwLock::new(()))
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

fn find_runtime(app: &AppHandle, file_name: &str) -> Option<PathBuf> {
    if let Ok(directory) = app_tools_dir(app) {
        let local = directory.join(file_name);
        if local.is_file() {
            return Some(local);
        }
    }
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|directory| directory.join(file_name))
            .find(|candidate| candidate.is_file())
    })
}

fn javascript_runtime(app: &AppHandle) -> Option<String> {
    find_runtime(app, "deno.exe")
        .map(|path| format!("deno:{}", path.to_string_lossy()))
        .or_else(|| {
            find_runtime(app, "node.exe").map(|path| format!("node:{}", path.to_string_lossy()))
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
    emit_progress: bool,
) -> Result<(), String> {
    let temporary = destination.with_extension("download");
    let request = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Falha ao preparar o download de {tool}: {e}"))?
        .get(url)
        .header(
            "User-Agent",
            format!("YT-DLP-Deck/{}", env!("CARGO_PKG_VERSION")),
        );
    let response = tokio::time::timeout(Duration::from_secs(45), request.send())
        .await
        .map_err(|_| format!("O servidor de {tool} demorou mais que o esperado."))?
        .map_err(|e| format!("Falha ao conectar para baixar {tool}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("O servidor recusou o download de {tool}: {e}"))?;
    let total = response.content_length();
    if total.is_some_and(|size| size > MAX_TOOL_DOWNLOAD_BYTES) {
        return Err(format!(
            "O pacote de {tool} é maior que o limite de segurança."
        ));
    }
    let mut stream = response.bytes_stream();
    let mut file = fs::File::create(&temporary)
        .await
        .map_err(|e| format!("Falha ao criar o arquivo de {tool}: {e}"))?;
    let mut received = 0_u64;
    let mut last_percent = 255_u8;

    loop {
        let next_chunk = tokio::time::timeout(Duration::from_secs(90), stream.next())
            .await
            .map_err(|_| format!("O download de {tool} ficou sem responder por muito tempo."))?;
        let Some(chunk) = next_chunk else {
            break;
        };
        let chunk = chunk.map_err(|e| format!("Download de {tool} interrompido: {e}"))?;
        received = received.saturating_add(chunk.len() as u64);
        if received > MAX_TOOL_DOWNLOAD_BYTES {
            return Err(format!(
                "O pacote de {tool} ultrapassou o limite de segurança."
            ));
        }
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| format!("Falha ao salvar {tool}: {e}"))?;
        let percent = total
            .filter(|size| *size > 0)
            .map(|size| ((received * 100 / size).min(100)) as u8);
        if let Some(value) = percent {
            if emit_progress && value != last_percent {
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
    if total.is_some_and(|size| size != received) {
        return Err(format!(
            "O download de {tool} terminou incompleto ({received} de {} bytes).",
            total.unwrap_or_default()
        ));
    }
    tokio::io::AsyncWriteExt::flush(&mut file)
        .await
        .map_err(|e| format!("Falha ao finalizar {tool}: {e}"))?;
    drop(file);
    fs::rename(&temporary, destination)
        .await
        .map_err(|e| format!("Falha ao instalar {tool}: {e}"))
}

async fn fetch_small_text(url: &str, label: &str) -> Result<String, String> {
    let response = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|e| format!("Falha ao preparar a consulta de {label}: {e}"))?
        .get(url)
        .header(
            "User-Agent",
            format!("YT-DLP-Deck/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .map_err(|e| format!("Falha ao consultar {label}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("O servidor recusou a consulta de {label}: {e}"))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Falha ao ler a resposta de {label}: {e}"))?;
    if bytes.len() > 4096 {
        return Err(format!("A resposta de {label} é maior que o esperado."));
    }
    let value = String::from_utf8_lossy(&bytes).trim().to_owned();
    if value.is_empty() {
        Err(format!("O servidor não informou {label}."))
    } else {
        Ok(value)
    }
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|e| format!("Falha ao abrir o pacote para validação: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Falha ao validar o pacote baixado: {e}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

async fn command_output(
    executable: &Path,
    args: &[&str],
    timeout_seconds: u64,
) -> Result<String, String> {
    let mut command = Command::new(executable);
    command
        .args(args)
        .kill_on_drop(true)
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_console(&mut command);
    let output = tokio::time::timeout(Duration::from_secs(timeout_seconds), command.output())
        .await
        .map_err(|_| "A verificação demorou mais que o esperado.".to_owned())?
        .map_err(|e| format!("Não foi possível executar a ferramenta: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if !output.status.success() {
        let detail = if stderr.is_empty() { stdout } else { stderr };
        return Err(if detail.is_empty() {
            format!(
                "A ferramenta terminou com código {}.",
                output.status.code().unwrap_or(-1)
            )
        } else {
            detail
        });
    }
    Ok(if stdout.is_empty() { stderr } else { stdout })
}

async fn yt_dlp_version(path: &Path) -> Result<String, String> {
    command_output(path, &["--no-config", "--version"], 20)
        .await
        .and_then(|output| {
            output
                .lines()
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim().to_owned())
                .ok_or_else(|| "O yt-dlp não informou a versão instalada.".to_owned())
        })
}

fn parse_media_tool_version(output: &str, tool: &str) -> Option<String> {
    let prefix = format!("{tool} version ");
    output.lines().find_map(|line| {
        line.trim()
            .strip_prefix(&prefix)
            .and_then(|value| value.split_whitespace().next())
            .map(str::to_owned)
    })
}

fn parse_ffmpeg_version(output: &str) -> Option<String> {
    parse_media_tool_version(output, "ffmpeg")
}

async fn ffmpeg_version(path: &Path) -> Result<String, String> {
    command_output(path, &["-hide_banner", "-version"], 20)
        .await
        .and_then(|output| {
            parse_ffmpeg_version(&output)
                .ok_or_else(|| "O FFmpeg não informou a versão instalada.".to_owned())
        })
}

async fn ffprobe_version(path: &Path) -> Result<String, String> {
    command_output(path, &["-hide_banner", "-version"], 20)
        .await
        .and_then(|output| {
            parse_media_tool_version(&output, "ffprobe")
                .ok_or_else(|| "O FFprobe não informou a versão instalada.".to_owned())
        })
}

fn ffmpeg_release_matches(local: &str, remote: &str) -> bool {
    local == remote
        || local
            .strip_prefix('n')
            .unwrap_or(local)
            .starts_with(&format!("{remote}-"))
}

fn release_version_parts(version: &str) -> Option<Vec<u32>> {
    let normalized = version.strip_prefix('n').unwrap_or(version);
    let prefix: String = normalized
        .chars()
        .take_while(|character| character.is_ascii_digit() || *character == '.')
        .collect();
    let parts: Option<Vec<u32>> = prefix
        .split('.')
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u32>().ok())
        .collect();
    parts.filter(|parts| !parts.is_empty())
}

fn local_release_is_newer(local: &str, remote: &str) -> bool {
    match (release_version_parts(local), release_version_parts(remote)) {
        (Some(mut local_parts), Some(mut remote_parts)) => {
            let length = local_parts.len().max(remote_parts.len());
            local_parts.resize(length, 0);
            remote_parts.resize(length, 0);
            local_parts > remote_parts
        }
        _ => false,
    }
}

fn is_ffmpeg_git_build(version: &str) -> bool {
    let normalized = version.to_ascii_lowercase();
    normalized.starts_with("n-") || normalized.contains("-git-") || normalized.ends_with("-git")
}

fn git_build_date(version: &str) -> Option<&str> {
    let bytes = version.as_bytes();
    if bytes.len() < 15
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[10..15].eq_ignore_ascii_case(b"-git-")
        || !bytes[..10]
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        return None;
    }
    Some(&version[..10])
}

fn ffmpeg_update_source(version: &str) -> Option<FfmpegUpdateSource> {
    if git_build_date(version).is_some() {
        Some(FfmpegUpdateSource {
            archive_url: FFMPEG_GIT_URL,
            version_url: FFMPEG_GIT_VERSION_URL,
            checksum_url: FFMPEG_GIT_CHECKSUM_URL,
            archive_extension: "7z",
            git_build: true,
        })
    } else if is_ffmpeg_git_build(version) {
        None
    } else {
        Some(FfmpegUpdateSource {
            archive_url: FFMPEG_URL,
            version_url: FFMPEG_VERSION_URL,
            checksum_url: FFMPEG_CHECKSUM_URL,
            archive_extension: "zip",
            git_build: false,
        })
    }
}

fn ffmpeg_tool_names() -> [&'static str; 2] {
    ["ffmpeg.exe", "ffprobe.exe"]
}

fn ffmpeg_backup_path(output_dir: &Path, name: &str) -> PathBuf {
    output_dir.join(format!("{name}.deck-backup"))
}

fn ffmpeg_pending_path(output_dir: &Path, name: &str) -> PathBuf {
    output_dir.join(format!("{name}.deck-new"))
}

fn recover_ffmpeg_transaction(output_dir: &Path) -> Result<(), String> {
    let marker = output_dir.join(FFMPEG_TRANSACTION_FILE);
    if !marker.is_file() {
        for name in ffmpeg_tool_names() {
            let _ = std::fs::remove_file(ffmpeg_backup_path(output_dir, name));
            let _ = std::fs::remove_file(ffmpeg_pending_path(output_dir, name));
        }
        return Ok(());
    }

    let marker_contents =
        std::fs::read(&marker).map_err(|e| format!("Falha ao ler a recuperação do FFmpeg: {e}"))?;
    let transaction: FfmpegUpdateTransaction = serde_json::from_slice(&marker_contents)
        .map_err(|e| format!("A recuperação do FFmpeg está inválida: {e}"))?;
    let mut errors = Vec::new();

    for name in ffmpeg_tool_names() {
        let target = output_dir.join(name);
        let backup = ffmpeg_backup_path(output_dir, name);
        let pending = ffmpeg_pending_path(output_dir, name);
        let existed_before = transaction.original_files.iter().any(|item| item == name);

        if backup.is_file() {
            if target.is_file() {
                if let Err(error) = std::fs::remove_file(&target) {
                    errors.push(format!("não foi possível remover {name}: {error}"));
                    continue;
                }
            }
            if let Err(error) = std::fs::rename(&backup, &target) {
                errors.push(format!("não foi possível restaurar {name}: {error}"));
                continue;
            }
        } else if !existed_before && target.is_file() {
            if let Err(error) = std::fs::remove_file(&target) {
                errors.push(format!("não foi possível reverter {name}: {error}"));
                continue;
            }
        }

        if pending.is_file() {
            if let Err(error) = std::fs::remove_file(&pending) {
                errors.push(format!("não foi possível limpar {name}: {error}"));
            }
        }
    }

    if errors.is_empty() {
        std::fs::remove_file(&marker)
            .map_err(|e| format!("Falha ao concluir a recuperação do FFmpeg: {e}"))
    } else {
        Err(format!(
            "A atualização anterior do FFmpeg não pôde ser revertida: {}. Os backups foram preservados.",
            errors.join("; ")
        ))
    }
}

fn begin_ffmpeg_install(staging_dir: &Path, output_dir: &Path) -> Result<(), String> {
    recover_ffmpeg_transaction(output_dir)?;
    let names = ffmpeg_tool_names();
    for name in names {
        if !staging_dir.join(name).is_file() {
            return Err(format!("O pacote validado não continha {name}."));
        }
    }

    for name in names {
        let pending = ffmpeg_pending_path(output_dir, name);
        std::fs::copy(staging_dir.join(name), &pending)
            .map_err(|e| format!("Falha ao preparar {name}: {e}"))?;
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&pending)
            .and_then(|file| file.sync_all())
            .map_err(|e| format!("Falha ao sincronizar {name}: {e}"))?;
    }

    let transaction = FfmpegUpdateTransaction {
        original_files: names
            .iter()
            .filter(|name| output_dir.join(name).is_file())
            .map(|name| (*name).to_owned())
            .collect(),
    };
    let marker = output_dir.join(FFMPEG_TRANSACTION_FILE);
    let marker_temporary = output_dir.join(format!("{FFMPEG_TRANSACTION_FILE}.new"));
    let marker_data = serde_json::to_vec(&transaction)
        .map_err(|e| format!("Falha ao registrar a atualização do FFmpeg: {e}"))?;
    {
        let mut marker_file = File::create(&marker_temporary)
            .map_err(|e| format!("Falha ao registrar a atualização do FFmpeg: {e}"))?;
        marker_file
            .write_all(&marker_data)
            .and_then(|_| marker_file.sync_all())
            .map_err(|e| format!("Falha ao registrar a atualização do FFmpeg: {e}"))?;
    }
    std::fs::rename(&marker_temporary, &marker)
        .map_err(|e| format!("Falha ao iniciar a atualização segura do FFmpeg: {e}"))?;

    let install_result = (|| {
        for name in names {
            let target = output_dir.join(name);
            let backup = ffmpeg_backup_path(output_dir, name);
            if target.is_file() {
                std::fs::rename(&target, &backup)
                    .map_err(|e| format!("O {name} está em uso e não pôde ser atualizado: {e}"))?;
            }
        }
        for name in names {
            std::fs::rename(ffmpeg_pending_path(output_dir, name), output_dir.join(name))
                .map_err(|e| format!("Falha ao instalar {name}: {e}"))?;
        }
        Ok::<(), String>(())
    })();

    if let Err(error) = install_result {
        return match recover_ffmpeg_transaction(output_dir) {
            Ok(()) => Err(format!("{error} O arquivo anterior foi restaurado.")),
            Err(rollback_error) => Err(format!("{error} {rollback_error}")),
        };
    }
    Ok(())
}

fn commit_ffmpeg_install(output_dir: &Path) -> Result<(), String> {
    let marker = output_dir.join(FFMPEG_TRANSACTION_FILE);
    std::fs::remove_file(&marker)
        .map_err(|e| format!("Falha ao confirmar a atualização do FFmpeg: {e}"))?;
    for name in ffmpeg_tool_names() {
        let _ = std::fs::remove_file(ffmpeg_backup_path(output_dir, name));
        let _ = std::fs::remove_file(ffmpeg_pending_path(output_dir, name));
    }
    Ok(())
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

    if extracted != wanted.len() {
        return Err("O pacote baixado não continha ffmpeg.exe e ffprobe.exe.".to_owned());
    }
    Ok(())
}

fn find_extracted_tool(directory: &Path, file_name: &str) -> Option<PathBuf> {
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = std::fs::read_dir(current).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(file_name)
            {
                return Some(path);
            }
        }
    }
    None
}

fn extract_ffmpeg_7z_archive(archive_path: &Path, output_dir: &Path) -> Result<(), String> {
    let unpacked = output_dir.join("unpacked");
    std::fs::create_dir_all(&unpacked)
        .map_err(|e| format!("Falha ao preparar a extração do FFmpeg: {e}"))?;
    sevenz_rust2::decompress_file(archive_path, &unpacked)
        .map_err(|e| format!("Pacote 7z do FFmpeg inválido: {e}"))?;
    for name in ffmpeg_tool_names() {
        let source = find_extracted_tool(&unpacked, name)
            .ok_or_else(|| format!("O pacote do FFmpeg não continha {name}."))?;
        std::fs::copy(source, output_dir.join(name))
            .map_err(|e| format!("Falha ao preparar {name}: {e}"))?;
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
            download_file(app, "yt-dlp", YTDLP_URL, &yt_dlp, true).await?;
        }
        emit_setup(app, "yt-dlp", "yt-dlp instalado", Some(100));
    }

    if !ffmpeg.is_file() {
        emit_setup(app, "ffmpeg", "Preparando FFmpeg…", Some(0));
        if !copy_local_tool_if_available(&ffmpeg, "ffmpeg.exe").await {
            let archive = directory.join("ffmpeg.zip");
            download_file(app, "ffmpeg", FFMPEG_URL, &archive, true).await?;
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
    let _operation = tool_operation_lock().try_write().map_err(|_| {
        "As ferramentas estão sendo usadas. Tente novamente em instantes.".to_owned()
    })?;
    ensure_tools_internal(&app).await
}

#[tauri::command]
async fn get_tool_status(app: AppHandle) -> Result<ToolStatus, String> {
    let _operation = tool_operation_lock().read().await;
    get_tool_status_internal(&app).await
}

#[tauri::command]
async fn update_yt_dlp(app: AppHandle) -> Result<ToolUpdateResult, String> {
    let _operation = tool_operation_lock().try_write().map_err(|_| {
        "Aguarde a pesquisa, o download ou a outra atualização terminar.".to_owned()
    })?;
    let executable = require_tool(&app, "yt-dlp.exe")?;
    let previous_version = yt_dlp_version(&executable).await?;
    command_output(
        &executable,
        &["--no-config", "--encoding", "utf-8", "-U"],
        180,
    )
    .await
    .map_err(|error| format!("Não foi possível atualizar o yt-dlp: {error}"))?;
    tokio::time::sleep(Duration::from_millis(350)).await;
    let current_version = yt_dlp_version(&executable).await?;
    let updated = previous_version != current_version;
    let message = if updated {
        format!("yt-dlp atualizado de {previous_version} para {current_version}.")
    } else {
        format!("yt-dlp já está na versão mais recente ({current_version}).")
    };
    Ok(ToolUpdateResult {
        tool: "yt-dlp".to_owned(),
        status: if updated { "updated" } else { "current" }.to_owned(),
        updated,
        previous_version: Some(previous_version),
        current_version: Some(current_version),
        message,
    })
}

#[tauri::command]
async fn update_ffmpeg(app: AppHandle) -> Result<ToolUpdateResult, String> {
    let _operation = tool_operation_lock().try_write().map_err(|_| {
        "Aguarde a pesquisa, o download ou a outra atualização terminar.".to_owned()
    })?;
    let directory = app_tools_dir(&app)?;
    recover_ffmpeg_transaction(&directory)?;
    let executable = require_tool(&app, "ffmpeg.exe")?;
    let previous_version = ffmpeg_version(&executable).await?;
    let Some(source) = ffmpeg_update_source(&previous_version) else {
        return Ok(ToolUpdateResult {
            tool: "ffmpeg".to_owned(),
            status: "preserved".to_owned(),
            updated: false,
            previous_version: Some(previous_version.clone()),
            current_version: Some(previous_version),
            message: "Este build de desenvolvimento do FFmpeg não informa uma data comparável e foi preservado para evitar downgrade.".to_owned(),
        });
    };
    let remote_version = fetch_small_text(source.version_url, "a versão mais recente do FFmpeg")
        .await?
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned();
    let valid_remote_version = if source.git_build {
        git_build_date(&remote_version).is_some()
            && remote_version
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
    } else {
        !remote_version.is_empty()
            && remote_version
                .chars()
                .all(|character| character.is_ascii_digit() || character == '.')
    };
    if !valid_remote_version {
        return Err("O servidor informou uma versão inválida do FFmpeg.".to_owned());
    }

    if ffmpeg_release_matches(&previous_version, &remote_version) {
        return Ok(ToolUpdateResult {
            tool: "ffmpeg".to_owned(),
            status: "current".to_owned(),
            updated: false,
            previous_version: Some(previous_version),
            current_version: Some(remote_version.clone()),
            message: format!("FFmpeg já está na versão mais recente ({remote_version})."),
        });
    }

    let local_is_newer = if source.git_build {
        git_build_date(&previous_version) > git_build_date(&remote_version)
    } else {
        local_release_is_newer(&previous_version, &remote_version)
    };
    if local_is_newer {
        return Ok(ToolUpdateResult {
            tool: "ffmpeg".to_owned(),
            status: "preserved".to_owned(),
            updated: false,
            previous_version: Some(previous_version.clone()),
            current_version: Some(previous_version.clone()),
            message: format!(
                "A versão local do FFmpeg ({previous_version}) é mais nova que a publicada ({remote_version}) e foi preservada."
            ),
        });
    }

    let archive = directory.join(format!("ffmpeg-update.{}", source.archive_extension));
    let staging_dir = directory.join("ffmpeg-update-staging");
    let _ = fs::remove_file(&archive).await;
    let _ = fs::remove_file(archive.with_extension("download")).await;
    let _ = fs::remove_dir_all(&staging_dir).await;
    fs::create_dir_all(&staging_dir)
        .await
        .map_err(|e| format!("Falha ao preparar a atualização do FFmpeg: {e}"))?;

    let update_result = async {
        let expected_checksum =
            fetch_small_text(source.checksum_url, "a assinatura do FFmpeg").await?;
        let expected_checksum = expected_checksum
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if expected_checksum.len() != 64
            || !expected_checksum
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Err("O servidor informou uma assinatura inválida do FFmpeg.".to_owned());
        }

        download_file(&app, "FFmpeg", source.archive_url, &archive, false).await?;
        let archive_for_hash = archive.clone();
        let actual_checksum = tokio::task::spawn_blocking(move || sha256_file(&archive_for_hash))
            .await
            .map_err(|e| format!("Falha interna ao validar o FFmpeg: {e}"))??;
        if actual_checksum != expected_checksum {
            return Err(
                "A validação SHA-256 do FFmpeg falhou. O arquivo atual foi preservado.".to_owned(),
            );
        }

        let archive_for_task = archive.clone();
        let staging_for_task = staging_dir.clone();
        let git_build = source.git_build;
        tokio::task::spawn_blocking(move || {
            if git_build {
                extract_ffmpeg_7z_archive(&archive_for_task, &staging_for_task)
            } else {
                extract_ffmpeg_archive(&archive_for_task, &staging_for_task)
            }
        })
        .await
        .map_err(|e| format!("Falha interna ao extrair o FFmpeg: {e}"))??;

        let staged_version = ffmpeg_version(&staging_dir.join("ffmpeg.exe")).await?;
        let staged_probe_version = ffprobe_version(&staging_dir.join("ffprobe.exe")).await?;
        if !ffmpeg_release_matches(&staged_version, &remote_version) {
            return Err(format!(
                "O pacote do FFmpeg informou a versão {staged_version}, diferente da esperada ({remote_version})."
            ));
        }
        if !ffmpeg_release_matches(&staged_probe_version, &remote_version) {
            return Err(format!(
                "O pacote do FFprobe informou a versão {staged_probe_version}, diferente da esperada ({remote_version})."
            ));
        }

        let staging_for_install = staging_dir.clone();
        let directory_for_install = directory.clone();
        tokio::task::spawn_blocking(move || {
            begin_ffmpeg_install(&staging_for_install, &directory_for_install)
        })
        .await
        .map_err(|e| format!("Falha interna ao instalar o FFmpeg: {e}"))??;

        let installed_validation = async {
            let installed_ffmpeg = ffmpeg_version(&directory.join("ffmpeg.exe")).await?;
            let installed_ffprobe = ffprobe_version(&directory.join("ffprobe.exe")).await?;
            if !ffmpeg_release_matches(&installed_ffmpeg, &remote_version)
                || !ffmpeg_release_matches(&installed_ffprobe, &remote_version)
            {
                return Err(
                    "Os executáveis instalados não correspondem ao pacote validado.".to_owned(),
                );
            }
            Ok::<(), String>(())
        }
        .await;
        if let Err(validation_error) = installed_validation {
            let recovery_dir = directory.clone();
            let recovery = tokio::task::spawn_blocking(move || {
                recover_ffmpeg_transaction(&recovery_dir)
            })
            .await
            .map_err(|e| format!("Falha interna ao restaurar o FFmpeg: {e}"))?;
            return match recovery {
                Ok(()) => Err(format!(
                    "{validation_error} A versão anterior foi restaurada."
                )),
                Err(recovery_error) => Err(format!("{validation_error} {recovery_error}")),
            };
        }

        let commit_dir = directory.clone();
        let commit_result =
            tokio::task::spawn_blocking(move || commit_ffmpeg_install(&commit_dir))
                .await
                .map_err(|e| format!("Falha interna ao confirmar o FFmpeg: {e}"))?;
        if let Err(commit_error) = commit_result {
            let recovery_dir = directory.clone();
            let recovery = tokio::task::spawn_blocking(move || {
                recover_ffmpeg_transaction(&recovery_dir)
            })
            .await
            .map_err(|e| format!("Falha interna ao restaurar o FFmpeg: {e}"))?;
            return match recovery {
                Ok(()) => Err(format!("{commit_error} A versão anterior foi restaurada.")),
                Err(recovery_error) => Err(format!("{commit_error} {recovery_error}")),
            };
        }
        Ok::<(), String>(())
    }
    .await;

    if !directory.join(FFMPEG_TRANSACTION_FILE).is_file() {
        let _ = fs::remove_file(&archive).await;
        let _ = fs::remove_file(archive.with_extension("download")).await;
        let _ = fs::remove_dir_all(&staging_dir).await;
    }
    update_result?;

    let current_version = ffmpeg_version(&executable).await?;
    Ok(ToolUpdateResult {
        tool: "ffmpeg".to_owned(),
        status: "updated".to_owned(),
        updated: true,
        previous_version: Some(previous_version.clone()),
        current_version: Some(current_version.clone()),
        message: format!("FFmpeg atualizado de {previous_version} para {current_version}."),
    })
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

    let _operation = tool_operation_lock()
        .try_read()
        .map_err(|_| "Aguarde a atualização do yt-dlp terminar.".to_owned())?;
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
        "--encoding",
        "utf-8",
        &format!("ytsearch5:{query}"),
    ]);
    if let Some(runtime) = javascript_runtime(&app) {
        command.args(["--js-runtimes", &runtime]);
    }
    command
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .stdin(Stdio::null());
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
    js_runtime: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "-o".to_owned(),
        output_template.to_string_lossy().into_owned(),
        "--no-playlist".to_owned(),
        "--add-metadata".to_owned(),
        "--newline".to_owned(),
        "--no-colors".to_owned(),
        "--encoding".to_owned(),
        "utf-8".to_owned(),
        "--windows-filenames".to_owned(),
        "--trim-filenames".to_owned(),
        "180".to_owned(),
        "--concurrent-fragments".to_owned(),
        "4".to_owned(),
        "--ffmpeg-location".to_owned(),
        ffmpeg.to_string_lossy().into_owned(),
    ];

    if let Some(runtime) = js_runtime {
        args.extend(["--js-runtimes".to_owned(), runtime.to_owned()]);
    }

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
    args.extend(["--".to_owned(), request.url.trim().to_owned()]);
    args
}

async fn forward_process_output<R>(reader: R, app: AppHandle)
where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(reader);
    let mut bytes = Vec::with_capacity(512);
    loop {
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes).await {
            Ok(0) => break,
            Ok(_) => {
                let line = decode_process_line(&bytes);
                if !line.is_empty() {
                    let _ = app.emit("download-output", line);
                }
            }
            Err(error) => {
                let _ = app.emit(
                    "download-output",
                    format!("Falha ao ler a saída do yt-dlp: {error}"),
                );
                break;
            }
        }
    }
}

fn decode_process_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches(['\r', '\n'])
        .to_owned()
}

#[tauri::command]
async fn start_download(
    app: AppHandle,
    request: DownloadRequest,
) -> Result<DownloadResult, String> {
    let _operation = tool_operation_lock()
        .try_read()
        .map_err(|_| "Aguarde a atualização das ferramentas terminar.".to_owned())?;
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
    let js_runtime = javascript_runtime(&app);
    let args = build_download_args(&request, &output_template, &ffmpeg, js_runtime.as_deref());

    let mut command = Command::new(executable);
    command
        .args(&args)
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .stdin(Stdio::null())
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

    let stdout_task = tokio::spawn(forward_process_output(stdout, out_app));
    let stderr_task = tokio::spawn(forward_process_output(stderr, err_app));

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

#[tauri::command]
async fn open_external_url(url: String) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url.trim())
        .map_err(|_| "O endereço do vídeo é inválido.".to_owned())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Somente endereços HTTP ou HTTPS podem ser abertos.".to_owned());
    }
    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    let trusted_host = host == "youtu.be"
        || host == "youtube.com"
        || host.ends_with(".youtube.com")
        || host == "youtube-nocookie.com"
        || host.ends_with(".youtube-nocookie.com");
    if !trusted_host {
        return Err("A prévia só pode abrir endereços oficiais do YouTube.".to_owned());
    }

    let mut command = Command::new("explorer.exe");
    command.arg(parsed.as_str());
    hide_console(&mut command);
    command
        .spawn()
        .map_err(|e| format!("Não foi possível abrir o navegador: {e}"))?;
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
        .setup(|app| {
            #[cfg(windows)]
            if let Ok(directory) = app_tools_dir(app.handle()) {
                let _ = recover_ffmpeg_transaction(&directory);
            }
            Ok(())
        })
        .plugin(tauri_plugin_mobile_downloader::init())
        .invoke_handler(tauri::generate_handler![
            ensure_tools,
            get_tool_status,
            update_yt_dlp,
            update_ffmpeg,
            search_videos,
            start_download,
            open_downloads_folder,
            open_external_url
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o YT-DLP Deck");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_ffmpeg_version() {
        let output = "ffmpeg version 8.1.2-essentials_build-www.gyan.dev Copyright";
        assert_eq!(
            parse_ffmpeg_version(output).as_deref(),
            Some("8.1.2-essentials_build-www.gyan.dev")
        );
        assert!(ffmpeg_release_matches(
            "8.1.2-essentials_build-www.gyan.dev",
            "8.1.2"
        ));
    }

    #[test]
    fn recognizes_ffmpeg_git_builds() {
        assert!(is_ffmpeg_git_build("2026-07-09-git-8de8405796"));
        assert!(is_ffmpeg_git_build("N-123456-gabc123"));
        assert!(!is_ffmpeg_git_build("8.1.2-essentials_build-www.gyan.dev"));
        assert_eq!(
            git_build_date("2026-07-27-git-a757b708ae-essentials_build"),
            Some("2026-07-27")
        );
        assert!(ffmpeg_update_source("2026-07-27-git-a757b708ae").is_some());
        assert!(ffmpeg_update_source("N-123456-gabc123").is_none());
        assert!(local_release_is_newer("9.0-full_build", "8.1.2"));
        assert!(!local_release_is_newer("7.1.1", "8.1.2"));
    }

    #[test]
    fn ffmpeg_install_can_rollback_and_commit() {
        let unique = format!(
            "yt-dlp-deck-transaction-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        let staging = root.join("staging");
        std::fs::create_dir_all(&staging).unwrap();
        for name in ffmpeg_tool_names() {
            std::fs::write(root.join(name), format!("old-{name}")).unwrap();
            std::fs::write(staging.join(name), format!("new-{name}")).unwrap();
        }

        begin_ffmpeg_install(&staging, &root).unwrap();
        assert!(root.join(FFMPEG_TRANSACTION_FILE).is_file());
        assert_eq!(
            std::fs::read(root.join("ffmpeg.exe")).unwrap(),
            b"new-ffmpeg.exe"
        );
        recover_ffmpeg_transaction(&root).unwrap();
        assert_eq!(
            std::fs::read(root.join("ffmpeg.exe")).unwrap(),
            b"old-ffmpeg.exe"
        );
        assert_eq!(
            std::fs::read(root.join("ffprobe.exe")).unwrap(),
            b"old-ffprobe.exe"
        );

        begin_ffmpeg_install(&staging, &root).unwrap();
        commit_ffmpeg_install(&root).unwrap();
        assert_eq!(
            std::fs::read(root.join("ffmpeg.exe")).unwrap(),
            b"new-ffmpeg.exe"
        );
        assert_eq!(
            std::fs::read(root.join("ffprobe.exe")).unwrap(),
            b"new-ffprobe.exe"
        );
        assert!(!root.join(FFMPEG_TRANSACTION_FILE).exists());
        assert!(!ffmpeg_backup_path(&root, "ffmpeg.exe").exists());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn process_output_tolerates_windows_1252_bytes() {
        let decoded = decode_process_line(&[b'T', 0x96, b'X', b'\r', b'\n']);
        assert!(decoded.starts_with('T'));
        assert!(decoded.ends_with('X'));
    }

    #[test]
    fn download_url_is_placed_after_argument_separator() {
        let request = DownloadRequest {
            url: "--exec=calc.exe".to_owned(),
            platform_folder: "YouTube".to_owned(),
            format: "mp3".to_owned(),
            quality: Some("best".to_owned()),
            cookies: "none".to_owned(),
            cookie_file: None,
        };
        let args = build_download_args(
            &request,
            Path::new("output.%(ext)s"),
            Path::new("ffmpeg.exe"),
            Some("node:C:\\Program Files\\nodejs\\node.exe"),
        );
        let separator = args
            .iter()
            .position(|argument| argument == "--")
            .expect("separador de argumentos ausente");
        assert_eq!(separator, args.len() - 2);
        assert_eq!(args.last().map(String::as_str), Some("--exec=calc.exe"));
        assert!(!args[..separator]
            .iter()
            .any(|argument| argument == "--exec=calc.exe"));
        assert!(args
            .iter()
            .any(|argument| argument == "--windows-filenames"));
        assert!(args.iter().any(|argument| argument.starts_with("node:")));
    }
}
