use url::Url;

fn under(host: &str, domain: &str) -> bool {
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

pub fn is_page(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some_and(|host| {
            matches!(
                host,
                "tiktok.com"
                    | "www.tiktok.com"
                    | "m.tiktok.com"
                    | "vt.tiktok.com"
                    | "vm.tiktok.com"
            )
        })
}

fn media_url(input: &str) -> Option<Url> {
    let url = Url::parse(input).ok()?;
    let host = url.host_str()?;
    (url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && [
            "tiktok.com",
            "tiktokcdn.com",
            "tiktokcdn-us.com",
            "tiktokv.com",
        ]
        .iter()
        .any(|domain| under(host, domain)))
    .then_some(url)
}

fn public_media(html: &str, cover: bool) -> Option<(String, Vec<Url>)> {
    let data = html.split("<script").skip(1).find_map(|script| {
        let (tag, body) = script.split_once('>')?;
        (tag.contains(" id=\"__UNIVERSAL_DATA_FOR_REHYDRATION__\"")
            || tag.contains(" id='__UNIVERSAL_DATA_FOR_REHYDRATION__'"))
        .then(|| body.split_once("</script>").map(|(json, _)| json))
        .flatten()
    })?;
    let doc: serde_json::Value = serde_json::from_str(data).ok()?;
    let item = doc.pointer("/__DEFAULT_SCOPE__/webapp.video-detail/itemInfo/itemStruct")?;
    let id = item["id"].as_str()?;
    if id.is_empty() || id.len() > 24 || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let video = &item["video"];
    let mut urls = Vec::new();
    let candidates = if cover {
        [
            &video["originCover"],
            &video["cover"],
            &video["originCoverUrl"],
        ]
    } else {
        [
            &video["playAddr"],
            &video["downloadAddr"],
            &video["download"]["url"],
        ]
    };
    for value in candidates {
        if let Some(url) = value.as_str().and_then(media_url) {
            if !urls.contains(&url) {
                urls.push(url);
            }
        }
    }
    (!urls.is_empty()).then(|| (id.to_owned(), urls))
}

#[cfg(target_os = "android")]
pub fn open(
    input: Url,
    cover: bool,
    transfer: &crate::web::transfer::Transfer,
) -> Result<(reqwest::blocking::Response, String), String> {
    use reqwest::{blocking::Client, header, redirect};
    use std::{io::Read, time::Duration};

    const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/131.0.0.0 Safari/537.36";
    const MAX_PAGE: u64 = 3 * 1024 * 1024;
    let client = Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(300))
        .user_agent(UA)
        .cookie_store(true)
        .redirect(redirect::Policy::custom(|attempt| {
            if attempt.previous().len() < 5 && media_url(attempt.url().as_str()).is_some() {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|e| e.without_url().to_string())?;
    let kind = if cover { "capa" } else { "vídeo" };
    transfer.log(&format!("[TikTok] Buscando {kind} da publicação…"));
    let get_page = |url| {
        client
            .get(url)
            .timeout(Duration::from_secs(15))
            .header(header::CACHE_CONTROL, "no-cache")
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(|e| {
                format!(
                    "Não foi possível abrir a publicação do TikTok: {}",
                    e.without_url()
                )
            })
    };
    let mut response = get_page(input)?;
    let mut canonical = response.url().clone();
    if is_page(&canonical) && canonical.query().is_some() && canonical.path().contains("/video/") {
        canonical.set_query(None);
        canonical.set_fragment(None);
        if let Ok(fresh) = get_page(canonical) {
            response = fresh;
        }
    }
    let mut html = Vec::new();
    response
        .take(MAX_PAGE + 1)
        .read_to_end(&mut html)
        .map_err(|_| "O TikTok não terminou de responder.".to_string())?;
    if html.len() as u64 > MAX_PAGE {
        return Err("A página do TikTok excedeu o limite de leitura.".into());
    }
    let (id, urls) = public_media(&String::from_utf8_lossy(&html), cover).ok_or_else(|| {
        format!("O TikTok não forneceu {kind} para esta publicação. Confira se o vídeo está acessível sem login.")
    })?;
    let mut last_status = None;
    for url in urls {
        transfer.checkpoint().map_err(|e| e.to_string())?;
        if let Ok(response) = client
            .get(url)
            .header(header::REFERER, "https://www.tiktok.com/")
            .send()
        {
            last_status = Some(response.status().as_u16());
            let mime = response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(';').next());
            let valid = if cover {
                matches!(
                    mime,
                    Some("image/jpeg" | "image/png" | "image/webp" | "image/avif")
                )
            } else {
                mime == Some("video/mp4")
            };
            if response.status().is_success() && valid {
                transfer.log(&format!(
                    "[TikTok] Arquivo de {kind} encontrado. Iniciando download…"
                ));
                let suffix = if cover { "-capa" } else { "" };
                return Ok((response, format!("TikTok-{id}{suffix}")));
            }
        }
    }
    let status = last_status
        .filter(|&code| code >= 400)
        .map_or(String::new(), |code| format!(" (HTTP {code})"));
    Err(format!(
        "O TikTok não entregou {kind}{status}. Verifique se a publicação continua pública."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_public_video_without_accepting_untrusted_hosts() {
        let html = r#"<script id="__UNIVERSAL_DATA_FOR_REHYDRATION__" type="application/json">{"__DEFAULT_SCOPE__":{"webapp.video-detail":{"itemInfo":{"itemStruct":{"id":"123","video":{"playAddr":"https://v16-webapp-prime.tiktok.com/path?token=abc","downloadAddr":"https://tiktok.com.evil.test/fake.mp4","originCover":"https://p16.tiktokcdn.com/cover.webp"}}}}}}</script>"#;
        let (id, urls) = public_media(html, false).unwrap();
        assert_eq!(id, "123");
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].query(), Some("token=abc"));
        assert_eq!(public_media(html, true).unwrap().1[0].path(), "/cover.webp");
        assert!(is_page(
            &Url::parse("https://vt.tiktok.com/example/").unwrap()
        ));
        assert!(!is_page(
            &Url::parse("https://www.tiktok.com.invalid/example/").unwrap()
        ));
        assert!(public_media("<html>Login</html>", false).is_none());
        assert!(media_url("http://tiktok.com/file").is_none());
        assert!(media_url("https://user@tiktok.com/file").is_none());
    }
}
