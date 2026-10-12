# Vim Exchange Action Classification

**Status:** Static range selection and exchange behavior classified; mode, platform, and end-to-end acceptance pending

**Baseline:** Zed commit 3569541038dd51524b03998ba4d38d253cb54f80

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-Exchange`; profile bindings `KEYBIND-VIM-HELIX-6fc9437f8aea` and `KEYBIND-VIM-HELIX-4a05bbefa86b`; cancellation `KEYBIND-VIM-HELIX-3f44433d13b1`

**Owner:** Editor owns text ranges, transient highlights, cursor/selections, and document edits; Vim owns the Exchange operator and mode transitions; Keymap owns the shared operator and motion routes.

## Evidence boundary

The pinned Linux action registry declares `vim::Exchange` as a unit action with no arguments or deprecated aliases. It is absent from the macOS, Linux, and Windows platform-default maps. The shared Vim/Helix profile binds `x` under `vim_operator == c` to start the Exchange operator. Once active, `vim_operator == cx` binds `x` to `vim::CurrentLine` for a linewise range and `c` to `vim::ClearExchange`. Motions and text objects provide other range forms.

The action handler routes a visual selection to `exchange_visual`; in non-visual modes it pushes `Operator::Exchange`, whose key-context label is `cx`. The Vim action namespace is shown in the Command Palette when Vim/Helix mode is enabled, and the palette enumerates available registered actions. Palette discovery is therefore present statically, but dispatch and focus behavior remain unverified. Invoking Exchange outside Normal or a visual selection is not mode-gated by the handler and needs direct acceptance.

## Pinned exchange workflow

In Normal mode, the user starts `c x` (`cx`), then supplies a motion or text object to choose the first range. The motion/object handler stores one anchored range as an Editor background highlight keyed by `VimExchange`, using the theme's `editor_document_highlight_read_background` color; it does not edit the buffer. A later `c x` followed by another motion or text object supplies the second range. `cx x` is the explicit current-line route. `c x c` invokes the separate ClearExchange action and cancels the operator and stored range.

For motion ranges, Exchange expands the newest selection using the motion's count and forced-motion setting. For text-object ranges it expands the newest selection with the around/inside flag but passes no count. Visual invocation uses the newest anchored selection as the range, then switches to Normal; when Helix mode is enabled this becomes Helix Normal. It consumes the Editor's existing selection, whether the user established it with keys or a pointer. The storage and selection helpers use only the newest selection and keep one highlighted range, so multi-cursor behavior is not a parallel per-cursor exchange.

When a stored range exists, the implementation clears its highlight before comparing it with the new range. Strictly separated ranges exchange their text and place the cursor at the new range's start. If the new range contains the stored range, the new range is replaced by the stored text; if the stored range contains the new range, the stored range is replaced by the new text. These containment cases replace the larger range with the smaller range's text rather than preserving the outer text. A partial overlap that satisfies neither containment case performs no edit, and the stored highlight has already been removed. The disjoint check uses strict `<` comparisons, so ranges whose endpoints exactly touch also fall through unless one contains the other. Cursor behavior for these no-edit cases needs direct acceptance.

Each selection path stops Vim recording when it resolves the range. Exchange state is transient Editor highlight state, not a persisted register or document edit. Successful text swaps use the ordinary Editor edit path, with undo grouping and subsequent edits governed by Editor behavior.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the shared `c x`, `cx x`, and `cx c` routes. A workspace search found no Exchange state, action, range storage, highlight, operator, or keymap route in Labonair's `crates/editor/src`.

There is no Exchange-specific setting, notification, event, persistence schema, or new UI control. If implemented, the saved range and highlight belong to the Editor owner and should be cleared through the existing typed Vim operator/action path; the Keymap owner supplies the profile bindings. The theme-derived range highlight uses existing Editor rendering and needs no local styling control.

## Pinned fixtures inspected

- `replace.rs::test_exchange_separate_range` checks two word ranges exchanged in `hello world`.
- `replace.rs::test_exchange_complete_overlap` checks both directions of a containing-range case.
- `replace.rs::test_exchange_partial_overlap` checks a no-edit partial-overlap result.
- `replace.rs::test_clear_exchange_clears_operator` checks cancellation before a range is stored.
- `replace.rs::test_clear_exchange` checks that cancellation removes a stored highlight.

These pinned-source fixtures were inspected, not run, and are not Labonair acceptance evidence. They do not establish Visual/Helix invocation, Command Palette behavior, exact touching endpoints, multi-selection, undo/repeat/recording, or platform dispatch.

## Acceptance still required

- Verify Normal `c x` with motion and text-object ranges, the `cx x` current-line route, a second `c x` range, and `c x c` cancellation both before and after a range is stored. Check that the Vim and Helix profile settings expose and disable the expected rows.
- Verify disjoint ranges in both document orders, each containment direction, exact endpoint contact, partial overlap, identical/empty ranges, final-newline ranges, and ranges changed by intervening edits. Record document contents, highlight lifetime, cursor, and mode after each step.
- Exercise Visual, Visual Line, Visual Block, and Helix Select invocation, multiple selections/cursors, selection direction, counts, forced motions, text-object counts, undo/redo grouping, dot-repeat and macro recording, cancellation, and recovery after focus changes.
- Invoke `vim::Exchange` from the Command Palette in Normal, visual, Insert, and Replace modes, and with Vim/Helix mode disabled. Verify action discovery, resulting operator context, focus restoration, and that out-of-context invocation cannot leave a stuck operator.
- Dispatch all shared key routes on Linux and Windows. Capture paired native macOS Editor and Keymap evidence with profile/mode, first and second ranges, overlap class, highlight, resulting text, cursor, theme, scale, and viewport recorded.

## Source records inspected

- Action schema and label: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Exchange and cancellation action bindings: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Action dispatch, Vim namespace visibility, and operator key-context label: `zed-refrence/zed/crates/vim/src/vim.rs` and `zed-refrence/zed/crates/vim/src/state.rs`.
- Motion/object routing, including count and forced-motion dispatch: `zed-refrence/zed/crates/vim/src/normal.rs`.
- Range storage, highlight, comparisons, edits, and focused fixtures: `zed-refrence/zed/crates/vim/src/replace.rs`.
- Command Palette action enumeration: `zed-refrence/zed/crates/command_palette/src/command_palette.rs`.
- Vim namespace visibility under Vim/Helix mode settings: `zed-refrence/zed/crates/vim/src/state.rs`.
- Labonair Editor owner search: `crates/editor/src`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
