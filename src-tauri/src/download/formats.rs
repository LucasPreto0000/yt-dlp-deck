

pub(crate) fn quality_selector(quality: Option<&str>) -> Option<&'static str> {
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

pub(crate) fn is_auxiliary_format(format: &str) -> bool {
    matches!(format, "image" | "subtitle")
}

pub(crate) fn supports_image_cover_url(url: &str) -> bool {
    reqwest::Url::parse(url).ok().is_some_and(|url| {
        url.host_str().is_some_and(|host| {
            let host = host.to_ascii_lowercase();
            host == "tiktok.com"
                || host.ends_with(".tiktok.com")
                || host == "instagram.com"
                || host.ends_with(".instagram.com")
        })
    })
}
