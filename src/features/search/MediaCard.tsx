import { memo } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { CheckCircle2, Clock3, Film, Play, UserRound } from "lucide-react";
import type { SearchResult } from "../../types";

export const MediaCard = memo(function MediaCard({
  media,
  selected,
  onClick,
  onDoubleClick,
  onPreview,
  large = false,
}: {
  media: SearchResult;
  selected: boolean;
  onClick?: () => void;
  onDoubleClick?: () => void;
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
          onDoubleClick={onDoubleClick}
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
