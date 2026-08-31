import React, { useCallback, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { cn } from "@/lib/utils";
import { usePreferencesStore } from "@/modules/settings/preferences";
import { useTabsStore } from "@/modules/tabs/store/tabsStore";
import type { WorkspaceTab } from "@/modules/tabs/types";
import { disposeSession } from "./lib/terminalSessionRegistry";
import type { TerminalPaneHandle } from "./TerminalPane";
import { WorkspacePane, type WorkspacePaneHandle } from "./WorkspacePane";

type Props = {
  workspacePaneRefs: React.MutableRefObject<Map<number, WorkspacePaneHandle>>;
  terminalRefs: React.MutableRefObject<Map<string, TerminalPaneHandle>>;
  onDetectedLocalUrl: (sessionId: string, url: string) => void;
};

// ─── Per-tab container ────────────────────────────────────────────────────────

type ContainerProps = {
  tabId: number;
  workspacePaneRefs: Props["workspacePaneRefs"];
  terminalRefs: Props["terminalRefs"];
  onDetectedLocalUrl: Props["onDetectedLocalUrl"];
};

function WorkspacePaneContainer({
  tabId,
  workspacePaneRefs,
  terminalRefs,
  onDetectedLocalUrl,
}: ContainerProps) {
  const tab = useTabsStore((s) => s.tabs.find((t) => t.id === tabId) as WorkspaceTab | undefined);
  const isActive = useTabsStore((s) => s.activeId === tabId);
  const terminalShowPaneFooter = usePreferencesStore((s) => s.terminalShowPaneFooter);

  const registerPaneRef = useCallback(
    (h: WorkspacePaneHandle | null) => {
      if (h) workspacePaneRefs.current.set(tabId, h);
      else workspacePaneRefs.current.delete(tabId);
    },
    [tabId, workspacePaneRefs],
  );

  const onSetActivePane = useCallback(
    (paneId: string) => useTabsStore.getState().setActivePaneId(tabId, paneId),
    [tabId],
  );

  const onRegisterHandle = useCallback(
    (sessionId: string, handle: TerminalPaneHandle | null) => {
      if (handle) terminalRefs.current.set(sessionId, handle);
      else terminalRefs.current.delete(sessionId);
    },
    [terminalRefs],
  );

  const onCwd = useCallback(
    (sessionId: string, cwd: string) => useTabsStore.getState().updatePaneSessionCwd(tabId, sessionId, cwd),
    [tabId],
  );

  const onProcessTitle = useCallback(
    (sessionId: string, title: string) =>
      useTabsStore.getState().updatePaneSessionProcessTitle(tabId, sessionId, title),
    [tabId],
  );

  const [pendingPaneClose, setPendingPaneClose] = useState<string | null>(null);

  const doClosePane = useCallback(
    (paneId: string) => {
      // Frees a bound or merely-retained renderer-pool slot (no-op if
      // neither exists — a backgrounded pane may have neither) before the
      // store removes the pane and its owning component unmounts.
      disposeSession(paneId);
      useTabsStore.getState().closePane(tabId, paneId);
    },
    [tabId],
  );

  const onClosePane = useCallback(
    (paneId: string) => {
      // `closePane` itself no-ops when this is the last pane in the only
      // open tab (`tabsStore.ts`'s own guard) — don't confirm an action
      // that will silently do nothing.
      const isLastPaneOverall = tab?.layout.type === "pane" && useTabsStore.getState().tabs.length <= 1;
      if (isLastPaneOverall) return;

      if (usePreferencesStore.getState().confirmCloseTerminalTab) {
        setPendingPaneClose(paneId);
        return;
      }
      doClosePane(paneId);
    },
    [doClosePane, tab],
  );

  if (!tab) return null;

  // Closing the last remaining pane in a tab is equivalent to closing the
  // whole tab (matches `closePane`'s own "only pane → close entire tab"
  // behavior), so it reuses that dialog's exact copy; otherwise only this
  // one pane's shell dies and the tab survives.
  const isLastPane = tab.layout.type === "pane";

  return (
    <div
      inert={!isActive || undefined}
      className={cn(
        "absolute inset-0 z-0 px-3 pt-2",
        terminalShowPaneFooter && "pb-2",
        !isActive && "opacity-0",
      )}
      aria-hidden={!isActive}
    >
      <WorkspacePane
        ref={registerPaneRef}
        tab={tab}
        tabVisible={isActive}
        onSetActivePane={onSetActivePane}
        onRegisterHandle={onRegisterHandle}
        onCwd={onCwd}
        onProcessTitle={onProcessTitle}
        onClosePane={onClosePane}
        onDetectedLocalUrl={onDetectedLocalUrl}
      />

      <AlertDialog
        open={pendingPaneClose !== null}
        onOpenChange={(open) => !open && setPendingPaneClose(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{isLastPane ? "Close terminal tab?" : "Close this pane?"}</AlertDialogTitle>
            <AlertDialogDescription>
              {isLastPane
                ? "The running shell process will be terminated."
                : "The running shell process in this pane will be terminated."}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (pendingPaneClose === null) return;
                doClosePane(pendingPaneClose);
              }}
            >
              Close
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

// ─── Stack ────────────────────────────────────────────────────────────────────

export const WorkspaceStack = React.memo(function WorkspaceStack({
  workspacePaneRefs,
  terminalRefs,
  onDetectedLocalUrl,
}: Props) {
  // Cold tabs (restored but never activated — see tabsStore's `newTab`/
  // `setActiveId`) simply don't mount here, so no PTY/SSH connection is
  // spawned for them until `setActiveId` clears the flag on first activation.
  const workspaceTabIds = useTabsStore(
    useShallow((s) =>
      s.tabs.filter((t) => t.kind === "workspace" && !(t as WorkspaceTab).cold).map((t) => t.id),
    ),
  );

  return (
    <>
      {workspaceTabIds.map((tabId) => (
        <WorkspacePaneContainer
          key={tabId}
          tabId={tabId}
          workspacePaneRefs={workspacePaneRefs}
          terminalRefs={terminalRefs}
          onDetectedLocalUrl={onDetectedLocalUrl}
        />
      ))}
    </>
  );
});
