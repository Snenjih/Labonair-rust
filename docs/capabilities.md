# Capability Matrix

**Status:** Normative ownership map
**Version:** 3
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

| Capability | Disposition | Owning module | Contract / domain crate | UI crate | Storage / integration | Canonical entry point | Current implementation state |
|---|---|---|---|---|---|---|---|
| Application composition | Keep / simplify | `application` | `labonair-shell` | `labonair-shell` | named integration siblings composed by `labonair-shell` | App startup | Composition root owns construction and registration only |
| Workspace orchestration | Redesign | `workspace` | `labonair-workspace` | `labonair-workspace` | session/layout persistence | Workspace surface | Typed Empty/Standalone/Project state and WorkspaceEvent boundary; explicit project picker; persisted project identity with legacy Standalone fallback; explicit return-to-standalone command; feature-view extraction remains ongoing |
| Backgrounds | Keep / isolate | `backgrounds` | `labonair-background` | `labonair-background` | local image storage and decoded cache | Appearance/background surface | Capability extracted from workspace; Settings UI no longer holds its entity |
| Terminal | Keep / isolate | `terminal` | `labonair-terminal` | `labonair-terminal` and workspace integration | PTY/process adapter | Terminal tab or standalone terminal | Active, extraction ongoing |
| Editor | Keep / isolate | `editor` | `labonair-editor` | `labonair-editor` and workspace integration | filesystem adapter | Editor tab or standalone editor | Active, extraction ongoing |
| Filesystem | Keep | `filesystem` | `labonair-filesystem` | Consuming feature UI | local filesystem and watcher | Explorer/editor consumers | Extracted |
| SSH | Keep / redesign entry points | `ssh` | `labonair-ssh` | SSH connection surface | `labonair-ssh-transport` owns russh, auth, tunnels, jump hosts | Host action or standalone SSH | Contract and injected workspace/Hosts paths migrated; concrete transport is isolated from the composition root |
| SFTP | Keep / isolate | `sftp` | `labonair-sftp` | SFTP browser | `labonair-sftp-ssh` owns russh-sftp session adapters | Host action or standalone SFTP | Authenticated session/browser contract and view migration complete; transfer queue remains separate |
| Hosts | Redesign | `hosts` | `labonair-hosts` | `labonair-hosts-ui` | `labonair-persistence`, credentials, SSH/SFTP integration siblings | Titlebar global menu → Hosts; picker in Command Palette | Canonical SQLite-backed manager is composed once by the shell; Hosts owns the native management window, picker snapshots, recent ordering, and typed SSH/SFTP requests. Settings has no host projection. |
| Credentials | Keep / isolate | `credentials` | `labonair-credentials` | Hosts UI consumer | `labonair-secrets`, keychain | Host management | Extracted |
| Transfers | Redesign | `transfers` | `labonair-transfers` | `labonair-transfers-ui` | `labonair-transfers-ssh` owns the SFTP worker and adapters | Statusbar Transfers badge | Typed registry, worker, and statusbar UI are isolated; shell only injects the concrete integration |
| Git / source control | Keep / isolate | `git` | `labonair-git` | `labonair-panel-scm`, `labonair-panel-git-graph`, Project Diff | `labonair-git-transport` owns local/remote Git execution and adapters | Source Control panel / palette | Contracts and concrete integration are isolated |
| Explorer | Redesign | `explorer` | Filesystem contracts | `labonair-panel-explorer` | `labonair-filesystem` | Explorer dock panel | Backend edge removed; workspace shim remains |
| Snippets | Keep / isolate | `snippets` | `labonair-snippets` | `labonair-panel-snippets` | persistence and injected SSH executor | Snippets panel / palette | Contracts and panel isolated |
| Settings | Redesign / reduce | `settings` | `labonair-settings-content`, `labonair-settings` | `labonair-settings-ui` | JSON/settings persistence | Settings window | Value-only navigation; legacy capability sections are migration-only and diagnostics use Notifications |
| Keymap | Redesign | `keymap` | `labonair-keymap` | dedicated keymap surface | keymap file and binding registry | Titlebar global menu → Keymap; quick access through palette | UI-free keymap crate extracted; editing and shell adapter migration ongoing |
| Command Palette | Redesign | `command-palette` | `labonair-command-palette-core` | `labonair-command-palette` | typed providers from feature modules | Titlebar global menu / global shortcut | UI-free registry crate introduced; global-menu navigation is typed; static entries and duplicate shell registry still being migrated |
| Notifications | Redesign | `notifications` | `labonair-notifications-core` | `labonair-notifications` | none; retained in registry | Statusbar notification dropdown | Registry and statusbar presentation are capability-owned; migrated operation errors use structured details/source/deduplication; no toast surface |
| Themes (color and icon) | Redesign / static first | `themes` | `labonair-theme` | `labonair-theme` unless a real UI boundary requires a sibling crate | built-in definitions first | Titlebar global menu → Themes / Icon Themes → palette submenu | Separate app-theme and icon-theme palette pages; both support registry-backed selection and transient preview; download/extension workflow remains deferred |
| Updates | Keep / isolate | `updater` | `labonair-updater` | shell updater view | release manifest, signed artifact verification and macOS installation | Settings/global menu update action | Capability logic is isolated from the shell UI; update presentation remains shell-owned |
| AI | Defer UI / keep core | `ai` | `labonair-ai` | `labonair-ai` until a real UI boundary requires a sibling crate | provider/session persistence; MCP integration in `labonair-mcp-server` | AI panel / workspace context | Core retained; UI rework open and no aggregate backend dependency |

## Explicit product dispositions

These items are deliberately not capability rows because they are surfaces,
implementation strategies, or predecessor behaviors rather than independent
product ownership boundaries:

| Item | Decision | Consequence |
|---|---|---|
| Settings categories for Hosts, Themes, Icon Themes, and Shortcuts | Remove as management surfaces | Settings stores values only; each capability owns its management UI. |
| Toast notifications | Remove | Passive messages are retained and displayed only by the notification registry and statusbar dropdown. |
| Duplicate feature-local operation-error banners | Remove | Operation failures publish notifications; actionable dialogs and field validation remain only where a decision or correction is required. |
| Jump-host primary menu/badge | Remove as a separate surface; keep the capability | Jump hosts remain part of SSH connection configuration and execution. |
| Remote theme/icon-theme downloads | Defer | Only built-in, explicitly registered themes are supported until an extension workflow has a concrete owner and user flow. |
| Static shell-wide command tables | Remove | Commands and submenus are contributed by owning modules through the command registry. |
| Full Zed fork or greenfield rewrite | Reject for the current migration | Continue the standalone Rust implementation and use Zed only as a clean-room reference. |

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
