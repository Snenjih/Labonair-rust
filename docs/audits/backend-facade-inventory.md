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
| `App` / `AppState` and `AppInner` | `backend::app` | `app`, `shell`, `workspace`, backend adapters | application composition plus injected capability services | Broad facade remains; split is the main task |
| `AppEvent`, `EventBus`, `EventChannel`, `RawEvent` | `backend::events` | `app`, `workspace::backend_event_bridge`, SSH/SFTP/transfer adapters | typed capability events and explicit transport adapters | Legacy global bus remains transitional |
| updater constants and operations | `backend::modules::updater` | `shell::updater` | updater/application boundary | Narrow enough to migrate independently |
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
| `fonts` | custom-font file operations and system-font discovery | `shell::settings_services` | theme/settings discovery contract; move concrete file operations out of backend |
| `fs` | filesystem re-exports and watcher command adapters | no active module import found | filesystem foundation; delete compatibility wrapper after watcher consumers are migrated |
| `git` | Git operation functions and `BackendGitService` / graph adapter | `shell::bootstrap`, `workspace` | `labonair-git` integration adapter; keep transport implementation narrow |
| `mcp` | MCP state, grants, server operations, host revocation callback | `shell`, `workspace`; internal PTY/secrets use | MCP/AI integration boundary; remove `App` dependency from the capability owner |
| `model_prefs` | model preference values and local load/save | none found | AI owner; verify against current AI configuration before moving |
| `pty` | local PTY state, sessions, events, I/O operations | indirect through backend/MCP | terminal owner; expose a terminal service rather than `App` state |
| `scrollback` | scrollback persistence helpers | `shell`, `workspace` | terminal owner; inject persistence and remove backend path |
| `secrets` | secret-state compatibility API | no external module import found | `labonair-secrets`; remove wrapper after internal adapters accept `SecretsState`/service |
| `settings` | legacy preferences, migrations, placement persistence, MCP prefs | `app`, `shell`, `workspace`; internal backend modules | settings/layout owners; keep only explicit migration or placement adapters |
| `sftp` | session adapter, remote operations, transfer worker state/commands | `shell`; internal SSH/transfer adapters | `labonair-sftp` and `labonair-transfers` integration boundaries |
| `shell` | local command execution, shell sessions, background processes | no active external module import found | terminal/workspace owner; split local process service from backend facade |
| `snippets` | snippet DB compatibility and SSH executor adapter | `shell`; internal backend use | `labonair-snippets` integration boundary |
| `ssh` | SSH state, transport, PTY, remote files, tunnels, config import/export | `shell`; internal Git/SFTP/snippet/MCP use | `labonair-ssh` integration boundary; inject narrow services |
| `terminal_exec` | terminal execution state and command helpers | internal MCP use | MCP/terminal contract; no public `App` access |
| `themes` | legacy theme values, import/export/download operations | none found | `labonair-theme`; static registry is canonical, legacy download path is deferred |
| `transfers` | `BackendTransferService` and event source | `shell` | `labonair-transfers` integration boundary; event translation stays once at adapter edge |
| `updater` | update manifest, verification, download/install helpers | `shell::updater`, root re-export | updater/application boundary; remove unrelated root re-export after shell imports the module directly |

## Direct dependency evidence

Cargo currently declares `labonair-backend` directly in these non-backend
crates:

| Crate | Why it currently imports backend | Removal seam |
|---|---|---|
| `labonair` | constructs `App`, emits startup events, runs legacy settings migration | composition receives concrete services and typed startup hooks |
| `labonair-shell` | constructs `App`, builds SSH/SFTP/Git/transfer adapters, reads MCP/settings/updater/font/scrollback compatibility APIs | one composition-only adapter import per capability, with no feature state access |
| `labonair-workspace` | owns legacy event bridge, scrollback, placement persistence, MCP grants, and direct Git adapter construction | injected workspace services plus workspace-owned layout/terminal contracts |
| `labonair-ai` | legacy host-tool dependency declaration | remove once host/SSH access is represented by AI-owned injected contracts |

`settings`, `settings-content`, and related crates contain historical comments
or migration references to backend names, but they do not declare or import the
backend crate as a runtime dependency.

## First completed boundary

The error contract no longer passes through the facade. Backend transport code
imports `labonair-errors` directly, while `backend::modules::errors`, the root
error re-exports, and the unused `AppResult` alias are gone. This establishes
the migration pattern for the remaining entries: move or consume the canonical
contract first, then delete the backend compatibility path once source search
and focused tests prove that no external consumer remains.

## Verification commands

```text
rg -n "labonair_backend|labonair-backend|backend::" crates --glob '*.rs' --glob 'Cargo.toml'
find crates/backend/src/modules -maxdepth 2 -type f | sort
cargo metadata --no-deps --format-version 1
scripts/check-crate-deps.sh
```
