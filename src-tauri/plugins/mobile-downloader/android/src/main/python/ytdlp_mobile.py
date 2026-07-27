import glob
import json
import os
import re

from yt_dlp import YoutubeDL

_ANSI_ESCAPE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
_RAW_PROGRESS = re.compile(r"^\[download\]\s+\d+(?:\.\d+)?%")


class _MobileLogger:
    def __init__(self, callback):
        self.callback = callback

    def debug(self, message):
        clean = _ANSI_ESCAPE.sub("", str(message or "")).strip()
        if _RAW_PROGRESS.match(clean):
            return
        self._send(clean)

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


def _format_bytes(value):
    if not value:
        return "tamanho desconhecido"
    size = float(value)
    units = ("B", "KiB", "MiB", "GiB")
    unit = units[0]
    for candidate in units:
        unit = candidate
        if size < 1024 or candidate == units[-1]:
            break
        size /= 1024
    return f"{size:.2f} {unit}"


def _progress_hook(callback, expected_tracks):
    completed_tracks = 0
    current_format = None
    announced_sizes = set()
    known_totals = {}
    total_announced = False

    def hook(status):
        nonlocal completed_tracks, current_format, total_announced
        callback.checkpoint()
        state = status.get("status")
        if state == "downloading":
            info = status.get("info_dict") or {}
            format_id = str(info.get("format_id") or status.get("filename") or "media")
            if current_format != format_id:
                current_format = format_id
            downloaded = status.get("downloaded_bytes") or 0
            total = status.get("total_bytes") or status.get("total_bytes_estimate") or 0
            raw_percent = (downloaded / total * 100) if total else 0
            phase_size = 88 / max(1, expected_tracks)
            percent = min(88.0, completed_tracks * phase_size + raw_percent * phase_size / 100)
            speed = str(status.get("_speed_str") or "").strip()
            eta = str(status.get("_eta_str") or "").strip()
            if total and format_id not in announced_sizes:
                callback.onLog(
                    f"[info] Tamanho da faixa {format_id}: {_format_bytes(total)}"
                )
                announced_sizes.add(format_id)
                known_totals[format_id] = total
                if len(known_totals) >= expected_tracks and not total_announced:
                    callback.onLog(
                        f"[info] Tamanho estimado total: "
                        f"{_format_bytes(sum(known_totals.values()))}"
                    )
                    total_announced = True
            callback.onProgress(
                f"[download] {percent:.1f}% · {_format_bytes(downloaded)} de "
                f"{_format_bytes(total)} · {speed} · ETA {eta}".strip()
            )
        elif state == "finished":
            completed_tracks = min(expected_tracks, completed_tracks + 1)
            percent = min(88.0, completed_tracks * (88 / max(1, expected_tracks)))
            callback.onProgress(
                f"[download] {percent:.1f}% Faixa recebida; preparando próxima etapa…"
            )

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
    expected_tracks = 1 if media_format in {"mp3", "flac", "wav", "m4a"} else 2
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
        "progress_hooks": [_progress_hook(callback, expected_tracks)],
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
