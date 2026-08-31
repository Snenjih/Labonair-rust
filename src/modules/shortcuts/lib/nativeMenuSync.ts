import type { ShortcutId } from "../shortcuts";
import type { KeyBinding, KeyBindingMap } from "../types";

/**
 * Builds a Tauri/muda accelerator string from a captured binding, e.g.
 * `"Ctrl+Shift+Tab"`. Uses LITERAL modifiers (not the cross-platform
 * "CmdOrCtrl" alias) so behavior stays in exact parity with the JS-side
 * `bindingMatchesEvent`, which checks each modifier flag independently.
 * The key token is `code` (e.g. "KeyK", "Slash", "Tab") — muda's `Code`
 * parser accepts these physical-key tokens directly, avoiding ambiguity
 * around shifted characters like "?" or "+" that `key` alone would produce.
 */
export function buildAcceleratorString(
  b: Pick<KeyBinding, "meta" | "ctrl" | "shift" | "alt" | "code">,
): string {
  const parts: string[] = [];
  if (b.ctrl) parts.push("Ctrl");
  if (b.alt) parts.push("Alt");
  if (b.shift) parts.push("Shift");
  if (b.meta) parts.push("Cmd");
  parts.push(b.code);
  return parts.join("+");
}

type MirrorEntry = { menuItemIds: string[]; defaultAccelerator: string };

/**
 * Maps every ShortcutId that has a native-menu twin to that menu item's
 * id(s) plus the SAME default accelerator string hardcoded in
 * `build_menu()` in src-tauri/src/lib.rs — this table must be kept in sync
 * with that function by hand, there is no single source of truth across
 * the Rust/TS boundary for the default strings. `command.palette` and
 * `shortcuts.open` are intentionally absent: the former has no native
 * mirror, the latter's native accelerator is permanently hardcoded to
 * CmdOrCtrl+K and never customizable (see reservedAccelerators.ts).
 */
export const MIRRORED_MENU_ITEMS: Partial<Record<ShortcutId, MirrorEntry>> = {
  "tab.new": { menuItemIds: ["new_terminal_tab"], defaultAccelerator: "CmdOrCtrl+T" },
  "tab.newPreview": { menuItemIds: ["new_preview_tab"], defaultAccelerator: "CmdOrCtrl+Shift+P" },
  "tab.newEditor": { menuItemIds: ["new_editor_tab"], defaultAccelerator: "CmdOrCtrl+E" },
  "tab.close": { menuItemIds: ["close_tab"], defaultAccelerator: "CmdOrCtrl+W" },
  "pane.close": { menuItemIds: ["close_pane"], defaultAccelerator: "CmdOrCtrl+Shift+W" },
  "sidebar.toggle": { menuItemIds: ["toggle_sidebar"], defaultAccelerator: "CmdOrCtrl+B" },
  "ai.toggle": { menuItemIds: ["toggle_ai", "toggle_ai_2"], defaultAccelerator: "CmdOrCtrl+I" },
  "view.zoomIn": { menuItemIds: ["zoom_in"], defaultAccelerator: "CmdOrCtrl+Plus" },
  "view.zoomOut": { menuItemIds: ["zoom_out"], defaultAccelerator: "CmdOrCtrl+-" },
  "view.zoomReset": { menuItemIds: ["zoom_reset"], defaultAccelerator: "CmdOrCtrl+0" },
  "pane.splitRight": { menuItemIds: ["split_pane_right"], defaultAccelerator: "CmdOrCtrl+D" },
  "pane.splitDown": { menuItemIds: ["split_pane_down"], defaultAccelerator: "CmdOrCtrl+Shift+D" },
  "search.focus": { menuItemIds: ["find"], defaultAccelerator: "CmdOrCtrl+F" },
  "tab.next": { menuItemIds: ["next_tab"], defaultAccelerator: "Ctrl+Tab" },
  "tab.prev": { menuItemIds: ["prev_tab"], defaultAccelerator: "Ctrl+Shift+Tab" },
  "ai.askSelection": { menuItemIds: ["ask_selection"], defaultAccelerator: "CmdOrCtrl+L" },
};

export type MenuAccelUpdate = { menuItemIds: string[]; accelerator: string | null };

/**
 * Pure, testable payload builder. Called whenever `overrides` (or
 * hydration) changes; always emits an entry for EVERY mirrored id (not a
 * diff) so `resetKeybind`/`resetAll` correctly restores the native default,
 * and a fresh install with `overrides === {}` sends the same defaults Rust
 * already hardcodes (a harmless no-op, keeps both sides provably
 * consistent instead of relying on Rust's initial state never being
 * touched).
 */
export function buildMenuSyncPayload(overrides: KeyBindingMap): MenuAccelUpdate[] {
  const updates: MenuAccelUpdate[] = [];
  for (const [shortcutId, mirror] of Object.entries(MIRRORED_MENU_ITEMS) as [ShortcutId, MirrorEntry][]) {
    const override = overrides[shortcutId];
    if (override === undefined) {
      updates.push({ menuItemIds: mirror.menuItemIds, accelerator: mirror.defaultAccelerator });
    } else if (override === null) {
      updates.push({ menuItemIds: mirror.menuItemIds, accelerator: null });
    } else if (typeof override.code === "string" && override.code.length > 0) {
      updates.push({ menuItemIds: mirror.menuItemIds, accelerator: buildAcceleratorString(override) });
    } else {
      // Legacy override saved before the `code` field existed — leave the
      // native menu's current accelerator untouched rather than guess.
      console.warn(
        `[menu-sync] override for "${shortcutId}" predates the .code field (legacy saved binding) — ` +
          "leaving native menu accelerator unchanged. Re-record this shortcut to fix.",
      );
    }
  }
  return updates;
}
