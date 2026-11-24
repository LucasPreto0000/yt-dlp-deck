use std::path::Path;
use crate::{DownloadRequest, SubtitleSelection};
use super::formats::{is_auxiliary_format, quality_selector};

pub(crate) fn append_access_args(args: &mut Vec<String>, request: &DownloadRequest, js_runtime: Option<&str>) {
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
}

pub(crate) fn build_download_args(
    request: &DownloadRequest,
    output_template: &Path,
    ffmpeg: Option<&Path>,
    js_runtime: Option<&str>,
    subtitle: Option<&SubtitleSelection>,
) -> Vec<String> {
    let mut args = vec![
        "--no-config".to_owned(),
        "-o".to_owned(),
        output_template.to_string_lossy().into_owned(),
        "--no-playlist".to_owned(),
        "--newline".to_owned(),
        "--no-colors".to_owned(),
        "--encoding".to_owned(),
        "utf-8".to_owned(),
        "--windows-filenames".to_owned(),
        "--trim-filenames".to_owned(),
        "180".to_owned(),
    ];

    if !is_auxiliary_format(&request.format) {
        args.extend([
            "--add-metadata".to_owned(),
            "--concurrent-fragments".to_owned(),
            "4".to_owned(),
        ]);
        if let Some(ffmpeg) = ffmpeg {
            args.extend([
                "--ffmpeg-location".to_owned(),
                ffmpeg.to_string_lossy().into_owned(),
            ]);
        }
    }
    append_access_args(&mut args, request, js_runtime);

    match request.format.as_str() {
        "image" => args.extend([
            "--skip-download".to_owned(),
            "--write-thumbnail".to_owned(),
            "--ignore-no-formats-error".to_owned(),
        ]),
        "subtitle" => args.extend([
            "--skip-download".to_owned(),
            "--ignore-no-formats-error".to_owned(),
        ]),
        "mp3" | "flac" | "wav" | "m4a" => args.extend([
            "-f".to_owned(),
            "bestaudio/best".to_owned(),
            "-x".to_owned(),
            "--audio-format".to_owned(),
            request.format.clone(),
            "--audio-quality".to_owned(),
            if request.format == "mp3" { "320K" } else { "0" }.to_owned(),
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
    if request.subtitles || request.format == "subtitle" {
        let selection = subtitle.expect("subtitle selection must be probed");
        args.push(if selection.automatic {
            "--write-auto-subs".to_owned()
        } else {
            "--write-subs".to_owned()
        });
        args.extend([
            "--sub-langs".to_owned(),
            selection.language.clone(),
            "--sub-format".to_owned(),
            "vtt/srt/best".to_owned(),
        ]);
    }
    args.extend(["--".to_owned(), request.url.trim().to_owned()]);
    args
}

pub(crate) fn build_standalone_subtitle_args(
    request: &DownloadRequest,
    output_template: &Path,
    js_runtime: Option<&str>,
    subtitle: &SubtitleSelection,
) -> Vec<String> {
    let mut subtitle_request = request.clone();
    subtitle_request.format = "subtitle".to_owned();
    subtitle_request.subtitles = true;
    let mut args = build_download_args(
        &subtitle_request,
        output_template,
        None,
        js_runtime,
        Some(subtitle),
    );
    let separator = args
        .iter()
        .position(|argument| argument == "--")
        .unwrap_or(args.len());
    args.splice(
        separator..separator,
        [
            "--retries".to_owned(),
            "4".to_owned(),
            "--retry-sleep".to_owned(),
            "http:linear=2::5".to_owned(),
            "--sleep-subtitles".to_owned(),
            "1".to_owned(),
        ],
    );
    args
}
