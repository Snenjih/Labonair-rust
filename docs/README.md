# Labonair Documentation

This directory contains the current product and engineering contract for Labonair.

The documents describe two different kinds of truth: normative documents
define the target rules, while `audits/architecture-inventory.md` records the
unfinished current tree. A target statement must not be used as evidence that
the migration is complete.

## Normative documents

- [`product.md`](product.md) — product identity, scope, and user-facing principles.
- [`architecture.md`](architecture.md) — runtime architecture, layers, and dependency direction.
- [`capabilities.md`](capabilities.md) — authoritative capability ownership and current migration matrix.
- [`modules.md`](modules.md) — ownership and crate rules for feature modules.
- [`registries.md`](registries.md) — command, keymap, theme, panel, status item, and notification registries.
- [`design-system.md`](design-system.md) — visual language and reusable UI component rules.
- [`repository-layout.md`](repository-layout.md) — canonical crate, source, and documentation placement.
- [`feature-lifecycle.md`](feature-lifecycle.md) — required boundary-first workflow for feature changes.
- [`documentation-governance.md`](documentation-governance.md) — document ownership, authority, archive, and change rules.
- [`workspace-model.md`](workspace-model.md) — project and standalone workspaces.
- [`settings.md`](settings.md) — the boundary of the settings system.
- [`settings-guidelines.md`](settings-guidelines.md) — normative Settings UI
  navigation and field rules.
- [`settings-inventory.md`](settings-inventory.md) — normative R05-001
  field-to-consumer, scope, and keep/move/remove/review decisions.
- [`visual-verification.md`](visual-verification.md) — exact native-bundle and
  PID-scoped rules for rendered UI checks.
- [`audits/product-surface-acceptance.md`](audits/product-surface-acceptance.md)
  — R07 evidence matrix for ownership and required visual states.
- [`rework-roadmap.md`](rework-roadmap.md) — the current implementation sequence.

The executable task queue derived from the roadmap is
[`../tasks/rework/README.md`](../tasks/rework/README.md). The historical
`tasks/archive/` tree and original roadmap are retained for traceability only.

Normative documents are written in English because they are also engineering contracts. `ideas/` contains proposals only; an idea becomes binding only after it is incorporated here or accepted in an ADR. The document classes and maintenance rules are defined in [`documentation-governance.md`](documentation-governance.md).

## Supporting documents

- [`adr/`](adr/) records accepted decisions and their consequences. Superseded
  ADRs are kept separately in [`archive/adr/`](archive/adr/).
- [`reports/`](reports/) contains source comparisons and research.
- [`performance.md`](performance.md) and [`perf-baseline.md`](perf-baseline.md) contain measurements and performance notes.
- [`archive/`](archive/) contains superseded architecture and design documents. Archived documents are historical references, not instructions.
- [`audits/architecture-inventory.md`](audits/architecture-inventory.md) records the current migration baseline.
- [`audits/remaining-boundaries.md`](audits/remaining-boundaries.md) orders the
  explicitly tracked follow-up boundary migrations and retained composition
  edges.
- [`audits/backend-facade-inventory.md`](audits/backend-facade-inventory.md)
  records the R06 symbol-level backend export and consumer map.

## Source of truth

When documents disagree, use this order:

1. the user's current product decision;
2. the normative documents listed above;
3. accepted ADRs;
4. implementation tasks;
5. reports and archived material.

## Required change record

Before implementing a new capability or migrating an existing one, identify
its owning module and canonical capability crate, its user entry point, public
typed contract, state and persistence owner, commands and keymap entries,
notifications, registry contributions, and UI-kit components. Update the
capability matrix and roadmap/task record before or with the implementation.

During migration, keep a single active owner, move contracts before adapters
and consumers, and give every temporary compatibility path a removal
condition. The final change must remove obsolete registrations, settings,
events, and dependencies rather than preserving them as parallel paths.
