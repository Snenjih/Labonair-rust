/**
 * Single source of truth for keyboard shortcuts. Each entry carries:
 * - `keys`: display tokens for the cheat-sheet dialog.
 * - `match`: predicate over the live KeyboardEvent used by `useGlobalShortcuts`.
 *
 * Keeping both on the same record means the dialog can never lie about a
 * binding the runtime no longer matches (or vice-versa).
 */

export type ShortcutId =
  | "tab.new"
  | "tab.newPreview"
  | "tab.newEditor"
  | "tab.close"
  | "tab.next"
  | "tab.prev"
  | "tab.selectTab1"
  | "tab.selectTab2"
  | "tab.selectTab3"
  | "tab.selectTab4"
  | "tab.selectTab5"
  | "tab.selectTab6"
  | "tab.selectTab7"
  | "tab.selectTab8"
  | "tab.selectTab9"
  | "pane.splitRight"
  | "pane.splitDown"
  | "pane.close"
  | "pane.focusNext"
  | "search.focus"
  | "ai.toggle"
  | "ai.askSelection"
  | "shortcuts.open"
  | "command.palette"
  | "sidebar.toggle"
  | "view.zenMode"
  | "view.zoomIn"
  | "view.zoomOut"
  | "view.zoomReset"
  | "bookmarks.open";

export type ShortcutGroup = "General" | "Tabs" | "Search" | "AI" | "View" | "Bookmarks";

export type Shortcut = {
  id: ShortcutId;
  label: string;
  keys: string[];
  group: ShortcutGroup;
  match: (e: KeyboardEvent) => boolean;
};

export const isMod = (e: KeyboardEvent) => e.metaKey || e.ctrlKey;
// Split shortcuts are macOS-only (Cmd key) — Ctrl+D must not trigger them.
const isMeta = (e: KeyboardEvent) => e.metaKey && !e.ctrlKey;

export const SHORTCUTS: Shortcut[] = [
  {
    id: "command.palette",
    label: "Open command palette",
    keys: ["⌘", "P"],
    group: "General",
    match: (e) => isMod(e) && !e.shiftKey && e.key.toLowerCase() === "p",
  },
  {
    id: "shortcuts.open",
    label: "Show keyboard shortcuts",
    keys: ["⌘", "?"],
    group: "General",
    match: (e) => isMod(e) && e.key === "?",
  },
  {
    id: "tab.new",
    label: "New tab",
    keys: ["⌘", "T"],
    group: "Tabs",
    match: (e) => isMod(e) && e.key.toLowerCase() === "t",
  },
  {
    id: "tab.newPreview",
    label: "New preview tab",
    keys: ["⌘", "⇧", "P"],
    group: "Tabs",
    match: (e) => isMod(e) && e.shiftKey && e.key.toLowerCase() === "p",
  },
  {
    id: "tab.newEditor",
    label: "New editor tab",
    keys: ["⌘", "E"],
    group: "Tabs",
    match: (e) => isMod(e) && !e.shiftKey && e.key.toLowerCase() === "e",
  },
  {
    id: "tab.close",
    label: "Close tab",
    keys: ["⌘", "W"],
    group: "Tabs",
    match: (e) => isMod(e) && !e.shiftKey && e.key.toLowerCase() === "w",
  },
  {
    id: "tab.next",
    label: "Next tab",
    keys: ["⌃", "⇥"],
    group: "Tabs",
    // Ctrl+Tab is conventionally Ctrl-only on every platform (including macOS).
    match: (e) => e.ctrlKey && !e.shiftKey && e.key === "Tab",
  },
  {
    id: "tab.prev",
    label: "Previous tab",
    keys: ["⌃", "⇧", "⇥"],
    group: "Tabs",
    match: (e) => e.ctrlKey && e.shiftKey && e.key === "Tab",
  },
  // Nine separate entries, not one range match — each must be independently
  // rebindable/disableable via the exact-single-key capture UI in
  // KeyboardShortcutsSection, which cannot represent a "1-9" range as one binding.
  {
    id: "tab.selectTab1",
    label: "Jump to tab 1",
    keys: ["⌘", "1"],
    group: "Tabs",
    match: (e) => isMod(e) && e.key === "1",
  },
  {
    id: "tab.selectTab2",
    label: "Jump to tab 2",
    keys: ["⌘", "2"],
    group: "Tabs",
    match: (e) => isMod(e) && e.key === "2",
  },
  {
    id: "tab.selectTab3",
    label: "Jump to tab 3",
    keys: ["⌘", "3"],
    group: "Tabs",
    match: (e) => isMod(e) && e.key === "3",
  },
  {
    id: "tab.selectTab4",
    label: "Jump to tab 4",
    keys: ["⌘", "4"],
    group: "Tabs",
    match: (e) => isMod(e) && e.key === "4",
  },
  {
    id: "tab.selectTab5",
    label: "Jump to tab 5",
    keys: ["⌘", "5"],
    group: "Tabs",
    match: (e) => isMod(e) && e.key === "5",
  },
  {
    id: "tab.selectTab6",
    label: "Jump to tab 6",
    keys: ["⌘", "6"],
    group: "Tabs",
    match: (e) => isMod(e) && e.key === "6",
  },
  {
    id: "tab.selectTab7",
    label: "Jump to tab 7",
    keys: ["⌘", "7"],
    group: "Tabs",
    match: (e) => isMod(e) && e.key === "7",
  },
  {
    id: "tab.selectTab8",
    label: "Jump to tab 8",
    keys: ["⌘", "8"],
    group: "Tabs",
    match: (e) => isMod(e) && e.key === "8",
  },
  {
    id: "tab.selectTab9",
    label: "Jump to tab 9",
    keys: ["⌘", "9"],
    group: "Tabs",
    match: (e) => isMod(e) && e.key === "9",
  },
  {
    id: "pane.splitRight",
    label: "Split pane right",
    keys: ["⌘", "D"],
    group: "Tabs",
    match: (e) => isMeta(e) && !e.shiftKey && e.key.toLowerCase() === "d",
  },
  {
    id: "pane.splitDown",
    label: "Split pane down",
    keys: ["⌘", "⇧", "D"],
    group: "Tabs",
    match: (e) => isMeta(e) && e.shiftKey && e.key.toLowerCase() === "d",
  },
  {
    id: "pane.close",
    label: "Close pane",
    keys: ["⌘", "⇧", "W"],
    group: "Tabs",
    match: (e) => isMod(e) && e.shiftKey && e.key.toLowerCase() === "w",
  },
  {
    id: "pane.focusNext",
    label: "Focus next pane",
    keys: ["⌘", "]"],
    group: "Tabs",
    match: (e) => isMeta(e) && !e.shiftKey && e.key === "]",
  },
  {
    id: "search.focus",
    label: "Find in current pane",
    keys: ["⌘", "F"],
    group: "Search",
    match: (e) => isMod(e) && e.key.toLowerCase() === "f",
  },
  {
    id: "ai.toggle",
    label: "Toggle AI agent",
    keys: ["⌘", "I"],
    group: "AI",
    match: (e) => isMod(e) && e.key.toLowerCase() === "i",
  },
  {
    id: "ai.askSelection",
    label: "Ask AI about selection",
    keys: ["⌘", "L"],
    group: "AI",
    match: (e) => isMod(e) && e.key.toLowerCase() === "l",
  },
  {
    id: "sidebar.toggle",
    label: "Toggle file explorer",
    keys: ["⌘", "B"],
    group: "View",
    match: (e) => isMod(e) && e.key.toLowerCase() === "b",
  },
  {
    id: "view.zenMode",
    label: "Toggle Zen mode",
    keys: ["⌘", "⇧", "Z"],
    group: "View",
    match: (e) => isMod(e) && e.shiftKey && e.key.toLowerCase() === "z",
  },
  {
    id: "view.zoomIn",
    label: "Zoom in",
    keys: ["⌘", "+"],
    group: "View",
    match: (e) => isMod(e) && (e.key === "+" || e.key === "="),
  },
  {
    id: "view.zoomOut",
    label: "Zoom out",
    keys: ["⌘", "−"],
    group: "View",
    match: (e) => isMod(e) && e.key === "-",
  },
  {
    id: "view.zoomReset",
    label: "Reset zoom",
    keys: ["⌘", "0"],
    group: "View",
    match: (e) => isMod(e) && e.key === "0",
  },
  {
    id: "bookmarks.open",
    label: "Open path bookmarks",
    keys: ["⌘", "⇧", "O"],
    group: "Bookmarks",
    match: (e) => isMod(e) && e.shiftKey && e.key.toLowerCase() === "o",
  },
];

export const SHORTCUT_GROUPS: ShortcutGroup[] = ["General", "Tabs", "View", "Search", "AI", "Bookmarks"];
