# Vim Enter Action Classification

**Status:** Replace-mode and waiting-mode newline input classified; platform and mode-specific acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns newline edits, selections, and indentation; Vim owns pending operators, Replace mode, and input routing; Keymap owns the `enter` routes.

## Evidence boundary

The pinned Linux action registry declares `vim::Enter` as a unit action with no arguments or deprecated aliases. It is bound to `enter` in `vim_mode == replace` and `vim_mode == waiting`; no platform-default map binds this Vim action.

This action sends a newline through Vim's `input_ignored` handler. It is distinct from normal-mode `enter`, which the shared Vim profile routes to `vim::NextLineStart`, and ordinary Insert-mode Enter, which uses the editor's text-input path. Direct palette invocation and platform dispatch remain unverified.

## Pinned input behavior

In Replace mode (`R`), `vim::Enter` reaches the fallback Replace handler, which calls `multi_replace`. Newline is special-cased as insertion rather than character replacement; the edit is applied at each selection with block-indent handling, and the mode remains Replace. The action does not take a count argument.

In Waiting mode, the newline is input to the current pending operator, so the result depends on that operator. For `r Enter`, the Replace operator consumes the pending count, replaces that many characters with one newline through the Editor newline action, and applies the active autoindent behavior. The focused fixture checks one and two characters, including indentation. Other waiting operators receive the newline through their own input handlers.

The Vim mode adapter enables Editor autoindent in Replace and Waiting contexts; indentation and newline insertion therefore use Editor/language behavior. There is no Enter-specific setting.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the mode-specific `enter` rows. Labonair's local Vim has no typed `vim::Enter` action, Replace mode, or shared keymap route. Its `VimKey::Enter` inserts `\n` in Insert mode, resolves as a linewise downward motion in Normal/Visual mode, and executes the command line in Command mode. Its pending `r` path clears on Enter without replacing the selected character with a newline. It does not provide pinned Replace-mode block indentation or waiting-operator input dispatch.

## Pinned fixtures inspected

- `normal.rs::test_r` checks `r Enter` and a counted `2 r Enter`, including indentation and the one-newline result.
- `replace.rs::test_replace_mode` checks Enter in Replace mode, continued Replace-mode input, and newline edits across multiple selections.
- `normal.rs::test_enter` exercises the separate normal-mode Enter route and helps distinguish it from `vim::Enter`.

These pinned-source fixtures were inspected, not run, and are not Labonair acceptance evidence. They do not establish platform dispatch, all waiting operators, autoindent settings/languages, or paired native acceptance.

## Acceptance still required

- Verify the `R` → Replace-mode → Enter flow and `r` → Waiting-mode → Enter flow in Vim Normal/Visual/Visual Line/Visual Block as applicable, plus Helix Normal/Select where the context accepts the action.
- Verify multi-count `r Enter` deletes the expected characters and inserts one newline; test empty rows, tabs, Unicode graphemes, EOF, final newline, selection direction, and multiple cursors.
- Verify Replace-mode Enter inserts a newline at each selection without replacing following characters, keeps Replace mode, and applies language-aware block indentation. Check undo grouping, `u`, replace undo, replay, and recovery after invalid input.
- Exercise Enter with other waiting operators, including find, literal, digraph, register, surround, and Helix input states; verify cancellation and no unexpected text mutation.
- Check whether the command palette exposes `vim::Enter` and what happens when invoked outside Replace/Waiting. Dispatch key routes on Linux and Windows. Capture paired native macOS Editor and Keymap evidence with mode, operator/count, indentation, selection state, resulting text, theme, and scale recorded.

## Source records inspected

- Action schema: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Replace/Waiting action bindings and normal-mode distinction: `zed-refrence/zed/assets/keymaps/vim.json`.
- Action registration, newline dispatch, and mode/editor autoindent state: `zed-refrence/zed/crates/vim/src/vim.rs`.
- Waiting-mode replace input and Replace-mode multi-selection insertion: `zed-refrence/zed/crates/vim/src/normal.rs` and `zed-refrence/zed/crates/vim/src/replace.rs`.
- Local Vim key handling: `crates/editor/src/vim.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
