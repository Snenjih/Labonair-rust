# R09-002 — Editor E1: file state, encoding, and conflict safety

## Status

⏳ Planned

## Owner

- Module: editor
- Capability-matrix row: [Editor](../../docs/capabilities.md)
- Composition entry point: labonair-shell injects the filesystem adapter into the Workspace Editor host

## Dependencies

- R09-001-editor-e0-text-model-contract
- R07-001-product-surface-acceptance must be complete.
- labonair-filesystem remains the local storage and watcher owner.

## Goal

Give every open document an explicit, testable file lifecycle that preserves
line endings and BOM information, handles read-only files safely, and never
silently overwrites an external change.

## Scope

- In scope: Editor-owned FileIdentity and FileState, EOL and encoding
  metadata, BOM preservation, file capabilities, mtime/content identity,
  reload/save decisions, external-change detection, conflict transitions,
  read-only behavior, and the typed adapter used by Workspace.
- Out of scope: remote editing, LSP, Git review, search, and visual split
  layout. Remote editing will use a later Editor/remote adapter contract.

## Contracts and ownership

- Public domain values: FileIdentity, FileState, FileCapabilities, LineEnding,
  Encoding, Bom, SaveIntent, ReloadDecision, and immutable FileSnapshot in
  labonair-editor.
- Service traits or typed events: EditorFileService for load, atomic save,
  stat, and watch translation; FileLifecycleEvent for DirtyChanged,
  ReloadNeeded, ConflictDetected, Saved, Reloaded, and ReadOnlyChanged.
  labonair-filesystem implements storage; Editor owns lifecycle decisions.
- Registry contributions: none. File operations use Editor-owned commands and
  the existing command/keymap registry.
- UI surface: the existing Editor tab/standalone Editor. The conflict banner
  remains an actionable decision surface; Workspace only adapts placement.
- Shared UI-kit components: UI-kit banner/dialog, buttons, badges, and
  disclosure/detail controls where available. Do not add a local notification
  or toast component.

## Commands and keymap

Editor-owned stable actions are Save, Save As, Reload File, Revert Buffer,
Keep Buffer, Reload From Disk, and Resolve File Conflict. Existing Save and
Reload identities are reused where available; new actions receive stable
Editor descriptors, Editor context, and owner-provided default bindings.
Conflict choices are explicit commands or typed dialog actions, never hidden
automatic resolution.

## Settings

This task adds no settings. Existing editor settings remain unchanged. The
future autosave and format-on-save values are intentionally deferred to E6,
where each key will receive a typed field, default, scope, validation, reset
behavior, migration disposition, and focused consumer test before code use.

## Persistence and migration

- Storage: labonair-filesystem retains atomic write behavior; Editor persists
  FileIdentity, selected encoding, EOL, BOM, and the last accepted disk
  identity through its versioned session snapshot.
- Migration: version 1 path-only Editor snapshots load as UTF-8, no BOM, and
  detected native line endings. This is a compatibility default, not a claim
  that older files contained those metadata values.
- Compatibility: keep legacy mtime-only fields as read-only migration input
  until versioned file identity and conflict tests pass. Remove the old
  boolean-only external-change path when no consumer remains.
- Secrets: never serialize file contents, credentials, or remote auth data
  into diagnostics or notifications.

## Notifications

Publish save, reload, permission, encoding, and conflict-operation failures to
labonair-notifications-core with source, details, and deduplication metadata.
The conflict decision UI may remain local because the user must choose an
action. No passive toast or duplicate feature-local error banner is allowed.

## Dependency restrictions

- labonair-editor may depend on a narrow filesystem contract only if the
  adapter boundary requires it; the core lifecycle must remain testable
  without filesystem I/O.
- labonair-filesystem must not depend on Workspace or Editor UI.
- Workspace may construct the adapter but must not inspect or mutate private
  FileState fields.
- Do not add remote transport edges or a backend facade in this local task.

## Implementation plan

1. Define FileState transitions and metadata normalization → verify a
   transition table test for New, Clean, Dirty, ReloadNeeded, Conflicted,
   ReadOnly, Missing, Binary, and TooLarge.
2. Implement load/save/reload through EditorFileService with EOL/BOM/encoding
   preservation and atomic writes → verify round trips for LF, CRLF, CR, UTF-8
   with and without BOM, invalid input, binary, size limits, and permissions.
3. Replace mtime-only Workspace logic with typed lifecycle events and explicit
   conflict decisions → verify clean auto-reload, dirty conflict protection,
   read-only rejection, and notification deduplication.
4. Migrate session data and remove the legacy lifecycle shim → verify
   idempotent migration and source/dependency audits.

## Acceptance criteria

- [ ] FileState transitions are explicit and impossible transitions return
      descriptive errors.
- [ ] EOL, BOM, and supported encoding metadata survive load/save/reload.
- [ ] Read-only, missing, binary, and too-large files are represented without
      corrupting or replacing user data.
- [ ] Dirty buffers are never silently overwritten after an external change.
- [ ] Workspace consumes typed lifecycle events and owns no FileState logic.
- [ ] Failures use the notification registry; the conflict decision remains
      the only local actionable surface.
- [ ] Focused lifecycle, filesystem-adapter, migration, and Workspace tests pass.
- [ ] All repository verification gates, including dependency, documentation,
      queue, and diff checks, pass.

## Removal / exit condition

The task is complete when the mtime-only lifecycle and ad-hoc reload flags are
gone, the typed FileState contract is the sole source of truth, all local file
formats in scope have regression coverage, and no user data can be lost by
automatic conflict handling.

## Notes and follow-ups

E2 builds multi-selection editing on the stable transaction and FileState
contracts. E6 may extend the session schema for unsaved buffers, but must
preserve this file metadata and migration boundary.

## Implementation evidence (pre-queue adapter slice)

The E1 core and its Workspace adapter are implemented in the shared worktree,
but this task remains `Planned` because `R07-001-product-surface-acceptance`
is still the active queue task. The adapter now uses only the typed lifecycle
load, stat, save, `FileState`, `FileLifecycleEvent`, `ReloadDecision`, and
`SaveIntent` APIs.

- `Workspace::open_path` maps text, read-only text, binary, oversized,
  unsupported-encoding, invalid-UTF-8, missing, and I/O failures to the
  editor-owned document boundary. Unsupported content never becomes a
  synthetic editable comment in the buffer.
- Activation checks use `EditorFileStat` and `Document::observe_file`; clean
  changes reload automatically, while dirty changes enter the actionable
  conflict state. Save performs a typed stat preflight and the filesystem
  adapter repeats the identity check immediately before the atomic rename.
- The existing UI-kit banner/button surface renders read-only, missing,
  binary, oversized, loading, error, and conflict states. Failures continue to
  the notification center; no toast or shell zone was added.
- Focused Workspace coverage asserts that binary, oversized, and missing
  inputs have empty buffers rather than placeholder text, while read-only
  text preserves its content and rejects editing/saving. The editor lifecycle
  suite also covers terminal error states with write permissions.

Verification for this adapter slice:

- `cargo fmt --all -- --check`
- `cargo test -p labonair-editor` (84 tests)
- `cargo test -p labonair-workspace --lib views::editor::tests` (10 tests)
- `cargo check -p labonair-workspace --all-targets`
- `git diff --check`

The remaining E1 gate is the repository-wide verification and the final
R07-001 native visual acceptance evidence after that queue dependency is
closed.
