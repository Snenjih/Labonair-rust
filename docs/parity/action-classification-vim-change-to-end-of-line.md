# Vim ChangeToEndOfLine Action Classification

**Status:** Static action, shared binding, text-change path, and local partial route classified; direct-action and native acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Editor owns text changes, selections, Vim register, and modal state; Keymap owns profile and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers `vim::ChangeToEndOfLine`. It has
no binding in the macOS, Linux, or Windows platform-default maps. The shared
`vim.json` layer binds `Shift-C` only in `vim_mode == normal`. In Helix Normal
and Select, the same chord maps to `vim::HelixDuplicateBelow`; that is a
separate action. Registered Vim actions also have a static focused-Editor
Command Palette route while Vim/Helix is enabled. These facts do not establish
macOS registration, profile loading, key dispatch, or native behavior.

`ChangeToEndOfLine` is a direct change operator action. It is separate from
lowercase `c` followed by the `$` motion, and from `vim::DeleteToEndOfLine`
(`Shift-D`). Palette invocation performs the edit at the focused Editor; it
does not merely enter Insert mode as a configuration action would.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-ChangeToEndOfLine](registered-actions-unbound.md) | Registered unit action that changes from the current selection/cursor to the end of one or more buffer lines and enters Insert mode. | Labonair has a partial uppercase `C` parser route, but no registered action or palette descriptor; its count and Vim-register behavior differ. |
| [KEYBIND-VIM-HELIX-4653d4ed0a84](keymap-overlay-requirements.md) | The shared mode layer binds `Shift-C` under `vim_mode == normal`; Helix Normal/Select bind `Shift-C` separately to `HelixDuplicateBelow`. No platform-default binding exists. | Labonair accepts uppercase `C` in its modal parser but has no shared mode profile or typed keybinding entry. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate the shared layer and Vim palette namespace; there is no ChangeToEndOfLine-specific setting. | Labonair has a partial Editor-owned Vim preference and no Helix mode/profile layer. |

## Pinned behavior

The action starts Vim recording, consumes the current count and forced-motion
state, then runs `Motion::EndOfLine { display_lines: false }` through the
change-motion path. The motion targets a buffer-line boundary rather than the
end of a soft-wrapped display row. A count can move the endpoint to the end of
a later buffer row. With no count, the motion defaults to the current line.

The change-motion path expands the active selections to the motion range,
copies their exclusive contents into the Vim register, deletes the ranges
with linked edits, and refreshes edit prediction. For EndOfLine, the handler
sets the change kind to Exclusive before evaluating the motion. It then
requests Vim Insert mode. At an empty range or a cursor already at line end,
there may be no text to delete, but the action still follows the Insert-mode
transition.

The direct action runs through one Editor transaction for the selection and
text update. It does not add a ChangeToEndOfLine-specific setting,
notification, or new UI surface. The effect of read-only buffers and palette
invocation from non-Normal modes still requires runtime acceptance.

The focused source fixture `test_change_end_of_line` exercises `c $` on
non-empty and empty lines. It validates the shared change-motion behavior,
but it does not invoke the registered direct `Shift-C` action. No focused
direct-action test was found; the fixture was inspected but not run.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; Keymap owns future mode
profiles and user overrides, Settings owns the Vim/Helix values, and the
existing Command Palette remains the action discovery surface. No
ChangeToEndOfLine-specific setting, notification, event, registry, or
shell-wide conditional is needed.

In `crates/editor/src/vim.rs`, uppercase `C` reaches `enter_insert`, which
deletes from the current caret to the end of that line and enters Insert mode.
That parser route ignores a pending count, does not copy the removed text to
the Vim register, and has no typed action or palette route. The local `c $`
operator-plus-motion path is separate and must not stand in for direct-action
coverage. The local uppercase-C count and register gaps are recorded in
[`memory/bugs_and_fixes.md`](../../memory/bugs_and_fixes.md); this inventory
does not change runtime code.

Text remains owned by the Editor and uses ordinary document persistence and
undo history. The Vim register and mode are transient Editor state.

## Acceptance still required

- Verify the shared `Shift-C` row in Vim Normal and excluded Vim Visual,
  Operator, Helix Normal, and Helix Select contexts. Confirm that Helix
  `Shift-C` still dispatches to `HelixDuplicateBelow`. Repeat dispatch
  separately on macOS, Linux, and Windows, including user overrides.
- Verify focused-Editor palette discovery and invocation in Normal, Visual,
  Insert, Helix Normal, and Helix Select. Capture text, register content,
  selection/cursor, focus, and resulting mode.
- Exercise absent and explicit counts, multiple buffer lines, soft-wrapped
  lines, empty lines, a cursor at line end, final buffer line, multiple
  selections, read-only buffers, and an edit that produces no text change.
- Confirm the exclusive register contents and Insert-mode transition when the
  selected text is empty or the buffer is read-only. Compare direct `Shift-C`
  separately from `c $`, `Shift-D`, and Helix DuplicateBelow.
- Capture paired native macOS Editor and Keymap evidence with platform,
  profile, mode, selection, cursor, viewport, theme, and scale recorded.
- Complete the local uppercase-C route in `labonair-editor`: preserve count,
  Vim-register, selection, read-only, and mode semantics, then expose one
  typed action and discoverable keymap/palette route through the canonical
  owners.

## Source records inspected

- Direct action registration and change-motion invocation:
  `zed-refrence/zed/crates/vim/src/normal.rs`.
- Change-motion selection, register, edit, and mode transition:
  `zed-refrence/zed/crates/vim/src/normal/change.rs`.
- EndOfLine motion and buffer-row count behavior:
  `zed-refrence/zed/crates/vim/src/motion.rs`.
- Shared Vim Normal Shift-C and Helix DuplicateBelow bindings:
  `zed-refrence/zed/assets/keymaps/vim.json`.
- The `c $` fixture, not a direct Shift-C fixture:
  `zed-refrence/zed/crates/vim/src/normal/change.rs`.
- Linux action registration evidence:
  `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Local uppercase-C parser and change-motion implementation:
  `crates/editor/src/vim.rs`.

Same-session pinned-source research and independently written requirements
are redevelopment, not separated-room clean-room implementation. This packet
records static evidence and does not claim runtime acceptance.
