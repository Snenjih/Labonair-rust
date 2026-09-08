# R08-012 — Remove Workspace ownership of Hosts-UI entities

## Status

`⏳ Planned`

## Owner

- Module: `hosts`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::bootstrap`

## Why deferred

This is the highest regression-risk decoupling in the `your-task-2.md` audit.
`crates/workspace/src/workspace.rs` (>4000 lines) stores
`Entity<HostManagerView>` and `Entity<ConnectionStatusStore>`, exposes
`host_manager()` publicly, and drives the live SSH connection-status path
(`set_status`, `set_active_tunnels`, `ConnectionEntry` handling) directly
against Hosts-UI entities. Untangling it touches the connection flow, the
status statusbar item, the tab views, and the MCP tab-open path. It is handed
off for review rather than executed at the tail of a long batch.

The violation is confirmed: `crates/workspace/Cargo.toml` depends on
`labonair-hosts-ui`; `workspace.rs` stores both entities and calls Hosts-UI
mutation methods (`docs/architecture.md`: "cross-feature behavior uses a typed
contract ... does not reach into another feature's private state").

## Scope (when activated)

- Keep `HostManagerView`, `ConnectionStatusStore`, `ConnectionEntry`,
  `HostStatus`, and picker semantics inside `labonair-hosts-ui`.
- Define a UI-free host/SSH status snapshot + event contract at the lowest
  sensible boundary (`labonair-hosts` or a new `labonair-hosts-host` leaf,
  mirroring `explorer-host` / `snippets-host`). It must carry only: host
  catalog lookup, a connection-status update sink, and an active-tunnel-rows
  update sink — not a broad host-service facade.
- Inject that narrow capability into `Workspace` from `bootstrap`; remove the
  `host_manager()` accessor and both `Entity<…>` fields.
- Keep host management and the palette picker (Enter = SSH, Shift+Enter =
  SFTP) owned by Hosts.

## Acceptance criteria (when activated)

- [ ] `labonair-workspace` no longer depends on `labonair-hosts-ui`.
- [ ] `Workspace` contains no `Entity<HostManagerView>` /
      `Entity<ConnectionStatusStore>` and cannot mutate Hosts-UI state
      directly.
- [ ] Host lookup, connection state, and active-tunnel rendering still work
      via typed values/events.
- [ ] Focused tests cover status updates and host action routing without
      constructing Hosts-UI entities.
- [ ] Dependency verifier + architecture inventory describe the same
      boundary.
- [ ] Full verification suite passes; native connection-flow visual check
      (folds into R07-001).

## Notes and follow-ups

Do not resolve this by introducing a broad host-service facade (explicit
`your-task-2.md` constraint). The `explorer-host` / `snippets-host` leaf-crate
pattern (R07-004 / R08-003) is the template.
