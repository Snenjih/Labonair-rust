# Vim GoToNextReference Action Classification

**Status:** Counted forward reference cycling and Labonair's first-result route classified; cross-file, selection/mode, platform, and native acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-GoToNextReference`; shared Vim binding `KEYBIND-VIM-HELIX-45ebf38d3848`

**Owner:** Editor owns reference navigation and cursor/selection state; Vim owns the pending count and profile behavior; Keymap owns the shared Vim/Helix route; Settings owns Vim mode/profile values. The language service supplies LSP reference locations through the Editor contract.

## Evidence boundary

The pinned Linux action registry declares `vim::GoToNextReference` as a unit action with no JSON arguments or deprecated aliases. It is absent from the macOS, Linux, and Windows default keymap files. The shared Vim/Helix profile binds `] r` when `vim_mode` is `normal`, `visual`, or `operator`; the neighboring `[ r` binding dispatches the separate previous-reference action. Because the action has no required arguments, the generic `vim:` Command Palette action list can construct it when the Vim namespace is available. Palette discovery and direct invocation remain runtime acceptance.

The action consumes the pending Vim count, defaulting to one, and calls the Editor's asynchronous reference-navigation helper in the forward direction. The helper asks the Project for LSP references at the newest selection's head. It retains references that can be mapped into the active MultiBuffer, sorts them by position, finds the result containing the current head, and moves `count` entries forward modulo the retained list. The LSP request includes the declaration. If no references are returned, or no returned range contains the current head, the helper does not move. A destination equal to the current result also does not move.

For a destination, the helper unfolds its range, scrolls it into view, and replaces the selection set with one caret at the destination range's start. It does not perform a text edit or use the normal definition-navigation history path. The action is asynchronous; the action handler detaches and logs request errors. The static path establishes count, ordering, wrap, and collapse behavior, but not which cross-file results remain mappable for each workspace/multibuffer configuration or the visible failure behavior.

## Pinned forward-reference workflow

In Vim Control mode, `] r` asks the language service for references to the symbol under the current selection head and moves to the next reference. A numeric prefix advances by that many results; the default advances once. The retained results are ordered by their positions in the active MultiBuffer and wrap from the last to the first. The helper includes the declaration in the LSP request, so it can be one of the destinations when the server returns it.

This is direct reference navigation, not a Vim range motion. The binding is also present in Visual and Operator contexts, but the action handler delegates to the direct navigation helper, which collapses the selection set to a single caret on success. Exact mode transition, pending-operator handling, forced-motion behavior, and multiple-caret outcomes need native acceptance. When the cursor is not inside any mapped result or the provider returns no locations, the pinned helper returns without moving.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-GoToNextReference](registered-actions-unbound.md) | Registered zero-argument Vim action; consumes a count and requests forward reference navigation from the current selection head. | `labonair-editor` exposes an Editor Go to References command, but it jumps to the first returned target rather than cycling relative to the current reference. |
| [KEYBIND-VIM-HELIX-45ebf38d3848](keymap-overlay-requirements.md) | Shared `] r` route for Vim Normal, Visual, and Operator contexts; count advances through sorted mappable references and wraps. | No local `] r` route or GoToNextReference action was found. |
| [SET-keymap-vim-mode](settings-requirements.md) | Vim mode/profile enables the shared keymap and `vim:` Command Palette namespace; no reference-cycling-specific setting exists. | Local Vim mode exists, but it has no equivalent cycling binding or action. |

## Labonair owner and current route

The canonical behavior owner is `labonair-editor`; `labonair-keymap` owns the future `] r` binding. The Editor language-service contract can request references and returns `LanguageServiceResponse::References`. Workspace routes the existing `EditorGoToReferences` command through that contract. Its response handler selects `targets.first()`, moves the caret to that target's selection start, and ensures it is visible. This gives Labonair a related LSP capability, but no counted cycle from the current occurrence, wrap behavior, or Vim binding. No new setting, event, notification, persistence, dependency, or control is needed for this action.

## Pinned fixture inspected

- `editor_tests.rs::test_next_prev_reference` supplies four same-buffer LSP reference ranges and verifies forward and backward one-step traversal, wrap in each direction, a forward count of three, and a backward count of two. The fixture exercises the shared Editor helper, not the Vim `] r` action, cross-file mapping, no-result behavior, selections, operator context, or platform key dispatch.

Pinned source and fixtures were inspected, not run. No focused `GoToNextReference` key-action or cross-file cycle fixture was found.

## Acceptance still required

- Dispatch `] r` on macOS, Linux, and Windows under the shared Vim profile. Verify the physical key sequence for each layout, profile overrides, Vim mode, menu context, and direct `vim:` palette discovery/invocation.
- Verify no count, `1]r`, larger counts, counts greater than the reference set, zero/count reset behavior, and the paired `[r` route. Record each destination and wrap point.
- Verify provider-supported and unsupported documents, no symbol under cursor, no references, duplicate and overlapping reference ranges, cursor positions within and outside returned ranges, current declaration inclusion, and asynchronous errors/cancellation.
- Verify results in the active buffer, open buffers, excerpts, other files, remote buffers, and unloaded files. Record which locations the active MultiBuffer can map and whether navigation should open an external target.
- Verify Normal, Visual, Visual Line, Visual Block, Operator, and Helix modes; single and multiple selections; reversed selections; pending and forced operators; focus; mode transitions; scroll and fold effects; and undo/jump history. The statically observed success path replaces the selection set with one caret.
- Capture paired native macOS Editor and Keymap evidence with platform, profile, language server/capabilities, returned locations, starting head, count, destination, selection/mode, viewport, theme, and scale recorded.

## Source records inspected

- Vim action schema, count consumption, and handler: `zed-refrence/zed/crates/vim/src/normal.rs`.
- Shared `[ r` and `] r` routes and mode contexts: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Reference request, location sorting/mapping, count cycling, no-result paths, unfold/scroll, and caret selection: `zed-refrence/zed/crates/editor/src/navigation.rs`.
- LSP `include_declaration: true` request and result conversion: `zed-refrence/zed/crates/project/src/lsp_command.rs`.
- Four-location traversal/count fixture: `zed-refrence/zed/crates/editor/src/editor_tests.rs::test_next_prev_reference`.
- Local Vim route absence and existing first-result LSP command: `crates/editor/src/command_provider.rs`, `crates/workspace/src/views/editor.rs`, and `crates/workspace/src/workspace.rs`.
- Linux action metadata: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
