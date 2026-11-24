use serde_json::{Deserializer, Value};

use crate::web::SearchResult;

pub(crate) fn extract_results(page: &str) -> Result<Vec<SearchResult>, String> {
    let data = ["var ytInitialData = ", "window[\"ytInitialData\"] = "]
        .iter()
        .find_map(|marker| page.split_once(marker).map(|(_, data)| data))
        .ok_or_else(|| "O YouTube não retornou os resultados da pesquisa.".to_owned())?;
    let data = Deserializer::from_str(data)
        .into_iter::<Value>()
        .next()
        .ok_or_else(|| "A resposta de pesquisa do YouTube está vazia.".to_owned())?
        .map_err(|_| "A resposta de pesquisa do YouTube mudou de formato.".to_owned())?;
    let mut results = Vec::new();
    collect(&data, &mut results);
    Ok(results)
}

fn collect(value: &Value, results: &mut Vec<SearchResult>) {
    if results.len() >= 5 {
        return;
    }
    if let Some(video) = value.get("videoRenderer") {
        if let Some(id) = video["videoId"].as_str().filter(|id| {
            id.len() == 11
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        }) {
            if results.iter().all(|result| result.id != id) {
                let title = if let Some(runs) = video["title"]["runs"].as_array() {
                    runs.iter()
                        .filter_map(|run| run["text"].as_str())
                        .collect::<String>()
                } else {
                    video["title"]["simpleText"]
                        .as_str()
                        .unwrap_or("Sem título")
                        .to_owned()
                };
                let duration = video["lengthText"]["simpleText"].as_str().unwrap_or("—");
                results.push(SearchResult {
                    id: id.to_owned(),
                    title,
                    duration: duration.to_owned(),
                    thumbnail: format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg"),
                });
            }
        }
        return;
    }
    match value {
        Value::Array(items) => items.iter().for_each(|item| collect(item, results)),
        Value::Object(items) => items.values().for_each(|item| collect(item, results)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn search_results_keep_title_and_duration() {
        let page = r#"<script>var ytInitialData = {"contents":[{"videoRenderer":{"videoId":"abc-def_123","title":{"runs":[{"text":"Rust & Android"}]},"lengthText":{"simpleText":"4:21"}}}]};</script>"#;
        let items = super::extract_results(page).unwrap();
        assert_eq!(items[0].title, "Rust & Android");
        assert_eq!(items[0].duration, "4:21");
        assert!(items[0].thumbnail.starts_with("https://i.ytimg.com/"));
    }
}
