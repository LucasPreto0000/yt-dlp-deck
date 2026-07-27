import glob
import json
import os
import re
from urllib.parse import urlsplit, urlunsplit

from yt_dlp import YoutubeDL

_ANSI_ESCAPE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
_RAW_PROGRESS = re.compile(r"^\[download\]\s+\d+(?:\.\d+)?%")
_URL = re.compile(r"https?://[^\s]+")


def _redact_url_queries(message):
    def redact(match):
        raw = match.group(0)
        trailing = ""
        while raw and raw[-1] in ".,;)]}":
            trailing = raw[-1] + trailing
            raw = raw[:-1]
        try:
            parts = urlsplit(raw)
            if not parts.query and not parts.fragment:
                return raw + trailing
            safe = urlunsplit((parts.scheme, parts.netloc, parts.path, "<oculto>", ""))
            return safe + trailing
        except ValueError:
            return "[URL ocultada]" + trailing

    return _URL.sub(redact, str(message or ""))


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
        text = _redact_url_queries(message).strip()
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
    quickjs_path,
    callback,
):
    os.makedirs(work_dir, exist_ok=True)
    common_options = {
        "concurrent_fragment_downloads": max(1, min(int(concurrent_fragments), 4)),
        "continuedl": True,
        "logger": _MobileLogger(callback),
        "nopart": False,
        "noplaylist": True,
        "overwrites": False,
        "quiet": True,
        "retries": 5,
        "fragment_retries": 5,
        "socket_timeout": 20,
        "trim_file_name": 180,
        "js_runtimes": {"quickjs": {"path": quickjs_path}},
    }
    if cookie_file and os.path.isfile(cookie_file):
        common_options["cookiefile"] = cookie_file

    callback.onLog(f"[sistema] Destino temporário: {work_dir}")
    callback.onLog(
        f"[sistema] Fragmentos simultâneos: {common_options['concurrent_fragment_downloads']}"
    )
    callback.onLog("[sistema] Runtime JavaScript QuickJS incorporado e ativo.")

    probe_options = {
        **common_options,
        "format": _selector(media_format, quality),
        "skip_download": True,
    }
    with YoutubeDL(probe_options) as ydl:
        callback.checkpoint()
        info = ydl.extract_info(url, download=False)

    if not info:
        raise RuntimeError(
            "O yt-dlp não conseguiu extrair este vídeo."
        )

    requested_formats = info.get("requested_formats") or [info]
    selected_tracks = [
        item for item in requested_formats
        if item and (item.get("format_id") or item.get("url"))
    ]
    if not selected_tracks:
        raise RuntimeError("O yt-dlp não encontrou faixas reproduzíveis para esta mídia.")

    expected_tracks = len(selected_tracks)
    expected_size = sum(
        int(item.get("filesize") or item.get("filesize_approx") or 0)
        for item in selected_tracks
    )
    if expected_size:
        callback.onExpectedSize(expected_size)
        callback.onLog(
            f"[info] Tamanho estimado das faixas: {_format_bytes(expected_size)}"
        )

    progress_hook = _progress_hook(callback, expected_tracks)
    files = []
    for selected in selected_tracks:
        callback.checkpoint()
        format_id = str(selected.get("format_id") or "best")
        track_options = {
            **common_options,
            "format": format_id,
            "outtmpl": os.path.join(
                work_dir,
                "%(title).150B [%(id)s].f%(format_id)s.%(ext)s",
            ),
            "progress_hooks": [progress_hook],
        }
        with YoutubeDL(track_options) as track_ydl:
            track_info = track_ydl.extract_info(url, download=True)
            downloads = (track_info or {}).get("requested_downloads") or []
            paths = [
                item.get("filepath") for item in downloads
                if item.get("filepath")
            ]
            if track_info:
                paths.append(track_ydl.prepare_filename(track_info))
            path = next(
                (
                    candidate for candidate in paths
                    if candidate
                    and os.path.isfile(candidate)
                    and not candidate.endswith((".part", ".ytdl"))
                ),
                None,
            )
            if not path:
                marker = f".f{format_id}."
                path = next(
                    (
                        candidate
                        for candidate in glob.glob(os.path.join(work_dir, "*"))
                        if marker in os.path.basename(candidate)
                        and os.path.isfile(candidate)
                        and not candidate.endswith((".part", ".ytdl"))
                    ),
                    None,
                )
            if path and all(item["path"] != path for item in files):
                files.append(
                    {
                        "path": path,
                        "vcodec": selected.get("vcodec") or "none",
                        "acodec": selected.get("acodec") or "none",
                    }
                )

    if not files:
        raise RuntimeError("O yt-dlp terminou sem produzir um arquivo de mídia.")

    if len(selected_tracks) > 1:
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
