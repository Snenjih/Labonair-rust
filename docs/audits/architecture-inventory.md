# Architecture Inventory — Reset Baseline

**Status:** Working migration inventory
**Date:** 2026-09-07

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
| `backend` | Removed | none | The former broad facade and its module tree were deleted. Concrete SSH snippet execution is owned by `labonair-snippets-ssh`; transfer execution and adapters are owned by `labonair-transfers-ssh`. |
| `ai` | AI providers, sessions, tools | AI module | Keep backend-facing core; rebuild UI later. |
| `command-palette-core` | UI-free command descriptors and registry (new migration boundary) | command-palette module | Keep metadata and provider discovery here; feature-owned behavior remains outside the palette. Initial owner providers now live in workspace, terminal, editor, hosts, theme, and settings crates. |
| `command-palette` | Palette UI and dynamic sub-pages | command-palette module | Consume the core registry; global-menu navigation and owner action forwarding are typed; shell does not duplicate feature dispatch. |
| `editor` | Editor engine | editor module | Separate core from workspace view. |
| `filesystem` | Local file access, traversal, mutation, search, and watcher implementation | foundation/platform service | Canonical owner; backend watcher state and filesystem compatibility re-exports have been removed. |
| `secrets` | Encrypted/plain local secret store and secret cache | foundation/platform service | Extracted from `backend`; backend and shell callers use `labonair-secrets` directly with explicit `SecretsState`. |
| `errors` | Structured error catalog and recovery hints | foundation/platform contract | Extracted from `backend`; the backend compatibility module and root re-exports were removed in R06-001's first boundary. |
| `events` | UI-free in-process adapter event transport | foundation/platform contract | Extracted from `backend`; it carries raw adapter events only, while typed event vocabulary remains in the owning capability contracts. |
| `ssh-transport` | Concrete russh SSH implementation and contract adapters | SSH module | Extracted from `backend`; `labonair-ssh` remains the UI-free contract crate, while this sibling owns sessions, PTY, authentication, tunnels, and remote operations. |
| `sftp-ssh` | Concrete russh-sftp session setup and SFTP contract adapters | SFTP module | Dedicated integration sibling extracted from the backend; receives SSH state and EventBus explicitly and owns the SFTP session lifecycle boundary. |
| `git-transport` | Concrete local/remote Git CLI execution and Git contract adapters | Git module | Dedicated integration sibling extracted from the backend; receives SSH state and EventBus explicitly and keeps `labonair-git` contracts implementation-free. |
| `mcp-server` | Concrete MCP HTTP server, grants, PTY bridge, and MCP event/service adapters | AI/MCP module | Dedicated integration sibling extracted from the backend; receives explicit terminal, SSH, database, secrets, and EventBus capabilities while `labonair-mcp-core` remains contracts-only. |
| `hosts` | Saved-host and host-group domain contract plus host store | hosts module | Models, host persistence, canonical picker snapshots, and typed SSH/SFTP requests are standalone; the shell composes one manager/window instance. MCP event and transport implementations are isolated in their integration siblings. |
| `persistence` | Shared SQLite connection and schema lifecycle | foundation/platform service | Extracted from the host adapter; feature queries are owned by their capability crates or integration siblings. |
| `credentials` | Credential domain, secret-backed metadata, and SSH keypair generation | credentials module | Extracted from `backend`; the unused backend compatibility module is removed. |
| `snippets` | Snippet domain, SQLite store, and local/SSH execution contracts | snippets module | Shared run events and the SSH executor contract are standalone; `labonair-snippets-ssh` supplies the concrete russh adapter. |
| `snippets-ssh` | Concrete SSH snippet execution adapter | snippets module | Dedicated integration sibling | Receives `SshState`, `Database`, and `EventBus` through the snippet execution contract; no application facade. |
| `gpui-ext` | Shared GPUI helpers | foundation | Keep dependency-free from features. |
| `interaction-contracts` | Stable shortcut and interaction identities | foundation | Keep UI-free and below command/keymap modules; no feature state or behavior. |
| `hosts-ui` | Host management UI and host-related dependencies | hosts module | Owns the native Hosts management window and consumes host, credential, snippet, database, and secret contracts directly; it emits typed open requests and operation failures publish through Notifications. |
| `notifications-core` | UI-free notification registry and lifecycle | notifications module | New owner of retention, ordering, deduplication, read state, and structured metadata. |
| `notifications` | GPUI notification adapter and statusbar dropdown | notifications module | Owns the statusbar notification item; shell only registers it. |
| `panel` | Panel/status contracts | workspace foundation | Keep contracts-only. |
| `explorer-host` | Explorer host contract plus the drag/preview value types shared with the terminal and preview views | explorer module | Leaf (only `gpui`); lets `panel-explorer` and `workspace` interoperate without either depending on the other (R07-004). |
| `snippets-host` | Snippet execution-host contract (inject / run local / run SSH terminal / SSH-session lookup) | snippets module | Leaf (only `gpui`); lets `panel-snippets` run snippets without depending on `labonair-workspace` (R08-003). |
| `panel-explorer` | Explorer panel | explorer module | Workspace dependency removed (R07-004): opens files/terminals/previews and reads the active file through the injected `labonair-explorer-host::ExplorerHost` contract. |
| `panel-git-graph` | Git graph panel | git module | Consumes `labonair-git::GitGraphService`; the `labonair-git-transport` implementation is injected at composition. |
| `panel-scm` | Source-control panel | git module | Consumes `labonair-git::GitService`; backend implementation is injected at composition. |
| `panel-snippets` | Snippet panel and execution UI | snippets module | Receives `Database` and execution contracts; has no backend-facade dependency. R08-003: no `labonair-workspace` edge — runs snippets through the injected `labonair-snippets-host::SnippetExecutionHost`. |
| `settings` | Layered settings store | settings module | Keep as core after removing misplaced categories. |
| `settings-content` | Typed settings data | settings module | Keep only actual configuration values. |
| `settings-json` | JSON editing | settings module | Keep as persistence adapter. |
| `settings-macros` | Settings derives | settings module | Keep implementation detail. |
| `settings-ui` | Settings views and generated fields | settings module | Value-only generated UI; receives only the system-font discovery contract. |
| `keymap` | UI-free keymap values, JSONC file format, default assets, resolution, and conflict handling | keymap module | File parser, default assets, lossless document, and management model belong to Keymap; GPUI presentation and keymap-owned diagnostics are isolated in `keymap-ui`, while shell retains only platform installation/watch wiring. |
| `shell` | App shell, menus, native actions, and composition | application composition + shell surface | Feature command/status contributions are owner-registered; remaining adapters are limited to composition and native-window actions. |
| `terminal` | Terminal engine and renderer support | terminal module | Split engine from GPUI view when useful. |
| `theme` | Runtime theme, fonts, and built-in color/icon registries | themes module | Keep one Themes owner; app and icon palette pickers now use separate registry-backed pages with transactional preview; the current catalog is embedded and deterministic, while file/remote extensions remain deferred. Keeps NO `labonair-settings` dependency. |
| `theme-ui` | Settings→`ThemeStore` application policy (`apply_prefs_to_theme`, metrics, preview/activate/persist) and the palette theme-action handler | themes module | Extracted from `settings-ui` (R08-005) so the Settings UI is values-only; the settings→theme direction lives only here. |
| `ui-kit` | Shared UI primitives | foundation | Enforce as the only source of shared controls. |
| `workspace` | Workspace, tabs, panes, docks, views, and compatibility bridges | workspace plus tool modules | Transfer lifecycle/UI moved to `labonair-transfers` / `labonair-transfers-ui`; Workspace only submits requests and refreshes SFTP panes. |
| `background` | Background image storage and GPUI layer | backgrounds module | `BackgroundStore`, image import/delete, persistence, and rendering now live in `labonair-background`; no longer workspace-owned. |
| `background-host` | Background presentation contract (`LayerScope`, `BackgroundHost`, `BackgroundPulse`) | backgrounds module | Leaf (only `gpui`); lets `labonair-workspace` render the background layer without depending on `labonair-background`'s image storage/import/decoding (R07-005 / B02). |
| `updater` | Update manifest, download/verification/install logic | updater module | GPUI presentation is isolated in the `updater-ui` sibling; capability logic remains in `labonair-updater` and is no longer backend-owned. |
| `transfers` | Typed transfer values, lifecycle registry, and service/event contracts | transfers module | UI-free owner; concrete SFTP execution is supplied by the `transfers-ssh` integration sibling. |
| `transfers-ssh` | Concrete SFTP transfer worker and russh/russh-sftp execution adapter | transfers module | Dedicated integration sibling extracted from the backend; owns chunking, checksums, conflicts, cancellation, and reconnect requeue behavior. |
| `transfers-ui` | Statusbar-anchored transfer queue and resolution dialogs | transfers module | New canonical transfer presentation; uses only typed transfer contracts and shared UI primitives. R08-001: no `labonair-workspace` edge — emits `TransferUiEvent::Completed`; the shell routes it to the SFTP pane refresh. |

## Generated current dependency graph

The following graph is generated from `cargo metadata` and describes the
current workspace, not the target architecture. Regenerate it with
[`scripts/gen-crate-graph.sh`](../../scripts/gen-crate-graph.sh); the source and
rendered artifact are committed so architecture reviews can inspect the exact
snapshot used by this inventory.

![Current Labonair crate dependency graph](../assets/crate-graph.svg)

Source: [`crate-graph.dot`](../assets/crate-graph.dot).

## Current boundary ledger

The current Cargo metadata contains the following explicit boundary edges. They
are either composition-approved public-contract edges or named transitional
edges; the dependency verifier rejects every unlisted edge.

- `workspace` depends directly on AI, command palette, hosts UI, notifications, settings, SFTP capability contracts, and feature views; transfer lifecycle state and adapter event transport are no longer direct responsibilities.
- Workspace identity/activity now has one UI-free owner in `workspace/context.rs`
  (`WorkspaceIdentity` + `WorkspaceState`). The previous Hosts shell callback
  was removed; cross-surface Hosts navigation and the project-picker request
  use `WorkspaceEvent` and composition-root subscriptions. Project settings
  now follow explicit workspace identity rather than terminal cwd. Explorer
  and Git root synchronization also gives that explicit identity precedence;
  only standalone workspaces fall back to terminal cwd. Root precedence is
  centralized in `Workspace::filesystem_root` / `Workspace::git_root`, with
  pure resolver tests owned by `workspace/context.rs`; the shell only consumes
  those contracts. Remaining feature-view dependencies are listed in the
  boundary table below and are not hidden by this state model. Session snapshots now persist the
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
  R08-005 moved the `ThemeStore` application policy and the palette theme
  handler into `labonair-theme-ui`; `settings-ui` now only *calls*
  `labonair_theme_ui::apply_prefs_to_theme` after it writes a value, and owns
  no theme preview/activation/persistence policy.
- `panel-explorer` no longer depends on `workspace` (R07-004). Its open-file,
  open-terminal, open-preview, and active-file intents cross the injected
  `labonair-explorer-host::ExplorerHost` contract; the shared `DraggedPaths` /
  `quote_paths` / `is_previewable` value types now live in the leaf
  `labonair-explorer-host` crate that both `panel-explorer` and `workspace`
  consume.
- `hosts-ui` no longer depends on Settings or the backend facade; the shell
  composes one `HostManagerView`, injects its database, secret state, transport
  contracts, and the narrow MCP-revocation callback, then opens the manager
  through the Hosts-owned native window. Workspace retains only the injected
  capability handle needed for connection/session orchestration. Its
  notification publication uses the retained Notifications contract.
- `command-palette` no longer depends on a backend facade. Dynamic data and
  action execution are supplied through owner-local registries; shell action
  handling only forwards opaque typed actions.
- Initial command metadata providers now live in the owning workspace, terminal,
  editor, hosts, themes, and settings crates. The shell retains only native
  window/debug command registration and composition callbacks; it does not
  provide feature rows. The stable shortcut identity now lives in
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
- The command-palette module now also owns the executable `Open Command
  Palette` contribution. The shell injects only the modal-toggle host callback
  during composition; it no longer stores palette behavior in its transitional
  command table.
- Workspace now owns the executable `Find` contribution as well. The shell
  injects only the search-overlay host callback, so the search behavior remains
  with the Workspace surface without coupling the Workspace crate to the shell
  modal composition.
- Hosts-UI now owns the executable connection command contributions. The shell
  injects only the callback that opens the canonical Hosts picker; SSH/SFTP
  intent remains represented by the picker row's primary and secondary actions.
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
- `keymap` now owns the UI-free file contract, last-good recovery, runtime
  resolution, and management surface contract. The shell retains only the
  GPUI installation and filesystem-watcher adapter.
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
- `labonair-shell` owns the composition-only `AppComposition` bundle in
  `shell::composition`. The former backend crate no longer exposes an
  aggregate `App`/`AppState` facade; concrete integrations are named sibling
  crates and are injected through capability contracts.
  Shell's direct `labonair-persistence` edge is intentional: the composition
  root initializes the shared database before injecting it into capability
  adapters.
- `labonair-filesystem` now owns the complete filesystem boundary. The backend no longer carries watcher state, watcher adapters, or filesystem compatibility re-exports; no filesystem event is synthesized through the global backend bus.
- Local MCP terminal observation and writes now use the injected
  `labonair_mcp_core::LocalTerminalAccess` contract backed by the real
  `labonair-terminal::TerminalRegistry`. The obsolete backend `PtyState`,
  `local_pty_id` grant address, and duplicate local PTY implementation are
  removed. Shared OSC 7/133 payloads live in the UI-free
  `labonair-terminal-integration` protocol crate, so the backend does not
  depend on the GPUI terminal engine.
- Secret storage belongs to `labonair-secrets`; SSH/SFTP/MCP and snippet
  integration call sites receive explicit state rather than an aggregate
  application handle.
- The structured error contract is consumed directly from `labonair-errors`.
  The stale `labonair-ai → backend` dependency and the complete backend crate
  were removed. System-font discovery belongs to `labonair-theme`; the unused
  backend custom-font module was removed. The R06 removal evidence is tracked
  in [`backend-facade-inventory.md`](backend-facade-inventory.md).
- Host CRUD/domain ownership and its compatibility signatures have left the
  former backend; `labonair-hosts` now owns the store and the shell injects the one
  MCP revocation handler. Backend transport code still reads host records while
  SSH/SFTP adapters are migrated to narrower capability services.
- Snippet models, persistence, run events, and execution contracts belong to
  `labonair-snippets`; the concrete russh executor is isolated in
  `labonair-snippets-ssh`.
- The former backend consumed the shared `labonair-persistence::Database`; the
  `HostsDb` compatibility alias and backend Hosts module are gone.
  SSH/SFTP transport adapters still query host records through that foundation
  database and remain explicit integration inputs.
- `panel-snippets` receives its database and SSH execution capabilities through
  the composition root, with concrete execution owned by `labonair-snippets-ssh`.
  R08-003 removed its `labonair-workspace` dependency: terminal inject, local
  run, SSH-terminal run, and active-SSH-session lookup now cross the narrow
  `labonair-snippets-host::SnippetExecutionHost` contract, wired to the active
  Workspace by `bootstrap`.
- `panel-explorer` no longer declares or imports a backend facade; filesystem
  access already uses `labonair-filesystem` directly.
- `panel-git-graph` consumes the `labonair-git` contract and receives the
  `labonair-git-transport` adapter from composition. Workspace receives the
  same Git contracts by injection instead of constructing adapters internally.
- `panel-scm` and workspace Project Diff consume `labonair-git`; source-control
  execution is supplied by `labonair-git-transport`.
- `shell/src/status_items.rs` is now a composition hook only. Panel
  contribution construction has moved to panel owners. The
  `labonair-command-palette-runtime` bridge lets owner crates contribute
  executable handlers; Workspace tab, pane, focus, and project-lifecycle
  behavior has been removed from the shell table. The Terminal `Clear
  Terminal`, Settings toggles, Settings window entrypoint, Hosts management
  entrypoint, updater Check-for-Updates, Workspace panel actions, Keymap
  management entrypoint, palette toggle, Workspace search, and Hosts picker
  commands are owner-registered.
- The dedicated Jump Hosts status item was removed. Jump-host routing remains
  part of SSH connection configuration and execution, while host management and
  host selection keep their canonical menu/palette entry points.
- `shell/src/titlebar.rs` now owns only the permanent global-menu trigger and
  typed navigation events; Settings, Keymap, Themes, Icon Themes, and Hosts
  are handled by their owning surfaces through the composition root.
- The notification statusbar item has moved into `labonair-notifications`;
  Agent Access now belongs to `labonair-workspace`, and the Transfers item to
  `labonair-transfers-ui`; all permanent feature status items now expose owner
  registrations, including notification item construction, and the remaining
  shell status-item code is composition. R08-001 removed the last
  `transfers-ui → workspace` edge: the Transfers item emits only
  `TransferUiEvent::Completed` and `bootstrap` wires the SFTP pane refresh.
- The pure CWD breadcrumb path/provider helpers now belong to
  `labonair-workspace::cwd_breadcrumb`; the interactive CWD view now lives in
  `labonair-workspace::cwd_status_item` as well. Cursor Position, Preview URL,
  and Dock panel buttons (including their move/hide menu) are also Workspace-
  owned contributions. The updater dialog, state view, and statusbar badge are
  now owned by the `updater-ui` sibling; shell composition only inserts the
  typed owner registration.
- `shell/src/commands.rs` now retains only the native/debug compatibility
  behavior registry; ordinary command execution is owner-registered. Dynamic
  palette actions now cross the typed `PaletteAction` runtime registry, with
  matching and execution contributed by Workspace, Hosts-UI, Themes/Settings-UI,
  Source Control, and Snippets. Shell actions only forward opaque values.
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

The dependency verifier allows only explicit edges while these boundaries are
reviewed. Transitional edges are deliberately visible in the allow-list and
must not be treated as target architecture.

The allow-list also contains the following explicitly tracked migration
families. These are not target dependencies; each has a removal condition:

| Transitional edge family | Temporary reason | Removal condition |
|---|---|---|
| `workspace → background-host` | Workspace/Terminal render the background layer through the injected `BackgroundHost` contract; `labonair-shell` builds it from the concrete `BackgroundStore` at composition. | Retain by design (R07-005): `background-host` is a leaf contract crate that breaks the `workspace ↔ background` coupling, mirroring the Explorer host pattern. |
| `workspace → ai`, `workspace → settings` | Workspace hosts the AI live bridge and consumes typed settings values for workspace-owned behavior. | Keep orchestration in Workspace; move AI context and any remaining direct implementation access behind narrow contracts. |
| `panel-explorer → explorer-host`, `workspace → explorer-host` | Explorer's open-file/open-terminal/open-preview/active-file intents and the `DraggedPaths` / `is_previewable` value types shared with the terminal and preview views. | Retain by design (R07-004): `explorer-host` is a leaf contract crate that breaks the `panel-explorer ↔ workspace` coupling; the shell injects the Workspace-backed `ExplorerHost`. |
| `panel-snippets → snippets-host`, `shell → snippets-host` | The Snippets panel's terminal-inject / run-local / run-SSH-terminal / active-SSH-session intents. | Retain by design (R08-003): `snippets-host` is a leaf contract crate that breaks the `panel-snippets → workspace` coupling; the shell injects the Workspace-backed `SnippetExecutionHost`. |
| `panel-explorer → settings` | Explorer consumes the public typed `ExplorerSettings` value contract. | Keep only the typed public settings contract; no Settings implementation details or management UI may cross the edge. |
| `panel-scm → editor`, `panel-scm → settings` | SCM reuses the public unified-diff contract and typed SCM presentation settings. | Keep public contracts; extract only if a future shared diff/settings contract has a real second consumer. |
| `command-palette → settings`, `command-palette → filesystem` | Palette reads its typed presentation values, persists palette preferences, and owns recent-command storage. | Replace only implementation-level access with narrow contracts if the palette surface needs another host; retain capability ownership and avoid a second registry. |
| `shell composition → integration siblings` | The shell constructs concrete platform integrations so feature modules remain contract-only. | Keep construction and registration in `labonair-shell`; never expose a replacement aggregate facade. |

The verifier's allow-list is the machine-readable source for the exact edge
set. Whenever an edge is added or removed, this table and the owning task must
be updated in the same change.

The ordered implementation backlog for these edges is
[`remaining-boundaries.md`](remaining-boundaries.md). It distinguishes edges
that should be extracted from typed contracts and composition edges that are
correct by design.

No new capability may be added to `shell` or `workspace` merely because those
crates already have access to it. New code must first establish
the owning capability crate and then inject or register it at composition.
This inventory is updated when a boundary moves; it is not a license to keep a
transitional edge after its removal condition has been met.

These are migration findings, not reasons to perform a destructive rewrite. Each edge should be removed when the owning contract exists and its consumers have moved.

## Migration order

1. Introduce stable IDs, typed domain events, and narrow service traits.
2. Extract platform services and capability contracts from the former backend without changing user behavior. The filesystem service, secret store, error contract, host domain contract, and shared database lifecycle are now standalone.
3. Split notification state from presentation and replace toast rendering.
  `labonair-notifications-core` now owns the UI-free registry; the GPUI
  adapter and statusbar dropdown consume retained records.
4. Split command/keymap registries from the palette view. Command metadata,
   owner handlers, dynamic submenus, and keymap runtime are now established;
   the shell retains only documented native/debug compatibility actions.
5. Move transfers to their own module and statusbar owner. The typed registry,
   concrete worker integration, and statusbar UI are now in place; raw adapter
   transport lives in `labonair-events` while decoding remains in
   `labonair-transfers-ssh`.
6. Move hosts and SSH ownership out of Settings/workspace. The Hosts manager,
   picker, and connection entry contributions are now owner-owned.
7. Move terminal/editor/SFTP views to their owning modules. Remaining view
   extraction and visual acceptance are tracked in R07.
8. Remove compatibility edges and enforce the target graph; the remaining
   documented compatibility adapters are reviewed by the R07 acceptance gate.

## Evidence commands

The inventory was produced from:

```text
cargo metadata --no-deps --format-version 1
test ! -e crates/backend
rg -n "pub struct|pub enum|pub trait|pub fn" crates/shell/src crates/workspace/src crates/*/src
```
