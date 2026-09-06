# R07-001 — Verify the coherent product surface

## Status

`⏳ Planned`

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

- [ ] Every capability has one owner, one canonical crate, one entry point,
      and one documented persistence/notification story.
- [ ] No stale document presents an archived task or report as current
      authority.
- [ ] Dependency graph, source layout, and inventory agree.
- [ ] All repository verification gates pass.
- [ ] Normal, narrow, focused, empty, loading, error, long-list, and overlay
      states are visually checked for each permanent surface.
- [ ] Any remaining product idea is explicitly keep, redesign, defer, or
      remove; none remains an implicit obligation.

## Removal condition

This task is complete only when the audit produces no unowned capability,
contradictory normative rule, undocumented compatibility path, or unverified
permanent UI surface.

## Notes and follow-ups

This gate does not authorize adding extensions or remote downloads. Those need
a separate product decision after the core workflow is stable.
