# R01-002 — Extract SSH and SFTP capability contracts

## Status

`✅ Done`

## Owner

- Module: `ssh` and `sftp` (transport capabilities)
- Capability matrix: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition root: `crates/shell/src/bootstrap.rs`

## Goal

Move SSH authentication, session lifecycle, PTY execution, tunnels, jump-host
routing, and SFTP operations behind typed capability contracts so workspace,
hosts, and views do not depend on the backend facade or transport internals.

## Scope

- In scope: session handles, connection prompts, PTY execution, tunnel
  lifecycle, remote filesystem operations, and backend adapters.
- Out of scope: redesigning the Hosts management UI, changing SSH/SFTP
  user-visible behavior, or extracting the transfer queue. The legacy
  transfer queue is tracked by `R01-003`, even though its old compatibility
  symbols still live below the backend's historical SFTP module.

## Contracts and ownership

- Create UI-free `labonair-ssh` and `labonair-sftp` contracts only where a
  real consumer boundary exists.
- Keep authentication, trust prompts, reconnect, and jump hosts as separate
  typed concerns; do not create one transport god-trait.
- Views receive injected services and typed events.
- User-facing failures enter the notification registry.

## Dependencies and migration

- Remove direct `workspace`/`hosts-ui` access to backend SSH/SFTP transport
  internals. Transfer-queue compatibility imports are explicitly excluded
  until `R01-003` supplies the transfer contract.
- Keep russh and russh-sftp implementations in backend adapters initially.
- Update the dependency verifier and capability matrix with every edge.

## Acceptance criteria

- [x] Contracts are UI-free and have focused tests.
- [x] Workspace, SFTP view, and Hosts UI do not import backend SSH/SFTP
      transport modules directly; transfer compatibility is tracked by
      `R01-003`.
- [x] Jump-host routing and trust/auth prompts retain their behavior.
- [x] No new raw global event payload is introduced.
- [x] Full task verification gates pass.
