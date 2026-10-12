# Vim AngleBrackets Text Object Classification

**Status:** Static action, profile bindings, operator semantics, and Labonair
gap classified; native Vim/Helix acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned `vim` owns modal object dispatch; Editor owns selections and
buffer edits; Keymap owns the conditional Vim/Helix layer. Labonair's Editor
module is the behavior owner; its Keymap module owns binding/profile state.

## Evidence boundary

The Linux runtime registers `vim::AngleBrackets`. It has no binding in the
default macOS, Linux, or Windows maps. The shared built-in `vim.json` layer has
two bindings in the conditional `KMP-MODE-VIM-HELIX` layer, represented by
[`KEYBIND-VIM-HELIX-ca7ff884331a`](keymap-overlay-requirements.md) and
[`KEYBIND-VIM-HELIX-d85cfc328391`](keymap-overlay-requirements.md). Linux action
registration and static profile data do not prove macOS registration, profile
loading, active-context matching, or runtime dispatch. The exact Vim/Helix
mode-setting UI and precedence when both modes are enabled remain open.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-AngleBrackets](registered-actions-unbound.md) | In the conditional Vim/Helix layer, `<` invokes `vim::AngleBrackets` with `opening: true`; `>` invokes it with the default `opening: false`. Both require `vim_operator == a`, `i`, `cs`, `helix_next`, or `helix_previous`. The action dispatches the `Object::AngleBrackets` text object to the active operator or selection flow. | `labonair-editor` has basic Vim mode, motions, and d/c/y operators, but no text-object action or angle-pair range implementation. Labonair has no selected base-profile overlay loader or Helix mode; the local Vim preference is Editor-owned and does not provide this binding layer. |
| [KEYBIND-VIM-HELIX-ca7ff884331a](keymap-overlay-requirements.md) | `<` passes the `opening: true` argument under the shared text-object contexts. | No Labonair equivalent profile row or action binding. |
| [KEYBIND-VIM-HELIX-d85cfc328391](keymap-overlay-requirements.md) | `>` invokes the same action with the default false argument under the shared text-object contexts. | No Labonair equivalent profile row or action binding. |

## Pinned behavior

The profile maps the action in these contexts:

```text
vim_operator == a || vim_operator == i || vim_operator == cs ||
vim_operator == helix_next || vim_operator == helix_previous
```

For Vim, `i` selects inside the angle pair and `a` selects around it. The
object participates in the active operator: for example, `ci<` changes the
text inside a pair and `da>` deletes the pair and its contents. The same object
can drive supported delete, yank, change, indentation, case conversion,
comment, surround, and rewrap operations. In Helix next/previous-object
contexts, the action selects the matching object in the requested direction.
In visual modes, the object can expand the current selection. `cs` routes the
matched pair into the change-surround continuation; the `opening` argument is
carried into that replacement flow and affects its whitespace handling.

`Object::AngleBrackets` uses the generic surrounding-marker scanner with `<`
and `>` and is classified as multiline. The scanner balances nested pairs,
ignores a delimiter preceded by a backslash, searches across lines, and can
look forward to a subsequent opening pair when it finds no enclosing opener.
An inside selection excludes the delimiters; an around selection includes
them. For multiline inside content, leading and trailing whitespace are
trimmed when the body contains non-whitespace text. The multiline path does
not expand an around selection to adjacent spaces. This is character-based
matching rather than language-aware parsing, so comparison operators and
malformed angle-bracket text need native characterization.

The normal-object handler consumes an optional count and passes it through the
operator pipeline, but the `AngleBrackets` branch of `Object::range` does not
read `times`; the inspected source therefore selects one matching pair rather
than expanding the object by a numeric count.

If no complete pair is found, the object range is absent. The normal operator
path clears the pending operator without applying an edit; visual selection
remains unchanged. The `<` and `>` actions are only statically bound in the
shared mode layer, not in platform defaults. The mode layer is specified to
load when `vim_mode` or `helix_mode` is enabled. Both pinned settings are
user-scoped with a false default; their exact UI widgets/conditions and
combined-mode precedence remain pending in the [Settings requirement
inventory](settings-requirements.md).

## Labonair owner and current route

`crates/editor/src/vim.rs` implements a compact Editor-owned Vim state machine
with normal/insert/visual/command modes, basic counted motions, d/c/y motion
operators, and editing commands. `crates/workspace/src/views/editor.rs`
forwards Editor keystrokes to it when the Editor-owned `editor.vimMode`
preference is enabled. The machine has no operator-object state, `AngleBrackets`
action, or text-object scanner; `<` and `>` therefore do not select an angle
pair through this route. The local Keymap has no selected base-profile overlay
loader, and no Helix mode was found. Basic Vim motion/operator support is a
partial counterpart, not implementation of this action.

The canonical future behavior remains Editor-owned in `labonair-editor`;
`labonair-keymap` supplies the active bindings and contexts. Workspace may
forward input and compose the view, but must not own text-object semantics.
No new Setting or Command Palette route is implied by this action.

## Acceptance still required

- Enable Vim and Helix separately, load the shared mode layer, and verify the
  effective `<` and `>` bindings, action arguments, conflicts, user overrides,
  and behavior when both mode settings are enabled.
- Exercise `i<`, `a<`, `ci<`, `ca<`, `di<`, `da>`, yank, supported object
  operators, change-surround replacement, Helix next/previous selection, and
  visual-mode expansion. Confirm native count behavior against the static
  source, which consumes the count but does not apply it in the angle-object
  range branch; verify operator cleanup.
- Verify caret positions on opening/closing delimiters, nested pairs, escaped
  delimiters, unmatched open/close markers, multiline content, empty pairs,
  whitespace-only bodies, adjacent spaces, and ordinary comparison operators.
- Confirm undo/redo, registers, cursor placement, selection direction, and
  buffer save state for every mutating operator.
- Verify the three platform defaults remain unaffected and test the shared
  conditional layer independently on macOS, Linux, and Windows. Static rows
  are not dispatch evidence.
- Compare normal, visual, operator-pending, and change-surround interactions
  against paired native macOS evidence. Record the effective keymap profile,
  Vim/Helix settings, platform, viewport, and source text.
- Before local implementation, preserve the Editor/Keymap owner boundary and
  keep matching responsive for large and multiline buffers.

## Source records inspected

- Action argument, registration, object type, range dispatch, and delimiter
  scanner: `zed-refrence/zed/crates/vim/src/object.rs`.
- Operator execution and unmatched-object cleanup:
  `zed-refrence/zed/crates/vim/src/normal.rs` and `visual.rs`.
- Change-surround continuation and `opening` behavior:
  `zed-refrence/zed/crates/vim/src/surrounds.rs`.
- Shared profile keys and contexts: `zed-refrence/zed/assets/keymaps/vim.json`.
- Action inventory: [`registered-actions-unbound.md`](registered-actions-unbound.md).
- Conditional layer and exact binding rows:
  [`keymap-overlay-requirements.md`](keymap-overlay-requirements.md) and
  [`keymap-nonmacos-requirements.md`](keymap-nonmacos-requirements.md).
- Mode settings: [`settings-requirements.md`](settings-requirements.md).
- Labonair Editor owner and partial behavior: [`docs/capabilities.md`](../capabilities.md),
  `crates/editor/src/vim.rs`, `crates/workspace/src/views/editor.rs`, and
  [`feature-crosswalk.md`](feature-crosswalk.md).
