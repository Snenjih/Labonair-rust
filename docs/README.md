# Labonair Documentation

This directory contains the current product and engineering contract for Labonair.

## Normative documents

- [`product.md`](product.md) — product identity, scope, and user-facing principles.
- [`architecture.md`](architecture.md) — runtime architecture, layers, and dependency direction.
- [`modules.md`](modules.md) — ownership and crate rules for feature modules.
- [`registries.md`](registries.md) — command, keymap, theme, panel, status item, and notification registries.
- [`design-system.md`](design-system.md) — visual language and reusable UI component rules.
- [`workspace-model.md`](workspace-model.md) — project and standalone workspaces.
- [`settings.md`](settings.md) — the boundary of the settings system.
- [`rework-roadmap.md`](rework-roadmap.md) — the current implementation sequence.

Normative documents are written in English because they are also engineering contracts. `ideas/` contains proposals only; an idea becomes binding only after it is incorporated here or accepted in an ADR.

## Supporting documents

- [`adr/`](adr/) records decisions and their consequences.
- [`reports/`](reports/) contains source comparisons and research.
- [`performance.md`](performance.md) and [`perf-baseline.md`](perf-baseline.md) contain measurements and performance notes.
- [`archive/`](archive/) contains superseded architecture and design documents. Archived documents are historical references, not instructions.
- [`audits/architecture-inventory.md`](audits/architecture-inventory.md) records the current migration baseline.

## Source of truth

When documents disagree, use this order:

1. the user's current product decision;
2. the normative documents listed above;
3. accepted ADRs;
4. implementation tasks;
5. reports and archived material.
