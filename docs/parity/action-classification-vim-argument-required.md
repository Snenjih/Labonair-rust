# Vim ArgumentRequired Action Classification

**Status:** Static action registration, Ex-command route, and prompt behavior
classified; native command/prompt acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned Vim owns Ex-command parsing and its error action; the focused
Editor registers the action handler; Keymap owns user bindings; the Command
Palette owns action discovery. Labonair's Editor module owns the command and
mark workflow, while Notifications owns user-visible error messages.

## Evidence boundary

The Linux action inventory registers `vim::ArgumentRequired`, with a
documentation description and no argument schema. It has no binding in any
platform-default keymap or the active Vim/Helix layer. The action is declared
through the normal `actions!` registration path; it is not the private
`DeleteMarks` action type, which is separately excluded from name/JSON
registration.

With Vim or Helix enabled and an Editor focused, the Vim namespace is exposed
in the Command Palette. The registered action therefore has a static direct
palette route, and a user keymap could bind it. In the live Vim Ex-command
table, the concrete built-in route is `:delmarks` with no mark argument and no
bang. Static registration and command interception do not prove macOS action
registration, native prompt presentation, or dispatch.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-ArgumentRequired](registered-actions-unbound.md) | Registered zero-argument action that opens the required-argument prompt. It is reached by the focused Editor palette or by submitting `:delmarks` without a mark argument or bang. | `labonair-editor` has no `ArgumentRequired` action, Ex-command interpreter for `:delmarks`, or mark deletion workflow. |
| [SET-keymap-vim-mode](settings-requirements.md), [SET-keymap-helix-mode](settings-requirements.md) | User-scoped false-default mode values control whether the Vim namespace and shared mode layer are active. There is no default action binding. | Labonair has an Editor-owned partial Vim preference but no Helix namespace/profile layer or matching action. |
| Notification and dialog surface contracts | The pinned behavior is a blocking Critical prompt with one Cancel button. | Labonair routes user-facing error explanations through the statusbar Notification Center and has a shared modal dialog contract. The dismiss-only prompt has no local counterpart; reconcile its message and correction flow with those existing contracts before implementation. |

## Pinned behavior

The `ArgumentRequired` action handler calls `window.prompt` at
`PromptLevel::Critical` with the title `Argument required`, no detail text,
and a single `Cancel` button. It does not contain an input field or mutate
Editor or mark state. Dismissing it ends the prompt; the user must reopen the
Vim command input and submit a corrected command. Prompt focus, keyboard
dismissal, and visual treatment need native verification on the target
platforms.

The active Ex-command table maps the `delm`/`arks` pattern to this action when
the query has no arguments and no `!`. Thus `:delmarks` with an empty argument
shows the prompt. `:delmarks mark-name` creates a private `DeleteMarks::Marks`
action, and `:delmarks!` without an argument creates
`DeleteMarks::AllLocal`, which removes lowercase local marks without showing
the required-argument prompt. Invalid mark syntax follows a different path:
the `DeleteMarks` handler opens a Critical prompt titled `Invalid argument: …`.
That validation prompt is not `ArgumentRequired` and should not be conflated
with the missing-argument action.

`ArgumentRequired` is registered and can be surfaced by the Vim action
namespace, but it has no platform-default or built-in profile binding. Direct
palette invocation opens the same prompt without running `:delmarks`; a user
binding can do the same. No default key, menu item, or separate mark-action
command is implied by this action.

No focused pinned test for the missing-argument action or `:delmarks` prompt
was found in the inspected Vim command sources. The action handler and command
table were read but not executed. The Linux metadata dump confirms action
registration only; it does not confirm macOS registration or runtime UI.

## Labonair owner and current route

`crates/editor/src/vim.rs` has a compact Vim command path for settings,
substitution, save/quit/reload, and search-highlight dismissal. It has no
`:delmarks` parsing, mark deletion command, or `ArgumentRequired` action.
`crates/keymap` can contribute typed commands and user bindings, but no
ArgumentRequired descriptor or built-in Vim/Helix profile entry exists.

The behavior owner is `labonair-editor`; Keymap owns optional user binding,
and the existing Command Palette is the discovery surface. Labonair's
notification registry is the sole user-message path and its statusbar dropdown
is the only global notification presentation. Shared modal dialogs remain
available when a user must make a decision or correct input. The pinned
Cancel-only Critical prompt communicates an error but provides no correction
control, so parity needs an explicit owner decision: report the missing
argument through the Notification Center and preserve a clear retry path, or
justify a shared dialog that accepts a correction. Do not add a toast or a
feature-local duplicate message. This reconciliation remains open under the
Editor owner with Notification Center as the message owner.

The missing-argument action itself adds no setting, event, persistent state,
or security boundary. Mark persistence and `:delmarks` command semantics are
separate Editor requirements and must be inventoried with that command before
implementation.

## Acceptance still required

- Verify macOS action registration, Vim namespace visibility, direct palette
  invocation, focus restoration, prompt title/detail/button, and Cancel/Escape
  dismissal. Repeat action/prompt checks on Linux and Windows.
- In a focused Vim Editor, submit `:delmarks` and `:delmarks `, confirm the
  same Critical prompt, no mark mutation, and the ability to retry with a
  mark argument after dismissal. Verify active mode and excluded mode states.
- Verify `:delmarks mark-name` takes the mark-deletion path and
  `:delmarks!` takes the all-local-marks path without this prompt. Check that
  malformed mark arguments use the separate `Invalid argument: …` behavior.
- Confirm no platform-default or built-in Vim/Helix key binding exists. If a
  user binds the action, verify conflict reporting, precedence, save/reload,
  and the same prompt behavior.
- Resolve how the Labonair message uses the canonical Notification Center and
  whether any correction dialog is justified. Do not report parity complete
  until the user can understand the error and retry the command in the
  accepted flow.
- Capture paired native macOS command-palette and Ex-command interaction with
  Editor focus, mode setting, prompt state, viewport, theme, and scale
  recorded. Static action metadata is not visual or dispatch evidence.

## Source records inspected

- Action declaration, handler, action documentation, and Ex-command table:
  `zed-refrence/zed/crates/vim/src/command.rs`.
- Command parsing and palette interception:
  `zed-refrence/zed/crates/vim/src/command.rs` and
  `zed-refrence/zed/crates/vim/src/state.rs`.
- Action registration semantics: `zed-refrence/zed/crates/gpui/src/action.rs`
  and `zed-refrence/zed/crates/gpui_macros/src/derive_action.rs`.
- Linux action metadata:
  [`zed-actions-3569541-linux-x86_64.json`](evidence/zed-actions-3569541-linux-x86_64.json).
- Labonair Editor, Keymap, and message contracts:
  [`docs/capabilities.md`](../capabilities.md),
  [`docs/registries.md`](../registries.md), [`docs/product.md`](../product.md),
  [`docs/design-system.md`](../design-system.md),
  `crates/editor/src/vim.rs`, and `crates/keymap/src/command_provider.rs`.

Same-session pinned-source research and independently written requirements
are redevelopment, not separated-room clean-room implementation.
