# Vim AnyPair Text Object Classification

**Status:** Static action, Helix profile binding, and query behavior classified;
native Editor/Helix acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned Vim owns modal text-object dispatch; Editor owns selection and
buffer changes; Keymap owns the shared conditional binding; the Command Palette
provides generic action discovery. Labonair's Editor module is the behavior
owner and Keymap owns mode/profile and user-binding state.

## Evidence boundary

The Linux runtime registers `vim::AnyPair`. No platform default map binds it.
The shared `vim.json` profile binds `m` to AnyPair under
`(vim_operator == a || vim_operator == i || vim_operator == helix_next ||
vim_operator == helix_previous) && helix_mode`. The row is
[`KEYBIND-VIM-HELIX-95e3d82eb63b`](keymap-overlay-requirements.md). The profile
comment describes `mim`/`mam` as selecting the closest surrounding pair;
without Helix mode, there is no built-in `m` text-object binding.

With Vim or Helix enabled and an Editor focused, the pinned Command Palette
also exposes the registered action and restores Editor focus before dispatch.
The palette route is static source evidence, not native dispatch evidence.
macOS registration, selected-profile loading, Linux/Windows dispatch, and
user-keymap overrides still require runtime verification.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-AnyPair](registered-actions-unbound.md) | Editor action for the innermost supported pair surrounding the cursor, or the current Helix selection range; discoverable from the focused Editor's Command Palette when Vim/Helix is enabled. | `labonair-editor` has no AnyPair action, bracket-query integration, or text-object range. The local typed palette has no AnyPair descriptor. |
| [KEYBIND-VIM-HELIX-95e3d82eb63b](keymap-overlay-requirements.md) | Shared profile key `m`, requiring Helix mode plus object or Helix next/previous context; the built-in current-pair flows are `mim` and `mam`. | Labonair has no Helix mode/profile layer or corresponding binding. |
| [SET-keymap-helix-mode](settings-requirements.md) | User-scoped `helix_mode`, false by default; the `m` row is conditional on it. | Labonair has an Editor-owned partial Vim mode but no Helix mode setting. |

## Pinned behavior

`AnyPair` is not the same matcher as `AnyBrackets`. The current-pair lookup
uses the active language's bracket queries to find the innermost enclosing
pair. A query result is accepted only when its opening and closing delimiters
are each one character and form one of the supported surround pairs:
`()`, `[]`, `{}`, `<>`, `""`, `''`, backticks, or `||`. A language without
bracket queries yields no current pair. Multi-character pairs such as HTML/XML
element tags are rejected by this action. Unlike AnyBrackets, a cursor
outside an enclosing query result does not fall back to the nearest pair
elsewhere in the buffer.

The normal text-object range includes both delimiters for an around request
and excludes them for an inside request. In Helix current-object selection,
the existing nonempty selection is used as the query range; a collapsed
selection uses the cursor character. `mim` selects inside the current pair and
`mam` selects around it. After an inside selection, requesting around can
convert that selection to the same pair's around range. Other exact repeat
matches skip the current pair so selection can expand to an enclosing pair.
The action is multiline, expands in both directions, and targets Visual mode.

Helix next/previous object selection differs from current-pair matching. Its
`NearestPair` adapter scans the buffer for the eight supported delimiter
families, chooses the earliest next range or latest previous range, and does
not use a language bracket query for those directional searches. If no range
is found, selection is not updated. The generic `AnyPair` range branch does
not use its numeric `times` argument, and the Helix directional selection
methods receive no count; count behavior needs native characterization.

The Command Palette can invoke the same Editor action. Like other Vim text
objects, it cannot select a pair from plain Normal mode without a pending
object operation. In other modes, the active Vim/Helix operator and selection
path determines the range supplied to the matcher. An absent language query
or absent enclosing pair produces no current selection range; this is silent.

## Labonair owner and current route

`crates/editor/src/vim.rs` implements a small Editor-owned Vim state machine,
but it has no text-object state, Helix selection engine, AnyPair action, or
language bracket-query integration. `crates/workspace/src/views/editor.rs`
forwards input when the Editor-owned `editor.vimMode` preference is enabled.
The Keymap module has typed command identities and user bindings but no Helix
mode setting or selected profile overlay loader.

The future AnyPair behavior belongs to `labonair-editor`. Keymap owns the
conditional `m` binding and user override; the existing command-palette
contract can expose the typed Editor command. No new shell route, persistent
selection state, notification, or cross-module event is required. The mode
setting and user bindings persist through their existing Settings/Keymap
owners.

## Acceptance still required

- Enable Helix mode and verify the shared `m` binding only in its recorded
  object and next/previous contexts. Exercise `mim`, `mam`, and the active
  next/previous object operators on macOS; verify profile precedence and
  user overrides. Repeat dispatch checks on Linux and Windows.
- Test current pair lookup with supported parentheses, brackets, braces,
  angle brackets, quotes, backticks, and vertical bars in languages whose
  bracket queries provide those pairs. Confirm query absence and rejected
  multi-character delimiters, including HTML/XML tags.
- Verify cursor positions inside/outside/on delimiters, nested pairs,
  selection ranges already inside or around a pair, same-pair inside/around
  conversion, enclosing-pair expansion, and absence of AnyBrackets-style
  nearest fallback.
- Verify Helix next/previous text scanning independently from current-pair
  queries, including nested/malformed delimiters, no-result behavior,
  multiple cursors, reversed selections, cursor direction, and count handling.
- Verify pending operator cleanup and non-editing behavior for missing query
  or pair; verify undo/redo, registers, and resulting buffer contents for
  mutating operations that consume the object.
- Confirm the Command Palette entry follows focused-Editor availability and
  Vim/Helix namespace settings, restores focus, and has no standalone Normal
  mode selection effect. Record paired native macOS evidence with language,
  query, mode, viewport, theme, scale, and selection state.
- Keep query and selection behavior in Editor, mode/binding state in Keymap,
  and use existing UI-kit-backed palette and Keymap surfaces.

## Source records inspected

- Action declaration/registration, pair-query matching, range dispatch, and
  delimiter validation: `zed-refrence/zed/crates/vim/src/object.rs`.
- Helix current/next/previous selection and range adapters:
  `zed-refrence/zed/crates/vim/src/helix/object.rs`, `helix/boundary.rs`, and
  `helix/select.rs`.
- Shared `m` binding and mode predicate:
  `zed-refrence/zed/assets/keymaps/vim.json`.
- Stable binding row and setting inventory:
  [`keymap-overlay-requirements.md`](keymap-overlay-requirements.md) and
  [`settings-requirements.md`](settings-requirements.md).
- Palette focused-action route and mode filter:
  `zed-refrence/zed/crates/gpui/src/window.rs`,
  `zed-refrence/zed/crates/command_palette/src/command_palette.rs`, and
  `zed-refrence/zed/crates/vim/src/state.rs`.
- Labonair capability/registry boundaries:
  [`docs/capabilities.md`](../capabilities.md),
  [`docs/registries.md`](../registries.md),
  `crates/editor/src/vim.rs`, and `crates/keymap/src/command_provider.rs`.

Same-session pinned-source research and independently written requirements
are redevelopment, not separated-room clean-room implementation.
