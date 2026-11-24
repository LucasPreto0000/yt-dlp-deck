import { AnimatePresence, motion } from "framer-motion";
import { CheckCircle2, TriangleAlert, Info } from "lucide-react";
import type { ToastItem } from "../features/downloads/uiTypes";
import { transition } from "../config/motion";

export function ToastViewport({ items }: { items: ToastItem[] }) {
  return (
    <div className="toast-viewport" aria-live="polite" aria-atomic="false">
      <AnimatePresence initial={false}>
        {items.map((item) => {
          const Icon = item.tone === "success"
            ? CheckCircle2
            : item.tone === "error"
              ? TriangleAlert
              : Info;
          return (
            <motion.div
              className={`toast-card is-${item.tone}`}
              key={item.id}
              initial={{ opacity: 0, x: 32, scale: 0.96 }}
              animate={{ opacity: 1, x: 0, scale: 1 }}
              exit={{ opacity: 0, x: 24, scale: 0.97 }}
              transition={transition}
              role={item.tone === "error" ? "alert" : "status"}
            >
              <span className="toast-icon">
                <Icon size={18} />
              </span>
              <span className="toast-copy">
                <strong>{item.title}</strong>
                <small>{item.message}</small>
              </span>
              <span className="toast-timer" />
            </motion.div>
          );
        })}
      </AnimatePresence>
    </div>
  );
}
