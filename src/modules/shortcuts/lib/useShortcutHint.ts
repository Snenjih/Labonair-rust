import { SHORTCUTS, type ShortcutId } from "../shortcuts";
import { useKeybindsStore } from "./useKeybindsStore";

/**
 * Live, override-aware display keys for a ShortcutId — for UI surfaces
 * outside Settings (e.g. Command Palette hint pills). Returns `undefined`
 * (not `[]`) when the shortcut is disabled via override, so callers using a
 * `shortcut && ...` truthy guard correctly hide the hint instead of
 * rendering an empty pill container — `[]` is truthy in JS.
 */
export function useShortcutHint(id: ShortcutId): string[] | undefined {
  const defaultKeys = SHORTCUTS.find((s) => s.id === id)?.keys ?? [];
  const keys = useKeybindsStore((s) => s.getEffectiveDisplayKeys(id, defaultKeys));
  return keys.length > 0 ? keys : undefined;
}
