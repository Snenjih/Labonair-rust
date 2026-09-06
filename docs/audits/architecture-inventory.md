# Architecture Inventory — Reset Baseline

**Status:** Working migration inventory
**Date:** 2026-09-06

This document records the current repository shape during the module migration. It is evidence for the rework; it is not a target design.

## Current workspace crates

| Current crate | Current role | Target owner | Migration note |
|---|---|---|---|
| `app` | Binary/bootstrap | application composition | Keep small; remove feature logic. |
| `backend` | Mixed filesystem, PTY, SSH, SFTP, Git, hosts, settings, updater, MCP, persistence | split across platform services and feature modules | Highest-priority god-object boundary. |
| `ai` | AI providers, sessions, tools | AI module | Keep backend-facing core; rebuild UI later. |
| `command-palette` | Palette UI, static commands, some keymap behavior | command-palette module + keymap module | Split registry/core from GPUI view. |
| `editor` | Editor engine | editor module | Separate core from workspace view. |
| `filesystem` | Local file access, traversal, mutation, search, and watcher implementation | foundation/platform service | First extracted service boundary; only the legacy `AppEvent` adapter remains in `backend` temporarily. |
| `secrets` | Encrypted/plain local secret store and secret cache | foundation/platform service | Extracted from `backend`; backend keeps a compatibility adapter while SSH/Hosts/MCP migrate. |
| `errors` | Structured error catalog and recovery hints | foundation/platform contract | Extracted from `backend`; capability crates can consume it without importing the backend facade. |
| `hosts` | Saved-host and host-group domain contract plus host store | hosts module | Models and all host persistence, including secret-bearing writes, are standalone; only the MCP event adapter remains transitional in `backend`. |
| `persistence` | Shared SQLite connection and schema lifecycle | foundation/platform service | Extracted from the host adapter; feature-specific queries still remain in `backend` and are next to migrate. |
| `credentials` | Credential domain, secret-backed metadata, and SSH keypair generation | credentials module | Extracted from `backend`; backend keeps App-signature adapters while callers migrate. |
| `snippets` | Snippet domain, SQLite store, and local/SSH execution contracts | snippets module | Shared run events and the SSH executor contract are standalone; backend owns only the russh adapter. |
| `gpui-ext` | Shared GPUI helpers | foundation | Keep dependency-free from features. |
| `hosts-ui` | Host management UI and host-related dependencies | hosts module | Remove settings and notification coupling. |
| `notifications-core` | UI-free notification registry and lifecycle | notifications module | New owner of retention, ordering, deduplication, read state, and structured metadata. |
| `notifications` | GPUI notification adapter | notifications module | Toast renderer removed; statusbar dropdown remains the consumer. |
| `panel` | Panel/status contracts | workspace foundation | Keep contracts-only. |
| `panel-explorer` | Explorer panel | explorer module | Remove workspace dependency through contracts. |
| `panel-git-graph` | Git graph panel | git module | Consumes `labonair-git::GitGraphService`; backend implementation is injected at composition. |
| `panel-scm` | Source-control panel | git module | Consumes `labonair-git::GitService`; backend implementation is injected at composition. |
| `panel-snippets` | Snippet panel and execution UI | snippets module | Receives `Database` and execution contracts; has no backend-facade dependency. |
| `settings` | Layered settings store | settings module | Keep as core after removing misplaced categories. |
| `settings-content` | Typed settings data | settings module | Keep only actual configuration values. |
| `settings-json` | JSON editing | settings module | Keep as persistence adapter. |
| `settings-macros` | Settings derives | settings module | Keep implementation detail. |
| `settings-ui` | Settings views and generated fields | settings module | Remove Hosts, Themes, Icon Themes, Shortcuts management. |
| `shell` | App shell, menus, commands, status items, updater | application composition + shell surface | Reduce to registration and composition. |
| `terminal` | Terminal engine and renderer support | terminal module | Split engine from GPUI view when useful. |
| `theme` | Runtime theme and fonts | themes module | Add explicit color/icon registries. |
| `ui-kit` | Shared UI primitives | foundation | Enforce as the only source of shared controls. |
| `workspace` | Workspace, tabs, panes, docks, views, transfers, bridges | workspace plus tool modules | Reduce cross-feature ownership gradually. |

## Current structural violations

The current Cargo metadata shows several transitional edges that conflict with the new rules:

- `workspace` depends directly on AI, backend, command palette, hosts UI, notifications, settings, SFTP-related views, and feature views.
- `settings-ui` depends on backend, workspace, hosts UI, command palette, notifications, and panel contracts.
- `panel-explorer` depends on workspace, which prevents independent feature ownership.
- `hosts-ui` depends on settings and notifications, even though Hosts is not a Settings concern and connection management should emit through the app notification contract.
- `command-palette` depends on backend even though the palette should receive dynamic data through providers.
- `backend` exposes a broad `App`, global event bus, and unrelated modules under one public crate.
- `backend` still owns the filesystem watcher adapter because it emits directly through the legacy app event bus; the actual watcher implementation now belongs to `labonair-filesystem`.
- `backend` still owns the public secret API adapter even though storage now belongs to `labonair-secrets`; existing SSH/Hosts/MCP call sites still pass the backend app handle.
- `backend` still re-exports the structured error contract for old internal paths, while the implementation now belongs to `labonair-errors`.
- `backend` still exposes compatibility signatures for host and credential operations and owns the MCP event adapter; host and credential models/persistence now belong to their capability crates.
- `backend` still owns only the transitional russh snippet-execution adapter and
  exposes a compatibility database re-export; snippet models, persistence,
  run events, and execution contracts now belong to `labonair-snippets`.
- `backend` still exposes the shared database under the compatibility name `HostsDb`; connection/schema lifecycle now belongs to `labonair-persistence`.
- `panel-snippets` no longer depends on `labonair-backend`; its database and SSH execution capabilities are injected from the composition root.
- `panel-git-graph` no longer depends on `labonair-backend`; its graph contract and commit values live in `labonair-git` and the backend supplies an adapter.
- `panel-scm` and workspace Project Diff no longer depend on `labonair-backend`; source-control values and operations live in `labonair-git`, with the backend supplying the execution adapter.
- `shell/src/commands.rs`, `shell/src/status_items.rs`, and workspace views still contain feature-specific behavior that belongs to owning modules.
- The former toast path has been removed; the statusbar is now the only
  notification presentation surface. The GPUI adapter remains until actions
  are migrated from callbacks to stable command IDs.

The dependency verifier allows four additional transitional edges while these
boundaries are extracted: command palette → settings, explorer → settings,
SCM → editor, and SCM → settings. They are deliberately visible in the
allow-list and must not be treated as target architecture.

These are migration findings, not reasons to perform a destructive rewrite. Each edge should be removed when the owning contract exists and its consumers have moved.

## Migration order

1. Introduce stable IDs, typed domain events, and narrow service traits.
2. Extract platform services and capability contracts from `backend` without changing user behavior. The filesystem service, secret store, error contract, host domain contract, and shared database lifecycle are now standalone; their legacy adapters and direct consumers remain to be migrated.
3. Split notification state from presentation and replace toast rendering.
   `labonair-notifications-core` now owns the UI-free registry; the GPUI
   adapter and statusbar dropdown consume retained records.
4. Split command/keymap registries from the palette view.
5. Move transfers to their own module and statusbar owner.
6. Move hosts and SSH ownership out of Settings/workspace.
7. Move terminal/editor/SFTP views to their owning modules.
8. Remove compatibility edges and enforce the target graph.

## Evidence commands

The inventory was produced from:

```text
cargo metadata --no-deps --format-version 1
find crates/backend/src/modules -maxdepth 2 -type f
rg -n "pub struct|pub enum|pub trait|pub fn" crates/backend/src/modules crates/shell/src crates/workspace/src
```
