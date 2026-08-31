import { isMod } from "../shortcuts";

/**
 * Native-menu-only accelerators that have no `ShortcutId` and are never
 * customizable — hardcoded in `src-tauri/src/lib.rs::build_menu()`. Kept
 * here so the Settings capture UI can block a user from rebinding some
 * other shortcut onto one of these (which would silently collide with the
 * native menu the same way the ⌘K command-palette/shortcuts-dialog clash
 * used to, see nativeMenuSync.ts).
 */
export type ReservedAccelerator = {
  id: string;
  label: string;
  match: (e: KeyboardEvent) => boolean;
};

export const RESERVED_ACCELERATORS: ReservedAccelerator[] = [
  {
    id: "reserved.settings",
    label: "Settings",
    match: (e) => isMod(e) && !e.shiftKey && e.key === ",",
  },
  {
    id: "reserved.newSshConnection",
    label: "New SSH Connection",
    match: (e) => isMod(e) && e.shiftKey && e.key.toLowerCase() === "n",
  },
  {
    id: "reserved.keyboardShortcuts",
    label: "Keyboard Shortcuts",
    match: (e) => isMod(e) && !e.shiftKey && e.key.toLowerCase() === "k",
  },
];
