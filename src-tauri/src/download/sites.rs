use crate::DownloadRequest;

pub(crate) fn is_twitter_url(url: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url.trim()) else {
        return false;
    };
    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    host == "x.com"
        || host.ends_with(".x.com")
        || host == "twitter.com"
        || host.ends_with(".twitter.com")
}

pub(crate) fn twitter_media_requires_login(output: &str) -> bool {
    let output = output.to_ascii_lowercase();
    (output.contains("video #") && output.contains("is unavailable"))
        || output.contains("no video could be found in this tweet")
        || output.contains("this tweet is age-restricted")
}

pub(crate) fn is_youtube_url(url: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url.trim()) else {
        return false;
    };
    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    host == "youtu.be"
        || host.ends_with(".youtu.be")
        || host == "youtube.com"
        || host.ends_with(".youtube.com")
}

pub(crate) fn youtube_download_was_forbidden(output: &str) -> bool {
    let output = output.to_ascii_lowercase();
    output.contains("[youtube]")
        && (output.contains("http error 403") || output.contains("403: forbidden"))
}

fn youtube_hls_fallback_selector(request: &DownloadRequest) -> String {
    if matches!(request.format.as_str(), "mp3" | "flac" | "wav" | "m4a") {
        return "ba[protocol^=m3u8]/ba/b".to_owned();
    }

    let (height, prefer_sixty_fps) = match request.quality.as_deref() {
        Some("2160p60") => (Some(2160), true),
        Some("1440p60") => (Some(1440), true),
        Some("1080p60") => (Some(1080), true),
        Some("1080p") => (Some(1080), false),
        Some("720p") => (Some(720), false),
        Some("480p") => (Some(480), false),
        _ => (None, false),
    };
    let limit = height
        .map(|height| format!("[height<={height}]"))
        .unwrap_or_default();
    let audio = "ba[protocol^=m3u8]";
    let mut choices = Vec::new();
    if prefer_sixty_fps {
        choices.push(format!("bv*[protocol^=m3u8]{limit}[fps>=60]+{audio}"));
    }
    choices.push(format!("bv*[protocol^=m3u8]{limit}+{audio}"));
    choices.push(format!("b[protocol^=m3u8]{limit}"));
    choices.push("18".to_owned());
    choices.join("/")
}

pub(crate) fn add_youtube_hls_fallback(args: &[String], request: &DownloadRequest) -> Vec<String> {
    let mut fallback = args.to_vec();
    let selector = youtube_hls_fallback_selector(request);
    if let Some(format_index) = fallback.iter().position(|argument| argument == "-f") {
        if let Some(value) = fallback.get_mut(format_index + 1) {
            *value = selector;
            return fallback;
        }
    }
    let insertion = fallback
        .iter()
        .position(|argument| argument == "--")
        .unwrap_or(fallback.len());
    fallback.splice(insertion..insertion, ["-f".to_owned(), selector]);
    fallback
}
