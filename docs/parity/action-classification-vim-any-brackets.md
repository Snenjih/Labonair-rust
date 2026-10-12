# Vim AnyBrackets Text Object Classification

**Status:** Static action and command-palette route classified; native Vim/Helix
acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned Vim owns modal object dispatch; Editor owns selections and
buffer edits; Keymap owns profile bindings; the Command Palette provides the
generic action entry. Labonair's Editor module is the behavior owner, Keymap
owns user binding state, and the existing command-palette contract owns
discoverability.

## Evidence boundary

The Linux runtime registers `vim::AnyBrackets`. It has no binding in the
platform default maps or active shared Vim/Helix profile. In `vim.json`, the
candidate `b` binding is commented out and the active `b` binding invokes
`vim::Parentheses` instead. This is not an active profile row.

When Vim or Helix is enabled and an Editor is focused, the pinned Command
Palette obtains available actions from that focused Editor's action listeners,
shows the Vim namespace, and dispatches a selected action after restoring
focus. The resulting palette label is `vim: any brackets`. This establishes a
static route even though the action has no built-in key binding. It does not
prove native dispatch on macOS or another platform. A user-defined key binding
and its precedence have not been exercised.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-AnyBrackets](registered-actions-unbound.md) | The Vim Editor registers the `AnyBrackets` object action. With Vim/Helix enabled and Editor focus, the Command Palette exposes it. It consumes a pending object operation or expands a Visual selection; plain Normal mode has no pending object, so this action cannot select a bracket by itself. | `labonair-editor` has basic Vim motions and operators but no text-object action or bracket-range scanner. The local palette has no AnyBrackets command descriptor. |
| [KMP-MODE-VIM-HELIX](keymap-overlay-requirements.md) | The shared layer has no active AnyBrackets binding. Its commented `b` candidate is inactive; active `b` maps to `vim::Parentheses`. Mode enablement controls palette namespace visibility, not this binding. | Labonair has no base-profile overlay loader or Helix mode. `editor.vimMode` is a partial Editor-owned Vim counterpart. |

## Pinned behavior

`AnyBrackets` is distinct from `AnyPair`. AnyBrackets checks four literal
delimiter pairs: `()`, `[]`, `{}`, and `<>`. It does not include quotes,
backticks, or vertical bars. AnyPair uses the language's bracket queries and
can match those additional pair types.

For each bracket type, the object uses the surrounding-marker matcher. If the
cursor is inside one or more matching ranges, it chooses the smallest
enclosing range. If no range encloses the cursor, it chooses the candidate
range nearest to the cursor. The shared matcher handles nested, escaped, and
multiline delimiters. This is character-based matching, not syntax-aware
parsing; angle comparisons and malformed text can be interpreted as pairs.

The object is multiline, always expands selection in both directions, and
targets Visual mode. Inside/around behavior comes from the pending operator:
inside excludes the delimiters, while around includes them. The normal object
pipeline can consume the range for supported operators such as change, delete,
yank, indentation, case conversion, and surround. Helix next/previous object
operations can also consume it when invoked through their corresponding
contexts. A selected range expands in Visual mode. The object-range branch
does not read its `times` argument, so a numeric object count does not expand
the chosen bracket pair in the inspected source.

If there is no complete matching pair, range lookup returns no result. A
pending normal-mode operator is cleared without editing the buffer. The
visual selection range remains unchanged; because the object targets Visual
mode, a prior Visual Line or Visual Block mode may still transition to Visual.
Insert and Replace modes ignore the action. A
plain Normal-mode palette invocation with no pending operator likewise does
not select a range; the palette entry is discoverable, but it is not a
standalone “select nearest bracket” command in that mode.

The shared profile context enclosing the commented candidate is
`vim_operator == a || vim_operator == i || vim_operator == cs ||
vim_operator == helix_next || vim_operator == helix_previous`. The active
profile currently binds `b` to Parentheses in this context. Do not represent
the commented AnyBrackets line as a working default binding.

## Labonair owner and current route

`crates/editor/src/vim.rs` implements a compact Editor-owned Vim state machine
with basic motions and d/c/y motion operators. It has no text-object state,
AnyBrackets action, or range scanner. `crates/workspace/src/views/editor.rs`
forwards keystrokes when the Editor-owned `editor.vimMode` preference is
enabled. There is no Helix mode or selected base-profile overlay loader.

The local command palette discovers typed `CommandId` contributions from
providers; `labonair-editor` has no AnyBrackets action/descriptor and
`labonair-keymap` cannot bind a command that is not registered. The future
behavior remains Editor-owned in `labonair-editor`. An optional user binding
belongs to `labonair-keymap`; discoverability uses the existing command
palette contract. No additional shell route, setting, notification, or
cross-module event is implied. There is no selection persistence; only a
user-authored binding would persist through the existing Keymap file.

## Acceptance still required

- Verify the Command Palette includes the action only with Editor focus and
  the pinned Vim/Helix mode enabled, restores focus, and dispatches the action
  on macOS. Confirm Linux and Windows behavior separately.
- Verify the palette action does not select in Normal mode without a pending
  operator, is ignored in Insert/Replace, expands Visual selections, and
  completes a supported pending object operation.
- Confirm platform default maps and the shared mode layer have no active
  AnyBrackets binding; `b` continues to invoke Parentheses. Exercise a custom
  override only after its user-file parsing, precedence, conflict, save, and
  reload behavior is available.
- Check `()`, `[]`, `{}`, and `<>`; nested, escaped, multiline, adjacent,
  unmatched, empty, malformed, and comparison-operator text; cursor positions
  inside and outside pairs; and nearest-pair fallback.
- Verify inside/around range boundaries, change/delete/yank and supported
  object operators, Visual expansion, unmatched-operator cleanup, cursor and
  selection direction, registers, undo/redo, and count behavior.
- Compare the native Command Palette and Editor interactions against paired
  macOS evidence with platform, mode settings, keymap profile, viewport,
  theme, scale, and source text recorded. Static source inspection is not
  runtime acceptance.
- Keep text-object behavior in Editor and binding/profile state in Keymap;
  matching must remain responsive for large and multiline buffers.

## Source records inspected

- Action declaration, registration, matching pairs, range dispatch, and
  delimiter matching: `zed-refrence/zed/crates/vim/src/object.rs` and
  `surrounds.rs`.
- Normal operator handling and Visual selection expansion:
  `zed-refrence/zed/crates/vim/src/normal.rs` and `visual.rs`.
- Shared profile status: `zed-refrence/zed/assets/keymaps/vim.json`.
- Focused action discovery and restoration before palette dispatch:
  `zed-refrence/zed/crates/gpui/src/window.rs` and
  `zed-refrence/zed/crates/command_palette/src/command_palette.rs`.
- Vim/Helix palette namespace visibility:
  `zed-refrence/zed/crates/vim/src/state.rs`.
- Action inventory: [`registered-actions-unbound.md`](registered-actions-unbound.md).
- Shared mode layer and Labonair owner boundaries:
  [`keymap-nonmacos-requirements.md`](keymap-nonmacos-requirements.md),
  [`docs/capabilities.md`](../capabilities.md),
  [`docs/registries.md`](../registries.md),
  `crates/editor/src/vim.rs`, and `crates/keymap/src/command_provider.rs`.

Same-session pinned-source research and independently written requirements
are redevelopment, not separated-room clean-room implementation.
