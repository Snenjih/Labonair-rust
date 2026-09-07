# Backend Facade Inventory

**Status:** R06-001 working inventory  
**Date:** 2026-09-07  
**Evidence:** the former `crates/backend/` tree, current Cargo metadata, and
source search for `labonair_backend` / `labonair-backend`.

This is a current-state inventory, not an ownership declaration. The target
ownership is defined by [`../architecture.md`](../architecture.md) and
[`../capabilities.md`](../capabilities.md). The former backend package has now
been removed; concrete platform integrations live in named sibling crates.

## Former public facade surface

| Surface | Current location | External consumers | Target boundary | R06 state |
|---|---|---|---|---|
| `AppComposition` | `shell::composition` | `app`, `shell` composition | application composition plus injected capability services | owns construction, worker startup, and capability extraction; no feature module receives a broad state facade |
| `EventBus`, `EventChannel`, `RawEvent` | `labonair-events` | shell composition and named integration adapters | UI-free adapter transport; typed capability events remain in their owning contracts | moved out of the former backend; the transport crate owns no product state |
| updater constants and operations | `labonair-updater` | `shell::updater`, app smoke tests | updater capability plus shell UI | Moved out of the backend; the shell consumes the dedicated capability crate |
| structured errors | formerly `backend::modules::errors` and root re-exports | no external backend import remains | `labonair-errors` | Root re-export and module removed in the first R06 slice |

## Former module export and consumer map

This table records the former module boundaries and their final disposition. It
is retained as removal evidence, not as a current package structure.

| Backend module | Public surface | Current external consumers | Intended owner / disposition |
|---|---|---|---|
| `agents` | removed | no active consumers | removed as an unreferenced backend copy; the AI owner must add a canonical contract before this capability is reintroduced |
| `credentials` | removed | no active backend consumers | `labonair-credentials` is the canonical owner; active Hosts UI callers use it directly |
| `directives` | removed | no active consumers | removed as an unreferenced backend copy; reintroduce only behind an AI-owned contract when a real UI/workflow consumer exists |
| `fonts` | custom-font file operations and system-font discovery | `shell::settings_services` | system-font discovery moved to `labonair-theme`; the unconsumed custom-font path and backend module were removed |
| `fs` | removed | no active consumers | filesystem foundation owns paths, operations, and watchers; backend compatibility module and dead watcher adapter removed |
| `git` | removed from backend | none | `labonair-git-transport` owns `GitTransportService` / graph adapter and Git operation functions; services receive only SSH state plus EventBus |
| `mcp` | removed from backend | none | `labonair-mcp-server` owns concrete MCP state, grants, server operations, host revocation, and contract/event adapters; shell composition supplies explicit capabilities |
| `model_prefs` | removed | no active consumers | removed as an unreferenced backend copy; model selection state remains an AI-owned concern when its UI contract is defined |
| `pty` | local PTY state, sessions, events, I/O operations | indirect through backend/MCP | terminal owner; expose a terminal service rather than `App` state |
| `scrollback` | scrollback persistence helpers | `shell`, `workspace` | moved to `labonair-terminal::scrollback`; Workspace supplies session/retention context |
| `secrets` | removed | no backend compatibility consumers | `labonair-secrets` is the canonical owner; SSH/SFTP/MCP adapters and shell composition use its `SecretsState` and operations directly |
| `settings` | removed | no active backend consumers | `labonair-settings` owns the one-time `legacy_migrations` boundary; SettingsContent remains the canonical runtime value model |
| `sftp` | removed from backend | none | Concrete SFTP session and contract adapters now live in `labonair-sftp-ssh`; transfer execution lives in `labonair-transfers-ssh` |
| `shell` | removed | no active consumers outside its own tests | No canonical runtime consumer existed; future local command/background-process capability must be introduced through its owning Terminal/AI contract rather than another backend module |
| `snippets` | SSH executor adapter | `shell` | `labonair-snippets` owns the store and execution contracts; `labonair-snippets-ssh` owns concrete SSH execution and receives explicit SSH state and EventBus capabilities |
| `ssh-transport` | SSH state, russh transport, PTY, remote files, tunnels, config import/export | `shell`; internal Git/SFTP/snippet/MCP use | `labonair-ssh-transport` integration sibling; it implements the UI-free `labonair-ssh` contracts and owns the concrete session registry and transport state |
| `terminal_exec` | removed | no active consumers; MCP owns its live terminal execution path | dead compatibility module and `App` state removed; MCP server remains the active owner |
| `themes` | removed | no active backend consumers | `labonair-theme` owns the static theme and icon-theme registries; network download is intentionally not part of the current product surface |
| `transfers` | `TransferServiceAdapter` and event source | `shell` | `labonair-transfers` owns the contracts and registry; `labonair-transfers-ssh` owns the concrete worker and adapters, with only explicit worker state and EventBus inputs |
| `updater` | moved out of backend | `shell::updater`, app smoke tests | `labonair-updater` owns manifest parsing, version checks, verification, download/install helpers, and check cadence; shell owns only the GPUI view |

## Direct dependency evidence

No current workspace crate declares `labonair-backend`. The final direct
dependency audit is:

| Crate | Why it currently imports backend | Removal seam |
|---|---|---|
| `labonair` | invokes shell-owned composition and startup hooks | no backend dependency; the app entrypoint remains thin |
| `labonair-shell` | constructs `AppComposition` and named integrations | composition-only adapter imports; feature state is injected through public contracts |
| `labonair-ai` | no backend usage | no backend dependency |

`settings`, `settings-content`, and related crates contain historical migration
references to the predecessor wire model, but they do not declare or import a
backend crate. The former backend Settings module is
gone; `labonair_settings::legacy_migrations` is the only owner of the legacy
pre-v2 wire migration path.

`labonair-workspace` is also no longer a direct backend consumer. It owns the
typed SSH/MCP event bridges, status/panel placement persistence, and MCP tab
orchestration while receiving all transport and event sources through injected
capability contracts.

## First completed boundary

The error contract no longer passes through a facade. Integration code imports
`labonair-errors` directly, while `backend::modules::errors`, the root error
re-exports, and the unused `AppResult` alias are gone.

The live `statusBarItemPlacements` and `panelToggleVisibility` blobs now have
their persistence implementation in `labonair-workspace::status_placements`.
The former backend Settings module is removed. The shared JSON helpers and
legacy value migrator now live together in
`labonair-settings::legacy_migrations`; they are not part of the runtime
settings value API and do not provide a second persistence path.

Terminal scrollback persistence likewise lives in
`labonair-terminal::scrollback`. The backend no longer exports a scrollback
module; Workspace calls the terminal capability for save/load/delete/cleanup,
and shell shutdown uses the same capability directly.

The stable MCP session/grant values, tab-operation result, and service
contracts now live in `labonair-mcp-core`. `AgentAccessStore` and Workspace
consume only injected contracts and no longer call MCP implementation
functions directly. The concrete server and its adapters are constructed by
shell composition from `labonair-mcp-server`. The raw adapter event transport
lives in `labonair-events`; integration adapters translate only their own
events into typed capability contracts. Workspace receives typed
`SshConnectionEvent` and `McpEvent` values through injected sources and has no
direct backend dependency.

The dead backend filesystem watcher and `App::watcher` state were removed after
source search confirmed that active Explorer and Settings consumers already use
`labonair-filesystem` directly. Backend feature modules now import the canonical
filesystem paths directly, so there is no remaining `backend::modules::fs`
compatibility layer.

The MCP grant adapter was narrowed as well: `McpSessionAccessAdapter` receives
only `McpState` and the shared `Database`, while host-block revocation receives
`McpState` plus `EventBus`. The MCP server's token, listener, and terminal-tool
integration now receives an explicit `McpServerAccess` bundle containing only
SSH state, local PTY state, database, secrets, and EventBus. The server and its
control functions now live in `labonair-mcp-server` and no longer retain or
accept the aggregate `App`.

The Git operation surface was narrowed next. All Git operation functions now
receive an explicit `EventBus` rather than the aggregate `App`, and both
transport services retain only `SshState` plus `EventBus`. Their constructors
remain composition-only extraction points while the remaining MCP and snippet
adapters are migrated.

The SSH contract adapters are split by responsibility inside
`labonair-ssh-transport`: PTY write/resize use
`BackendSshPtyService` with only `SshState`, remote command/file operations
use `BackendSshRemoteService` with `SshState + EventBus`, and connection,
tester, config, and tunnel contracts each have their own named adapter. The
connection adapter owns the explicit database/secrets/trust/session inputs;
the tester owns its read-only connection inputs, config owns the host database,
and tunnels own tunnel state plus their connection inputs. No SSH contract is
backed by the former aggregate `BackendSshService` anymore.

The lower-level SSH connection pipeline was narrowed independently. Transport
setup, host-key verification, authentication, jump-host handshakes, and PTY
reader disconnect reporting now receive only `EventBus` for their event/log
side effects. Database, secret, trust, and session-state access remains in the
composition-facing connection adapter until those responsibilities are split
into their own capability services.

The standalone Secrets compatibility wrappers were narrowed as well. Secret
reads/writes, encryption access, and service-name migration now receive only
`SecretsState`; the SSH/SFTP/MCP callers no longer pass `App` merely to
reach the secret store. Jump-host resolution consequently depends only on its
database and secrets inputs.

The aggregate `App::emit` convenience method was removed after the SSH logging
macro was switched to its already-injected `EventBus`. Event producers now use
the explicit event capability directly; the application root remains a state
and composition container rather than an event facade.

## Verification commands

```text
rg -n "labonair_backend|labonair-backend|BackendComposition|crate::backend|backend::" crates --glob '*.rs' --glob 'Cargo.toml'
test ! -e crates/backend
cargo metadata --no-deps --format-version 1
scripts/check-crate-deps.sh
```
