import type { LucideIcon } from "lucide-react";
import { Globe2, Search, Film, SlidersHorizontal, Download, Youtube, Music2, Instagram, Twitter, Radio, MessageCircle, Video, Disc3, Sparkles, AudioLines, FileAudio, ImageDown, Subtitles, ShieldCheck, Chrome, Compass, Flame, FileText } from "lucide-react";
import type { CookieId, FormatId, QualityId } from "../types";

export const steps: Array<{ title: string; subtitle: string; icon: LucideIcon }> = [
  { title: "Plataforma", subtitle: "Organize o destino", icon: Globe2 },
  { title: "Conteúdo", subtitle: "Link ou pesquisa", icon: Search },
  { title: "Formato", subtitle: "Vídeo, áudio ou extras", icon: Film },
  { title: "Ajustes", subtitle: "Qualidade e acesso", icon: SlidersHorizontal },
  { title: "Finalizar", subtitle: "Revise e baixe", icon: Download },
];

export const platforms: Array<{
  id: string;
  label: string;
  description: string;
  icon: LucideIcon;
  color: string;
  urlExample: string;
}> = [
  { id: "YouTube", label: "YouTube", description: "Vídeos, Shorts e lives", icon: Youtube, color: "#ff4164", urlExample: "https://www.youtube.com/watch?v=..." },
  { id: "Spotify", label: "Spotify", description: "Importar faixa · áudio via YouTube", icon: Music2, color: "#1ed760", urlExample: "https://open.spotify.com/track/..." },
  { id: "TikTok", label: "TikTok", description: "Vídeos e tendências", icon: Music2, color: "#46edf2", urlExample: "https://www.tiktok.com/@usuario/video/..." },
  { id: "Instagram", label: "Instagram", description: "Reels e publicações", icon: Instagram, color: "#f765a3", urlExample: "https://www.instagram.com/reel/..." },
  { id: "X_Twitter", label: "X / Twitter", description: "Vídeos e clipes", icon: Twitter, color: "#60baff", urlExample: "https://x.com/usuario/status/..." },
  { id: "Twitch", label: "Twitch", description: "Clipes e transmissões", icon: Radio, color: "#a978ff", urlExample: "https://www.twitch.tv/videos/..." },
  { id: "Reddit", label: "Reddit", description: "Mídias de comunidades", icon: MessageCircle, color: "#ff7045", urlExample: "https://www.reddit.com/r/comunidade/comments/..." },
  { id: "Outro", label: "Outro site", description: "Qualquer site compatível", icon: Globe2, color: "#aab2d1", urlExample: "https://exemplo.com/video" },
];

export const formats: Array<{
  id: FormatId;
  label: string;
  description: string;
  category: "Vídeo" | "Áudio" | "Imagem" | "Legenda";
  icon: LucideIcon;
  color: string;
}> = [
  { id: "mp4", label: "MP4", description: "Compatibilidade máxima", category: "Vídeo", icon: Video, color: "#62a8ff" },
  { id: "mkv", label: "MKV", description: "Todos os codecs", category: "Vídeo", icon: Disc3, color: "#a97aff" },
  { id: "webm", label: "WEBM", description: "Formato web moderno", category: "Vídeo", icon: Globe2, color: "#43d9ff" },
  { id: "best", label: "Melhor", description: "Qualidade sem limites", category: "Vídeo", icon: Sparkles, color: "#ffbd58" },
  { id: "mp3", label: "MP3", description: "320 kbps · melhor fonte disponível", category: "Áudio", icon: Music2, color: "#ff6e9a" },
  { id: "flac", label: "FLAC", description: "Sem novas perdas na conversão", category: "Áudio", icon: AudioLines, color: "#44dda8" },
  { id: "wav", label: "WAV", description: "Sem compressão", category: "Áudio", icon: FileAudio, color: "#dfe8ff" },
  { id: "m4a", label: "M4A", description: "Eficiente e moderno", category: "Áudio", icon: FileAudio, color: "#9d9cff" },
  { id: "image", label: "Imagem (capa)", description: "Capa original do vídeo", category: "Imagem", icon: ImageDown, color: "#ff9f68" },
  { id: "subtitle", label: "Legenda junto", description: "Ative junto com vídeo ou áudio", category: "Legenda", icon: Subtitles, color: "#62e6b6" },
];

export const qualities: Array<{ id: QualityId; label: string; detail: string }> = [
  { id: "best", label: "Melhor disponível", detail: "Sem limite" },
  { id: "2160p60", label: "4K · 60 FPS", detail: "2160p" },
  { id: "1440p60", label: "2K · 60 FPS", detail: "1440p" },
  { id: "1080p60", label: "Full HD · 60 FPS", detail: "1080p" },
  { id: "1080p", label: "Full HD", detail: "1080p" },
  { id: "720p", label: "HD", detail: "720p" },
  { id: "480p", label: "Compacto", detail: "480p" },
];

export const mobileStatusLabels: Record<string, string> = {
  queued: "Na fila",
  running: "Baixando",
  paused: "Pausado",
  processing: "Processando",
  saving: "Salvando",
  completed: "Concluído",
  failed: "Falhou",
  cancelled: "Cancelado",
};

export const cookieOptions: Array<{
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
