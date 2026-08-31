import { Cancel01Icon, TerminalIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cn } from "@/lib/utils";
import { useTabsStore } from "@/modules/tabs";
import { useChatStore } from "../store/chatStore";

/**
 * Shows which terminal tab the active chat session's `bash_run` calls are
 * currently sticky-bound to (see `src/modules/ai/lib/resolveTerminalTarget.ts`)
 * — so the user always knows where the agent is acting even after they've
 * switched away to a different tab. Self-heals: renders nothing the instant
 * the bound tab is closed, without waiting for the next tool call to notice.
 */
export function BoundTabBadge() {
  const sessionId = useChatStore((s) => s.activeSessionId);
  const bound = useChatStore((s) => (sessionId ? s.boundTabs[sessionId] : undefined));
  const clearBoundTab = useChatStore((s) => s.clearBoundTab);
  const tab = useTabsStore((s) => s.tabs.find((t) => t.id === bound?.tabId));
  const setActiveId = useTabsStore((s) => s.setActiveId);

  if (!sessionId || !bound || !tab || tab.kind !== "workspace") return null;
  const label = tab.customTitle ?? tab.title ?? "Terminal";

  return (
    <div
      className={cn(
        "flex min-w-0 max-w-32 items-center gap-1 rounded-md px-1.5 py-1",
        "text-[11px] text-muted-foreground transition-colors",
        "hover:bg-accent hover:text-foreground",
      )}
    >
      <button
        type="button"
        onClick={() => setActiveId(tab.id)}
        title={`bash_run is bound to "${label}" — click to jump there`}
        className="flex min-w-0 items-center gap-1"
      >
        <HugeiconsIcon icon={TerminalIcon} size={10} strokeWidth={1.75} className="shrink-0 opacity-70" />
        <span className="truncate">{label}</span>
      </button>
      <button
        type="button"
        onClick={() => clearBoundTab(sessionId)}
        title="Unbind terminal"
        className="shrink-0 rounded-sm p-0.5 opacity-60 hover:bg-accent hover:opacity-100"
      >
        <HugeiconsIcon icon={Cancel01Icon} size={9} strokeWidth={2} />
      </button>
    </div>
  );
}
