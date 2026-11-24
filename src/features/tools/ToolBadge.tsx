import { Check, Info, LoaderCircle, RotateCcw } from "lucide-react";
import { isAndroidRuntime } from "../../nativeBridge";
import type { ToolId } from "../downloads/uiTypes";

export function ToolBadge({
  tool,
  label,
  ready,
  version,
  checking,
  recentlyUpdated,
  disabled,
  onUpdate,
}: {
  tool: ToolId;
  label: string;
  ready: boolean;
  version?: string;
  checking: boolean;
  recentlyUpdated: boolean;
  disabled: boolean;
  onUpdate: (tool: ToolId) => void;
}) {
  const action = isAndroidRuntime
    ? "Ver informações de atualização"
    : "Procurar atualização silenciosamente";
  return (
    <button
      type="button"
      className={`tool-badge ${ready ? "is-ready" : ""} ${checking ? "is-checking" : ""} ${recentlyUpdated ? "is-updated" : ""}`}
      title={checking
        ? `Procurando atualização do ${label}`
        : ready
          ? `${version ? `${version} · ` : ""}${action}`
          : `${label} não encontrado`}
      aria-label={`${label} ${ready ? "instalado" : "não encontrado"}. ${action}`}
      aria-busy={checking}
      aria-disabled={!ready || disabled}
      disabled={!ready || disabled}
      onClick={() => {
        if (!checking && !disabled) onUpdate(tool);
      }}
    >
      <span className="tool-light" />
      <span>{label}</span>
      <span className="tool-badge-icons" aria-hidden="true">
        {checking ? (
          <LoaderCircle className="spin" size={12} />
        ) : ready ? (
          isAndroidRuntime ? (
            <Info className="tool-info-icon" size={12} />
          ) : (
            <>
              <Check className="tool-ready-icon" size={12} />
              <RotateCcw className="tool-update-icon" size={12} />
            </>
          )
        ) : null}
      </span>
    </button>
  );
}
