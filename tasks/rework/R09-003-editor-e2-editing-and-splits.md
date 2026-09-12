# R09-003 — Editor E2: pro editing and internal split interaction

## Status

⏳ Planned

## Owner

- Module: editor
- Capability-matrix row: [Editor](../../docs/capabilities.md)
- Composition entry point: Workspace hosts the Editor tab; labonair-editor registers the Editor view and actions

## Dependencies

- R09-001-editor-e0-text-model-contract
- R09-002-editor-e1-file-lifecycle
- ADR 0004 editor-internal splits
- R07-001-product-surface-acceptance must be complete.

## Goal

Deliver reliable multi-selection editing and editor-internal split groups while
preserving the existing keyboard-first Editor workflow and the separation
between Editor groups and Workspace capability panes.

## Scope

- In scope: multiple selections and cursors, rectangular selections where
  supported by the model, occurrence selection, line/block editing, smart
  indentation, bracket/quote handling, clipboard behavior, repeatable edit
  transactions, EditorSplitTree, focus/close/merge behavior, and narrow
  layout states.
- Out of scope: LSP completion, Git operations, remote collaboration,
  extension APIs, and a second Workspace layout system.

## Contracts and ownership

- Public domain values: MultiSelection, SelectionSet, EditIntent,
  EditorGroupId, EditorSplitTree, SplitOrientation, EditorViewSnapshot, and
  EditorViewEvent in labonair-editor.
- Service traits or typed events: EditorCommandSink for typed editing and
  split intents; EditorSnapshotStream for immutable updates. Workspace receives
  only open/focus/close results and does not mutate groups.
- Registry contributions: Editor contributes stable command descriptors for
  add cursor, select next occurrence, select all occurrences, expand/shrink
  selection, duplicate/move lines, indent/outdent, split right/down, focus
  next/previous group, close group, and close other groups.
- UI surface: Editor tab or standalone Editor with internal groups. No new
  permanent shell zone; cross-capability splits remain Workspace-owned.
- Shared UI-kit components: split divider/resizer if available, focusable
  surfaces, menus, context menus, buttons, tabs, scroll containers, and
  keyboard hints. Text selection and gutter rendering remain Editor-owned.

## Commands and keymap

All actions use the Editor context and owner-provided default bindings through
labonair-command-palette-core and labonair-keymap. Existing undo/redo,
selection, line movement, and clipboard identities are preserved. The action
IDs introduced by ADR 0004 are canonical and must not be mirrored in
Workspace. Vim mode maps its modal behavior to the same semantic intents where
possible.

## Settings

Existing editorTabSize, editorIndentWithTabs, editorWordWrap, and Vim settings
remain unchanged. No new persistent setting is necessary for the initial split
tree: split orientation and active group are session state, not user
preferences. If a user-facing toggle is proven necessary, add it only through
the Settings inventory with type, scope, default, validation, migration, and a
consumer test.

## Persistence and migration

- Storage: live selection and split state are held by labonair-editor. Durable
  editor-session serialization is implemented in E6.
- Migration: before E6, all restored Editor tabs start with one group and one
  view; no split state is inferred from Workspace panes.
- Compatibility: the current single-cursor adapter may translate to a
  one-entry SelectionSet until every consumer uses the new contract. Remove
  that adapter after syntax, search, Vim, and rendering consume SelectionSet.

## Notifications

Normal editing and split actions are silent. Invalid close/save decisions may
use a local UI-kit dialog; operation failures are structured notifications
through labonair-notifications-core. No split-specific toast or passive banner
is introduced.

## Dependency restrictions

- Keep the split tree and editing behavior in the Editor owner.
- Do not make labonair-editor depend on labonair-workspace or labonair-shell.
- Do not reuse private Workspace pane nodes or introduce a second command table.
- Any GPUI-specific view remains an Editor-owned boundary; reusable controls
  come from labonair-ui-kit.

## Implementation plan

1. Implement SelectionSet and semantic edit intents on the E0 transaction
   contract → verify multi-cursor insertion/deletion, overlap normalization,
   Unicode, undo/redo, and clipboard tests.
2. Add line/block operations, auto-indent, bracket matching/closing, and
   Vim semantic mappings → verify keyboard and Vim regression tests.
3. Implement EditorSplitTree and its view adapter according to ADR 0004 →
   verify focus, split, close, merge, narrow-width, empty-group, and
   cross-capability isolation tests plus a native visual check.
4. Register commands/keymap contributions and remove single-cursor dispatch
   paths → verify registry, conflict, palette, and source audits.

## Implementation evidence (2026-09-12)

- `labonair-editor` now exposes `EditIntent`, `EditorCommand`,
  `EditorCommandSink`, `EditorSnapshotStream`, and `EditorViewEvent`.
- `Document` applies multi-selection insert/delete/edit transactions through
  the existing Rope and transaction history. Selection normalization,
  occurrence selection, movement, expansion/shrink, line duplication,
  transpose, comment toggling, indentation, bracket auto-close/skip-over,
  bracket matching, and a small typed snippet session are covered by focused
  unit tests.
- `EditorSplitTree` owns split orientation, focus, open, close/merge, close
  others, and clamped resize transitions. Nested-tree invariants and focus
  behavior are covered by focused unit tests.
- Editor command metadata is contributed through the existing command provider
  with stable `editor.*` identities. No Workspace split tree or shell chrome
  was added.

The core and Workspace GPUI adapter now consume the typed selection, display,
and command contracts; durable split/session persistence remains R09-007.
R07-001 is still incomplete, so this task is not marked complete and the
active queue remains truthful.

## Implementation evidence (command bridge)

The active-tab command bridge is now wired through the canonical
`CommandHandlerRegistry`. The Editor owner publishes the executable command
inventory and typed `EditorCommandRoute`; Workspace only resolves the active
editor entity and forwards edit, split, language-service, or Git intents.
Non-editor tabs return `NoActiveEditor`, unknown IDs return a typed unsupported
outcome, and EditorView reports unavailable language-service/Git operations via
the existing notification center. Git change navigation and Project Diff use
the existing revision-bound bridge. The shell registers this contribution only
when composing a live Workspace entity; it does not keep a parallel table.

Focused route and descriptor tests cover non-editor IDs, occurrence/Git route
mapping, descriptor coverage, and duplicate inventory detection. The task
remains formally Planned while R07-001 is open.

## Acceptance criteria

- [ ] Multi-selection operations are deterministic, transactionally undoable,
      and work with Unicode and folded/wrapped views.
- [ ] Core editing operations cover line/block duplication, movement,
      indentation, bracket handling, and clipboard semantics.
- [ ] Internal splits implement the ADR tree, focus, close, merge, and
      narrow/empty states without changing Workspace pane ownership.
- [ ] All commands are owner-registered and keymap overrides work.
- [ ] Shared controls use labonair-ui-kit and no duplicate shell chrome exists.
- [ ] Focused core, Vim, registry, integration, and visual tests pass.
- [ ] Full repository, dependency, documentation, queue, and diff checks pass.

## Removal / exit condition

The task is complete when the Editor has no single-cursor-only compatibility
path, internal split state is exclusively Editor-owned, all new actions are
registered through the canonical registries, and Workspace has no
editor-specific split conditionals.

## Notes and follow-ups

E3 consumes SelectionSet and EditorViewSnapshot for display mapping, folding,
search, and navigation. E6 owns durable split/session persistence.

## Implementation evidence (visible split surface)

The Workspace GPUI adapter now recursively renders the Editor-owned split tree
inside the editor surface: each nested node keeps its own orientation and
ratio, the focused group receives the interactive editor, sibling groups
render a revision-consistent read-only mirror, and each group exposes a focus
target, active header, and close control. Divider drag uses frame-local typed
paths, while session normalization rejects invalid split data. This is
intentionally a composition adapter; Workspace does not own split state or
commands.
