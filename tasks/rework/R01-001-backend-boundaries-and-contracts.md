# R01-001 — Establish capability-owned backend boundaries

## Status

`✅ Done`

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

## Progress

- The first platform boundary is extracted as `labonair-filesystem`.
- Pure file access, directory traversal, mutation, path resolution, and search
  no longer live in `labonair-backend`.
- The legacy watcher remains in `labonair-backend` until its direct `App`/event
  bus dependency is replaced by a typed callback or domain-event contract.
- Secret storage is now extracted as `labonair-secrets`; the backend exposes
  only compatibility wrappers while its capability callers migrate.
- The structured error catalog is now extracted as `labonair-errors`; the
  backend keeps only a re-export for old internal paths.
- Saved-host domain models are now extracted as `labonair-hosts`; the backend
  still owns the transitional SQLite store and App-bound operations.
- Shared SQLite connection/schema lifecycle is now extracted as
  `labonair-persistence`; feature-specific stores still use the compatibility
  backend database handle until their queries move behind capability APIs.
- Host reads, ordering, group mutations, and secret-bearing writes now use
  `labonair_hosts::store`; the backend retains only compatibility signatures
  and the MCP-specific event adapter.
- Credential metadata, secret-backed operations, host references, and SSH
  keypair generation now use `labonair-credentials`; the backend retains only
  App-signature adapters while callers migrate.
- Snippet models and SQLite persistence now use `labonair-snippets`; local
  process execution, shared run events, and the `SshCommandExecutor` contract
  live there as well. The backend now supplies only a russh adapter, injected
  into the panel as a capability trait object; the panel no longer subscribes
  to raw snippet events or imports backend execution functions.
- Notification lifecycle is now UI-free in `labonair-notifications-core`.
  The GPUI adapter delegates retention, deduplication, read state, details,
  and actions to that registry; the shell no longer mounts a toast overlay.
- The statusbar notification dropdown now shows the unread badge, all retained
  records in a scrollable list, and expandable details. Notification records
  are not auto-dismissed.
- `panel-snippets` no longer depends on `labonair-backend`; its store receives
  cloneable `labonair-persistence::Database` infrastructure and its SSH runner
  receives the `SshCommandExecutor` capability from the composition root.
- The Git graph now consumes `labonair-git::GitGraphService` and
  `labonair-git::CommitInfo`; `BackendGitGraphService` is injected by the
  composition root, so the graph panel no longer imports the backend facade.
- The SCM panel and workspace Project Diff now consume `labonair-git::GitService`;
  Git value types and source-control operations are UI-free, and
  `BackendGitService` is the only concrete execution adapter.
- `panel-explorer` no longer declares the backend facade; it already uses the
  standalone filesystem service directly.
- The remaining backend adapters and legacy event consumers are explicitly
  recorded as transitional edges in `docs/audits/architecture-inventory.md`.
  They are follow-up migrations, not an unfinished acceptance criterion for
  this foundation task.

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

## Completion note

This task establishes the first capability-owned contracts and the strict
inventory/allow-list boundary. It does not claim that `labonair-backend` has
already disappeared; its remaining adapters have named removal conditions in
the inventory and are migrated by later bounded tasks.
