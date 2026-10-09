# Evidence Model

**Status:** Normative
**Version:** 1
**Related:** [`../visual-verification.md`](../visual-verification.md), [`visual-matrix.md`](visual-matrix.md), [`visual-evidence-schema.md`](visual-evidence-schema.md), [`../capabilities/schema.md`](../capabilities/schema.md), [`../evidence/scorecard.toml`](../evidence/scorecard.toml)

Evidence answers what a claim is based on. A status without an evidence
reference is not a completion claim.

## Evidence classes

| Class | Proves | Does not prove by itself |
|---|---|---|
| `contract` | A typed owner boundary exists and is covered. | That every consumer is integrated. |
| `unit` | Isolated domain behavior. | Native rendering or process/network behavior. |
| `integration` | Real owner/adapter/consumer wiring. | Visual consistency or supported-platform packaging. |
| `command` | A stable user action reaches its owner. | Correct visual availability in every context. |
| `surface` | A canonical entry point exists and is owned. | State completeness without state evidence. |
| `persistence` | Load/save/reload/migration behavior. | Secret safety unless explicitly tested. |
| `notification` | User-visible success/failure routing and deduplication. | Native placement and anchoring. |
| `visual` | A native binary renders the required state. | Backend correctness or architecture ownership. |
| `security` | Trust, grant, path, secret, or denial behavior. | General product completeness. |
| `performance` | A declared budget or baseline. | Functional correctness. |
| `regression` | A previous bug or migration edge stays covered. | New behavior not represented by the regression. |

Use `Verified`, `Partial`, `Pending`, or `N/A` for evidence state. Every
`Pending` item records the exact blocker and a next action. A successful Rust
build is evidence for its executed checks only; it does not close visual,
security, remote, or workflow evidence.

## Evidence record

The structured scorecard in [`../evidence/scorecard.toml`](../evidence/scorecard.toml)
is the first repository-wide evidence index. Each item contains an ID, area,
target, owner, status, exact source/artifact references, verification date,
limitation, and next action. The generated projection is
[`../generated/scorecard.md`](../generated/scorecard.md).

Visual evidence is indexed separately in
[`visual-evidence.toml`](visual-evidence.toml). It owns the per-surface state
labels and deterministic artifact paths; the scorecard links to that index
instead of duplicating a second visual matrix.
