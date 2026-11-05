# Vim ChangeListOlder Action Classification

**Status:** Static action, shared binding, transient state, and owner-route behavior classified; native dispatch and navigation acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Editor owns edit-position recording, transient change-list state, and selection navigation; Keymap owns profile and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers `vim::ChangeListOlder`. It has
no binding in the macOS, Linux, or Windows platform-default maps. The shared
`vim.json` layer binds `g ;` under `VimControl && !menu`. `VimControl` is
provided in Vim Normal, Visual, Operator, Helix Normal, and Helix Select
contexts. Registered Vim actions also have a static focused-Editor Command
Palette route while Vim/Helix is enabled. These facts do not establish macOS
registration, active profile loading, key dispatch, or native behavior.

The action traverses the Editor's edit-position change list. It is distinct
from document undo/redo, Git changed-line navigation, and Workspace navigation
history: `g ;` restores previously recorded caret positions without reverting
text or navigating between files. `vim::ChangeListNewer` is the paired action
that traverses in the opposite direction.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-ChangeListOlder](registered-actions-unbound.md) | Registered unit action that moves Editor selections to older recorded edit positions; it consumes a Vim count, defaulting to one. | Labonair's Editor has document undo/redo history but no edit-position change list or matching action. |
| [KEYBIND-VIM-HELIX-e9e063fe4820](keymap-overlay-requirements.md) | The shared mode layer binds `g ;` in `VimControl && !menu`; no platform-default binding exists. | Labonair has no shared Vim/Helix profile layer or equivalent `g ;` route. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate the shared layer and Vim palette namespace; there is no change-list-specific setting. | Labonair has an Editor-owned partial Vim preference and no Helix mode/profile layer. |

## Pinned behavior

The state is shared with `vim::ChangeListNewer`: Editor edit events record
selection heads as anchor groups, offset one display position left in Insert
mode. A new edit may update the latest group when its selection count matches
and each anchor remains on the corresponding current head's row; otherwise it
adds a group. Adding an edit resets the current navigation index. Vim updates
the `.` mark when recording an edit, not when this action navigates.

`ChangeListOlder` consumes the Vim count (default one) and any forced-motion
state, then asks the Editor's transient list for the previous position in the
older direction. With no current index, traversal begins one past the list's
end; the first request therefore selects the newest group for count one. A
larger count subtracts farther into the list. Further requests move toward
older entries, and saturating subtraction clamps at the oldest group. The
saved anchors become point selections, restoring one caret per recorded
selection. If the list is empty, the Editor leaves its current selections
unchanged and sends no message. Navigation does not edit the buffer or change
the active Vim/Helix mode.

Because both directions share the same unset-index origin, the first `g ;`
and first `g ,` each resolve to the newest group. After moving older, `g ,`
moves forward from the selected group; after moving newer, `g ;` moves back.

The source fixtures `test_change_list_insert` and `test_change_list_delete`
exercise older movement, the newer counterpart, and a count. They were
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
Editor-owned Vim parser has no change-list state, action, or `g ;` binding;
there is no corresponding Keymap profile or palette descriptor. The pinned
behavior therefore has no local route today. This inventory records the gap
without changing runtime code.

Change-list state is transient Editor state and has no independent
persistence. Existing document edits continue through normal document
persistence and undo history.

## Acceptance still required

- Verify the shared `g ;` row with Vim and Helix modes on and off, including
  inactive contexts and user-binding precedence, save, and reload. Repeat
  dispatch separately on macOS, Linux, and Windows.
- Verify focused-Editor palette discovery and invocation in Normal, Visual,
  Operator, Helix Normal, and Helix Select; capture the resulting positions,
  selection count, mode, and focus.
- Exercise empty and populated lists, first older movement from an unset
  index, count traversal, older movement at the oldest entry, older-then-newer
  and newer-then-older traversal, multi-cursor positions, grouped same-row
  edits, Insert-mode offset, and an edit after navigation.
- Confirm selection restoration as point carets and no-op behavior when the
  list is empty. Verify navigation changes neither text nor the `.` mark.
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
- Shared `g ;` profile row: `zed-refrence/zed/assets/keymaps/vim.json`.
- Linux action registration evidence:
  `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Local Vim parser, document history, and changed-line navigation:
  `crates/editor/src/vim.rs`, `crates/editor/src/history.rs`, and
  `crates/editor/src/git.rs`.

Same-session pinned-source research and independently written requirements
are redevelopment, not separated-room clean-room implementation. This packet
records static evidence and does not claim runtime acceptance.
