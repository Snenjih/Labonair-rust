# Vim GoToPreviousReference Action Classification

**Status:** Counted backward reference cycling and Labonair's first-result route classified; cross-file, selection/mode, platform, and native acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-GoToPreviousReference`; shared Vim binding `KEYBIND-VIM-HELIX-040b589c9bc9`

**Owner:** Editor owns reference navigation and cursor/selection state; Vim owns the pending count and profile behavior; Keymap owns the shared Vim/Helix route; Settings owns Vim mode/profile values. The language service supplies LSP reference locations through the Editor contract.

## Evidence boundary

The pinned Linux action registry declares `vim::GoToPreviousReference` as a unit action with no JSON arguments or deprecated aliases. It is absent from the platform-default keymap rows; the shared Vim/Helix profile binds `[ r` when `vim_mode` is `normal`, `visual`, or `operator`. The neighboring `] r` binding dispatches the separate next-reference action. Because this action has no required arguments, the generic `vim:` Command Palette action list can construct it when the Vim namespace is available. Palette discovery and direct invocation remain runtime acceptance.

The action consumes the pending Vim count, defaulting to one, and calls the Editor's asynchronous reference-navigation helper in the previous direction. The helper requests LSP references at the newest selection's head, including the declaration. It retains locations mappable into the active MultiBuffer, sorts them by position, and finds the result containing the current head. For a retained list of length `N`, current index `i`, and effective count `c`, the previous target is `(i + N - (c mod N)) mod N`. This walks backward and wraps from the first result to the last without adding the raw count to the index.

When no locations are returned, or no returned range contains the current head, the helper does not move. A destination equal to the current result also does not move. Otherwise it unfolds the destination, scrolls it into view, and replaces the selection set with one caret at the destination range's start. The action uses direct Editor navigation, not the Vim definition-navigation history path. The helper is asynchronous; the action handler detaches and logs request errors. The static path does not settle cross-file/multibuffer availability or visible failure behavior.

## Pinned backward-reference workflow

In Vim Control mode, `[ r` asks the language service for references to the symbol under the current selection head and moves to the previous retained reference. A numeric prefix steps backward by that many results; the default steps once. Results are ordered by their positions in the active MultiBuffer and wrap from the first to the last. The LSP request includes the declaration, so it can be a destination when returned by the server.

This is direct reference navigation, not a Vim range motion. The profile binding is available in Normal, Visual, and Operator contexts, but the helper's success path collapses the selection set to a single caret. Exact mode transition, pending-operator handling, forced motion, and multiple-caret outcomes need native acceptance. A one-result set, no result, or a current head outside returned ranges produces no movement in the statically observed helper.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-GoToPreviousReference](registered-actions-unbound.md) | Registered zero-argument action; consumes the count and requests backward navigation from the current selection head. | `labonair-editor` exposes an Editor Go to References command, but it jumps to the first returned target rather than cycling relative to the current reference. |
| [KEYBIND-VIM-HELIX-040b589c9bc9](keymap-overlay-requirements.md) | Shared `[ r` route for Vim Normal, Visual, and Operator contexts; count moves backward through sorted mappable references and wraps. | No local `[ r` route or GoToPreviousReference action was found. |
| [SET-keymap-vim-mode](settings-requirements.md) | Vim mode/profile enables the shared keymap and `vim:` Command Palette namespace; no reference-cycling-specific setting exists. | Local Vim mode exists, but it has no equivalent cycling binding or action. |

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the future `[ r` binding. Labonair's existing `EditorGoToReferences` command reaches its language-service `References` request, then the response handler selects `targets.first()`, moves the caret to that target's selection start, and ensures it is visible. That related LSP workflow does not cycle backward from the current occurrence and has no Vim count or wrap behavior. No new setting, event, notification, persistence, dependency, or control is needed for this action.

## Pinned fixture inspected

- `editor_tests.rs::test_next_prev_reference` supplies four same-buffer reference ranges. It verifies next/previous one-step movement, wrap in both directions, a forward count of three, and a backward count of two. The fixture exercises the shared Editor helper, not the Vim `[ r` action, cross-file mapping, no-result behavior, selections, operator context, or platform key dispatch.

Pinned source and fixtures were inspected, not run. No focused `GoToPreviousReference` key-action or cross-file cycle fixture was found.

## Acceptance still required

- Dispatch `[ r` on macOS, Linux, and Windows under the shared Vim profile. Verify the physical key sequence for each layout, profile overrides, Vim mode, menu context, and direct `vim:` palette discovery/invocation.
- Verify no count, `1[r`, larger counts, counts greater than the result set, zero/count reset behavior, and the paired `]r` route. Record each destination and wrap point.
- Verify provider-supported and unsupported documents, no symbol under cursor, no references, duplicate/overlapping ranges, cursor positions inside and outside returned ranges, declaration inclusion, and asynchronous errors/cancellation.
- Verify results in the active buffer, open buffers, excerpts, other files, remote buffers, and unloaded files. Record which locations the active MultiBuffer can map and whether navigation should open an external target.
- Verify Normal, Visual, Visual Line, Visual Block, Operator, and Helix modes; single/multiple/reversed selections; pending and forced operators; focus; mode transitions; scroll/fold effects; and undo/jump history. The statically observed success path replaces the selection set with one caret.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, language server/capabilities, returned locations, starting head, count, destination, selection/mode, viewport, theme, and scale recorded.

## Source records inspected

- Action schema, count consumption, and backward handler: `zed-refrence/zed/crates/vim/src/normal.rs`.
- Shared `[ r`/`] r` profile routes and mode contexts: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Reference request, location mapping/sorting, previous count formula, no-result paths, unfold/scroll, and caret selection: `zed-refrence/zed/crates/editor/src/navigation.rs`.
- LSP `include_declaration: true` request and result conversion: `zed-refrence/zed/crates/project/src/lsp_command.rs`.
- Four-location forward/backward traversal and count fixture: `zed-refrence/zed/crates/editor/src/editor_tests.rs::test_next_prev_reference`.
- Local Vim route absence and existing first-result LSP command: `crates/editor/src/command_provider.rs`, `crates/workspace/src/views/editor.rs`, and `crates/workspace/src/workspace.rs`.
- Linux action metadata: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
