# Vim ChangeListNewer Action Classification

**Status:** Static action, shared binding, transient state, and owner-route behavior classified; native dispatch and navigation acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Editor owns edit-position recording, transient change-list state, and selection navigation; Keymap owns profile and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers `vim::ChangeListNewer`. It has
no binding in the macOS, Linux, or Windows platform-default maps. The shared
`vim.json` layer binds `g ,` under `VimControl && !menu`. `VimControl` is
provided in Vim Normal, Visual, Operator, Helix Normal, and Helix Select
contexts. Registered Vim actions also have a static focused-Editor Command
Palette route while Vim/Helix is enabled. These facts do not establish macOS
registration, active profile loading, key dispatch, or native behavior.

The action traverses the Editor's edit-position change list. It is distinct
from document undo/redo, Git changed-line navigation, and Workspace navigation
history: `g ,` restores previously recorded caret positions without reverting
text or navigating between files.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-ChangeListNewer](registered-actions-unbound.md) | Registered unit action that moves Editor selections to a newer recorded edit position; it consumes a Vim count, defaulting to one. | Labonair's Editor has document undo/redo history but no edit-position change list or matching action. |
| [KEYBIND-VIM-HELIX-30d451e3ee88](keymap-overlay-requirements.md) | The shared mode layer binds `g ,` in `VimControl && !menu`; no platform-default binding exists. | Labonair has no shared Vim/Helix profile layer or equivalent `g ,` route. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate the shared layer and Vim palette namespace; there is no change-list-specific setting. | Labonair has an Editor-owned partial Vim preference and no Helix mode/profile layer. |

## Pinned behavior

Editor records selection heads when edits occur. In Insert mode each recorded
head is offset one display position to the left. Each change-list entry keeps
the position for every active selection. An edit joins the last group when
the selection count matches and each old position is on the current selection
head's row; a grouped edit replaces that group's current positions. A new
entry resets the current navigation index. Vim also updates the `.` mark when
an edit is added; `ChangeListNewer` itself does not change that mark.

`ChangeListNewer` consumes the Vim count (default one) and any forced-motion
state, then asks the Editor's transient list for the next position in the
newer direction. With no current index, traversal starts from one past the
list's end, so the first newer request resolves to the newest recorded group.
Further requests move toward newer entries and clamp at the newest group.
The saved anchors become point selections, restoring one caret per recorded
selection. If the list is empty, the Editor leaves its current selections
unchanged and sends no message. Navigation does not edit the buffer or change
the active Vim/Helix mode.

The source fixtures `test_change_list_insert` and `test_change_list_delete`
exercise insertion, deletion, older/newer movement, and a count. They were
inspected but not run. The fixtures do not cover multi-cursor groups,
same-row grouping, Insert-mode anchor offset, or adding an edit after
navigation.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; Keymap owns future shared
mode bindings and user overrides, Settings owns the Vim/Helix mode values,
and the existing Command Palette remains the action discovery surface. No
change-list setting, notification, new event, registry, or shell-wide
conditional is needed for this navigation action.

`crates/editor/src/history.rs` and `Document` provide transaction undo/redo
history. `crates/editor/src/git.rs` provides changed-line navigation. Neither
stores prior selection-head positions for Vim navigation. The local
Editor-owned Vim parser has no change-list state, action, or `g ,` binding;
there is no corresponding Keymap profile or palette descriptor. The pinned
behavior therefore has no local route today. This inventory records the gap
without changing runtime code.

Change-list state is transient Editor state and has no independent
persistence. Existing document edits continue through normal document
persistence and undo history.

## Acceptance still required

- Verify the shared `g ,` row with Vim and Helix modes on and off, including
  inactive contexts and user-binding precedence, save, and reload. Repeat
  dispatch separately on macOS, Linux, and Windows.
- Verify focused-Editor palette discovery and invocation in Normal, Visual,
  Operator, Helix Normal, and Helix Select; capture the resulting positions,
  selection count, mode, and focus.
- Exercise empty and populated lists, the first newer request, repeated
  requests at the newest entry, older-then-newer traversal, count behavior,
  multi-cursor positions, grouped same-row edits, Insert-mode offset, and an
  edit after navigation.
- Confirm selection restoration as point carets and no-op behavior when the
  list is empty. Verify that navigation changes neither text nor the `.` mark.
- Capture paired native macOS Editor and Keymap evidence with platform,
  profile, mode, selections, cursor, viewport, theme, and scale recorded.
- Implement the state and typed action through `labonair-editor`; keep the
  list transient and do not conflate it with document undo or Git line
  navigation.

## Source records inspected

- Action registration, handler, state capture, and source fixtures:
  `zed-refrence/zed/crates/vim/src/change_list.rs`.
- Editor `ChangeList` anchor storage and traversal:
  `zed-refrence/zed/crates/editor/src/editor.rs`.
- Vim control-context construction and Edited-event hook:
  `zed-refrence/zed/crates/vim/src/vim.rs`.
- Shared `g ,` profile row: `zed-refrence/zed/assets/keymaps/vim.json`.
- Linux action registration evidence:
  `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Local Vim parser, document history, and changed-line navigation:
  `crates/editor/src/vim.rs`, `crates/editor/src/history.rs`, and
  `crates/editor/src/git.rs`.

Same-session pinned-source research and independently written requirements
are redevelopment, not separated-room clean-room implementation. This packet
records static evidence and does not claim runtime acceptance.
