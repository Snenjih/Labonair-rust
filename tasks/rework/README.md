# Active Rework Tasks

These tasks are derived from [`docs/rework-roadmap.md`](../../docs/rework-roadmap.md) and follow the ownership rules in [`docs/modules.md`](../../docs/modules.md). This directory is the active implementation queue; the older `tasks/phase-*` directories are historical.

Each task must define its owning module, public contracts, allowed dependency changes, registry contributions, UI-kit usage, persistence/migration impact, tests, and full verification gates.

The active sequence is:

1. `R01-002-ssh-sftp-capability-contracts.md` — Done
2. `R01-003-typed-transfer-registry.md` — Done
3. `R01-004-command-palette-keymap-registry.md` — Done
4. `R01-005-settings-ownership-and-reduction.md` — Done
5. `R01-006-background-capability-boundary.md` — Done
6. `R01-007-hosts-ui-capability-boundary.md` — Done
7. `R01-008-notification-surface-owner.md` — Done
8. `R01-009-inline-error-notification-adoption.md` — Done
9. `R02-001-global-menu-and-theme-entrypoints.md` — Done
10. `R02-002-shell-composition-and-standalone-workspaces.md` — In Progress
11. `R02-003-project-entry-and-workspace-transitions.md` — Planned

Phase 3 is complete. R02-001 is the first bounded Phase 2 shell task and also
connects the already-established theme registry to its intended palette
surface. R02-002 is the active task; R02-003 is queued for session identity
persistence and explicit project/standalone lifecycle completion. Later
command-palette/keymap migrations remain blocked on their own task contracts.

Only the earliest task whose dependencies are complete may be started.
