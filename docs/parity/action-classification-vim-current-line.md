# Vim CurrentLine Action Classification

**Status:** Static linewise motion, shared Vim operator matrix, and local line-operation gap classified; platform dispatch and native acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Owner:** Editor owns motion ranges, selections, and document effects; Keymap owns the shared Vim profile; Settings owns Vim mode values.

## Evidence boundary

The pinned Linux action register contains `vim::CurrentLine`, with no arguments and documentation that it selects the current line. The shared `vim.json` profile binds it in 25 Vim operator contexts; no Linux, macOS, or Windows platform-default map binds this action. These shared profile rows are static evidence and do not prove dispatch on any platform.

`CurrentLine` dispatches as the Vim `Motion::CurrentLine` motion. It is linewise by default when an operator consumes it. With a pending count `n`, its target is the end of the current row plus `n - 1` rows, clipped at the buffer end; linewise expansion then includes the selected line boundaries and newline when another line follows. If invoked in normal mode without an operator, it moves the cursor to the current line end; it does not create a full-line selection. The generic Vim action palette may expose the registered action when Vim mode is enabled, but its listing, focus behavior, and direct no-operator effect remain unverified.

## Shared operator matrix

Every row below invokes the same motion in a distinct pending operator context. The final key is the `CurrentLine` completion. A linewise range is then consumed by the pending operator.

| Pending context | Completion key(s) and complete route | Binding requirement IDs | Operator effect |
|---|---|---|---|
| `c` | `c` → `cc` | [4fcfcbedd005](keymap-overlay-requirements.md) | Change the current line. |
| `d` | `d` → `dd` | [f2ca19c896bd](keymap-overlay-requirements.md) | Delete the current line. |
| `gu` | `g u` or `u` → `gugu` or `guu` | [ff28c4b5f23f](keymap-overlay-requirements.md), [010df7016e29](keymap-overlay-requirements.md) | Lowercase the current line. |
| `gU` | `g shift-u` or `shift-u` → `gUgU` or `gUU` | [74e1c850a9ff](keymap-overlay-requirements.md), [c55e6ceea4f3](keymap-overlay-requirements.md) | Uppercase the current line. |
| `g~` | `g ~` or `~` → `g~g~` or `g~~` | [8849300cebff](keymap-overlay-requirements.md), [73ff9b5e174d](keymap-overlay-requirements.md) | Toggle case on the current line. |
| `g?` | `g ?` or `?` → `g?g?` or `g??` | [36edc885aabb](keymap-overlay-requirements.md), [1b6529d89d9f](keymap-overlay-requirements.md) | Apply ROT13 to the current line. |
| `gq` | `g q`, `q`, `g w`, or `w` → `gqgq`, `gqq`, `gqgw`, or `gqw` | [39ad2a231765](keymap-overlay-requirements.md), [74a3a736859a](keymap-overlay-requirements.md), [00bca69c8213](keymap-overlay-requirements.md), [07f569351fd1](keymap-overlay-requirements.md) | Rewrap the current line. |
| `y` | `y` → `yy` | [e1c32270a4ef](keymap-overlay-requirements.md) | Yank a linewise range to the Vim register. |
| `ys` | `s` → `yss` | [6de6594a85b9](keymap-overlay-requirements.md) | Add a surround around the current line. |
| `>` / `<` / `eq` / `sh` | `>` → `>>`; `<` → `<<`; `=` → `==`; `!` → `!!` | [96a354ccccde](keymap-overlay-requirements.md), [dfdfe273a16d](keymap-overlay-requirements.md), [5456953f90d1](keymap-overlay-requirements.md), [952500a7d840](keymap-overlay-requirements.md) | Indent, outdent, auto-indent, or pass the current line to the entered shell filter. |
| `gc` / `gb` | `c` → `gcc` or `gbc` | [c3c49f439b8f](keymap-overlay-requirements.md), [df366715458c](keymap-overlay-requirements.md) | Toggle line comments or block comments. |
| `gR` | `r` or `shift-r` after `gR` | [431cfd081b2d](keymap-overlay-requirements.md), [c3df9d741dbd](keymap-overlay-requirements.md) | Replace the current line using the active Vim register. |
| `cx` | `x` after `cx` → `cxx` | [4a05bbefa86b](keymap-overlay-requirements.md) | Use the current line as the exchange range. |

The matrix covers 25 binding rows. These are Vim operator contexts in the shared profile; it contains no Helix-specific CurrentLine operator. Counted routes extend the current line range by the pending count. The operator owns editing, register, formatting, comment, shell-filter, surround, and exchange effects.

## Pinned behavior and fixtures

`Motion::CurrentLine` has `Linewise` as its default kind and computes its motion point at the end of the counted target line. When expanded for an operator, the range is normalized to line boundaries. A count of two therefore targets the current line and the next line; a count larger than the remaining buffer is clipped to the last line. The operator may change this kind when a forced motion is pending, so forced-motion context must be tested separately. Multiple selections compute a range for each active cursor.

The registered motion action by itself follows the active mode's ordinary motion path. With no pending operator in Vim Normal, that path moves the cursor to the current line end. The profile's user-facing linewise workflows are the operator routes in the matrix, not a dedicated `select current line` chord.

The pinned fixtures inspected include `test_yank_line_with_trailing_newline`, `test_yank_line_without_trailing_newline`, and `test_yank_multiline_without_trailing_newline`; they cover linewise register contents with and without a final newline and a counted two-line yank. `test_dd_then_paste_without_trailing_newline` covers deletion of the final line and paste recovery. `test_toggle_comments` exercises `gcc`, `test_toggle_block_comments` exercises `gbc`, and `test_indent_outdent` exercises `>>` and `<<`. `test_gq` verifies line rewrap using the configured text width. `test_add_surrounds` includes the `yss` current-line case and a count. No focused CurrentLine fixture was found for every conversion alias, auto-indent, shell filter, register replacement, or linewise exchange. Sources and fixtures were inspected, not run.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns future shared operator bindings, and Settings owns Vim Mode. No new dependency, event, action registry, notification, or shell-wide condition is needed to classify the action.

Labonair's Vim parser implements doubled `c`, `d`, and `y` through a direct linewise branch and also has a direct `Y` line-yank path. It does not expose a reusable `CurrentLine` motion/action or the pinned shared operator matrix. Local `~` is a cursor-local case toggle, not the `g~` linewise operator. The existence of `dd`, `cc`, or `yy` therefore covers only those independently implemented paths.

## Acceptance still required

- Dispatch every shared `CurrentLine` row in Vim mode on macOS, Linux, and Windows. Verify each active operator context, excluded contexts, conflicts, base profile, and user override.
- Verify count one, count greater than one, count beyond EOF, empty and whitespace-only lines, final line with/without newline, multiple cursors, reversed selections, and forced-motion behavior.
- Verify `cc`, `dd`, `yy`, `guu`/`gugu`, `gUU`/`gUgU`, `g~~`/`g~g~`, `g??`/`g?g?`, all four `gq` aliases, `yss`, `>>`, `<<`, `==`, `!!`, `gcc`, `gbc`, both `gR` completions, and `cxx`. Check effects, mode transitions, cursor, undo grouping, register contents, and repeat behavior.
- For `!!`, verify the command prompt, explicitly entered command, current-line input, nonzero exit, missing executable, cancellation, output replacement, and recovery. Confirm line contents are supplied as filter input rather than interpreted as the shell command.
- Verify whether the generic Command Palette lists `vim::CurrentLine`; if it does, confirm that direct invocation without an operator moves to line end rather than selecting the line, and check focus restoration.
- Capture paired native macOS Editor and Keymap evidence with profile, mode, operator/count state, theme, scale, resulting range, and document effect recorded. Dispatch the shared profile separately on Linux and Windows.
- Implement one typed linewise motion/action in `labonair-editor` and use it from the accepted operator paths; preserve existing dedicated line operations only when each has a named removal or reuse decision.

## Source records inspected

- Action registration, motion kind, target point, count, and linewise expansion: `zed-refrence/zed/crates/vim/src/motion.rs`.
- Vim operator contexts and shared profile: `zed-refrence/zed/assets/keymaps/vim.json`.
- Operator dispatch and direct normal motion: `zed-refrence/zed/crates/vim/src/normal.rs` and `zed-refrence/zed/crates/vim/src/normal/`.
- Linewise delete, yank, conversion, comments, rewrap, indent, and shell-command consumers: `zed-refrence/zed/crates/vim/src/normal/`.
- Linewise add-surround behavior: `zed-refrence/zed/crates/vim/src/surrounds.rs`.
- Linux action registration: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Local doubled operators, linewise ranges, and `Y`: `crates/editor/src/vim.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence and does not claim runtime acceptance.
