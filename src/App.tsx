import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, MotionConfig, motion } from "framer-motion";
import {
  Activity,
  AlertCircle,
  ArrowLeft,
  ArrowRight,
  AudioLines,
  Check,
  CheckCheck,
  CheckCircle2,
  CirclePause,
  CirclePlay,
  Clipboard,
  Copy,
  Cookie,
  Download,
  FileText,
  Film,
  FolderOpen,
  Gauge,
  Globe2,
  HardDriveDownload,
  History,
  ImageDown,
  Instagram,
  Link2,
  LoaderCircle,
  MonitorPlay,
  Pause,
  Play,
  RotateCcw,
  Search,
  Settings2,
  Share2,
  SquareTerminal,
  Subtitles,
  Trash2,
  TriangleAlert,
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
  MobileClipboard,
  MobileCookieFile,
  MobileDownloadHistory,
  MobileDownloadRecord,
  MobileDownloadState,
  MobileSettings,
  QualityId,
  SearchResult,
  SourceMode,
  ToolStatus,
  ToolUpdateResult,
} from "./types";
import {
  appInvoke,
  isAndroidRuntime,
  listenDownloadOutput,
  listenMobilePluginEvent,
} from "./nativeBridge";
import { startGpuBackdrop, type BackdropController } from "./visuals/gpuBackdrop";

import { transition } from "./config/motion";
import { steps, platforms, formats, qualities, mobileStatusLabels, cookieOptions } from "./config/downloadOptions";
import { errorText, isAudio, isImage, supportsImageCover } from "./features/downloads/formatUtils";
import { TOOL_UPDATE_CACHE_MS, type ToolId, type ToastTone, type ToastItem, type DesktopHistoryRecord, type HistoryRecord, type DownloadVisualStatus } from "./features/downloads/uiTypes";
import { getConsoleLineTone, readDownloadTelemetry } from "./features/downloads/consoleUtils";
import { groupHistory } from "./features/history/historyUtils";
import { ToastViewport } from "./components/ToastViewport";
import { ChoiceCard } from "./components/ChoiceCard";
import { PageHeader, InlineError, ReviewRow } from "./components/PagePrimitives";
import { ToolBadge } from "./features/tools/ToolBadge";
import { MediaCard } from "./features/search/MediaCard";
import { VideoPreview } from "./features/search/VideoPreview";

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
  const [downloadSubtitles, setDownloadSubtitles] = useState(false);
  const [quality, setQuality] = useState<QualityId>("best");
  const [cookies, setCookies] = useState<CookieId>("none");
  const [cookieFile, setCookieFile] = useState("");
  const [wifiOnly, setWifiOnly] = useState(false);
  const [tools, setTools] = useState<ToolStatus | null>(null);
  const [checkingTools, setCheckingTools] = useState(true);
  const [updatingTools, setUpdatingTools] = useState<ReadonlySet<ToolId>>(() => new Set());
  const [recentlyUpdatedTools, setRecentlyUpdatedTools] = useState<ReadonlySet<ToolId>>(
    () => new Set(),
  );
  const [downloading, setDownloading] = useState(false);
  const [downloadPercent, setDownloadPercent] = useState(0);
  const [downloadLines, setDownloadLines] = useState<string[]>([]);
  const [downloadMessage, setDownloadMessage] = useState("");
  const [downloadError, setDownloadError] = useState("");
  const [consoleAutoScroll, setConsoleAutoScroll] = useState(true);
  const [consoleCopied, setConsoleCopied] = useState(false);
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const [desktopHistory, setDesktopHistory] = useState<DesktopHistoryRecord[]>(() => {
    if (isAndroidRuntime) return [];
    try {
      const stored = JSON.parse(window.localStorage.getItem("yt-dlp-deck-history") || "[]");
      return Array.isArray(stored) ? stored.slice(0, 40) : [];
    } catch {
      return [];
    }
  });
  const [mobileDownloadState, setMobileDownloadState] = useState<MobileDownloadState | null>(null);
  const [mobileHistory, setMobileHistory] = useState<MobileDownloadRecord[]>([]);
  const [mobileSettings, setMobileSettings] = useState<MobileSettings | null>(null);
  const [mobileActionBusy, setMobileActionBusy] = useState(false);
  const consoleRef = useRef<HTMLDivElement>(null);
  const downloadBufferRef = useRef<string[]>([]);
  const downloadPercentRef = useRef(0);
  const downloadFlushRef = useRef<number | null>(null);
  const searchRequestRef = useRef(0);
  const searchInFlightRef = useRef(false);
  const downloadInFlightRef = useRef(false);
  const previewTriggerRef = useRef<HTMLButtonElement | null>(null);
  const gpuCanvasRef = useRef<HTMLCanvasElement>(null);
  const gpuControllerRef = useRef<BackdropController | null>(null);
  const toastIdRef = useRef(0);
  const toastTimersRef = useRef<number[]>([]);
  const updateInFlightRef = useRef<Set<ToolId>>(new Set());
  const toolUpdateCacheRef = useRef(
    new Map<ToolId, { checkedAt: number; result: ToolUpdateResult }>(),
  );
  const [gpuRenderer, setGpuRenderer] = useState("GPU iniciando");

  const pushToast = useCallback((tone: ToastTone, title: string, message: string) => {
    const id = ++toastIdRef.current;
    setToasts((current) => [...current.slice(-2), { id, tone, title, message }]);
    const timer = window.setTimeout(() => {
      setToasts((current) => current.filter((item) => item.id !== id));
    }, 4_200);
    toastTimersRef.current.push(timer);
  }, []);

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
  const spotify = platform === "Spotify";
  const searchAvailable = platform === "YouTube" || spotify;
  const progress = (step / (steps.length - 1)) * 100;
  const mobileCurrentStatus = mobileDownloadState?.current?.status;
  const mobilePaused = isAndroidRuntime && (
    mobileDownloadState?.paused || mobileCurrentStatus === "paused"
  );
  const mobileCanPause = mobilePaused || mobileCurrentStatus === "queued" || mobileCurrentStatus === "running";
  const downloadStageLabel = mobilePaused
    ? "Download pausado"
    : mobileCurrentStatus === "processing"
      ? isImage(format)
        ? "Preparando imagem"
        : "Processando com FFmpeg"
      : mobileCurrentStatus === "saving"
        ? "Salvando arquivo"
        : downloading
          ? "Transferindo mídia"
          : "Processo finalizado";

  const downloadTelemetry = useMemo(() => readDownloadTelemetry(downloadLines), [downloadLines]);
  const downloadVisualStatus: DownloadVisualStatus = downloadError
    ? "failed"
    : downloadMessage
      ? "completed"
      : mobileCurrentStatus || (downloading ? "running" : "idle");
  const showDownloadActivity =
    downloading || downloadLines.length > 0 || Boolean(downloadMessage) || Boolean(downloadError);
  const downloadTitle =
    selectedMedia?.title ||
    (directUrl.trim() ? `${platformFolder || "Mídia"} · link direto` : "Nova transferência");
  const historyItems = useMemo<HistoryRecord[]>(
    () => isAndroidRuntime ? mobileHistory : desktopHistory,
    [mobileHistory, desktopHistory],
  );
  const historyGroups = useMemo(() => groupHistory(historyItems), [historyItems]);

  const validStep = useMemo(() => {
    if (step === 0) return Boolean(platformFolder.trim());
    if (step === 1) return Boolean(activeUrl.trim()) && (!spotify || sourceMode === "search");
    if (step === 2) return Boolean(format);
    if (step === 3) return cookies !== "file" || Boolean(cookieFile.trim());
    return true;
  }, [step, platformFolder, activeUrl, format, cookies, cookieFile, spotify, sourceMode]);

  function applyMobileRecord(record: MobileDownloadRecord) {
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
    if ((record.console || []).length > 0) {
      downloadBufferRef.current = record.console.slice(-300);
      setDownloadLines([...downloadBufferRef.current]);
    }
    if (record.status === "completed") {
      setDownloadMessage(`${record.message} Arquivo salvo em ${record.outputDir}`);
      setDownloadError("");
    } else if (record.status === "failed" || record.status === "cancelled") {
      setDownloadError(record.message);
    }
    setMobileHistory((current) => {
      const existing = current.find((item) => item.id === record.id);
      const merged = record.console.length > 0
        ? record
        : { ...record, console: existing?.console || [] };
      return [merged, ...current.filter((item) => item.id !== record.id)].slice(0, 40);
    });
  }

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
            applyMobileRecord(record);
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
    if (!isAndroidRuntime || !downloading) return;
    let active = true;
    let polling = false;
    const pollState = async () => {
      if (!active || polling) return;
      polling = true;
      try {
        const state = await appInvoke<MobileDownloadState>("get_download_state");
        if (active && state.current) applyMobileRecord(state.current);
      } catch {
        // The event listener remains the primary path; polling repairs WebView event delays.
      } finally {
        polling = false;
      }
    };
    void pollState();
    const timer = window.setInterval(() => void pollState(), 1_500);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [downloading]);

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
    if (consoleAutoScroll && consoleRef.current) {
      consoleRef.current.scrollTo({ top: consoleRef.current.scrollHeight, behavior: "smooth" });
    }
  }, [downloadLines, consoleAutoScroll]);

  useEffect(() => {
    if (!isAndroidRuntime) {
      window.localStorage.setItem("yt-dlp-deck-history", JSON.stringify(desktopHistory.slice(0, 40)));
    }
  }, [desktopHistory]);

  useEffect(() => () => {
    toastTimersRef.current.forEach((timer) => window.clearTimeout(timer));
  }, []);

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

  async function checkTools(announce = false) {
    if (announce) toolUpdateCacheRef.current.clear();
    setCheckingTools(true);
    try {
      const status = await appInvoke<ToolStatus>("get_tool_status");
      setTools(status);
      if (announce) {
        const readyCount = Number(status.ytDlp) + Number(status.ffmpeg);
        pushToast(
          readyCount === 2 ? "success" : "info",
          readyCount === 2 ? "Ferramentas prontas" : "Verificação concluída",
          readyCount === 2
            ? "yt-dlp e FFmpeg foram encontrados nesta instalação."
            : `${readyCount} de 2 ferramentas foram encontradas.`,
        );
      }
    } catch (error) {
      setTools(null);
      if (announce) {
        pushToast("error", "Falha na verificação", errorText(error));
      }
    } finally {
      setCheckingTools(false);
    }
  }

  async function updateTool(tool: ToolId) {
    const label = tool === "yt-dlp" ? "yt-dlp" : "FFmpeg";
    if (isAndroidRuntime) {
      pushToast(
        "info",
        `${label} incorporado`,
        `No Android, o ${label} é atualizado junto com uma nova versão do aplicativo.`,
      );
      return;
    }
    if (updateInFlightRef.current.has(tool)) {
      pushToast(
        "info",
        `${label} já está sendo verificado`,
        "A consulta atual continua em andamento.",
      );
      return;
    }

    const cached = toolUpdateCacheRef.current.get(tool);
    if (cached && performance.now() - cached.checkedAt < TOOL_UPDATE_CACHE_MS) {
      pushToast(
        "info",
        `${label} verificado recentemente`,
        cached.result.message,
      );
      return;
    }
    const downloadBusy = downloadInFlightRef.current || downloading;
    if (downloadBusy || (tool === "yt-dlp" && (searchInFlightRef.current || searching))) {
      pushToast(
        "info",
        "Atualização aguardando",
        downloadBusy
          ? `Finalize o download atual antes de atualizar o ${label}.`
          : "Finalize a pesquisa atual antes de atualizar o yt-dlp.",
      );
      return;
    }

    updateInFlightRef.current.add(tool);
    setUpdatingTools((current) => new Set(current).add(tool));
    setRecentlyUpdatedTools((current) => {
      const next = new Set(current);
      next.delete(tool);
      return next;
    });
    try {
      const command = tool === "yt-dlp" ? "update_yt_dlp" : "update_ffmpeg";
      const result = await appInvoke<ToolUpdateResult>(command);
      toolUpdateCacheRef.current.set(tool, { checkedAt: performance.now(), result });
      setTools((current) => {
        if (!current) return current;
        return tool === "yt-dlp"
          ? {
              ...current,
              ytDlp: true,
              ytDlpVersion: result.currentVersion || current.ytDlpVersion,
            }
          : {
              ...current,
              ffmpeg: true,
              ffmpegVersion: result.currentVersion || current.ffmpegVersion,
            };
      });
      if (result.updated) {
        setRecentlyUpdatedTools((current) => new Set(current).add(tool));
        const timer = window.setTimeout(() => {
          setRecentlyUpdatedTools((current) => {
            const next = new Set(current);
            next.delete(tool);
            return next;
          });
        }, 1_800);
        toastTimersRef.current.push(timer);
      }
      pushToast(
        result.updated ? "success" : "info",
        result.updated
          ? `${label} atualizado`
          : result.status === "preserved"
            ? `${label} preservado`
            : `${label} já está atualizado`,
        result.message,
      );
    } catch (error) {
      pushToast("error", `Falha ao atualizar ${label}`, errorText(error));
    } finally {
      updateInFlightRef.current.delete(tool);
      setUpdatingTools((current) => {
        const next = new Set(current);
        next.delete(tool);
        return next;
      });
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
    if (!supportsImageCover(detected)) {
      setFormat((current) => current === "image" ? null : current);
    }
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
        const persisted = (history.items || []).find((item) => item.id === state.current?.id);
        setDownloading(state.active);
        setDownloadPercent(state.current.percent);
        downloadPercentRef.current = state.current.percent;
        downloadBufferRef.current = (persisted?.console || state.current.console || []).slice(-300);
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
      if (state.current) applyMobileRecord(state.current);
      else {
        setMobileDownloadState(state);
        setDownloading(state.active);
      }
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

  async function toggleImmersiveNavigation() {
    if (!isAndroidRuntime || mobileActionBusy) return;
    setMobileActionBusy(true);
    try {
      const settings = await appInvoke<MobileSettings>("set_immersive_navigation", {
        enabled: !(mobileSettings?.immersiveNavigation ?? true),
      });
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

  async function deleteMobileCookieFile() {
    if (!isAndroidRuntime || mobileActionBusy) return;
    setMobileActionBusy(true);
    try {
      await appInvoke("delete_cookie_file");
      setCookieFile("");
      setCookies("none");
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
    searchRequestRef.current += 1;
    setSearching(false);
    setPlatform(id);
    setPlatformFolder(id === "Outro" ? "" : id);
    if (id === "Spotify") {
      setFormat("mp3");
      setDownloadSubtitles(false);
      setResults([]);
      setSelectedResult(null);
    }
    if (!supportsImageCover(id)) {
      setFormat((current) => current === "image" ? null : current);
    }
    if (id !== "YouTube") {
      setSourceMode("url");
      setPreviewMedia(null);
    }
  }

  async function pasteUrl() {
    try {
      const value = isAndroidRuntime
        ? (await appInvoke<MobileClipboard>("read_clipboard")).text
        : await navigator.clipboard.readText();
      if (value) {
        setDirectUrl(value.trim());
        setSearchError("");
        pushToast("success", "Link colado", "A URL foi adicionada ao campo de mídia.");
      } else {
        setSearchError("A área de transferência não contém um link.");
        pushToast("info", "Nada para colar", "A área de transferência não contém um link.");
      }
    } catch (error) {
      setSearchError(`Não foi possível acessar a área de transferência: ${errorText(error)}`);
      pushToast("error", "Falha ao colar", errorText(error));
    }
  }

  async function searchVideos() {
    if (!searchAvailable || !searchQuery.trim() || searchInFlightRef.current) return;
    if (updateInFlightRef.current.has("yt-dlp") || updatingTools.has("yt-dlp")) {
      pushToast(
        "info",
        "yt-dlp em atualização",
        "A pesquisa ficará disponível assim que a verificação terminar.",
      );
      return;
    }
    const requestId = ++searchRequestRef.current;
    searchInFlightRef.current = true;
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
      searchInFlightRef.current = false;
      if (requestId === searchRequestRef.current) setSearching(false);
    }
  }

  async function importSpotifyTrack() {
    if (searchInFlightRef.current || !directUrl.trim() || updatingTools.has("yt-dlp")) return;
    const requestId = ++searchRequestRef.current;
    searchInFlightRef.current = true;
    setSearching(true);
    setSearchError("");
    setResults([]);
    setSelectedResult(null);
    try {
      const title = await appInvoke<string>("spotify_track_query", { url: directUrl.trim() });
      if (requestId !== searchRequestRef.current) return;
      setSearchQuery(title);
      setSourceMode("search");
      const items = await appInvoke<SearchResult[]>("search_videos", { query: `${title} official audio` });
      if (requestId === searchRequestRef.current) setResults(items);
    } catch (error) {
      if (requestId === searchRequestRef.current) setSearchError(errorText(error));
    } finally {
      searchInFlightRef.current = false;
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
    if (!format || downloading || downloadInFlightRef.current) return;
    if (spotify && (sourceMode !== "search" || !selectedMedia || !isAudio(format))) {
      pushToast("info", "Escolha a faixa", "Importe o link, selecione um resultado do YouTube e escolha um formato de áudio.");
      return;
    }
    if (updateInFlightRef.current.size > 0 || updatingTools.size > 0) {
      pushToast(
        "info",
        "Ferramenta em atualização",
        "O download ficará disponível assim que a verificação terminar.",
      );
      return;
    }
    downloadInFlightRef.current = true;
    const startedAt = Date.now();
    const desktopJobId = globalThis.crypto?.randomUUID?.() || `download-${startedAt}`;
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
        downloadInFlightRef.current = false;
        return;
      }
      setMobileActionBusy(false);
    }
    setDownloading(true);
    setDownloadPercent(0);
    setConsoleAutoScroll(true);
    setConsoleCopied(false);
    const initialLines = isAndroidRuntime
      ? [
          "[sistema] Abrindo console Android…",
          isImage(format)
            ? "[sistema] Preparando o yt-dlp incorporado…"
            : "[sistema] Preparando yt-dlp e FFmpeg incorporados…",
        ]
      : [];
    setDownloadLines(initialLines);
    downloadBufferRef.current = initialLines;
    downloadPercentRef.current = 0;
    setDownloadMessage("");
    setDownloadError("");
    const request: DownloadRequest = {
      url: activeUrl.trim(),
      platformFolder: platformFolder.trim(),
      format,
      subtitles: downloadSubtitles,
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
      if (!isAndroidRuntime) {
        const completedRecord: DesktopHistoryRecord = {
          id: desktopJobId,
          title: downloadTitle,
          url: activeUrl.trim(),
          status: "completed",
          percent: 100,
          message: result.message,
          outputDir: result.outputDir,
          createdAt: startedAt,
        };
        setDesktopHistory((current) => [completedRecord, ...current].slice(0, 40));
      }
      pushToast("success", "Download concluído", result.message);
    } catch (error) {
      const message = errorText(error);
      setDownloadError(message);
      setDownloadLines((current) => [...current, `ERRO: ${message}`]);
      if (!isAndroidRuntime) {
        const failedRecord: DesktopHistoryRecord = {
          id: desktopJobId,
          title: downloadTitle,
          url: activeUrl.trim(),
          status: "failed",
          percent: downloadPercentRef.current,
          message,
          outputDir: platformFolder,
          createdAt: startedAt,
        };
        setDesktopHistory((current) => [failedRecord, ...current].slice(0, 40));
      }
      pushToast("error", "O download falhou", message);
    } finally {
      downloadInFlightRef.current = false;
      setDownloading(false);
    }
  }

  async function openFolder() {
    try {
      await appInvoke("open_downloads_folder", { platformFolder: platformFolder || null });
    } catch (error) {
      setDownloadError(errorText(error));
      pushToast("error", "Não foi possível abrir a pasta", errorText(error));
    }
  }

  async function copyConsoleLog() {
    if (!downloadLines.length) return;
    try {
      await navigator.clipboard.writeText(downloadLines.join("\n"));
      setConsoleCopied(true);
      pushToast("success", "Log copiado", `${downloadLines.length} linhas foram copiadas.`);
      window.setTimeout(() => setConsoleCopied(false), 1_800);
    } catch (error) {
      pushToast("error", "Falha ao copiar o log", errorText(error));
    }
  }

  async function copyCommand() {
    const command = commandPreview();
    if (!command) return;
    try {
      await navigator.clipboard.writeText(command);
      pushToast("success", "Comando copiado", "A prévia segura foi enviada para a área de transferência.");
    } catch (error) {
      pushToast("error", "Falha ao copiar o comando", errorText(error));
    }
  }

  function handleConsoleScroll(event: React.UIEvent<HTMLDivElement>) {
    const element = event.currentTarget;
    const distanceFromBottom = element.scrollHeight - element.scrollTop - element.clientHeight;
    setConsoleAutoScroll(distanceFromBottom < 28);
  }

  async function historyOpen(item: HistoryRecord) {
    if (isAndroidRuntime) {
      await mobileHistoryAction("open_download_item", item.id);
    } else {
      await openFolder();
    }
  }

  async function historyDelete(item: HistoryRecord) {
    if (isAndroidRuntime) {
      await mobileHistoryAction("delete_download_item", item.id);
    } else {
      setDesktopHistory((current) => current.filter((record) => record.id !== item.id));
      pushToast("info", "Item removido", "O registro foi retirado do histórico local.");
    }
  }

  async function clearHistoryLibrary() {
    if (isAndroidRuntime) {
      await clearMobileHistory();
    } else {
      setDesktopHistory([]);
      pushToast("info", "Histórico limpo", "Os registros desta instalação foram removidos.");
    }
  }

  function goNext() {
    if (validStep) setStep((current) => Math.min(steps.length - 1, current + 1));
  }

  function commandPreview() {
    if (!format) return "";
    const parts = [
      "yt-dlp",
      isAndroidRuntime
        ? `-o "Downloads/YT-DLP Deck/${platformFolder}/%(title)s [%(id)s].%(ext)s"`
        : `-o "Downloads\\YT-DLP Deck\\${platformFolder}\\%(uploader)s - %(title)s [%(id)s].%(ext)s"`,
      "--no-playlist",
    ];
    if (!isImage(format)) parts.push("--add-metadata", "--concurrent-fragments 4");
    if (cookies === "file" && cookieFile) parts.push(`--cookies "${cookieFile}"`);
    else if (cookies !== "none") parts.push(`--cookies-from-browser ${cookies}`);
    if (isAudio(format)) parts.push(`-f bestaudio/best -x --audio-format ${format} --audio-quality ${format === "mp3" ? "320K" : "0"}`);
    else if (isImage(format)) parts.push("--skip-download --write-thumbnail --ignore-no-formats-error");
    else {
      parts.push(`-f "${quality}" --merge-output-format ${format === "best" ? "mkv" : format}`);
    }
    if (downloadSubtitles) {
      parts.push("--write-subs --write-auto-subs");
      parts.push('--sub-langs "pt-BR,pt,pt.*,en,en.*" --sub-format "vtt/srt/best"');
    }
    parts.push("--", `"${activeUrl}"`);
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
          {platforms.filter((item) => !isAndroidRuntime || item.id !== "Spotify").map((item) => (
            <ChoiceCard
              key={item.id}
              active={platform === item.id}
              icon={item.icon}
              color={item.color}
              title={item.label}
              description={item.description}
              onClick={() => choosePlatform(item.id)}
              onDoubleClick={() => {
                choosePlatform(item.id);
                if (item.id !== "Outro") setStep(1);
              }}
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
                onDoubleClick={() => platformFolder.trim() && setStep(1)}
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
        <div
          className={`source-tabs ${searchAvailable ? "" : "is-single"}`}
          role="tablist"
          aria-label="Forma de escolher o conteúdo"
        >
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
          {searchAvailable && (
            <button
              type="button"
              role="tab"
              aria-selected={sourceMode === "search"}
              aria-controls="source-search-panel"
              className={sourceMode === "search" ? "is-active" : ""}
              onClick={() => setSourceMode("search")}
              title="Pesquisar no YouTube"
            >
              <Search size={16} /> Pesquisar vídeo
            </button>
          )}
          <motion.span
            className="tab-indicator"
            animate={{ x: sourceMode === "url" ? 0 : "100%" }}
            transition={transition}
          />
        </div>
        {spotify && (
          <div className="source-search-note" role="note">
            <AudioLines size={15} /> Spotify fornece apenas o nome da faixa. O áudio será de um resultado do YouTube escolhido por você, não do Spotify. Confira a versão e o artista; edite a pesquisa se necessário.
          </div>
        )}
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
                    onKeyDown={(event) => event.key === "Enter" && directUrl.trim() && (spotify ? void importSpotifyTrack() : goNext())}
                    onDoubleClick={() => !spotify && directUrl.trim() && setStep(2)}
                    placeholder={directUrlPlaceholder}
                  />
                  <button className="soft-button" onClick={pasteUrl} title="Colar URL">
                    <Clipboard size={17} /> Colar
                  </button>
                </div>
              </div>
              {spotify && (
                <button type="button" className="primary-button" onClick={importSpotifyTrack} disabled={!directUrl.trim() || searching || updatingTools.has("yt-dlp")}>
                  {searching ? <LoaderCircle className="spin" size={18} /> : <Search size={18} />}
                  {searching ? "Consultando faixa…" : "Importar faixa e buscar no YouTube"}
                </button>
              )}
              {searchError && <InlineError message={searchError} />}
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
                    disabled={!searchQuery.trim() || searching || updatingTools.has("yt-dlp")}
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
                    onDoubleClick={() => {
                      chooseResult(item, index);
                      setStep(2);
                    }}
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
          description="Escolha a mídia e ative a legenda como complemento para salvar os dois arquivos juntos."
          icon={Film}
        />
        <div className="format-sections">
          {(["Vídeo", "Áudio", "Imagem", "Legenda"] as const)
            .filter((category) => !spotify || category === "Áudio")
            .filter((category) => category !== "Imagem" || supportsImageCover(platform))
            .map((category) => (
            <section key={category}>
              <div className="section-label">
                {category === "Vídeo"
                  ? <MonitorPlay size={15} />
                  : category === "Áudio"
                    ? <AudioLines size={15} />
                    : category === "Imagem"
                      ? <ImageDown size={15} />
                      : <Subtitles size={15} />}
                {category}
              </div>
              <div className="card-grid format-grid">
                {formats
                  .filter((item) => item.category === category)
                  .map((item) => (
                    <ChoiceCard
                      key={item.id}
                      active={item.id === "subtitle" ? downloadSubtitles : format === item.id}
                      icon={item.icon}
                      color={item.color}
                      title={item.label}
                      description={item.description}
                      tag={item.category}
                      onClick={() => {
                        if (item.id === "subtitle") {
                          setDownloadSubtitles((current) => !current);
                          if (format === "image") setFormat(null);
                        } else {
                          setFormat(item.id);
                          if (item.id === "image") setDownloadSubtitles(false);
                        }
                      }}
                      onDoubleClick={() => {
                        if (item.id === "subtitle") {
                          setDownloadSubtitles(true);
                          if (format && format !== "image") setStep(3);
                        } else {
                          setFormat(item.id);
                          if (item.id === "image") setDownloadSubtitles(false);
                          setStep(3);
                        }
                      }}
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
    const image = isImage(format);
    const qualityDisabled = audio || image;
    return (
      <>
        <PageHeader
          eyebrow="Passo 04 · controle"
          title={<>Ajuste qualidade e <em>acesso.</em></>}
          description={
            isAndroidRuntime
              ? "Escolha a resolução quando necessário. Imagens e legendas são extraídas diretamente pelo yt-dlp incorporado."
              : "Escolha a resolução quando necessário. Para conteúdo restrito, use uma sessão do navegador ou um arquivo cookies.txt."
          }
          icon={Settings2}
        />
        <div className="settings-layout">
          <section className={`settings-panel ${qualityDisabled ? "is-disabled" : ""}`}>
            <div className="panel-heading">
              <span>
                <Gauge size={19} />
              </span>
              <div>
                <strong>{qualityDisabled ? "Saída selecionada" : "Qualidade do vídeo"}</strong>
                <small>
                  {audio
                    ? "Não se aplica ao formato de áudio"
                    : image
                      ? "A capa será salva na resolução original"
                      : "Com fallback automático"}
                </small>
              </div>
            </div>
            {qualityDisabled ? (
              <div className="audio-quality-message">
                {audio ? <AudioLines size={28} /> : <ImageDown size={28} />}
                <strong>
                  {audio
                    ? "Qualidade máxima de áudio"
                    : "Melhor capa disponível"}
                </strong>
                <small>
                  {audio
                    ? format === "mp3"
                      ? "Melhor faixa disponível → MP3 320 kbps. A qualidade final depende da fonte original."
                      : format === "flac"
                        ? "Melhor faixa disponível → FLAC sem novas perdas. Uma fonte com perdas não vira áudio lossless original; frequência e canais não são aumentados artificialmente."
                        : "A conversão usará a melhor faixa disponível."
                    : "O arquivo original fornecido pela plataforma será preservado."}
                </small>
              </div>
            ) : (
              <div className="quality-list">
                {qualities.map((item) => (
                  <button
                    key={item.id}
                    className={quality === item.id ? "is-active" : ""}
                    onClick={() => setQuality(item.id)}
                    onDoubleClick={() => {
                      setQuality(item.id);
                      setStep(4);
                    }}
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
                    onDoubleClick={() => {
                      setCookies(item.id);
                      if (item.id !== "file") setStep(4);
                    }}
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
                    <>
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
                      {cookieFile && (
                        <button
                          type="button"
                          className="mobile-file-picker is-danger"
                          onClick={deleteMobileCookieFile}
                          disabled={mobileActionBusy}
                        >
                          <Trash2 size={16} /> Apagar cookies importados
                        </button>
                      )}
                    </>
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
              <button
                type="button"
                className={`mobile-setting-toggle ${mobileSettings?.immersiveNavigation !== false ? "is-active" : ""}`}
                role="switch"
                aria-checked={mobileSettings?.immersiveNavigation !== false}
                onClick={toggleImmersiveNavigation}
                disabled={mobileActionBusy}
              >
                <span>
                  <MonitorPlay size={17} />
                  <span>
                    <strong>Ocultar navegação do sistema</strong>
                    <small>Deslize de baixo para cima para mostrá-la temporariamente</small>
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
    const StatusIcon = downloadVisualStatus === "completed"
      ? CheckCheck
      : downloadVisualStatus === "failed" || downloadVisualStatus === "cancelled"
        ? TriangleAlert
        : downloadVisualStatus === "paused"
          ? CirclePause
          : downloadVisualStatus === "idle"
            ? HardDriveDownload
            : Activity;
    const visualStatusLabel = downloadVisualStatus === "idle"
      ? "Pronto"
      : mobileStatusLabels[downloadVisualStatus] || downloadStageLabel;
    return (
      <>
        <PageHeader
          eyebrow="Passo 05 · download"
          title={<>Seu download está <em>pronto para decolar.</em></>}
          description="Revise tudo e acompanhe o progresso sem sair do aplicativo."
          icon={HardDriveDownload}
        />
        <button
          type="button"
          className="back-button review-back-button"
          onClick={() => setStep(3)}
        >
          <ArrowLeft size={17} /> Voltar para ajustes
        </button>
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
                  isAudio(format)
                    ? "Melhor faixa de áudio"
                    : isImage(format)
                      ? "Capa original"
                      : selectedQuality?.label || "—"
                }${downloadSubtitles ? " · + legenda" : ""}`}
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
              <div className="terminal-head-tools">
                <span>
                  <SquareTerminal size={14} /> comando seguro
                </span>
                <button type="button" onClick={copyCommand} title="Copiar comando">
                  <Copy size={14} /> Copiar
                </button>
              </div>
            </div>
            <pre>{commandPreview()}</pre>
          </section>
        </div>
        <section className={`download-station status-${downloadVisualStatus}`}>
          <header className="download-station-head">
            <div>
              <span className="station-mark">
                <HardDriveDownload size={20} />
              </span>
              <span>
                <small>Central de execução</small>
                <strong>Download e telemetria</strong>
              </span>
            </div>
            <span className={`download-status-pill status-${downloadVisualStatus}`}>
              <StatusIcon size={15} />
              {visualStatusLabel}
            </span>
          </header>

          <div className="download-actions">
            <button
              className="download-button"
              onClick={startDownload}
              disabled={downloading || updatingTools.size > 0}
            >
              <span className="button-shine" />
              {downloading ? <LoaderCircle className="spin" /> : <Download />}
              <span>
                <strong>{downloading ? downloadStageLabel : "Iniciar download"}</strong>
                <small>
                  {mobilePaused
                    ? "Toque em Retomar para continuar"
                    : downloading
                      ? "Acompanhe o console em tempo real"
                      : isImage(format)
                        ? "Extrair diretamente com yt-dlp"
                        : "Executar com yt-dlp + FFmpeg"}
                </small>
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
          </div>

          <AnimatePresence>
            {showDownloadActivity && (
              <motion.article
                className={`download-job-card status-${downloadVisualStatus}`}
                initial={{ opacity: 0, y: 14, scale: 0.985 }}
                animate={{
                  opacity: 1,
                  y: 0,
                  scale: 1,
                  x: downloadVisualStatus === "failed" ? [0, -4, 4, -2, 2, 0] : 0,
                }}
                exit={{ opacity: 0, y: 10, scale: 0.99 }}
                transition={downloadVisualStatus === "failed"
                  ? { duration: 0.38 }
                  : transition}
              >
                <div className="download-job-thumbnail">
                  {chosenMedia?.thumbnail ? (
                    <img src={chosenMedia.thumbnail} alt="" />
                  ) : (
                    <HardDriveDownload size={28} />
                  )}
                  <motion.span
                    className={`job-state-icon status-${downloadVisualStatus}`}
                    animate={downloadVisualStatus === "completed"
                      ? { scale: [0.75, 1.16, 1], rotate: [-12, 4, 0] }
                      : { scale: 1, rotate: 0 }}
                  >
                    <StatusIcon size={16} />
                  </motion.span>
                </div>

                <div className="download-job-body">
                  <div className="download-job-title">
                    <span>
                      <small>Item atual</small>
                      <strong>{downloadTitle}</strong>
                    </span>
                    <span className="job-percent">{Math.round(downloadPercent)}%</span>
                  </div>
                  <p>{activeUrl}</p>
                  <div className="download-telemetry">
                    <span>
                      <small>Tamanho</small>
                      <strong>{downloadTelemetry.size}</strong>
                    </span>
                    <span>
                      <small>Velocidade</small>
                      <strong>{downloadTelemetry.speed}</strong>
                    </span>
                    <span>
                      <small>Tempo restante</small>
                      <strong>{downloadTelemetry.eta}</strong>
                    </span>
                  </div>
                  <div
                    className="job-progress-track"
                    role="progressbar"
                    aria-label="Progresso do download"
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={Math.round(downloadPercent)}
                  >
                    <motion.i
                      animate={{ width: `${downloadPercent}%` }}
                      transition={{ type: "spring", stiffness: 105, damping: 22 }}
                    />
                    <span style={{ left: `${Math.min(100, Math.max(0, downloadPercent))}%` }} />
                  </div>
                </div>

                <div className="download-job-actions">
                  <button type="button" onClick={openFolder} title="Abrir pasta">
                    <FolderOpen size={16} />
                    <span>Abrir pasta</span>
                  </button>
                  {isAndroidRuntime && downloading && (
                    <>
                      <button
                        type="button"
                        onClick={() =>
                          controlMobileDownload(mobileDownloadState?.paused ? "resume" : "pause")
                        }
                        disabled={mobileActionBusy || !mobileCanPause}
                        title={mobileCanPause ? undefined : "Pausa indisponível durante o processamento"}
                      >
                        {mobileDownloadState?.paused ? <CirclePlay size={16} /> : <CirclePause size={16} />}
                        <span>{mobileDownloadState?.paused ? "Retomar" : "Pausar"}</span>
                      </button>
                      <button
                        type="button"
                        className="is-danger"
                        onClick={() => controlMobileDownload("cancel")}
                        disabled={mobileActionBusy}
                      >
                        <X size={16} />
                        <span>Cancelar</span>
                      </button>
                    </>
                  )}
                </div>
              </motion.article>
            )}
          </AnimatePresence>

          <AnimatePresence mode="popLayout">
            {downloadMessage && (
              <motion.div
                className="success-message"
                role="status"
                aria-live="polite"
                initial={{ opacity: 0, y: 8, scale: 0.985 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                exit={{ opacity: 0, y: -6 }}
              >
                <CheckCircle2 size={17} /> {downloadMessage}
              </motion.div>
            )}
            {downloadError && <InlineError message={downloadError} />}
          </AnimatePresence>

          {(downloadLines.length > 0 || (isAndroidRuntime && downloading)) && (
            <motion.div
              className="live-console-shell"
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={transition}
            >
              <div className="live-console-head">
                <span>
                  <SquareTerminal size={14} />
                  <span>
                    <strong>console yt-dlp + FFmpeg</strong>
                    <small>{downloadLines.length} linhas · {isAndroidRuntime ? "Android" : "Windows"} nativo</small>
                  </span>
                </span>
                <div className="console-actions">
                  <button
                    type="button"
                    className={consoleAutoScroll ? "is-live" : ""}
                    onClick={() => setConsoleAutoScroll((current) => !current)}
                    title={consoleAutoScroll ? "Pausar acompanhamento" : "Retomar acompanhamento"}
                  >
                    {consoleAutoScroll ? <Pause size={14} /> : <Play size={14} />}
                    {consoleAutoScroll ? "Ao vivo" : "Pausado"}
                  </button>
                  <button
                    type="button"
                    onClick={copyConsoleLog}
                    disabled={!downloadLines.length}
                    title="Copiar log"
                  >
                    {consoleCopied ? <Check size={14} /> : <Copy size={14} />}
                    {consoleCopied ? "Copiado" : "Copiar"}
                  </button>
                </div>
              </div>
              <div
                className="live-console"
                ref={consoleRef}
                role="log"
                tabIndex={0}
                onScroll={handleConsoleScroll}
                aria-label="Saída em tempo real do yt-dlp e FFmpeg"
              >
                {downloadLines.length ? (
                  downloadLines.map((line, index) => (
                    <span className={`console-line is-${getConsoleLineTone(line)}`} key={`${index}-${line}`}>
                      <i>{String(index + 1).padStart(3, "0")}</i>
                      <code>{line}</code>
                    </span>
                  ))
                ) : (
                  <span className="console-line is-info">
                    <i>001</i>
                    <code>[sistema] Aguardando saída do yt-dlp…</code>
                  </span>
                )}
              </div>
            </motion.div>
          )}
        </section>
        <section className="mobile-download-library history-library">
          <div className="mobile-library-head">
            <div>
              <span><History size={17} /> Histórico</span>
              <small>
                {isAndroidRuntime
                  ? "Biblioteca persistente de downloads"
                  : "Atividade salva localmente nesta instalação"}
              </small>
            </div>
            {historyItems.length > 0 && (
              <button onClick={() => void clearHistoryLibrary()} disabled={mobileActionBusy}>
                <Trash2 size={15} /> Limpar finalizados
              </button>
            )}
          </div>
          {historyItems.length === 0 ? (
            <motion.div
              className="mobile-library-empty history-empty-state"
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
            >
              <span><History size={24} /></span>
              <strong>Nenhum download no histórico</strong>
              <small>As tarefas finalizadas aparecerão aqui, organizadas por data.</small>
            </motion.div>
          ) : (
            <div className="history-groups">
              {historyGroups.map((group) => (
                <section className="history-date-group" key={group.label}>
                  <div className="history-date-label">
                    <span>{group.label}</span>
                    <i />
                    <small>{group.records.length} {group.records.length === 1 ? "item" : "itens"}</small>
                  </div>
                  <div className="mobile-history-list">
                    {group.records.slice(0, 12).map((item, index) => {
                      const RecordIcon = item.status === "completed"
                        ? CheckCircle2
                        : item.status === "failed" || item.status === "cancelled"
                          ? TriangleAlert
                          : item.status === "paused"
                            ? CirclePause
                            : Activity;
                      const isActiveRecord = ["queued", "running", "paused", "processing", "saving"].includes(item.status);
                      return (
                        <motion.article
                          className={`mobile-history-item status-${item.status}`}
                          key={item.id}
                          initial={{ opacity: 0, y: 10 }}
                          animate={{ opacity: 1, y: 0 }}
                          transition={{ ...transition, delay: Math.min(index * 0.045, 0.22) }}
                        >
                          <span className="history-status-icon">
                            <RecordIcon size={17} />
                          </span>
                          <div className="mobile-history-main">
                            <strong>{item.title || "Download"}</strong>
                            <span>
                              {mobileStatusLabels[item.status] || item.status}
                              <i>·</i>
                              <b>{Math.round(item.percent)}%</b>
                              <i>·</i>
                              {new Intl.DateTimeFormat("pt-BR", {
                                hour: "2-digit",
                                minute: "2-digit",
                              }).format(new Date(item.createdAt))}
                            </span>
                            <small>{item.message}</small>
                            {isActiveRecord && (
                              <div className="history-progress-track" aria-hidden="true">
                                <motion.i
                                  animate={{ width: `${item.percent}%` }}
                                  transition={{ type: "spring", stiffness: 105, damping: 22 }}
                                />
                              </div>
                            )}
                          </div>
                          <div className="mobile-history-actions">
                            {item.status === "completed" && (!isAndroidRuntime || item.fileUri) && (
                              <button onClick={() => void historyOpen(item)}>
                                <FolderOpen size={14} /> Abrir
                              </button>
                            )}
                            {isAndroidRuntime && item.status === "completed" && item.fileUri && (
                              <button onClick={() => mobileHistoryAction("share_download_item", item.id)}>
                                <Share2 size={14} /> Compartilhar
                              </button>
                            )}
                            {!isActiveRecord && (
                              <button className="is-danger" onClick={() => void historyDelete(item)}>
                                <Trash2 size={14} /> Excluir
                              </button>
                            )}
                          </div>
                        </motion.article>
                      );
                    })}
                  </div>
                </section>
              ))}
            </div>
          )}
        </section>
      </>
    );
  }

  const pages = [renderPlatform, renderSource, renderFormats, renderSettings, renderReview];
  const CurrentStepIcon = steps[step].icon;

  return (
    <MotionConfig reducedMotion="user">
    <div className="app-shell">
      <canvas className="gpu-backdrop" ref={gpuCanvasRef} aria-hidden="true" />
      <div className="desktop-atmosphere" aria-hidden="true">
        <span className="desktop-grid" />
        <span className="desktop-aurora desktop-aurora-orange" />
        <span className="desktop-aurora desktop-aurora-violet" />
        <span className="desktop-aurora desktop-aurora-cyan" />
        <span className="desktop-orbit desktop-orbit-one" />
        <span className="desktop-orbit desktop-orbit-two" />
        <span className="desktop-beam" />
      </div>
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
          <ToolBadge
            tool="yt-dlp"
            label="yt-dlp"
            ready={Boolean(tools?.ytDlp)}
            version={tools?.ytDlpVersion}
            checking={checkingTools || updatingTools.has("yt-dlp")}
            recentlyUpdated={recentlyUpdatedTools.has("yt-dlp")}
            disabled={checkingTools || updatingTools.has("yt-dlp")}
            onUpdate={updateTool}
          />
          <ToolBadge
            tool="ffmpeg"
            label="FFmpeg"
            ready={Boolean(tools?.ffmpeg)}
            version={tools?.ffmpegVersion}
            checking={checkingTools || updatingTools.has("ffmpeg")}
            recentlyUpdated={recentlyUpdatedTools.has("ffmpeg")}
            disabled={checkingTools || updatingTools.has("ffmpeg")}
            onUpdate={updateTool}
          />
          <span className="sr-only" role="status" aria-live="polite" aria-atomic="true">
            {checkingTools
              ? "Verificando se o yt-dlp e o FFmpeg estão instalados."
              : updatingTools.size > 0
                ? `Procurando atualizações de ${[
                    updatingTools.has("yt-dlp") ? "yt-dlp" : "",
                    updatingTools.has("ffmpeg") ? "FFmpeg" : "",
                  ].filter(Boolean).join(" e ")}.`
                : ""}
          </span>
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
              <button onClick={() => void checkTools(true)} disabled={checkingTools}>
                {checkingTools ? <LoaderCircle className="spin" size={15} /> : <RotateCcw size={15} />}
                {checkingTools ? "Verificando" : "Checar novamente"}
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
        <AnimatePresence>
          {isAndroidRuntime && step < steps.length - 1 && validStep && (
            <motion.button
              type="button"
              className="mobile-continue-fab"
              onClick={goNext}
              initial={{ opacity: 0, y: 24, scale: 0.92 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 18, scale: 0.94 }}
              transition={transition}
            >
              Continuar <ArrowRight size={18} />
            </motion.button>
          )}
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
      <ToastViewport items={toasts} />
    </div>
    </MotionConfig>
  );
}

export default App;
