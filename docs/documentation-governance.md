# Documentation Governance

**Status:** Normative
**Version:** 2

This document defines where project knowledge belongs and how documentation
changes are kept consistent. It complements the product, architecture, and
feature-lifecycle contracts; it does not replace them.

## Document classes

| Location | Class | Authority | Purpose |
|---|---|---|---|
| `AGENTS.md` | engineering instructions | binding | Repository-wide implementation constraints and verification gates. |
| `docs/*.md` | normative contract | binding | Current product, architecture, module, registry, design, settings, workspace, and lifecycle rules. |
| `docs/architecture/*.md` | decomposed normative contract | binding | Layer, composition, lifecycle, and dependency rules used by agents and reviewers. |
| `docs/architecture/graph.toml` | machine-readable policy source | binding | Exact crate ownership, layer, and internal dependency policy checked against Cargo metadata. |
| `docs/generated/` | generated projection | derived | Human-readable views generated from canonical policy sources; never edit directly. Every view carries a deterministic source fingerprint. |
| `docs/agents/` | agent operating guide | binding workflow | Orientation, structured request routing, change routing, task templates, handoffs, anti-patterns, and finish gates. |
| `docs/product/` | product catalog source | binding | Structured command, surface, and menu ownership/evidence records. |
| `docs/testing/` | testing/evidence contract | binding | Evidence classes, visual matrix, and verification interpretation. |
| `docs/testing/visual-evidence.toml` | structured visual evidence source | binding data | Per-surface state status, deterministic artifact naming, and native capture records. |
| `docs/evidence/` | structured evidence source | binding data | Scorecard records with status, references, limitations, and next actions. |
| `docs/reform/` | reform traceability source | binding data | Definition-of-done records with status, source evidence, limitations, and bounded next tasks. |
| `docs/automation/` | automation contract | binding | MCP tools, grants, limits, and positive/negative test decisions. |
| `docs/security/` | security contract | binding | Threat model, trust boundaries, secret handling, and remote-access rules. |
| `docs/settings/` | settings lifecycle contract | binding | Persistence, migration, value ownership, and the machine-readable area catalog. |
| `docs/performance/` | performance contract | binding | Budgets, measurement interpretation, and the machine-readable budget catalog. |
| `docs/release/` | release operations contract | binding | Platform, packaging, rollback, support policy, and the machine-readable release checklist. |
| `docs/adr/` | accepted decision | binding for its decision | Records a deliberate architectural decision, its rationale, and consequences. |
| `docs/audits/` | current-state evidence | descriptive | Records what the source tree currently proves, including partial migrations and blockers. |
| `docs/reports/` | research and comparison | advisory | Preserves investigations and source comparisons; it cannot change the target. |
| `zed-refrence/zed/` | optional frozen external reference checkout | non-authoritative | Zed comparison links may resolve here when the local gitlink is populated; CI is allowed to omit this read-only reference tree. |
| `tasks/rework/` | active implementation queue | execution authority | Contains the ordered, bounded work needed to change the current tree. |
| `tasks/archive/` | historical tasks | non-authoritative | Preserves the former plan and completed historical work for traceability only. |
| `ideas/` | proposal | non-authoritative | Explores possible future directions before a product or architecture decision exists. |
| `memory/` | continuity and debugging notes | operational | Records session handoff, non-obvious fixes, and lessons that prevent repeated mistakes. |

The current product decision wins when documents disagree. Otherwise use this
order: normative contracts, accepted ADRs, active tasks, audits, reports, and
archived material. An audit may describe a violation of a normative contract;
it must never silently redefine the contract.

## Canonical ownership

Every subject has one canonical document. The index in `docs/README.md` is the
map of those authorities:

- product identity and scope → `docs/product.md`;
- runtime layers and crate boundaries → `docs/architecture.md`;
- machine-readable ownership and dependency policy → `docs/architecture/graph.toml`;
- decomposed layer, composition, lifecycle, and dependency rules →
  `docs/architecture/`;
- capability ownership and dispositions → `docs/capabilities.md`;
- module and crate rules → `docs/modules.md`;
- registry contracts → `docs/registries.md`;
- visual language → `docs/design-system.md`;
- repository placement → `docs/repository-layout.md`;
- feature change workflow → `docs/feature-lifecycle.md`;
- settings values and navigation → `docs/settings.md` and
  `docs/settings-guidelines.md`;
- workspace identity and workflows → `docs/workspace-model.md`;
- ordered implementation work → `docs/rework-roadmap.md` and
  `tasks/rework/`.
- agent onboarding and change routing → `docs/agents/README.md`;
- architecture projection → `docs/generated/architecture.md` (derived only).
- command/surface/menu ownership → `docs/product/`;
- evidence and visual interpretation → `docs/testing/` and `docs/evidence/`;
- native visual preparation and capture procedure →
  `docs/testing/native-visual-capture-runbook.md`;
- reform definition-of-done traceability → `docs/reform/requirements.toml` and
  `docs/generated/reform-coverage.md`;
- deterministic screenshot indexes → `docs/testing/visual-evidence.toml` and `docs/generated/visual-evidence.md`;
- automation tools and grants → `docs/automation/`;
- security and remote access → `docs/security/`;
- settings data lifecycle → `docs/settings/`;
- performance budgets → `docs/performance/`;
- release operations → `docs/release/`.

Do not create a second document that independently defines the same rule. Add
detail to the canonical document or link to it from a supporting audit or
report.

## Change protocol

Before changing code or product behavior:

1. Identify the owning capability and canonical crate.
2. Locate the canonical document for the affected rule.
3. Update the capability matrix, architecture inventory, roadmap, or active
   task when ownership, dependencies, persistence, or user entry points change.
4. Define a typed contract before moving adapters or consumers across a
   module boundary.
5. Record a compatibility path with an explicit removal condition, or remove
   it in the same change.
6. Run the repository and dependency checks required by `AGENTS.md`. For
   documentation or repository-control changes, also run
   `python3 scripts/check_documentation.py`; it validates normative metadata,
   current local Markdown links, and stale predecessor markers in automation
   configuration.

Architecture source or Cargo dependency changes additionally require
`python3 scripts/verify.py --scope architecture`; regenerate derived views
with their generator and include the resulting diff in the review.

For a new feature, the active task must state its user workflow, entry point,
owner, contracts, registry contributions, settings, persistence, notification
behavior, UI-kit components, migration impact, and acceptance evidence. A
feature without a current workflow is recorded as `defer` or `remove`; it does
not receive a new permanent surface merely because the predecessor had one.

## Audits and evidence

Audits must distinguish these states explicitly:

- `Verified` — current source or runtime evidence proves the claim;
- `Partial` — the owner exists but a documented migration edge remains;
- `Pending` — required evidence is missing;
- `N/A` — the state does not apply, with a reason.

Compilation and tests prove behavior covered by those checks. They do not prove
visual consistency, correct anchoring, or an untested ownership claim. Visual
claims require native-bundle inspection under `docs/visual-verification.md`.
When evidence is blocked, record the exact blocker and keep the affected claim
pending.

## Archiving and deletion

Move a document to an archive when it is superseded, not merely old. The
archive index must explain that it is historical and link to the current
authority. Historical files may retain old terminology and paths, but links
from current normative documents must never point to them as instructions.

Delete documentation only when it is duplicate, contains sensitive data, or
has no traceability value. Prefer a short redirect or an archive note when a
reader could reasonably need the historical context.

## Language and style

Normative documentation, code, comments, commits, and task files are written
in English. User-facing discussion may use German. Keep documents concise,
use stable terminology from the architecture contract, and prefer tables for
ownership maps and explicit decisions. Every normative document starts with a
status and version so obsolete authority can be recognized immediately.
