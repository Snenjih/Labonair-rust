# R08-012 — Remove Workspace ownership of Hosts-UI entities

## Status

`✅ Done`

## Owner

- Module: `hosts`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::bootstrap`

## Goal

`labonair-workspace` no longer depends on `labonair-hosts-ui` and holds no
`Entity<HostManagerView>` / `Entity<ConnectionStatusStore>`. Host catalog reads
and the two live snapshots it pushes back cross a narrow contract; host
management and picker semantics stay owned by Hosts.

## What shipped

- **`ConnectionStatusStore` + its vocabulary** (`ConnectionState`,
  `ConnectionKind`, `ConnStage`, `StageStatus`, `ConnectionEntry`,
  `detect_stage`) moved from `crates/hosts-ui/src/ssh_connection.rs` to
  `crates/workspace/src/ssh_connection.rs`. It is a UI-free `gpui` entity that
  nothing in the Hosts UI ever used — the workspace creates it, drives its
  state machine off the SSH event stream, and renders the `SshLoadingScreen`.
- **New leaf crate `crates/hosts-host` (`labonair-hosts-host`)** — deps `gpui`
  + `labonair-hosts` (for `HostPickerRow`). Holds:
  - `HostStatus` (with `pub fn label()`) and `ActiveTunnelRow` (moved from
    `hosts-ui`, which re-exports them for compatibility)
  - `HostView` — seven injected callbacks: `host_ids`, `host_name`,
    `jump_host_label`, `picker_rows`, `recent_picker_rows`, `set_status`,
    `set_active_tunnels`; plus `disconnected()` for headless/tests.
- **`labonair-workspace`** — `Entity<HostManagerView>` field/param replaced by
  `HostView`; the `host_manager()` accessor (no callers) removed; the
  `cx.observe(&host_manager)` re-notify removed (the panel notifies itself);
  every read/mutation goes through `HostView`. `Cargo.toml`: `-hosts-ui`,
  `+hosts-host`.
- **`labonair-shell::bootstrap`** — builds the `HostView` from the active
  `HostManagerView` entity (seven `hm.read` / `hm.update` closures).
  `hosts-ui` still constructs the `HostManagerView` and stays wired for the
  Hosts window + `HostManagerEvent`.
- **`scripts/check_crate_deps.py`** — `hosts-host` leaf entry
  (`{labonair-hosts}`); `workspace` allow-list `-hosts-ui` / `+hosts-host`;
  `hosts-ui` and `shell` gain `hosts-host`. Crate graph regenerated.

## Dependencies

- Existing edges removed: `labonair-workspace → labonair-hosts-ui`.
- New edges: `workspace → hosts-host`, `hosts-ui → hosts-host`,
  `shell → hosts-host` (leaf; `gpui` + `labonair-hosts`).
- Dependency verifier change: as above.

## Persistence and migration

None — same `HostManagerView` / `ConnectionStatusStore` behaviour, only the
crate boundaries moved. No command ids, keymaps, settings, or persistence
formats changed.

## User-visible behavior

None.

## Acceptance criteria

- [x] `labonair-workspace` no longer depends on `labonair-hosts-ui`
      (Cargo.toml + verifier allow-list).
- [x] `Workspace` contains no `Entity<HostManagerView>` /
      `Entity<ConnectionStatusStore>` and cannot mutate Hosts-UI state
      directly (it calls `HostView::set_status` / `set_active_tunnels`, which
      the composition root routes to the entity).
- [x] Host lookup, connection state, and active-tunnel rendering still work
      via typed values (`HostView` + the workspace-owned `ssh_connection`).
- [x] The decoupling did not introduce a host-service facade — `HostView` is
      seven narrow callbacks.
- [x] Registry docs + the dependency graph describe the same boundary.
- [x] `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace --no-fail-fast` (0 failures),
      `scripts/check-crate-deps.sh` (57 crates, 228 edges), and
      `git diff --check` pass.
- [ ] Native connection-flow visual check — folds into R07-001.

## Notes and follow-ups

`ssh_connection.rs`'s `SshLoadingScreen` render + its enums stay in
`workspace.rs` (they use `Palette` / ui-kit); only the store + data types
moved. If a future task wants the loading screen as a Hosts-owned view, that
is a separate move. This is the last `your-task-2.md` P0/P1/P2 code item.
