# Vim ConvertToLowerCase Action Classification

**Status:** Static action, mode-dependent ranges, shared bindings, Unicode transform, and local owner gap classified; runtime and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns case transforms, selections, and document edits; Keymap owns shared profile and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers <code>vim::ConvertToLowerCase</code>. It has no binding in the macOS, Linux, or Windows platform-default maps. The shared Vim/Helix profile binds <code>u</code> in Vim Visual and backtick in Helix Normal/Select. Registered Vim actions also have a focused-Editor Command Palette route when Vim/Helix is enabled. Shared profile declarations do not prove active profile loading or native dispatch.

The direct action is distinct from Vim Normal <code>g u{motion}</code>, which starts the <code>vim::PushLowercase</code> operator and waits for a motion or object. It is also distinct from <code>vim::ChangeCase</code> on <code>~</code>, which toggles case. The similarly named <code>editor::ConvertToLowerCase</code> is a separate Editor action used by non-Vim platform profiles; its bindings do not establish a platform-default binding for this Vim action.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-ConvertToLowerCase](registered-actions-unbound.md) | Registered unit action that lowercases a mode-derived selection or cursor range using Rust's Unicode lowercase mapping. | Labonair has no direct lower-case Editor action or selected-range transform. Its local <code>~</code> route toggles case at the cursor and is a different operation. |
| [KEYBIND-VIM-HELIX-5bda91ec3ec4](keymap-overlay-requirements.md) | Shared <code>u</code> binding in Vim Visual mode, including characterwise, linewise, and blockwise visual selections. | Labonair has no Vim Visual lowercase binding or range conversion. |
| [KEYBIND-VIM-HELIX-73e43d83e3b1](keymap-overlay-requirements.md) | Shared backtick binding in Helix Normal/Select when no menu is active; lowercases the current selection or the character at an empty cursor. | Labonair has no Helix profile or cursor/selection lower-case action. |
| [KEYBIND-VIM-HELIX-b1bda7e33a63](keymap-overlay-requirements.md) | Related Vim Normal <code>g u</code> entry starts <code>PushLowercase</code>, an operator route completed by a motion/object. This is not a direct binding for ConvertToLowerCase. | Labonair does not implement this typed operator/motion workflow; its local <code>~</code> action does not supply it. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate shared profile bindings and Vim palette discovery; no case-conversion setting exists. | Labonair has partial Editor-owned Vim support and no Helix profile. |

## Pinned behavior

The action records the current action and visual marks, consumes the pending count (default one) and forced-motion state, then edits all active ranges in an Editor transaction. Each character is mapped with Rust's full lowercase conversion; a Unicode mapping can produce more than one scalar. The transform does not itself toggle characters back to uppercase. For Visual ranges, count does not expand the selection. In Normal/Insert/Replace, count determines the number of characters transformed from each cursor on the current line.

| Mode | Range transformed | Selection/cursor and resulting mode |
|---|---|---|
| Vim Normal direct invocation | Count characters from each cursor on the current line; default one. The action is not bound to a Vim Normal key in the shared profile, but can be invoked through the focused Editor palette. | Cursor advances to the transformed range end, except a range reaching line end leaves it on the final character. Vim Normal remains active unless temporary Normal mode is active, in which case the transition returns to Insert. |
| Vim Visual / Visual Line | Each active selected range. | Selections collapse to their starts; the handler requests Vim Normal. |
| Vim Visual Block | Each block-derived range. | The handler records one cursor at the first range start, then requests Vim Normal. |
| Helix Normal | A non-empty selection, or one character at an empty cursor. | A non-empty selection remains selected; an empty cursor stays at that character. Helix Normal remains active. |
| Helix Select | Each selected range, or one character for an empty selection. | The handler requests Mode::Normal after the transform; switch_mode remaps that request to Helix Normal when Helix mode remains enabled. No focused Helix Select fixture was found. |
| Insert / Replace direct invocation | Count characters from each selection start on the current line. | Uses the Normal cursor handling and requests Mode::Normal; switch_mode remaps that request to Helix Normal when Helix mode is enabled. When temporary Normal mode is active, switch_mode instead exits back to Insert. No direct profile binding exposes these modes; palette behavior needs acceptance. |

The Visual fixture <code>test_convert_to_lower_case</code> covers characterwise, linewise, and blockwise selection and multibyte text. It does not establish one-to-many lowercase expansion. The Helix fixture <code>test_change_case_helix_mode</code> exercises the backtick action at an empty cursor in Helix Normal and confirms the mode remains Helix Normal; it does not cover Helix Select or an explicit range for this action. The separate <code>test_change_case_motion</code> exercises <code>g u</code> as a lowercase operator. These source fixtures were inspected, not run.

The generic Vim <code>manipulate_text</code> path requests Mode::Normal after Visual, Visual Line, Visual Block, Helix Select, Insert, and Replace actions even when conversion makes no text change. <code>Vim::switch_mode</code> remaps that request to Helix Normal when the Helix mode setting remains enabled; Helix Normal stays active directly. Its temporary-mode special case returns to Insert after a Normal request when temporary Normal mode is active. Direct palette invocation therefore performs a text transform according to the current selection and mode rather than acting as a passive command entry. Read-only/no-op results, focus, temporary-mode return, and mode presentation need runtime acceptance.

## Labonair owner and current route

The canonical behavior owner is <code>labonair-editor</code>; <code>labonair-keymap</code> owns the future shared mode bindings and user overrides, Settings owns Vim/Helix mode values, and the existing Command Palette remains the discovery surface. No lower-case-specific setting, notification, event, registry, dependency, shell-wide conditional, or new UI control is needed.

Labonair's <code>crates/editor/src/vim.rs</code> supports <code>~</code> as a cursor-local case toggle using the count. It does not convert an active Visual selection, expose a lowercase-only typed action, provide the Vim Visual <code>u</code> binding, or implement the Helix backtick profile. Its per-character toggle takes only the first scalar from Rust's case mapping; this separate expansion gap is already recorded in <code>memory/bugs_and_fixes.md</code>. No direct lower-case action or selected-range transform was found in <code>crates/editor</code>.

Converted text remains ordinary Editor document content and uses the standard edit/undo/save lifecycle. The action introduces no persistence of its own.

## Acceptance still required

- Verify Vim Visual <code>u</code> for characterwise, linewise, and blockwise ranges, counts, multiple selections, boundaries, empty ranges, and the Normal-mode transition. Confirm the selected text changes once and each resulting cursor/selection matches the expected range start.
- Verify Helix Normal and Select backtick dispatch, with and without a selection, with menus active/inactive, and with mode transitions. Confirm Helix Normal remains active and determine Helix Select behavior.
- Verify direct Command Palette discovery and invocation in Normal, Visual, Helix Normal, Helix Select, Insert, Replace, and temporary Normal from Insert. Capture text, cursor, selection, focus, mode, count, and forced-motion state, including read-only and no-op cases; confirm temporary Normal returns to Insert.
- Exercise Unicode lower-case mappings that expand to multiple scalars, uncased symbols, combining sequences, supplementary-plane characters, mixed scripts, and selections spanning line endings. Confirm offsets and undo/redo remain valid after expansion.
- Verify <code>g u{motion}</code> separately from direct ConvertToLowerCase, and distinguish both from the <code>~</code> ChangeCase toggle. Include counts, object/motion ranges, multiple cursors, and undo grouping.
- Dispatch shared Visual and Helix rows separately on macOS, Linux, and Windows, including profile settings and user overrides.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, mode, selection, cursor, theme, and scale recorded.
- Implement the typed lower-case action, mode-aware selection/cursor transform, and shared Vim/Helix bindings within <code>labonair-editor</code> and <code>labonair-keymap</code>; retain <code>~</code> as the distinct case-toggle command.

## Source records inspected

- Action registration and mode-specific case handlers: <code>zed-refrence/zed/crates/vim/src/normal.rs</code> and <code>zed-refrence/zed/crates/vim/src/normal/convert.rs</code>.
- Helix and temporary-mode Normal remapping: <code>zed-refrence/zed/crates/vim/src/vim.rs</code>.
- Shared Vim Normal operator, Visual <code>u</code>, and Helix backtick profile rows: <code>zed-refrence/zed/assets/keymaps/vim.json</code>.
- Visual conversion, Helix cursor conversion, and separate <code>g u</code> operator fixtures: <code>zed-refrence/zed/crates/vim/src/normal/convert.rs</code>.
- Editor Unicode conversion implementation: <code>zed-refrence/zed/crates/editor/src/editor.rs</code>.
- Labonair cursor-local toggle: <code>crates/editor/src/vim.rs</code>; existing local Unicode finding: <code>memory/bugs_and_fixes.md</code>.
- Linux Vim action registration evidence: <code>docs/parity/evidence/zed-actions-3569541-linux-x86_64.json</code>.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence and does not claim runtime acceptance.
