import { memo } from "react";
import { motion } from "framer-motion";
import { Check, type LucideIcon } from "lucide-react";
import { transition } from "../config/motion";

export const ChoiceCard = memo(function ChoiceCard({
  active,
  icon: Icon,
  color,
  title,
  description,
  tag,
  onClick,
  onDoubleClick,
}: {
  active: boolean;
  icon: LucideIcon;
  color: string;
  title: string;
  description: string;
  tag?: string;
  onClick: () => void;
  onDoubleClick?: () => void;
}) {
  return (
    <motion.button
      type="button"
      className={`choice-card ${active ? "is-active" : ""}`}
      style={{ "--card-color": color } as React.CSSProperties}
      onClick={onClick}
      onDoubleClick={onDoubleClick}
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
