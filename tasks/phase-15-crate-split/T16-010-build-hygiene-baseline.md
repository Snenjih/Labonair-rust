# T16-010: Build-Hygiene & Baseline

## Status
📋 Geplant

## Phase
15 — Crate-Zerlegung & Fundament

## Abhängigkeiten
T16-009 (`labonair-shell` + `labonair-app` schlank)

## Ziel
Die neue Crate-Zerlegung absichern: die Abhängigkeitsregeln aus
`docs/architecture.md` maschinell prüfen, den Crate-Graphen dokumentieren, und
`cargo check`/`clippy`/`test`-Zeiten als Baseline festhalten (Vergleichspunkt
für die Perf-Abnahme in T21-002).

## Kontext
- Regeln aus `docs/architecture.md` (T16-001): `panel` ohne Workspace-Deps,
  `panel-*` nicht untereinander / nicht auf `shell` / nicht auf `ui`,
  `backend`/`ai`/`terminal`/`editor` ohne UI-Deps, `ui-kit` nur auf
  `gpui`/`gpui-component`/`theme`/`gpui-ext`.
- CI heute: `.github/` enthält Workflows (`cargo check/clippy/test/fmt`).
- Werkzeuge: `cargo tree`, `cargo-depgraph` (Graphviz-Ausgabe),
  `cargo metadata` (JSON — für ein kleines Prüfskript).
- Zed-Vorbild: `zed-refrence/zed/clippy.toml`, `zed-refrence/zed/typos.toml`,
  `zed-refrence/zed/tooling/` (Workspace-weite Lints + Skripte).

## Anweisungen zur Umsetzung
1. **Dependency-Regel-Check** `scripts/check-crate-deps.sh` (oder ein kleines
   Rust-Bin unter `tooling/`):
   - `cargo metadata --format-version 1` parsen.
   - Für jede verbotene Kante (siehe Regeln oben) → Exit 1 mit klarer Meldung
     („`labonair-panel-ai` hängt von `labonair-panel-scm` — verboten laut
     docs/architecture.md").
   - Whitelist der erlaubten Kanten pro Crate im Skript, kommentiert.
2. **CI einbinden**: neuer Job/Step in `.github/workflows/*.yml`, der
   `scripts/check-crate-deps.sh` ausführt. Zusätzlich `cargo clippy` pro Crate
   (`--workspace` bleibt, aber ein `-p`-Matrix-Step macht Regressionen
   sichtbarer) — optional, wenn CI-Zeit es erlaubt.
3. **Crate-Graph-Doku**: `docs/architecture.md` um einen generierten Abschnitt
   „Ist-Graph" ergänzen — `cargo depgraph --workspace-only | dot -Tsvg` →
   `docs/assets/crate-graph.svg`, plus die Textliste `cargo tree
   --workspace --depth 1`. Ein `scripts/gen-crate-graph.sh` legt beides an.
4. **Baseline-Messung** `docs/perf-baseline.md`:
   - Kalt (`cargo clean`) + warm: Zeit für `cargo check --workspace`,
     `cargo clippy --workspace --all-targets`, `cargo test --workspace`,
     `cargo build --release`.
   - Inkrementell: Zeit für `cargo check -p labonair-shell` nach einer
     1-Zeilen-Änderung in `app_shell.rs` (der Wert, den die Zerlegung
     verbessern soll).
   - Maschine/Toolchain/Datum notieren.
5. **`handshake.md`** + Memory: Baseline-Zahlen + „Phase 15 abgeschlossen,
   Crate-Graph steht" festhalten.

## Akzeptanzkriterien
- [ ] `scripts/check-crate-deps.sh` existiert, ist ausführbar, schlägt bei
      einer künstlich eingefügten verbotenen Kante fehl (im Test kurz
      verifizieren, dann zurücknehmen) und ist grün für den echten Graphen.
- [ ] CI führt den Dependency-Check aus; ein PR mit verbotener Kante wird rot.
- [ ] `docs/assets/crate-graph.svg` + Textliste in `docs/architecture.md`
      spiegeln den realen Workspace nach Phase 15.
- [ ] `docs/perf-baseline.md` enthält kalte/warme/inkrementelle Zahlen mit
      Kontext (Maschine, Toolchain, Datum).
- [ ] `handshake.md` ist aktualisiert.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Der inkrementelle `-p labonair-shell`-Wert ist die wichtigste Zahl: Ende
  Phase 15 sollte eine 1-Zeilen-Änderung in `app_shell.rs` **nicht** mehr den
  halben `ui`-Monolithen neu übersetzen.
- `cargo-depgraph` ggf. in CI vorinstallieren oder das SVG nur lokal
  generieren und committen (CI prüft dann nur die Textregeln).

## Warnungen
- ⚠️ Das Regel-Skript muss transitive Kanten korrekt behandeln — `panel-ai`
  darf `workspace` (erlaubt) ziehen, aber nicht über `workspace` doch wieder
  bei `shell` landen. `cargo metadata` liefert nur direkte Deps; für transitiv
  den Graphen selbst traversieren.

## Weiterführende Tasks
- [T17-001: `Panel`-Trait & `PanelRegistry` verdrahten](../phase-16-registries/T17-001-panel-trait-and-registry.md)
- [T21-002: Build-Budget](../phase-20-perf-signoff/T21-002-build-budget.md)
