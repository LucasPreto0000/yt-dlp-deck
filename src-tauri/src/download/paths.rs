

pub(crate) fn safe_folder_name(input: &str) -> Result<String, String> {
    let value = input.trim();
    if value.is_empty() || value == "." || value == ".." {
        return Err("Escolha um nome válido para a pasta da plataforma.".to_owned());
    }
    let sanitized: String = value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            ch if ch.is_control() => '_',
            _ => ch,
        })
        .collect();
    let sanitized = sanitized.trim_end_matches([' ', '.']).to_owned();
    if sanitized.is_empty() {
        Err("Escolha um nome válido para a pasta da plataforma.".to_owned())
    } else {
        Ok(sanitized)
    }
}
