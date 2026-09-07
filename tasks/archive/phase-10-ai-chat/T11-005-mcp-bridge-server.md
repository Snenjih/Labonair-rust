# T11-005: MCP-Bridge — Server

## Status
✅ Done

## Phase
10 — AI-Chat-System (MCP-Bridge ist agent-nahe Infrastruktur, technisch aber unabhängig vom Chat)

## Abhängigkeiten
T03-005 (lokale PTY-Sessions / Multi-Tab-Terminal), T07-001 (SSH-Verbindung)

## Ziel
Parität zu `reference-src/src-tauri/src/modules/mcp/`: ein lokaler MCP-Server, über den eine
externe Agent-CLI (z.B. Claude Code) einen **freigegebenen** Tab steuert — Befehle sichtbar im
echten Pane ausführen, Output lesen, Tabs öffnen/schließen. Ein Server, einmalig hinzugefügt,
nicht pro Tab.

## Kontext
Referenz (`reference-src/src-tauri/src/modules/mcp/`):
- `server.rs` — `rmcp`-Crate (offizielles Rust-MCP-SDK) über **Streamable HTTP** (`axum`),
  bearer-token-gated, gebunden an `127.0.0.1:<port>`. 6 Tools: `list_sessions`, `run_command`,
  `read_output`, `send_keys`, `open_tab`, `close_tab`.
- `osc133.rs` — `vte`-basierter Streaming-Parser: strippt CSI/Farb-Escapes, erkennt den
  OSC 133 `D;<exit_code>`-Marker (aus den Shell-Integration-Skripten,
  `reference-src/src-tauri/src/modules/pty/scripts/`) → so weiß `run_command`, dass ein Befehl
  fertig ist + Exit-Code.
- Output-Tap: `RushSession.agent_tap: broadcast::Sender<...>` neben dem UI-`Channel` — Bridge
  liest dieselben Bytes wie das sichtbare Pane, ohne den Single-Consumer-UI-Channel zu stören.
  Lokaler PTY: `Session.agent_tap` (rohe Bytes) + `write_raw`/`subscribe_agent_tap`.
- Grants: nach **`tab_id`** gekeyt (nicht `session_id` — Tab überlebt Session-Rebind),
  `session_established` re-pusht den Grant.
- Tab-Lifecycle: `open_tab`/`close_tab` können UI-State nicht direkt ändern → Event +
  `tokio::oneshot`-Antwort (wie der Host-Key-Confirm-Flow).
- Commands: `mcp_get_status`, `mcp_set_enabled`, `mcp_regenerate_token`, `mcp_set_session_grant`,
  `mcp_tab_op_response`, `mcp_set_port`, `mcp_set_max_command_timeout_secs`,
  `mcp_set_auto_revoke_minutes`. Auto-Revoke-Sweeper als Background-Task.
- Token im Secrets-Store (`secrets.rs`), nie in SQLite.

## Anweisungen
1. `mcp`-Modul nach `crates/backend/src/mcp/` (oder eigenes Crate) portieren: `server.rs` +
   `osc133.rs`. Tauri-`#[command]`-Wrapper → normale `async fn`, aufgerufen aus der GPUI-App.
2. Den `agent_tap`-Broadcast an die neue Terminal-Session-Struktur (T03-005 lokal, T07-001 SSH)
   anbauen — dieselben Bytes wie der GPUI-Terminal-Renderer.
3. Tab-Lifecycle-Bridge: `open_tab`/`close_tab` senden ein internes Event an den App-Coordinator
   (GPUI), der die echten Tab-Actions ausführt und über `oneshot` antwortet. `open_tab` bleibt
   SSH-only; `open_tab` lehnt Hosts ab, die interaktive Passphrase/2FA bräuchten.
4. Grants `HashMap<tab_id, Grant>` inkl. `kind: Ssh|Local` + `local_pty_id`. Re-push bei
   `session_established`. `close_tab`/Tab-Schließen widerruft den Grant.
5. Runtime-Settings (Port, Max-Timeout, Auto-Revoke-Minuten) als Atomics + Setter; Port-Wechsel
   startet den Listener neu. Auto-Revoke-Sweeper einmal beim App-Start spawnen.
6. Fehlerpfade an das Notification-System (T04-004) hängen: Port-Bind-Fehler → `enabled` zurück
   auf `false` + Fehler-Toast (bekannter Bug im Original, hier direkt richtig machen).
7. `mcp_activity`-Signal für die optionale „bei Aktivität benachrichtigen"-Preference (UI in T11-006).

## Akzeptanzkriterien
- [ ] `mcp_set_enabled(true)` startet den HTTP-Listener auf `127.0.0.1:<port>`, bearer-gated
- [ ] `claude mcp add --transport http …` verbindet; `list_sessions` zeigt nur freigegebene Tabs
- [ ] `run_command` führt im sichtbaren Pane aus, liefert Output + Exit-Code (OSC133) — SSH **und** lokal
- [ ] `read_output` / `send_keys` funktionieren gegen einen freigegebenen Tab
- [ ] `open_tab` (SSH) / `close_tab` treiben echte Tab-Actions und antworten via oneshot
- [ ] Grant folgt `tab_id` über einen Session-Rebind hinweg; Tab-Schließen widerruft
- [ ] Port-Bind-Fehler flippt `enabled=false` + Toast statt „hängt enabled ohne Listener"
- [ ] Auto-Revoke-Sweeper entzieht Grants nach der konfigurierten Zeit
- [ ] `cargo check` + `clippy -- -D warnings` + `cargo test` (OSC133-Parser unit-getestet) grün

## Notizen
- OSC133-Parser: Tests aus dem Original übernehmen (Marker über Chunk-Grenze gesplittet,
  bare-`D`, CSI-Stripping).
- `rmcp` braucht `schemars` als **direkte** Dependency (Derive-Makro emittiert `::schemars::`-Pfade).

## Warnungen
- ⚠️ Nur an `127.0.0.1` binden. Bearer-Token Pflicht auf jedem Request.
- ⚠️ Pro Session ein `tokio::Mutex` — kein Limit auf Gesamt-Sessions; für v1 ok, dokumentieren.
- ⚠️ Sicherheits-Feature — sorgfältig portieren, keine Grant-Checks „vereinfachen".
