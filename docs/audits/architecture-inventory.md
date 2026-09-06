# Architecture Inventory — Reset Baseline

**Status:** Working migration inventory
**Date:** 2026-09-06

This document records the current repository shape during the module migration.
It is evidence for the rework; it is not a target design. The target rules are
in the normative documents linked from `docs/README.md`.

## Current workspace crates

| Current crate | Current role | Target owner | Migration note |
|---|---|---|---|
| `app` | Binary/bootstrap | application composition | Keep small; remove feature logic. |
| `backend` | Mixed filesystem, PTY, SSH, SFTP, Git, hosts, settings, updater, MCP, persistence | split across platform services and feature modules | Highest-priority god-object boundary; SSH/SFTP contracts and adapters now isolate transport consumers. |
| `ai` | AI providers, sessions, tools | AI module | Keep backend-facing core; rebuild UI later. |
| `command-palette-core` | UI-free command descriptors and registry (new migration boundary) | command-palette module | Keep metadata and provider discovery here; feature-owned behavior remains outside the palette. |
| `command-palette` | Palette UI, static commands, duplicate shell dispatch integration | command-palette module | Consume the core registry; remove static entries and the duplicate shell registry. |
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
| `settings-ui` | Settings views and generated fields | settings module | Misplaced management categories are removed; remaining value audit and UI integration migration are open. |
| `keymap` | UI-free keymap values, resolution, and conflict handling | keymap module | Extracted from command-palette; management UI and shell integration remain to migrate. |
| `shell` | App shell, menus, commands, status items, updater | application composition + shell surface | Reduce to registration and composition. |
| `terminal` | Terminal engine and renderer support | terminal module | Split engine from GPUI view when useful. |
| `theme` | Runtime theme, fonts, and built-in color/icon registries | themes module | Keep one Themes owner; finish palette picker and transactional preview without remote downloads. |
| `ui-kit` | Shared UI primitives | foundation | Enforce as the only source of shared controls. |
| `workspace` | Workspace, tabs, panes, docks, views, and compatibility bridges | workspace plus tool modules | Transfer lifecycle/UI moved to `labonair-transfers` / `labonair-transfers-ui`; Workspace only submits requests and refreshes SFTP panes. |
| `transfers` | Typed transfer values, lifecycle registry, and service/event contracts | transfers module | New UI-free owner; backend worker adapter remains transitional. |
| `transfers-ui` | Statusbar-anchored transfer queue and resolution dialogs | transfers module | New canonical transfer presentation; uses only typed transfer contracts and shared UI primitives. |

## Current structural violations

The current Cargo metadata shows several transitional edges that conflict with the new rules:

- `workspace` depends directly on AI, backend, command palette, hosts UI, notifications, settings, SFTP capability contracts, and feature views; transfer lifecycle state is no longer one of those responsibilities.
- `settings-ui` depends on backend, workspace, hosts UI, command palette, notifications, and panel contracts.
- `panel-explorer` still depends on workspace for drag/preview shims, but its
  obsolete backend dependency has been removed; those remaining UI contracts
  are a later extraction boundary.
- `hosts-ui` depends on settings and notifications, even though Hosts is not a Settings concern and connection management should emit through the app notification contract.
- `command-palette` depends on backend even though the palette should receive dynamic data through providers.
- `keymap` is now UI-free, but the temporary GPUI adapter and some consumers
  still enter through `command-palette`; the keymap editor and stable command
  registration path are not complete.
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
- `panel-explorer` no longer declares or imports `labonair-backend`; filesystem
  access already uses `labonair-filesystem` directly.
- `panel-git-graph` no longer depends on `labonair-backend`; its graph contract and commit values live in `labonair-git` and the backend supplies an adapter.
- `panel-scm` and workspace Project Diff no longer depend on `labonair-backend`; source-control values and operations live in `labonair-git`, with the backend supplying the execution adapter.
- `shell/src/commands.rs`, `shell/src/status_items.rs`, and workspace views still contain feature-specific behavior that belongs to owning modules.
- `shell/src/commands.rs` still maintains a second behavior registry beside
  the command-palette entries; the migration must leave one typed command
  registry and keep execution in the owning modules.
- The former toast path has been removed; the statusbar is now the only
  notification presentation surface. The GPUI adapter remains until actions
  are migrated from callbacks to stable command IDs.

The dependency verifier allows only explicit transitional edges while these
boundaries are extracted. They are deliberately visible in the allow-list and
must not be treated as target architecture.

The allow-list also contains the following explicitly tracked migration
families. These are not target dependencies; each has a removal condition:

| Transitional edge family | Temporary reason | Removal condition |
|---|---|---|
| `settings-ui → backend`, `settings-ui → hosts-ui`, `settings-ui → workspace`, `settings-ui → command-palette`, `settings-ui → notifications`, `settings-ui → ai` | The existing settings window still composes legacy management and integration views. | Settings owns only value fields; management surfaces register independently and the window consumes contracts only. |
| `workspace → backend`, `workspace → ai`, `workspace → settings` | Workspace still hosts session bridges and legacy global settings consumers. SSH/SFTP and transfer access are now injected. | Settings providers and remaining session adapters are injected capabilities; workspace keeps orchestration only. |
| `hosts-ui → backend`, `hosts-ui → settings`, `hosts-ui → settings-content` | Host CRUD, credential writes, and the legacy Settings projection are still being migrated. | Host management uses `labonair-hosts`, credentials, secrets, and SSH contracts directly; no Settings projection remains. |
| `panel-explorer → workspace`, `panel-explorer → settings` | Explorer still reuses workspace drag/preview contracts and a legacy settings read. | Drag/drop and preview contracts move to foundation/owning modules and explorer receives a settings capability. |
| `panel-scm → editor`, `panel-scm → settings` | SCM reuses unified diff helpers and one legacy presentation preference. | Diff contracts are shared by the Git module and the preference is provided through a narrow settings contract. |
| `panel-ai → backend`, `panel-ai → editor`, `panel-ai → workspace` | AI UI is parked while the workspace/editor context bridge is redesigned. | AI consumes AI, editor-context, and workspace-session contracts without facade access. |
| `command-palette → backend`, `command-palette → settings`, `command-palette → filesystem` | Palette still contains legacy action dispatch and settings/file providers. | All entries are registered by owning modules through provider contracts. |
| `backend → feature contracts` | The backend is the current implementation adapter for extracted capabilities. | All consumers use injected adapters and backend exports no broad feature façade. |

The verifier's allow-list is the machine-readable source for the exact edge
set. Whenever an edge is added or removed, this table and the owning task must
be updated in the same change.

No new capability may be added to `backend`, `shell`, or `workspace` merely
because those crates already have access to it. New code must first establish
the owning capability crate and then inject or register it at composition.
This inventory is updated when a boundary moves; it is not a license to keep a
transitional edge after its removal condition has been met.

These are migration findings, not reasons to perform a destructive rewrite. Each edge should be removed when the owning contract exists and its consumers have moved.

## Migration order

1. Introduce stable IDs, typed domain events, and narrow service traits.
2. Extract platform services and capability contracts from `backend` without changing user behavior. The filesystem service, secret store, error contract, host domain contract, and shared database lifecycle are now standalone; their legacy adapters and direct consumers remain to be migrated.
3. Split notification state from presentation and replace toast rendering.
   `labonair-notifications-core` now owns the UI-free registry; the GPUI
   adapter and statusbar dropdown consume retained records.
4. Split command/keymap registries from the palette view.
5. Move transfers to their own module and statusbar owner. The typed registry, worker adapter, and statusbar UI are now in place; raw event compatibility remains only at the backend boundary.
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
