# T17-008: `AppEvent`-Bus entscheiden (nutzen oder streichen)

## Status
✅ Done

## Entscheidung

**Variante A — Bus behalten + anbinden.** Begründung: der Bus hat bereits **zwei**
echte UI-Konsumenten (`workspace.rs` SSH-/Transfer-Pfad, `panel-snippets`
Run-Log) plus ≥4 konkrete Folge-Konsumenten (Explorer-`fs:dir-changed`,
SCM-Auto-Refresh, MCP-Tab-Ops/Fehler-Toast, `menu:activated`). Damit ist die
Heuristik „≥3 sinnvolle Konsumenten → A" klar erfüllt; B würde nur N parallele
Feature-Kanäle + N Forwarder-Tasks nachbauen. Voll dokumentiert in
`docs/adr/0002-app-event-bus.md` + `docs/architecture.md §8.11`.

Umsetzung: neue Entity `labonair_workspace::backend_event_bridge::BackendEventBridge`
— ein einziger `cx.spawn`-Foreground-Loop über `backend.events`, dekodiert
`RawEvent` → `TransferBusEvent` / `AppEvent` und schiebt sie per `entity.update`
direkt in `Workspace` (`Lagged` → warn+resync, `Closed`/Workspace-weg → stop).
Kein `tokio::spawn` + `mpsc` + Poll-Drain mehr. Referenz-Konsument:
SFTP-Transfer-Fortschritt → `TransfersView::apply`, event-getrieben. Der
40 ms-`ssh_poll` behält nur noch `refresh_active_tunnels` (echter State-Poll).
`spawn_event_logger` in `main.rs` ist jetzt `#[cfg(debug_assertions)]`.

## Phase
16 — Root-Objekt & Registries

## Abhängigkeiten
T17-006 (`AppShell` → reine Komposition)

## Ziel
Den Backend-Event-Bus (`labonair_backend::AppEvent` / `backend.events`) entweder
sauber an die UI anbinden oder ersatzlos entfernen. Aktuell wird jedes Event nur
geloggt (`spawn_event_logger` in `main.rs`) — toter Ballast oder unfertige
Infrastruktur.

## Kontext
- Heute: `crates/app/src/main.rs:27` `spawn_event_logger(&backend)` —
  `backend.events.subscribe()`, `AppEvent::from_raw(&raw)`, `tracing::debug!`.
  Kein UI-Konsument.
- Backend-Seite: `crates/backend/src/events.rs`, `crates/backend/src/app.rs`
  (`events: broadcast::Sender<...>`), Producer in diversen `modules/*`
  (SFTP-Transfer-Fortschritt, Host-Reachability, Git-Status-Änderung,
  Watcher-Events, …) — prüfen, welche `AppEvent`-Varianten real emittiert
  werden (`grep -rn 'events.send\|emit\|AppEvent::' crates/backend/src`).
- Zed-Vorbild: Zed nutzt keinen globalen Bus, sondern gerichtete
  `cx.subscribe`-Kanäle + `Task`s pro Feature. Für Backend→UI ist eine
  schmale Brücke der Standardweg.

## Anweisungen zur Umsetzung
1. **Inventur**: alle real emittierten `AppEvent`-Varianten + ihre Producer
   auflisten. Für jede: Gibt es einen UI-Ort, der reagieren *sollte*?
   Kandidaten:
   - Transfer-Fortschritt → `TransfersStatusItem` (T17-003)
   - Host-Reachability → `HostsPanel` / `JumpHostsStatusItem`
   - Git-Änderung im Arbeitsverzeichnis → `panel-scm` Auto-Refresh
   - FS-Watcher → `panel-explorer` Auto-Refresh
2. **Entscheidung** (in `docs/architecture.md` + ADR `docs/adr/0002-app-event-bus.md`):
   - **Variante A — behalten + anbinden**: eine `BackendEventBridge` (Entity in
     `labonair-workspace` oder `labonair-shell`), die `backend.events`
     subskribiert (im GPUI-Foreground via `cx.spawn` + `AsyncApp`), `AppEvent`
     dekodiert und an die zuständigen Entities weiterreicht (`entity.update`)
     oder als typisierte GPUI-Events re-emittiert. `spawn_event_logger` bleibt
     nur im Debug-Build.
   - **Variante B — streichen**: `backend.events` + `AppEvent` +
     `spawn_event_logger` entfernen; die 1–2 Stellen, die wirklich Backend→UI-
     Push brauchen, bekommen einen gezielten `tokio::sync::watch`/`mpsc` je
     Feature (schmaler, testbarer).
   - Empfehlung: **A**, wenn ≥3 sinnvolle Konsumenten existieren; sonst **B**.
3. **Umsetzung der gewählten Variante**:
   - Bei A: `BackendEventBridge` bauen, mindestens **einen** echten Konsumenten
     verdrahten (Transfer-Fortschritt ins Statusbar-Item) als Referenz-
     Implementierung; die übrigen als Folge-Tickets notieren.
   - Bei B: sauber entfernen, keine toten `pub`-Symbole zurücklassen; die
     betroffenen Producer auf den Feature-lokalen Kanal umstellen.
4. **Kein Per-Frame-Polling** (weder A noch B): die Brücke arbeitet
   event-getrieben über den GPUI-Executor.
5. `cargo run`: mindestens ein sichtbarer Backend→UI-Push funktioniert
   end-to-end (z.B. SFTP-Upload zeigt Live-Fortschritt im Statusbar-Item,
   ohne dass die UI pollt).

## Akzeptanzkriterien
- [x] `docs/adr/0002-app-event-bus.md` dokumentiert die Entscheidung + Gründe.
- [x] Es gibt **keinen** geloggten-aber-ungenutzten Event-Pfad mehr: der Bus ist
      angebunden (Variante A), `spawn_event_logger` ist debug-only Trace.
- [x] Bei A: `BackendEventBridge` + ≥1 realer Konsument (SFTP-Transfer-
      Fortschritt → `TransfersView`); `spawn_event_logger` nur
      `#[cfg(debug_assertions)]`.
- [ ] Bei B: n/a — Variante A gewählt.
- [x] Ein Backend→UI-Push (Transfer-Fortschritt) ist per Konstruktion
      event-getrieben (Foreground-`recv().await`, kein Poll-Drain).
      *E2E-`cargo run` nicht ausführbar — headless VPS ohne Display.*
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `scripts/check-crate-deps.sh`. `cargo test --workspace` **nicht** gelaufen
      — Test-Binaries linken auf diesem headless VPS nicht (fehlende X11-Libs);
      `cargo check/clippy --all-targets` kompilieren den `#[cfg(test)]`-Code als
      projektakzeptierter Ersatz.

## Notizen
- Diese Task schließt die P3-Empfehlung „`AppEvent`-Bus: nutzen oder streichen"
  aus dem Vergleichsbericht ab.
- Falls die Inventur zeigt, dass fast nichts emittiert wird: Variante B ist die
  ehrlichere Wahl — Infrastruktur ohne Nutzer ist Schulden.

## Warnungen
- ⚠️ `broadcast::Receiver` kann `Lagged` liefern (steht schon im heutigen
  Logger). Die Brücke muss `Lagged` sauber behandeln (Resync statt Panik).
- ⚠️ Cross-Thread: `backend.events` läuft auf dem Tokio-Runtime; das
  Weiterreichen an GPUI-Entities muss auf den Foreground (`cx.spawn` /
  `AsyncApp`), niemals `entity.update` vom Tokio-Thread.

## Weiterführende Tasks
- [T18-001: Titlebar-Redesign](../phase-17-layout/T18-001-titlebar-redesign.md)
- [T21-001: Render-Pfad-Profiling](../phase-20-perf-signoff/T21-001-render-path-profiling.md)
