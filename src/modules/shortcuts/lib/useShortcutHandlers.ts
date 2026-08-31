import type React from "react";
import { useMemo } from "react";
import { useCommandStore } from "@/modules/command-palette";
import type { EditorPaneHandle } from "@/modules/editor";
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  DEFAULT_PREFERENCES,
  setEditorFontSize,
  setSftpFontSize,
  setTerminalFontSize,
  setZenModeShowHeader,
  setZenModeShowStatusbar,
} from "@/modules/settings/store";
import { useGlobalShortcuts } from "@/modules/shortcuts";
import type { WorkspaceTab } from "@/modules/tabs";
import { selectActiveTabKind, useTabsStore } from "@/modules/tabs";
import { collectLeafIds } from "@/modules/tabs/types";
import type { WorkspacePaneHandle } from "@/modules/terminal";

export interface UseShortcutHandlersOptions {
  openNewTab: () => void;
  handleClose: (id: number) => void;
  cycleTab: (delta: 1 | -1) => void;
  togglePanelAndFocus: () => void;
  askFromSelection: () => void;
  toggleSidebar: () => void;
  openPreviewTab: (url: string) => number;
  workspacePaneRefs: React.MutableRefObject<Map<number, WorkspacePaneHandle>>;
  activeEditorHandle: EditorPaneHandle | null;
  openShortcuts: () => void;
  openFind: () => void;
}

export function useShortcutHandlers(opts: UseShortcutHandlersOptions): void {
  const {
    openNewTab,
    handleClose,
    cycleTab,
    togglePanelAndFocus,
    askFromSelection,
    toggleSidebar,
    openPreviewTab,
    workspacePaneRefs,
    activeEditorHandle,
    openShortcuts,
  } = opts;

  const toggleCommandPalette = useCommandStore((s) => s.toggle);

  const { openUntitledTab, selectByIndex, splitPane, closePane, setActivePaneId } = useTabsStore.getState();

  const shortcutHandlers = useMemo(
    () => ({
      "command.palette": () => toggleCommandPalette(),
      "tab.new": openNewTab,
      "tab.newPreview": () => openPreviewTab(""),
      "tab.newEditor": () => void openUntitledTab(),
      "tab.close": () => handleClose(useTabsStore.getState().activeId),
      "tab.next": () => cycleTab(1),
      "tab.prev": () => cycleTab(-1),
      "tab.selectTab1": () => selectByIndex(0),
      "tab.selectTab2": () => selectByIndex(1),
      "tab.selectTab3": () => selectByIndex(2),
      "tab.selectTab4": () => selectByIndex(3),
      "tab.selectTab5": () => selectByIndex(4),
      "tab.selectTab6": () => selectByIndex(5),
      "tab.selectTab7": () => selectByIndex(6),
      "tab.selectTab8": () => selectByIndex(7),
      "tab.selectTab9": () => selectByIndex(8),
      "search.focus": () => {
        const kind = selectActiveTabKind(useTabsStore.getState());
        const { activeId: aid } = useTabsStore.getState();
        if (kind === "workspace") workspacePaneRefs.current.get(aid)?.openFind();
        else if (kind === "editor") activeEditorHandle?.openFind();
      },
      "ai.toggle": togglePanelAndFocus,
      "ai.askSelection": askFromSelection,
      "shortcuts.open": () => openShortcuts(),
      "sidebar.toggle": toggleSidebar,
      "view.zenMode": () => {
        const { zenModeShowHeader: showH, zenModeShowStatusbar: showS } = usePreferencesStore.getState();
        const anyVisible = showH || showS;
        void setZenModeShowHeader(!anyVisible);
        void setZenModeShowStatusbar(!anyVisible);
      },
      "pane.splitRight": () => {
        const { tabs: storeTabs, activeId: aid } = useTabsStore.getState();
        if (storeTabs.find((t) => t.id === aid)?.kind === "workspace") splitPane(aid, "horizontal");
      },
      "pane.splitDown": () => {
        const { tabs: storeTabs, activeId: aid } = useTabsStore.getState();
        if (storeTabs.find((t) => t.id === aid)?.kind === "workspace") splitPane(aid, "vertical");
      },
      "pane.close": () => {
        const { tabs: storeTabs, activeId: aid } = useTabsStore.getState();
        const tab = storeTabs.find((t) => t.id === aid);
        if (tab?.kind === "workspace") closePane(aid, (tab as WorkspaceTab).activePaneId);
      },
      "pane.focusNext": () => {
        const { tabs: storeTabs, activeId: aid } = useTabsStore.getState();
        const tab = storeTabs.find((t) => t.id === aid);
        if (tab?.kind !== "workspace") return;
        const workspaceTab = tab as WorkspaceTab;
        const leafIds = collectLeafIds(workspaceTab.layout);
        if (leafIds.length <= 1) return; // single-pane tab — nothing to focus
        const currentIndex = leafIds.indexOf(workspaceTab.activePaneId);
        const nextIndex = currentIndex === -1 ? 0 : (currentIndex + 1) % leafIds.length;
        setActivePaneId(aid, leafIds[nextIndex]);
      },
      "view.zoomIn": () => {
        const kind = selectActiveTabKind(useTabsStore.getState());
        if (kind === "workspace")
          void setTerminalFontSize(Math.min(usePreferencesStore.getState().terminalFontSize + 1, 32));
        else if (kind === "editor")
          void setEditorFontSize(Math.min(usePreferencesStore.getState().editorFontSize + 1, 32));
        else if (kind === "sftp")
          void setSftpFontSize(Math.min(usePreferencesStore.getState().sftpFontSize + 1, 20));
      },
      "view.zoomOut": () => {
        const kind = selectActiveTabKind(useTabsStore.getState());
        if (kind === "workspace")
          void setTerminalFontSize(Math.max(usePreferencesStore.getState().terminalFontSize - 1, 8));
        else if (kind === "editor")
          void setEditorFontSize(Math.max(usePreferencesStore.getState().editorFontSize - 1, 8));
        else if (kind === "sftp")
          void setSftpFontSize(Math.max(usePreferencesStore.getState().sftpFontSize - 1, 10));
      },
      "view.zoomReset": () => {
        const kind = selectActiveTabKind(useTabsStore.getState());
        if (kind === "workspace") void setTerminalFontSize(DEFAULT_PREFERENCES.terminalFontSize);
        else if (kind === "editor") void setEditorFontSize(DEFAULT_PREFERENCES.editorFontSize);
        else if (kind === "sftp") void setSftpFontSize(DEFAULT_PREFERENCES.sftpFontSize);
      },
      "bookmarks.open": () => {
        const prefs = usePreferencesStore.getState();
        // Deliberate exception to the general "hidden never disables the
        // shortcut" bar-item convention (see BarItemPlacement.hidden in
        // settings/lib/barItems.ts) — this shortcut has no effect without
        // the popover that only the visible BookmarksDropdown renders, so
        // treat "badge hidden" the same as "feature disabled" rather than
        // dispatching into a void with no listener.
        if (!prefs.bookmarksEnabled || prefs.barItemPlacements.bookmarks.hidden) return;
        // BookmarksDropdown listens for this to open the same popover the
        // titlebar icon opens — see comment there for why a CustomEvent
        // instead of a threaded `open` prop.
        window.dispatchEvent(new CustomEvent("labonair:bookmarks-open"));
      },
    }),
    [
      activeEditorHandle,
      cycleTab,
      handleClose,
      openNewTab,
      openPreviewTab,
      workspacePaneRefs,
      togglePanelAndFocus,
      askFromSelection,
      toggleSidebar,
      toggleCommandPalette,
      openShortcuts,
      openUntitledTab,
      selectByIndex,
      splitPane,
      closePane,
      setActivePaneId,
    ],
  );

  useGlobalShortcuts(shortcutHandlers);
}
