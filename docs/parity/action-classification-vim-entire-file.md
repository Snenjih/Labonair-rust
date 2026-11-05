# Vim EntireFile Action Classification

**Status:** Vim whole-file text-object behavior classified; platform and end-to-end acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-EntireFile`; shared profile binding `KEYBIND-VIM-HELIX-6d0473ebc0fd`

**Owner:** Editor owns document ranges, selections, and operator effects; Vim owns text-object dispatch, pending operators, and mode transitions; Keymap owns the context binding.

## Evidence boundary

The pinned Linux action registry declares `vim::EntireFile` as a unit action with no arguments or deprecated aliases. The action is absent from the three platform default maps, but the shared Vim/Helix profile binds `e` in the text-object context `vim_operator == a || vim_operator == i || vim_operator == cs || vim_operator == helix_next || vim_operator == helix_previous`.

In an object-consuming operation, the Vim `a` and `i` prefixes followed by `e` reach the around/inside text-object paths (for example `d a e` and `c i e`). The same shared row also exposes `cs e` and Helix next/previous contexts. Those latter routes do not all select the file: change-surround lookup has no `EntireFile` bracket-pair mapping, and the Helix text-object converter does not support `EntireFile`. Platform dispatch, Command Palette discovery, and direct invocation remain unverified.

## Pinned whole-file behavior

The registered action calls `vim.object(Object::EntireFile)`. In Normal and Helix Normal modes, it dispatches through `normal_object`; in Visual, Visual Line, Visual Block, and Helix Select modes, it dispatches through `visual_object` only while the text-object operator is active. Insert and Replace modes ignore text-object actions. A direct invocation without an active text-object operator therefore does not select the file.

`Object::EntireFile` is multiline and always expands in both directions. Its range is `DisplayPoint::zero()..map.max_point()`. The range helper ignores the `around` flag and the text-object `times` argument, so `a e` and `i e` have the same whole-file range. The target visual mode is Visual Line regardless of the current mode or around/inside prefix. On an empty buffer, the computed range is empty; final-newline and display-map endpoint effects still need runtime acceptance.

With a Vim text-object operator, the whole-file range is consumed by the corresponding change, delete, yank, indent, outdent, autoindent, shell-command, rewrap, case-conversion, ROT13/ROT47, add-surrounds, comment-toggle, block-comment-toggle, replace-with-register, exchange, or Helix-match path. Each operator owns its own edit, register, selection, and undo effects; this packet classifies the shared range, not every resulting workflow.

The shared `e` binding is also in the `cs` and Helix next/previous contexts. `cs e` reaches the ChangeSurrounds handler, but `object_to_bracket_pair` has no `EntireFile` case, so no surround pair is prepared and the pending operator is cleared. The Helix adapter's `to_helix_object` match omits `EntireFile`; `select_next_object` and `select_previous_object` return without changing the selection when conversion fails. These are exposed keymap contexts with no whole-file selection behavior in the inspected implementation.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the shared Vim profile and its `a e`/`i e` routes. Local `crates/editor/src/vim.rs` has no typed `vim::EntireFile` action, Vim text-object operator route, or equivalent whole-file text-object handling. `Document::select_all()` exists in `crates/editor/src/document.rs`, but a workspace search found no caller; it is not connected to the local Vim key path or operator dispatch.

## Pinned fixtures inspected

No focused `EntireFile` fixture was found under `zed-refrence/zed/crates/vim/src`. The action registration, object range and mode rules, Normal/Visual dispatch, Helix conversion, and change-surround lookup were inspected statically. Generic text-object fixtures do not establish this object's exact selection endpoints or operator results. The reference source was not run, and this is not Labonair acceptance evidence.

## Acceptance still required

- Verify `a e` and `i e` in Normal and applicable Visual modes, including Visual Line, Visual Block, reversed selections, multiple cursors, and selection expansion/shrink behavior. Confirm that both prefixes select the same file range and enter Visual Line where specified.
- Check empty buffers, one-line and multiline buffers, a final newline, no final newline, Unicode, soft wraps, folds, and display-map endpoints. Record the selected text and cursor/anchor positions.
- Exercise the whole-file range with each applicable operator, including counts, registers, undo grouping, replay, language indentation, shell-command cancellation/failure, comment toggling, surround insertion, and exchange state.
- Verify that `cs e`, Helix next/previous `e`, direct action invocation without an object operator, and invocation in Insert/Replace have the classified behavior and do not mutate text unexpectedly. Check Command Palette discovery and outside-context invocation.
- Dispatch the shared binding on Linux and Windows. Capture paired native macOS Editor and Keymap evidence with mode, prefix/operator, buffer contents, selection endpoints, theme, scale, and viewport recorded.

## Source records inspected

- Action schema: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Shared object key/context: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Action registration, object range, multiline/expansion properties, and target mode: `zed-refrence/zed/crates/vim/src/object.rs`.
- Operator consumers and `cs` routing: `zed-refrence/zed/crates/vim/src/normal.rs` and `zed-refrence/zed/crates/vim/src/surrounds.rs`.
- Visual object selection: `zed-refrence/zed/crates/vim/src/visual.rs`.
- Helix object conversion and next/previous selection: `zed-refrence/zed/crates/vim/src/helix/object.rs` and `zed-refrence/zed/crates/vim/src/helix/select.rs`.
- Local Vim and the uncalled select-all primitive: `crates/editor/src/vim.rs` and `crates/editor/src/document.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
