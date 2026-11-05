# Vim EndRepeat Action Classification

**Status:** Dot-repeat completion sentinel classified; runtime and direct-invocation acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor/Vim owns recorded actions, replay state, and mode transitions; Keymap owns the user-facing `.` route to `vim::Repeat`.

## Evidence boundary

The pinned Linux action registry declares `vim::EndRepeat` as a unit action with no arguments or deprecated aliases. It has no direct binding in the pinned keymaps. The user-facing `.` binding invokes the separate `vim::Repeat` action; that action appends `EndRepeat` to its recorded dot-repeat sequence. The distinct macro actions are `vim::ToggleRecord` and `vim::ReplayLastRecording`.

The generated action description says “Ends the repeat recording,” while the inspected handler uses this action to finish dot-repeat playback. Whether the action is exposed by the command palette or is safe to invoke outside a replay remains unverified.

## Pinned completion behavior

When the replayer dispatches `EndRepeat`, its handler clears `VimGlobals.dot_replaying` and switches Vim to Normal mode. For a temporary-mode replay, `vim::InsertBefore` is appended after the sentinel to restore the insert path. The action is a completion marker in the dot-repeat pipeline, not the action that records or replays a macro.

The `Replayer` processes recorded actions in order and schedules the next action after each dispatch. If focus moves away and the sentinel is not handled, the exhaustion path takes and clears the replayer and resets `dot_replaying` to false. It also aborts after 10,000 replay steps and reaches this reset path, preventing a stuck replay flag. That fallback does not dispatch the `EndRepeat` handler's mode transition, so focus and mode recovery still require runtime acceptance.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the `.` profile route. The local `crates/editor/src/vim.rs` has no `vim::Repeat`, replay action sequence, replay sentinel, or replay-state cleanup path. No new dependency, event, registry, notification, setting, or UI control is needed to classify this action.

The classified user entry flow is `.` → `vim::Repeat` → replayed changes → internal `vim::EndRepeat`. This packet classifies only the terminal action and recovery contract; the full `vim::Repeat` behavior remains its own registered-action requirement.

## Pinned fixtures inspected

- `normal/repeat.rs::test_dot_repeat` covers repeating edits, motions, paste, changes, counts, and a macro-related last-change route.
- `normal/repeat.rs::test_repeat_completion` covers completion behavior during replay.
- `normal/repeat.rs::test_repeat_after_blur_resets_dot_replaying` forces focus away before `EndRepeat` can run and checks the fallback reset.
- `normal/repeat.rs::test_undo_repeated_insert` checks that one undo restores the document after a counted repeated insertion.

These pinned-source fixtures were inspected, not run, and are not Labonair acceptance evidence. They do not establish action-palette exposure, direct invocation behavior, all mode transitions, cancellation, or paired native acceptance.

## Acceptance still required

- Verify the `.` → `Repeat` route and EndRepeat dispatch in Vim Normal/Visual/Visual Line/Visual Block and Helix Normal/Select, including count, register, recorded-selection, and temporary Insert-mode replay.
- Verify replay completion returns to the expected mode, clears `dot_replaying`, releases the replayer, and finalizes one coherent undo transaction. Check repeat after edits, motions, paste, macro changes, zero recorded actions, cancellation, and failed action dispatch.
- Move focus to search, a palette, another workspace item, and a closed/detached Editor during replay; verify the fallback clears state without leaving a stuck mode or corrupting later repeats. Exercise the 10,000-step abort path safely.
- Check whether the command palette exposes `EndRepeat`; if it does, verify and specify its behavior when no replay is active and while a replay is active.
- Dispatch `.` and the replay completion path on Linux and Windows. Capture paired native macOS Editor and Keymap evidence with mode, recorded change, count/register, focus destination, resulting document, theme, and scale recorded.

## Source records inspected

- Action schema: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- `EndRepeat` registration, replay sequence insertion, completion handler, fallback, and tests: `zed-refrence/zed/crates/vim/src/normal/repeat.rs`.
- User-facing `.` profile rows: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Local Vim owner: `crates/editor/src/vim.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
