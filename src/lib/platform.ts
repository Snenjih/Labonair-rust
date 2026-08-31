import { platform } from "@tauri-apps/plugin-os";

const PLATFORM = (() => {
  try {
    return platform();
  } catch {
    return "";
  }
})();

export const IS_MAC = PLATFORM === "macos";
export const IS_LINUX = PLATFORM === "linux";
export const IS_WINDOWS = PLATFORM === "windows";

/** Custom window controls (min/max/close) are rendered by us only on
 * non-macOS platforms — macOS keeps the native traffic lights via the
 * overlay title bar. */
export const USE_CUSTOM_WINDOW_CONTROLS = !IS_MAC && PLATFORM !== "";

/** Display labels for the alt/ctrl/shift modifier keys, using macOS's
 * native key names (Option/Control) instead of the generic PC names. The
 * underlying values (browser `altKey`/`ctrlKey`/`shiftKey`) are identical
 * on every OS — only the label shown to the user changes. */
export const MODIFIER_KEY_LABELS: Record<"alt" | "ctrl" | "shift", string> = IS_MAC
  ? { alt: "Option (⌥)", ctrl: "Control (⌃)", shift: "Shift (⇧)" }
  : { alt: "Alt", ctrl: "Ctrl", shift: "Shift" };
