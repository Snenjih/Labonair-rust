# R07-001 — Verify the coherent product surface

## Status

`🔄 In Progress`

## Owner

- Module: application composition with every capability owner
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: application startup and registered surfaces

## Dependencies

- `R06-001-backend-adapter-eradication`
- all preceding active rework tasks

## Goal

Prove that the target architecture, documentation, dependency graph, and
user-facing product behavior agree. This is an acceptance gate, not a place
to introduce new features.

## Scope

- In scope: full capability matrix audit, repository layout, registry
  ownership, Settings boundary, notifications, command/keymap flows, hosts,
  themes, transfers, standalone/project workflows, and visual consistency.
- Out of scope: marketplace, remote theme downloads, extension hosting, and
  speculative product expansion.

## Contracts and ownership

- Normative sources: `docs/product.md`, `docs/architecture.md`,
  `docs/modules.md`, `docs/registries.md`, `docs/design-system.md`,
  `docs/repository-layout.md`, and `docs/feature-lifecycle.md`.
- Evidence sources: capability matrix, architecture inventory, active task
  queue, dependency verifier, focused tests, and visual captures.
- Working evidence matrix: [`../../docs/audits/product-surface-acceptance.md`](../../docs/audits/product-surface-acceptance.md)
- Each discrepancy becomes a bounded follow-up task; it is not silently
  accepted as a target statement.

## Dependencies

- Existing edges removed: any final transitional dependency or duplicate
  surface found by the audit.
- New edges: none.
- Dependency verifier change: none except removal of obsolete allowances.

## Persistence and migration

- Settings, hosts, themes, keymap, transfers, and session data must each have
  one documented owner and migration story.
- No audit step may delete user data; unresolved data migrations are blockers
  for the relevant capability, not reasons to hide the finding.

## User-visible behavior

- Verify local and remote standalone workflows, project workspace workflows,
  notification history, transfer statusbar access, and keyboard navigation.
- Passive messages appear only in the notification dropdown; actionable
  controls remain where the user must decide or correct input.
- No undocumented permanent chrome, toast system, duplicate settings surface,
  or feature-local styling variant remains.

## Implementation plan

1. Run source, metadata, documentation, and registry audits → record evidence
   for every capability and disposition.
2. Run full tests, dependency checks, and visual state matrix → compare against
   the design contract.
3. Resolve discrepancies or create explicitly ordered follow-up tasks → verify
   the final tree contains no unowned capability or stale authority.

## Acceptance criteria

- [x] Every capability has one owner, one canonical crate, one entry point,
      and one documented persistence/notification story in the capability
      matrix and registry contracts; remaining implementation gaps are marked
      explicitly as partial or pending.
- [x] No normative document presents an archived task or report as current
      authority; `docs/README.md` and `tasks/rework/README.md` define the
      current documentation and implementation sources.
- [x] Dependency graph, source layout, and inventory agree; the dependency
      verifier reports an acyclic graph with no untracked boundary violations.
- [x] All repository verification gates pass on the current tree.
- [ ] Normal, narrow, focused, empty, loading, error, long-list, and overlay
      states are visually checked for each permanent surface.
- [x] Any remaining product idea is explicitly keep, redesign, defer, or
      remove in the capability matrix and product dispositions; none remains
      an implicit obligation.

## Removal condition

This task is complete only when the audit produces no unowned capability,
contradictory normative rule, undocumented compatibility path, or unverified
permanent UI surface.

## Notes and follow-ups

This gate does not authorize adding extensions or remote downloads. Those need
a separate product decision after the core workflow is stable.

## Audit findings

- [x] The broad backend facade is absent from the workspace and the dependency
      graph; R06-001 records the removal evidence.
- [x] Current normative documents identify the product scope, capability
      owners, registry contracts, Settings boundary, and deferred ideas.
- [x] Product command behavior is owner-registered. Workspace, Terminal,
  Hosts-UI, Settings, Settings-UI, Keymap-UI, Updater-UI, and Command
  Palette now contribute executable handlers through the typed runtime
  registry. The shell command table retains only the native Fullscreen
  action and the debug-only component gallery, which are composition/native
  concerns rather than product capability behavior.
- [x] The active source audit found no passive toast renderer or duplicate
      operation-error surface. The Explorer clipboard strip and editor conflict
      banner are actionable decision surfaces, not passive error reporting;
      operational failures are published to the notification registry.
- [x] The AI error model no longer embeds a removed Settings → AI route or a
      chat-error-banner assumption; callers choose the notification or future
      actionable surface at the capability boundary.
- [x] Theme registry selections remain persistable values, but `appTheme` and
      `themeVariantOverrides` are no longer registered as editable Settings
      fields; theme management now has only the Themes palette surface.
- [x] Hosts management uses the canonical `OpenHosts` command identity;
      `connections::OpenHostSettings` remains readable only as a keymap
      compatibility alias and is not discoverable.
- [x] The Notifications dropdown now composes notification rows from the
      shared UI-kit `ListItem`; its expansion, read-state, scrolling, and
      action behavior remain owned by Notifications.
- [x] Dynamic palette submenu actions now cross the typed `PaletteAction`
      contract and are dispatched through
      `labonair-command-palette-runtime::PaletteActionHandlerRegistry`.
      Workspace, Hosts-UI, Themes/Settings-UI, Source Control, and Snippets
      own their action matching; `shell/src/actions.rs` only forwards opaque
      actions.
- [x] The final status-item source audit found only typed owner registrations
      composed by `shell/src/status_items.rs`; no feature-owned statusbar
      behavior remains in the shell.
- [x] Known legacy Settings input is classified once by the Settings owner;
      retained migration and capability-owned paths are quiet while unknown
      future paths remain non-fatal warnings. R07-003 records the disposition
      and focused regression evidence.
- [ ] A complete visual state matrix is still required for the permanent
      surfaces. The working matrix is recorded in
      [`../../docs/audits/product-surface-acceptance.md`](../../docs/audits/product-surface-acceptance.md);
      all applicable visual cells remain pending until native-bundle evidence
      is recorded.
- [ ] Native launch confirms the Rust window exists, but macOS currently denies
      Screen Recording to the capture runner. Capture the visual matrix with
      the native Rust bundle once Screen Recording access is available.
