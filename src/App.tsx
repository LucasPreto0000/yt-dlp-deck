import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, MotionConfig, motion } from "framer-motion";
import {
  Activity,
  AlertCircle,
  ArrowLeft,
  ArrowRight,
  AudioLines,
  Check,
  CheckCircle2,
  Chrome,
  Clipboard,
  Clock3,
  Compass,
  Cookie,
  Disc3,
  Download,
  ExternalLink,
  FileAudio,
  FileText,
  Film,
  Flame,
  FolderOpen,
  Gauge,
  Globe2,
  HardDriveDownload,
  Instagram,
  Link2,
  ListVideo,
  LoaderCircle,
  LucideIcon,
  MessageCircle,
  MonitorPlay,
  Music2,
  Pause,
  Play,
  Radio,
  RotateCcw,
  Search,
  Settings2,
  Share2,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  SquareTerminal,
  Trash2,
  Twitter,
  UserRound,
  Video,
  WandSparkles,
  Wifi,
  X,
  Youtube,
} from "lucide-react";
import type {
  CookieId,
  DownloadRequest,
  DownloadResult,
  FormatId,
  MobileCookieFile,
  MobileDownloadHistory,
  MobileDownloadRecord,
  MobileDownloadState,
  MobileSettings,
  QualityId,
  SearchResult,
  SourceMode,
  ToolStatus,
} from "./types";
import {
  appInvoke,
  isAndroidRuntime,
  listenDownloadOutput,
  listenMobilePluginEvent,
} from "./nativeBridge";
import { startGpuBackdrop, type BackdropController } from "./visuals/gpuBackdrop";

const transition = { type: "spring" as const, stiffness: 320, damping: 30 };

const steps: Array<{ title: string; subtitle: string; icon: LucideIcon }> = [
  { title: "Plataforma", subtitle: "Organize o destino", icon: Globe2 },
  { title: "Conteúdo", subtitle: "Link ou pesquisa", icon: Search },
  { title: "Formato", subtitle: "Vídeo ou áudio", icon: Film },
  { title: "Ajustes", subtitle: "Qualidade e acesso", icon: SlidersHorizontal },
  { title: "Finalizar", subtitle: "Revise e baixe", icon: Download },
];

const platforms: Array<{
  id: string;
  label: string;
  description: string;
  icon: LucideIcon;
  color: string;
  urlExample: string;
}> = [
  { id: "YouTube", label: "YouTube", description: "Vídeos, Shorts e lives", icon: Youtube, color: "#ff4164", urlExample: "https://www.youtube.com/watch?v=..." },
  { id: "TikTok", label: "TikTok", description: "Vídeos e tendências", icon: Music2, color: "#46edf2", urlExample: "https://www.tiktok.com/@usuario/video/..." },
  { id: "Instagram", label: "Instagram", description: "Reels e publicações", icon: Instagram, color: "#f765a3", urlExample: "https://www.instagram.com/reel/..." },
  { id: "X_Twitter", label: "X / Twitter", description: "Vídeos e clipes", icon: Twitter, color: "#60baff", urlExample: "https://x.com/usuario/status/..." },
  { id: "Twitch", label: "Twitch", description: "Clipes e transmissões", icon: Radio, color: "#a978ff", urlExample: "https://www.twitch.tv/videos/..." },
  { id: "Reddit", label: "Reddit", description: "Mídias de comunidades", icon: MessageCircle, color: "#ff7045", urlExample: "https://www.reddit.com/r/comunidade/comments/..." },
  { id: "Outro", label: "Outro site", description: "Qualquer site compatível", icon: Globe2, color: "#aab2d1", urlExample: "https://exemplo.com/video" },
];

const formats: Array<{
  id: FormatId;
  label: string;
  description: string;
  category: "Vídeo" | "Áudio";
  icon: LucideIcon;
  color: string;
}> = [
  { id: "mp4", label: "MP4", description: "Compatibilidade máxima", category: "Vídeo", icon: Video, color: "#62a8ff" },
  { id: "mkv", label: "MKV", description: "Todos os codecs", category: "Vídeo", icon: Disc3, color: "#a97aff" },
  { id: "webm", label: "WEBM", description: "Formato web moderno", category: "Vídeo", icon: Globe2, color: "#43d9ff" },
  { id: "best", label: "Melhor", description: "Qualidade sem limites", category: "Vídeo", icon: Sparkles, color: "#ffbd58" },
  { id: "mp3", label: "MP3", description: "Universal e compacto", category: "Áudio", icon: Music2, color: "#ff6e9a" },
  { id: "flac", label: "FLAC", description: "Áudio sem perdas", category: "Áudio", icon: AudioLines, color: "#44dda8" },
  { id: "wav", label: "WAV", description: "Sem compressão", category: "Áudio", icon: FileAudio, color: "#dfe8ff" },
  { id: "m4a", label: "M4A", description: "Eficiente e moderno", category: "Áudio", icon: FileAudio, color: "#9d9cff" },
];

const qualities: Array<{ id: QualityId; label: string; detail: string }> = [
  { id: "best", label: "Melhor disponível", detail: "Sem limite" },
  { id: "2160p60", label: "4K · 60 FPS", detail: "2160p" },
  { id: "1440p60", label: "2K · 60 FPS", detail: "1440p" },
  { id: "1080p60", label: "Full HD · 60 FPS", detail: "1080p" },
  { id: "1080p", label: "Full HD", detail: "1080p" },
  { id: "720p", label: "HD", detail: "720p" },
  { id: "480p", label: "Compacto", detail: "480p" },
];

const mobileStatusLabels: Record<string, string> = {
  queued: "Na fila",
  running: "Baixando",
  paused: "Pausado",
  processing: "Processando",
  saving: "Salvando",
  completed: "Concluído",
  failed: "Falhou",
  cancelled: "Cancelado",
};

const cookieOptions: Array<{
  id: CookieId;
  label: string;
  detail: string;
  icon: LucideIcon;
}> = [
  { id: "none", label: "Sem cookies", detail: "Conteúdo público", icon: ShieldCheck },
  { id: "chrome", label: "Chrome", detail: "Sessão do navegador", icon: Chrome },
  { id: "edge", label: "Edge", detail: "Sessão do navegador", icon: Compass },
  { id: "firefox", label: "Firefox", detail: "Sessão do navegador", icon: Flame },
  { id: "file", label: "Arquivo", detail: "cookies.txt", icon: FileText },
];

const errorText = (error: unknown) => (error instanceof Error ? error.message : String(error));
const isAudio = (format: FormatId | null) =>
  format === "mp3" || format === "flac" || format === "wav" || format === "m4a";

function ToolBadge({
  label,
  ready,
  version,
}: {
  label: string;
  ready: boolean;
  version?: string;
}) {
  return (
    <div className={`tool-badge ${ready ? "is-ready" : ""}`} title={version}>
      <span className="tool-light" />
      <span>{label}</span>
      {ready && <Check size={12} />}
    </div>
  );
}

const ChoiceCard = memo(function ChoiceCard({
  active,
  icon: Icon,
  color,
  title,
  description,
  tag,
  onClick,
}: {
  active: boolean;
  icon: LucideIcon;
  color: string;
  title: string;
  description: string;
  tag?: string;
  onClick: () => void;
}) {
  return (
    <motion.button
      type="button"
      className={`choice-card ${active ? "is-active" : ""}`}
      style={{ "--card-color": color } as React.CSSProperties}
      onClick={onClick}
      whileHover={{ y: -6 }}
      whileTap={{ scale: 0.98 }}
    >
      <span className="choice-glow" />
      <span className="choice-icon">
        <Icon size={24} />
      </span>
      {tag && <span className="choice-tag">{tag}</span>}
      <span className="choice-copy">
        <strong>{title}</strong>
        <small>{description}</small>
      </span>
      <motion.span
        className="choice-check"
        initial={false}
        animate={{ scale: active ? 1 : 0, opacity: active ? 1 : 0 }}
        transition={transition}
      >
        <Check size={14} />
      </motion.span>
    </motion.button>
  );
});

const MediaCard = memo(function MediaCard({
  media,
  selected,
  onClick,
  onPreview,
  large = false,
}: {
  media: SearchResult;
  selected: boolean;
  onClick?: () => void;
  onPreview?: (trigger: HTMLButtonElement) => void;
  large?: boolean;
}) {
  const uploader = "Resultado da pesquisa";
  return (
    <motion.article
      className={`media-card ${selected ? "is-selected" : ""} ${large ? "is-large" : ""}`}
      whileHover={onClick ? { y: -6 } : undefined}
    >
      {onClick && (
        <button
          type="button"
          className="media-select-hitbox"
          onClick={onClick}
          aria-label={`Selecionar ${media.title}`}
          aria-pressed={selected}
        >
          <span className="sr-only">Selecionar {media.title}</span>
        </button>
      )}
      <div className="media-cover">
        {media.thumbnail ? (
          <img
            src={media.thumbnail}
            alt={`Capa de ${media.title}`}
            loading="lazy"
            decoding="async"
          />
        ) : (
          <div className="cover-fallback">
            <Film size={34} />
          </div>
        )}
        <div className="cover-shade" />
        <span className="duration-badge">
          <Clock3 size={12} /> {media.duration || "—"}
        </span>
        {onPreview && (
          <button
            type="button"
            className="play-orbit"
            aria-label={`Reproduzir prévia de ${media.title}`}
            title="Reproduzir prévia"
            onClick={(event) => {
              event.stopPropagation();
              onPreview(event.currentTarget);
            }}
          >
            <Play size={18} fill="currentColor" />
          </button>
        )}
      </div>
      <div className="media-content">
        <div className="media-title">{media.title}</div>
        <div className="media-meta">
          <UserRound size={12} />
          <span>{uploader}</span>
        </div>
      </div>
      <AnimatePresence>
        {selected && (
          <motion.span
            className="selected-chip"
            initial={{ opacity: 0, scale: 0.7, y: -6 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.7 }}
          >
            <CheckCircle2 size={14} /> Selecionado
          </motion.span>
        )}
      </AnimatePresence>
    </motion.article>
  );
});

function getYouTubeEmbedUrl(media: SearchResult) {
  let videoId = media.id.trim();
  if (!/^[\w-]{11}$/.test(videoId)) {
    try {
      const parsed = new URL(media.url);
      videoId = parsed.hostname.includes("youtu.be")
        ? parsed.pathname.slice(1)
        : parsed.searchParams.get("v") || "";
    } catch {
      videoId = "";
    }
  }
  if (!videoId) return "";
  return `https://www.youtube-nocookie.com/embed/${encodeURIComponent(videoId)}?autoplay=1&rel=0&modestbranding=1`;
}

function VideoPreview({
  media,
  onClose,
  onOpenExternal,
}: {
  media: SearchResult;
  onClose: () => void;
  onOpenExternal: () => void;
}) {
  const embedUrl = getYouTubeEmbedUrl(media);
  const dialogRef = useRef<HTMLElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const background = document.querySelectorAll<HTMLElement>(".topbar, .sidebar, .main-stage");
    background.forEach((element) => element.setAttribute("inert", ""));
    closeButtonRef.current?.focus();

    const keepFocusInside = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== "Tab" || !dialogRef.current) return;
      const focusable = Array.from(
        dialogRef.current.querySelectorAll<HTMLElement>(
          'button:not([disabled]), iframe, [href], [tabindex]:not([tabindex="-1"])',
        ),
      );
      if (!focusable.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };

    window.addEventListener("keydown", keepFocusInside);
    return () => {
      window.removeEventListener("keydown", keepFocusInside);
      background.forEach((element) => element.removeAttribute("inert"));
    };
  }, [onClose]);

  return (
    <motion.div
      className="video-preview-backdrop"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      onClick={onClose}
    >
      <motion.section
        ref={dialogRef}
        className="video-preview-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={`Prévia de ${media.title}`}
        initial={{ opacity: 0, y: 24, scale: 0.96 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, y: 18, scale: 0.97 }}
        transition={transition}
        onClick={(event) => event.stopPropagation()}
      >
        <header className="video-preview-head">
          <div>
            <span><MonitorPlay size={15} /> Prévia do vídeo</span>
            <strong>{media.title}</strong>
          </div>
          <button
            ref={closeButtonRef}
            type="button"
            onClick={onClose}
            aria-label="Fechar prévia"
            title="Fechar"
          >
            <X size={20} />
          </button>
        </header>
        <div className="video-preview-frame">
          {embedUrl ? (
            <iframe
              key={embedUrl}
              src={embedUrl}
              title={`Prévia de ${media.title}`}
              allow="autoplay; encrypted-media; picture-in-picture; fullscreen"
              allowFullScreen
              referrerPolicy="strict-origin-when-cross-origin"
            />
          ) : (
            <div className="video-preview-unavailable">
              <AlertCircle size={26} />
              <strong>Não foi possível montar a prévia deste vídeo.</strong>
            </div>
          )}
        </div>
        <footer className="video-preview-footer">
          <div>
            <span><CheckCircle2 size={15} /> Este vídeo já está selecionado para download</span>
            <small>Se a prévia não carregar, abra o vídeo diretamente no YouTube.</small>
          </div>
          <button type="button" onClick={onOpenExternal}>
            <ExternalLink size={15} /> Abrir no YouTube
          </button>
        </footer>
      </motion.section>
    </motion.div>
  );
}

function App() {
  const [step, setStep] = useState(0);
  const [platform, setPlatform] = useState<string | null>(null);
  const [platformFolder, setPlatformFolder] = useState("");
  const [sourceMode, setSourceMode] = useState<SourceMode>("url");
  const [directUrl, setDirectUrl] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedResult, setSelectedResult] = useState<number | null>(null);
  const [previewMedia, setPreviewMedia] = useState<SearchResult | null>(null);
  const [format, setFormat] = useState<FormatId | null>(null);
  const [quality, setQuality] = useState<QualityId>("best");
  const [cookies, setCookies] = useState<CookieId>("none");
  const [cookieFile, setCookieFile] = useState("");
  const [wifiOnly, setWifiOnly] = useState(false);
  const [tools, setTools] = useState<ToolStatus | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [downloadPercent, setDownloadPercent] = useState(0);
  const [downloadLines, setDownloadLines] = useState<string[]>([]);
  const [downloadMessage, setDownloadMessage] = useState("");
  const [downloadError, setDownloadError] = useState("");
  const [mobileDownloadState, setMobileDownloadState] = useState<MobileDownloadState | null>(null);
  const [mobileHistory, setMobileHistory] = useState<MobileDownloadRecord[]>([]);
  const [mobileSettings, setMobileSettings] = useState<MobileSettings | null>(null);
  const [mobileActionBusy, setMobileActionBusy] = useState(false);
  const consoleRef = useRef<HTMLPreElement>(null);
  const downloadBufferRef = useRef<string[]>([]);
  const downloadPercentRef = useRef(0);
  const downloadFlushRef = useRef<number | null>(null);
  const searchRequestRef = useRef(0);
  const previewTriggerRef = useRef<HTMLButtonElement | null>(null);
  const gpuCanvasRef = useRef<HTMLCanvasElement>(null);
  const gpuControllerRef = useRef<BackdropController | null>(null);
  const [gpuRenderer, setGpuRenderer] = useState("GPU iniciando");

  const selectedFormat = formats.find((item) => item.id === format);
  const selectedQuality = qualities.find((item) => item.id === quality);
  const availableCookieOptions = isAndroidRuntime
    ? cookieOptions.filter((item) => item.id === "none" || item.id === "file")
    : cookieOptions;
  const selectedCookie = availableCookieOptions.find((item) => item.id === cookies);
  const directUrlPlaceholder =
    platforms.find((item) => item.id === platform)?.urlExample ?? "https://exemplo.com/video";
  const selectedMedia = selectedResult !== null ? results[selectedResult] ?? null : null;
  const activeUrl = sourceMode === "search" ? selectedMedia?.url || "" : directUrl;
  const searchAvailable = platform === "YouTube";
  const progress = (step / (steps.length - 1)) * 100;

  const validStep = useMemo(() => {
    if (step === 0) return Boolean(platformFolder.trim());
    if (step === 1) return Boolean(activeUrl.trim());
    if (step === 2) return Boolean(format);
    if (step === 3) return cookies !== "file" || Boolean(cookieFile.trim());
    return true;
  }, [step, platformFolder, activeUrl, format, cookies, cookieFile]);

  useEffect(() => {
    let active = true;
    const unlisteners: Array<() => void> = [];
    void checkTools();
    void (async () => {
      unlisteners.push(
        await listenDownloadOutput((payload) => {
          if (!active) return;
          const line = String(payload || "");
          downloadBufferRef.current.push(line);
          if (downloadBufferRef.current.length > 300) {
            downloadBufferRef.current = downloadBufferRef.current.slice(-300);
          }
          const match = line.match(/\[download\]\s+([0-9.]+)%/);
          if (match) downloadPercentRef.current = Math.min(100, Number(match[1]));
          if (downloadFlushRef.current === null) {
            downloadFlushRef.current = window.setTimeout(() => {
              setDownloadLines([...downloadBufferRef.current]);
              setDownloadPercent(downloadPercentRef.current);
              downloadFlushRef.current = null;
            }, 80);
          }
        }),
      );
      if (isAndroidRuntime) {
        unlisteners.push(
          await listenMobilePluginEvent<MobileDownloadRecord>("download-state", (record) => {
            if (!active) return;
            const isActive = ["queued", "running", "paused", "processing", "saving"].includes(record.status);
            setMobileDownloadState({
              active: isActive,
              paused: record.status === "paused",
              cancelled: record.status === "cancelled",
              current: record,
            });
            setDownloading(isActive);
            setDownloadPercent(record.percent);
            downloadPercentRef.current = record.percent;
            if (record.console?.length) {
              downloadBufferRef.current = record.console.slice(-300);
              setDownloadLines([...downloadBufferRef.current]);
            }
            if (record.status === "completed") {
              setDownloadMessage(`${record.message} Arquivo salvo em ${record.outputDir}`);
              setDownloadError("");
            } else if (record.status === "failed" || record.status === "cancelled") {
              setDownloadError(record.message);
            }
            setMobileHistory((current) => [
              record,
              ...current.filter((item) => item.id !== record.id),
            ].slice(0, 40));
          }),
        );
        unlisteners.push(
          await listenMobilePluginEvent<{ url?: string }>("shared-url", ({ url }) => {
            if (active && url) applySharedUrl(url);
          }),
        );
        void refreshMobileData();
      }
    })();
    return () => {
      active = false;
      if (downloadFlushRef.current !== null) window.clearTimeout(downloadFlushRef.current);
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    let active = true;
    if (gpuCanvasRef.current) {
      void startGpuBackdrop(gpuCanvasRef.current)
        .then((controller) => {
          if (!active) {
            controller.destroy();
            return;
          }
          gpuControllerRef.current = controller;
          setGpuRenderer(controller.renderer === "webgpu-wgsl" ? "WASM · WebGPU · WGSL" : "WASM · WebGL2 · GLSL");
        })
        .catch(() => setGpuRenderer("GPU fallback CSS"));
    }
    return () => {
      active = false;
      gpuControllerRef.current?.destroy();
      gpuControllerRef.current = null;
    };
  }, []);

  useEffect(() => {
    const normalized = Math.max(step / (steps.length - 1), downloadPercent / 100);
    gpuControllerRef.current?.setProgress(normalized);
  }, [step, downloadPercent]);

  useEffect(() => {
    gpuControllerRef.current?.setPaused(Boolean(previewMedia));
  }, [previewMedia]);

  useEffect(() => {
    if (consoleRef.current) consoleRef.current.scrollTop = consoleRef.current.scrollHeight;
  }, [downloadLines]);

  useEffect(() => {
    if (!isAndroidRuntime) return;
    window.history.replaceState({ ...window.history.state, deckStep: 0 }, "");
    const handlePopState = (event: PopStateEvent) => {
      const target = Number(event.state?.deckStep);
      if (Number.isInteger(target) && target >= 0 && target < steps.length) {
        setPreviewMedia(null);
        setStep(target);
      }
    };
    window.addEventListener("popstate", handlePopState);
    return () => window.removeEventListener("popstate", handlePopState);
  }, []);

  useEffect(() => {
    if (!isAndroidRuntime || window.history.state?.deckStep === step) return;
    window.history.pushState({ ...window.history.state, deckStep: step }, "");
  }, [step]);

  async function checkTools() {
    try {
      const status = await appInvoke<ToolStatus>("get_tool_status");
      setTools(status);
    } catch {
      setTools(null);
    }
  }

  function applySharedUrl(url: string) {
    let detected = "Outro";
    try {
      const host = new URL(url).hostname.toLowerCase();
      if (host.includes("youtube.com") || host === "youtu.be") detected = "YouTube";
      else if (host.includes("instagram.com")) detected = "Instagram";
      else if (host.includes("tiktok.com")) detected = "TikTok";
      else if (host === "x.com" || host.endsWith(".x.com") || host.includes("twitter.com")) detected = "X_Twitter";
      else if (host.includes("twitch.tv")) detected = "Twitch";
      else if (host.includes("reddit.com") || host === "redd.it") detected = "Reddit";
    } catch {
      return;
    }
    setPlatform(detected);
    setPlatformFolder(detected === "Outro" ? "Outro" : detected);
    setSourceMode("url");
    setDirectUrl(url);
    setSearchError("");
    setStep(1);
  }

  async function refreshMobileData() {
    if (!isAndroidRuntime) return;
    try {
      const [state, history, settings] = await Promise.all([
        appInvoke<MobileDownloadState>("get_download_state"),
        appInvoke<MobileDownloadHistory>("get_download_history"),
        appInvoke<MobileSettings>("get_mobile_settings"),
      ]);
      setMobileDownloadState(state);
      setMobileHistory(history.items || []);
      setMobileSettings(settings);
      if (settings.sharedUrl) applySharedUrl(settings.sharedUrl);
      if (state.current) {
        setDownloading(state.active);
        setDownloadPercent(state.current.percent);
        downloadPercentRef.current = state.current.percent;
        downloadBufferRef.current = (state.current.console || []).slice(-300);
        setDownloadLines([...downloadBufferRef.current]);
      }
    } catch (error) {
      setDownloadError(errorText(error));
    }
  }

  async function controlMobileDownload(action: "pause" | "resume" | "cancel") {
    if (!isAndroidRuntime || mobileActionBusy) return;
    setMobileActionBusy(true);
    try {
      const state = await appInvoke<MobileDownloadState>("control_download", { action });
      setMobileDownloadState(state);
      setDownloading(state.active);
    } catch (error) {
      setDownloadError(errorText(error));
    } finally {
      setMobileActionBusy(false);
    }
  }

  async function chooseMobileDownloadDirectory() {
    if (!isAndroidRuntime || mobileActionBusy) return;
    setMobileActionBusy(true);
    try {
      const settings = await appInvoke<MobileSettings>("choose_download_directory");
      setMobileSettings(settings);
    } catch (error) {
      setDownloadError(errorText(error));
    } finally {
      setMobileActionBusy(false);
    }
  }

  async function chooseMobileCookieFile() {
    if (!isAndroidRuntime || mobileActionBusy) return;
    setMobileActionBusy(true);
    try {
      const selected = await appInvoke<MobileCookieFile>("choose_cookie_file");
      setCookieFile(selected.path);
      setCookies("file");
    } catch (error) {
      setDownloadError(errorText(error));
    } finally {
      setMobileActionBusy(false);
    }
  }

  async function mobileHistoryAction(
    command: "open_download_item" | "share_download_item" | "delete_download_item",
    id: string,
  ) {
    if (!isAndroidRuntime || mobileActionBusy) return;
    setMobileActionBusy(true);
    try {
      await appInvoke(command, { id });
      if (command === "delete_download_item") {
        setMobileHistory((current) => current.filter((item) => item.id !== id));
      }
    } catch (error) {
      setDownloadError(errorText(error));
    } finally {
      setMobileActionBusy(false);
    }
  }

  async function clearMobileHistory() {
    if (!isAndroidRuntime || mobileActionBusy) return;
    setMobileActionBusy(true);
    try {
      await appInvoke("clear_download_history");
      setMobileHistory((current) =>
        current.filter((item) => ["queued", "running", "paused", "processing", "saving"].includes(item.status)),
      );
    } catch (error) {
      setDownloadError(errorText(error));
    } finally {
      setMobileActionBusy(false);
    }
  }

  function choosePlatform(id: string) {
    setPlatform(id);
    setPlatformFolder(id === "Outro" ? "" : id);
    if (id !== "YouTube") {
      searchRequestRef.current += 1;
      setSearching(false);
      setSourceMode("url");
      setPreviewMedia(null);
    }
  }

  async function pasteUrl() {
    try {
      const value = await navigator.clipboard.readText();
      if (value) {
        setDirectUrl(value.trim());
      }
    } catch (error) {
      setSearchError(`Não foi possível acessar a área de transferência: ${errorText(error)}`);
    }
  }

  async function searchVideos() {
    if (!searchAvailable || !searchQuery.trim()) return;
    const requestId = ++searchRequestRef.current;
    setSearching(true);
    setSearchError("");
    setResults([]);
    setSelectedResult(null);
    setPreviewMedia(null);
    try {
      const items = await appInvoke<SearchResult[]>("search_videos", { query: searchQuery.trim() });
      if (requestId !== searchRequestRef.current) return;
      setResults(items);
    } catch (error) {
      if (requestId !== searchRequestRef.current) return;
      setSearchError(errorText(error));
    } finally {
      if (requestId === searchRequestRef.current) setSearching(false);
    }
  }

  function chooseResult(item: SearchResult, index: number) {
    setSelectedResult(index);
  }

  function previewResult(item: SearchResult, index: number, trigger: HTMLButtonElement) {
    chooseResult(item, index);
    previewTriggerRef.current = trigger;
    setPreviewMedia(item);
  }

  const closePreview = useCallback(() => {
    setPreviewMedia(null);
  }, []);

  async function openPreviewInBrowser() {
    if (!previewMedia) return;
    try {
      await appInvoke("open_external_url", { url: previewMedia.url });
    } catch (error) {
      setSearchError(`Não foi possível abrir o navegador: ${errorText(error)}`);
    }
  }

  async function startDownload() {
    if (!format || downloading) return;
    if (isAndroidRuntime) {
      setMobileActionBusy(true);
      try {
        const settings = await appInvoke<MobileSettings>("request_mobile_permissions");
        setMobileSettings(settings);
        if (!settings.storageGranted) {
          throw new Error("Permita o acesso ao armazenamento para salvar o download.");
        }
      } catch (error) {
        setDownloadError(errorText(error));
        setMobileActionBusy(false);
        return;
      }
      setMobileActionBusy(false);
    }
    setDownloading(true);
    setDownloadPercent(0);
    setDownloadLines([]);
    downloadBufferRef.current = [];
    downloadPercentRef.current = 0;
    setDownloadMessage("");
    setDownloadError("");
    const request: DownloadRequest = {
      url: activeUrl.trim(),
      platformFolder: platformFolder.trim(),
      format,
      quality,
      cookies,
      cookieFile: cookieFile.trim() || null,
      wifiOnly: isAndroidRuntime && wifiOnly,
    };
    try {
      const result = await appInvoke<DownloadResult>("start_download", { request });
      setDownloadPercent(100);
      downloadPercentRef.current = 100;
      setDownloadMessage(`${result.message} Arquivos salvos em ${result.outputDir}`);
    } catch (error) {
      const message = errorText(error);
      setDownloadError(message);
      setDownloadLines((current) => [...current, `ERRO: ${message}`]);
    } finally {
      setDownloading(false);
    }
  }

  async function openFolder() {
    try {
      await appInvoke("open_downloads_folder", { platformFolder: platformFolder || null });
    } catch (error) {
      setDownloadError(errorText(error));
    }
  }

  function goNext() {
    if (validStep) setStep((current) => Math.min(steps.length - 1, current + 1));
  }

  function commandPreview() {
    if (!format) return "";
    const parts = [
      "yt-dlp",
      `"${activeUrl}"`,
      isAndroidRuntime
        ? `-o "Downloads/YT-DLP Deck/${platformFolder}/%(title)s [%(id)s].%(ext)s"`
        : `-o "Downloads\\YT-DLP Deck\\${platformFolder}\\%(uploader)s - %(title)s [%(id)s].%(ext)s"`,
      "--no-playlist",
      "--add-metadata",
      "--concurrent-fragments 4",
    ];
    if (cookies === "file" && cookieFile) parts.push(`--cookies "${cookieFile}"`);
    else if (cookies !== "none") parts.push(`--cookies-from-browser ${cookies}`);
    if (isAudio(format)) parts.push(`-x --audio-format ${format} --audio-quality 0`);
    else parts.push(`-f "${quality}" --merge-output-format ${format === "best" ? "mkv" : format}`);
    return parts.join(" \\\n  ");
  }

  const pageMotion = {
    initial: { opacity: 0, y: 24 },
    animate: { opacity: 1, y: 0 },
    exit: { opacity: 0, y: -16 },
    transition,
  };

  function renderPlatform() {
    return (
      <>
        <PageHeader
          eyebrow="Passo 01 · organização"
          title={<>Primeiro, escolha o seu <em>destino.</em></>}
          description="Cada plataforma recebe uma pasta própria. Assim sua biblioteca continua organizada mesmo depois de centenas de downloads."
          icon={Globe2}
        />
        <div className="card-grid platform-grid">
          {platforms.map((item) => (
            <ChoiceCard
              key={item.id}
              active={platform === item.id}
              icon={item.icon}
              color={item.color}
              title={item.label}
              description={item.description}
              onClick={() => choosePlatform(item.id)}
            />
          ))}
        </div>
        <AnimatePresence>
          {platform === "Outro" && (
            <motion.div
              className="field-panel custom-folder"
              initial={{ opacity: 0, height: 0, y: -10 }}
              animate={{ opacity: 1, height: "auto", y: 0 }}
              exit={{ opacity: 0, height: 0 }}
            >
              <label htmlFor="platform-folder">
                <FolderOpen size={15} /> Nome da pasta
              </label>
              <input
                id="platform-folder"
                value={platformFolder}
                onChange={(event) => setPlatformFolder(event.target.value)}
                placeholder="Ex.: Vimeo, Dailymotion ou Cursos"
                autoFocus
              />
            </motion.div>
          )}
        </AnimatePresence>
      </>
    );
  }

  function renderSource() {
    return (
      <>
        <PageHeader
          eyebrow="Passo 02 · conteúdo"
          title={<>Encontre exatamente o que você <em>quer baixar.</em></>}
          description="Cole um link direto ou pesquise pelo nome. Na pesquisa, cada resultado aparece com sua capa para você escolher com segurança."
          icon={WandSparkles}
        />
        <div className="source-tabs" role="tablist" aria-label="Forma de escolher o conteúdo">
          <button
            type="button"
            role="tab"
            aria-selected={sourceMode === "url"}
            aria-controls="source-url-panel"
            className={sourceMode === "url" ? "is-active" : ""}
            onClick={() => setSourceMode("url")}
          >
            <Link2 size={16} /> Link direto
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={sourceMode === "search"}
            aria-controls="source-search-panel"
            className={sourceMode === "search" ? "is-active" : ""}
            onClick={() => setSourceMode("search")}
            disabled={!searchAvailable}
            title={searchAvailable ? "Pesquisar no YouTube" : "A pesquisa por nome está disponível para YouTube"}
          >
            <Search size={16} /> Pesquisar vídeo
          </button>
          <motion.span
            className="tab-indicator"
            animate={{ x: sourceMode === "url" ? 0 : "100%" }}
            transition={transition}
          />
        </div>
        {!searchAvailable && (
          <div className="source-search-note" role="note">
            <Youtube size={15} />
            A pesquisa por nome usa o YouTube. Para {platformFolder || "esta plataforma"}, use um link direto.
          </div>
        )}
        <AnimatePresence mode="wait">
          {sourceMode === "url" ? (
            <motion.div
              id="source-url-panel"
              role="tabpanel"
              key="url"
              {...pageMotion}
              className="source-area"
            >
              <div className="field-panel hero-input">
                <label htmlFor="direct-media-url">
                  <Link2 size={15} /> URL da mídia
                </label>
                <div className="input-actions">
                  <input
                    id="direct-media-url"
                    type="url"
                    inputMode="url"
                    value={directUrl}
                    onChange={(event) => setDirectUrl(event.target.value)}
                    onKeyDown={(event) => event.key === "Enter" && directUrl.trim() && goNext()}
                    placeholder={directUrlPlaceholder}
                  />
                  <button className="soft-button" onClick={pasteUrl} title="Colar URL">
                    <Clipboard size={17} /> Colar
                  </button>
                </div>
              </div>
            </motion.div>
          ) : (
            <motion.div
              id="source-search-panel"
              role="tabpanel"
              key="search"
              {...pageMotion}
              className="source-area"
            >
              <div className="field-panel hero-input">
                <label htmlFor="video-search-query">
                  <Search size={15} /> Pesquisa inteligente
                </label>
                <div className="input-actions">
                  <input
                    id="video-search-query"
                    type="search"
                    value={searchQuery}
                    onChange={(event) => setSearchQuery(event.target.value)}
                    onKeyDown={(event) => event.key === "Enter" && void searchVideos()}
                    placeholder="Música, tutorial, podcast, artista..."
                  />
                  <button
                    className="primary-button"
                    onClick={searchVideos}
                    disabled={!searchQuery.trim() || searching}
                  >
                    {searching ? <LoaderCircle className="spin" size={18} /> : <Search size={18} />}
                    {searching ? "Buscando" : "Buscar"}
                  </button>
                </div>
              </div>
              {searchError && <InlineError message={searchError} />}
              <div className="sr-only" aria-live="polite" aria-atomic="true">
                {searching
                  ? "Pesquisando vídeos no YouTube."
                  : results.length
                    ? `${results.length} vídeos encontrados.`
                    : searchError
                      ? `Erro na pesquisa: ${searchError}`
                      : ""}
              </div>
              {searching && (
                <div className="skeleton-grid">
                  {[0, 1, 2].map((item) => (
                    <div className="media-skeleton" key={item}>
                      <span />
                      <i />
                      <i />
                    </div>
                  ))}
                </div>
              )}
              {!searching && !results.length && !searchError && (
                <div className="empty-state">
                  <span className="empty-orbit">
                    <Search size={26} />
                  </span>
                  <strong>As capas aparecerão aqui</strong>
                  <small>Mostraremos até cinco resultados para você comparar.</small>
                </div>
              )}
              <motion.div className="media-grid">
                {results.map((item, index) => (
                  <MediaCard
                    key={`${item.id}-${index}`}
                    media={item}
                    selected={selectedResult === index}
                    onClick={() => chooseResult(item, index)}
                    onPreview={(trigger) => previewResult(item, index, trigger)}
                  />
                ))}
              </motion.div>
            </motion.div>
          )}
        </AnimatePresence>
      </>
    );
  }

  function renderFormats() {
    return (
      <>
        <PageHeader
          eyebrow="Passo 03 · formato"
          title={<>Escolha como a mídia deve <em>chegar até você.</em></>}
          description="Vídeo completo ou somente áudio. O FFmpeg cuida da conversão e da melhor combinação de faixas."
          icon={Film}
        />
        <div className="format-sections">
          {(["Vídeo", "Áudio"] as const).map((category) => (
            <section key={category}>
              <div className="section-label">
                {category === "Vídeo" ? <MonitorPlay size={15} /> : <AudioLines size={15} />}
                {category}
              </div>
              <div className="card-grid format-grid">
                {formats
                  .filter((item) => item.category === category)
                  .map((item) => (
                    <ChoiceCard
                      key={item.id}
                      active={format === item.id}
                      icon={item.icon}
                      color={item.color}
                      title={item.label}
                      description={item.description}
                      tag={item.category}
                      onClick={() => setFormat(item.id)}
                    />
                  ))}
              </div>
            </section>
          ))}
        </div>
      </>
    );
  }

  function renderSettings() {
    const audio = format ? isAudio(format) : false;
    return (
      <>
        <PageHeader
          eyebrow="Passo 04 · controle"
          title={<>Ajuste qualidade e <em>acesso.</em></>}
          description={
            isAndroidRuntime
              ? "Escolha a resolução. No Android, o yt-dlp e o FFmpeg já estão incorporados ao aplicativo."
              : "Escolha a resolução desejada. Para conteúdo restrito, use uma sessão do navegador ou um arquivo cookies.txt."
          }
          icon={Settings2}
        />
        <div className="settings-layout">
          <section className={`settings-panel ${audio ? "is-disabled" : ""}`}>
            <div className="panel-heading">
              <span>
                <Gauge size={19} />
              </span>
              <div>
                <strong>Qualidade do vídeo</strong>
                <small>{audio ? "Não se aplica ao formato de áudio" : "Com fallback automático"}</small>
              </div>
            </div>
            {audio ? (
              <div className="audio-quality-message">
                <AudioLines size={28} />
                <strong>Qualidade máxima de áudio</strong>
                <small>A conversão usará a melhor faixa disponível.</small>
              </div>
            ) : (
              <div className="quality-list">
                {qualities.map((item) => (
                  <button
                    key={item.id}
                    className={quality === item.id ? "is-active" : ""}
                    onClick={() => setQuality(item.id)}
                  >
                    <span className="radio-dot" />
                    <span>
                      <strong>{item.label}</strong>
                      <small>{item.detail}</small>
                    </span>
                    {quality === item.id && <Check size={15} />}
                  </button>
                ))}
              </div>
            )}
          </section>
          <section className="settings-panel">
            <div className="panel-heading">
              <span>
                <Cookie size={19} />
              </span>
              <div>
                <strong>Cookies e autenticação</strong>
                <small>Para login, idade ou conteúdo privado</small>
              </div>
            </div>
            <div className="cookie-grid">
              {availableCookieOptions.map((item) => {
                const Icon = item.icon;
                return (
                  <button
                    key={item.id}
                    className={cookies === item.id ? "is-active" : ""}
                    onClick={() => setCookies(item.id)}
                  >
                    <Icon size={18} />
                    <span>
                      <strong>{item.label}</strong>
                      <small>{item.detail}</small>
                    </span>
                    {cookies === item.id && <CheckCircle2 size={16} />}
                  </button>
                );
              })}
            </div>
            <AnimatePresence>
              {cookies === "file" && (
                <motion.div
                  className="cookie-file"
                  initial={{ opacity: 0, height: 0 }}
                  animate={{ opacity: 1, height: "auto" }}
                  exit={{ opacity: 0, height: 0 }}
                >
                  <label htmlFor="cookie-file-path">
                    <FileText size={14} /> {isAndroidRuntime ? "Arquivo cookies.txt" : "Caminho do cookies.txt"}
                  </label>
                  {isAndroidRuntime ? (
                    <button
                      id="cookie-file-path"
                      type="button"
                      className="mobile-file-picker"
                      onClick={chooseMobileCookieFile}
                      disabled={mobileActionBusy}
                    >
                      <FileText size={16} />
                      {cookieFile ? "cookies.txt importado · trocar arquivo" : "Selecionar cookies.txt"}
                    </button>
                  ) : (
                    <input
                      id="cookie-file-path"
                      value={cookieFile}
                      onChange={(event) => setCookieFile(event.target.value)}
                      placeholder="C:\caminho\cookies.txt"
                    />
                  )}
                </motion.div>
              )}
            </AnimatePresence>
          </section>
          {isAndroidRuntime && (
            <section className="settings-panel mobile-network-panel">
              <div className="panel-heading">
                <span>
                  <Wifi size={19} />
                </span>
                <div>
                  <strong>Rede e bateria</strong>
                  <small>Controle o consumo de dados móveis</small>
                </div>
              </div>
              <button
                type="button"
                className={`mobile-setting-toggle ${wifiOnly ? "is-active" : ""}`}
                role="switch"
                aria-checked={wifiOnly}
                onClick={() => setWifiOnly((current) => !current)}
              >
                <span>
                  <Wifi size={17} />
                  <span>
                    <strong>Baixar somente por Wi-Fi</strong>
                    <small>Bloqueia o início em redes móveis</small>
                  </span>
                </span>
                <i />
              </button>
            </section>
          )}
        </div>
      </>
    );
  }

  function renderReview() {
    const chosenMedia = sourceMode === "search" ? selectedMedia : null;
    return (
      <>
        <PageHeader
          eyebrow="Passo 05 · download"
          title={<>Seu download está <em>pronto para decolar.</em></>}
          description="Revise tudo e acompanhe o progresso sem sair do aplicativo."
          icon={HardDriveDownload}
        />
        <div className="review-grid">
          <section className="review-panel">
            {chosenMedia && (
              <MediaCard
                media={chosenMedia}
                selected
                large
                onPreview={(trigger) => {
                  previewTriggerRef.current = trigger;
                  setPreviewMedia(chosenMedia);
                }}
              />
            )}
            <div className="review-list">
              <ReviewRow icon={FolderOpen} label="Destino" value={platformFolder} />
              <ReviewRow
                icon={Film}
                label="Formato"
                value={`${selectedFormat?.label || "—"} · ${
                  isAudio(format) ? "Melhor faixa de áudio" : selectedQuality?.label || "—"
                }`}
              />
              <ReviewRow icon={Cookie} label="Acesso" value={selectedCookie?.label || "—"} />
              {!chosenMedia && <ReviewRow icon={Link2} label="URL" value={activeUrl} />}
            </div>
          </section>
          <section className="terminal-panel">
            <div className="terminal-head">
              <span>
                <i className="terminal-dot red" />
                <i className="terminal-dot yellow" />
                <i className="terminal-dot green" />
              </span>
              <span>
                <SquareTerminal size={14} /> comando seguro
              </span>
            </div>
            <pre>{commandPreview()}</pre>
          </section>
        </div>
        <section className="download-station">
          <div className="download-actions">
            <button
              className="download-button"
              onClick={startDownload}
              disabled={downloading}
            >
              <span className="button-shine" />
              {downloading ? <LoaderCircle className="spin" /> : <Download />}
              <span>
                <strong>{downloading ? "Baixando mídia…" : "Iniciar download"}</strong>
                <small>{downloading ? "Acompanhe o progresso abaixo" : "Executar com yt-dlp + FFmpeg"}</small>
              </span>
            </button>
            <button className="folder-button" onClick={openFolder}>
              <FolderOpen size={19} /> Abrir pasta
            </button>
            {isAndroidRuntime && (
              <button
                className="folder-button"
                onClick={chooseMobileDownloadDirectory}
                disabled={mobileActionBusy || downloading}
                title={mobileSettings?.downloadDirectory || "Escolher pasta de destino"}
              >
                <Settings2 size={18} /> Escolher pasta
              </button>
            )}
            {isAndroidRuntime && downloading && (
              <>
                <button
                  className="folder-button mobile-control-button"
                  onClick={() =>
                    controlMobileDownload(mobileDownloadState?.paused ? "resume" : "pause")
                  }
                  disabled={mobileActionBusy}
                >
                  {mobileDownloadState?.paused ? <Play size={18} /> : <Pause size={18} />}
                  {mobileDownloadState?.paused ? "Retomar" : "Pausar"}
                </button>
                <button
                  className="folder-button mobile-control-button is-danger"
                  onClick={() => controlMobileDownload("cancel")}
                  disabled={mobileActionBusy}
                >
                  <X size={18} /> Cancelar
                </button>
              </>
            )}
          </div>
          {(downloading || downloadLines.length > 0 || downloadMessage || downloadError) && (
            <motion.div
              className="download-progress"
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
            >
              <div className="progress-caption">
                <span>
                  <Activity size={14} /> {downloading ? "Transferindo e processando" : "Processo finalizado"}
                </span>
                <strong>{Math.round(downloadPercent)}%</strong>
              </div>
              <div
                className="progress-track"
                role="progressbar"
                aria-label="Progresso do download"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={Math.round(downloadPercent)}
              >
                <motion.i animate={{ width: `${downloadPercent}%` }} transition={{ duration: 0.35 }} />
              </div>
              {downloadMessage && (
                <div className="success-message" role="status" aria-live="polite">
                  <CheckCircle2 size={17} /> {downloadMessage}
                </div>
              )}
              {downloadError && <InlineError message={downloadError} />}
              {downloadLines.length > 0 && (
                <div className="live-console-shell">
                  <div className="live-console-head">
                    <span><SquareTerminal size={14} /> console yt-dlp + FFmpeg</span>
                    <small>{isAndroidRuntime ? "Android nativo" : "Windows nativo"}</small>
                  </div>
                  <pre className="live-console" ref={consoleRef}>
                    {downloadLines.join("\n")}
                  </pre>
                </div>
              )}
            </motion.div>
          )}
        </section>
        {isAndroidRuntime && (
          <section className="mobile-download-library">
            <div className="mobile-library-head">
              <div>
                <span>Biblioteca Android</span>
                <small>Histórico persistente de downloads</small>
              </div>
              {mobileHistory.length > 0 && (
                <button onClick={clearMobileHistory} disabled={mobileActionBusy}>
                  <Trash2 size={15} /> Limpar finalizados
                </button>
              )}
            </div>
            {mobileHistory.length === 0 ? (
              <div className="mobile-library-empty">
                Seus downloads concluídos aparecerão aqui.
              </div>
            ) : (
              <div className="mobile-history-list">
                {mobileHistory.slice(0, 12).map((item) => (
                  <article className={`mobile-history-item status-${item.status}`} key={item.id}>
                    <div className="mobile-history-main">
                      <strong>{item.title || item.fileName || "Download"}</strong>
                      <span>{mobileStatusLabels[item.status] || item.status} · {Math.round(item.percent)}%</span>
                      <small>{item.message}</small>
                    </div>
                    <div className="mobile-history-actions">
                      {item.status === "completed" && item.fileUri && (
                        <>
                          <button onClick={() => mobileHistoryAction("open_download_item", item.id)}>
                            <Play size={14} /> Abrir
                          </button>
                          <button onClick={() => mobileHistoryAction("share_download_item", item.id)}>
                            <Share2 size={14} /> Compartilhar
                          </button>
                        </>
                      )}
                      {!["queued", "running", "paused", "processing", "saving"].includes(item.status) && (
                        <button
                          className="is-danger"
                          onClick={() => mobileHistoryAction("delete_download_item", item.id)}
                        >
                          <Trash2 size={14} /> Excluir
                        </button>
                      )}
                    </div>
                  </article>
                ))}
              </div>
            )}
          </section>
        )}
      </>
    );
  }

  const pages = [renderPlatform, renderSource, renderFormats, renderSettings, renderReview];
  const CurrentStepIcon = steps[step].icon;

  return (
    <MotionConfig reducedMotion="user">
    <div className="app-shell">
      <canvas className="gpu-backdrop" ref={gpuCanvasRef} aria-hidden="true" />
      <div className="ambient ambient-one" />
      <div className="ambient ambient-two" />
      <div className="noise" />

      <header className="topbar">
        <div className="brand">
          <span className="brand-logo">
            <Download size={22} />
            <i />
          </span>
          <div>
            <strong>YT-DLP <em>DECK</em></strong>
            <small>Media acquisition studio</small>
          </div>
        </div>
        <div className="top-center">
          <CurrentStepIcon size={14} />
          <span>{steps[step].title}</span>
          <i />
          <span>{step + 1} de {steps.length}</span>
        </div>
        <div className="tools">
          <ToolBadge label="yt-dlp" ready={Boolean(tools?.ytDlp)} version={tools?.ytDlpVersion} />
          <ToolBadge label="FFmpeg" ready={Boolean(tools?.ffmpeg)} version={tools?.ffmpegVersion} />
        </div>
      </header>

      <aside className="sidebar">
        <div className="sidebar-title">Fluxo de download</div>
        <nav>
          {steps.map((item, index) => {
            const Icon = item.icon;
            const done = index < step;
            return (
              <button
                key={item.title}
                className={`${index === step ? "is-active" : ""} ${done ? "is-done" : ""}`}
                disabled={index > step}
                onClick={() => index <= step && setStep(index)}
              >
                <span className="step-icon">
                  {done ? <Check size={16} /> : <Icon size={17} />}
                </span>
                <span>
                  <strong>{item.title}</strong>
                  <small>{item.subtitle}</small>
                </span>
                {index === step && <motion.i layoutId="active-step" transition={transition} />}
              </button>
            );
          })}
        </nav>
        <div className="sidebar-progress">
          <div>
            <span>Progresso</span>
            <strong>{Math.round(progress)}%</strong>
          </div>
          <div
            className="progress-track"
            role="progressbar"
            aria-label="Progresso das etapas"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(progress)}
          >
            <motion.i animate={{ width: `${progress}%` }} />
          </div>
          <small>Suas escolhas são mantidas entre as etapas.</small>
          <small className="renderer-label">{gpuRenderer}</small>
        </div>
      </aside>

      <main className="main-stage">
        <AnimatePresence>
          {tools && (!tools.ytDlp || !tools.ffmpeg) && (
            <motion.div
              className="tool-check-banner"
              initial={{ opacity: 0, y: -16 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -12 }}
            >
              <AlertCircle size={19} />
              <div>
                <strong>Ferramentas não encontradas nesta instalação</strong>
                <span>
                  Coloque <b>yt-dlp.exe</b> e <b>ffmpeg.exe</b> na mesma pasta do aplicativo.
                </span>
              </div>
              <button onClick={checkTools}>
                <RotateCcw size={15} /> Checar novamente
              </button>
            </motion.div>
          )}
        </AnimatePresence>
        <AnimatePresence mode="wait">
          <motion.div className="page" key={step} {...pageMotion}>
            {pages[step]()}
            {step < steps.length - 1 && (
              <div className="transport">
                <button
                  className="back-button"
                  onClick={() => setStep((current) => Math.max(0, current - 1))}
                  disabled={step === 0}
                >
                  <ArrowLeft size={17} /> Voltar
                </button>
                <div className="transport-hint">
                  {validStep ? (
                    <>
                      <CheckCircle2 size={15} /> Etapa pronta
                    </>
                  ) : (
                    <>
                      <AlertCircle size={15} /> Complete a seleção
                    </>
                  )}
                </div>
                <button className="next-button" onClick={goNext} disabled={!validStep}>
                  Continuar <ArrowRight size={17} />
                </button>
              </div>
            )}
          </motion.div>
        </AnimatePresence>
      </main>
      <AnimatePresence onExitComplete={() => previewTriggerRef.current?.focus()}>
        {previewMedia && (
          <VideoPreview
            key={previewMedia.id}
            media={previewMedia}
            onClose={closePreview}
            onOpenExternal={openPreviewInBrowser}
          />
        )}
      </AnimatePresence>
    </div>
    </MotionConfig>
  );
}

function PageHeader({
  eyebrow,
  title,
  description,
  icon: Icon,
}: {
  eyebrow: string;
  title: React.ReactNode;
  description: string;
  icon: LucideIcon;
}) {
  return (
    <header className="page-header">
      <div className="eyebrow">
        <span>
          <Icon size={14} />
        </span>
        {eyebrow}
      </div>
      <h1>{title}</h1>
      <p>{description}</p>
    </header>
  );
}

function InlineError({ message }: { message: string }) {
  return (
    <motion.div
      className="inline-error"
      role="alert"
      initial={{ opacity: 0, y: -8 }}
      animate={{ opacity: 1, y: 0 }}
    >
      <AlertCircle size={16} />
      <span>{message}</span>
    </motion.div>
  );
}

function ReviewRow({
  icon: Icon,
  label,
  value,
}: {
  icon: LucideIcon;
  label: string;
  value: string;
}) {
  return (
    <div className="review-row">
      <span>
        <Icon size={16} />
      </span>
      <div>
        <small>{label}</small>
        <strong>{value}</strong>
      </div>
    </div>
  );
}

export default App;
