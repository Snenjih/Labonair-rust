# Vim HelixJumpToWord Action Classification

**Status:** Visible word targets, label/operator flow, and mode-specific outcomes classified; viewport rendering, keyboard-layout behavior, platform dispatch, and native acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-HelixJumpToWord`; binding `KEYBIND-VIM-HELIX-dcba2097e0d3`; settings `SET-keymap-vim-mode` and `SET-keymap-helix-mode`

**Owner:** Editor/Vim owns visible word-target discovery, mode-dependent cursor/selection effects, and temporary operator state; Editor owns selection and navigation-overlay presentation; Keymap owns the Helix mode-scoped `g w` route. Labonair's Editor/Vim state belongs to `labonair-editor`; a Helix profile route belongs to `labonair-keymap`, with mode values owned by Settings.

## Evidence boundary

The pinned Linux registry declares `vim::HelixJumpToWord` as a unit action with no JSON arguments or deprecated aliases. The shared Vim/Helix profile binds `g w` in `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`. This route is distinct from Vim's other `g w` entries, which dispatch Rewrap/PushRewrap or CurrentLine under their own contexts. Linux and Windows share this conditional Vim/Helix profile data, but static inheritance does not prove dispatch on either platform.

The action creates a transient list of navigation targets from the visible Editor range. When the Editor does not provide a usable visible range, it derives one from scroll position and display rows. It scans contiguous runs of underscore and alphanumeric characters, and keeps only words containing at least two characters. A word containing any selection head is excluded. Nonempty selection ranges are also excluded when the active mode does not already represent a selection; Helix and Visual selection modes can jump to words inside their current ranges.

Targets are ordered from the primary cursor, interleaving forward and backward candidates, with nearby candidates assigned first. At most 676 targets are labeled with sequential two-letter lowercase labels from `aa` through `zz`. The action creates a navigation overlay per target using the configured text font and the `vim_helix_jump_label_foreground` theme color. The label may cover text; proportional-font fitting can hide part of the target text, shift the label into preceding whitespace, or scale it. Actual readability and overlap depend on native rendering and remain unverified.

If there are no targets, the handler clears an existing jump overlay and does not push a pending operator. Otherwise it installs the overlays and pushes a `HelixJump` operator. The operator waits for two single-character keys without Control, Alt, platform, or function modifiers; Shift is accepted. The first valid letter leaves the full overlay visible and records the prefix. The second resolves the target. Input is lowercased for comparison, so label letters are case-insensitive. Invalid first or second input clears the overlay/operator. Escape uses the modal cancellation path, which also clears the jump overlay. The key handling consumes label characters before text/IME input. The recognized label alphabet is ASCII; behavior on non-ASCII keyboard layouts needs direct verification.

The resolved effect depends on Vim mode. Helix Normal selects the full target word. Helix Select extends the current selection toward the target word, ending at the target word end when moving forward and extending back to its start when moving backward. Vim Normal collapses at the target word start. Vim Visual, VisualLine, and VisualBlock extend as a motion to the target word start. Other modes use full-word selection. The default `g w` binding only exposes Helix Normal/Select; Vim mode results are from the registered action's separately bound test route, not a default Vim `g w` mapping.

The action does not explicitly read a count. Count-prefix interaction, selection changes while the overlay is pending, focus loss, and palette/direct invocation behavior are not established by the inspected fixtures. Static source and test inspection does not establish macOS, Linux, or Windows runtime behavior.

## Pinned Helix workflow

In Helix Normal or Helix Select, the user invokes `g w`. Two-letter labels appear over eligible visible words. The user types the matching pair; the Editor then selects the target word in Helix Normal or extends the existing range in Helix Select. The text is not edited. Escape or an invalid label cancels the pending jump and removes the overlay. A pending operation is represented internally as `gw`; its status presentation needs native acceptance.

The label list is viewport-bound. A cursor's current word is omitted when the cursor head is inside it, but a word immediately before the cursor boundary can be offered. Helix selections can target words within their selected range, including another word on a line selected with `x`. One-character words are omitted. Underscored identifiers are treated as one word; Unicode alphanumeric characters participate in word runs. Label capacity is 676, with nearby candidates retained before more distant ones.

The Helix route does not replace Vim `g w` Rewrap/PushRewrap or the separate CurrentLine entry. It is an additional mode-scoped navigation workflow. Labonair currently has no Helix mode, two-character jump operator, or visible word-label overlay.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-HelixJumpToWord](registered-actions-unbound.md) | Unit action discovers visible word targets, shows two-letter labels, waits for a label pair, then applies the active mode's move/selection behavior. It does not edit text. | `labonair-editor` has no equivalent transient word-jump state or target-label overlay; its current Vim mode set has no Helix Normal/Select behavior. |
| [KEYBIND-VIM-HELIX-dcba2097e0d3](keymap-overlay-requirements.md) | `g w` in Helix Normal/Select while no menu is open. Other Vim contexts retain separate `g w` routes. | `labonair-keymap` has no Helix profile route or typed jump action. Existing Vim mappings must retain their own meanings. |
| [SET-keymap-helix-mode](settings-requirements.md) | Helix mode exposes the Normal/Select contexts that own this route. | Labonair has no Helix mode setting or selection-mode implementation. |

## Labonair owner and current route

The canonical future capability owner is `labonair-editor` for word discovery, selection results, and transient navigation state, with `labonair-keymap` owning the Helix profile binding. Settings owns mode values. No new action-specific setting, notification, persistence path, event, or registry contribution is needed. The interaction should compose with the Editor's navigation overlay and selection APIs; it does not call for a new local button, list, menu, or badge. No dependency change is proposed.

The local Editor implements single-cursor Vim state and text navigation, but `crates/editor/src/vim.rs` has no Helix word-jump route. The Keymap owner loads and manages profiles but has no Helix Normal/Select layer. The local crosswalk must therefore record this as missing functionality rather than equating ordinary Vim `g w` or a generic Go To Line command with label navigation.

## Pinned fixtures inspected

- `vim/src/helix.rs::test_helix_jump_starts_operator`, the Escape and invalid-first/invalid-second cancellation fixtures, and the first-character overlay fixture cover the pending operator lifecycle.
- `test_helix_jump_includes_word_before_cursor_boundary`, `test_helix_jump_skips_single_char_words`, `test_helix_jump_handles_underscored_words`, `test_helix_jump_at_end_of_buffer`, `test_helix_jump_moves_to_target_word`, and `test_helix_jump_includes_line_selection_targets` cover target boundaries and representative Helix Normal results.
- `test_helix_jump_consumes_label_keystrokes_before_ime` checks interception before IME and confirms the label keys do not propagate as text. Vim Normal and Visual fixtures cover the separately bound motion and extension behaviors; Helix Select fixtures cover forward and backward range extension.
- Other inspected fixtures cover case-insensitive input, Unicode words, label ordering, theme label color, and prioritizing nearby targets when truncating to the 676-label limit.
- `vim/src/helix.rs` defines candidate discovery, ordering, label generation, and overlay data; `vim/src/vim.rs` owns label-key handling, operator cleanup, and mode-specific resolution; `vim/src/state.rs` records the pending operator status as `gw`.
- Fixtures and source were inspected, not run.

## Acceptance still required

- Verify macOS, Linux, and Windows action registration and dispatch, Helix profile precedence, menu exclusion, focus ownership, and interactions with Vim Mode/Helix Mode settings. Verify palette visibility and direct invocation separately from the default keymap route.
- Verify the visible range at viewport edges and after scrolling, soft wrapping, resizing, folding, inlay hints, multibuffer excerpts, and partially visible rows. Include empty/no-target views and stale or unavailable Editor state.
- Verify target word boundaries for ASCII, Unicode letters/digits, underscore, combining marks, emoji, punctuation, one-character words, EOF, and the cursor exactly at each word boundary. Verify selected-range overlap, multiple cursors, reversed ranges, and selection changes while the operator waits.
- Verify candidate order, direction interleaving, the 676-target cap, and the exact labels retained for forward-only, backward-only, and mixed target lists. Verify count prefixes and count cleanup.
- Verify first/second label input, uppercase and shifted keys, non-ASCII keyboard layouts, dead keys, IME composition, modified keys, invalid labels, Escape/Ctrl-C/Ctrl-[, focus loss, buffer/tab changes, and cancellation when the visible target set changes.
- Verify label placement, target-text coverage, scaling, whitespace shifts, proportional/monospace fonts, theme color contrast, overlapping labels, accessibility/zoom, and the pending `gw` status indicator. Capture paired native macOS Editor/Keymap evidence with window bounds, scale, theme, viewport, cursor/selection geometry, labels, and key sequence recorded.
- Verify Helix Normal word selection, Helix Select extension in both directions, and custom Vim Normal/Visual action behavior without changing the separate default Vim `g w` routes.

## Source records inspected

- Action schema and description: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Registration, candidate collection, visible-range calculation, target ordering, label rendering data, and direct fixtures: `zed-refrence/zed/crates/vim/src/helix.rs`.
- Input interception, pending operator lifecycle, cancellation, and mode-specific result: `zed-refrence/zed/crates/vim/src/vim.rs` and `zed-refrence/zed/crates/vim/src/state.rs`.
- Helix `g w` binding and neighboring Vim `g w` routes: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Labonair Editor/Vim and Keymap owners: `crates/editor/src/vim.rs` and `crates/keymap/src`.
- Mode settings: `docs/parity/settings-requirements.md`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
