# R02-002 — Shell composition and standalone workspaces

## Status

`🔄 In Progress`

## Owner

- Modules: `workspace` and application composition
- Canonical crates: `labonair-workspace` and `labonair-shell`
- Related contracts: [`../../docs/architecture.md`](../../docs/architecture.md),
  [`../../docs/workspace-model.md`](../../docs/workspace-model.md)

## Goal

Make the shell a predictable composition layer and make a workspace a useful
runtime context even when it has no project root. The same capability surfaces
must work for project workflows and one-off terminal, editor, SSH, and SFTP
actions without inventing a second application mode.

## Scope

- Inventory remaining shell/workspace feature behavior and broad injected
  dependencies before moving code.
- Keep permanent shell chrome limited to titlebar, workspace content, docks,
  statusbar, and modal/overlay hosts.
- Move one bounded feature-specific behavior slice out of shell/workspace into
  its owning capability and replace it with a typed contract or registration.
- Define and test empty-workspace, project-workspace, and standalone-tool
  transitions without special-case shell layouts.
- Reuse UI-kit controls and the existing overlay anchor contract.

## Dependencies

- `R02-001-global-menu-and-theme-entrypoints.md` — Done

## Acceptance criteria

- [ ] The selected shell/workspace slice has one documented owner and no
  duplicate shell implementation.
- [ ] Empty, project, and standalone workspace states have explicit typed
  state transitions and tests.
- [ ] Permanent chrome remains in its documented zones and uses existing
  shared components.
- [ ] No new broad backend or shell facade is introduced.
- [ ] Capability matrix, inventory, roadmap, and dependency allow-list agree
  with the moved boundary.
- [ ] Full workspace verification gates and a visual shell check pass.

## Removal condition

The task is complete only when the selected compatibility path is deleted, the
workspace no longer needs a feature-specific shell branch for the migrated
slice, and the empty/standalone workflow is covered by the same surface model
as project workspaces.

## Progress

The Hosts navigation callback has been replaced by the typed
`WorkspaceEvent::OpenHosts` contract and a shell subscription. The workspace
now owns the UI-free `WorkspaceIdentity` / `WorkspaceState` model, starts in a
standalone identity, exposes explicit project/standalone transitions, and
renders the identity on the empty surface. The native `Open Project…` action
now emits `WorkspaceEvent::OpenProject`; the composition root presents GPUI's
platform folder picker and applies the selected root through the workspace
boundary. Project settings follow that explicit identity instead of terminal
CWD changes. The stale native `Keyboard Shortcuts` entry was removed; its
legacy keymap action name remains only as a compatibility alias to the
canonical keymap JSON editor. Focused workspace tests, compile, and Clippy
checks pass.

Remaining work is the required manual visual check of the
empty/project/standalone shell states. `cargo run` reaches the native Rust
binary, but the process currently exposes no capturable layer-0 window. The
first generic-name screenshot was invalid because it captured the installed
legacy Tauri app; `scripts/screenshot.sh` now requires the Rust PID and fails
closed in that situation. This check cannot be inferred from startup or
compilation.
Session identity persistence and the final removal audit continue in
`R02-003`.
