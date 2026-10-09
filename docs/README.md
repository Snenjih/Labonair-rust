# Labonair Documentation

This directory contains the current product and engineering contract for Labonair.

The documents describe two different kinds of truth: normative documents
define the target rules, while `audits/architecture-inventory.md` records the
unfinished current tree. A target statement must not be used as evidence that
the migration is complete.

## Normative documents

- [`product.md`](product.md) — product identity, scope, and user-facing principles.
- [`architecture.md`](architecture.md) — runtime architecture, layers, and dependency direction.
- [`architecture/layers.md`](architecture/layers.md) — normative layer and direction rules.
- [`architecture/composition-root.md`](architecture/composition-root.md) — composition-root responsibilities and prohibitions.
- [`architecture/runtime-lifecycle.md`](architecture/runtime-lifecycle.md) — construct-to-shutdown lifecycle and evidence vocabulary.
- [`architecture/dependency-rules.md`](architecture/dependency-rules.md) — explicit dependency policy and verification.
- [`architecture/graph.toml`](architecture/graph.toml) — machine-readable crate ownership and dependency source.
- [`capabilities.md`](capabilities.md) — authoritative capability ownership and current migration matrix.
- [`capabilities/schema.md`](capabilities/schema.md) — structured capability descriptor contract.
- [`capabilities/coverage.toml`](capabilities/coverage.toml) — coverage record for every capability-matrix row.
- [`modules.md`](modules.md) — ownership and crate rules for feature modules.
- [`registries.md`](registries.md) — command, keymap, theme, panel, status item, and notification registries.
- [`design-system.md`](design-system.md) — visual language and reusable UI component rules.
- [`repository-layout.md`](repository-layout.md) — canonical crate, source, and documentation placement.
- [`product/catalog-schema.md`](product/catalog-schema.md) — normative command, surface, and menu catalog rules.
- [`product/commands.toml`](product/commands.toml) — structured command ownership and evidence source.
- [`product/surfaces.toml`](product/surfaces.toml) — structured canonical surface source.
- [`product/menu.toml`](product/menu.toml) — structured global menu source.
- [`product/workflows.md`](product/workflows.md) — end-to-end workflow record
  contract.
- [`product/workflows.toml`](product/workflows.toml) — structured workflow
  source.
- [`automation/overview.md`](automation/overview.md) — MCP trust model and
  automation boundary.
- [`automation/grants.md`](automation/grants.md) — explicit session and host
  grant lifecycle.
- [`automation/limits.md`](automation/limits.md) — timeout, port, retention,
  and operation limits.
- [`automation/test-matrix.md`](automation/test-matrix.md) — automation
  positive/negative test decisions.
- [`automation/tools.toml`](automation/tools.toml) — structured MCP tool,
  grant, limit, and negative-test source.
- [`security/threat-model.md`](security/threat-model.md) — automation and
  remote threat model.
- [`security/trust-boundaries.md`](security/trust-boundaries.md) — MCP,
  composition, capability, and secret boundaries.
- [`security/secrets.md`](security/secrets.md) — secret storage, redaction, and
  fixture rules.
- [`security/remote-access.md`](security/remote-access.md) — SSH/SFTP/host
  safety contract.
- [`settings/README.md`](settings/README.md) — settings documentation index.
- [`settings/catalog-schema.md`](settings/catalog-schema.md) — machine-readable settings lifecycle schema.
- [`settings/catalog.toml`](settings/catalog.toml) — structured settings area and data-lifecycle source.
- [`settings/persistence.md`](settings/persistence.md) — settings/data
  lifecycle and ownership.
- [`performance/budgets.md`](performance/budgets.md) — target performance
  budgets and measurement state.
- [`performance/catalog-schema.md`](performance/catalog-schema.md) — structured
  performance budget and measurement contract.
- [`performance/budgets.toml`](performance/budgets.toml) — machine-readable
  performance budget source.
- [`release/README.md`](release/README.md) — release documentation index.
- [`release/platforms.md`](release/platforms.md) — supported and prepared
  platform policy.
- [`release/packaging.md`](release/packaging.md) — native artifact and signing
  flow.
- [`release/rollback.md`](release/rollback.md) — updater rollback and recovery.
- [`release/support-policy.md`](release/support-policy.md) — release support
  evidence and limitations.
- [`release/checklist-schema.md`](release/checklist-schema.md) — structured
  release-check contract.
- [`release/checklist.toml`](release/checklist.toml) — machine-readable
  package, signing, updater, rollback, and support source.
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
- [`testing/evidence-model.md`](testing/evidence-model.md) — evidence classes,
  status vocabulary, and scorecard contract.
- [`testing/visual-matrix.md`](testing/visual-matrix.md) — required visual
  states and native artifact rules.
- [`testing/visual-evidence-schema.md`](testing/visual-evidence-schema.md) —
  deterministic screenshot registry and per-surface evidence contract.
- [`testing/visual-evidence.toml`](testing/visual-evidence.toml) — structured
  per-surface state and native-capture source.
- [`evidence/scorecard.toml`](evidence/scorecard.toml) — structured product
  evidence source.
- [`audits/product-surface-acceptance.md`](audits/product-surface-acceptance.md)
  — R07 evidence matrix for ownership and required visual states.
- [`rework-roadmap.md`](rework-roadmap.md) — the current implementation sequence.

The executable task queue derived from the roadmap is
[`../tasks/rework/README.md`](../tasks/rework/README.md). The historical
`tasks/archive/` tree and original roadmap are retained for traceability only.

Normative documents are written in English because they are also engineering contracts. `ideas/` contains proposals only; an idea becomes binding only after it is incorporated here or accepted in an ADR. The document classes and maintenance rules are defined in [`documentation-governance.md`](documentation-governance.md).

## Supporting documents

- [`../docs-reform-plan.md`](../docs-reform-plan.md) — project goal for an
  agent-discoverable, generated, evidence-backed documentation and verification
  system.
- [`generated/architecture.md`](generated/architecture.md) — generated readable
  projection of the architecture manifest; do not edit directly.
- [`generated/capabilities.md`](generated/capabilities.md) — generated pilot
  capability index; do not edit directly.
- [`generated/capability-coverage.md`](generated/capability-coverage.md) —
  generated coverage for all capability rows; do not edit directly.
- [`generated/commands.md`](generated/commands.md), [`generated/surfaces.md`](generated/surfaces.md), and [`generated/menu.md`](generated/menu.md) — generated product catalogs; do not edit directly.
- [`generated/scorecard.md`](generated/scorecard.md) — generated evidence
  scorecard; do not edit directly.
- [`generated/workflows.md`](generated/workflows.md) — generated end-to-end
  workflow catalog; do not edit directly.
- [`generated/settings.md`](generated/settings.md) — generated Settings area
  and native field catalog; do not edit directly.
- [`generated/performance.md`](generated/performance.md) — generated
  performance budget and measurement view; do not edit directly.
- [`generated/release.md`](generated/release.md) — generated native release
  checklist; do not edit directly.
- [`generated/agent-index.md`](generated/agent-index.md) — generated request
  routing index; do not edit directly.
- [`generated/tools.md`](generated/tools.md) — generated MCP tool catalog; do
  not edit directly.
- [`generated/visual-evidence.md`](generated/visual-evidence.md) — generated
  per-surface screenshot index; do not edit directly.
- [`agents/README.md`](agents/README.md) — five-minute agent orientation,
  change routing, pre-change contract, and completion handoff.
- [`agents/change-matrix.md`](agents/change-matrix.md) — change type to
  canonical documents and verification gates.
- [`agents/finish-gates.md`](agents/finish-gates.md) — required evidence and
  validation by change scope.
- [`agents/routes-schema.md`](agents/routes-schema.md) — structured agent
  routing contract.
- [`agents/routes.toml`](agents/routes.toml) — request-to-document-and-gate
  route source.
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
