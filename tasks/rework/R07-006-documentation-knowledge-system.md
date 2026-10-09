# R07-006 — Complete the agent-discoverable knowledge system

## Status

`⏳ Planned`

## Owner

- Module: repository documentation and application composition acceptance
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `scripts/verify.py` and `docs/README.md`

## Dependencies

- `R07-001-product-surface-acceptance` must be complete.
- `R07-004-explorer-host-contract` and `R07-005-background-presentation-boundary`
  must have their R07 visual evidence accepted.
- No R09 task may be used to bypass this acceptance dependency.

## Goal

Complete the remaining evidence and coverage work in
[`docs-reform-plan.md`](../../docs-reform-plan.md) so an agent can navigate
from a capability or workflow to its owner, contract, implementation,
evidence, and next safe task without relying on chat history.

## Scope

- In scope: full capability descriptor expansion from the explicit coverage
  records, native visual artifact indexing, supported-host security denial
  evidence, performance measurements, release/rollback handoffs, and
  deterministic semantic drift checks where a source-backed rule exists.
- Out of scope: product-scope expansion, new shell chrome, a second task
  queue, web/WASM reintroduction, or a replacement of the native architecture.

## Contracts and ownership

- Documentation authority: [`../../docs/README.md`](../../docs/README.md) and
  [`../../docs/documentation-governance.md`](../../docs/documentation-governance.md)
- Agent routes: [`../../docs/agents/routes.toml`](../../docs/agents/routes.toml)
- Capability source: [`../../docs/capabilities/coverage.toml`](../../docs/capabilities/coverage.toml)
- Evidence source: [`../../docs/evidence/scorecard.toml`](../../docs/evidence/scorecard.toml)
- Verification entry point: `python3 scripts/verify.py --scope all`

## Dependencies and migration

- Existing edges removed: none; this task must not add product dependency edges.
- New edges: none.
- Compatibility: no new compatibility path; every retained migration must
  keep its existing removal condition.

## User-visible and agent-visible behavior

- Every capability and workflow states its canonical entry point, current
  status, evidence state, and next action.
- Native visual claims reference the exact Rust bundle, PID/window, viewport,
  state label, and artifact path.
- Security, performance, release, and rollback claims identify the host,
  command, artifact, limitation, and follow-up without recording secrets.

## Implementation plan

1. Close R07 native visual acceptance → verify exact-bundle/PID matrix and
   `python3 scripts/verify.py --scope knowledge`.
2. Convert matrix-tracked capability rows in owner/task order → verify
   `python3 scripts/verify.py --scope capabilities` and generated freshness.
3. Execute supported-host security, performance, and release evidence → verify
   the relevant scopes and update `docs/evidence/scorecard.toml`.
4. Add only deterministic, source-backed semantic drift checks → verify
   `python3 scripts/verify.py --scope docs` and `--scope all`.

## Acceptance criteria

- [ ] R07 native visual matrix is complete or every non-applicable state has an
      explicit reason and owner-approved status.
- [ ] Every capability has a full descriptor or a current explicit coverage
      record with a bounded next task.
- [ ] Completed workflows link contract, integration, visual, security,
      performance, and not-applicable evidence as appropriate.
- [ ] Security denial, performance, packaging, and rollback evidence is
      attached where the supported environment permits execution.
- [ ] Generated views are fresh and all canonical verifier scopes pass.
- [ ] No secrets, credentials, or unrelated worktree changes are included.

## Evidence and handoff

Record exact commands, host/toolchain, commit, artifacts, unavailable gates,
and the next safe task in the handoff template. Keep the task Planned until
R07-001 is complete; it must never become a parallel active queue.
