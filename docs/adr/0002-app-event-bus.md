# ADR 0002 — Keep the `AppEvent` backend→UI bus; add `BackendEventBridge`

* **Status:** Accepted
* **Date:** 2026-09-04
* **Deciders:** project owner (Snenjih)
* **Related:** `docs/architecture.md §8.11`, roadmap task **T17-008**,
  `vergleichsbericht-zed-vs-rust.md` (P3 recommendation "`AppEvent` bus:
  use it or drop it"), ADR 0001

## Context

`labonair_backend::EventBus` (a `tokio::sync::broadcast` fan-out) carries named
backend→UI events. Two layers sit on it: the raw `(name, payload)`
[`RawEvent`] wire form every mechanically-stripped Tauri call site emits, and a
typed [`AppEvent`] enum that new code emits / decodes via
`AppEvent::from_raw`.

Before this ADR the only *dedicated* subscriber was `spawn_event_logger` in
`crates/app/src/main.rs` — it decoded every event and wrote a `tracing`
line, nothing else. The comparison report flagged this as "logged-but-unused
infrastructure — either wire it or delete it".

### Inventory — what is actually emitted, and who consumes it

Real emitters (`App::emit` / `App::emit_event` call sites in
`crates/backend/src/modules/*`):

| Event(s) | Producer | Should drive |
| --- | --- | --- |
| `transfer_progress`, `transfer_completed`, `file_conflict`, `transfer_step` | `sftp/worker.rs` | transfers view / transfers status item |
| `session_established`, `auth_required`, `passphrase_required`, `known_hosts_warning`, `ssh_connection_lost`, `ssh_connect_log` | `ssh/client.rs`, `ssh/pty.rs`, `git/executor.rs` | SSH loading screen + connect prompt |
| `fs:dir-changed` | `fs/watcher.rs` (`emit_event`) | explorer auto-refresh (follow-up) |
| `menu:activated` | native menus | shell command dispatch (follow-up) |
| `mcp_open_tab_request`, `mcp_close_tab_request`, `mcp_grant_expired`, `mcp_server_error`, `mcp_activity` | `mcp/server.rs`, `mcp/mod.rs`, `hosts/db.rs` | workspace MCP tab ops + error toast |
| `snippet_output`, `snippet_step`, `snippet_exit`, … | `snippets/exec.rs` | snippets panel run log |

Existing UI consumers (already subscribing before this ADR):

* `crates/workspace/src/workspace.rs` — forwarded `AppEvent` + `TransferBusEvent`
  through a `tokio::spawn` + `std::sync::mpsc` + a 40 ms `cx.spawn` poll-drain.
* `crates/panel-snippets/src/panel_snippets.rs` — same shape for snippet-run
  events, 60 ms poll-drain.

So the bus already has **two** real consumers plus a fistful of well-defined
follow-up consumers (explorer, scm, mcp toast, native menus). That clears the
task's "≥3 sensible consumers → keep" bar comfortably.

## Decision

**Variant A — keep the bus, connect it properly.**

1. New entity **`labonair_workspace::backend_event_bridge::BackendEventBridge`**
   is the single GPUI-side subscriber. It runs one `cx.spawn` loop on the
   **foreground** executor (`tokio::sync::broadcast::Receiver::recv` is
   runtime-agnostic — the `sync` primitives need no active runtime), decodes
   each `RawEvent` into `TransferBusEvent` / `AppEvent`, and pushes it straight
   into the `Workspace` entity via `entity.update`. `Lagged` → warn + resync;
   `Closed` / workspace dropped → stop. No `tokio::spawn`, no intermediate
   `mpsc`, no per-frame poll drain.
2. `Workspace` gains `apply_transfer_bus_event` and makes `handle_ssh_event`
   `pub(crate)` so the bridge can call them. The former 40 ms `ssh_poll` loop
   keeps only its genuinely periodic job — `refresh_active_tunnels` (a state
   poll, not an event feed).
3. `spawn_event_logger` in `crates/app/src/main.rs` is now
   `#[cfg(debug_assertions)]` only — a developer trace, not a product code path.

### Reference consumer

Transfer progress (`transfer_progress` / `transfer_step` / `transfer_completed`
→ `TransferBusEvent` → `TransfersView::apply`) is wired end-to-end through the
bridge: an SFTP upload/download updates the transfers view live, event-driven,
with the UI never polling.

### Follow-up tickets (not in T17-008)

* `fs:dir-changed` → `labonair-panel-explorer` auto-refresh.
* Git-status change → `labonair-panel-scm` auto-refresh.
* Convert `panel-snippets` run-log off its own `tokio::spawn`+`mpsc`+poll onto
  the same foreground pattern (or a bridge-emitted typed GPUI event).
* Route `menu:activated` through `AppShell::dispatch_command`.

## Consequences

* One documented backend→UI seam instead of ad-hoc `tokio::spawn` + `mpsc` +
  timer polls scattered per feature.
* No new crate edge — the bridge lives in `labonair-workspace`, which already
  depends on `labonair-backend` (87 internal edges, unchanged, acyclic).
* `broadcast` back-pressure still applies: a slow foreground loop can `Lag`.
  The bridge handles it (warn + resync) exactly as the old logger did; the
  buffer is 1024 events.
* Cross-thread safety: producers run on the Tokio runtime, but the bridge only
  ever touches GPUI entities from inside `AsyncApp::update` on the foreground —
  never `entity.update` from a Tokio thread.

## Alternatives considered

* **Variant B — delete `AppEvent` / `EventBus` / `spawn_event_logger`, give the
  1–2 real push paths a feature-local `watch`/`mpsc`.** Rejected: there are
  already two consumers and ≥4 concrete follow-ups; removing the bus would just
  mean re-growing N parallel channels and N forwarder tasks. The bus is not
  speculative infrastructure any more.
