# T21-001: Render-Pfad-Profiling & Frame-Hygiene

## Status
🔄 In Progress

## Phase
20 — Performance & Modularitäts-Abnahme

## Abhängigkeiten
T17-006 (`AppShell` → reine Komposition), T20-003 (View-Migration Welle 2)

## Ziel
Den Render-Pfad messen und die pro-Frame-Verschwendung eliminieren, die der
alte God-Object-Aufbau hatte: unnötige `cx.notify`, pro-Frame-Allokationen
(`build_palette_data`, `sync_live_bridge`, `ActiveTheme`-Recompute) und
überbreite `observe`-Ketten. Ziel ist ein ruhiger Idle-Frame und ein
schlanker aktiver Frame.

## Kontext
- Alt-Schmerzpunkte (Vergleichsbericht §6.2): `render()` von `AppShell` rief
  `drain_pending_*` + `sync_live_bridge` + `build_palette_data` **pro Frame**
  (großteils in Phase 16 bereits entfernt — hier verifizieren + Rest jagen).
- `~10` `cx.observe(&x, |_,_,cx| cx.notify())` im alten `AppShell` — jede
  Änderung an irgendeiner Entity rendert die ganze Shell.
- Werkzeuge: GPUIs `--profile`/Frame-Timing, `tracing` mit Spans um
  `render`/`recompute`, `puffin`/`tracy`-Integration falls in `gpui` 0.2.2
  verfügbar (sonst `std::time::Instant`-Spans), `dhat`/`heaptrack` für
  Allokationen.
- `zed-refrence/zed/crates/gpui/` — GPUIs Dirty-Region-/Invalidierungs-Modell
  (`cx.notify` nur bei echter Änderung), `zed-refrence/zed/crates/
  input_latency_ui/` als Inspirationsquelle für Messung.

## Anweisungen zur Umsetzung
1. **Instrumentierung**: `tracing`-Spans um `Render::render` der Kern-Views
   (Shell, Workspace, Dock, StatusBar, jede Panel-View), um
   `SettingsStore::recompute`, `ThemeStore`-`ActiveTheme`-Recompute,
   `WorkspaceLiveBridge`-Snapshot. Ein `RUST_LOG=labonair::perf=trace`-Ziel.
2. **Idle-Messung**: App offen, keine Eingabe, 10 s. Erwartung: **0**
   `render`-Aufrufe der Kern-Views (GPUI rendert nur bei `notify`/Input).
   Jeder Idle-Render ist ein Bug → Quelle finden (welche Entity notifiziert
   grundlos — Timer? Observe-Kaskade? Terminal-Blink?).
3. **Aktiver-Frame-Messung**: definierte Interaktionen (Tab wechseln, tippen
   im Terminal, Panel togglen, Settings-Feld ändern, Split-Resize). Pro
   Interaktion: welche Views rendern, wie oft, wie lange. Baseline-Tabelle in
   `docs/perf-baseline.md` (neben den Build-Zahlen aus T16-010).
4. **Fixes**:
   - Überbreite `observe`: `AppShell`/`Workspace` sollen nicht bei *jeder*
     Sub-Entity-Änderung komplett rendern. Gezielter observen (nur was das
     Layout betrifft) bzw. GPUI die Teil-Invalidierung überlassen (Kind-View
     rendert sich selbst).
   - Pro-Frame-Allokationen: `build_palette_data` (falls Rest da) →
     event-getrieben cachen; `ActiveTheme`/`merged`-Settings → nur bei
     Änderung; String-/Vec-Neubau in `render` → `SharedString`/Felder.
   - Terminal-Cursor-Blink: nur das Terminal-View invalidieren, nicht die
     Shell.
   - `WorkspaceLiveBridge`: bestätigt event-getrieben (T17-006) — hier nur
     verifizieren, dass kein Frame-Trigger übrig ist.
5. **Regressions-Guard**: ein `#[test]` (GPUI-Test-Kontext), der eine kurze
   Interaktionssequenz fährt und die Anzahl `render`-Aufrufe der Shell gegen
   eine Obergrenze prüft (z.B. „Tab-Wechsel rendert die Shell ≤ 2×").
6. **`docs/perf-baseline.md`** aktualisieren: Idle-Renders (soll 0),
   Frame-Zeiten pro Interaktion, Peak-Allokationen pro Interaktion, Vergleich
   zum Zustand vor Phase 15 (falls messbar) bzw. zur ersten Rework-Messung.

## Akzeptanzkriterien
- [ ] `tracing`-Spans decken die Render-/Recompute-Pfade; `labonair::perf`-Ziel
      dokumentiert.
- [ ] Idle (10 s, keine Eingabe): **0** Kern-View-Renders (oder jede Ausnahme
      ist erklärt + als Bug getrackt).
- [ ] Keine vermeidbaren Pro-Frame-Allokationen in `render` der Kern-Views
      (dhat/heaptrack-Beleg im PR für mind. „Tab-Wechsel" + „Terminal tippen").
- [ ] Überbreite `observe`-Ketten entschärft: eine Sub-Panel-Änderung rendert
      nicht die ganze Shell.
- [ ] Regressions-Test für Render-Anzahl bei einer Interaktionssequenz.
- [ ] `docs/perf-baseline.md` enthält die neuen Zahlen + Vergleich.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- „Ruhiger Idle-Frame" ist der beste Einzelindikator, dass der God-Object-
  Umbau gelungen ist.
- Nicht Mikro-optimieren, wo es nicht misst — erst messen, dann fixen, dann
  gegenmessen (Goal-Driven).

## Warnungen
- ⚠️ Der Terminal-Cursor-Blink + laufende PTY-Ausgabe erzeugen legitime
  Renders — die vom „grundlosen" Idle-Render unterscheiden (Terminal ohne
  Ausgabe + Fokus weg sollte still sein).
- ⚠️ `tracing`-Spans im Render-Pfad selbst kosten etwas — im Release-Profiling
  mit `RUST_LOG` aus messen, Spans nur bei Bedarf.

## Weiterführende Tasks
- [T21-002: Build-Budget](./T21-002-build-budget.md)
- [T21-003: Startup-Profiling](./T21-003-startup-profiling.md)
