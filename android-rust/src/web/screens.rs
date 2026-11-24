use super::history::Record;
use super::transfer::{Phase, Snapshot};
use super::{choice, esc, header, icon, Choice, Flow, SearchResult, QUALITIES};

fn size(bytes: u64) -> String {
    let (scale, units) = if bytes >= 1_048_576 {
        (1_048_576., "MiB")
    } else {
        (1024., "KiB")
    };
    format!("{:.2} {units}", bytes as f64 / scale)
}

pub(super) fn job(job: &Snapshot, url: &str, folder: &str, live: bool) -> String {
    let phase = job.phase;
    let status = phase.key();
    let percent = job.percent();
    let glyph = match phase {
        Phase::Paused => "circle-pause",
        Phase::Failed | Phase::Cancelled => "triangle-alert",
        Phase::Completed => "check-circle-2",
        Phase::Running => "activity",
    };
    let size_label = job.total.map(size).unwrap_or_else(|| {
        if job.received == 0 {
            "Calculando".into()
        } else {
            size(job.received)
        }
    });
    let speed = if job.received > 0 && phase == Phase::Running && !job.elapsed.is_zero() {
        format!(
            "{}/s",
            size((job.received as f64 / job.elapsed.as_secs_f64()) as u64)
        )
    } else {
        "—".into()
    };
    let eta = if let Some(total) = job
        .total
        .filter(|_| job.received > 0 && phase == Phase::Running)
    {
        let seconds = (job.elapsed.as_secs_f64() * total.saturating_sub(job.received) as f64
            / job.received as f64)
            .ceil() as u64;
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    } else {
        "—".into()
    };
    let actions = if phase.active() {
        let (action, label, glyph) = if phase == Phase::Paused {
            ("resume", "Retomar", "circle-play")
        } else {
            ("pause", "Pausar", "circle-pause")
        };
        format!(
            r#"<button name="action" value="{action}">{}<span>{label}</span></button><button name="action" value="cancel" class="is-danger">{}<span>Cancelar</span></button>"#,
            icon(glyph, 16),
            icon("x", 16)
        )
    } else {
        String::new()
    };
    let (badge, live_class, live_glyph) = if live {
        ("Ao vivo", "is-live", "pause")
    } else {
        ("Pausado", "", "play")
    };
    let mut lines = job.lines.clone();
    if phase.active() && job.received > 0 {
        lines.push(format!(
            "[download] {percent}% de {size_label} · {speed} · ETA {eta}"
        ));
    }
    let count = lines.len();
    let console: String = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let kind = if line.starts_with("ERRO:") {
                "error"
            } else if line.starts_with("[download]") {
                "progress"
            } else {
                "info"
            };
            format!(
                r#"<span class="console-line is-{kind}"><i>{:03}</i><code>{}</code></span>"#,
                i + 1,
                esc(line)
            )
        })
        .collect();
    let message = if job.message.is_empty() {
        String::new()
    } else {
        let class = if phase == Phase::Completed {
            "success-message"
        } else {
            "inline-error"
        };
        format!(
            r#"<div class="{class}" role="status">{} <span>{}</span></div>"#,
            icon(glyph, 17),
            esc(&job.message)
        )
    };
    format!(
        r#"<article id="download-live" class="download-job-card status-{status}">
      <div class="download-job-thumbnail">{}<span class="job-state-icon status-{status}">{}</span></div>
      <div class="download-job-body"><div class="download-job-title"><span><small>Item atual</small><strong>{} · link direto</strong></span><span class="job-percent">{percent}%</span></div>
      <p>{}</p><div class="download-telemetry"><span><small>Tamanho</small><strong>{size_label}</strong></span><span><small>Velocidade</small><strong>{speed}</strong></span><span><small>Tempo restante</small><strong>{eta}</strong></span></div>
      <div class="job-progress-track" role="progressbar" aria-label="Progresso do download" aria-valuemin="0" aria-valuemax="100" aria-valuenow="{percent}"><i style="width:{percent}%"></i><span style="left:{percent}%"></span></div></div>
      <div class="download-job-actions"><button name="action" value="folder">{}<span>Abrir pasta</span></button>{actions}</div></article>{message}
      <div class="live-console-shell"><div class="live-console-head"><span>{}<span><strong>console Rust</strong><small>{count} linhas · Android nativo</small></span></span>
      <div class="console-actions"><button name="action" value="live" class="{live_class}">{} {badge}</button><button name="action" value="copy-log">{} Copiar</button></div></div>
      <div class="live-console" role="log" tabindex="0" aria-label="Saída em tempo real do motor Rust">{console}</div></div>"#,
        icon("hard-drive-download", 28),
        icon(glyph, 16),
        esc(folder),
        esc(url),
        icon("folder-open", 16),
        icon("square-terminal", 14),
        icon(live_glyph, 14),
        icon("copy", 14)
    )
}

pub(super) fn extra_format(
    group: &'static str,
    title: &'static str,
    detail: &'static str,
    glyph: &'static str,
    color: &'static str,
    id: &'static str,
) -> String {
    let card = choice(
        &Choice {
            id,
            title,
            detail,
            icon: glyph,
            color,
        },
        false,
        "action",
        group,
    );
    format!(
        r#"<section><div class="section-label">{} {group}</div><div class="card-grid format-grid">{card}</div></section>"#,
        icon(glyph, 15)
    )
}

pub(super) fn source(flow: &Flow) -> String {
    let search_available = flow.platform == "YouTube";
    let search = flow.search && search_available;
    let direct = if search { "" } else { "is-active" };
    let search_class = if search { "is-active" } else { "" };
    let tabs = if search_available { "" } else { "is-single" };
    let position = if search { "100%" } else { "0" };
    let search_button = if search_available {
        format!(
            r#"<button type="submit" name="action" value="tab-search" formnovalidate role="tab" aria-selected="{search}" class="{search_class}">{} Pesquisar vídeo</button>"#,
            icon("search", 16)
        )
    } else {
        String::new()
    };
    let note = if search_available {
        String::new()
    } else {
        format!(
            r#"<div class="source-search-note" role="note">{} A pesquisa por nome usa o YouTube. Para {}, use um link direto.</div>"#,
            icon("youtube", 15),
            esc(&flow.platform)
        )
    };
    let panel = if search {
        let results = if flow.results.is_empty() {
            format!(
                r#"<div class="empty-state"><span class="empty-orbit">{}</span><strong>As capas aparecerão aqui</strong><small>Mostraremos até cinco resultados para você comparar.</small></div>"#,
                icon("search", 26)
            )
        } else {
            let cards: String = flow
                .results
                .iter()
                .enumerate()
                .map(|(index, video)| {
                    result_card(video, index, flow.selected == Some(index), false)
                })
                .collect();
            format!(r#"<div class="media-grid">{cards}</div>"#)
        };
        format!(
            r#"<div id="source-search-panel" role="tabpanel" class="source-area"><div class="field-panel hero-input"><label for="video-search-query">{} Pesquisa inteligente</label>
          <div class="input-actions"><input id="video-search-query" name="query" type="search" value="{}" placeholder="Música, tutorial, podcast, artista...">
          <button name="action" value="search" class="primary-button">{} Buscar</button></div></div>
          {results}</div>"#,
            icon("search", 15),
            esc(&flow.query),
            icon("search", 18)
        )
    } else {
        let example = match flow.platform.as_str() {
            "YouTube" => "https://www.youtube.com/watch?v=...",
            "TikTok" => "https://www.tiktok.com/@usuario/video/...",
            "Instagram" => "https://www.instagram.com/reel/...",
            "X_Twitter" => "https://x.com/usuario/status/...",
            "Twitch" => "https://www.twitch.tv/videos/...",
            "Reddit" => "https://www.reddit.com/r/comunidade/comments/...",
            _ => "https://exemplo.com/video",
        };
        format!(
            r#"<div id="source-url-panel" role="tabpanel" class="source-area"><div class="field-panel hero-input"><label for="direct-media-url">{} URL da mídia</label>
          <div class="input-actions"><input id="direct-media-url" name="url" type="url" inputmode="url" placeholder="{example}" value="{}" required>
          <button class="soft-button" name="action" value="clipboard" formnovalidate>{} Colar</button></div></div></div>"#,
            icon("link-2", 15),
            esc(&flow.url),
            icon("clipboard", 17)
        )
    };
    format!(r#"{}<div class="source-tabs {tabs}" role="tablist" aria-label="Forma de escolher o conteúdo"><button name="action" value="tab-direct" formnovalidate class="{direct}" role="tab" aria-selected="{}">{} Link direto</button>
      {search_button}<span class="tab-indicator" style="transform:translateX({position})"></span></div>{note}{panel}"#,
      header("Passo 02 · conteúdo", "Encontre exatamente o que você <em>quer baixar.</em>", "Cole um link direto ou pesquise pelo nome. Na pesquisa, cada resultado aparece com sua capa para você escolher com segurança.", "wand-sparkles"),
      !search, icon("link-2", 16))
}

pub(super) fn result_card(
    video: &SearchResult,
    index: usize,
    selected: bool,
    large: bool,
) -> String {
    let active = if selected { "is-selected" } else { "" };
    let layout = if large { "is-large" } else { "" };
    let title = esc(&video.title);
    let thumbnail = if video.thumbnail.is_empty() {
        format!(r#"<div class="cover-fallback">{}</div>"#, icon("film", 34))
    } else {
        format!(
            r#"<img loading="lazy" decoding="async" src="{}" alt="Capa de {title}">"#,
            esc(&video.thumbnail)
        )
    };
    let label = if selected {
        format!(
            r#"<span class="selected-chip">{} Selecionado</span>"#,
            icon("check-circle-2", 14)
        )
    } else {
        String::new()
    };
    let hitbox = if large {
        String::new()
    } else {
        format!(
            r#"<button name="result" value="{index}" class="media-select-hitbox" aria-label="Selecionar {title}" aria-pressed="{selected}"><span class="sr-only">Selecionar {title}</span></button>"#
        )
    };
    format!(
        r#"<article class="media-card {active} {layout}">{hitbox}
      <div class="media-cover">{thumbnail}<div class="cover-shade"></div><span class="duration-badge">{} {}</span>
      <button name="preview" value="{index}" class="play-orbit" aria-label="Reproduzir prévia de {title}" title="Reproduzir prévia">{}</button></div>
      <div class="media-content"><div class="media-title">{title}</div><div class="media-meta">{}<span>Resultado da pesquisa</span></div></div>{label}</article>"#,
        icon("clock-3", 12),
        esc(&video.duration),
        icon("play", 18).replace(r#"fill="none""#, r#"fill="currentColor""#),
        icon("user-round", 12)
    )
}

pub(super) fn preview(video: &SearchResult, index: usize) -> String {
    format!(
        r#"<div class="video-preview-backdrop"><section class="video-preview-dialog" role="dialog" aria-modal="true" aria-label="Prévia de {}">
      <header class="video-preview-head"><div><span>{} Prévia do vídeo</span><strong>{}</strong></div>
      <button form="wizard" name="action" value="close-preview" aria-label="Fechar prévia" title="Fechar">{}</button></header>
      <div class="video-preview-frame"><iframe src="https://www.youtube-nocookie.com/embed/{}?autoplay=1&amp;rel=0&amp;modestbranding=1"
        sandbox="allow-scripts allow-same-origin allow-presentation" allow="autoplay; encrypted-media; picture-in-picture; fullscreen" allowfullscreen referrerpolicy="strict-origin-when-cross-origin" title="Prévia de {}"></iframe></div>
      <footer class="video-preview-footer"><div><span>{} Este vídeo já está selecionado para download</span><small>Se a prévia não carregar, abra o vídeo diretamente no YouTube.</small></div>
      <button form="wizard" name="video-open" value="{index}">{} Abrir no YouTube</button></footer>
      </section></div>"#,
        esc(&video.title),
        icon("monitor-play", 15),
        esc(&video.title),
        icon("x", 20),
        esc(&video.id),
        esc(&video.title),
        icon("check-circle-2", 15),
        icon("external-link", 15)
    )
}
pub(super) fn settings(flow: &Flow) -> String {
    let wifi_only = flow.wifi_only;
    let wifi_class = if wifi_only { "is-active" } else { "" };
    let immersive = flow.immersive;
    let immersive_class = if immersive { "is-active" } else { "" };
    let audio = matches!(flow.format.as_str(), "mp3" | "flac" | "wav" | "m4a");
    let (label, explanation, class, quality) = if audio {
        (
            "Saída selecionada",
            "Não se aplica ao formato de áudio",
            "is-disabled",
            format!(
                r#"<div class="audio-quality-message">{}<strong>Qualidade máxima de áudio</strong><small>O arquivo será preservado com a qualidade original.</small></div>"#,
                icon("audio-lines", 28)
            ),
        )
    } else {
        let mut choices = String::new();
        for (id, title, detail) in QUALITIES {
            let selected = id == flow.quality;
            let class = if selected { "is-active" } else { "" };
            let mark = if selected {
                icon("check", 15)
            } else {
                String::new()
            };
            choices.push_str(&format!(r#"<button class="{class}" name="quality" value="{id}" aria-pressed="{selected}"><span class="radio-dot"></span><span><strong>{title}</strong><small>{detail}</small></span>{mark}</button>"#));
        }
        (
            "Qualidade do vídeo",
            "Com fallback automático",
            "",
            format!(r#"<div class="quality-list">{choices}</div>"#),
        )
    };
    format!(
        r#"{}<div class="settings-layout">
      <section class="settings-panel {class}"><div class="panel-heading"><span>{}</span><div><strong>{label}</strong><small>{explanation}</small></div></div>{quality}</section>
      <section class="settings-panel"><div class="panel-heading"><span>{}</span><div><strong>Cookies e autenticação</strong><small>Para login, idade ou conteúdo privado</small></div></div>
      <div class="cookie-grid"><button type="button" class="is-active">{}<span><strong>Sem cookies</strong><small>Conteúdo público</small></span>{}</button>
      <button name="action" value="cookies">{}<span><strong>Arquivo</strong><small>cookies.txt</small></span></button></div></section>
      <section id="network-settings" class="settings-panel mobile-network-panel"><div class="panel-heading"><span>{}</span><div><strong>Rede e bateria</strong><small>Controle o consumo de dados móveis</small></div></div>
      <button class="mobile-setting-toggle {wifi_class}" name="action" value="wifi" role="switch" aria-checked="{wifi_only}"><span>{}<span><strong>Baixar somente por Wi-Fi</strong><small>Bloqueia o início em redes móveis</small></span></span><i></i></button>
      <button class="mobile-setting-toggle {immersive_class}" name="action" value="immersive" role="switch" aria-checked="{immersive}"><span>{}<span><strong>Ocultar navegação do sistema</strong><small>Deslize de baixo para cima para mostrá-la temporariamente</small></span></span><i></i></button></section></div>"#,
        header(
            "Passo 04 · controle",
            "Ajuste qualidade e <em>acesso.</em>",
            "Escolha a resolução quando necessário. Links diretos preservam a qualidade do arquivo original da mídia.",
            "settings-2"
        ),
        icon("gauge", 19),
        icon("cookie", 19),
        icon("shield-check", 18),
        icon("check-circle-2", 16),
        icon("file-text", 18),
        icon("wifi", 19),
        icon("wifi", 17),
        icon("monitor-play", 17)
    )
}

pub(super) fn history(records: &[Record], today: &str) -> String {
    let clear = if records.is_empty() {
        String::new()
    } else {
        format!(
            r#"<button name="action" value="clear-history">{} Limpar finalizados</button>"#,
            icon("trash-2", 15)
        )
    };
    let content = if records.is_empty() {
        format!(
            r#"<div class="mobile-library-empty history-empty-state"><span>{}</span><strong>Nenhum download no histórico</strong><small>As tarefas finalizadas aparecerão aqui, organizadas por data.</small></div>"#,
            icon("history", 24)
        )
    } else {
        let mut html = String::from(r#"<div class="history-groups">"#);
        for group in records.chunk_by(|a, b| a.date == b.date) {
            let date = &group[0].date;
            let date = if date == today { "Hoje" } else { date };
            let count = group.len();
            let label = if count == 1 { "item" } else { "itens" };
            html.push_str(&format!(r#"<section class="history-date-group"><div class="history-date-label"><span>{}</span><i></i><small>{count} {label}</small></div><div class="mobile-history-list">"#, esc(date)));
            for record in group.iter().take(12) {
                let (status, label, glyph) = match record.status.as_str() {
                    "completed" => ("completed", "Concluído", "check-circle-2"),
                    "cancelled" => ("cancelled", "Cancelado", "triangle-alert"),
                    _ => ("failed", "Falhou", "triangle-alert"),
                };
                html.push_str(&format!(r#"<article class="mobile-history-item status-{status}"><span class="history-status-icon">{}</span>
                  <div class="mobile-history-main"><strong>{}</strong><span>{label}<i>·</i><b>{}%</b><i>·</i>{}</span><small>{}</small></div>
                  <div class="mobile-history-actions"><button name="action" value="folder" title="Abrir pasta">{}</button></div></article>"#,
                  icon(glyph,17), esc(&record.title), record.percent, esc(&record.time), esc(&record.message), icon("folder-open",16)));
            }
            html.push_str("</div></section>");
        }
        html.push_str("</div>");
        html
    };
    format!(
        r#"<section class="mobile-download-library history-library"><div class="mobile-library-head"><div><span>{} Histórico</span><small>Biblioteca persistente de downloads</small></div>{clear}</div>{content}</section>"#,
        icon("history", 17)
    )
}
