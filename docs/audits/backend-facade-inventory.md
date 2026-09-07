# Backend Facade Inventory

**Status:** R06-001 working inventory  
**Date:** 2026-09-07  
**Evidence:** `crates/backend/src/lib.rs`, `app.rs`, `events.rs`, every file under
`crates/backend/src/modules/`, Cargo metadata, and source search for
`labonair_backend` / `labonair-backend`.

This is a current-state inventory, not an ownership declaration. The target
ownership is defined by [`../architecture.md`](../architecture.md) and
[`../capabilities.md`](../capabilities.md). A backend entry may remain only as
a concrete platform adapter with a named consumer and removal condition.

## Public facade surface

| Surface | Current location | External consumers | Target boundary | R06 state |
|---|---|---|---|---|
| `App` / `AppState` and `AppInner` | `backend::app` | `app`, `shell`, backend adapters | application composition plus injected capability services | Broad facade remains; split is the main task |
| `EventBus`, `EventChannel`, `RawEvent` | `backend::events` | `app`, backend transport adapters | typed capability events and explicit transport adapters | global bus remains only as a raw internal adapter source; event-source adapters receive `EventBus` directly and no longer retain `App` |
| updater constants and operations | `backend::modules::updater` | `shell::updater`, app smoke tests | updater/application boundary | Root re-export removed; consumers use the updater module directly |
| structured errors | formerly `backend::modules::errors` and root re-exports | no external backend import remains | `labonair-errors` | Root re-export and module removed in the first R06 slice |

## Module export and consumer map

The export count is based on public declarations in the module source. “Internal
only” means no non-backend crate currently imports that module; it still may
participate in the `App` state graph or in another backend module.

| Backend module | Public surface | Current external consumers | Intended owner / disposition |
|---|---|---|---|
| `agents` | agent values, built-in/load/save helpers | none found | AI/agent owner; verify whether the current implementation is still required |
| `credentials` | credential CRUD adapters and credential values | none found | credentials owner; callers should use `labonair-credentials` contracts |
| `directives` | directive values and local persistence helpers | none found | AI/agent owner; remove if no current workflow needs it |
| `fonts` | custom-font file operations and system-font discovery | `shell::settings_services` | system-font discovery moved to `labonair-theme`; the unconsumed custom-font path and backend module were removed |
| `fs` | removed | no active consumers | filesystem foundation owns paths, operations, and watchers; backend compatibility module and dead watcher adapter removed |
| `git` | Git operation functions and `BackendGitService` / graph adapter | `shell::bootstrap`, `workspace` | `labonair-git` integration adapter; keep transport implementation narrow |
| `mcp` | MCP state, grants, server operations, host revocation callback | `shell`, `workspace`; internal PTY/secrets use | `labonair-mcp-core` owns UI-free grant and tab-operation contracts; grant/revoke adapter receives only MCP state/database/events, while aggregate server state still needs App extraction |
| `model_prefs` | model preference values and local load/save | none found | AI owner; verify against current AI configuration before moving |
| `pty` | local PTY state, sessions, events, I/O operations | indirect through backend/MCP | terminal owner; expose a terminal service rather than `App` state |
| `scrollback` | scrollback persistence helpers | `shell`, `workspace` | moved to `labonair-terminal::scrollback`; Workspace supplies session/retention context |
| `secrets` | secret-state compatibility API | no external module import found | `labonair-secrets`; remove wrapper after internal adapters accept `SecretsState`/service |
| `settings` | legacy preferences, migrations, MCP prefs | `app`, `shell`, `workspace`; internal backend modules | Settings owns value persistence; Workspace owns live status-bar/panel layout persistence; legacy `barItemPlacements` is migration-only |
| `sftp` | session adapter, remote operations, transfer worker state/commands | `shell`; internal SSH/transfer adapters | `labonair-sftp` and `labonair-transfers` integration boundaries; SFTP service now receives only SSH state plus the raw event bus |
| `shell` | local command execution, shell sessions, background processes | no active external module import found | terminal/workspace owner; split local process service from backend facade |
| `snippets` | snippet DB compatibility and SSH executor adapter | `shell`; internal backend use | `labonair-snippets` integration boundary |
| `ssh` | SSH state, transport, PTY, remote files, tunnels, config import/export | `shell`; internal Git/SFTP/snippet/MCP use | `labonair-ssh` integration boundary; inject narrow services |
| `terminal_exec` | terminal execution state and command helpers | internal MCP use | MCP/terminal contract; no public `App` access |
| `themes` | legacy theme values, import/export/download operations | none found | `labonair-theme`; static registry is canonical, legacy download path is deferred |
| `transfers` | `BackendTransferService` and event source | `shell` | `labonair-transfers` integration boundary; service receives only `TransferWorkerState`, event translation stays once at adapter edge |
| `updater` | update manifest, verification, download/install helpers | `shell::updater`, app smoke tests | updater/application boundary; root re-export removed and consumers use the updater module directly |

## Direct dependency evidence

Cargo currently declares `labonair-backend` directly in these non-backend
crates:

| Crate | Why it currently imports backend | Removal seam |
|---|---|---|
| `labonair` | constructs `App`, emits startup events, runs legacy settings migration | composition receives concrete services and typed startup hooks |
| `labonair-shell` | constructs `App`, builds SSH/SFTP/Git/transfer adapters, reads MCP/settings/updater compatibility APIs | one composition-only adapter import per capability, with no feature state access |
| `labonair-ai` | no active backend usage; stale dependency declaration | removed in the R06 inventory pass |

`settings`, `settings-content`, and related crates contain historical comments
or migration references to backend names, but they do not declare or import the
backend crate as a runtime dependency.

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
The backend Settings module retains only the shared JSON helpers needed by
legacy migrations and value-settings adapters; it no longer owns locks or live
layout read/write functions.

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
integration remain a separate App-boundary extraction.

## Verification commands

```text
rg -n "labonair_backend|labonair-backend|backend::" crates --glob '*.rs' --glob 'Cargo.toml'
find crates/backend/src/modules -maxdepth 2 -type f | sort
cargo metadata --no-deps --format-version 1
scripts/check-crate-deps.sh
```
