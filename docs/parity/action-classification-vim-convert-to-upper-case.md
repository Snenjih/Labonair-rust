# Vim ConvertToUpperCase Action Classification

**Status:** Static Vim/Helix direct routes, Unicode transform, operator distinction, and local owner gap classified; runtime and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns case transforms, selections, and document edits; Keymap owns shared profile and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers `vim::ConvertToUpperCase` and the separate `vim::PushUppercase` operator action. The Vim/Helix profile binds `shift-u` to the direct action in Vim Visual and Alt-backtick in Helix Normal/Select. Those shared profile rows do not prove active profile loading or native dispatch. No platform-default map binds either Vim action; platform keymaps' similarly named `editor::ConvertToUpperCase` entries are distinct.

The direct action is separate from Vim Normal `g shift-u{motion}`, which starts `PushUppercase` and waits for a motion or object. Both registered Vim actions have the generic focused-Editor Command Palette route when Vim/Helix is enabled, but palette discovery and invocation remain to be verified. `vim::ChangeCase` on `~` toggles case, and `g ~{motion}` starts the separate `PushOppositeCase` operator.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-ConvertToUpperCase](registered-actions-unbound.md) | Registered unit action that applies full Unicode uppercase mapping to mode-derived Editor ranges. | Labonair has no direct uppercase action, selection transform, or Vim palette route. |
| [KEYBIND-VIM-HELIX-faaae7fbadea](keymap-overlay-requirements.md) | Shared `shift-u` binding in Vim Visual, transforming the active characterwise, linewise, or blockwise selection. | Labonair has no Vim Visual uppercase binding or selected-range transform. |
| [KEYBIND-VIM-HELIX-221f4f535f5d](keymap-overlay-requirements.md) | Shared Alt-backtick binding in Helix Normal/Select when no menu is active; transforms a selection or the character at an empty cursor. | Labonair has no Helix profile or direct uppercase selection/cursor action. |
| [KEYBIND-VIM-HELIX-b799ed330773](keymap-overlay-requirements.md) | Related Vim Normal `g shift-u` starts `PushUppercase`, an operator route completed by a motion/object. This is not a direct ConvertToUpperCase binding. | Labonair does not implement this typed operator/motion workflow; its local `~` toggle is a different operation. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate shared profile bindings and Vim palette discovery; no uppercase-specific setting exists. | Labonair has partial Editor-owned Vim support and no Helix profile. |

## Pinned behavior

The direct action uses the shared `manipulate_text` path, consuming count and forced-motion state and transforming mode-derived Editor ranges in a transaction. Each character is mapped through Rust's full `to_uppercase` conversion; one scalar may expand to multiple scalars, such as `ß` to `SS`. Selection, cursor, and resulting-mode behavior follows the [ConvertToLowerCase action](action-classification-vim-convert-to-lower-case.md): Vim Visual transforms the selected range and collapses to its start before requesting Normal; Helix Normal transforms a selection or cursor character and remains Helix Normal; Helix Select transforms its selection and requests Normal, which remaps to Helix Normal when Helix is enabled. A direct palette invocation from temporary Normal mode returns to Insert. The direct action's count does not widen a Vim Visual selection.

`PushUppercase` is a different action that pushes the uppercase operator. Normal motion and object dispatch apply the same transform to the requested range. Its shared `g shift-u` binding is in Vim Normal. Do not conflate this with `g u` lowercasing, `g ~` case toggling, or the direct Visual/Helix actions.

The pinned `test_convert_to_upper_case` fixture covers characterwise, linewise, and blockwise Visual selections with accented letters, punctuation, and a non-BMP symbol. `test_change_case_motion` and `test_change_case_motion_object` exercise the operator path. `test_change_case_helix_mode` covers direct Alt-backtick conversion at an empty Helix cursor and confirms Helix Normal remains active. These fixtures were inspected, not run; no focused fixture checks a multi-scalar uppercase expansion.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns future mode profiles and user overrides, Settings owns Vim/Helix mode values, and the existing Command Palette remains the discovery surface. No uppercase-specific setting, notification, event, registry, dependency, shell-wide conditional, or new UI control is needed.

Labonair's Editor has no direct uppercase transform, Vim Visual or Helix action, PushUppercase operator, or shared profile route. Its cursor-local `~` action is a case toggle: it acts on the cursor and count rather than the active selection, and takes only the first scalar from each case mapping, so expansions such as `ß` to `SS` are truncated.

Converted text remains ordinary Editor document content and uses the standard edit/undo/save lifecycle. The action introduces no persistence of its own.

## Acceptance still required

- Verify Vim Visual `shift-u` for characterwise, linewise, and blockwise ranges, counts, multiple selections, empty ranges, and the Normal-mode transition. Confirm transformed ranges, cursor placement, selection state, undo, and redo.
- Verify Helix Normal and Select Alt-backtick dispatch, with and without a selection, menus active/inactive, and mode transitions. Capture the resulting mode, cursor, and selection.
- Verify direct Command Palette discovery and invocation in Normal, Visual, Helix Normal, Helix Select, Insert, Replace, and temporary Normal states. Capture text, cursor, selection, focus, mode, count, forced-motion state, and no-op/read-only behavior; confirm temporary Normal returns to Insert.
- Verify `g shift-u{motion}` and `g shift-u` plus text objects separately from the direct action. Include counts, forced-motion state, multiple cursors, cancellation, and undo grouping.
- Exercise expanding uppercase mappings such as `ß` to `SS`, uncased symbols, combining sequences, supplementary-plane characters, mixed scripts, and ranges spanning line endings. Confirm resulting offsets, selection anchors, cursor positions, and undo/redo remain valid after expansion.
- Dispatch shared Visual and Helix rows on macOS, Linux, and Windows, including profile settings and user overrides. Confirm similarly named `editor::ConvertToUpperCase` platform bindings do not dispatch the Vim action.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, mode, selection, cursor, theme, and scale recorded.
- Implement the typed direct transform and optional operator path within `labonair-editor`; add only justified profile entries through `labonair-keymap` and keep `~` as the distinct case-toggle command.

## Source records inspected

- Action registration and direct conversion: `zed-refrence/zed/crates/vim/src/normal.rs` and `zed-refrence/zed/crates/vim/src/normal/convert.rs`.
- PushUppercase registration and operator dispatch: `zed-refrence/zed/crates/vim/src/vim.rs`, `zed-refrence/zed/crates/vim/src/normal.rs`, and `zed-refrence/zed/crates/vim/src/state.rs`.
- Visual, Helix, and Normal operator bindings: `zed-refrence/zed/assets/keymaps/vim.json`.
- Visual, motion/object, and Helix fixtures: `zed-refrence/zed/crates/vim/src/normal/convert.rs`.
- Labonair cursor-local case toggle: `crates/editor/src/vim.rs`; the first-scalar Unicode limitation is recorded in `memory/bugs_and_fixes.md`.
- Linux Vim action registration evidence: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence and does not claim runtime acceptance.
