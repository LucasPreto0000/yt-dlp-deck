use serde_json::Value;
use crate::SubtitleSelection;

fn safe_subtitle_language(language: &str) -> bool {
    !language.is_empty()
        && language.len() <= 64
        && language != "live_chat"
        && language
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn subtitle_language_rank(language: &str) -> (u8, String) {
    let normalized = language.to_ascii_lowercase();
    let rank = if matches!(normalized.as_str(), "pt-br" | "pt_br") {
        0
    } else if normalized == "pt" {
        1
    } else if normalized.starts_with("pt-") || normalized.starts_with("pt_") {
        2
    } else if normalized == "en" {
        3
    } else if normalized.starts_with("en-") || normalized.starts_with("en_") {
        4
    } else {
        5
    };
    (rank, normalized)
}

fn best_subtitle_language(collection: Option<&Value>) -> Option<String> {
    let entries = collection?.as_object()?;
    let mut languages = entries
        .keys()
        .filter(|language| safe_subtitle_language(language))
        .cloned()
        .collect::<Vec<_>>();
    languages.sort_by_key(|language| subtitle_language_rank(language));
    languages.into_iter().next()
}

pub(crate) fn select_subtitle(info: &Value) -> Option<SubtitleSelection> {
    best_subtitle_language(info.get("subtitles"))
        .map(|language| SubtitleSelection {
            language,
            automatic: false,
        })
        .or_else(|| {
            best_subtitle_language(info.get("automatic_captions")).map(|language| {
                SubtitleSelection {
                    language,
                    automatic: true,
                }
            })
        })
}
