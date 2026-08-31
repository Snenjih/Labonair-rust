import { CheckListIcon, Key01Icon, Settings01Icon, SparklesIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { createElement } from "react";
import { useShortcutHint } from "@/modules/shortcuts";
import type { CommandPage, RegistryCallbacks } from "../types";

export function useSystemCommands(cb: RegistryCallbacks): CommandPage {
  const shortcutsHintKeys = useShortcutHint("shortcuts.open");
  const aiToggleShortcut = useShortcutHint("ai.toggle");
  const aiAskShortcut = useShortcutHint("ai.askSelection");

  return {
    id: "system",
    searchPlaceholder: "Search commands...",
    actions: [
      {
        id: "system.settings",
        title: "Open Settings",
        section: "Application",
        icon: createElement(HugeiconsIcon, {
          icon: Settings01Icon,
          strokeWidth: 2,
          className: "size-4",
        }),
        perform: () => cb.openSettings(),
      },
      {
        id: "system.shortcuts",
        title: "Keyboard Shortcuts",
        section: "Application",
        shortcut: shortcutsHintKeys,
        icon: createElement(HugeiconsIcon, {
          icon: CheckListIcon,
          strokeWidth: 2,
          className: "size-4",
        }),
        perform: () => cb.openShortcuts(),
      },
      {
        id: "system.ai-toggle",
        title: "Toggle AI Panel",
        section: "Application",
        shortcut: aiToggleShortcut,
        icon: createElement(HugeiconsIcon, {
          icon: SparklesIcon,
          strokeWidth: 2,
          className: "size-4",
        }),
        perform: () => cb.toggleAi(),
      },
      {
        id: "system.ai-ask",
        title: "Ask AI About Selection",
        section: "Application",
        shortcut: aiAskShortcut,
        icon: createElement(HugeiconsIcon, {
          icon: SparklesIcon,
          strokeWidth: 2,
          className: "size-4",
        }),
        perform: () => cb.askSelection(),
      },
      {
        id: "system.settings-models",
        title: "Manage AI Keys & Models",
        section: "Application",
        icon: createElement(HugeiconsIcon, {
          icon: Key01Icon,
          strokeWidth: 2,
          className: "size-4",
        }),
        perform: () => cb.openSettings("models"),
      },
    ],
  };
}
