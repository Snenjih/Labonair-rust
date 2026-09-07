# Architecture Inventory — Reset Baseline

**Status:** Working migration inventory
**Date:** 2026-09-06

This document records the current repository shape during the module migration.
It is evidence for the rework; it is not a target design. The target rules are
in the normative documents linked from `docs/README.md`.

The detailed R06 backend export and consumer map is maintained in
[`backend-facade-inventory.md`](backend-facade-inventory.md). This baseline
keeps the crate-level graph; the detailed inventory keeps the symbol-level
removal conditions.

## Current workspace crates

| Current crate | Current role | Target owner | Migration note |
|---|---|---|---|
| `app` | Binary/bootstrap | application composition | Keep small; remove feature logic. |
| `backend` | Mixed filesystem, PTY, SSH, SFTP, Git transport adapters, settings, updater, MCP, and persistence wiring | split across platform services and feature modules | Highest-priority god-object boundary; Host CRUD/domain ownership has left this crate, while SSH/SFTP adapters and other compatibility surfaces remain. |
| `ai` | AI providers, sessions, tools | AI module | Keep backend-facing core; rebuild UI later. |
| `command-palette-core` | UI-free command descriptors and registry (new migration boundary) | command-palette module | Keep metadata and provider discovery here; feature-owned behavior remains outside the palette. Initial owner providers now live in workspace, terminal, editor, hosts, theme, and settings crates. |
| `command-palette` | Palette UI, dynamic sub-pages, and transitional duplicate shell dispatch integration | command-palette module | Consume the core registry; global-menu navigation is typed; remove static entries and the duplicate shell registry. |
| `editor` | Editor engine | editor module | Separate core from workspace view. |
| `filesystem` | Local file access, traversal, mutation, search, and watcher implementation | foundation/platform service | First extracted service boundary; only the legacy `AppEvent` adapter remains in `backend` temporarily. |
| `secrets` | Encrypted/plain local secret store and secret cache | foundation/platform service | Extracted from `backend`; backend keeps a compatibility adapter while SSH/Hosts/MCP migrate. |
| `errors` | Structured error catalog and recovery hints | foundation/platform contract | Extracted from `backend`; the backend compatibility module and root re-exports were removed in R06-001's first boundary. |
| `hosts` | Saved-host and host-group domain contract plus host store | hosts module | Models, host persistence, canonical picker snapshots, and typed SSH/SFTP requests are standalone; the shell composes one manager/window instance. Only the MCP event adapter and transport implementations remain transitional in `backend`. |
| `persistence` | Shared SQLite connection and schema lifecycle | foundation/platform service | Extracted from the host adapter; feature-specific queries still remain in `backend` and are next to migrate. |
| `credentials` | Credential domain, secret-backed metadata, and SSH keypair generation | credentials module | Extracted from `backend`; backend keeps App-signature adapters while callers migrate. |
| `snippets` | Snippet domain, SQLite store, and local/SSH execution contracts | snippets module | Shared run events and the SSH executor contract are standalone; backend owns only the russh adapter. |
| `gpui-ext` | Shared GPUI helpers | foundation | Keep dependency-free from features. |
| `interaction-contracts` | Stable shortcut and interaction identities | foundation | Keep UI-free and below command/keymap modules; no feature state or behavior. |
| `hosts-ui` | Host management UI and host-related dependencies | hosts module | Owns the native Hosts management window and consumes host, credential, snippet, database, and secret contracts directly; it emits typed open requests and operation failures publish through Notifications. |
| `notifications-core` | UI-free notification registry and lifecycle | notifications module | New owner of retention, ordering, deduplication, read state, and structured metadata. |
| `notifications` | GPUI notification adapter and statusbar dropdown | notifications module | Owns the statusbar notification item; shell only registers it. |
| `panel` | Panel/status contracts | workspace foundation | Keep contracts-only. |
| `panel-explorer` | Explorer panel | explorer module | Remove workspace dependency through contracts. |
| `panel-git-graph` | Git graph panel | git module | Consumes `labonair-git::GitGraphService`; backend implementation is injected at composition. |
| `panel-scm` | Source-control panel | git module | Consumes `labonair-git::GitService`; backend implementation is injected at composition. |
| `panel-snippets` | Snippet panel and execution UI | snippets module | Receives `Database` and execution contracts; has no backend-facade dependency. |
| `settings` | Layered settings store | settings module | Keep as core after removing misplaced categories. |
| `settings-content` | Typed settings data | settings module | Keep only actual configuration values. |
| `settings-json` | JSON editing | settings module | Keep as persistence adapter. |
| `settings-macros` | Settings derives | settings module | Keep implementation detail. |
| `settings-ui` | Settings views and generated fields | settings module | Value-only generated UI; receives only the system-font discovery contract. |
| `keymap` | UI-free keymap values, JSONC file format, default assets, resolution, and conflict handling | keymap module | File parser, default assets, lossless document, and management model belong to Keymap; GPUI presentation and keymap-owned diagnostics are isolated in `keymap-ui`, while shell retains only platform installation/watch wiring. |
| `shell` | App shell, menus, commands, status items, updater | application composition + shell surface | Reduce to registration and composition. |
| `terminal` | Terminal engine and renderer support | terminal module | Split engine from GPUI view when useful. |
| `theme` | Runtime theme, fonts, and built-in color/icon registries | themes module | Keep one Themes owner; app and icon palette pickers now use separate registry-backed pages with transactional preview; the current catalog is embedded and deterministic, while file/remote extensions remain deferred. |
| `ui-kit` | Shared UI primitives | foundation | Enforce as the only source of shared controls. |
| `workspace` | Workspace, tabs, panes, docks, views, and compatibility bridges | workspace plus tool modules | Transfer lifecycle/UI moved to `labonair-transfers` / `labonair-transfers-ui`; Workspace only submits requests and refreshes SFTP panes. |
| `background` | Background image storage and GPUI layer | backgrounds module | `BackgroundStore`, image import/delete, persistence, and rendering now live in `labonair-background`; no longer workspace-owned. |
| `transfers` | Typed transfer values, lifecycle registry, and service/event contracts | transfers module | New UI-free owner; backend worker adapter remains transitional. |
| `transfers-ui` | Statusbar-anchored transfer queue and resolution dialogs | transfers module | New canonical transfer presentation; uses only typed transfer contracts and shared UI primitives. |

## Current structural violations

The current Cargo metadata shows several transitional edges that conflict with the new rules:

- `workspace` depends directly on AI, backend, command palette, hosts UI, notifications, settings, SFTP capability contracts, and feature views; transfer lifecycle state is no longer one of those responsibilities.
- Workspace identity/activity now has one UI-free owner in `workspace/context.rs`
  (`WorkspaceIdentity` + `WorkspaceState`). The previous Hosts shell callback
  was removed; cross-surface Hosts navigation and the project-picker request
  use `WorkspaceEvent` and composition-root subscriptions. Project settings
  now follow explicit workspace identity rather than terminal cwd. Explorer
  and Git root synchronization also gives that explicit identity precedence;
  only standalone workspaces fall back to terminal cwd. Root precedence is
  centralized in `Workspace::filesystem_root` / `Workspace::git_root`, with
  pure resolver tests owned by `workspace/context.rs`; the shell only consumes
  those contracts. Remaining feature-view dependencies are still transitional
  and are not hidden by this state model. Session snapshots now persist the
  explicit workspace identity, with missing identity fields normalized to
  Standalone for backward compatibility. Identity mutation is now centralized
  behind the typed `WorkspaceTransition` contract; project-picker selection,
  session restore, and return-to-standalone do not use separate setter paths.
  Project-settings loading is also transition-driven rather than performed
  from `Workspace::render`; rejected project keys are reported once at that
  boundary.
- `settings-ui` depends on settings values, theme/UI primitives, notifications,
  command-palette fuzzy matching, and filesystem paths; it no longer depends on
  a workspace-owned background store, backend, Hosts UI, or panel contracts.
- `panel-explorer` still depends on workspace for drag/preview shims, but its
  obsolete backend dependency has been removed; those remaining UI contracts
  are a later extraction boundary.
- `hosts-ui` no longer depends on Settings or the backend facade; the shell
  composes one `HostManagerView`, injects its database, secret state, transport
  contracts, and the narrow MCP-revocation callback, then opens the manager
  through the Hosts-owned native window. Workspace retains only the injected
  capability handle needed for connection/session orchestration. Its
  notification contract migration is still open.
- `command-palette` still depends on backend even though the palette should receive dynamic data through providers; the runtime snapshot registry now makes that handoff explicit and is the removal seam for this edge.
- Initial command metadata providers now live in the owning workspace, terminal,
  editor, hosts, themes, and settings crates. The shell still contains
  transitional execution adapters and descriptors for those IDs; the adapter
  asserts equality against the provider snapshot so it cannot silently create
  a second palette source. The stable shortcut identity now lives in
  `interaction-contracts`, so keymap can publish its own command metadata
  through the one-way keymap → command-core edge.
- Workspace tab/pane commands, Git commands, and Snippet commands now also
  contribute owner-local provider metadata. The remaining shell palette rows
  are now absent; `Toggle Full Screen` is a shell-owned provider because it
  targets the native window directly. Keymap identity inversion remains a
  separate migration item.
- The command-palette core now owns the global palette command metadata, and
  Settings owns its toggle-command metadata. The palette-only shell table is
  consequently removed as a separate table; all rows enter through providers.
- The dependency verifier now explicitly allows owner crates to consume the
  UI-free command registry contract. `interaction-contracts` owns the stable
  shortcut identity, so the former command-core → keymap edge is removed and
  keymap owns its `Open Keymap` provider through the reverse one-way edge.
- `settings::OpenShortcuts` is modeled as a compatibility alias for the
  canonical `zed::OpenKeymap` action. Validation accepts it, while discovery
  excludes it from visible command rows.
- Dynamic palette pages now consume a typed `SubmenuRegistry` of immutable
  snapshots for tabs, hosts, recent hosts, themes, icon themes, editor themes,
  snippets, branches, symbols, and hidden status-bar items. Snapshot builders
  now live in the workspace, hosts, editor, theme, snippets, and Git provider
  modules; the shell only supplies live values and registers the snapshots.
  Hidden status-bar state and labels are now owned by the workspace status
  registry as well.
- `keymap` now owns the UI-free file contract, last-good recovery, and runtime
  resolution. The shell retains only the temporary GPUI installation and
  filesystem-watcher adapter; the keymap editor and stable command
  registration path are not complete.
- Command descriptors now carry owner-contributed default bindings. Keymap
  materializes those typed defaults into its default layer before applying the
  user file, so new feature defaults do not require a shell-owned shortcut
  table.
- `keymap::adapter::load` now owns command-vocabulary assembly, default-layer
  composition, and diagnostic snapshots. Shell keymap code is limited to the
  GPUI installation/display and filesystem-watch adapter.
- Owner providers now publish the migrated defaults for palette, editor search,
  workspace navigation, Zen mode, and shell debug actions. The old JSON asset
  remains as a compatibility layer until all default metadata is migrated.
- The command palette, panel tooltips, and tab context-menu hints now render
  from effective `CommandId` bindings. The old `ShortcutId` lookup is no longer
  an active UI source and remains exported only for migration compatibility.
- `keymap::file::KeymapDocument` now separates authoritative raw user source
  from derived parsing/validation state, providing the lossless foundation for
  the dedicated keymap management/editor surface.
- `backend` exposes a broad `App`, global event bus, and unrelated modules under one public crate.
- `backend` still owns the filesystem watcher adapter because it emits directly through the legacy app event bus; the actual watcher implementation now belongs to `labonair-filesystem`.
- `backend` still owns the public secret API adapter even though storage now belongs to `labonair-secrets`; existing SSH/Hosts/MCP call sites still pass the backend app handle.
- `backend` no longer re-exports the structured error contract; backend transport code imports `labonair-errors` directly. The remaining facade exports are tracked in [`backend-facade-inventory.md`](backend-facade-inventory.md).
- Host CRUD/domain ownership and its compatibility signatures have left
  `backend`; `labonair-hosts` now owns the store and the shell injects the one
  MCP revocation handler. Backend transport code still reads host records while
  SSH/SFTP adapters are migrated to narrower capability services.
- `backend` still owns only the transitional russh snippet-execution adapter and
  exposes a compatibility database re-export; snippet models, persistence,
  run events, and execution contracts now belong to `labonair-snippets`.
- `backend` now consumes the shared `labonair-persistence::Database` directly;
  the former `HostsDb` compatibility alias and backend Hosts module are gone.
  SSH/SFTP transport adapters still query host records through that foundation
  database and remain tracked migration work.
- `panel-snippets` no longer depends on `labonair-backend`; its database and SSH execution capabilities are injected from the composition root.
- `panel-explorer` no longer declares or imports `labonair-backend`; filesystem
  access already uses `labonair-filesystem` directly.
- `panel-git-graph` no longer depends on `labonair-backend`; its graph contract and commit values live in `labonair-git` and the backend supplies an adapter.
- `panel-scm` and workspace Project Diff no longer depend on `labonair-backend`; source-control values and operations live in `labonair-git`, with the backend supplying the execution adapter.
- `shell/src/commands.rs`, `shell/src/status_items.rs`, and workspace views still contain feature-specific behavior that belongs to owning modules.
- `shell/src/titlebar.rs` now owns only the permanent global-menu trigger and
  typed navigation events; Settings, Keymap, Themes, Icon Themes, and Hosts
  are handled by their owning surfaces through the composition root.
- The notification statusbar item has moved into `labonair-notifications`; the
  remaining shell status-item code is composition and other status surfaces.
- `shell/src/commands.rs` still maintains a second behavior registry beside
  the command-palette entries; the migration must leave one typed command
  registry and keep execution in the owning modules.
- The former toast path has been removed; the statusbar is now the only
  notification presentation surface. The GPUI adapter remains until actions
  are migrated from callbacks to stable command IDs.

### Notification surface audit

The Phase 3 audit is complete. Operation failures in Explorer, SFTP, Preview,
Git Graph, SCM, Hosts, SSH connection setup, editor operations, and Project
Diff publish to the central notification registry with a neutral summary and
expandable details. Repeated watcher/retry failures use source-scoped
deduplication keys. No active view duplicates those failures as a raw inline
banner, row, or form message.

The following surfaces are intentional exceptions because they are task
controls rather than passive error reporting:

| Surface | Why it remains | Boundary |
|---|---|---|
| Transfer conflict/file-error dialog | The user must choose overwrite, rename, skip, or abort before the worker can continue. | `labonair-transfers-ui` modal; the queue row exposes status only. |
| SFTP permission field validation | The invalid value must be corrected in the active form before submission. | `PermDialog` field validation. |
| Editor external-change prompt | The user must choose reload or keep the local buffer. | Editor conflict prompt. |

These controls may show the details needed to make the decision, but they must
not also emit a duplicate passive notification for the same interaction.

The dependency verifier allows only explicit transitional edges while these
boundaries are extracted. They are deliberately visible in the allow-list and
must not be treated as target architecture.

The allow-list also contains the following explicitly tracked migration
families. These are not target dependencies; each has a removal condition:

| Transitional edge family | Temporary reason | Removal condition |
|---|---|---|
| `workspace → background` | Workspace and the app shell render the background layer while the capability is being separated from workspace ownership. | Move background settings synchronization and any remaining background actions behind the dedicated background capability contract. |
| `workspace → backend`, `workspace → ai`, `workspace → settings` | Workspace still hosts session bridges and legacy global settings consumers. SSH/SFTP and transfer access are now injected. | Settings providers and remaining session adapters are injected capabilities; workspace keeps orchestration only. |
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
