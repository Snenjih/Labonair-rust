# Vim ChangeCase Action Classification

**Status:** Static action, shared bindings, transformation, and owner-route behavior classified; native dispatch and mode acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Editor owns case transforms, selection, and modal state; Keymap owns profile and user bindings; Settings owns Vim/Helix mode values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers `vim::ChangeCase`. It has no
binding in the macOS, Linux, or Windows platform-default maps. The shared
`vim.json` layer binds `~` in Vim Normal, Vim Visual, Helix Normal, and Helix
Select contexts. Registered Vim actions also have a static focused-Editor
Command Palette route while Vim/Helix is enabled. These facts do not establish
macOS registration, active profile loading, key dispatch, or native behavior.

The direct `~` action is separate from `g~{motion}`: the shared profile routes
`g~` to `vim::PushOppositeCase`, which waits for a motion. `ChangeCase` acts
immediately on the current cursor or selection. It is also distinct from the
separate explicit upper/lower conversion actions.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-ChangeCase](registered-actions-unbound.md) | Registered unit action that toggles case over a mode-dependent cursor range or selection. Palette invocation directly changes the current range. | Labonair's Editor-owned Vim parser has a normal-key `~` branch, but no registered action, palette descriptor, selected-range transform, or Helix mode. |
| [KEYBIND-VIM-HELIX-6a17dd765f07](keymap-overlay-requirements.md), [KEYBIND-VIM-HELIX-0dd439257b0f](keymap-overlay-requirements.md), and [KEYBIND-VIM-HELIX-e8d6dbaa65eb](keymap-overlay-requirements.md) | The shared layer binds `~` in Vim Visual, Vim/Helix Normal, and Helix Select. There is no platform-default binding. | Labonair accepts `~` through its local modal parser, but selection handling, Unicode expansion, mode transitions, Helix behavior, and profile management differ or are missing. |
| [SET-keymap-vim-mode](settings-requirements.md) and [SET-keymap-helix-mode](settings-requirements.md) | Mode values gate the shared layer and Vim palette namespace; there is no case-specific setting. | Labonair has an Editor-owned partial Vim preference and no Helix mode/profile layer. |

## Pinned behavior

The action transforms each character independently. Lowercase characters use
Rust's full uppercase mapping; all other characters use the full lowercase
mapping. A mapping can expand to multiple Unicode scalars, as `ß` becomes
`SS`. Uncased symbols remain unchanged. Edits are collected from the original
snapshot and applied in reverse range order so earlier offsets remain valid.
Read-only Editors do not edit, and a range with no changed text produces no
buffer edit.

The selected range and post-action cursor depend on the current mode:

| Mode | Range transformed | Selection/cursor and resulting mode |
|---|---|---|
| Vim Normal | `count` characters from each cursor on the current line; count defaults to one. | Cursor advances to the range end, except a range reaching line end leaves it on the final transformed character. Requests Vim Normal afterward. |
| Vim Visual / Visual Line | Each selected range. | Selections collapse to their starts; the handler requests Vim Normal. |
| Vim Visual Block | Each block-derived selection range. | The implementation records one cursor at the first range's start, then requests Vim Normal. |
| Helix Normal | A non-empty selection, or one character at an empty cursor. | A non-empty selection remains selected; an empty selection keeps the cursor at its character. Helix Normal remains active. |
| Helix Select | Each selected range; an empty selection uses one character. | The range is transformed, then the handler requests Vim Normal rather than Helix Normal. No focused Helix Select fixture was found. |
| Insert / Replace action invocation | `count` characters from each selection start on the current line. | Uses the Normal range/cursor handling and requests Vim Normal. No default key binding exposes these modes, but the generic palette route can invoke the action. |

The direct action consumes a count (default one) and forced-motion state,
records the current action, and stores visual marks before editing. A direct
palette invocation is not a passive setting: it performs the same mode-based
text change at the active Editor selection/cursor. If the action is invoked
from Vim Visual, Visual Line, Visual Block, or Helix Select, the source handler
requests a transition to `Mode::Normal` even when the transformation is a
no-op or the Editor is read-only; native focus and mode presentation need
verification.

The `g~{motion}` operator is a separate route. The `vim::PushOppositeCase`
action starts the operator and a later motion defines its range. Tests for
that operator do not stand in for the direct `vim::ChangeCase` action.

Focused source fixtures cover Normal count movement, multibyte text and
multi-cursor edits, characterwise/linewise/blockwise Visual ranges, and
Helix Normal with both a selection and a cursor. A Visual test shows `ß`
expanding to `SS`. The Helix fixture does not cover Helix Select or its
post-action transition. These tests were inspected but not run.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; Keymap owns optional user
bindings and future mode profiles, and Settings owns the mode values. The
existing Command Palette remains the action discovery surface. No setting,
notification, event, registry, or shell-wide conditional is required for case
transformation.

`crates/editor/src/vim.rs` handles `~` in the local Vim Normal, Visual, and
Visual Line key parser. Its `toggle_case` function edits `count` characters
starting at the current cursor and does not read or transform the active
selection. It has no Helix modes, registered action, palette route, or
Keymap-owned shared profile. For each character it takes only the first
scalar from `to_uppercase()` or `to_lowercase()`, so it truncates expanding
Unicode mappings; local `ß` becomes `S`, not `SS`. The local action also keeps
its current mode rather than applying the pinned Visual/Helix Select
transition.

Local text changes remain owned by Editor and use its document edit/undo
route. No extra persistence or case preference is implied. The Unicode
truncation is recorded as a local behavior bug in `memory/bugs_and_fixes.md`;
this inventory slice records it without changing runtime code.

## Acceptance still required

- Verify the three shared `~` binding rows with Vim and Helix mode values on
  and off, including inactive contexts and user-binding precedence/save/reload.
  Repeat dispatch separately on macOS, Linux, and Windows.
- Verify focused-Editor palette discovery and invocation in Normal, Visual,
  Helix Normal, Helix Select, Insert, and Replace states; capture resulting
  text, cursor, selection, focus, and mode.
- Check Normal count and line-end behavior, multiple cursors, characterwise,
  linewise and blockwise Visual ranges, selected Helix ranges, empty Helix
  cursors, read-only buffers, uncased characters, and multi-scalar Unicode
  mappings such as `ß`.
- Verify whether Helix Select should transition to Vim Normal after the edit.
  Confirm Visual and Helix mode transitions when the text does not change or
  the buffer is read-only.
- Exercise `g~{motion}` separately and confirm it remains the pending
  `PushOppositeCase` operator route.
- Capture paired native macOS Editor and Keymap evidence with platform,
  profile, mode, selection, cursor, viewport, theme, and scale recorded.
- Implement case behavior through `labonair-editor`'s typed text-edit and
  selection contract; preserve complete Unicode case expansions and verify
  undo, cursor, and selection results.

## Source records inspected

- Action declaration, registration, transformation, and focused fixtures:
  `zed-refrence/zed/crates/vim/src/normal.rs` and
  `zed-refrence/zed/crates/vim/src/normal/convert.rs`.
- Editor selection transformation and full Unicode case mappings:
  `zed-refrence/zed/crates/editor/src/editor.rs`.
- Direct `~` and separate `g~` profile rows:
  `zed-refrence/zed/assets/keymaps/vim.json`.
- Linux action registration evidence:
  `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Local modal key route and per-character conversion:
  `crates/editor/src/vim.rs`.

Same-session pinned-source research and independently written requirements
are redevelopment, not separated-room clean-room implementation. This packet
records static evidence and does not claim runtime acceptance.
