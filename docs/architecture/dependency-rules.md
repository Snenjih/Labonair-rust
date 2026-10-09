# Dependency Rules

**Status:** Normative
**Version:** 2
**Related:** [`../architecture.md`](../architecture.md), [`layers.md`](layers.md), [`graph.toml`](graph.toml), [`../audits/remaining-boundaries.md`](../audits/remaining-boundaries.md)

The architecture graph is an explicit policy, not a suggestion. Every internal
Cargo edge must be declared in [`graph.toml`](graph.toml), and every temporary
edge must have a removal condition, a bounded removal task, and a review
interval.

## Rules

1. Every product capability has one owner and one canonical capability crate.
2. Sibling crates are allowed only for a real contract, UI, storage, or
   integration boundary under that same owner.
3. Foundation crates do not depend on product modules.
4. Feature crates do not depend on `labonair-shell` or private state from
   another feature.
5. Composition roots may wire concrete implementations but must not become
   feature god objects.
6. Shared discovery uses a typed registry only when there are multiple
   providers or consumers; one caller gets a narrow typed contract.
7. UI-free engines remain UI-free transitively.
8. Panels never depend on another panel, and the panel contract stays below
   Workspace and the shell.
9. New dependencies require an ownership explanation, a task, and a passing
   architecture check.
10. A removed edge must be removed from the manifest, code, audit, and task
    once no consumer needs it.
11. A transitional edge whose review interval has expired fails the
    architecture check until its owner records a fresh review.

## Verification

Run the narrow check while editing boundaries:

```text
python3 scripts/verify.py --scope architecture
```

This validates the manifest against Cargo metadata, checks acyclicity, and
retains the transitive UI/panel invariants. Regenerate the readable projection
with:

```text
python3 scripts/gen_architecture.py
```

The full repository verifier is `python3 scripts/verify.py --scope all`.
