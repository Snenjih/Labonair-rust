# Vim ConvertToRot13 Action Classification

**Status:** Static ASCII transform, mode-derived ranges, Visual direct binding, and Normal operator distinction classified; runtime and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns text transforms, selections, and document edits; Keymap owns shared profile and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers <code>vim::ConvertToRot13</code>. It has no binding in the macOS, Linux, or Windows platform-default maps. The shared <code>vim.json</code> layer binds <code>g ?</code> to ConvertToRot13 in Vim Visual mode. In Vim Normal, the same chord starts the separate <code>vim::PushRot13</code> operator. There is no active Helix direct key binding for ConvertToRot13. Registered Vim actions also have a focused-Editor Command Palette route when Vim/Helix is enabled; profile declarations and action registration do not prove native dispatch.

The ROT13 action maps ASCII Latin letters only, preserving letter case and leaving all other characters unchanged. ROT47 is a separate transform; its candidate <code>g ?</code> profile rows are commented out and inactive.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-ConvertToRot13](registered-actions-unbound.md) | Registered unit action applying ASCII ROT13 to a mode-derived Editor range. | Labonair has no ROT13 transform action, selection route, or palette entry. |
| [KEYBIND-VIM-HELIX-45608077ee29](keymap-overlay-requirements.md) | Shared Vim Visual <code>g ?</code> binding invokes the direct ConvertToRot13 action on the selection. | Labonair has no Vim Visual ROT13 binding or range conversion. |
| [KEYBIND-VIM-HELIX-9ee8c30586f4](keymap-overlay-requirements.md) | Shared Vim Normal <code>g ?</code> binding starts PushRot13, which waits for a motion/object. It is not a direct action binding. | Labonair has no typed ROT13 operator or motion workflow. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate shared profile bindings and Vim palette discovery; no ROT13-specific setting exists. | Labonair has partial Editor-owned Vim support and no Helix profile. |

## Pinned behavior

The direct action records the current action and visual marks, consumes the pending count (default one) and forced-motion state, then transforms each active range in an Editor transaction. Its mapping is <code>A-M</code> to <code>N-Z</code>, <code>N-Z</code> to <code>A-M</code>, and the equivalent lowercase ranges. Digits, punctuation, whitespace, accented letters, emoji, and other non-ASCII characters are unchanged. ROT13 preserves case and maps each character to exactly one character.

The direct action uses the same mode-derived ranges and cursor transitions as the shared <code>manipulate_text</code> path described in the [ConvertToLowerCase classification](action-classification-vim-convert-to-lower-case.md): Visual/Visual Line transforms each selected range and collapses to its start; Visual Block transforms block-derived ranges and records one cursor at the first range start; Helix Normal transforms a selected range or one character at an empty cursor and remains Helix Normal; Helix Select transforms its selection or one character and requests Normal, which resolves to Helix Normal when the Helix setting is enabled. Direct palette invocation in Normal/Insert/Replace uses the count from each cursor on the current line; temporary Normal mode returns to Insert after a Normal request. Visual selection bounds, not count, define the direct Visual range.

The Vim Visual fixture <code>test_convert_to_rot13</code> covers characterwise, linewise, and blockwise selection. Its input includes an emoji, accented É, punctuation, and a digit; those characters remain unchanged while ASCII letters rotate. The separate <code>test_change_rot13_motion</code> and <code>test_change_rot13_object</code> fixtures exercise the Normal operator with motions and objects. No direct Helix transform fixture was found. Source fixtures were inspected, not run.

Normal <code>g ?{motion/object}</code> is the PushRot13 operator route. It derives a range from the following motion/object and invokes the shared conversion path; tests for it do not stand in for Visual <code>g ?</code> or direct palette invocation. The active keymap has no ROT47 <code>g ?</code> route; the commented candidate does not count as a binding.

Direct palette invocation in Helix Select, Insert, Replace, or temporary Normal uses the same mode-dependent selection/cursor logic. A no-op range or read-only Editor can still reach the helper's mode-transition step; resulting text, selection, and mode need runtime acceptance.

## Labonair owner and current route

The canonical behavior owner is <code>labonair-editor</code>; <code>labonair-keymap</code> owns future mode bindings and user overrides, Settings owns Vim/Helix mode values, and the existing Command Palette remains the discovery surface. No ROT13-specific setting, notification, event, registry, dependency, shell-wide conditional, or new UI control is needed.

Labonair's Editor has no ROT13 transform, Vim Visual <code>g ?</code> route, Normal <code>g ?</code> motion operator, Helix entry, or Command Palette action. Its cursor-local <code>~</code> route performs a different case toggle.

ROT13 edits remain ordinary Editor document content and use the standard save/undo lifecycle. There is no transform-specific persistence.

## Acceptance still required

- Verify Visual <code>g ?</code> with characterwise, linewise, and blockwise selections, multiple cursors, counts, empty ranges, boundaries, no-op ranges, read-only buffers, and the resulting cursor/mode. Confirm that Visual count does not widen the selected range.
- Verify Normal <code>g ?{motion/object}</code> independently, including operator counts, motions, objects, repeat, cursor restoration, and undo grouping.
- Verify focused-Editor palette discovery and invocation in Normal, Visual, Helix Normal, Helix Select, Insert, Replace, and temporary Normal states. Capture text, selection, cursor, focus, mode, count, and forced-motion state.
- Exercise ASCII alphabet boundaries, upper/lower case, non-ASCII scripts, accented letters, emoji, digits, punctuation, whitespace, multi-selections, and multiline selections. Confirm non-ASCII and uncased characters remain unchanged and text length remains stable.
- Dispatch the shared profile separately on macOS, Linux, and Windows, including mode settings, user overrides, Visual mode entry/exit, and the absent Helix direct key binding.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, mode, selection, cursor, theme, and scale recorded.
- Implement the direct transform and separate Normal motion operator within <code>labonair-editor</code>; add profile routes through <code>labonair-keymap</code> without activating the commented ROT47 candidate.

## Source records inspected

- Action registration, direct transform, mode-derived selection processing, and fixtures: <code>zed-refrence/zed/crates/vim/src/normal.rs</code> and <code>zed-refrence/zed/crates/vim/src/normal/convert.rs</code>.
- Normal operator and Visual direct bindings, plus inactive ROT47 candidates: <code>zed-refrence/zed/assets/keymaps/vim.json</code>.
- Helix and temporary-mode result of Normal requests: <code>zed-refrence/zed/crates/vim/src/vim.rs</code>.
- Linux action registration evidence: <code>docs/parity/evidence/zed-actions-3569541-linux-x86_64.json</code>.
- Labonair action search across Editor sources found no ROT13 route: <code>crates/editor/src</code>.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence and does not claim runtime acceptance.
