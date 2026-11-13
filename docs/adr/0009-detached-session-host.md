# ADR 0009 — Detached session host: terminals, SSH, and SFTP sessions can outlive the app

* **Status:** Accepted
* **Date:** 2026-11-13
* **Deciders:** project owner and implementation team
* **Related:** [`0008-onboarding-module.md`](0008-onboarding-module.md), [`../capabilities.md`](../capabilities.md), [`../architecture.md`](../architecture.md), [`../security/`](../security/)

## Context

Terminals, SSH sessions, SFTP sessions, and transfer jobs live inside the app
process. Quitting or crashing the app kills every shell, remote connection, and
running transfer. Users who run long-lived work (agents, builds, remote shells)
want them to survive an app restart, as with `tmux`, without giving up native
rendering, selection, and search. The seams to do this already exist: the
UI-free traits `SshConnectionService`, `SshPtyService`, `SshRemoteCommandService`,
`SshRemoteFileService`, `SftpSessionService`, `SftpBrowserService`,
`TransferService` (crates `ssh`, `sftp`, `transfers`) and the terminal
`RemoteSession` / `RemoteFeed` seam in `labonair-terminal`.

## Decision

1. **Optional background daemon `labonair-host`.** A small binary (no GPUI)
   owns PTYs, SSH/SFTP sessions, and transfer workers when the user opts in.
   The default mode is `app` (today's behaviour; no daemon is started, zero
   cost). Windows is deferred; everything is `cfg(unix)` and the setting is
   hidden elsewhere.
2. **Mode is a user setting and is fixed per session at creation.**
   `terminal.sessionHost: app | detached`. Changing it affects only new
   sessions; the registry may hold both kinds at once. SSH/SFTP choose at
   connect time; a transfer job belongs to the host of its SSH session.
3. **Crates, one owner (terminal).** `labonair-terminal-core` — the GPUI-free
   emulator, local PTY session, scrollback, and shell-integration bootstrap,
   extracted from `labonair-terminal`, which re-exports it so downstream
   import paths are unchanged. `labonair-terminal-host` — wire protocol,
   client, daemon library, `SessionHost` trait. Binary `labonair-host`. The
   daemon depends on `terminal-core` but never on GPUI. The theme-to-color
   bridge (`TerminalColorsExt::from_theme`) stays in `labonair-terminal`
   because it needs `labonair-theme` (GPUI); the daemon uses
   `TerminalColors::xterm_default()`.
4. **`SessionHost` trait.** `InProcessHost` wraps today's behaviour; the
   `DaemonHost` client talks to the daemon. `SessionBackend` in the terminal
   registry gains a hosted variant that reuses the `RemoteSession` /
   `RemoteFeed` seam, so the view layer does not learn about the daemon. SSH,
   SFTP, and transfers get thin proxy implementations of the existing traits;
   app code does not change.
5. **Two emulators.** The app keeps its local emulator as the render source
   (selection, search, scroll stay local). The daemon runs a shadow emulator
   that always parses, also while detached. On attach the daemon serializes
   its state as escape sequences (screen, scrollback, cursor, scroll region,
   all `TermMode` flags, title, cwd, prompt phase, alt screen) which the app
   feeds into a fresh emulator, then live bytes follow. The daemon resizes to
   the client size before the snapshot and finishes with a resize wiggle so
   TUIs redraw. A round-trip test (corpus to snapshot to fresh emulator,
   `render()` equal including modes) guards this.
6. **Wire format.** Unix socket, length-prefixed frames. PTY input/output is a
   hot frame (type byte, session id, length, raw bytes); control frames are
   JSON. A per-client bounded queue (1-4 MB) drops the live stream and
   resyncs with a fresh snapshot on overflow; the PTY reader never blocks.
   Idle cost is zero wakeups. Handshake carries `proto_version` and
   `min_compat`; the protocol is additive; incompatible daemons are offered a
   restart.
7. **Credentials and trust.** The app resolves keychain secrets and sends them
   in the connect request; the daemon keeps them in RAM only and never opens
   the keychain or writes them to disk. Host-key trust prompts are events to
   the attached client; with no client attached the connection fails closed.
   A transfer conflict with no UI attached parks the job in awaiting
   resolution; it never auto-overwrites. TCP loss still ends an SSH
   connection (status "connection lost").
8. **Restore and resume.** The session snapshot stores `host_session_id` and
   `host_kind` per pane. Restore order: attach if alive in the daemon, else
   scrollback replay, then the resume hook. The daemon keeps its own registry
   and reconciles on app start; unmatched sessions appear in a "Recovered
   sessions" picker. On SIGTERM the daemon persists scrollbacks through
   `scrollback.rs`. The resume hook detects the foreground process and maps it
   through `terminal.resumeCommands` (defaults such as `claude` ->
   `claude --continue`), sent as `SessionOptions.startup_command`, governed by
   `terminal.resumeOnRestore: off | ask | auto` (default `ask`).
9. **Lifecycle.** Closing a tab kills the session; quitting the app in
   detached mode only detaches; the palette command "Quit and stop all
   sessions" stops everything; `confirm_quit_with_active_sessions` is skipped
   for detached sessions. An exited shell stays `Exited(code)` until
   acknowledged, at most 24 h. The daemon exits after 60 s without sessions,
   jobs, or clients. A second app instance attaching the same session steals it
   (`Detached(Stolen)`). Single instance via `flock` on `host.lock`.
10. **OS integration.** macOS: the app spawns the daemon (`setsid`, stdio to a
    log), not launchd, so TCC attribution stays with the app; the helper is
    copied to a versioned path in the data dir. Socket directory `0700`,
    socket `0600`, peer-uid check; long `sun_path` falls back to
    `/tmp/labonair-<uid>/`. The environment (PATH, `SSH_AUTH_SOCK`, ...) is
    passed per spawn request and never inherited from the daemon. Linux uses
    `XDG_RUNTIME_DIR`; `logind` `KillUserProcesses` may kill the daemon at
    logout and is documented.
11. **Settings and onboarding.** `terminal.sessionHost`,
    `terminal.resumeOnRestore`, `terminal.resumeCommands`, and the daemon
    scrollback limit (default 5000 lines) are ordinary settings; the Settings
    page shows daemon status and the session list with kill. Onboarding gains
    a step `onboarding.sessions` (ADR 0008) that writes the mode through the
    normal settings path.

Explicitly out of scope: PTY fd migration between app and daemon, multi-client
mirroring, a socket auth token (the peer-uid check and file modes are the
boundary), MCP/agent control while the app is closed, macOS notifications from
the daemon, and Windows.

## Consequences

* `labonair-terminal-core` is a new canonical sibling under the terminal owner
  (real dependency boundary: it must stay GPUI-free). `labonair-terminal`
  depends on it; `docs/architecture/graph.toml` records the edge.
* Downstream crates keep their `labonair_terminal::{engine, session, ...}`
  paths. One import changed: `TerminalColors::from_theme` / `from_palette`
  now need `labonair_terminal::TerminalColorsExt` in scope (the struct moved,
  the GPUI-dependent constructors cannot be inherent methods of a foreign type).
* The terminal registry keeps `SessionBackend` and can hold app-bound and
  hosted sessions side by side; closing the app is no longer equivalent to
  killing every session in detached mode.
* A long-lived process holding credentials in RAM is a new trust boundary: the
  security docs and negative tests (untrusted peer, host-key prompt with no
  client, conflict with no UI) are part of the implementation tasks.
* Behaviour in the default `app` mode must remain byte-for-byte identical to
  the pre-ADR terminal; `InProcessHost` exists to prove that.

## Implementation notes: app side (terminals)

* **Client.** `labonair-terminal-host::client::HostClient` is one synchronous
  socket connection per app process (a reader and a writer thread, request
  correlation by `req_id`, per-session `SessionSink` routing). `spawn::shared_client`
  is the process-wide instance; `DaemonHost` (`labonair-terminal`, because it returns
  the registry's `SessionBackend`) implements `SessionHost` on top of it.
* **Helper discovery.** `spawn::ensure_daemon` looks for `labonair-host` at
  `$LABONAIR_HOST_BIN`, next to the running executable (`target/{debug,release}`,
  `Contents/MacOS`) and in `Contents/Resources`, copies it to
  `<data_dir>/host/bin/labonair-host-<content hash>` (older copies are removed) and
  spawns that copy with `setsid`, stdio to `/dev/null`. `scripts/package-macos.sh`
  builds `labonair-terminal-host` with the app and puts the helper into
  `Contents/MacOS`. In development `cargo run -p labonair` does not build it: run
  `cargo build -p labonair-terminal-host` once (the notification on a missing helper
  says so and the pane falls back to an app-bound session).
* **Lifecycle in the registry.** `TerminalRegistry::close` kills a hosted session
  (acknowledges it when it already exited); `close_all` and dropping a handle only
  detach. `has_running_shells` ignores hosted sessions.
* **Restore.** Pane snapshots carry `hostSessionId` and `hostKind` (`app` is the
  default and is not written). A `detached` pane attaches only when the daemon's
  session carries the pane's scrollback uuid as `pane_tag` (ids restart with every
  daemon lifetime); otherwise the scrollback replay and resume hook apply.
  Unattached sessions matching no open pane are offered once at startup
  ("N background sessions found") and by `Terminal: Recover Background Sessions`.

## Implementation notes: daemon side (SSH, SFTP, transfers)

* **Seam.** `server::HostServices` bundles the UI-free trait objects the daemon
  drives (`SshConnectionService`, `SshPtyService`, `SshRemoteCommandService`,
  `SshRemoteFileService`, `SshTunnelService`, `SftpSessionService`,
  `SftpBrowserService`, `TransferService`) plus the `EventBus` they report on.
  `labonair_terminal_host::stack::concrete_services` wires the same adapters
  the app composes in `labonair-shell`; tests inject fakes. A host built
  without services refuses every service call. This is why `labonair-terminal-host`
  depends on `labonair-ssh-transport`, `labonair-sftp-ssh` and
  `labonair-transfers-ssh` (the daemon is its own composition root).
* **Wire.** Request/response is `Message::Rpc { req_id, call: RpcCall }`
  answered by `RpcOk { req_id, value: RpcValue }` or `RpcErr { req_id, error:
  WireError }` (answers may arrive out of order). `RpcCall` mirrors the traits
  1:1 (`SshConnect`, `SshDisconnect`, `SshTrustHost`, `SshPtyWrite`,
  `SshPtyResize`, `SshChown`, `SshCalculateSize`, `SshPrepareRemoteEdit`,
  `SshSaveRemoteEdit`, `SshCleanupRemoteEditTemp`, `SshTunnelStart/Stop/Active`,
  `SftpOpen/Close/ReadDir/Rename/Delete/Mkdir/CreateFile/Chmod`,
  `TransferEnqueue/Cancel/Resolve`) plus `SubscribeEvents`, `UnsubscribeEvents`,
  `TransferConfigure`, `ListTransfers`, `ListSshSessions`. `WireError.code`
  carries the `LabonairError` variant so typed errors survive the wire.
* **Events.** Service events arrive as `Message::BusEvent { name, payload }` in
  the shape of the in-process bus (`ssh_connect_log`, `known_hosts_warning`,
  `auth_required`, `passphrase_required`, `session_established`,
  `ssh_connection_lost`, `sftp_ready`, `transfer_progress`, `transfer_step`,
  `file_conflict`, `file_error`), so the app proxy re-emits them on a local bus
  and the existing `SshEventSourceAdapter` / `TransferEventSourceAdapter` work
  unchanged. They are sent only to connections that called `SubscribeEvents`,
  which first replays the current state (live SSH sessions, transfer jobs with
  steps and pending prompts) and then streams live events.
* **SSH PTY.** `SshConnect` creates a hosted session (`Source::Ssh`: shadow
  emulator fed by the transport's output sink; input and resizes go to
  `SshPtyService`). It uses the same `Attach`/`Snapshot`/`Resync`/steal/`Ack`
  machinery as a local terminal and appears in `ListSessions`. Input bytes are
  converted to UTF-8 text because `SshPtyService::write` takes a `String`.
* **Credentials.** `SshConnectParams` carries `password`, `passphrase` and an
  `SshHostSnapshot` (host row, jump host, credential rows, resolved secrets).
  The daemon loads them into an in-memory SQLite database and
  `SecretsState::in_memory()`; nothing is read from the app's database or secret
  store and nothing is written to disk. `Debug` of every carrier redacts
  secrets. Tunnels need the same snapshot (`SshTunnelStart.host`). The daemon
  uses the `SSH_AUTH_SOCK` it inherited when it was spawned.
* **Fail closed.** `known_hosts_warning` with no subscribed connection, or the
  last subscriber leaving with a prompt open, calls `trust_host(.., false)`.
  Tunnels and jump hops never prompt (they fail fast in the transport).
* **Transfers.** The worker lives in the daemon; the daemon keeps a
  `TransferRegistry` for replay and `jobs_active` (queued, running, paused or
  parked jobs keep the daemon alive; its auto-resolve answers are ignored).
  While no client is subscribed a configured `overwrite` default is downgraded
  to `ask`, so a conflict parks the job until `TransferResolve`. Local paths in
  `TransferEnqueue` and `Ssh{Save,Cleanup}RemoteEdit` must be absolute (or
  `~`-prefixed). History is in memory only, as in the app; finished jobs are
  dropped from the daemon once replayed to a subscriber.
* **Connection loss.** `ssh_connection_lost` turns the hosted session into an
  `Exited`-like state and sends `Message::ConnectionLost { session, reason }`
  (also on a later attach). The session stays until `Ack` (24 h at most); the
  daemon never reconnects. Keepalive defaults to 25 s x 3 when the request
  leaves it open.
