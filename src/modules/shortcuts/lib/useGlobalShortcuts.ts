import { useEffect, useRef } from "react";
import { SHORTCUTS, type ShortcutId } from "../shortcuts";
import { useKeybindsStore } from "./useKeybindsStore";

export type ShortcutHandler = (e: KeyboardEvent) => void;
export type ShortcutHandlers = Partial<Record<ShortcutId, ShortcutHandler>>;

export type UseGlobalShortcutsOptions = {
  isDisabled?: (id: ShortcutId, e: KeyboardEvent) => boolean;
};

/** True when the given element is a plain text-entry control (`<input>`/
 *  `<textarea>`) — e.g. `TabRenameInput`. Deliberately does NOT include
 *  `[contenteditable]`: CodeMirror's editor surface is contenteditable, and
 *  shortcuts like `search.focus` (⌘F) legitimately need to keep firing while
 *  the cursor is in the code editor. Every entry in `SHORTCUTS` requires a
 *  modifier key (none bind a bare key or Escape), so restricting this guard
 *  to literal text inputs can't accidentally block a shortcut that's meant
 *  to work while the user is typing plain text.
 *
 *  Explicitly excludes xterm.js's hidden `.xterm-helper-textarea` — a real
 *  `<textarea>` xterm keeps focused offscreen to capture raw keyboard/IME
 *  input whenever a terminal pane has focus. Without this exclusion every
 *  global shortcut (tab switching included) would silently stop matching
 *  the moment a terminal is focused, since the naive `HTMLTextAreaElement`
 *  check can't tell it apart from a real text field. */
export function isPlainTextInputFocused(el: Element | null): boolean {
  if (el instanceof HTMLTextAreaElement && el.classList.contains("xterm-helper-textarea")) return false;
  return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement;
}

export function useGlobalShortcuts(handlers: ShortcutHandlers, options?: UseGlobalShortcutsOptions) {
  const latest = useRef({ handlers, options });
  latest.current = { handlers, options };

  const matchesShortcut = useKeybindsStore((s) => s.matchesShortcut);
  const overrides = useKeybindsStore((s) => s.overrides);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isPlainTextInputFocused(document.activeElement)) return;
      const { handlers, options } = latest.current;
      for (const s of SHORTCUTS) {
        if (!matchesShortcut(s.id, s.match, e)) continue;
        if (options?.isDisabled?.(s.id, e)) return;
        const h = handlers[s.id];
        if (!h) return;
        e.preventDefault();
        e.stopImmediatePropagation();
        h(e);
        return;
      }
    };
    window.addEventListener("keydown", onKey, { capture: true });
    return () => window.removeEventListener("keydown", onKey, { capture: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [overrides]);
}
