import { motion } from "framer-motion";
import { AlertCircle, type LucideIcon } from "lucide-react";

export function PageHeader({
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

export function InlineError({ message }: { message: string }) {
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

export function ReviewRow({
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
