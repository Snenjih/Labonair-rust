import type { Shortcut, ShortcutId } from "../shortcuts";
import type { KeyBinding, KeyBindingMap } from "../types";
import { bindingMatchesEvent } from "./captureKeyBinding";
import { RESERVED_ACCELERATORS } from "./reservedAccelerators";

function syntheticEvent(b: KeyBinding): KeyboardEvent {
  return {
    key: b.key,
    metaKey: b.meta,
    ctrlKey: b.ctrl,
    shiftKey: b.shift,
    altKey: b.alt,
  } as KeyboardEvent;
}

export type ConflictResult =
  | { kind: "shortcut"; id: ShortcutId; label: string }
  | { kind: "reserved"; label: string }
  | null;

export function findConflict(
  newBinding: KeyBinding,
  excludeId: string,
  shortcuts: Shortcut[],
  overrides: KeyBindingMap,
): ConflictResult {
  const synthetic = syntheticEvent(newBinding);

  for (const reserved of RESERVED_ACCELERATORS) {
    if (reserved.match(synthetic)) return { kind: "reserved", label: reserved.label };
  }

  for (const s of shortcuts) {
    if (s.id === excludeId) continue;
    const override = overrides[s.id];
    if (override === undefined) {
      if (s.match(synthetic)) return { kind: "shortcut", id: s.id as ShortcutId, label: s.label };
    } else if (override !== null) {
      if (bindingMatchesEvent(override as KeyBinding, synthetic)) {
        return { kind: "shortcut", id: s.id as ShortcutId, label: s.label };
      }
    }
  }
  return null;
}
