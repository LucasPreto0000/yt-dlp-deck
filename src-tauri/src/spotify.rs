//! Spotify supplies track metadata only. Audio is selected separately on YouTube.
use serde_json::Value;
use std::time::Duration;

fn track_url(input: &str) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(input.trim())
        .map_err(|_| "Cole um link de faixa do Spotify válido.".to_owned())?;
    let segments: Vec<_> = url.path_segments().into_iter().flatten().collect();
    let track = match segments.as_slice() {
        ["track", id] => Some(*id),
        [locale, "track", id] if locale.starts_with("intl-") => Some(*id),
        _ => None,
    };
    let valid = track.is_some_and(|id| id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric()));
    if url.scheme() != "https" || url.host_str() != Some("open.spotify.com")
        || url.port().is_some() || !url.username().is_empty() || url.password().is_some() || !valid
    {
        return Err("Use https://open.spotify.com/track/... (uma faixa; álbuns e playlists ainda não são suportados).".to_owned());
    }
    let path = format!("/track/{}", track.unwrap());
    url.set_path(&path);
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

#[tauri::command]
pub(crate) async fn spotify_track_query(url: String) -> Result<String, String> {
    let track = track_url(&url)?;
    let response = crate::http_client()?
        .get("https://open.spotify.com/oembed")
        .query(&[("url", track.as_str())])
        .timeout(Duration::from_secs(12))
        .send().await
        .map_err(|e| format!("Não foi possível consultar o Spotify: {e}"))?
        .error_for_status().map_err(|e| format!("Faixa indisponível no Spotify: {e}"))?;
    let text = response.text().await.map_err(|e| e.to_string())?;
    let info: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    info["title"].as_str().map(str::trim).filter(|s| !s.is_empty())
        .map(str::to_owned).ok_or_else(|| "O Spotify não informou o nome da faixa.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_spotify_tracks() {
        let id = "4uLU6hMCjMI75M1A2tKUQC";
        assert_eq!(track_url(&format!("https://open.spotify.com/intl-pt/track/{id}?si=abc")).unwrap().as_str(), format!("https://open.spotify.com/track/{id}"));
        for url in ["https://evil.test/track/4uLU6hMCjMI75M1A2tKUQC", "https://open.spotify.com/album/4uLU6hMCjMI75M1A2tKUQC", "http://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC", "https://open.spotify.com/track/bad"] {
            assert!(track_url(url).is_err());
        }
    }
}
