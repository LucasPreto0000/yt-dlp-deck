import glob
import json
import os

from yt_dlp import YoutubeDL


class _MobileLogger:
    def __init__(self, callback):
        self.callback = callback

    def debug(self, message):
        self._send(message)

    def info(self, message):
        self._send(message)

    def warning(self, message):
        if "requested merging of multiple formats" in str(message).lower():
            self._send("[sistema] Vídeo e áudio serão unidos pelo FFmpeg incorporado.")
            return
        self._send(f"WARNING: {message}")

    def error(self, message):
        self._send(f"ERROR: {message}")

    def _send(self, message):
        text = str(message or "").strip()
        if text:
            self.callback.onLog(text)


def _duration_text(seconds):
    if not seconds:
        return "duração indisponível"
    seconds = int(seconds)
    hours, remainder = divmod(seconds, 3600)
    minutes, seconds = divmod(remainder, 60)
    if hours:
        return f"{hours}:{minutes:02d}:{seconds:02d}"
    return f"{minutes}:{seconds:02d}"


def _thumbnail(entry):
    if entry.get("thumbnail"):
        return entry["thumbnail"]
    thumbnails = entry.get("thumbnails") or []
    return thumbnails[-1].get("url", "") if thumbnails else ""


def search(query):
    options = {
        "extract_flat": "in_playlist",
        "ignoreerrors": True,
        "noplaylist": True,
        "playlistend": 5,
        "quiet": True,
        "skip_download": True,
        "socket_timeout": 15,
    }
    with YoutubeDL(options) as ydl:
        data = ydl.extract_info(f"ytsearch5:{query}", download=False)

    items = []
    for entry in (data or {}).get("entries") or []:
        if not entry:
            continue
        video_id = str(entry.get("id") or "")
        url = entry.get("webpage_url") or entry.get("url") or ""
        if video_id and not str(url).startswith(("http://", "https://")):
            url = f"https://www.youtube.com/watch?v={video_id}"
        items.append(
            {
                "id": video_id,
                "title": entry.get("title") or "Sem título",
                "duration": entry.get("duration_string")
                or _duration_text(entry.get("duration")),
                "thumbnail": _thumbnail(entry),
                "url": url,
            }
        )
    return json.dumps(items, ensure_ascii=False)


def _height_limit(quality):
    values = {
        "2160p60": 2160,
        "1440p60": 1440,
        "1080p60": 1080,
        "1080p": 1080,
        "720p": 720,
        "480p": 480,
    }
    return values.get(quality)


def _selector(media_format, quality):
    height = _height_limit(quality)
    height_filter = f"[height<={height}]" if height else ""
    if media_format in {"mp3", "flac", "wav", "m4a"}:
        return "bestaudio/best"
    if media_format == "mp4":
        return (
            f"bestvideo[ext=mp4][vcodec^=avc1]{height_filter}+"
            f"bestaudio[ext=m4a][acodec^=mp4a]/"
            f"bestvideo[ext=mp4]{height_filter}+bestaudio[ext=m4a]/"
            f"best[ext=mp4]{height_filter}/best"
        )
    if media_format == "webm":
        return (
            f"bestvideo[ext=webm]{height_filter}+bestaudio[ext=webm]/"
            f"best[ext=webm]{height_filter}/best"
        )
    return f"bestvideo{height_filter}+bestaudio/best{height_filter}/best"


def _progress_hook(callback):
    def hook(status):
        callback.checkpoint()
        state = status.get("status")
        if state == "downloading":
            percent = str(status.get("_percent_str") or "").strip()
            speed = str(status.get("_speed_str") or "").strip()
            eta = str(status.get("_eta_str") or "").strip()
            callback.onProgress(
                f"[download] {percent} de mídia · {speed} · ETA {eta}".strip()
            )
        elif state == "finished":
            callback.onProgress("[download] 90.0% Fluxo recebido; processando mídia…")

    return hook


def download(
    url,
    work_dir,
    media_format,
    quality,
    concurrent_fragments,
    cookie_file,
    callback,
):
    os.makedirs(work_dir, exist_ok=True)
    options = {
        "concurrent_fragment_downloads": max(1, min(int(concurrent_fragments), 4)),
        "continuedl": True,
        "format": _selector(media_format, quality),
        # Android uses FFmpegKit after yt-dlp finishes. This prevents yt-dlp from
        # requiring a standalone ffmpeg binary to merge split video/audio streams.
        "allow_unplayable_formats": True,
        "logger": _MobileLogger(callback),
        "nopart": False,
        "noplaylist": True,
        "outtmpl": os.path.join(work_dir, "%(title).150B [%(id)s].%(ext)s"),
        "overwrites": False,
        "progress_hooks": [_progress_hook(callback)],
        "quiet": True,
        "retries": 5,
        "fragment_retries": 5,
        "socket_timeout": 20,
        "trim_file_name": 180,
    }
    if cookie_file and os.path.isfile(cookie_file):
        options["cookiefile"] = cookie_file

    callback.onLog(f"[sistema] Destino temporário: {work_dir}")
    callback.onLog(
        f"[sistema] Fragmentos simultâneos: {options['concurrent_fragment_downloads']}"
    )
    with YoutubeDL(options) as ydl:
        callback.checkpoint()
        info = ydl.extract_info(url, download=True)

    if not info:
        raise RuntimeError(
            "O yt-dlp não conseguiu extrair ou baixar este vídeo."
        )

    requested_formats = info.get("requested_formats") or []
    downloads = info.get("requested_downloads") or []
    files = []
    for item in downloads:
        path = item.get("filepath")
        if path and os.path.isfile(path):
            files.append(
                {
                    "path": path,
                    "vcodec": item.get("vcodec") or "none",
                    "acodec": item.get("acodec") or "none",
                }
            )

    if not files and len(requested_formats) > 1:
        candidates = glob.glob(os.path.join(work_dir, "*"))
        for item in requested_formats:
            format_id = str(item.get("format_id") or "")
            marker = f".f{format_id}."
            path = next(
                (
                    candidate
                    for candidate in candidates
                    if marker in os.path.basename(candidate)
                    and os.path.isfile(candidate)
                    and not candidate.endswith((".part", ".ytdl"))
                ),
                None,
            )
            if path:
                files.append(
                    {
                        "path": path,
                        "vcodec": item.get("vcodec") or "none",
                        "acodec": item.get("acodec") or "none",
                    }
                )

    if not files:
        prepared = ydl.prepare_filename(info)
        candidates = [prepared, *glob.glob(os.path.join(work_dir, "*"))]
        for path in candidates:
            if os.path.isfile(path) and not path.endswith((".part", ".ytdl")):
                files.append(
                    {
                        "path": path,
                        "vcodec": info.get("vcodec") or "none",
                        "acodec": info.get("acodec") or "none",
                    }
                )
                break

    if not files:
        raise RuntimeError("O yt-dlp terminou sem produzir um arquivo de mídia.")

    if len(requested_formats) > 1:
        has_video = any(item["vcodec"] != "none" for item in files)
        has_audio = any(
            item["vcodec"] == "none" and item["acodec"] != "none" for item in files
        )
        if not has_video or not has_audio:
            raise RuntimeError(
                "O yt-dlp não conseguiu baixar todas as faixas de vídeo e áudio."
            )

    return json.dumps(
        {
            "id": str(info.get("id") or ""),
            "title": info.get("title") or "download",
            "files": files,
        },
        ensure_ascii=False,
    )
