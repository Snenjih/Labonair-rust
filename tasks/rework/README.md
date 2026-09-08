# Active Rework Tasks

These tasks are derived from [`docs/rework-roadmap.md`](../../docs/rework-roadmap.md) and follow the ownership rules in [`docs/modules.md`](../../docs/modules.md). This directory is the active implementation queue; the older `tasks/archive/phase-*` directories are historical.

Each task must define its owning module, public contracts, allowed dependency changes, registry contributions, UI-kit usage, persistence/migration impact, tests, and full verification gates.

The active sequence is:

0. `R00-001-documentation-reset.md` — Done
1. `R01-001-backend-boundaries-and-contracts.md` — Done
2. `R01-002-ssh-sftp-capability-contracts.md` — Done
3. `R01-003-typed-transfer-registry.md` — Done
4. `R01-004-command-palette-keymap-registry.md` — Done
5. `R01-005-settings-ownership-and-reduction.md` — Done
6. `R01-006-background-capability-boundary.md` — Done
7. `R01-007-hosts-ui-capability-boundary.md` — Done
8. `R01-008-notification-surface-owner.md` — Done
9. `R01-009-inline-error-notification-adoption.md` — Done
10. `R02-001-global-menu-and-theme-entrypoints.md` — Done
11. `R02-002-shell-composition-and-standalone-workspaces.md` — Done
12. `R02-003-project-entry-and-workspace-transitions.md` — Done
13. `R03-001-command-palette-provider-registry.md` — Done
14. `R03-002-keymap-runtime-and-editor.md` — Done
15. `R04-001-static-theme-registries-and-preview.md` — Done
16. `R04-002-host-management-and-connection-pickers.md` — Done
17. `R05-001-settings-audit-and-value-normalization.md` — Done
18. `R06-001-backend-adapter-eradication.md` — Done
19. `R07-001-product-surface-acceptance.md` — In Progress
20. `R07-002-owner-registered-surface-contributions.md` — Done
21. `R07-003-settings-legacy-warning-disposition.md` — Done
22. `R07-004-explorer-host-contract.md` — Planned (structural migration landed early; visual evidence folds into R07-001)
23. `R07-005-background-presentation-boundary.md` — Planned (structural migration landed early; visual evidence folds into R07-001)
24. `R08-001-transfers-ui-workspace-decoupling.md` — Done
25. `R08-002-remove-no-op-command-affordances.md` — Done
26. `R08-003-panel-snippets-workspace-decoupling.md` — Done
27. `R08-004-popover-trigger-bounds-anchoring.md` — Done
28. `R08-005-theme-management-out-of-settings-ui.md` — Done
29. `R08-006-appcomposition-accessors.md` — Done

Phases 0–6 are complete. R02-001 is the first bounded Phase 2 shell task and
also connects the already-established theme registry to its intended palette
surface. R02-002, R02-003, R03-001, R03-002, R04-001, R04-002, R05-001, and
R06-001 are complete. R07-001 is now active as the product-surface acceptance
audit; it may only create bounded follow-up tasks for discrepancies and must
not silently expand the product scope. R07-002 is complete and records the
owner-registered dynamic palette action boundary that resolved one of those
findings.

R07-004 is the follow-up for boundary B01 in the remaining-boundary backlog.
Its structural migration (the `labonair-explorer-host` contract crate, the
removed `panel-explorer → workspace` edge, the Workspace adapter, and all
code/dependency/test gates) has landed early. It stays `Planned` rather than
`Done` only because its native Explorer visual-state evidence folds into
R07-001's still-open visual acceptance matrix.

R07-005 is the follow-up for boundary B02. Its structural migration (the
`labonair-background-host` contract crate, the removed `workspace → background`
edge, the `BackgroundHost` adapter, and all code/dependency/test gates) has
also landed early, for the same reason and with the same `Planned` status
pending the R07-001 visual matrix.

Only the earliest task whose dependencies are complete may be started.
