# R02-003 — Explicit project entry and workspace transitions

## Status

`✅ Done`

## Owner

- Module: Workspace and application composition
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::bootstrap`

## Goal

Complete the project and standalone lifecycle over the shared workspace
surface. Project identity is already selected by the explicit `Open Project…`
flow from `R02-002`; this task makes that identity persistent and ensures
returning to standalone or empty state is explicit and lossless.

## Scope

- Carry project identity through the workspace/session contract where
  persistence is enabled; keep standalone sessions temporary by default.
- Add an explicit close-project/return-standalone action and composition
  wiring using the existing workspace transition contract.
- Ensure project-scoped settings follow the explicit workspace identity and do
  not silently change when a terminal changes directory.
- Exercise empty, active, project, and standalone transitions with focused
  contract tests and shell-level integration tests.

Out of scope: remote project discovery, project templates, a project
marketplace, or a second project-specific layout. Remote roots will use the
same contract after SSH/SFTP define their transport-owned identity.

## Contracts and ownership

- Public domain values: `labonair-workspace::context` and workspace session
  snapshots
- Service/event boundary: existing typed workspace request events and
  immutable state snapshots; no shell callback fields
- Registry contributions: none unless more than one project provider exists
- UI surface: the existing workspace/overlay surface, with any picker owned by
  the project entry capability
- Shared UI-kit components: standard command, list, search, dialog, and empty
  state primitives only

## Dependencies

- Existing edges removed in R02-002: cwd-based project inference for the active
  project settings layer
- New edges: only a narrow project-picker contract if a provider is required
- Dependency verifier change: update the allow-list and inventory together;
  never add a shell-to-feature implementation edge

## Persistence and migration

- Settings: project-layer loading is keyed by the explicit project root;
  preserve existing `.labonair/settings.json` data
- Storage: session identity migration is required if snapshots gain a project
  root; old snapshots remain valid as standalone snapshots
- Compatibility: any remaining legacy cwd inference must have no consumers
  before this task is marked complete

## User-visible behavior

- Canonical entry point: `Open Project…` opens a project root; terminal,
  editor, SSH, and SFTP actions can still start standalone
- Notifications: invalid or inaccessible roots use the notification registry
  with actionable details; no duplicate inline error row or toast
- Inline errors/toasts: none for passive project-entry failures

## Implementation plan

1. Extend the workspace context and snapshot model → verify identity,
   transition, and backward-compatibility tests.
2. Add explicit close-project/standalone behavior → verify a project can be
   left without changing the shell layout model.
3. Remove any remaining cwd inference and update settings/session consumers →
   verify the dependency inventory and project-scope tests.
4. Verify normal, empty, standalone, and project shell states visually.

## Acceptance criteria

- [x] Project identity changes only through the explicit typed
      `WorkspaceTransition` contract.
- [x] Empty, active, project, and standalone states share one workspace model.
- [x] Session and project settings behavior is explicit and backward compatible.
- [x] No shell callback or feature-specific duplicate layout is introduced.
- [x] Passive failures use notifications; no new toast or duplicate inline error.
- [x] Focused tests and full repository gates pass; the two OS-restricted tests
      pass in their isolated outside-sandbox runs.
- [x] Required visual shell check passes.

## Progress

The workspace session snapshot now carries the explicit
`WorkspaceIdentity`. Legacy snapshots without the field deserialize as
Standalone, while project snapshots round-trip their root. Restored project
identity is applied before tabs are recreated so project settings and surface
root observers receive the correct scope at startup. `Return to Standalone`
is registered as one typed workspace command and is available from the native
File menu and command palette; it changes identity only and preserves the
existing tabs, panes, and shell layout. Both picker selection and session
restore now use the same `WorkspaceTransition::OpenProject` path, while the
command uses `WorkspaceTransition::ReturnToStandalone`; direct identity
setters are no longer exposed. The pure `WorkspaceContext` owns the identity
mutation and reports no-op transitions; the GPUI `Workspace` only applies the
corresponding settings and invalidation side effects. Focused workspace,
command-registry, and shell tests pass, including a shell-registry assertion
that both lifecycle commands use their canonical typed action names and
execution registrations.

Project-settings synchronization was removed from `Workspace::render`. The
settings layer now loads only after an explicit `WorkspaceTransition` or an
explicit project-settings refresh, keeping rendering free of stateful I/O;
whitelist rejection notifications are deduplicated at that transition
boundary. The UI-free `WorkspaceContext` performs the actual identity mutation
and returns whether a transition changed state; GPUI `Workspace` only applies
the resulting settings and invalidation side effects.

The current release bundle (build 265) passes the exact-path five-second
native launch smoke test and was visually checked by the user as the native
Rust application. The helper rejects any supplied PID whose executable is not
the exact native Rust binary, and reports Screen Recording failures
separately. No legacy Tauri process was used as evidence.

## Removal condition

The task is complete only when project opening uses the typed workspace
contract, cwd changes no longer mutate project identity, and no legacy project
inference or compatibility path remains in active code.

## Notes and follow-ups

This task follows `R02-002`. It intentionally does not create a project
manager crate until a second project provider or independent project lifecycle
requires one.
