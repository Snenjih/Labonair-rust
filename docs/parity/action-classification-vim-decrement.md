# Vim Decrement Action Classification

**Status:** Static numeric/toggle behavior and shared Vim/Helix routes classified; platform dispatch and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns target discovery, document edits, selection and mode transitions; Keymap owns the shared profile; Settings owns Vim and Helix mode values.

## Evidence boundary

The pinned Linux action registry declares `vim::Decrement` with one optional Boolean argument, `step`, defaulting to `false`. Its action description covers decrementing the number under the cursor and toggling Boolean values. No platform-default map binds this action; the user routes are in the shared Vim profile. Static profile entries do not prove runtime dispatch on any platform. The Vim action palette may expose registered actions, but availability and direct invocation were not verified here.

The shared `vim.json` profile has five `ctrl-x` routes: Vim Normal, Vim Visual, and Helix Select use the default `step: false`; Visual `g ctrl-x` passes `step: true`; and a later `(vim_mode == normal || vim_mode == helix_normal) && !menu` row repeats Normal's `ctrl-x` action while adding Helix Normal and excluding menus. The two Normal-capable rows name the same action and arguments. Their effective precedence, menu behavior, and user overrides still require runtime checking.

## Pinned behavior

The action consumes a pending Vim count (default one) and forced-motion state, then handles every active selection in one Editor transaction. A cursor with no selection searches forward from its position for the next supported number or toggle word on that line. The scan can find inline tokens without whitespace; its backward scan finds the token start when the cursor is already on a token. A leading minus is part of a signed decimal, but a hyphen stops the scan from reaching an earlier token. A period separates digit runs rather than forming a decimal number. Search stops at the current line boundary.

For a non-empty Visual or Helix selection, the action scans the selected range row by row and changes the first supported token found within each row's selected portion. Partial first and last rows remain clipped to the selection. Normal `ctrl-x` applies the same negative count to each numeric target. Visual `g ctrl-x` starts at `-count`, then reduces the delta by another count after each found target, so numeric targets in selection order receive `-count`, `-2*count`, `-3*count`, and so on. The step progression applies across the action's active selections and rows; Boolean pairs toggle once regardless of the delta.

Supported numeric forms are signed decimal, `0x`/`0X` hexadecimal, and `0b`/`0B` binary. The radix prefix remains in the document. Decimal leading zeroes are retained when possible; hex digits retain the input's letter-case convention; binary and hexadecimal values are zero-padded to their previous digit width. Numeric arithmetic uses 64-bit wrapping behavior. Octal and floating-point values are not parsed as such: periods split digit runs. Boolean pairs `true`/`false`, `yes`/`no`, and `on`/`off` are matched case-insensitively and retain lowercase, title case, or uppercase style.

After a token is changed from an empty cursor, the caret is placed on the token's final digit. A non-empty selection collapses to its starting anchor, whether or not a token was found; visual marks are stored first. If no token is found, no text edit is made. The action requests Normal mode, which the pinned mode helper maps to Helix Normal when Helix mode is enabled. Cursor, selection, and mode effects are part of the acceptance contract, not evidence of local implementation.

## Pinned fixtures inspected

`test_increment` exercises `ctrl-x` counts and the transition through zero into negative values. `test_increment_with_dot` and `test_increment_with_two_dots` distinguish digit runs around periods. The leading-zero fixtures cover padding and width changes; sign fixtures cover zero and negative transitions. `test_increment_bin_wrapping_and_padding`, `test_increment_hex_wrapping_and_padding`, `test_increment_wrapping`, and `test_increment_hex_casing` cover radix formatting, wraparound, and letter case. `test_increment_steps` covers stepped Visual decrement; `test_increment_negative_numbers` covers signed numbers and hyphen boundaries. `test_increment_toggle` covers Boolean pairs and Visual line/block selections. `test_increment_order` covers target discovery order; `test_increment_visual_partial_number` covers partial Visual targets. Markdown list markers, inline tokens, and multibyte cursor positions have dedicated fixtures. These pinned-source fixtures were inspected, not run, and are not Labonair acceptance evidence.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the shared Vim/Helix profile; Settings owns the mode values. No new dependency, event, registry, notification, or shell-wide condition is needed to classify the action.

The local Editor Vim implementation has no `vim::Decrement` action, numeric decrement/increment behavior, Boolean toggle behavior, or matching Vim/Helix profile route. Any generic platform `ctrl-x` binding is a separate action and does not establish this Vim workflow.

## Acceptance still required

- Verify macOS action registration, palette availability and focus restoration, and each `ctrl-x` route in Vim Normal, Visual, Helix Normal, and Helix Select. Verify Visual `g ctrl-x` passes `step: true`, and test menu exclusion, chord precedence, profile loading, and user overrides.
- Verify signed decimal, zero/sign crossings, leading zeros, 64-bit wrapping, binary and hexadecimal prefixes, radix digit padding, hexadecimal case, inline tokens, hyphen and period boundaries, unsupported forms, and multibyte text.
- Verify case-insensitive `true`/`false`, `yes`/`no`, and `on`/`off` toggling, including lower, title, and upper case.
- Verify Normal count defaults and explicit counts; Visual default decrement and stepped decrement; multiple rows, partial ranges, multiple selections, selection order, block selection, no-target behavior, EOF, and large selections/counts.
- Verify final caret positions, selection collapse, Vim/Helix mode transitions, one-step undo grouping, repeat behavior, save/reload, and recovery from invalid or absent targets.
- Dispatch the shared profile on Linux and Windows separately. Capture paired native macOS Editor and Keymap evidence with mode, selection, token, count, result, theme, and scale recorded.

## Source records inspected

- Action argument schema and registration: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json` and `zed-refrence/zed/crates/vim/src/normal/increment.rs`.
- Token scan, numeric formatting, selection transaction, and mode transition: `zed-refrence/zed/crates/vim/src/normal/increment.rs` and `zed-refrence/zed/crates/vim/src/vim.rs`.
- Shared action contexts: `zed-refrence/zed/assets/keymaps/vim.json`.
- Local Vim route search: `crates/editor/src/vim.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
