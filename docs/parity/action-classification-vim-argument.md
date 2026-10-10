# Vim Argument Text Object Classification

**Status:** Static action, shared profile binding, syntax-range behavior, and
owner gap classified; native Editor/Helix acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned Vim owns modal object dispatch; Editor owns syntax-backed
selection and edits; Keymap owns shared-profile and user bindings; Settings
owns persisted mode values; the Command Palette exposes registered Editor
actions. Labonair's Editor module owns future argument-object behavior, while
Keymap owns optional bindings and the existing palette contract owns discovery.

## Evidence boundary

The Linux runtime registers `vim::Argument`. No platform-default map binds it.
The shared `vim.json` Vim/Helix layer binds `a` to `vim::Argument` under
`vim_operator == a || vim_operator == i || vim_operator == cs ||
vim_operator == helix_next || vim_operator == helix_previous`. This is the
existing row [`KEYBIND-VIM-HELIX-bdd6381ba4ad`](keymap-overlay-requirements.md);
the row has no per-binding `helix_mode` predicate, while loading the layer is
controlled by the Vim/Helix mode settings. Linux registration and static
profile contents do not prove macOS registration, profile loading, or key
dispatch. The same shared profile data applies to Linux and Windows, whose
dispatch still needs separate runtime verification.

With Vim or Helix enabled and an Editor focused, the pinned Command Palette
exposes registered Vim actions and restores Editor focus before dispatch.
That is static source evidence. Invoking `vim::Argument` from plain Normal mode
without a pending object operation does not by itself select an argument.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-Argument](registered-actions-unbound.md) | Editor text-object action bound to shared-profile `a` for inside/around object operations. It also has a focused-Editor Command Palette route while Vim/Helix is enabled. | `labonair-editor` has no Argument action, text-object operator state, or syntax-backed argument range. The local typed palette has no Argument descriptor. |
| [KEYBIND-VIM-HELIX-bdd6381ba4ad](keymap-overlay-requirements.md) | Shared layer key `a` in the recorded `a`, `i`, `cs`, Helix next, and Helix previous operator contexts; no platform-default binding. | Labonair has user bindings but no selected built-in Vim/Helix profile loader or Helix mode layer. |
| [SET-keymap-vim-mode](settings-requirements.md), [SET-keymap-helix-mode](settings-requirements.md) | User-scoped false-default mode values control loading the shared layer and Vim action namespace. | Labonair has an Editor-owned partial Vim preference, but no Helix mode/profile composition. |

## Pinned behavior

`vim::Argument` dispatches `Object::Argument`. The range finder first skips
whitespace forward from the cursor, then looks for the innermost enclosing
nonempty bracket pair whose opening character is one of `(`, `[`, `{`, `<`, or
`|`. It excludes a cursor offset exactly at the opening marker's start or
after the closing marker, as well as empty pairs. It then requires a syntax
layer at the offset and uses the syntax tree's sibling nodes to locate the
comma-delimited item containing the cursor. This is parser- and language-
dependent; an absent syntax layer or enclosing bracket produces no range.

Inside selection targets the syntax node for the current comma-delimited item,
expanding over whitespace nodes up to adjacent commas or the enclosing pair.
The action can therefore select generic parameters, function arguments, and
tuple/vector/array elements when the active grammar exposes those ranges. If
the syntax cursor is at a delimiter or comma boundary where the helper cannot
find the expected sibling, it can fall back to the full content between the
enclosing pair. The current pinned fixture also retains a commented-out
macro-argument case marked as regressed with the up-to-date Rust grammar, so
macro and grammar-specific behavior is not established by the active tests.

Around selection does not include the call's opening or closing delimiter.
When a comma-separated sibling exists, it includes one comma separator:
when there is a preceding comma it includes that comma and the intervening
whitespace; otherwise, when a following comma exists, it extends through
that comma and whitespace up to the next syntax argument. This makes the
first and later argument boundaries asymmetric. A sole argument has no comma
to include. Empty pairs, unsupported syntax layers, unmatched brackets, and
other cases with no computed range leave the operation without an Argument
range. The numeric object count is not applied by this range branch.

The object is marked multiline, does not always expand selection in both
directions, and targets Visual mode. The pinned `test_argument_object` source
fixture covers a generic parameter, deleting a function argument while
removing its separator, selecting the first function argument around (with
the following comma), changing inside a nested tuple item, array/vector
elements, and cursor positions immediately before an enclosing bracket or
after a call's name. The fixture was inspected but not executed here.

The active shared keymap row also advertises `cs`, `helix_next`, and
`helix_previous` contexts. Those contexts are not working Argument selection
routes. `Object::Argument` has no `HelixTextObject` adapter, so Helix directional
selection cannot convert it to a range and leaves selection unchanged. The
change-surround source-pair resolver has no Argument-to-delimiter mapping;
the `cs` handler finds no bracket anchors and does not proceed to the
replacement delimiter. These are context rows with unsupported consumers,
not successful Argument flows.

The Command Palette can invoke the registered action, but the action needs an
object-pending operation or a mode that consumes the range to make a
selection. The palette itself does not turn plain Normal mode into an
argument selection. No-range cases are silent at the object layer; pending
operator cleanup and exact selection/cursor outcomes still need runtime
acceptance.

## Labonair owner and current route

`crates/editor/src/vim.rs` implements a compact Editor-owned Vim state machine
with basic modes, motions, and d/c/y motion operators. It has no object-pending
state, syntax-backed text-object selector, Helix mode, or Argument action.
`crates/workspace/src/views/editor.rs` forwards Editor input when the
Editor-owned `editor.vimMode` preference is enabled. The Keymap crate exposes
typed command identities and user binding support, but no selected built-in
profile overlay loader or Vim/Helix mode layer.

The future behavior belongs to `labonair-editor`; Keymap owns profile and user
binding composition, and the Settings value contract owns persisted mode
values. The existing typed Command Palette is the discovery route. Selection
and pending operator state remain transient in Editor. No new setting,
notification, cross-module event, shell route, or selection persistence is
implied by this action.

## Acceptance still required

- Verify macOS registration, Vim/Helix layer loading, the `a` row in its exact
  active and excluded contexts, palette visibility/focus restoration, and
  native dispatch. Repeat shared-profile dispatch checks on Linux and Windows.
- Exercise inside/around argument selection, delete/change/yank, Visual
  expansion, cursor direction, selection orientation, counts, multi-cursor
  behavior, pending-operator cleanup, and undo/redo/register contents.
- Cover generic parameters and function arguments; nested calls and
  delimiters; tuples, arrays, vectors, maps/records, macros, malformed and
  empty lists, trailing commas, comments, whitespace, multiline arguments,
  unmatched pairs, and language grammars with or without syntax support.
- Place the cursor on, immediately before, and after opening/closing
  delimiters, commas, argument nodes, and whitespace. Verify the exact
  syntax-node range and the full-inner-pair fallback at AST boundaries.
- Verify inside and around behavior at first, middle, last, and sole argument
  positions, including which comma/whitespace is consumed. Confirm counts do
  not extend the selected argument in the pinned behavior.
- Confirm `cs` with Argument makes no source-pair selection and does not wait
  for a replacement delimiter; confirm Helix next/previous with Argument does
  not change selection because the adapter is absent. Keep those behaviors
  distinct from supported text-object consumers.
- Capture paired native macOS Editor/Keymap interaction with language,
  profile, Vim/Helix values, viewport, theme, scale, source text, and selection
  recorded. Keep syntax selection in Editor and binding state in Keymap.

## Source records inspected

- Action declaration/registration, object flags, range dispatch, parser walk,
  and active fixture: `zed-refrence/zed/crates/vim/src/object.rs`.
- `cs` object handling and source-pair resolution:
  `zed-refrence/zed/crates/vim/src/normal.rs` and
  `zed-refrence/zed/crates/vim/src/surrounds.rs`.
- Helix adapter and directional selection:
  `zed-refrence/zed/crates/vim/src/helix/object.rs` and `helix/select.rs`.
- Shared binding and context:
  `zed-refrence/zed/assets/keymaps/vim.json` and
  [`keymap-overlay-requirements.md`](keymap-overlay-requirements.md).
- Mode settings and registered action:
  [`settings-requirements.md`](settings-requirements.md) and
  [`registered-actions-unbound.md`](registered-actions-unbound.md).
- Labonair Editor/Keymap boundaries:
  [`docs/capabilities.md`](../capabilities.md),
  [`docs/registries.md`](../registries.md), `crates/editor/src/vim.rs`,
  `crates/workspace/src/views/editor.rs`, and
  `crates/keymap/src/command_provider.rs`.

Same-session pinned-source research and independently written requirements
are redevelopment, not separated-room clean-room implementation.
