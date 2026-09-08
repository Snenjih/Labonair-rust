# R04-002 — Establish one Hosts owner and SSH/SFTP picker flow

## Status

`✅ Done`

## Owner

- Module: `hosts` with transport actions delegated to `ssh` and `sftp`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::bootstrap`

## Dependencies

- `R03-001-command-palette-provider-registry`
- `R03-002-keymap-runtime-and-editor`
- `R02-003-project-entry-and-workspace-transitions`

## Goal

Give saved hosts a quiet, discoverable home outside Settings. Management and
selection must use the same host registry, with Enter opening SSH and
Shift+Enter opening SFTP, while jump hosts remain part of SSH configuration.

## Scope

- In scope: HostStore ownership, management view, searchable palette provider,
  stable host IDs, SSH/SFTP action requests, recent-host ordering, and secret
  references.
- Out of scope: a separate jump-host menu, remote project discovery, host
  marketplace/sync, or duplicating transport implementations in Hosts.

## Contracts and ownership

- `labonair-hosts` owns host models, lifecycle, ordering, persistence contract,
  and management commands.
- `labonair-ssh` and `labonair-sftp` own connection/session execution; Hosts
  emits typed requests and never reaches transport internals.
- The palette consumes immutable host snapshots and dispatches typed primary
  and secondary actions.
- The management UI uses UI-kit search, list, form, disclosure, dialog, and
  validation components.

## Dependencies

- Existing edges removed: host data in Settings, shell-owned host tables, and
  any standalone jump-host presentation surface.
- New edges: Hosts may depend on persistence, secrets, credentials, and public
  SSH/SFTP contracts; it must not depend on workspace or shell UI.
- Dependency verifier change: remove transitional backend/settings edges as
  each consumer moves.

## Persistence and migration

- Settings: no host definitions are stored in Settings.
- Storage: host records remain in the Hosts-owned store; secrets remain keychain
  references and never enter SQLite or serialized settings.
- Compatibility: migrate legacy host records once, preserving IDs where
  possible; unknown/invalid records remain recoverable and are diagnosed.

## User-visible behavior

- Canonical entry point: titlebar global menu → Hosts for management, and the
  Hosts submenu in the command palette for fast connection.
- Notifications: connection, migration, and persistence failures use the
  notification registry with source/details and actionable IDs where useful.
- Inline errors/toasts: passive failures use notifications; form validation
  may remain beside the field being corrected.

## Implementation plan

1. Define HostStore snapshots and typed open requests → verify ordering,
   filtering, migration, and Enter/Shift+Enter semantics.
2. Move management UI and persistence behind the Hosts owner → verify secret
   references and notification behavior.
3. Remove Settings/shell host paths and the standalone jump-host surface →
   verify dependency graph and visual management/picker states.

## Acceptance criteria

- [x] Hosts management is absent from Settings and has one canonical surface
      (the Hosts-owned native management window; `OpenHosts` command identity).
- [x] The palette lists configured hosts, supports filtering, and opens SSH on
      Enter and SFTP on Shift+Enter (owner-provided `HostPickerRow` snapshots).
- [x] Jump hosts remain available in SSH configuration without a separate
      primary menu or badge (verified by the R07-001 anti-pattern sweep).
- [x] Host definitions and secrets have one persistence owner each
      (`labonair-hosts` store + `labonair-secrets`; SQLite-to-Settings
      projection removed).
- [x] Focused tests and all repository verification gates pass.
- [ ] Empty, filtered, configured, invalid, and connection-pending states are
      visually checked. *Deferred into the R07-001 visual matrix.*

## Progress

- [x] Added `HostOpenRequest`/`HostOpenMode` and the immutable
      `HostPickerRow` contract to `labonair-hosts`; SSH is primary and SFTP is
      the secondary picker action.
- [x] Composed one `HostManagerView` in the shell and added the Hosts-owned
      native management window. Titlebar Hosts, the Open Hosts command, and
      host-connection error recovery now use that canonical surface.
- [x] The command palette consumes owner-provided host snapshots and converts
      selections into typed SSH/SFTP requests at the composition boundary.
- [x] Removed the obsolete SQLite-hosts-to-Settings projection, migration
      marker, and Settings host model. Host definitions remain in the
      Hosts-owned store and secrets remain in the secret store.
- [x] Removed the backend Hosts CRUD compatibility module and generic
      `HostsDb` alias. Persistence initialization now uses the foundation
      database contract directly; only the explicitly tracked MCP integration
      and transport adapters remain in backend.
- [ ] Perform the visual state review for empty, filtered, invalid, and
      pending flows. *Deferred into the R07-001 visual matrix.*

## Removal condition

This task is complete only when no host definition, host-management action, or
connection picker is implemented in Settings, shell, or the palette itself.

## Notes and follow-ups

Standalone SSH/SFTP remains a first-class workflow; it may use a quick-connect
dialog without creating a saved host.
