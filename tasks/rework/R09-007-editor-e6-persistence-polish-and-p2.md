# R09-007 — Editor E6: persistence, polish, and deferred P2 capabilities

## Status

⏳ Planned

## Owner

- Module: editor
- Capability-matrix row: [Editor](../../docs/capabilities.md)
- Composition entry point: labonair-shell composes the Editor session store and Workspace Editor tab adapter

## Dependencies

- R09-001-editor-e0-text-model-contract
- R09-002-editor-e1-file-lifecycle
- R09-003-editor-e2-editing-and-splits
- R09-004-editor-e3-display-search-navigation
- R09-005-editor-e4-local-language-services
- R09-006-editor-e5-git-gutter-review-bridge
- R07-001-product-surface-acceptance must be complete.

## Goal

Complete durable Editor session behavior, opt-in productivity settings, native
formatting/autosave, accessibility and visual polish, and the remaining
bounded P2 capabilities without creating a second owner or hiding unfinished
behavior behind settings.

## Scope

- In scope: versioned Editor session persistence, internal split persistence,
  selections/folds/scroll restoration, unsaved-buffer recovery policy,
  autosave, format-on-save, trailing-whitespace trimming, final-newline
  policy, editor minimap/sticky polish, whitespace visualization, accessibility,
  expanded Vim behavior, breadcrumbs, inlay/inline presentation polish, and
  final visual verification.
- Out of scope: remote Editor/LSP, collaboration, extensions/marketplace,
  remote theme downloads, a new shell zone, and any unsupported feature
  presented as enabled.

## Contracts and ownership

- Public domain values: versioned EditorSessionSnapshot, EditorSessionId,
  serialized split/view state, SavePolicy, FormatPolicy, RecoveryDecision,
  and typed EditorPersistenceEvent in labonair-editor.
- Service traits or typed events: EditorSessionStore for versioned load/save/
  migrate, FormatterProvider for local formatting, and RecoveryEvent for
  explicit restore decisions. Filesystem remains the storage adapter;
  language services may provide formatting through a typed Editor contract.
- Registry contributions: Editor registers autosave/formatting/toggle,
  recovery, Vim, breadcrumb, and display commands. Formatter providers use
  the existing language-service contract; no generic registry is added unless
  multiple real providers require discovery.
- UI surface: Editor tab/standalone Editor, recovery dialog, formatting
  decision state, and existing notification dropdown. No permanent shell
  chrome is added.
- Shared UI-kit components: dialogs, buttons, fields, menus, tabs, list rows,
  badges, tooltips, keyboard hints, empty/loading/error states, and scroll
  containers. Editor-specific text, minimap, gutter, and inline rendering use
  design tokens.

## Commands and keymap

Register stable Editor actions for Save All, Format Document, Format
Selection, Toggle Format on Save, Toggle Autosave, Recover Unsaved Buffer,
Discard Recovery, Toggle Whitespace, Toggle Minimap, Toggle Sticky Context,
Vim macro/register/mark operations, and breadcrumb navigation as implemented.
Format and autosave actions must not be palette-visible without handlers.
Defaults come from owner descriptors and can be overridden through the
canonical keymap; no legacy shortcut table is reintroduced.

## Settings

All settings below are value-only fields. They require a Settings inventory
row, generated UI or a justified custom field, global/project scope decision,
validation, reset behavior, idempotent migration, and a focused consumer test
in the same code change:

| Key | Type / default | Scope | Migration and test |
|---|---|---|---|
| editorAutoSave | bool / false | Global + safe Project | Promote retained legacy input; default off, validate bool, test debounce/cancel/failure |
| editorAutoSaveDelay | integer milliseconds / 1000 | Global + safe Project | Promote retained legacy input; clamp 250–60000, test timing without sleeping foreground |
| editorFormatOnSave | bool / false | Global + safe Project | Promote retained legacy input; default off, test formatter success/failure and save ordering |
| editorTrimTrailingWhitespace | bool / false | Global + safe Project | Promote retained legacy input; default off, test line-ending-safe save |
| editorInsertFinalNewline | bool / false | Global + safe Project | Promote retained legacy input; default off, test empty/non-empty and EOL preservation |
| editorShowCursorPosition | bool / true | Global | Promote retained legacy input; default true, test status presentation |
| editorShowSelectionStats | bool / false | Global | Promote retained legacy input; default false, test multi-selection presentation |

Existing editorFontFamily, editorFontSize, editorLineHeight, editorTabSize,
editorWordWrap, editorLineNumbers, editorRelativeLineNumbers,
editorIndentWithTabs, editorVimMode, Vim settings, and editorTheme retain
their current values, defaults, scope, and migrations. Settings ownership does
not include session layout, recovery contents, language-server processes,
formatter executables, or Git state.

## Persistence and migration

- Storage: Editor owns a versioned session payload containing file identity,
  internal split tree, active group, selections, folds, scroll offsets,
  language ID, and safe unsaved-buffer recovery metadata. Workspace persists
  outer tab identity and embeds this payload through the typed boundary.
- Schema: start at editor-session version 2. Version 1 path-only snapshots
  migrate to one group/one leaf, empty selections, no folds, and clamped
  scroll. Invalid records fall back to one leaf and publish a recoverable
  notification; they never delete or overwrite the source file.
- Recovery: unsaved contents are stored only in the local session store under
  the existing persistence policy, with no secrets. Recovery requires an
  explicit user decision and keeps the original file conflict checks.
- Compatibility: retain versioned readers only for supported prior schemas.
  Remove migration branches when their retention window and user-data policy
  allow it; never keep a parallel live persistence format.

## Notifications

Autosave, formatting, recovery, migration, and persistence failures publish
structured notifications through labonair-notifications-core with source,
details, deduplication, and bounded actions. Successful autosave remains quiet.
Recovery and conflict prompts are local actionable decisions. No toast,
passive popup, or duplicate inline operation-error surface is permitted.

## Dependency restrictions

- Keep policy and persistence behavior in the Editor owner; filesystem and
  language-service adapters remain narrow injected services.
- Do not make Settings, Workspace, Shell, Git, or Notifications own Editor
  state. They consume values/events through typed contracts.
- Do not add a persistence facade or generic plugin/extension framework.
- Background autosave/formatting/recovery work must not block GPUI; use the
  existing background execution boundary.
- Every new dependency requires a documented single responsibility,
  license/security review, focused tests, and dependency-verifier approval.

## Implementation plan

1. Define and migrate the versioned session schema → verify one-leaf legacy
   migration, invalid-data recovery, idempotence, and no-data-loss tests.
2. Persist internal splits, selections, folds, scroll, and safe recovery
   state → verify restart restoration, file identity changes, conflict, and
   corrupted-session tests.
3. Implement opt-in autosave, format-on-save, whitespace, and final-newline
   policies → verify debounce, cancellation, ordering, atomic save, formatter
   failure, EOL/BOM, read-only, and notification tests.
4. Add P2 Vim/breadcrumb/inlay/presentation/accessibility polish → verify
   command/keymap coverage and normal, narrow, focused, empty, loading,
   error, long-file, split, overlay, and recovery visual states.
5. Remove obsolete adapters, settings placeholders, and no-op actions → verify
   source, dependency, settings-inventory, documentation, queue, and full
   repository gates.

## Acceptance criteria

- [ ] Editor session and split state restore through one versioned, tested
      Editor-owned persistence contract.
- [ ] Autosave and format-on-save are fully available but default to false;
      failures are recoverable and never overwrite conflicts silently.
- [ ] Trimming and final-newline policies preserve EOL/BOM/encoding and are
      opt-in.
- [ ] Every setting has owner, type, default, scope, validation, reset,
      migration, UI, and focused consumer-test evidence.
- [ ] P2 capabilities are either implemented with tests/visual evidence or
      explicitly deferred; no dead command or misleading setting remains.
- [ ] No remote/collaboration/extension path is implied by local contracts.
- [ ] Focused persistence, policy, command, notification, accessibility, and
      visual tests pass.
- [ ] Full repository, dependency, documentation, queue, and diff checks pass.

## Removal / exit condition

The task is complete only when the Editor rework has one owner and one
canonical contract, all planned P0/P0.5/P1 behavior is verified, the selected
P2 scope is either verified or explicitly deferred, old adapters and no-op
actions are removed, and the resulting UI has native visual evidence for all
required states.

## Notes and follow-ups

Remote language services, remote Editor buffers, collaboration, and extension
hosting require a later product decision and separate tasks. They must not be
smuggled into E6 as compatibility behavior.

## Implementation evidence (E6 core)

- Added a versioned Editor session contract with migration from the legacy
  path-only record, corruption-safe decode, split/selection/fold/scroll/search
  state, and unsaved-buffer recovery metadata. Workspace embeds the typed
  payload without taking ownership of Editor state.
- Added Editor-owned save policies for autosave, format-on-save, trailing
  whitespace, and final newline. Autosave and format-on-save are configurable
  and default to `false`; policy behavior is covered by focused tests.
- Extended canonical editor settings and the generated settings UI with
  validated font, display, diagnostics, semantic-token, Git-gutter, cursor,
  minimap, save-policy, and related fields. Defaults, migration, and consumer
  tests are present; unsupported external LSP process management remains
  explicitly deferred.
- Verification: 155 `labonair-editor` unit tests plus 10 LSP/runtime
  integration/fixture tests, 22 settings-UI tests, and 140 Workspace tests; formatting,
  check, Clippy,
  dependency, queue, documentation, and diff checks pass. The full workspace
  suite remains environment-blocked by the existing AI socket test and the
  settings watcher rename test; unrelated code was not weakened.
- Native visual acceptance remains under R07-001. This task therefore remains
  formally `Planned` even though the E6 implementation slice is present.

## Implementation evidence (settings consumers and save flow)

The editor surface now has a real cursor-blink task and applies the configured
font/display settings in the paint path. When `formatOnSave` is enabled, the
Editor requests formatting from the active local service before the atomic
write, applies returned edits transactionally, and saves the current text with
an explicit fallback notification if formatting fails. The default remains
disabled; no shell-level save policy or duplicate notification surface was
introduced. A pending unsaved session is surfaced as an editor-owned Restore /
Discard banner after the file load, with recovery content applied through the
normal transaction path.

The current bounded follow-up also includes the Editor-owned project-search
session contract and the Workspace filesystem adapter used by the existing
Cmd+F overlay. It adds no setting or persistence field: query scope is transient
overlay state, while query/result lifecycle and stale-result rejection remain
owned by Editor. The Open File workflow follows the same boundary: its fuzzy
ranking and stale-safe session live in Editor, while Workspace owns only the
background `fs_glob` adapter, root validation, and `Workspace::open_file`
composition route.

Current focused verification: 155 `labonair-editor` unit tests plus 10
LSP/runtime integration/fixture tests and 140 `labonair-workspace` tests pass.
This includes the Finder ranking, empty/no-match, stale/cancel, adapter,
root-safety, and command-ownership tests. Native visual acceptance remains
under R07-001, so this task remains formally `Planned`.

## Implementation evidence (breadcrumb/navigation slice, 2026-09-12)

- Added an Editor-owned, UI-free breadcrumb state helper that exposes only
  truthful path and current-symbol data for ready buffers, suppresses symbols
  while loading or in missing/error states, and middle-truncates long paths
  while preserving the full tooltip value. Binary and oversized buffers do
  not expose symbol context.
- Added the breadcrumb row inside EditorView above the existing buffer/split
  surface. Symbol navigation is a focusable UI-kit ListItem with hover,
  focus, and tooltip states and delegates to the existing goto_line path;
  no command, setting, dependency, or permanent shell zone was added.
- Focused verification: `cargo test -p labonair-editor --all-targets` passed
  with 161 unit tests, 3 language-runtime integration tests, and 7
  LSP-process integration tests; `cargo test -p labonair-workspace
  --all-targets` passed with 140 tests. `cargo fmt --all -- --check`,
  targeted Clippy with `-D warnings`, and `git diff --check` passed.
- This bounded slice does not include AI, debugger, collaboration,
  extensions, or other P2 systems. Native visual acceptance remains owned by
  R07-001; this task remains formally `Planned`.
