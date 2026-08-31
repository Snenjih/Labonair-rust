import type { DragEndEvent } from "@dnd-kit/core";
import { closestCenter, DndContext } from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Cancel01Icon, PlusSignIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ReactNode } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { ContextMenu, ContextMenuTrigger } from "@/components/ui/context-menu";
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { type ConnectionStatus, useConnectionStatusStore } from "@/modules/hosts/store/connectionStatusStore";
import { useHostsStore } from "@/modules/hosts/store/hostsStore";
import { usePreferencesStore } from "@/modules/settings/preferences";
import { useTransferStore } from "@/modules/sftp/store/transferStore";
import { isCommandRunning, subscribeIntegrationState } from "@/modules/terminal/lib/terminalSessionRegistry";
import { NonWorkspaceTabContextMenuContent } from "./components/NonWorkspaceTabContextMenuContent";
import { WorkspaceTabContextMenuContent } from "./components/WorkspaceTabContextMenuContent";
import { buildGroupedRenderPlan, isBlockedCrossGroupDrag, pathKeyFor } from "./lib/tabGrouping";
import {
  labelFor,
  NewTabDropdownItems,
  remoteHostLabelFor,
  sidebarInfoLineFor,
  TabIconFor,
} from "./lib/tabUtils";
import { resolveDragReorder, shouldMiddleClickClose, useTabList } from "./lib/useTabList";
import { useTabsStore } from "./store/tabsStore";
import type { Tab, WorkspaceTab } from "./types";

type Props = {
  onSelect: (id: number) => void;
  onNew: () => void;
  onNewPreview: () => void;
  onNewEditor: () => void;
  onNewSsh: (hostId: string, title: string) => void;
  onNewSftp: (hostId: string, title: string) => void;
  onOpenHostManager: () => void;
  onClose: (id: number) => void;
  onCloseOthers: (id: number) => void;
  onCloseAll: () => void;
  onCloseByKind: (kind: Tab["kind"]) => void;
  onDuplicate: (id: number) => void;
  onRename: (id: number, label: string) => void;
  onNewGitGraph?: () => void;
};

export function SidebarTabList({
  onSelect,
  onNew,
  onNewPreview,
  onNewEditor,
  onNewSsh,
  onNewSftp,
  onOpenHostManager,
  onClose,
  onCloseOthers,
  onCloseAll,
  onCloseByKind,
  onDuplicate,
  onRename,
  onNewGitGraph,
}: Props) {
  const { tabs, activeId, sensors, handleDragEnd, isNewTab, editingId, startEditing, finishEditing } =
    useTabList();
  const scrollRef = useRef<HTMLDivElement>(null);
  const groupByFolder = usePreferencesStore((s) => s.sidebarGroupByFolder);
  const groupSingleTabs = usePreferencesStore((s) => s.sidebarGroupSingleTabs);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const active = el.querySelector<HTMLElement>(`[data-tab-id="${activeId}"]`);
    active?.scrollIntoView({ block: "nearest" });
  }, [activeId, tabs.length]);

  // Folder grouping is presentation-only — it reads live path/cwd data but
  // is debounced by 1.5s so a rapid `cd` doesn't re-shuffle the sidebar on
  // every prompt. The very first time grouping turns on, apply immediately
  // instead of waiting — the debounce is for smoothing ongoing churn, not
  // for delaying a deliberate settings toggle.
  const [debouncedPathKeys, setDebouncedPathKeys] = useState<Map<number, string | undefined>>(
    () => new Map(tabs.map((t) => [t.id, pathKeyFor(t)])),
  );
  const wasGroupingRef = useRef(false);
  const groupDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    if (!groupByFolder) {
      wasGroupingRef.current = false;
      return;
    }
    const freshKeys = new Map(tabs.map((t) => [t.id, pathKeyFor(t)]));
    if (!wasGroupingRef.current) {
      wasGroupingRef.current = true;
      setDebouncedPathKeys(freshKeys);
      return;
    }
    if (groupDebounceRef.current) clearTimeout(groupDebounceRef.current);
    groupDebounceRef.current = setTimeout(() => setDebouncedPathKeys(freshKeys), 1500);
    return () => {
      if (groupDebounceRef.current) clearTimeout(groupDebounceRef.current);
    };
  }, [groupByFolder, tabs]);

  const renderPlan = useMemo(
    () =>
      groupByFolder
        ? buildGroupedRenderPlan(tabs, debouncedPathKeys, groupSingleTabs ? 1 : 2)
        : tabs.map((t) => ({ kind: "tab" as const, tab: t, groupKey: undefined, groupSize: 0 })),
    [groupByFolder, groupSingleTabs, tabs, debouncedPathKeys],
  );
  const visualTabIds = useMemo(
    () => renderPlan.filter((e) => e.kind === "tab").map((e) => e.tab.id),
    [renderPlan],
  );

  const guardedHandleDragEnd = useCallback(
    (event: DragEndEvent) => {
      if (groupByFolder) {
        const reorder = resolveDragReorder(event);
        if (!reorder) return;
        const fromEntry = renderPlan.find((e) => e.kind === "tab" && e.tab.id === reorder.from);
        const toEntry = renderPlan.find((e) => e.kind === "tab" && e.tab.id === reorder.to);
        const fromGroupKey = fromEntry?.kind === "tab" ? fromEntry.groupKey : undefined;
        const toGroupKey = toEntry?.kind === "tab" ? toEntry.groupKey : undefined;
        if (isBlockedCrossGroupDrag(fromGroupKey, toGroupKey)) return;
      }
      handleDragEnd(event);
    },
    [groupByFolder, renderPlan, handleDragEnd],
  );

  return (
    <div className="flex h-full flex-col">
      <div
        ref={scrollRef}
        className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto p-1.5 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
      >
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={guardedHandleDragEnd}>
          <SortableContext items={visualTabIds} strategy={verticalListSortingStrategy}>
            {renderPlan.map((entry) =>
              entry.kind === "header" ? (
                <SidebarGroupHeader key={entry.key} name={entry.name} />
              ) : (
                <SidebarTabRow
                  key={entry.tab.id}
                  t={entry.tab}
                  groupKey={entry.groupKey}
                  groupSize={entry.groupSize}
                  isActive={entry.tab.id === activeId}
                  isNew={isNewTab(entry.tab.id)}
                  isRenaming={editingId === entry.tab.id && entry.tab.kind === "workspace"}
                  tabsLength={tabs.length}
                  onSelect={onSelect}
                  onClose={onClose}
                  onCloseOthers={onCloseOthers}
                  onCloseAll={onCloseAll}
                  onCloseByKind={onCloseByKind}
                  onDuplicate={onDuplicate}
                  onStartRename={() => startEditing(entry.tab.id)}
                  onCommitRename={(value) => {
                    onRename(entry.tab.id, value);
                    finishEditing();
                  }}
                  onCancelRename={finishEditing}
                />
              ),
            )}
          </SortableContext>
        </DndContext>
      </div>

      <div className="shrink-0 border-t border-border/60 p-1.5">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              className="w-full justify-start gap-1.5 text-xs text-muted-foreground hover:bg-accent hover:text-foreground"
              title="New tab"
            >
              <HugeiconsIcon icon={PlusSignIcon} size={13} strokeWidth={2} />
              New Tab
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="min-w-44">
            <NewTabDropdownItems
              onNew={onNew}
              onNewPreview={onNewPreview}
              onNewEditor={onNewEditor}
              onNewSsh={onNewSsh}
              onNewSftp={onNewSftp}
              onOpenHostManager={onOpenHostManager}
              onNewGitGraph={onNewGitGraph}
            />
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}

function dotClassFor(status: ConnectionStatus): string {
  switch (status) {
    case "connecting":
      return "bg-warning animate-pulse";
    case "connected":
      return "bg-success";
    case "error":
      return "bg-destructive";
  }
}

function SidebarGroupHeader({ name }: { name: string }) {
  return (
    <div className="truncate border-t border-border/60 px-2 pt-3 pb-1.5 text-[13px] text-muted-foreground first:border-t-0 first:pt-1">
      {name}
    </div>
  );
}

function SidebarTabRow({
  t,
  groupKey,
  groupSize,
  isActive,
  isNew,
  isRenaming,
  tabsLength,
  onSelect,
  onClose,
  onCloseOthers,
  onCloseAll,
  onCloseByKind,
  onDuplicate,
  onStartRename,
  onCommitRename,
  onCancelRename,
}: {
  t: Tab;
  groupKey: string | undefined;
  groupSize: number;
  isActive: boolean;
  isNew: boolean;
  isRenaming: boolean;
  tabsLength: number;
  onSelect: (id: number) => void;
  onClose: (id: number) => void;
  onCloseOthers: (id: number) => void;
  onCloseAll: () => void;
  onCloseByKind: (kind: Tab["kind"]) => void;
  onDuplicate: (id: number) => void;
  onStartRename: () => void;
  onCommitRename: (value: string) => void;
  onCancelRename: () => void;
}) {
  const selectedInfoTypes = usePreferencesStore((s) => s.sidebarTabInfoLine);
  const hosts = useHostsStore((s) => s.hosts);
  const connections = useConnectionStatusStore((s) => s.connections);
  // Only shown for tabs inside an actual multi-member folder-group — disambiguates
  // a remote tab from local tabs sharing the same folder. A solo tab grouped alone
  // (eager "group single tabs" mode) has nothing to disambiguate against, so it's
  // skipped even though groupKey is set.
  const remoteBadge =
    groupKey !== undefined && groupSize >= 2 ? remoteHostLabelFor(t, hosts, connections) : undefined;
  const transferJobs = useTransferStore((s) => s.jobs);

  // "Busy" only makes sense for the active pane of a workspace tab — track
  // it reactively via the terminal session registry's own subscription
  // (same mechanism ShellComposerInput uses for command-running state).
  const busyEnabled = t.kind === "workspace" && selectedInfoTypes.includes("busy");
  const activeSessionId = t.kind === "workspace" ? (t as WorkspaceTab).activePaneId : undefined;
  const [isBusy, setIsBusy] = useState(false);
  useEffect(() => {
    if (!busyEnabled || !activeSessionId) {
      setIsBusy(false);
      return;
    }
    setIsBusy(isCommandRunning(activeSessionId));
    return subscribeIntegrationState(activeSessionId, () => setIsBusy(isCommandRunning(activeSessionId)));
  }, [busyEnabled, activeSessionId]);

  // "Uptime" text needs to tick forward on its own — nothing else re-renders
  // this row on a timer otherwise.
  const uptimeEnabled = selectedInfoTypes.includes("uptime");
  const [nowMs, setNowMs] = useState(() => Date.now());
  useEffect(() => {
    if (!uptimeEnabled) return;
    const id = setInterval(() => setNowMs(Date.now()), 30_000);
    return () => clearInterval(id);
  }, [uptimeEnabled]);

  const infoSegments =
    selectedInfoTypes.length > 0 && !isRenaming
      ? sidebarInfoLineFor(t, selectedInfoTypes, hosts, connections, transferJobs, isBusy, nowMs)
      : [];

  // Render a non-button cell while renaming (input can't be inside button).
  if (isRenaming) {
    return (
      <SortableSidebarTabWrapper id={t.id} disabled>
        <div
          data-tab-id={t.id}
          className="flex w-full items-center gap-1.5 rounded-lg bg-foreground/[0.07] px-2 py-1.5 text-xs"
        >
          <span className="shrink-0">
            <TabIconFor tab={t} active={true} />
          </span>
          <SidebarRenameInput initial={labelFor(t)} onCommit={onCommitRename} onCancel={onCancelRename} />
        </div>
      </SortableSidebarTabWrapper>
    );
  }

  const hasInfoLine = infoSegments.length > 0;

  const btn = (
    <button
      type="button"
      data-tab-id={t.id}
      onClick={() => onSelect(t.id)}
      onAuxClick={(e) => {
        if (shouldMiddleClickClose(e.button, tabsLength, t.kind)) {
          e.preventDefault();
          e.stopPropagation();
          onClose(t.id);
        }
      }}
      onMouseDown={(e) => {
        if (e.button === 1) e.preventDefault();
      }}
      onDoubleClick={() => {
        // Peek tabs promote to permanent on double-click, same as editing
        // them — VS Code's "keep open" gesture.
        if (t.kind === "editor" && t.peek) {
          useTabsStore.getState().updateTab(t.id, { peek: false });
        }
      }}
      className={cn(
        "group flex w-full gap-1.5 rounded-lg px-2 py-1.5 text-left text-xs transition-colors",
        hasInfoLine ? "items-start" : "items-center",
        isNew && "labonair-tab-in",
        isActive
          ? "bg-foreground/[0.07] text-foreground"
          : "text-muted-foreground hover:bg-accent/30 hover:text-foreground",
      )}
    >
      <span className={cn("shrink-0", hasInfoLine && "mt-0.5")}>
        <TabIconFor tab={t} active={isActive} />
      </span>
      <span className="flex min-w-0 flex-1 flex-col gap-0.5 overflow-hidden">
        <span className="flex items-center gap-1.5 truncate">
          <span className={cn("truncate", t.kind === "editor" && t.peek && "italic")}>{labelFor(t)}</span>
          {remoteBadge && (
            <span
              title={remoteBadge}
              className="max-w-[90px] shrink-0 truncate rounded-sm border border-border/60 bg-accent/40 px-1 py-px text-[9px] leading-normal text-muted-foreground"
            >
              {remoteBadge}
            </span>
          )}
          {t.kind === "editor" && t.remoteSyncFailed ? (
            <span
              aria-label="Failed to save to remote — click to retry"
              title="Failed to save to remote — click to retry"
              className="size-1.5 shrink-0 rounded-full bg-destructive"
            />
          ) : t.kind === "editor" && t.dirty ? (
            <span aria-label="Unsaved changes" className="size-1.5 shrink-0 rounded-full bg-foreground/70" />
          ) : null}
        </span>
        {hasInfoLine && (
          <span className="flex items-center gap-1 truncate text-[10px] text-muted-foreground">
            {infoSegments.map((seg, i) => (
              <span
                key={seg.type}
                className={cn("flex min-w-0 items-center gap-1", seg.type === "path" ? "shrink" : "shrink-0")}
              >
                {i > 0 && (
                  <span aria-hidden className="shrink-0 text-muted-foreground/50">
                    ·
                  </span>
                )}
                {seg.type === "connection" && (
                  <span className={cn("size-1 shrink-0 rounded-full", dotClassFor(seg.status))} />
                )}
                {seg.type === "busy" && (
                  <span className="size-1 shrink-0 animate-pulse rounded-full bg-foreground/60" />
                )}
                {seg.type === "path" ? (
                  // Truncate from the front so the meaningful tail (the folder/file
                  // itself) stays visible instead of the shared prefix — `dir="rtl"`
                  // moves the ellipsis to the start while the LTR path text still
                  // reads left-to-right.
                  <span dir="rtl" className="min-w-0 truncate text-left">
                    {seg.text}
                  </span>
                ) : (
                  <span className="truncate">{seg.text}</span>
                )}
              </span>
            ))}
          </span>
        )}
      </span>
      {tabsLength > 1 && t.kind !== "home" && (
        <span
          role="button"
          tabIndex={0}
          aria-label="Close tab"
          onClick={(e) => {
            e.stopPropagation();
            onClose(t.id);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              e.stopPropagation();
              onClose(t.id);
            }
          }}
          className={cn(
            "shrink-0 rounded p-0.5 opacity-0 outline-none transition-opacity hover:bg-accent hover:opacity-100 focus-visible:opacity-100 focus-visible:ring-1 focus-visible:ring-ring/50 group-hover:opacity-60",
            hasInfoLine && "mt-0.5",
          )}
        >
          <HugeiconsIcon icon={Cancel01Icon} size={11} strokeWidth={2} />
        </span>
      )}
    </button>
  );

  return (
    <SortableSidebarTabWrapper id={t.id} disabled={false}>
      <ContextMenu>
        <ContextMenuTrigger asChild>{btn}</ContextMenuTrigger>
        {t.kind === "workspace" ? (
          <WorkspaceTabContextMenuContent
            tab={t as WorkspaceTab}
            tabsLength={tabsLength}
            onStartRename={onStartRename}
            onClose={onClose}
            onDuplicate={onDuplicate}
            onCloseOthers={onCloseOthers}
            onCloseByKind={onCloseByKind}
          />
        ) : (
          <NonWorkspaceTabContextMenuContent
            tab={t}
            onClose={onClose}
            onDuplicate={onDuplicate}
            onCloseOthers={onCloseOthers}
            onCloseAll={onCloseAll}
            onCloseByKind={onCloseByKind}
          />
        )}
      </ContextMenu>
    </SortableSidebarTabWrapper>
  );
}

function SortableSidebarTabWrapper({
  id,
  disabled,
  children,
}: {
  id: number;
  disabled: boolean;
  children: ReactNode;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id,
    disabled,
  });

  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={isDragging ? "relative z-50 opacity-50" : undefined}
      {...attributes}
      {...listeners}
    >
      {children}
    </div>
  );
}

function SidebarRenameInput({
  initial,
  onCommit,
  onCancel,
}: {
  initial: string;
  onCommit: (value: string) => void;
  onCancel: () => void;
}) {
  const ref = useRef<HTMLInputElement>(null);
  const done = useRef(false);

  useEffect(() => {
    const raf = requestAnimationFrame(() => {
      ref.current?.focus();
      ref.current?.select();
    });
    return () => cancelAnimationFrame(raf);
  }, []);

  const finish = (fn: () => void) => {
    if (done.current) return;
    done.current = true;
    fn();
  };

  const commit = (value: string, explicit: boolean) => {
    if (!explicit && value.trim() === initial.trim()) finish(onCancel);
    else finish(() => onCommit(value));
  };

  return (
    <input
      ref={ref}
      defaultValue={initial}
      aria-label="Rename tab"
      className="min-w-0 flex-1 rounded-sm bg-background px-1 text-xs text-foreground outline-none ring-1 ring-border focus:ring-ring"
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === "Enter") commit(e.currentTarget.value, true);
        else if (e.key === "Escape") finish(onCancel);
      }}
      onBlur={(e) => {
        if (!document.hasFocus()) return;
        commit(e.currentTarget.value, false);
      }}
    />
  );
}
