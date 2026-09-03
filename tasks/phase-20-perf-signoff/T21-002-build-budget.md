# T21-002: Build-Budget & Crate-Graph-Verifikation

## Status
📋 Geplant

## Phase
20 — Performance & Modularitäts-Abnahme

## Abhängigkeiten
T16-010 (Build-Hygiene & Baseline), T19-009 (Settings-Migrator — letzter großer Crate-Zuwachs)

## Ziel
Nachweisen, dass die Crate-Zerlegung ihr Ziel erreicht hat: kürzere
inkrementelle Builds bei Änderungen in einem Crate, ein sauberer azyklischer
Graph, und die Compile-Zeit ist trotz ~22 Crates nicht schlechter als der alte
7-Crate-Zustand (bevorzugt besser bei Inkrement, neutral bei Cold).

## Kontext
- Baseline aus T16-010: `docs/perf-baseline.md` mit Cold/Warm/Inkrementell-
  Zahlen des 7-Crate-Zustands (bzw. direkt nach Phase 15).
- Regel-Check-Skript aus T16-010: `scripts/check-crate-deps.sh`.
- Nach Phase 15–19 sind alle neuen Crates da (`gpui-ext`, `ui-kit`,
  `notifications`, `command-palette`, `panel`, `workspace`, `shell`,
  `settings-content`, `settings-macros`, `settings`, `settings-json`,
  `settings-ui`, `panel-*` ×6, ggf. `keymap`).
- Zed-Referenz: `zed-refrence/zed/` selbst ist der Beweis, dass ~300 Crates
  mit `sccache`/Workspace-Deps handhabbar sind — Muster (shared
  `[workspace.dependencies]`, dünne Contracts-Crates) übernehmen.

## Anweisungen zur Umsetzung
1. **Messreihe** (Skript `scripts/bench-build.sh`, Ergebnisse nach
   `docs/perf-baseline.md`):
   - Cold: `cargo clean && time cargo check --workspace`;
     `time cargo build --release`.
   - Warm-Noop: `touch` einer Kommentarzeile in `labonair-shell` →
     `time cargo check -p labonair-shell`; dasselbe für
     `labonair-panel-explorer`, `labonair-ui-kit`, `labonair-settings-content`,
     `labonair-backend`.
   - Warm-Downstream: 1-Zeilen-Änderung in `labonair-ui-kit` →
     `time cargo check --workspace` (misst den „ändert alles"-Fall).
   - Jeweils 3×, Median. Maschine/Toolchain/`sccache`-Status notieren.
2. **Zielwerte** (in `docs/perf-baseline.md` festhalten, als Richtwert nicht
   als harter Gate):
   - Inkrementell `-p labonair-shell` nach 1-Zeilen-Änderung: **deutlich**
     unter dem alten „Änderung in `crates/ui/src/app_shell.rs`"-Wert.
   - Cold `cargo check --workspace`: ≤ 115 % des alten Werts (mehr Crates
     kosten etwas Overhead — akzeptabel, wenn Inkrement gewinnt).
3. **Graph-Verifikation**: `scripts/check-crate-deps.sh` läuft grün;
   `scripts/gen-crate-graph.sh` erzeugt das aktualisierte
   `docs/assets/crate-graph.svg`. `cargo-depgraph --workspace-only` zeigt
   **keine** Zyklen. Die Kanten stimmen mit `docs/architecture.md` überein
   (jede Abweichung → entweder Code-Fix oder Doku-Update mit Begründung).
4. **`[workspace.dependencies]`-Hygiene**: alle geteilten Deps zentral
   gepinnt; kein Crate pinnt `gpui`/`serde`/`tokio` lokal abweichend
   (Skript-Check ergänzen).
5. **CI**: `bench-build.sh` **nicht** in CI (zu langsam/varianz), aber
   `check-crate-deps.sh` + `cargo-depgraph`-Zyklus-Check als CI-Step (aus
   T16-010 vorhanden — verifizieren, dass er die neuen Crates abdeckt).
6. **`handshake.md`** + Memory: Build-Zahlen vorher/nachher + Fazit.

## Akzeptanzkriterien
- [ ] `scripts/bench-build.sh` existiert; `docs/perf-baseline.md` enthält
      Cold/Warm-Noop/Warm-Downstream-Zahlen (Median aus 3) mit Kontext.
- [ ] Inkrementeller `-p labonair-shell`-Check nach 1-Zeilen-Änderung ist
      deutlich schneller als der alte `app_shell.rs`-Änderungsfall (Zahlen im
      PR).
- [ ] Cold `cargo check --workspace` ≤ ~115 % des alten Werts (oder Abweichung
      begründet).
- [ ] `cargo-depgraph --workspace-only` zeigt **keine** Zyklen;
      `check-crate-deps.sh` grün; Graph == `docs/architecture.md`.
- [ ] Keine lokal abweichend gepinnten geteilten Deps.
- [ ] CI deckt Dependency-Regeln + Zyklus-Check für alle neuen Crates ab.
- [ ] `docs/perf-baseline.md` + `handshake.md` aktualisiert.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Wenn Cold spürbar schlechter wird: prüfen, ob ein „Fan-in"-Crate
  (`ui-kit`, `settings-content`) unnötig schwere Deps zieht, die man
  feature-gaten kann.
- `sccache`/`mold`/`lld` als Empfehlung in `docs/architecture.md` festhalten,
  falls noch nicht.

## Warnungen
- ⚠️ Build-Zeiten schwanken stark (Thermik, Hintergrundlast) — Median aus 3,
  gleiche Maschine, sonst ist der Vergleich wertlos.
- ⚠️ `cargo-depgraph` muss die Contracts-Crate-Trennung als zyklenfrei
  bestätigen — falls doch ein Zyklus auftaucht, ist die `panel`-Contracts-
  Regel (T16-005) irgendwo verletzt: dort fixen, nicht den Check aufweichen.

## Weiterführende Tasks
- [T21-003: Startup-Profiling](./T21-003-startup-profiling.md)
- [T21-005: Architektur-Doku finalisieren](./T21-005-architecture-doc.md)
