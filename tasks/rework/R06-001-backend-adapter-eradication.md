# R06-001 — Remove the broad backend facade

## Status

`✅ Done`

## Owner

- Module: application composition and the capability owners being migrated
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::bootstrap`

## Dependencies

- `R05-001-settings-audit-and-value-normalization`
- `R04-002-host-management-and-connection-pickers`
- `R03-001-command-palette-provider-registry`

## Goal

Replace `labonair-backend` as a broad public application state container with
narrow injected platform adapters and capability-owned services. The
composition root may construct adapters, but no feature may use a backend
facade to reach unrelated state.

## Scope

- In scope: remaining backend modules, `App`, legacy event transport adapters,
  persistence/secret/transport consumers, injection points, and dependency
  allow-list cleanup.
- Out of scope: changing the behavior of terminal, SSH, SFTP, Git, or AI
  beyond what is required to move ownership and preserve their contracts.

## Contracts and ownership

- Each capability keeps its public domain contract in its canonical crate.
- Platform services expose narrow traits or typed channels; external events
  are translated once at the adapter boundary.
- The shell/app crates construct concrete implementations and register them;
  they do not expose a replacement god object.

## Dependencies

- Existing edges removed: feature/workspace/palette access to
  `labonair-backend` internals and the backend's unrelated public re-exports.
- New edges: application composition to concrete adapters; capability owners to
  their own public service contracts.
- Dependency verifier change: remove every transitional backend edge whose
  removal condition is met; fail the verifier if a new facade edge appears.

## Persistence and migration

- Settings: none beyond the completed value migration.
- Storage: keep schemas and secret references stable while moving query and
  adapter ownership.
- Compatibility: legacy event names or signatures may remain only at an
  explicit boundary with a named final consumer and deletion task.

## User-visible behavior

- Canonical entry point: unchanged feature surfaces; this is an ownership
  migration.
- Notifications: adapter failures are translated into the owning capability's
  structured notification request exactly once.
- Inline errors/toasts: no new surface.

## Implementation plan

1. Inventory every backend export and consumer, grouping it by owning module →
   verify the inventory and dependency graph agree.
2. Move the final contracts/adapters and inject them from composition → verify
   focused service tests and no private backend imports.
3. Delete the broad `App` facade paths, legacy re-exports, and allow-list
   exceptions → verify source search, metadata, and full test gates.

## Acceptance criteria

- [x] No feature module depends on broad backend application state. The
      `labonair-backend` crate is absent; `AppComposition` is confined to
      `labonair-shell` / `labonair` (composition roots only).
- [x] Every remaining backend function is either a narrow platform adapter or
      has moved to its capability owner (SSH transport adapters remain the
      `BackendSsh*` structs in `labonair-ssh-transport`).
- [x] Cross-module communication uses typed contracts, events, or registries.
- [x] The application root contains construction and registration only.
- [x] Transitional edges and compatibility paths have been removed or have a
      separately named owner/task. *Residual naming/shape cleanups
      (`AppComposition` `Deref`, `BackendSsh*` names, notification action
      closures) are tracked by the P2.4 / P2.5 follow-up tasks.*
- [x] Focused tests and all repository verification gates pass.

## Removal condition

This task is complete only when deleting the backend facade does not require
feature-specific rewrites outside their public contracts and the dependency
verifier has no broad-facade exception.

## Notes and follow-ups

The binary may retain a small platform-adapter package if a concrete platform
boundary still exists; that package must not become a second capability owner.

## Progress

- [x] Export and consumer inventory recorded in
      [`docs/audits/backend-facade-inventory.md`](../../docs/audits/backend-facade-inventory.md).
- [x] Structured error contract removed from the backend facade and consumed
      directly from `labonair-errors`.
- [x] Stale `labonair-ai → labonair-backend` dependency removed; AI's native
      tool host already consumes `labonair-filesystem` directly.
- [x] System-font discovery moved to `labonair-theme`; the unconsumed backend
      custom-font module and dependency were removed.
- [x] Removed the dead `barItemPlacements` backend read/write facade while
      preserving its migration-only input path. Live status-bar placement and
      panel-toggle persistence now belong to the Workspace chrome owner.
- [x] Workspace no longer constructs Git or Git Graph backend adapters; the
      composition root injects the canonical `labonair-git` services.
- [x] Moved terminal scrollback persistence out of the backend into
      `labonair-terminal::scrollback`; Workspace now supplies only session and
      retention context.
- [x] Established `labonair-mcp-core` for stable MCP session/grant contracts
      and injected it into the Workspace agent-access mirror and tab-operation
      flow; the concrete bridge remains an explicit shell-constructed backend
      adapter.
- [x] Moved Workspace MCP grant revocation, grant creation, and tab-operation
      responses behind the injected `McpSessionAccessService` and
      `McpTabOperationService` contracts; the concrete server remains isolated
      in `labonair-mcp-server` and only its contracts cross into Workspace.
- [x] Replaced Workspace's direct global event-bus subscription with injected
      typed `SshEventSource` and `McpEventSource` contracts; backend event
      decoding now stays inside shell-composed adapters.
- [x] Removed the unconsumed backend filesystem watcher, `App::watcher` state,
      and `backend::modules::fs` compatibility layer; filesystem paths and
      watcher ownership now stay in `labonair-filesystem`.
- [x] Removed the broad backend `AppEvent` enum and typed-emitter helper;
      SSH and MCP adapters now decode only their own raw event names directly
      into the canonical capability contracts.
- [x] Event-source adapters for SSH, MCP, and Transfers now receive only the
      shared raw `EventBus`; they no longer retain the aggregate `App` handle.
- [x] The Transfer service adapter now receives only `TransferWorkerState`
      instead of retaining the aggregate `App` handle.
- [x] The SFTP service adapter now receives only `SshState` and `EventBus`;
      remote SFTP operations use the event bus directly for connection-loss
      reporting instead of receiving the aggregate `App`.
- [x] The MCP grant/session adapter now receives only `McpState` and
      `Database`; host-block revocation receives explicit MCP state and
      `EventBus`, while the concrete MCP server is a separate integration
      adapter.
- [x] The Git operation surface now receives explicit `EventBus` values, and
      both Git service adapters retain only `SshState` plus `EventBus`.
- [x] SSH PTY operations are provided by a dedicated adapter with only
      `SshState`; remote command/file operations use a dedicated adapter with
      only `SshState` plus `EventBus`.
- [x] The SSH connection transport/authentication pipeline now receives only
      `EventBus` for connection logs, trust notifications, and disconnect
      events; database, secrets, and trust orchestration remain at the
      composition adapter boundary.
- [x] The former aggregate SSH adapter was split into separately injected
      connection, connection-test, config, and tunnel adapters; each stores
      only the state required by its contract and no SSH contract is backed by
      `BackendSshService` anymore.
- [x] The concrete SFTP transfer worker was extracted into the dedicated
      `labonair-transfers-ssh` integration sibling; backend SFTP retains only
      session/remote-operation and transfer service/event translation.
- [x] The concrete SFTP session and contract adapters were extracted into the
      dedicated `labonair-sftp-ssh` integration sibling; the backend no longer
      owns an SFTP module.
- [x] The concrete Git CLI/SSH executor and Git contract adapters were
      extracted into the dedicated `labonair-git-transport` integration
      sibling; the backend no longer owns a Git module.
- [x] The concrete MCP server, state, PTY bridge, and contract/event adapters
      were extracted into the dedicated `labonair-mcp-server` integration
      sibling; the backend no longer owns an MCP module.
- [x] MCP server control and HTTP tool execution now use an explicit
      `McpServerAccess` capability bundle; the MCP server no longer stores the
      aggregate backend `App`, and local PTY state is shared through an owned
      reference-counted capability state.
- [x] The SFTP transfer worker now receives `SshState`, `EventBus`, transfer
      conflict state, and transfer settings directly; worker progress,
      reconnect handling, and transfer errors no longer flow through `App`.
- [x] The legacy SFTP connection orchestration now receives `EventBus` rather
      than `App`; health checks, SFTP setup logs, session-established events,
      and connection-loss events use the explicit event capability.
- [x] The unused backend Credentials compatibility module and dependency were
      removed; active credential consumers already use `labonair-credentials`
      directly.
- [x] The unused backend Themes compatibility module, bundled duplicate theme
      asset, and legacy theme dependency surface were removed; static theme
      and icon-theme registries remain owned by `labonair-theme`.
- [x] The unreferenced `terminal_exec` compatibility module and its aggregate
      `App` state were removed; active MCP terminal execution remains owned by
      the MCP server capability.
- [x] The Git service and graph adapters no longer accept the aggregate `App`;
      shell composition injects only SSH state and EventBus.
- [x] The snippets SSH runner no longer accepts the aggregate `App`; event
      delivery is an explicit EventBus capability.
- [x] The aggregate `App::emit` convenience method and final SSH logging macro
      dependency were removed; event producers use explicit EventBus values.
- [x] The aggregate backend `App`/`AppState` facade was removed from
      `labonair-backend`; `BackendComposition` now lives in the shell and owns
      only construction, capability extraction, and worker startup.
- [x] The binary no longer declares direct backend, filesystem, or feature
      engine dependencies; startup migration, diagnostics, and updater types
      are exposed through the shell composition boundary.
- [x] Legacy status-bar placement migration moved into
      `labonair_workspace::status_placements`; the backend Settings module no
      longer owns Workspace chrome compatibility data.
- [x] MCP bridge preferences moved into
      `labonair_mcp_core::preferences`; the backend Settings module no longer
      owns MCP persistence or its wire model.
- [x] Removed the unreferenced backend copies of agents, directives, and model
      preferences; these capabilities have no active consumer and must return
      through an owner contract rather than another backend module.
- [x] Reduced the legacy editor migration surface: `EditorPrefs` is private to
      the v1→v2 migrator, while the unused backend editor persistence adapter
      and projection API were removed.
- [x] Moved the remaining pre-v2 Settings wire model and one-time config
      migration into `labonair-settings::legacy_migrations`; the backend
      Settings module and its `labonair-settings-content` dependency are gone.
- [x] Moved the updater manifest, verification, download/install, and cadence
      logic into the dedicated UI-free `labonair-updater` crate; the shell
      retains only the GPUI presentation and the backend no longer owns an
      updater module.
- [x] Removed the unreferenced backend `shell` compatibility module, including
      one-shot commands, persistent agent shells, and background-process state;
      no active crate used this duplicate execution path.
- [x] Removed the backend Secrets compatibility wrapper; SSH, SFTP, MCP, and
      shell composition now use the canonical `labonair-secrets` state and
      operations directly.
- [x] SSH secret access and jump-host resolution no longer take the aggregate
  `App`; they receive only their actual state dependencies.
- [x] Local MCP terminal access now uses an injected `LocalTerminalAccess`
  capability backed by the real `labonair-terminal::TerminalRegistry`; the
  obsolete `local_pty_id` grant field and duplicate backend PTY implementation
  were removed.
- [x] Shared OSC 7/133 shell-integration payloads moved into the UI-free
  `labonair-terminal-integration` protocol crate, consumed by both local PTY
  and remote SSH adapters.
- [x] Removed the unreferenced backend Snippet-store re-export; the canonical
  `labonair-snippets` store remains the only persistence owner.
- [x] Moved transfer queue state and command dispatch out of the SFTP module
      into the canonical `labonair-transfers` crate and its backend adapter;
      unreferenced SFTP settings and reconnect command wrappers were removed.
- [x] Moved the raw adapter event transport (`EventBus`, `EventChannel`, and
      `RawEvent`) out of `labonair-backend` into the UI-free
      `labonair-events` foundation crate; capability-specific event vocabulary
      remains in the owning contracts.
- [x] Extracted the concrete SSH/russh module, session state, network-error
      classification, and SSH contract adapters into the dedicated
      `labonair-ssh-transport` integration sibling; the backend no longer
      owns an SSH module.
- [x] Move the remaining contracts and adapters behind injected capability
      services.
- [x] Delete the broad `App` facade paths and remaining compatibility edges.

## Completion evidence

The former `labonair-backend` package and its module tree are removed from the
workspace. Concrete SSH snippet execution is owned by
`labonair-snippets-ssh`, transfer execution and adapters are owned by
`labonair-transfers-ssh`, and application construction is named
`labonair-shell::composition::AppComposition`. No workspace crate declares or
imports the broad backend facade; feature modules receive typed contracts or
registry snapshots instead.
