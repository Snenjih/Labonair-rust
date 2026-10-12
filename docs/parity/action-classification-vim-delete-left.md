# Vim DeleteLeft Action Classification

**Status:** Static Normal-mode deletion route and local uppercase-X partial equivalent classified; register, platform dispatch, and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns Vim motion ranges, delete/register effects, and buffer edits; Keymap owns the shared profile; Settings owns Vim and Helix mode values.

## Evidence boundary

The pinned Linux action registry declares `vim::DeleteLeft` as a unit action with no arguments or deprecated aliases. No platform-default map binds the action. The shared Vim profile binds it only in Vim Normal as `shift-x`. The neighboring Vim Visual route maps `shift-x` to `vim::VisualDeleteLine`; the later Helix Normal/Select context maps it to `editor::SelectLine`. Those are different actions and workflows. Static profile rows do not prove that the profile loads or dispatches on any platform. Palette availability and direct action behavior are also unverified.

## Pinned behavior

The `shift-x` action records the current action, consumes the pending Vim count and forced-motion flag, and calls the common delete path with `Motion::Left`. A missing count means one. Left is an exclusive, characterwise motion. It saturates at the start of the current line, so deleting from column zero does not cross the preceding newline. The action therefore deletes up to the counted number of characters to the left of each active cursor; its range is clipped at the line start.

The shared delete path expands the motion for each selection, copies deleted ranges into the Vim register as a delete, removes the selected ranges in an Editor transaction, and collapses each selection at the motion head after clipping. It does not explicitly switch Vim modes. Existing Visual selections and multiple cursors, direct invocation outside the bound Normal context, register selection, undo grouping, and clipboard interactions require runtime acceptance.

## Pinned fixture inspected

`test_delete_left` exercises `shift-x` at each cursor offset of a single-line word and at the start of the second line in a two-line buffer. The fixture is Neovim-backed and compares behavior; it does not by itself verify count, register contents, multiple selections, direct palette invocation, or native Labonair dispatch. The fixture was inspected, not run.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the shared Vim profile; Settings owns the mode values. No new dependency, event, registry, notification, or shell-wide condition is needed to classify the action.

Labonair's `labonair-editor` Vim parser has an uppercase `X` route that deletes left by the pending count, alongside lowercase `x` deletion to the right. The uppercase-X path edits the local buffer directly and does not update the local Vim register; it is not a typed `vim::DeleteLeft` action or shared Keymap route. The existing count test covers lowercase `x`, not uppercase `X`. The local parser has no separate Visual `shift-x` or Helix selection behavior corresponding to the pinned profile.

## Acceptance still required

- Verify macOS action registration, palette availability and focus restoration, shared Vim profile loading, and Normal `shift-x` dispatch. Verify the distinct Visual `vim::VisualDeleteLine` and Helix `editor::SelectLine` routes, context precedence, menu behavior, and user overrides.
- Verify default count one and explicit counts; deletion at the first character, within a line, at line ends, on empty lines, in empty buffers, and across multibyte characters. Confirm Left never crosses a line boundary.
- Verify the Vim register receives the deleted characterwise text, including multiple cursors and selected-register behavior; check marks, caret position, undo grouping, repeat, save/reload, and clipboard interaction.
- Compare local uppercase `X` behavior with the pinned path, including its register gap and count handling. Add runtime coverage for uppercase X before treating it as acceptance evidence.
- Dispatch the shared profile on Linux and Windows separately. Capture paired native macOS Editor and Keymap evidence with mode, count, cursor, deleted range, register result, theme, and scale recorded.

## Source records inspected

- Action registration and binding: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json` and `zed-refrence/zed/assets/keymaps/vim.json`.
- Delete motion, range kind, Left boundary, and register update: `zed-refrence/zed/crates/vim/src/normal.rs`, `zed-refrence/zed/crates/vim/src/normal/delete.rs`, `zed-refrence/zed/crates/vim/src/motion.rs`, and `zed-refrence/zed/crates/vim/src/normal/yank.rs`.
- Focused pinned fixture: `zed-refrence/zed/crates/vim/src/normal.rs::test_delete_left`.
- Local uppercase-X and count behavior: `crates/editor/src/vim.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
