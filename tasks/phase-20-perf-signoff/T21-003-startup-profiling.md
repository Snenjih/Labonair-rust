# T21-003: Startup-Profiling (Zeit bis erstes Frame, Speicher-Baseline)

## Status
📋 Geplant

## Phase
20 — Performance & Modularitäts-Abnahme

## Abhängigkeiten
T17-006 (`AppShell` → reine Komposition), T19-002 (`SettingsStore`)

## Ziel
Den App-Start messen und die Rework-bedingten Startkosten (Migratoren,
`SettingsStore`-Merge, Registry-Aufbau, `inventory`-Sammeln, Theme-/Icon-
Registry-Load, fs-Watcher) einordnen — und mit dem ROADMAP-Erfolgskriterium 15
(„Performance messbar besser als die Referenz") abgleichen.

## Kontext
- ROADMAP-Erfolgskriterium 15: Start/Rendering/Speicher messbar besser als die
  Tauri-Referenz.
- Neue Startsequenz (`bootstrap`, T17-006): `migrate_bar_item_placements`
  (T18-006) → `migrate_settings_v1_to_v2` (T19-009) → `settings::init`
  (Layer-Merge, fs-Watch) → `keymap::load` (T19-008) → `ThemeRegistry`/
  `IconThemeRegistry`-builtin+user (T20-005/006) → Panel-/StatusItem-/
  Command-Registry-Aufbau → Session-Snapshot-Replay → erstes `render`.
- `main.rs` startet vorher schon Tokio + Backend + Workers.
- Werkzeuge: `tracing`-Spans mit Zeitstempeln über jede Bootstrap-Stufe;
  `std::time::Instant` von `main()` bis zum ersten `Window`-Draw-Callback;
  RSS via `/proc/self/status` (Linux) bzw. `task_info` (macOS) direkt nach
  dem ersten Frame und nach 30 s Idle.

## Anweisungen zur Umsetzung
1. **Instrumentierung**: `tracing`-Span je Bootstrap-Stufe (Name + Dauer),
   ein Marker „first-frame" im ersten `render` der Shell. Ein
   `LABONAIR_STARTUP_TRACE=1`-Env, das die Stufen-Zeiten nach stdout/JSON
   schreibt.
2. **Messung** (`scripts/bench-startup.sh`, 5×, Median):
   - Kaltstart (frisch gebautes Release-Binary, kein Warm-Cache):
     `main()` → first-frame.
   - Warmstart (Binary + OS-Datei-Cache warm).
   - Mit vorhandener großer `labonair-settings.json` + `keymap.json` +
     3 User-Themes + Session-Snapshot mit 10 Tabs + 2×2-Split.
   - RSS bei first-frame und nach 30 s Idle.
3. **Analyse**: welche Stufe kostet was. Erwartete Verdächtige:
   `inventory`-Sammeln (sollte ~0 sein), `SettingsStore`-Merge (klein),
   Migratoren (nur beim allerersten Start teuer — separat ausweisen: „erster
   Start je Nutzer" vs. „danach"), Theme-JSON-Parsen, fs-Watcher-Registrierung.
4. **Fixes, falls nötig**:
   - Migratoren: nach dem ersten Lauf per `schemaVersion`-Check sofort
     no-op (in T18-006/T19-009 schon so — hier verifizieren, dass der
     No-op-Pfad wirklich <1 ms ist).
   - Theme-/Icon-User-Ordner-Scan: lazy (erst wenn Settings-UI geöffnet wird)
     laden, nur das **aktive** Theme beim Start.
   - Session-Replay: teure Tab-Inhalte (Terminal-PTY-Spawn) gestaffelt nach
     dem ersten Frame.
   - fs-Watcher: nach dem ersten Frame registrieren.
5. **`docs/perf-baseline.md`**: Startup-Tabelle (Stufen-Zeiten, first-frame
   kalt/warm, RSS), plus — wenn beschaffbar — eine grobe Vergleichszahl der
   Tauri-Referenz (aus früheren Messungen / `handshake.md` / einmal manuell).
6. **Regressions-Guard**: ein einfacher Smoke-Test / CI-Log, der die
   first-frame-Zeit im Debug-Build grob prüft (großzügige Obergrenze, nur um
   Ausreißer zu fangen).

## Akzeptanzkriterien
- [ ] Bootstrap-Stufen sind einzeln zeitlich vermessen
      (`LABONAIR_STARTUP_TRACE=1`).
- [ ] `scripts/bench-startup.sh` liefert Median-Zahlen für kalt/warm +
      „großer Zustand" + RSS (first-frame & 30 s Idle).
- [ ] „Erster Start je Nutzer" (mit Migration) ist getrennt ausgewiesen von
      „normaler Start" (Migration = no-op < 1 ms).
- [ ] Kein Bootstrap-Schritt blockiert das erste Frame unnötig (Theme-Scan
      lazy, fs-Watcher & schwere Session-Inhalte nach first-frame).
- [ ] `docs/perf-baseline.md` enthält die Startup-Tabelle + eine Referenz-
      Vergleichszahl (oder die Notiz, warum sie nicht beschaffbar war).
- [ ] Grober first-frame-Regressions-Guard vorhanden.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Die Referenz-Vergleichszahl muss nicht perfekt sein — eine
  Größenordnung („Rust-Start ~X ms vs. Tauri ~Y ms") reicht für
  Erfolgskriterium 15.
- Falls first-frame kalt auffällig hoch ist und die Ursache Linker/Binary-
  Größe ist → als Input für T22-001 (gpui-Basis) notieren, nicht hier lösen.

## Warnungen
- ⚠️ `inventory` sammelt zur Programmstart-Zeit über Linker-Sektionen — auf
  beiden Plattformen verifizieren, dass das nicht linear mit der Anzahl
  registrierter Settings wächst (sollte konstant/klein sein).
- ⚠️ Session-Replay mit vielen Terminal-Tabs spawnt viele PTYs — die Zeit
  dafür ist echte Arbeit, nicht „Startup-Overhead". Sauber trennen in der
  Messung (first-frame vs. „alle Tabs interaktiv").

## Weiterführende Tasks
- [T21-004: Modularitäts- & Personalisierungs-Abnahme](./T21-004-modularity-personalization-signoff.md)
