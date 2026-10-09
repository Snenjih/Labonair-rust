# Capability Matrix

**Status:** Normative ownership map
**Version:** 6
**Related:** [`architecture.md`](architecture.md), [`modules.md`](modules.md), [`registries.md`](registries.md)

This matrix is the authoritative index for product capabilities. A capability
has one owning module, one canonical capability crate, one canonical user
entry point, and one composition boundary. A module may have sibling crates
for real UI, storage, or integration boundaries, but they remain under the
same owner. The `Current implementation` column is deliberately separate from
the target columns so that documentation does not turn an unfinished
migration into an architectural claim.

The application-composition and foundation rows make non-product boundaries
visible for dependency auditing. The one-capability/one-capability-crate rule
applies to product rows; composition and foundation crates have one explicit
coordination or reusable-service responsibility instead.

The dispositions below describe existing implementation and earlier product
decisions. Under the current parity target, they are crosswalk inputs, not
exclusions for capabilities present in the pinned Zed baseline. R07-000 must
reconcile them before any later implementation task starts.

| Capability | Disposition | Owning module | Contract / domain crate | UI crate | Storage / integration | Canonical entry point | Current implementation state |
|---|---|---|---|---|---|---|---|
| Application composition | Keep / simplify | `application` | `labonair-shell` | `labonair-shell` | named integration siblings composed by `labonair-shell` | App startup | Composition root owns construction and registration only |
| Workspace orchestration | Redesign | `workspace` | `labonair-workspace` | `labonair-workspace` | session/layout persistence | Workspace surface (titlebar tab strip, or the Tabs dock panel when `tabsLocation == "sidebar"`) | Typed Empty/Standalone/Project state and WorkspaceEvent boundary; explicit project picker; persisted project identity with legacy Standalone fallback; explicit return-to-standalone command; feature-view extraction remains ongoing |
| Backgrounds | Keep / isolate | `backgrounds` | `labonair-background`, `labonair-background-host` | `labonair-background` | local image storage and decoded cache | Appearance/background surface | Capability extracted from workspace; Settings UI no longer holds its entity; Workspace/Terminal render through the injected `labonair-background-host::BackgroundHost` contract instead of the `workspace → background` dependency (R07-005) |
| Terminal | Keep / isolate | `terminal` | `labonair-terminal` | `labonair-terminal` and workspace integration | PTY/process adapter | Terminal tab or standalone terminal | Active, extraction ongoing |
| Editor | Keep / isolate | `editor` | `labonair-editor` | `labonair-editor` and workspace integration | filesystem adapter plus Editor-owned local language-service runtime/process boundary | Editor tab or standalone editor | Active, extraction ongoing; syntax-first editor now has an injectable, opt-in local LSP runtime with revision-safe document sync and fallback |
| Filesystem | Keep | `filesystem` | `labonair-filesystem` | Consuming feature UI | local filesystem and watcher | Explorer/editor consumers | Extracted |
| SSH | Keep / redesign entry points | `ssh` | `labonair-ssh` | SSH connection surface | `labonair-ssh-transport` owns russh, auth, tunnels, jump hosts | Host action or standalone SSH | Contract and injected workspace/Hosts paths migrated; concrete transport is isolated from the composition root |
| SFTP | Keep / isolate | `sftp` | `labonair-sftp` | SFTP browser | `labonair-sftp-ssh` owns russh-sftp session adapters | Host action or standalone SFTP | Authenticated session/browser contract and view migration complete; transfer queue remains separate. Browser chrome (pane label + item count, address/search/refresh/hidden toolbar, reorderable metadata column headers, zebra rows, `..` row, directional drop highlight) is view-local, driven by the `fileManager` `Sftp*` settings via a `SettingsStore` observer. The `>_ Term` action emits `SftpEvent::OpenRemoteTerminal`, opening an SSH terminal tab through the workspace. |
| Hosts | Redesign | `hosts` | `labonair-hosts`, `labonair-hosts-host` | `labonair-hosts-ui` | `labonair-persistence`, credentials, SSH/SFTP integration siblings | Titlebar global menu / Command Palette → the workspace `Hosts` tab (occasional-use, never a startup tab, mirrors the Keymap tab) | Canonical SQLite-backed manager is composed once by the shell; Hosts owns rendering, picker snapshots, recent ordering, and typed SSH/SFTP requests, while `Workspace` holds `Entity<HostManagerView>` directly and owns the tab lifecycle (supersedes R08-012's narrow-contract-only rule; the `labonair-hosts-host::HostView` contract still serves Workspace's continuous host-data needs — tunnel status, picker rows — independent of tab visibility). Settings has no host projection. |
| Credentials | Keep / isolate | `credentials` | `labonair-credentials` | Hosts UI consumer | `labonair-secrets`, keychain | Host management | Extracted |
| Transfers | Redesign | `transfers` | `labonair-transfers` | `labonair-transfers-ui` | `labonair-transfers-ssh` owns the SFTP worker and adapters | Statusbar Transfers badge | Typed registry, worker, and statusbar UI are isolated; shell only injects the concrete integration |
| Git / source control | Keep / isolate | `git` | `labonair-git` | `labonair-panel-scm`, `labonair-panel-git-graph`, Project Diff | `labonair-git-transport` owns local/remote Git execution and adapters | Source Control panel / palette | Contracts and concrete integration are isolated |
| Explorer | Redesign | `explorer` | `labonair-panel-explorer`, `labonair-explorer-host` | `labonair-panel-explorer` | `labonair-filesystem`; shell-injected SSH/SFTP adapter | Explorer dock panel | Canonical Explorer capability/UI crate; `workspace`, `ssh`, and `sftp` implementation edges are removed (R07-004). Open-file/terminal/preview/active-file intents and SSH-backed directory reads cross the injected `labonair-explorer-host` contracts; concrete remote services remain in shell composition. |
| Snippets | Keep / isolate | `snippets` | `labonair-snippets`, `labonair-snippets-host` | `labonair-panel-snippets` | persistence and injected SSH executor | Snippets panel / palette | Contracts and panel isolated; the `workspace` dependency is removed (R08-003) — inject/run/SSH-session intents cross the injected `labonair-snippets-host::SnippetExecutionHost` contract |
| Settings | Redesign / reference parity | `settings` | `labonair-settings` | `labonair-settings-ui` | `labonair-settings-content`, `labonair-settings-json`, `labonair-settings-macros` | Settings window | Existing value model remains current state; category, field, navigation, and control parity is specified by R07-000 |
| Keymap | Redesign | `keymap` | `labonair-keymap` | `labonair-keymap-ui` | keymap file and binding registry | Titlebar global menu → Keymap; quick access through palette | UI-free keymap crate is active; `labonair-keymap-ui`'s `KeymapManagementView` renders as a workspace `Keymap` tab (`Workspace::open_keymap_tab`), not an OS window; shell retains only platform key installation/watch wiring. The tab groups commands by section, filters by All/Modified/Conflicts/Unbound, is keyboard-navigable, records keystrokes instead of requiring hand-typed chords (`keymap-ui`'s `keystroke` module + `App::intercept_keystrokes`), edits in an anchored popover with a context picker for new bindings, "Reset to default" (`file::remove_user_binding_override`), surfaces `keymap.json` diagnostics as an in-tab banner, and live-reloads on external file changes (`KeymapManagementView::reload`, wired to `settings::watch_file` by the workspace). Conflict/override state is derived data on `management` (`OverrideState`, `conflict_commands`, `shadowed_defaults`). |
| Command Palette | Redesign | `command-palette` | `labonair-command-palette-core` | `labonair-command-palette` | typed providers and `PaletteActionHandlerRegistry` contributions from feature modules | Titlebar global menu / global shortcut | Metadata, ordinary command handlers, and dynamic submenu actions are owner-contributed; shell only composes and forwards actions |
| Notifications | Redesign | `notifications` | `labonair-notifications-core` | `labonair-notifications` | none; retained in registry | Statusbar notification dropdown | Registry and statusbar presentation are capability-owned; migrated operation errors use structured details/source/deduplication; no toast surface |
| Themes (color and icon) | Redesign / reference parity | `themes` | `labonair-theme`, `labonair-theme-ui` | `labonair-theme` (runtime + catalogs); `labonair-theme-ui` (settings→store policy + palette handler) | extension and distribution contracts pending R07-000 | Titlebar global menu → Themes / Icon Themes → palette submenu | Existing app-theme and icon-theme registries remain current state; extension, import, discovery, and management behavior is crosswalked against the pinned baseline |
| Updates | Keep / isolate | `updater` | `labonair-updater` | `labonair-updater-ui` | release manifest, signed artifact verification and macOS installation | Settings/global menu update action | Capability logic, dialog, state view, status item, and command contribution are isolated from the shell UI |
| AI | Keep / reference parity | `ai` | `labonair-ai` | `labonair-ai` until a real UI boundary requires a sibling crate | provider/session persistence; MCP integration in `labonair-mcp-server` | AI panel / workspace context | Core retained; UI and service rework are open and must be crosswalked against the pinned baseline |

## Explicit product dispositions

These items are deliberately not capability rows because they are surfaces,
implementation strategies, or predecessor behaviors rather than independent
product ownership boundaries. Earlier keep/defer/remove decisions are subject
to the pinned-reference crosswalk; only independent architecture rules remain
binding without that review:

| Item | Decision | Consequence |
|---|---|---|
| Settings categories for Hosts, Themes, Icon Themes, and Shortcuts | Remove as management surfaces | Settings stores values only; each capability owns its management UI. |
| Toast notifications | Remove | Passive messages are retained and displayed only by the notification registry and statusbar dropdown. |
| Duplicate feature-local operation-error banners | Remove | Operation failures publish notifications; actionable dialogs and field validation remain only where a decision or correction is required. |
| Jump-host primary menu/badge | Remove as a separate surface; keep the capability | Jump hosts remain part of SSH connection configuration and execution; no dedicated statusbar item is registered. |
| Remote theme/icon-theme downloads | Crosswalk against the pinned baseline | Extension and theme distribution workflows are in the parity inventory; the owner, trust model, and service boundary are defined before implementation. |
| User theme-file import/export (local `.json`) | Crosswalk against the pinned baseline | Existing code remains dormant until the parity inventory decides whether the pinned reference has an equivalent workflow and which owner exposes it. |
| Static shell-wide command tables | Remove | Commands and submenus are contributed by owning modules through the command registry. |
| Full Zed feature and UI/UX parity | Adopt as a clean-room target, not a source fork | Keep an independent native Rust/GPUI implementation and use the fixed reference and [`zed-parity.md`](zed-parity.md) contracts to drive the complete capability inventory. |

## Rules for changing this matrix

1. Add a row before adding a new product capability.
2. Record exactly one owning module and one canonical capability crate per
   row. Sibling crates must be named in the same row and must not claim
   ownership.
3. A new crate must have one responsibility, a current consumer or independent
   test boundary, and a real dependency, lifecycle, storage, or platform
   reason. Visual primitives belong in `labonair-ui-kit` instead.
4. Record the canonical entry point before implementation. Alternate entry
   points may delegate to it but must not duplicate behavior.
5. Update the owning module's task, this matrix, the inventory, and the
   dependency verifier in the same change as a new internal edge or ownership
   move.
6. Do not delete a row because a feature is temporarily parked. Mark its
   disposition and migration state instead; remove it only when the product
   decision is explicitly `remove` and its data/command migration is complete.
