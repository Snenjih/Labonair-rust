import {
  Cancel01Icon,
  Cancel02Icon,
  CancelSquareIcon,
  Copy01Icon,
  Layers01Icon,
  PinIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { ContextMenuContent, ContextMenuItem, ContextMenuSeparator } from "@/components/ui/context-menu";
import { pluralLabelFor } from "../lib/tabUtils";
import { useTabsStore } from "../store/tabsStore";
import type { Tab } from "../types";

export interface NonWorkspaceTabContextMenuContentProps {
  tab: Tab;
  onClose: (id: number) => void;
  onDuplicate: (id: number) => void;
  onCloseOthers: (id: number) => void;
  onCloseAll: () => void;
  onCloseByKind: (kind: Tab["kind"]) => void;
}

/** Shared context menu for every non-workspace tab kind (editor, preview,
 *  sftp, home, git-graph, ...) — used identically by `TabBar` and
 *  `SidebarTabList`. */
export function NonWorkspaceTabContextMenuContent({
  tab,
  onClose,
  onDuplicate,
  onCloseOthers,
  onCloseAll,
  onCloseByKind,
}: NonWorkspaceTabContextMenuContentProps) {
  return (
    <ContextMenuContent>
      {tab.kind === "editor" && tab.peek && (
        <>
          <ContextMenuItem onSelect={() => useTabsStore.getState().updateTab(tab.id, { peek: false })}>
            <HugeiconsIcon icon={PinIcon} size={14} strokeWidth={1.75} />
            <span className="flex-1">Keep Tab Open</span>
          </ContextMenuItem>
          <ContextMenuSeparator />
        </>
      )}
      {tab.kind !== "home" && (
        <>
          <ContextMenuItem onSelect={() => onDuplicate(tab.id)}>
            <HugeiconsIcon icon={Copy01Icon} size={14} strokeWidth={1.75} />
            <span className="flex-1">Duplicate Tab</span>
          </ContextMenuItem>
          <ContextMenuSeparator />
        </>
      )}
      <ContextMenuItem onSelect={() => onCloseOthers(tab.id)}>
        <HugeiconsIcon icon={Cancel02Icon} size={14} strokeWidth={1.75} />
        <span className="flex-1">Close Others</span>
      </ContextMenuItem>
      <ContextMenuItem onSelect={onCloseAll}>
        <HugeiconsIcon icon={CancelSquareIcon} size={14} strokeWidth={1.75} />
        <span className="flex-1">Close All</span>
      </ContextMenuItem>
      <ContextMenuItem onSelect={() => onCloseByKind(tab.kind)}>
        <HugeiconsIcon icon={Layers01Icon} size={14} strokeWidth={1.75} />
        <span className="flex-1">Close All {pluralLabelFor(tab.kind)}</span>
      </ContextMenuItem>
      {tab.kind !== "home" && (
        <>
          <ContextMenuSeparator />
          <ContextMenuItem onSelect={() => onClose(tab.id)}>
            <HugeiconsIcon icon={Cancel01Icon} size={14} strokeWidth={1.75} />
            <span className="flex-1">Close Tab</span>
          </ContextMenuItem>
        </>
      )}
    </ContextMenuContent>
  );
}
