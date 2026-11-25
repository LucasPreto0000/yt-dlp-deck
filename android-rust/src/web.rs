use std::{
    io::Read,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use tiny_http::{Header, Method, Response, Server, StatusCode};

mod history;
mod screens;
pub(crate) mod transfer;
use history::{History, Record};
use transfer::{Snapshot, Transfer};

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

const CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/deck.css"));

pub trait Actions: Send + Sync + 'static {
    fn download(
        &self,
        url: &str,
        format: &str,
        directory: &str,
        wifi_only: bool,
        transfer: &Transfer,
    ) -> Result<String, String>;
    fn clipboard(&self) -> Result<String, String> {
        Err("Área de transferência indisponível.".into())
    }
    fn copy(&self, _text: &str) -> Result<(), String> {
        Err("Área de transferência indisponível.".into())
    }
    fn open_folder(&self) -> Result<(), String> {
        Err("Downloads indisponíveis.".into())
    }
    fn search(&self, _query: &str) -> Result<Vec<SearchResult>, String> {
        Err("A pesquisa está indisponível.".into())
    }
    fn open_video(&self, _id: &str) -> Result<(), String> {
        Err("Não foi possível abrir o YouTube.".into())
    }
    fn set_immersive(&self, _enabled: bool) -> Result<(), String> {
        Err("O controle da navegação está indisponível.".into())
    }
    fn history_path(&self) -> Option<PathBuf> {
        None
    }
    fn date_and_time(&self) -> (String, String) {
        (String::new(), String::new())
    }
}

pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub duration: String,
    pub thumbnail: String,
}

#[cfg(test)]
impl<F: Fn(&str, &str, &str) -> Result<String, String> + Send + Sync + 'static> Actions for F {
    fn download(
        &self,
        url: &str,
        format: &str,
        directory: &str,
        _: bool,
        _: &Transfer,
    ) -> Result<String, String> {
        self(url, format, directory)
    }
}
const STEPS: [(&str, &str, &str); 5] = [
    ("Plataforma", "Organize o destino", "globe-2"),
    ("Conteúdo", "Link ou pesquisa", "search"),
    ("Formato", "Vídeo, áudio ou extras", "film"),
    ("Ajustes", "Qualidade e acesso", "sliders-horizontal"),
    ("Finalizar", "Revise e baixe", "download"),
];
const QUALITIES: [(&str, &str, &str); 7] = [
    ("best", "Melhor disponível", "Sem limite"),
    ("2160p60", "4K · 60 FPS", "2160p"),
    ("1440p60", "2K · 60 FPS", "1440p"),
    ("1080p60", "Full HD · 60 FPS", "1080p"),
    ("1080p", "Full HD", "1080p"),
    ("720p", "HD", "720p"),
    ("480p", "Compacto", "480p"),
];

struct Choice {
    id: &'static str,
    title: &'static str,
    detail: &'static str,
    icon: &'static str,
    color: &'static str,
}

const PLATFORMS: &[Choice] = &[
    Choice {
        id: "YouTube",
        title: "YouTube",
        detail: "Vídeos, Shorts e lives",
        icon: "youtube",
        color: "#ff4164",
    },
    Choice {
        id: "TikTok",
        title: "TikTok",
        detail: "Vídeos e tendências",
        icon: "music-2",
        color: "#46edf2",
    },
    Choice {
        id: "Instagram",
        title: "Instagram",
        detail: "Reels e publicações",
        icon: "instagram",
        color: "#f765a3",
    },
    Choice {
        id: "X_Twitter",
        title: "X / Twitter",
        detail: "Vídeos e clipes",
        icon: "twitter",
        color: "#60baff",
    },
    Choice {
        id: "Twitch",
        title: "Twitch",
        detail: "Clipes e transmissões",
        icon: "radio",
        color: "#a978ff",
    },
    Choice {
        id: "Reddit",
        title: "Reddit",
        detail: "Mídias de comunidades",
        icon: "message-circle",
        color: "#ff7045",
    },
    Choice {
        id: "Outro",
        title: "Outro site",
        detail: "Qualquer site compatível",
        icon: "globe-2",
        color: "#aab2d1",
    },
];

const COVER_FORMAT: Choice = Choice {
    id: "image",
    title: "Imagem (capa)",
    detail: "Capa original do vídeo",
    icon: "image-down",
    color: "#ff9f68",
};

const FORMATS: &[(&str, Choice)] = &[
    (
        "Vídeo",
        Choice {
            id: "mp4",
            title: "MP4",
            detail: "Compatibilidade máxima",
            icon: "video",
            color: "#62a8ff",
        },
    ),
    (
        "Vídeo",
        Choice {
            id: "mkv",
            title: "MKV",
            detail: "Todos os codecs",
            icon: "disc-3",
            color: "#a97aff",
        },
    ),
    (
        "Vídeo",
        Choice {
            id: "webm",
            title: "WEBM",
            detail: "Formato web moderno",
            icon: "globe-2",
            color: "#43d9ff",
        },
    ),
    (
        "Vídeo",
        Choice {
            id: "best",
            title: "Melhor",
            detail: "Qualidade sem limites",
            icon: "sparkles",
            color: "#ffbd58",
        },
    ),
    (
        "Áudio",
        Choice {
            id: "mp3",
            title: "MP3",
            detail: "Áudio MP3 da fonte",
            icon: "music-2",
            color: "#ff6e9a",
        },
    ),
    (
        "Áudio",
        Choice {
            id: "flac",
            title: "FLAC",
            detail: "Áudio FLAC da fonte",
            icon: "audio-lines",
            color: "#44dda8",
        },
    ),
    (
        "Áudio",
        Choice {
            id: "wav",
            title: "WAV",
            detail: "Sem compressão",
            icon: "file-audio",
            color: "#dfe8ff",
        },
    ),
    (
        "Áudio",
        Choice {
            id: "m4a",
            title: "M4A",
            detail: "Eficiente e moderno",
            icon: "file-audio",
            color: "#9d9cff",
        },
    ),
];

struct Flow {
    step: usize,
    platform: String,
    url: String,
    format: String,
    quality: String,
    search: bool,
    query: String,
    results: Vec<SearchResult>,
    selected: Option<usize>,
    preview: Option<usize>,
    destination: String,
    wifi_only: bool,
    immersive: bool,
    transfer: Option<Arc<Transfer>>,
    live: bool,
    history: Arc<History>,
    day: String,
    status: Result<Option<String>, String>,
}

impl Default for Flow {
    fn default() -> Self {
        Self {
            step: 0,
            platform: String::new(),
            url: String::new(),
            format: String::new(),
            quality: "best".into(),
            search: false,
            query: String::new(),
            results: Vec::new(),
            selected: None,
            preview: None,
            destination: String::new(),
            wifi_only: false,
            immersive: true,
            transfer: None,
            live: true,
            history: Arc::new(History::load(None)),
            day: String::new(),
            status: Ok(None),
        }
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn icon(name: &str, size: u8) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{}</svg>"#,
        icon_paths(name)
    )
}

fn header(eyebrow: &str, title: &str, description: &str, glyph: &str) -> String {
    format!(
        r#"<header class="page-header"><div class="eyebrow"><span>{}</span>{eyebrow}</div><h1>{title}</h1><p>{description}</p></header>"#,
        icon(glyph, 14)
    )
}

fn choice(item: &Choice, selected: bool, action: &str, category: &str) -> String {
    let active = if selected { "is-active" } else { "" };
    let mark = if selected { 1 } else { 0 };
    let tag = if category.is_empty() {
        String::new()
    } else {
        format!(r#"<span class="choice-tag">{category}</span>"#)
    };
    format!(
        r#"<button type="submit" name="{action}" value="{}" class="choice-card {active}" style="--card-color:{}"><span class="choice-glow"></span><span class="choice-icon">{}</span>{tag}<span class="choice-copy"><strong>{}</strong><small>{}</small></span><span class="choice-check" style="scale:{mark};opacity:{mark}">{}</span></button>"#,
        item.id,
        item.color,
        icon(item.icon, 24),
        item.title,
        item.detail,
        icon("check", 14)
    )
}

fn review_row(glyph: &str, label: &str, value: &str) -> String {
    format!(
        r#"<div class="review-row"><span>{}</span><div><small>{label}</small><strong>{}</strong></div></div>"#,
        icon(glyph, 16),
        esc(value)
    )
}

impl Flow {
    fn command(&self) -> String {
        let safe_url = self.url.replace('\'', "'\"'\"'");
        let extension = if self.format == "best" {
            "mídia"
        } else {
            &self.format
        };
        format!("curl --fail --location --proto '=https' --output 'arquivo.{extension}' -- '{safe_url}'")
    }

    fn page(&self, transfer: Option<&Snapshot>) -> String {
        match self.step {
            0 => {
                let cards: String = PLATFORMS
                    .iter()
                    .map(|p| choice(p, p.id == self.platform, "platform", ""))
                    .collect();
                let custom = if self.platform == "Outro" {
                    format!(
                        r#"<div class="field-panel custom-folder"><label for="platform-folder">{} Nome da pasta</label><input id="platform-folder" name="destination" value="{}" placeholder="Ex.: Vimeo, Dailymotion ou Cursos" maxlength="60" required></div>"#,
                        icon("folder-open", 15),
                        esc(&self.destination)
                    )
                } else {
                    String::new()
                };
                format!(r#"{}<div class="card-grid platform-grid">{cards}</div>{custom}"#,
                    header("Passo 01 · organização", "Primeiro, escolha o seu <em>destino.</em>", "Cada plataforma recebe uma pasta própria. Assim sua biblioteca continua organizada mesmo depois de centenas de downloads.", "globe-2"))
            }
            1 => screens::source(self),
            2 => {
                let mut groups = String::new();
                for (category, glyph) in [("Vídeo", "monitor-play"), ("Áudio", "audio-lines")] {
                    let cards: String = FORMATS
                        .iter()
                        .filter(|(group, _)| *group == category)
                        .map(|(_, f)| choice(f, f.id == self.format, "format", category))
                        .collect();
                    groups.push_str(&format!(r#"<section><div class="section-label">{} {category}</div><div class="card-grid format-grid">{cards}</div></section>"#, icon(glyph, 15)));
                }
                if self.platform == "TikTok" {
                    let card = choice(&COVER_FORMAT, self.format == "image", "format", "Imagem");
                    groups.push_str(&format!(r#"<section><div class="section-label">{} Imagem</div><div class="card-grid format-grid">{card}</div></section>"#, icon("image-down", 15)));
                } else if self.platform == "Instagram" {
                    groups.push_str(&screens::extra_format(
                        "Imagem",
                        "Imagem (capa)",
                        "Capa original do vídeo",
                        "image-down",
                        "#ff9f68",
                        "image",
                    ));
                }
                groups.push_str(&screens::extra_format(
                    "Legenda",
                    "Legenda junto",
                    "Ative junto com vídeo ou áudio",
                    "subtitles",
                    "#62e6b6",
                    "subtitle",
                ));
                format!(
                    r#"{}<div class="format-sections">{groups}</div>"#,
                    header(
                        "Passo 03 · formato",
                        "Escolha como a mídia deve <em>chegar até você.</em>",
                        "Escolha a mídia e ative a legenda como complemento para salvar os dois arquivos juntos.",
                        "film"
                    )
                )
            }
            3 => screens::settings(self),
            _ => {
                let format = FORMATS
                    .iter()
                    .find(|(_, item)| item.id == self.format)
                    .map(|(_, item)| item.title)
                    .unwrap_or(if self.format == "image" { COVER_FORMAT.title } else { "—" });
                let detail = if self.format == "image" {
                    "Imagem original"
                } else if matches!(self.format.as_str(), "mp3" | "flac" | "wav" | "m4a") {
                    "Melhor faixa de áudio"
                } else {
                    QUALITIES
                        .iter()
                        .find(|item| item.0 == self.quality)
                        .map_or("Melhor disponível", |item| item.1)
                };
                let format = format!("{format} · {detail}");
                let selected = self
                    .selected
                    .filter(|_| self.search)
                    .and_then(|index| self.results.get(index).map(|video| (index, video)));
                let cover = selected
                    .map(|(index, video)| screens::result_card(video, index, true, true))
                    .unwrap_or_default();
                let review_url = if selected.is_none() {
                    review_row("link-2", "URL", &self.url)
                } else {
                    String::new()
                };
                let (status, label) = transfer.map_or(("idle", "Pronto"), |job| {
                    (job.phase.key(), job.phase.label())
                });
                let working = transfer.is_some_and(|job| job.phase.active());
                let disabled = if working { "disabled" } else { "" };
                let button_label = if working {
                    "Transferindo mídia"
                } else {
                    "Iniciar download"
                };
                let button_hint = if working {
                    "Acompanhe o console em tempo real"
                } else {
                    "Executar com o motor Rust"
                };
                let job = transfer
                    .map(|snapshot| screens::job(snapshot, &self.url, &self.destination, self.live))
                    .unwrap_or_default();
                format!(
                    r#"{}<button formnovalidate name="action" value="back" class="back-button review-back-button">{} Voltar para ajustes</button>
                  <div class="review-grid"><section class="review-panel">{cover}<div class="review-list">{}{}{}{review_url}</div></section>
                  <section class="terminal-panel"><div class="terminal-head"><span><i class="terminal-dot red"></i><i class="terminal-dot yellow"></i><i class="terminal-dot green"></i></span>
                  <div class="terminal-head-tools"><span>{} comando seguro</span><button name="action" value="copy-command" title="Copiar comando">{} Copiar</button></div></div><pre>{}</pre></section></div>
                  <section class="download-station status-{status}"><header class="download-station-head"><div><span class="station-mark">{}</span>
                  <span><small>Central de execução</small><strong>Download e telemetria</strong></span></div><span class="download-status-pill status-{status}">{} {label}</span></header>
                  <div class="download-actions"><button name="action" value="download" class="download-button" {disabled}><span class="button-shine"></span>{}<span><strong>{button_label}</strong><small>{button_hint}</small></span></button>
                  <button name="action" value="folder" class="folder-button">{} Abrir pasta</button><button name="action" value="directory" class="folder-button" {disabled}>{} Escolher pasta</button></div>{job}</section>{}"#,
                    header(
                        "Passo 05 · download",
                        "Seu download está <em>pronto para decolar.</em>",
                        "Revise tudo e acompanhe o progresso sem sair do aplicativo.",
                        "hard-drive-download"
                    ),
                    icon("arrow-left", 17),
                    review_row("folder-open", "Destino", &self.destination),
                    review_row("film", "Formato", &format),
                    review_row("cookie", "Acesso", "Sem cookies"),
                    icon("square-terminal", 14),
                    icon("copy", 14),
                    esc(&self.command()),
                    icon("hard-drive-download", 20),
                    icon("hard-drive-download", 15),
                    icon("download", 22),
                    icon("folder-open", 19),
                    icon("settings-2", 18),
                    screens::history(&self.history.records(), &self.day)
                )
            }
        }
    }

    fn document(&self, poll: u32) -> String {
        let step = self.step;
        let transfer = if step == 4 {
            self.transfer.as_ref().map(|job| job.snapshot())
        } else {
            None
        };
        let active = transfer.as_ref().is_some_and(|job| job.phase.active());
        let refresh = if self.live && active {
            format!(
                r#"<meta http-equiv="refresh" content="1;url=?poll={}#download-live">"#,
                poll.wrapping_add(1)
            )
        } else {
            String::new()
        };
        let percent = step * 25;
        let nav: String = STEPS.iter().enumerate().map(|(i, (title, subtitle, glyph))| {
            let class = if step == i { "is-active" } else if step > i { "is-done" } else { "" };
            let indicator = if step == i { "<i></i>" } else { "" };
            let disabled = if i > step { "disabled" } else { "" };
            format!(r#"<button form="wizard" formnovalidate name="step" value="{i}" class="{class}" {disabled}><span class="step-icon">{}</span><span><strong>{title}</strong><small>{subtitle}</small></span>{indicator}</button>"#,
                icon(if i < step { "check" } else { glyph }, 17))
        }).collect();
        let valid = match step {
            0 => !self.platform.is_empty(),
            2 => !self.format.is_empty(),
            1 => !self.search || self.selected.is_some(),
            _ => true,
        };
        let disabled = if valid { "" } else { "disabled" };
        let floating = if step < 4 && valid {
            format!(
                r#"<button form="wizard" class="mobile-continue-fab" name="action" value="next">Continuar {}</button>"#,
                icon("arrow-right", 18)
            )
        } else {
            String::new()
        };
        let controls = if step < 4 {
            let back = if step == 0 { "disabled" } else { "" };
            let hint = if valid {
                "Etapa pronta"
            } else {
                "Complete a seleção"
            };
            format!(
                r#"<div class="transport"><button class="back-button" name="action" value="back" formnovalidate {back}>{} Voltar</button>
                <div class="transport-hint">{} {hint}</div><button class="next-button" name="action" value="next" {disabled}>Continuar {}</button></div>"#,
                icon("arrow-left", 17),
                icon("check-circle-2", 15),
                icon("arrow-right", 17)
            )
        } else {
            String::new()
        };
        let feedback = match &self.status {
            Err(error) => format!(
                r#"<div class="inline-error" role="alert">{} <span>{}</span></div>"#,
                icon("alert-circle", 16),
                esc(error)
            ),
            Ok(Some(message)) => format!(
                r#"<div class="source-search-note" role="status">{} {}</div>"#,
                icon("check-circle-2", 16),
                esc(message)
            ),
            Ok(None) => String::new(),
        };
        let preview = self
            .preview
            .and_then(|index| {
                self.results
                    .get(index)
                    .map(|video| screens::preview(video, index))
            })
            .unwrap_or_default();
        let default_submit = if step == 1 {
            let action = if self.search { "search" } else { "next" };
            format!(
                r#"<button form="wizard" class="sr-only" name="action" value="{action}" tabindex="-1" aria-hidden="true">Continuar</button>"#
            )
        } else {
            String::new()
        };
        format!(
            r##"<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta name="theme-color" content="#08090f"><title>YT-DLP Deck</title><link rel="stylesheet" href="deck.css">{refresh}</head>
          <body>{default_submit}<div id="root"><div class="app-shell"><canvas class="gpu-backdrop is-unavailable" aria-hidden="true"></canvas><div class="ambient ambient-one"></div><div class="ambient ambient-two"></div><div class="noise"></div>
          <header class="topbar"><div class="brand"><span class="brand-logo">{}<i></i></span><div><strong>YT-DLP <em>DECK</em></strong><small>Media acquisition studio</small></div></div>
          <div class="top-center">{}<span>{}</span><i></i><span>{} de 5</span></div><div class="tools"><span class="tool-badge is-ready"><span class="tool-light"></span>Rust</span></div></header>
          <aside class="sidebar"><div class="sidebar-title">Fluxo de download</div><nav>{nav}</nav><div class="sidebar-progress"><div><span>Progresso</span><strong>{percent}%</strong></div>
          <div class="progress-track"><i style="width:{percent}%"></i></div><small>Suas escolhas são mantidas entre as etapas.</small></div></aside>
          <main class="main-stage" data-download-active="{active}"><form id="wizard" class="page" method="post">{}{feedback}{controls}</form>{floating}</main>{preview}</div></div></body></html>"##,
            icon("download", 22),
            icon(STEPS[step].2, 14),
            STEPS[step].0,
            step + 1,
            self.page(transfer.as_ref())
        )
    }

    fn submit(&mut self, body: &[u8], actions: &Arc<impl Actions>) {
        self.status = Ok(None);
        self.preview = None;
        let fields: std::collections::HashMap<_, _> = url::form_urlencoded::parse(body).collect();
        if let Some(step) = fields.get("step").and_then(|n| n.parse::<usize>().ok()) {
            self.step = self.step.min(step);
        } else if let Some(p) = fields
            .get("platform")
            .filter(|p| self.step == 0 && PLATFORMS.iter().any(|item| item.id == p.as_ref()))
        {
            self.platform = p.to_string();
            self.search = false;
            self.selected = None;
            self.results.clear();
            if p.as_ref() != "Outro" {
                self.destination = p.to_string();
            } else {
                self.destination.clear();
            }
        } else if let Some(f) = fields
            .get("format")
            .filter(|f| self.step == 2 && (FORMATS.iter().any(|(_, item)| item.id == f.as_ref())
                || (self.platform == "TikTok" && f.as_ref() == COVER_FORMAT.id)))
        {
            self.format = f.to_string();
        } else if let Some(quality) = fields.get("quality").filter(|quality| {
            self.step == 3 && QUALITIES.iter().any(|item| item.0 == quality.as_ref())
        }) {
            self.quality = quality.to_string();
        } else if self.step == 1 && self.search && fields.contains_key("result") {
            self.selected = fields
                .get("result")
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|index| *index < self.results.len());
        } else if matches!(self.step, 1 | 4) && self.search && fields.contains_key("preview") {
            self.preview = fields
                .get("preview")
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|index| *index < self.results.len());
            self.selected = self.preview;
        } else if matches!(self.step, 1 | 4) && self.search && fields.contains_key("video-open") {
            self.preview = fields
                .get("video-open")
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|index| *index < self.results.len());
            if let Some(video) = self.preview.and_then(|index| self.results.get(index)) {
                if let Err(error) = actions.open_video(&video.id) {
                    self.status = Err(error);
                }
            }
        } else {
            match fields.get("action").map(|value| value.as_ref()) {
                Some("back") => self.step = self.step.saturating_sub(1),
                Some("tab-direct") if self.step == 1 => {
                    self.search = false;
                    self.selected = None;
                }
                Some("tab-search") if self.step == 1 && self.platform == "YouTube" => {
                    self.search = true
                }
                Some("search") if self.step == 1 && self.search => {
                    self.query = fields
                        .get("query")
                        .map(|value| value.trim())
                        .unwrap_or_default()
                        .chars()
                        .take(160)
                        .collect();
                    self.selected = None;
                    self.results.clear();
                    if !self.query.is_empty() {
                        match actions.search(&self.query) {
                            Ok(results) if !results.is_empty() => self.results = results,
                            Ok(_) => self.status = Err("Nenhum resultado encontrado.".into()),
                            Err(error) => self.status = Err(error),
                        }
                    }
                }
                Some("wifi") if self.step == 3 => self.wifi_only = !self.wifi_only,
                Some("immersive") if self.step == 3 => match actions.set_immersive(!self.immersive)
                {
                    Ok(()) => self.immersive = !self.immersive,
                    Err(error) => self.status = Err(error),
                },
                Some("pause" | "resume" | "cancel") if self.step == 4 => {
                    if let (Some(job), Some(action)) = (&self.transfer, fields.get("action")) {
                        job.control(action);
                    }
                }
                Some("live") if self.step == 4 => self.live = !self.live,
                Some("clear-history") if self.step == 4 => self.history.clear(),
                Some("copy-log") if self.step == 4 => {
                    if let Some(job) = &self.transfer {
                        if let Err(error) = actions.copy(&job.snapshot().lines.join("\n")) {
                            self.status = Err(error);
                        }
                    }
                }
                Some("clipboard") if self.step == 1 => match actions.clipboard() {
                    Ok(text) if !text.trim().is_empty() => {
                        self.url = text.trim().chars().take(4096).collect()
                    }
                    Ok(_) => {
                        self.status = Err("A área de transferência não contém um link.".into())
                    }
                    Err(error) => self.status = Err(error),
                },
                Some("copy-command") if self.step == 4 => {
                    if let Err(error) = actions.copy(&self.command()) {
                        self.status = Err(error);
                    }
                }
                Some("folder") if self.step == 4 => {
                    if let Err(error) = actions.open_folder() {
                        self.status = Err(error);
                    }
                }
                Some("cookies" | "directory" | "immersive" | "subtitle" | "image" | "quality") => {
                    self.status = Err("Esta ação ainda não está disponível no motor Rust. Use um link HTTPS direto com a qualidade original.".into());
                }
                Some("next") => match self.step {
                    0 if !self.platform.is_empty() => {
                        if self.platform == "Outro" {
                            let name = fields
                                .get("destination")
                                .map(|value| value.trim())
                                .unwrap_or_default();
                            if name.is_empty()
                                || name.chars().count() > 60
                                || !name
                                    .chars()
                                    .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '_' | '-'))
                            {
                                self.status = Err("Informe um nome de pasta válido.".into());
                                return;
                            }
                            self.destination = name.to_owned();
                        }
                        self.step = 1;
                    }
                    1 if !self.search => {
                        let input = fields
                            .get("url")
                            .map(|value| value.trim())
                            .unwrap_or_default();
                        match url::Url::parse(input) {
                            Ok(url)
                                if url.scheme() == "https"
                                    && url.host().is_some()
                                    && url.username().is_empty()
                                    && url.password().is_none() =>
                            {
                                self.url = url.into();
                                self.step = 2;
                            }
                            _ => {
                                self.status = Err("Informe um link HTTPS direto do arquivo.".into())
                            }
                        }
                    }
                    1 if self.selected.is_some() => {
                        let item = &self.results[self.selected.unwrap()];
                        self.url = format!("https://www.youtube.com/watch?v={}", item.id);
                        self.step = 2;
                    }
                    2 if !self.format.is_empty() => self.step = 3,
                    3 => self.step = 4,
                    _ => {}
                },
                Some("download") if self.step == 4 => {
                    if self
                        .transfer
                        .as_ref()
                        .is_some_and(|job| job.snapshot().phase.active())
                    {
                        return;
                    }
                    let transfer = Arc::new(Transfer::new());
                    let worker = Arc::clone(&transfer);
                    let actions = Arc::clone(actions);
                    let (url, format, folder) = (
                        self.url.clone(),
                        self.format.clone(),
                        self.destination.clone(),
                    );
                    let wifi_only = self.wifi_only;
                    let history = Arc::clone(&self.history);
                    let title = format!("{folder} · link direto");
                    self.transfer = Some(Arc::clone(&transfer));
                    self.live = true;
                    if let Err(error) = std::thread::Builder::new()
                        .name("deck-transfer".into())
                        .spawn(move || {
                            let result =
                                actions.download(&url, &format, &folder, wifi_only, &worker);
                            worker.finish(result);
                            let finished = worker.snapshot();
                            let (date, time) = actions.date_and_time();
                            history.append(Record {
                                title,
                                message: finished.message.clone(),
                                status: finished.phase.key().into(),
                                percent: finished.percent(),
                                date,
                                time,
                            });
                        })
                    {
                        transfer.finish(Err(error.to_string()));
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn listen() -> Result<(Server, String), String> {
    let server = Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
    let mut key = [0_u8; 24];
    getrandom::fill(&mut key).map_err(|e| e.to_string())?;
    let token: String = key.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok((server, format!("/{token}/")))
}

pub fn run(server: Server, root: &str, stop: &AtomicBool, actions: impl Actions) {
    let stylesheet = format!("{root}deck.css");
    let mut flow = Flow {
        history: Arc::new(History::load(actions.history_path())),
        day: actions.date_and_time().0,
        ..Flow::default()
    };
    let actions = Arc::new(actions);
    while !stop.load(Ordering::Acquire) {
        let Ok(request) = server.recv_timeout(Duration::from_millis(250)) else {
            break;
        };
        let Some(mut request) = request else { continue };
        let (path, query) = request.url().split_once('?').unwrap_or((request.url(), ""));
        let poll = query
            .strip_prefix("poll=")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0);
        let (body, kind, status) = match (request.method(), path) {
            (&Method::Get, path) if path == stylesheet => {
                (CSS.to_owned(), "text/css; charset=utf-8", 200)
            }
            (&Method::Get, path) if path == root => {
                (flow.document(poll), "text/html; charset=utf-8", 200)
            }
            (&Method::Post, path) if path == root => {
                let mut body = Vec::new();
                if request
                    .as_reader()
                    .take(8193)
                    .read_to_end(&mut body)
                    .is_ok()
                    && body.len() <= 8192
                {
                    let network_button = flow.step == 3
                        && url::form_urlencoded::parse(&body).any(|(name, value)| {
                            name == "action" && matches!(value.as_ref(), "wifi" | "immersive")
                        });
                    flow.submit(&body, &actions);
                    let location = if flow.step == 4 && flow.transfer.is_some() {
                        format!("{root}#download-live")
                    } else if network_button {
                        format!("{root}#network-settings")
                    } else {
                        root.to_owned()
                    };
                    let response = Response::empty(StatusCode(303))
                        .with_header(header_value("Location", &location));
                    let _ = request.respond(response);
                    continue;
                }
                ("Pedido inválido.".into(), "text/plain; charset=utf-8", 400)
            }
            _ => (String::new(), "text/plain", 404),
        };
        let response = Response::from_string(body).with_status_code(StatusCode(status))
            .with_header(header_value("Content-Type", kind))
            .with_header(header_value("Cache-Control", "no-store"))
            .with_header(header_value("Referrer-Policy", "no-referrer"))
            .with_header(header_value("Content-Security-Policy", "default-src 'none'; script-src 'none'; style-src 'self' 'unsafe-inline'; font-src data:; img-src https://i.ytimg.com; frame-src https://www.youtube-nocookie.com; form-action 'self'; frame-ancestors 'none'; base-uri 'none'"));
        let _ = request.respond(response);
    }
    if let Some(job) = flow.transfer {
        job.control("cancel");
    }
}

fn header_value(name: &str, value: &str) -> Header {
    Header::from_bytes(name, value).expect("valid HTTP header")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpStream, thread};

    #[test]
    fn local_server_serves_only_the_random_route_and_shared_styles() {
        let (server, route) = listen().unwrap();
        let address = server.server_addr().to_ip().unwrap();
        let stop = AtomicBool::new(false);
        thread::scope(|scope| {
            scope.spawn(|| {
                run(
                    server,
                    &route,
                    &stop,
                    |_: &str, _: &str, _: &str| unreachable!(),
                )
            });
            let get = |path: &str| -> String {
                let mut socket = TcpStream::connect(address).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                write!(
                    socket,
                    "GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
                let mut response = String::new();
                socket.read_to_string(&mut response).unwrap();
                response
            };
            let responses = (get("/"), get(&route), get(&format!("{route}deck.css")));
            stop.store(true, Ordering::Release);
            assert!(responses.0.starts_with("HTTP/1.1 404"));
            assert!(responses.1.contains("href=\"deck.css\""));
            assert!(responses.1.contains("form-action 'self'"));
            assert!(responses.2.contains("@font-face"));
            assert!(responses.2.contains(".platform-grid"));
        });
    }

    #[test]
    fn tiktok_shows_the_link_without_a_single_tab_and_accepts_cover() {
        let mut flow = Flow::default();
        let actions = Arc::new(|_: &str, _: &str, _: &str| Ok(String::new()));
        flow.submit(b"platform=TikTok", &actions);
        flow.submit(b"action=next", &actions);
        let html = screens::source(&flow);
        assert!(html.contains("direct-media-url"));
        assert!(html.contains("Links encurtados"));
        assert!(!html.contains("role=\"tablist\""));
        flow.submit(b"action=next&url=https%3A%2F%2Fvt.tiktok.com%2Fabc%2F", &actions);
        flow.submit(b"format=image", &actions);
        assert_eq!(flow.format, "image");
        flow.platform = "YouTube".into();
        assert!(screens::source(&flow).contains("Pesquisar vídeo"));
    }

    #[test]
    fn direct_url_wizard_reaches_the_rust_download_handler() {
        let mut flow = Flow::default();
        assert!(flow
            .document(0)
            .contains("Primeiro, escolha o seu <em>destino.</em>"));
        let download = Arc::new(|url: &str, format: &str, platform: &str| {
            assert_eq!(
                (url, format, platform),
                ("https://example.org/test.mp4", "mp4", "Outro")
            );
            Ok("Arquivo salvo".into())
        });
        for body in [
            "platform=Outro",
            "action=next&destination=Outro",
            "action=next&url=https%3A%2F%2Fexample.org%2Ftest.mp4",
            "format=mp4",
            "action=next",
            "action=next",
            "action=download",
        ] {
            flow.submit(body.as_bytes(), &download);
        }
        assert_eq!(flow.step, 4);
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while flow.transfer.as_ref().unwrap().snapshot().phase.active() {
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        let html = flow.document(0);
        assert!(html.contains("Arquivo salvo"));
        assert!(!html.contains("<script"));
    }
}
