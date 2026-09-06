# R02-003 — Explicit project entry and workspace transitions

## Status

`⏳ Planned`

## Owner

- Module: Workspace and application composition
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::bootstrap`

## Goal

Make project entry, standalone tool entry, and returning to an empty workspace
explicit user flows over the same workspace surface. Project identity must be
chosen by a project-opening action and must never be inferred from a terminal
working directory.

## Scope

- Add the typed request/response boundary for opening a local project root.
- Connect the application or a dedicated project picker to
  `WorkspaceContext::set_project` and `set_standalone_context`.
- Carry project identity through the workspace/session contract where
  persistence is enabled; keep standalone sessions temporary by default.
- Ensure project-scoped settings follow the explicit workspace identity and do
  not silently change when a terminal changes directory.
- Exercise empty, active, project, and standalone transitions with focused
  contract tests and shell-level integration tests.

Out of scope: remote project discovery, project templates, a project
marketplace, or a second project-specific layout. Remote roots will use the
same contract after SSH/SFTP define their transport-owned identity.

## Contracts and ownership

- Public domain values: `labonair-workspace::context`
- Service/event boundary: typed workspace request events and immutable state
  snapshots; no shell callback fields
- Registry contributions: none unless more than one project provider exists
- UI surface: the existing workspace/overlay surface, with any picker owned by
  the project entry capability
- Shared UI-kit components: standard command, list, search, dialog, and empty
  state primitives only

## Dependencies

- Existing edges removed: cwd-based project inference once identity sync moves
  to the explicit transition
- New edges: only a narrow project-picker contract if a provider is required
- Dependency verifier change: update the allow-list and inventory together;
  never add a shell-to-feature implementation edge

## Persistence and migration

- Settings: project-layer loading is keyed by the explicit project root;
  preserve existing `.labonair/settings.json` data
- Storage: session identity migration is required if snapshots gain a project
  root; old snapshots remain valid as standalone snapshots
- Compatibility: any legacy cwd inference must have no remaining consumers
  before this task is marked complete

## User-visible behavior

- Canonical entry point: a command/picker action opens a project root; terminal,
  editor, SSH, and SFTP actions can still start standalone
- Notifications: invalid or inaccessible roots use the notification registry
  with actionable details; no duplicate inline error row or toast
- Inline errors/toasts: none for passive project-entry failures

## Implementation plan

1. Extend the workspace context contract and snapshot model → verify transition,
   identity, and backward-compatibility tests.
2. Add the explicit project-opening action and composition wiring → verify a
   project can be opened without changing the shell layout model.
3. Remove cwd inference and update settings/session consumers → verify the
   dependency inventory and project-scope tests.
4. Verify normal, empty, standalone, and project shell states visually.

## Acceptance criteria

- [ ] Project identity changes only through an explicit typed transition.
- [ ] Empty, active, project, and standalone states share one workspace model.
- [ ] Session and project settings behavior is explicit and backward compatible.
- [ ] No shell callback or feature-specific duplicate layout is introduced.
- [ ] Passive failures use notifications; no new toast or duplicate inline error.
- [ ] Focused tests and full repository gates pass.
- [ ] Required visual shell check passes.

## Removal condition

The task is complete only when project opening uses the typed workspace
contract, cwd changes no longer mutate project identity, and no legacy project
inference or compatibility path remains in active code.

## Notes and follow-ups

This task follows `R02-002`. It intentionally does not create a project
manager crate until a second project provider or independent project lifecycle
requires one.

