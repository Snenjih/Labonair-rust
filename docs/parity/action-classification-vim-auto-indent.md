# Vim AutoIndent Action Classification

**Status:** Static action, shared-profile, and owner-route behavior classified; native dispatch and mode-transition acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Editor owns syntax-based reindentation, selections, and modal state; Keymap owns profile and user bindings; Settings owns mode and language indentation values; the focused Editor's Command Palette provides existing Vim action discovery.

## Evidence boundary

The pinned Linux action inventory registers `vim::AutoIndent`. It has no
binding in the macOS, Linux, or Windows platform-default maps. The shared
`vim.json` layer binds `=` in Vim Visual mode and Helix Normal/Select mode.
With Vim or Helix enabled and an Editor focused, the registered Vim actions
also have a static Command Palette route. These facts establish registration
and profile data, not macOS registration, active-profile loading, key dispatch,
or native mode behavior.

Vim Normal `=` is a separate path: it pushes `vim::PushAutoIndent`, then waits
for an operator motion. It is not the direct Visual/Helix `vim::AutoIndent`
binding. Both paths ultimately call the Editor's syntax-based autoindent
operation, but they have distinct action and keymap contracts.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-AutoIndent](registered-actions-unbound.md) | Registered unit action; direct `=` binding in Vim Visual and Helix Normal/Select; focused-Editor palette discovery when Vim/Helix is enabled. | `labonair-editor` has no registered syntax-reindent action or selected-line syntax reindent implementation. Its internal `EditIntent::AutoIndent` only inserts indentation at empty cursors and has no dedicated command/keymap/palette route. |
| [KEYBIND-VIM-HELIX-eb4a7edc66e0](keymap-overlay-requirements.md) and [KEYBIND-VIM-HELIX-6fec4f2e94e5](keymap-overlay-requirements.md) | The shared mode layer binds `=` for `vim_mode == visual` and `(vim_mode == helix_normal || vim_mode == helix_select) && !menu`. Neither is a platform-default binding. | Labonair Keymap currently has no Vim/Helix profile layer. Linux and Windows share the pinned data but still need separate native dispatch verification. |
| [SET-LANGUAGE-FIELD-auto-indent](settings-requirements.md) | Direct range reindent computes indentation from syntax `indents_config` queries and language indent size. The `auto_indent` mode setting is used on the automatic newline-insertion path; it is not consulted by the inspected `autoindent_ranges` path. | Labonair Settings has a global editor tab-size value, but no language-specific indentation values or syntax-query-backed reindent route. Do not treat local insertion indentation as this action. |

## Pinned behavior

The direct handler records the current action, consumes the repeat count
(default one), takes any forced motion, stores visual marks, and saves selection
starts. It calls the Editor's autoindent operation once per count within an
Editor transaction, then restores the selection cursors. If the current mode
is visual, it requests a switch to `Mode::Normal` after the operation. The
mode predicate includes Vim Visual, Visual Line, Visual Block, and Helix
Select; therefore the Helix Select binding also requests Vim Normal. Native
mode display and whether this transition is intended need runtime acceptance.

The Editor action returns without editing in read-only buffers. Single-line
Editors propagate the action instead of reindenting. Otherwise it passes the
current selection ranges to the multi-buffer layer, which groups overlapping
ranges by underlying buffer and queues language-buffer autoindent requests.
Language buffers use syntax-tree indentation queries and language indent size
to calculate suggestions, skip empty lines for this action, and apply edits
where the suggested indentation differs. A buffer with no syntax snapshot
does not produce suggestions. The action itself does not add an error or
notification when these paths leave the text unchanged.

In Helix Normal, the action runs without the handler's visual-mode transition;
in Helix Select, it runs and then requests Vim Normal. This distinction, the
preserved cursor positions, multi-selection behavior, and count handling must
be checked in runtime acceptance.

The separate Vim Normal route binds `=` to `vim::PushAutoIndent` and applies
syntax indentation to the range created by a following motion. The pinned
`test_autoindent_op` fixture exercises `=ap`. Other inspected indentation
fixtures cover visual-repeat indentation and Helix `>`/`<`, but no focused
test for direct `vim::AutoIndent` was found. None of these fixtures were run.

The registered action is not a platform-default shortcut or menu command.
The shared mode binding and conditional focused-Editor Command Palette are
its built-in direct entry points; a custom user binding can be managed through
Keymap. No additional direct Vim Normal `=` binding should be added for this
action because that chord starts the distinct operator workflow.

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; Keymap owns mode-profile
composition and optional user bindings. Settings owns mode values and any
future language indentation values. Command Palette remains the existing
discovery surface. No shell condition, event, new registry, notification, or
persistent selection state is needed for the action.

The local `EditIntent::AutoIndent { unit }` is implemented in
`crates/editor/src/editing.rs`. It inserts a whitespace prefix only at empty
selections, deriving that prefix from leading whitespace and a small set of
preceding-line delimiters. It does not recompute indentation for selected
existing lines. The intent is not registered in the Editor command provider
or exposed as a Vim/Helix action. Local `editor_tab_size` is global; the local
Settings and language-service sources have no corresponding language-level
indent-size/AutoIndentMode model for this workflow.

The similarly named pinned `auto_indent` language setting describes the
automatic newline indentation mode. The inspected selected-range reindent
path instead obtains syntax indentation suggestions and language indent size;
the action classification does not add a setting or conflate those two
behaviors. Normal document transactions remain the persistence route for
resulting text edits.

## Acceptance still required

- Verify the `=` binding in Vim Visual and Helix Normal/Select with the mode
  setting enabled and disabled, including excluded contexts and menu state.
  Repeat profile dispatch separately on macOS, Linux, and Windows.
- Verify direct Command Palette discovery and dispatch with Vim/Helix enabled,
  focused-Editor scoping, and a user override's save/reload and conflict
  behavior.
- Confirm the syntax result for single and multiple selections, empty lines,
  no syntax support, multiple language excerpts, read-only buffers, and
  single-line Editors. Verify selection cursor restoration and repeated
  invocation/count behavior.
- Verify the post-action mode in Vim Visual, Visual Line, Visual Block, Helix
  Normal, and Helix Select. Resolve whether Helix Select should enter Vim
  Normal or Helix Normal after direct autoindent.
- Exercise Vim Normal `=ap` separately from the direct action and confirm the
  `PushAutoIndent` operator route remains distinct.
- Capture paired native macOS Editor and Keymap evidence with platform,
  profile, mode setting, syntax/language, selection, viewport, theme, and
  scale recorded. Static map rows do not establish visual or dispatch parity.
- Implement and test the owner route only after the Editor text model, syntax
  service, language indentation settings, and canonical Vim/Helix Keymap
  contracts are available.

## Source records inspected

- Action declaration, direct handler, operator helper, and fixtures:
  `zed-refrence/zed/crates/vim/src/indent.rs`.
- Vim mode predicate and mode transition:
  `zed-refrence/zed/crates/vim/src/state.rs` and `vim.rs`.
- Normal-mode `PushAutoIndent` action/operator mapping and operator fixture:
  `zed-refrence/zed/crates/vim/src/vim.rs`, `normal.rs`, and `state.rs`.
- Editor read-only/single-line behavior and selection forwarding:
  `zed-refrence/zed/crates/editor/src/editor.rs`.
- Multi-buffer range routing and language syntax indentation requests:
  `zed-refrence/zed/crates/multi_buffer/src/multi_buffer.rs` and
  `zed-refrence/zed/crates/language/src/buffer.rs`.
- Shared keymap and Linux registration evidence:
  `zed-refrence/zed/assets/keymaps/vim.json` and
  `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Local editing intent, command provider, Settings, and Keymap sources:
  `crates/editor/src/editing.rs`, `crates/editor/src/command_provider.rs`,
  `crates/settings/src`, and `crates/keymap/src`.

Same-session pinned-source research and independently written requirements
are redevelopment, not separated-room clean-room implementation. This packet
records static evidence and does not claim runtime acceptance.
