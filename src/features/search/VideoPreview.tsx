import { useEffect, useRef } from "react";
import { motion } from "framer-motion";
import { AlertCircle, CheckCircle2, ExternalLink, MonitorPlay, X } from "lucide-react";
import type { SearchResult } from "../../types";
import { transition } from "../../config/motion";

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

export function VideoPreview({
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
