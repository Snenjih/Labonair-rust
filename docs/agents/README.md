# Labonair Agent Guide

**Status:** Agent workflow guide
**Owner:** Repository maintainers
**Authority:** [`../README.md`](../README.md) and the normative documents it indexes

This is the shortest safe reading path for an AI agent or contributor. It
routes work to the existing owner, contract, task, and verification systems;
it does not create a second authority.

## Five-minute orientation

Read in this order:

1. [`../../AGENTS.md`](../../AGENTS.md) for hard repository rules.
2. [`../README.md`](../README.md) for authority and document classes.
3. [`../product.md`](../product.md) for product identity and non-goals.
4. [`../capabilities.md`](../capabilities.md) for the capability owner.
5. [`../architecture.md`](../architecture.md) for boundaries and dependency direction.
6. [`../../tasks/rework/README.md`](../../tasks/rework/README.md) for the only active implementation queue.
7. [`change-matrix.md`](change-matrix.md) for the request-specific reading and gate list.

The machine-readable route source is [`routes.toml`](routes.toml); its
generated projection is [`../generated/agent-index.md`](../generated/agent-index.md).
Use it when routing a request programmatically, but keep `AGENTS.md` and the
normative documents as the authority.

Do not read every document by default. After the request is classified, follow
only the relevant rows in the change matrix and the canonical documents they
name.

## Authority order

When sources disagree, use:

1. the current user product decision;
2. normative documents in `docs/`;
3. accepted ADRs;
4. active implementation tasks;
5. current-state audits;
6. reports, ideas, and archived material.

A report can explain why a change is useful, but it cannot override a
normative contract. An audit can prove that the current tree violates a
target rule, but it cannot silently redefine that rule.

## Request routing

Classify the request before searching implementation code:

| Request | First owner lookup | Required workflow |
|---|---|---|
| New capability or migration | `capabilities.md` and `modules.md` | Boundary-first contract and active task |
| UI or layout | capability owner and `design-system.md` | Surface evidence and native visual check |
| Menu, palette, keymap, panel, dock, status item | `registries.md` and the owner | Command/surface ownership check |
| Settings or persistence | `settings.md`, `settings-inventory.md` | Value owner, scope, migration, consumer audit |
| SSH, SFTP, remote, transfer | capability contracts and security docs | Typed adapter and negative security tests |
| AI, MCP, automation | automation and security docs | Grant, limit, side-effect, and denial checks |
| Dependency or crate placement | `architecture.md`, `modules.md` | Dependency verifier and ADR/task if needed |
| Documentation or agent workflow | `documentation-governance.md` and this guide | Canonical index, link, freshness, and metadata checks |
| Bugfix | capability owner and existing regression tests | Preserve owner; add evidence for the failure |

## Before editing

Copy the fields from [`task-template.md`](task-template.md) into the task,
issue, or working note before making a non-trivial change. At minimum identify
the owner, canonical crate, entry point, public contract, state/persistence
owner, commands, settings, notifications, tests, and required evidence.

If the owner or contract is unknown, stop and resolve that boundary first. Do
not put the behavior in `labonair-shell`, `labonair`, or `workspace` merely
because the caller is convenient.

## Implementation order

Use this order for feature and boundary work:

```text
contract → owner → adapter → consumer → command/surface → evidence → docs
```

Keep feature behavior in its owner. Use the smallest typed trait, event, or
registry that matches the number of providers and consumers. A registry is not
the default abstraction for one caller.

## Queue rule

The only active architecture queue is `tasks/rework/`. Work only on its
earliest incomplete task. Later tasks are planning records, not permission to
skip a dependency. Historical tasks under `tasks/archive/` are not alternate
instructions.

## Completion

Use [`finish-gates.md`](finish-gates.md) to determine the required checks. A
successful compile is not proof of a complete user workflow. Record the exact
tests, screenshots, measurements, security checks, and known limitations.
Use [`handoff-template.md`](handoff-template.md) when returning work to
another agent or the maintainer.

## Stop conditions

Stop and report the boundary instead of guessing when:

- two documents claim different owners or entry points;
- the active queue does not permit the requested task;
- a new dependency lacks a documented reason;
- a user-visible action has no owner or command decision;
- a compatibility path has no removal condition;
- a remote or MCP action has no grant and denial behavior;
- required visual, security, or persistence evidence cannot be produced;
- a secret, credential, token, or sensitive host value would enter source,
  logs, tests, SQLite, or a commit.

## Canonical verification

Use the local orchestrator where possible:

```text
python3 scripts/verify.py --scope docs
python3 scripts/verify.py --scope agents
python3 scripts/verify.py --scope architecture
python3 scripts/verify.py --scope capabilities
python3 scripts/verify.py --scope surfaces
python3 scripts/verify.py --scope workflows
python3 scripts/verify.py --scope settings
python3 scripts/verify.py --scope performance
python3 scripts/verify.py --scope visual
python3 scripts/verify.py --scope evidence
python3 scripts/verify.py --scope automation
python3 scripts/verify.py --scope release
python3 scripts/verify.py --scope knowledge
python3 scripts/verify.py --scope rust
python3 scripts/verify.py --scope all
```

Report failed or unavailable scopes explicitly. Never hide a failure by
running only a smaller command and calling the broader change complete.
