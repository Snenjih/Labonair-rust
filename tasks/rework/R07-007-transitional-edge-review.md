# R07-007 — Enforce transitional-edge review and expiry semantics

## Status

`⏳ Planned`

## Owner

- Module: architecture and repository governance
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `docs/architecture/graph.toml` and
  `scripts/check_architecture.py`

## Dependencies

- `R07-006-documentation-knowledge-system` must be complete.
- No later boundary migration may bypass this policy source.

## Goal

Make every tolerated dependency edge auditable and time-bounded. Agents must
be able to find the removal task, the event that requires re-evaluation, the
last review date, and the deterministic expiry rule without relying on an
untracked discussion.

## Scope

- In scope: transitional-edge schema fields, task-path validation, review-date
  and interval checks, generated architecture output, and the remaining-edge
  audit.
- Out of scope: speculative extraction of a contract without a real consumer,
  product-surface changes, and dependency additions.

## Contracts, dependencies, and ownership

- Public contract: the `transitional_edge` record in
  `docs/architecture/graph.toml`.
- Allowed dependency changes: none; this task only governs existing edges.
- Registry contributions: none.
- UI-kit and product surfaces: none.
- Persistence and migration: none; no user data or serialization format is
  changed.
- Security and performance: no runtime behavior changes; the checker only
  protects architecture metadata.

## Contract

Each `transitional_edge` record must declare:

- `removal_condition` — the concrete condition under which the edge is removed;
- `removal_task` — the bounded active-queue task responsible for the review or
  removal decision;
- `review_trigger` — the product or architecture event that requires review;
- `last_reviewed` — the last evidence-backed review date;
- `review_interval_days` — the maximum interval before the architecture check
  fails closed.

The checker must reject an edge that is not a real Cargo dependency, points to
a missing task, lacks one of these fields, or has exceeded its review
interval. The generated architecture view must expose all fields.

## Acceptance criteria

- [ ] All transitional edges have task, trigger, date, interval, and removal
      condition metadata.
- [ ] `scripts/check_architecture.py` rejects missing tasks and expired review
      intervals.
- [ ] The generated architecture view exposes the review/removal metadata.
- [ ] `docs/audits/remaining-boundaries.md` agrees with the manifest.
- [ ] No product dependency edge is added and all repository gates pass.

## Evidence and handoff

Record the exact manifest diff, checker output, generated view fingerprint,
and any edge that is removed or deliberately retained. If a review finds a
real consumer for a narrower contract, create the next bounded owner task;
do not add a facade in this task.

## Required gates

```text
python3 scripts/verify.py --scope architecture
python3 scripts/verify.py --scope docs
python3 scripts/check_rework_queue.py
git diff --check
```
