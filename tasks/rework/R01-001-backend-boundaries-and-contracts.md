# R01-001 — Establish capability-owned backend boundaries

## Status

📋 Planned

## Scope

Phase 1 — Foundation contracts

## Owning area

Platform services and feature-module boundaries

## Goal

Reduce `labonair-backend` from a broad public application state container to narrow platform services and capability-owned contracts without changing user-facing behavior.

## Required outcome

- define stable IDs and typed domain events for hosts, SSH, SFTP, transfers, filesystem, PTY, Git, and persistence;
- extract the first service boundary without introducing a replacement god object;
- keep secrets and persistence behind narrow interfaces;
- make feature crates consume contracts rather than backend internals;
- preserve the existing app event transport only as a compatibility seam during migration;
- remove at least one documented transitional dependency edge;
- update `docs/audits/architecture-inventory.md` and the dependency allow-list.

## Constraints

- no feature behavior belongs in `labonair-shell`;
- no new stringly typed global event names;
- core contracts must not depend on GPUI;
- no broad `common` crate for unrelated types;
- preserve existing persistence formats unless a migration is explicitly defined.

## Verification

- focused unit tests for every new contract;
- dependency verifier passes with fewer transitional edges than before;
- `cargo fmt --check`;
- `cargo check --workspace --all-targets`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo test --workspace`.
