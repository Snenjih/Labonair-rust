# R02-002 — Shell composition and standalone workspaces

## Status

`✅ Done`

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

- [x] The selected shell/workspace slice has one documented owner and no
  duplicate shell implementation.
- [x] Empty, project, and standalone workspace states have explicit typed
  state transitions and tests.
- [x] Permanent chrome remains in its documented zones and uses existing
  shared components.
- [x] No new broad backend or shell facade is introduced.
- [x] Capability matrix, inventory, roadmap, and dependency allow-list agree
  with the moved boundary.
- [x] Full workspace verification gates and a visual shell check pass.

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

The correct Rust app bundle was launched through
`target/release/bundle/macos/Labonair.app` and captured with a PID-scoped
visual check. The valid screenshot shows the empty standalone workspace and
the expected shell zones. Project-state persistence and the explicit
transition action are implemented in the following R02-003 task; the helper
supports both debug and bundled Rust executables and rejects the installed
legacy Tauri app.

The shell's Explorer and Git root synchronization now uses the explicit
workspace project identity before consulting the active terminal cwd. This
means opening a project updates its file and Git surfaces, while standalone
terminals retain cwd-driven behavior. Regression tests cover both precedence
orders.

The native launch verification path is now PID- and executable-specific. The
optional release smoke test rejects a Rust process that exits during the
interval. The macOS test opens the absolute Rust bundle path through
LaunchServices and never resolves the shared `Labonair` display name, so the
legacy Tauri application cannot satisfy the check.

The launch boundary was narrowed further: `open -n -W` with the absolute Rust
bundle path starts the native app reliably, and the smoke test resolves and
checks only the resulting Rust executable PID. A current PID-scoped capture
confirms the standalone shell zones; project identity persistence and the
return transition are verified by R02-003.

The native Empty-state shortcut hint now matches the canonical keymap
registry: Commands is shown as `⌘P`, not the stale `⌘K`. The corrected bundle
was rebuilt and checked with a PID-scoped screenshot; project selection and
transition behavior are covered by R02-003.

Workspace root precedence is now owned by `labonair-workspace`: its public
`Workspace::filesystem_root` and `Workspace::git_root` contracts centralize
the project-versus-standalone fallback rules, while pure resolver tests live
beside `WorkspaceContext`. The shell now only consumes those contracts when
composing Explorer and Git surfaces; the duplicate shell resolver and tests
were removed.
R02-002 is complete. Session identity persistence and the explicit
return-to-standalone lifecycle continue in `R02-003`.
