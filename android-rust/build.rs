use std::{
    env, fs,
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use regex::Regex;

fn read(path: &Path) -> String {
    println!("cargo:rerun-if-changed={}", path.display());
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn spring(stiffness: f64, damping: f64, distance: f64) -> String {
    let decay = damping / 2.0;
    let frequency = (stiffness - decay * decay).sqrt();
    if !frequency.is_finite() || frequency == 0.0 {
        return "260ms cubic-bezier(.2,.8,.2,1)".to_owned();
    }
    let sample = |t: f64| {
        let (sin, cos) = (frequency * t).sin_cos();
        let envelope = (-decay * t).exp();
        let position = 1.0 - envelope * (cos + decay * sin / frequency);
        let speed = envelope * stiffness * sin / frequency;
        (position, speed)
    };
    let (rest_speed, rest_delta) = if distance < 5.0 {
        (0.01, 0.005)
    } else {
        (2.0, 0.5)
    };
    let duration = (10..=1500)
        .step_by(10)
        .find(|ms| {
            let (progress, speed) = sample(f64::from(*ms) / 1000.0);
            (1.0 - progress).abs() * distance <= rest_delta && speed.abs() * distance <= rest_speed
        })
        .unwrap_or(1500);
    let points = (0..24)
        .map(|i| {
            let t = f64::from(duration) / 1000.0 * f64::from(i) / 24.0;
            format!("{:.6}", sample(t).0)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{duration}ms linear({points},1)")
}

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-arg-cdylib=-Wl,-z,max-page-size=16384");
    }
    let root = Path::new("..");
    let modules = root.join("node_modules");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let mut css = String::new();
    let url = Regex::new(r"url\(\./files/([^\)]+)\)").unwrap();
    for family in ["space-grotesk", "manrope", "jetbrains-mono"] {
        let folder = modules.join("@fontsource-variable").join(family);
        let font_styles = read(&folder.join("index.css"));
        css.push_str(&url.replace_all(&font_styles, |matches: &regex::Captures| {
            let path = folder.join("files").join(&matches[1]);
            println!("cargo:rerun-if-changed={}", path.display());
            format!(
                "url(data:font/woff2;base64,{})",
                STANDARD.encode(fs::read(path).unwrap())
            )
        }));
    }
    let src = root.join("src");
    let shaders = read(&src.join("visuals/gpuBackdrop.ts"));
    for (identifier, file) in [("glslVertex", "deck.vert"), ("glslFragment", "deck.frag")] {
        let source = shaders
            .split_once(&format!("const {identifier} = `"))
            .and_then(|(_, rest)| rest.split_once('`'))
            .expect("original backdrop shader")
            .0;
        fs::write(out.join(file), source).unwrap();
    }
    for line in read(&src.join("styles.css")).lines() {
        if let Some(path) = line
            .strip_prefix("@import \"./")
            .and_then(|s| s.strip_suffix("\";"))
        {
            css.push_str(&read(&src.join(path)));
            css.push('\n');
        }
    }
    css.push_str("\n:root, .app-shell { background: transparent; }\n");
    css.push_str(
        ".main-stage:has(:is(#direct-media-url,#platform-folder):invalid) .mobile-continue-fab { display:none; }\n\
         .page:has(:is(#direct-media-url,#platform-folder):invalid) .next-button { opacity:.4; pointer-events:none; }\n\
         ",
    );
    let motion = read(&src.join("config/motion.ts"));
    let params = Regex::new(r"stiffness:\s*([0-9.]+),\s*damping:\s*([0-9.]+)").unwrap();
    let params = params.captures(&motion).expect("original drawer spring");
    let stiffness = params[1].parse().unwrap();
    let damping = params[2].parse().unwrap();
    let movement = spring(stiffness, damping, 24.0);
    let opacity = spring(stiffness, damping, 1.0);
    css.push_str(&format!(
        "@media (prefers-reduced-motion:no-preference) {{\n\
           [data-download-active=false] > .page {{ animation:deck-page-y 260ms ease-out both, deck-page-opacity 260ms ease-out both; animation:deck-page-y {movement} both, deck-page-opacity {opacity} both; }}\n\
           @keyframes deck-page-y {{ from {{ transform:translateY(24px); }} to {{ transform:none; }} }}\n\
           @keyframes deck-page-opacity {{ from {{ opacity:0; }} to {{ opacity:1; }} }}\n\
        }}\n"
    ));
    fs::write(out.join("deck.css"), css).unwrap();

    let icons = [
        "activity",
        "alert-circle",
        "arrow-left",
        "arrow-right",
        "audio-lines",
        "check",
        "check-circle-2",
        "circle-pause",
        "circle-play",
        "clipboard",
        "clock-3",
        "cookie",
        "copy",
        "disc-3",
        "download",
        "external-link",
        "file-audio",
        "file-text",
        "film",
        "folder-open",
        "gauge",
        "globe-2",
        "hard-drive-download",
        "history",
        "image-down",
        "instagram",
        "link-2",
        "message-circle",
        "monitor-play",
        "music-2",
        "pause",
        "play",
        "radio",
        "search",
        "settings-2",
        "shield-check",
        "sliders-horizontal",
        "sparkles",
        "square-terminal",
        "subtitles",
        "twitter",
        "user-round",
        "trash-2",
        "triangle-alert",
        "video",
        "wand-sparkles",
        "wifi",
        "x",
        "youtube",
    ];
    let tag = Regex::new(r#"\[\s*"([a-z]+)"\s*,\s*\{([^{}]*)\}\s*\]"#).unwrap();
    let attribute = Regex::new(r#"([\w-]+)\s*:\s*"([^"]*)""#).unwrap();
    let alias = Regex::new(r"export \{ default \} from '\./([a-z0-9-]+\.js)'").unwrap();
    let mut generated = String::from("fn icon_paths(name: &str) -> &'static str { match name {\n");
    for name in icons {
        let path = modules
            .join("lucide-react/dist/esm/icons")
            .join(format!("{name}.js"));
        let mut source = read(&path);
        if let Some(target) = alias.captures(&source) {
            source = read(&path.with_file_name(&target[1]));
        }
        let mut svg = String::new();
        for node in tag.captures_iter(&source) {
            svg.push_str(&format!("<{}", &node[1]));
            for attr in attribute.captures_iter(&node[2]) {
                if &attr[1] != "key" {
                    svg.push_str(&format!(r#" {}="{}""#, &attr[1], &attr[2]));
                }
            }
            svg.push_str("/>");
        }
        assert!(!svg.is_empty(), "no paths for {name}");
        generated.push_str(&format!("{name:?} => {svg:?},\n"));
    }
    generated.push_str("_ => \"\",\n} }\n");
    fs::write(out.join("icons.rs"), generated).unwrap();
}
