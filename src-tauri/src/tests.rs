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
fn parses_latest_version_from_yt_dlp_updater_output() {
    let output = "\
Current version: stable@2026.06.30 from yt-dlp/yt-dlp
Latest version: stable@2026.07.04 from yt-dlp/yt-dlp
Updated yt-dlp to stable@2026.07.04.";
    assert_eq!(
        parse_yt_dlp_update_version(output).as_deref(),
        Some("2026.07.04")
    );
    assert!(valid_yt_dlp_version("2026.07.04"));
    assert!(!valid_yt_dlp_version("2026.07.04."));
    assert!(!valid_yt_dlp_version("latest"));
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
    let git_source = ffmpeg_update_source("2026-07-27-git-a757b708ae").unwrap();
    assert!(git_source.git_build);
    assert!(git_source.seven_zip);
    assert_eq!(git_source.archive_extension, "7z");
    let release_source = ffmpeg_update_source("8.1.2-essentials_build").unwrap();
    assert!(!release_source.git_build);
    assert!(release_source.seven_zip);
    assert_eq!(release_source.archive_extension, "7z");
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
fn recognizes_twitter_sensitive_media_failure() {
    assert!(is_twitter_url(
        "https://x.com/usuario/status/2091883694038606230"
    ));
    assert!(is_twitter_url(
        "https://mobile.twitter.com/usuario/status/1"
    ));
    assert!(!is_twitter_url("https://example.com/x.com/status/1"));
    assert!(twitter_media_requires_login(
        "ERROR: [twitter] 123: Video #1 is unavailable"
    ));
    assert!(twitter_media_requires_login(
        "ERROR: [twitter] 123: No video could be found in this tweet"
    ));
}

#[test]
fn recognizes_youtube_forbidden_download_and_builds_hls_retry() {
    assert!(is_youtube_url("https://www.youtube.com/watch?v=abc"));
    assert!(is_youtube_url("https://youtu.be/abc"));
    assert!(!is_youtube_url("https://example.com/youtube.com/watch"));
    assert!(youtube_download_was_forbidden(
        "[youtube] abc: Downloading webpage\nERROR: unable to download video data: HTTP Error 403: Forbidden"
    ));

    let mut request = auxiliary_request("mp4");
    request.url = "https://youtu.be/abc".to_owned();
    request.quality = Some("1080p60".to_owned());
    let args = vec![
        "-f".to_owned(),
        "bv*+ba/b".to_owned(),
        "--".to_owned(),
        "https://youtu.be/abc".to_owned(),
    ];
    let retry = add_youtube_hls_fallback(&args, &request);
    let selector = retry
        .windows(2)
        .find(|pair| pair[0] == "-f")
        .map(|pair| pair[1].as_str())
        .unwrap();
    assert!(selector.contains("[protocol^=m3u8]"));
    assert!(selector.contains("[height<=1080][fps>=60]"));
    assert!(selector.ends_with("/18"));
    assert!(!selector.contains("bv*+ba/b"));
    assert_eq!(
        retry.last().map(String::as_str),
        Some("https://youtu.be/abc")
    );
}

#[test]
fn twitter_sensitive_media_error_explains_cookie_login() {
    let mut request = auxiliary_request("mp4");
    request.url = "https://x.com/usuario/status/1".to_owned();
    let output = "ERROR: [twitter] 1: Video #1 is unavailable";
    let anonymous = download_failure_message(&request, 1, output);
    assert!(anonymous.contains("sessão autenticada"));
    assert!(anonymous.contains("Cookies e autenticação"));

    request.cookies = "edge".to_owned();
    let authenticated = download_failure_message(&request, 1, output);
    assert!(authenticated.contains("cookies selecionados"));
    assert!(authenticated.contains("conteúdo sensível"));
}

#[test]
fn download_url_is_placed_after_argument_separator() {
    let request = DownloadRequest {
        url: "--exec=calc.exe".to_owned(),
        platform_folder: "YouTube".to_owned(),
        format: "mp3".to_owned(),
        subtitles: false,
        quality: Some("best".to_owned()),
        cookies: "none".to_owned(),
        cookie_file: None,
    };
    let args = build_download_args(
        &request,
        Path::new("output.%(ext)s"),
        Some(Path::new("ffmpeg.exe")),
        Some("node:C:\\Program Files\\nodejs\\node.exe"),
        None,
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

fn auxiliary_request(format: &str) -> DownloadRequest {
    DownloadRequest {
        url: "https://www.instagram.com/reel/example/".to_owned(),
        platform_folder: "Instagram".to_owned(),
        format: format.to_owned(),
        subtitles: false,
        quality: None,
        cookies: "none".to_owned(),
        cookie_file: None,
    }
}

#[test]
fn audio_download_uses_best_source_and_explicit_mp3_bitrate() {
    for format in ["mp3", "flac", "wav", "m4a"] {
        let request = auxiliary_request(format);
        let args = build_download_args(&request, Path::new("out.%(ext)s"), None, None, None);
        assert!(args.windows(2).any(|p| p == ["-f", "bestaudio/best"]));
        assert!(args.windows(2).any(|p| p == ["--audio-format", format]));
        let quality = if format == "mp3" { "320K" } else { "0" };
        assert!(args.windows(2).any(|p| p == ["--audio-quality", quality]));
        assert!(!args.iter().any(|p| p == "-ar" || p == "-ac"));
    }
}

#[test]
fn image_download_writes_only_the_thumbnail() {
    let request = auxiliary_request("image");
    let args = build_download_args(&request, Path::new("output.%(ext)s"), None, None, None);
    assert!(args.iter().any(|argument| argument == "--write-thumbnail"));
    assert!(args.iter().any(|argument| argument == "--skip-download"));
    assert!(!args.iter().any(|argument| argument == "-f"));
    assert!(!args.iter().any(|argument| argument == "--ffmpeg-location"));
}

#[test]
fn subtitle_download_uses_the_exact_selected_source() {
    let request = auxiliary_request("subtitle");
    let manual = SubtitleSelection {
        language: "pt-BR".to_owned(),
        automatic: false,
    };
    let args = build_download_args(
        &request,
        Path::new("output.%(ext)s"),
        None,
        None,
        Some(&manual),
    );
    assert!(args.iter().any(|argument| argument == "--write-subs"));
    assert!(!args.iter().any(|argument| argument == "--write-auto-subs"));
    assert!(args.windows(2).any(|pair| pair == ["--sub-langs", "pt-BR"]));
}

#[test]
fn subtitle_can_be_downloaded_alongside_mp4() {
    let mut request = auxiliary_request("mp4");
    request.subtitles = true;
    request.quality = Some("1080p".to_owned());
    let automatic = SubtitleSelection {
        language: "pt-BR".to_owned(),
        automatic: true,
    };
    let args = build_download_args(
        &request,
        Path::new("output.%(ext)s"),
        Some(Path::new("ffmpeg.exe")),
        None,
        Some(&automatic),
    );
    assert!(args
        .iter()
        .any(|argument| argument == "--merge-output-format"));
    assert!(args.iter().any(|argument| argument == "--write-auto-subs"));
    assert!(args.windows(2).any(|pair| pair == ["--sub-langs", "pt-BR"]));

    let standalone =
        build_standalone_subtitle_args(&request, Path::new("output.%(ext)s"), None, &automatic);
    assert!(standalone.windows(2).any(|pair| pair == ["--retries", "4"]));
    assert!(standalone
        .windows(2)
        .any(|pair| pair == ["--retry-sleep", "http:linear=2::5"]));
    assert!(standalone
        .iter()
        .any(|argument| argument == "--skip-download"));
}

#[test]
fn subtitle_selection_prefers_author_then_portuguese_automatic() {
    let manual = serde_json::json!({
        "subtitles": {"en": [{}]},
        "automatic_captions": {"pt-BR": [{}]}
    });
    assert_eq!(
        select_subtitle(&manual),
        Some(SubtitleSelection {
            language: "en".to_owned(),
            automatic: false,
        })
    );

    let automatic = serde_json::json!({
        "subtitles": {},
        "automatic_captions": {"en": [{}], "pt-BR": [{}]}
    });
    assert_eq!(
        select_subtitle(&automatic),
        Some(SubtitleSelection {
            language: "pt-BR".to_owned(),
            automatic: true,
        })
    );
}

#[test]
fn image_cover_hosts_are_restricted() {
    assert!(supports_image_cover_url(
        "https://www.tiktok.com/@a/video/1"
    ));
    assert!(supports_image_cover_url("https://instagram.com/reel/a"));
    assert!(!supports_image_cover_url(
        "https://example.com/instagram.com"
    ));
}
