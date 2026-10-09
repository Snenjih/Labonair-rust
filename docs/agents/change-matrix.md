# Agent Change Matrix

**Status:** Agent workflow guide
**Owner:** Repository maintainers

Use this matrix before editing. The listed documents are the minimum reading
set; add the owner-specific contract and task files discovered there.

| Change type | Read first | Update if affected | Required gates |
|---|---|---|---|
| New capability | `capabilities.md`, `modules.md`, `feature-lifecycle.md` | capability record, roadmap/task, architecture, registries, product workflow | docs, queue, dependency, focused tests, workspace gates |
| Boundary migration | `architecture.md`, `modules.md`, `remaining-boundaries.md` | ADR/audit, task, dependency inventory, capability row | dependency, focused contract tests, full Rust gates |
| Crate/dependency | `repository-layout.md`, `architecture.md` | graph/allow-list, reason, transition/removal task | dependency graph, docs, full Rust gates |
| Command | `registries.md`, owner capability, command contract | owner descriptor, command/surface catalog, keymap | command uniqueness/reachability, tests, docs |
| Menu/palette/surface | `product.md`, `registries.md`, `design-system.md` | surface/menu catalog, owner contribution, visual matrix | surface checks, native visual evidence, docs |
| Panel/dock/status item | `registries.md`, `workspace-model.md` | owner registration, surface record, evidence | registry, layout, visual states, docs |
| Settings field | `settings.md`, `settings-guidelines.md`, `settings-inventory.md`, `settings/catalog.toml` | owner, scope, default, persistence, migration, consumer | settings scope, malformed input, notifications, docs |
| Persistence/schema | owner capability, `settings.md`, data lifecycle | schema/migration record, recovery behavior | migration fixtures, load/save/reload, security if sensitive |
| SSH/SFTP/remote | SSH/SFTP contracts, automation/security docs | adapter boundary, threat model, negative tests | dependency, denial/error/cancellation, secret checks |
| MCP/AI/automation | automation and security docs | tool catalog, grant/limit record, workflow evidence | positive and negative tool tests, docs, security |
| Visual/layout | `design-system.md`, `visual-verification.md`, `testing/visual-evidence.toml` | surface state matrix, generated screenshot index, task evidence | visual, exact native binary, normal/narrow/focused/etc. |
| Performance | `performance.md`, `performance/budgets.toml`, baseline records | budget, baseline, method, regression note | performance/evidence scopes, repeatable measurement, artifact, docs |
| Release/packaging | release docs, `release/checklist.toml`, and CI workflow | platform/support/signing/recovery records | release scope, package, smoke, artifact, rollback checks |
| Documentation only | `documentation-governance.md`, `docs/README.md` | canonical document and index | docs/agents/knowledge scopes, link check, diff check |
| Agent instructions | `AGENTS.md`, this directory, `agents/routes.toml` | one canonical guide or adapter | agents/docs/knowledge scopes, link check, queue check if tasks change |

## Required change contract

For non-trivial work, record:

```text
Request type:
Owner module:
Canonical capability crate:
Canonical user entry point:
Public typed contract:
State owner:
Persistence owner:
Commands and keymap entries:
Events and registry contributions:
Settings:
Notifications:
UI-kit components:
Allowed dependency changes:
Tests:
Visual evidence:
Security impact:
Performance impact:
Removal condition for compatibility code:
```

## Required evidence decision

For each evidence class, write one of:

- a concrete artifact or test name;
- `N/A` with a reason;
- `Pending` with an owner and next task.

Never leave evidence as an unqualified “tested” or “works”.
