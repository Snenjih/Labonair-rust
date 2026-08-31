import {
  ArrowDown01Icon,
  ArrowRight01Icon,
  Cancel01Icon,
  Copy01Icon,
  File02Icon,
  Folder01Icon,
  TerminalIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { createElement } from "react";
import { useShortcutHint } from "@/modules/shortcuts";
import type { CommandAction, CommandPage, RegistryCallbacks } from "../types";

export function useLayoutCommands(cb: RegistryCallbacks, activeTabKind: string | undefined): CommandPage {
  const isWorkspace = activeTabKind === "workspace";

  const newTabShortcut = useShortcutHint("tab.new");
  const newEditorShortcut = useShortcutHint("tab.newEditor");
  const splitRightShortcut = useShortcutHint("pane.splitRight");
  const splitDownShortcut = useShortcutHint("pane.splitDown");
  const closePaneShortcut = useShortcutHint("pane.close");

  const actions: CommandAction[] = [
    {
      id: "layout.new-tab",
      title: "New Terminal Tab",
      section: "Layout",
      shortcut: newTabShortcut,
      icon: createElement(HugeiconsIcon, {
        icon: TerminalIcon,
        strokeWidth: 2,
        className: "size-4",
      }),
      perform: () => cb.newTab(),
    },
    {
      id: "layout.new-editor",
      title: "New Editor Tab",
      section: "Layout",
      shortcut: newEditorShortcut,
      icon: createElement(HugeiconsIcon, {
        icon: File02Icon,
        strokeWidth: 2,
        className: "size-4",
      }),
      perform: () => cb.openUntitledTab(),
    },
    {
      id: "layout.open-host-manager",
      title: "Open Host Manager",
      section: "Layout",
      icon: createElement(HugeiconsIcon, {
        icon: TerminalIcon,
        strokeWidth: 2,
        className: "size-4",
      }),
      perform: () => cb.openHomeTab(),
    },
    {
      id: "layout.open-sftp",
      title: "Open SFTP...",
      section: "Layout",
      icon: createElement(HugeiconsIcon, {
        icon: Folder01Icon,
        strokeWidth: 2,
        className: "size-4",
      }),
      subPageId: "hosts-sftp",
    },
    {
      id: "layout.duplicate-tab",
      title: "Duplicate Tab",
      section: "Layout",
      icon: createElement(HugeiconsIcon, {
        icon: Copy01Icon,
        strokeWidth: 2,
        className: "size-4",
      }),
      perform: () => cb.duplicateCurrentTab(),
    },
    {
      id: "layout.close-other-tabs",
      title: "Close Other Tabs",
      section: "Layout",
      icon: createElement(HugeiconsIcon, {
        icon: Cancel01Icon,
        strokeWidth: 2,
        className: "size-4",
      }),
      perform: () => cb.closeOtherTabs(),
    },
  ];

  if (isWorkspace) {
    actions.push(
      {
        id: "layout.split-right",
        title: "Split Pane Right",
        section: "Layout",
        shortcut: splitRightShortcut,
        icon: createElement(HugeiconsIcon, {
          icon: ArrowRight01Icon,
          strokeWidth: 2,
          className: "size-4",
        }),
        perform: () => cb.splitRight(),
      },
      {
        id: "layout.split-down",
        title: "Split Pane Down",
        section: "Layout",
        shortcut: splitDownShortcut,
        icon: createElement(HugeiconsIcon, {
          icon: ArrowDown01Icon,
          strokeWidth: 2,
          className: "size-4",
        }),
        perform: () => cb.splitDown(),
      },
      {
        id: "layout.close-pane",
        title: "Close Active Pane",
        section: "Layout",
        shortcut: closePaneShortcut,
        icon: createElement(HugeiconsIcon, {
          icon: Cancel01Icon,
          strokeWidth: 2,
          className: "size-4",
        }),
        perform: () => cb.closePane(),
      },
    );
  }

  return {
    id: "layout",
    searchPlaceholder: "Search layout commands...",
    actions,
  };
}
