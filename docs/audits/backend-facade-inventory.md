# Backend Facade Inventory

**Status:** R06-001 working inventory  
**Date:** 2026-09-07  
**Evidence:** `crates/backend/src/lib.rs`, `events.rs`, every file under
`crates/backend/src/modules/`, Cargo metadata, and source search for
`labonair_backend` / `labonair-backend`.

This is a current-state inventory, not an ownership declaration. The target
ownership is defined by [`../architecture.md`](../architecture.md) and
[`../capabilities.md`](../capabilities.md). A backend entry may remain only as
a concrete platform adapter with a named consumer and removal condition.

## Public facade surface

| Surface | Current location | External consumers | Target boundary | R06 state |
|---|---|---|---|---|
| `BackendComposition` | `shell::backend` | `app`, `shell` composition | application composition plus injected capability services | owns construction, worker startup, and capability extraction; no feature module receives a broad state facade |
| `EventBus`, `EventChannel`, `RawEvent` | `backend::events` | `app`, backend transport adapters | typed capability events and explicit transport adapters | global bus remains only as a raw internal adapter source; event-source adapters receive `EventBus` directly and no longer retain `App` |
| updater constants and operations | `labonair-updater` | `shell::updater`, app smoke tests | updater capability plus shell UI | Moved out of the backend; the shell consumes the dedicated capability crate |
| structured errors | formerly `backend::modules::errors` and root re-exports | no external backend import remains | `labonair-errors` | Root re-export and module removed in the first R06 slice |

## Module export and consumer map

The export count is based on public declarations in the module source. “Internal
only” means no non-backend crate currently imports that module; it still may
participate in the `BackendComposition` state graph or in another backend module.

| Backend module | Public surface | Current external consumers | Intended owner / disposition |
|---|---|---|---|
| `agents` | removed | no active consumers | removed as an unreferenced backend copy; the AI owner must add a canonical contract before this capability is reintroduced |
| `credentials` | removed | no active backend consumers | `labonair-credentials` is the canonical owner; active Hosts UI callers use it directly |
| `directives` | removed | no active consumers | removed as an unreferenced backend copy; reintroduce only behind an AI-owned contract when a real UI/workflow consumer exists |
| `fonts` | custom-font file operations and system-font discovery | `shell::settings_services` | system-font discovery moved to `labonair-theme`; the unconsumed custom-font path and backend module were removed |
| `fs` | removed | no active consumers | filesystem foundation owns paths, operations, and watchers; backend compatibility module and dead watcher adapter removed |
| `git` | Git operation functions and `BackendGitService` / graph adapter | `shell::bootstrap`, `workspace` | `labonair-git` integration adapter; service and operation functions now receive only SSH state plus EventBus |
| `mcp` | MCP state, grants, server operations, host revocation callback | `shell`, `workspace`; internal PTY/secrets use | `labonair-mcp-core` owns UI-free grant and tab-operation contracts; grant/revoke adapter receives only MCP state/database/events, while server state is supplied by shell composition |
| `model_prefs` | removed | no active consumers | removed as an unreferenced backend copy; model selection state remains an AI-owned concern when its UI contract is defined |
| `pty` | local PTY state, sessions, events, I/O operations | indirect through backend/MCP | terminal owner; expose a terminal service rather than `App` state |
| `scrollback` | scrollback persistence helpers | `shell`, `workspace` | moved to `labonair-terminal::scrollback`; Workspace supplies session/retention context |
| `secrets` | secret-state compatibility API | internal SSH/SFTP/MCP adapters | `labonair-secrets`; compatibility wrappers now accept only `SecretsState`, with no aggregate `App` parameter |
| `settings` | removed | no active backend consumers | `labonair-settings` owns the one-time `legacy_migrations` boundary; SettingsContent remains the canonical runtime value model |
| `sftp` | session adapter, remote operations, transfer worker state/commands | `shell`; internal SSH/transfer adapters | `labonair-sftp` and `labonair-transfers` integration boundaries; SFTP service now receives only SSH state plus the raw event bus, and legacy connection orchestration receives EventBus instead of App |
| `shell` | removed | no active consumers outside its own tests | No canonical runtime consumer existed; future local command/background-process capability must be introduced through its owning Terminal/AI contract rather than another backend module |
| `snippets` | snippet DB compatibility and SSH executor adapter | `shell`; internal backend use | `labonair-snippets` integration boundary; SSH execution now receives explicit SSH state and EventBus capabilities |
| `ssh` | SSH state, transport, PTY, remote files, tunnels, config import/export | `shell`; internal Git/SFTP/snippet/MCP use | `labonair-ssh` integration boundary; PTY and remote-file adapters now receive only their required capability state, while connection/config/tunnel extraction remains |
| `terminal_exec` | removed | no active consumers; MCP owns its live terminal execution path | dead compatibility module and `App` state removed; MCP server remains the active owner |
| `themes` | removed | no active backend consumers | `labonair-theme` owns the static theme and icon-theme registries; network download is intentionally not part of the current product surface |
| `transfers` | `BackendTransferService` and event source | `shell` | `labonair-transfers` integration boundary; service receives only `TransferWorkerState`, event translation stays once at adapter edge, and the worker receives explicit SSH/EventBus/queue state |
| `updater` | moved out of backend | `shell::updater`, app smoke tests | `labonair-updater` owns manifest parsing, version checks, verification, download/install helpers, and check cadence; shell owns only the GPUI view |

## Direct dependency evidence

Cargo currently declares `labonair-backend` directly in these non-backend
crates:

| Crate | Why it currently imports backend | Removal seam |
|---|---|---|
| `labonair` | invokes shell-owned composition and startup hooks | no direct runtime backend dependency; terminal/theme edges are smoke-test-only dev dependencies and the app entrypoint remains thin |
| `labonair-shell` | constructs `BackendComposition`, builds SSH/SFTP/Git/transfer adapters, reads MCP/settings/updater compatibility APIs | one composition-only adapter import per capability, with no feature state access; Git adapters receive explicit SSH/EventBus capabilities |
| `labonair-ai` | no active backend usage; stale dependency declaration | removed in the R06 inventory pass |

`settings`, `settings-content`, and related crates contain historical comments
or migration references to backend names, but they do not declare or import the
backend crate as a runtime dependency. The former backend Settings module is
gone; `labonair_settings::legacy_migrations` is the only owner of the legacy
pre-v2 wire migration path.

`labonair-workspace` is also no longer a direct backend consumer. It owns the
typed SSH/MCP event bridges, status/panel placement persistence, and MCP tab
orchestration while receiving all transport and event sources through injected
capability contracts.

## First completed boundary

The error contract no longer passes through the facade. Backend transport code
imports `labonair-errors` directly, while `backend::modules::errors`, the root
error re-exports, and the unused `AppResult` alias are gone. This establishes
the migration pattern for the remaining entries: move or consume the canonical
contract first, then delete the backend compatibility path once source search
and focused tests prove that no external consumer remains.

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
functions directly. The backend adapter is constructed by shell composition
and remains the explicit bridge to aggregate MCP implementation state. The
legacy global event bus stays inside shell-composed adapters; Workspace
receives typed `SshConnectionEvent` and `McpEvent` values through injected
sources and has no direct backend dependency.

The dead backend filesystem watcher and `App::watcher` state were removed after
source search confirmed that active Explorer and Settings consumers already use
`labonair-filesystem` directly. Backend feature modules now import the canonical
filesystem paths directly, so there is no remaining `backend::modules::fs`
compatibility layer.

The MCP grant adapter was narrowed as well: `BackendMcpSessionAccess` receives
only `McpState` and the shared `Database`, while host-block revocation receives
`McpState` plus `EventBus`. The MCP server's token, listener, and terminal-tool
integration now receives an explicit `McpServerAccess` bundle containing only
SSH state, local PTY state, database, secrets, and EventBus. The server and its
control functions no longer retain or accept the aggregate `App`.

The Git operation surface was narrowed next. All Git operation functions now
receive an explicit `EventBus` rather than the aggregate `App`, and both
`BackendGitService` and `BackendGitGraphService` retain only `SshState`
plus `EventBus`. Their constructors remain composition-only extraction
points while the remaining SSH and MCP server adapters are migrated.

The SSH contract adapters are split by responsibility: PTY write/resize use
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
rg -n "labonair_backend|labonair-backend|backend::" crates --glob '*.rs' --glob 'Cargo.toml'
find crates/backend/src/modules -maxdepth 2 -type f | sort
cargo metadata --no-deps --format-version 1
scripts/check-crate-deps.sh
```
