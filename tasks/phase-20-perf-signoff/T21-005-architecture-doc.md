# T21-005: Architektur-Doku finalisieren

## Status
📋 Geplant

## Phase
20 — Performance & Modularitäts-Abnahme

## Abhängigkeiten
T21-004 (Modularitäts- & Personalisierungs-Abnahme)

## Ziel
`docs/architecture.md` von einem Plan-Dokument (T16-001) auf eine **präzise
Beschreibung des Ist-Zustands** nach dem Rework bringen, `handshake.md`
konsolidieren, und die Rework-Berichte/Task-Struktur als abgeschlossen
markieren.

## Kontext
- `docs/architecture.md` wurde in T16-001 als Ziel angelegt und in mehreren
  Tasks fortgeschrieben (Crate-Heimat-Entscheidungen T16-009, Registries T17,
  Layout-Vertrag T18, Settings-Schichten T19, UI-Kit/Theme T20).
- `docs/adr/000{1,2}-*.md` (Crate-Zerlegung, AppEvent-Bus).
- `docs/perf-baseline.md` (T16-010/T21-002/003), `docs/signoff-*.md`
  (T21-004), `docs/assets/crate-graph.svg`.
- `handshake.md` (Repo-Root) — über die Rework-Phasen mit vielen Einträgen
  gewachsen.
- Vergleichsberichte im Repo-Root: `vergleichsbericht-zed-vs-rust.md`,
  `bericht-architektur-rework-roadmap.md`, `vergleichsbericht-subagent-1..4.md`.

## Anweisungen zur Umsetzung
1. **`docs/architecture.md` umschreiben** — von Futur („wird") auf Präsens
   („ist"). Abschnitte, jeder mit aktuellem Stand:
   - **Philosophie** (unverändert normativ).
   - **Crate-Graph (Ist)** — die tatsächliche Liste + das generierte SVG +
     `cargo tree --workspace --depth 1`-Auszug; Abhängigkeitsregeln + wie sie
     erzwungen werden (`check-crate-deps.sh`, CI).
   - **Root & Registries** — `AppShell` (dünn), `Workspace`, `PanelRegistry`,
     `StatusItemRegistry`, `CommandRegistry`, `Dock` (L/R/B), `PaneGroup`,
     `ModalLayer`/`ToastLayer`. Für jede: „wie füge ich eine hinzu" (kurzes
     Rezept, aus T21-004-Sign-off).
   - **Layout-Vertrag (Ist)** — Titlebar/Workspace/Statusbar/Panels/Overlays,
     inkl. der bewussten Abweichungen ggü. `reference-src`.
   - **Settings-System** — `SettingsContent`-Baum, Layer-Merge-Reihenfolge,
     `Settings`-Trait/Registrierung, generierte UI, `settings.json`-Surgical-
     Edit, Schema, `keymap.json`, Projekt-Settings, Migratoren + `schemaVersion`.
   - **Theme-System** — `ThemeRegistry`, JSON-Familien, Icon-Themes,
     `ThemeSettings`/`UiDensity`, `ActiveTheme`.
   - **Performance-Leitplanken** — Idle = 0 Renders, kein Pro-Frame-Recompute,
     Bootstrap-Reihenfolge, wo Lazy geladen wird.
   - **Verweise** — auf die ADRs, `perf-baseline.md`, `signoff-*.md`.
2. **`CLAUDE.md`** (Repo-Root): den „## Architecture"-Abschnitt an den
   Ist-Zustand anpassen (die ASCII-Grafik der eingebetteten Module aktualisieren
   auf den neuen Crate-Graphen bzw. auf `docs/architecture.md` verweisen);
   Critical Rules ggf. um die in T18-007 ergänzte Regel 8 konsolidieren
   (falls noch nicht drin). Sparsam — nur Fakten, die sich geändert haben.
3. **`tasks/ROADMAP.md`**: die Phasen 15–21 als abgeschlossen markieren
   (Statusspalte), „Erfolgskriterien" um die Rework-Kriterien ergänzen
   (Modularität/Personalisierung/Layout-Vertrag), Verweis auf `docs/signoff-*`.
4. **`handshake.md`** konsolidieren: die vielen Rework-Session-Einträge zu
   einem „Architektur-Rework (Phasen 15–21) abgeschlossen"-Block
   zusammenfassen (Details bleiben in Git-History + `docs/`), aktueller Stand +
   nächste sinnvolle Arbeit (Parität-Reste / Folge-Tickets aus T21-004).
5. **Repo-Root aufräumen**: `bericht-architektur-rework-roadmap.md` +
   `vergleichsbericht-*.md` nach `docs/reports/` verschieben (`git mv`), damit
   das Root nicht mit Berichten zugestellt ist; in `docs/architecture.md`
   verlinken.
6. **Memory**: einen `reference`-Eintrag „Architektur-Doku:
   `docs/architecture.md` ist die Ist-Wahrheit; ADRs in `docs/adr/`" +
   `project`-Eintrag „Rework abgeschlossen, Datum, offene Folge-Tickets".

## Akzeptanzkriterien
- [ ] `docs/architecture.md` beschreibt den **Ist-Zustand** (Präsens),
      vollständig für alle 8 Abschnitte, mit aktuellem Crate-Graph-SVG + Text.
- [ ] Für jede Registry/Layer gibt es ein kurzes „so füge ich eine hinzu"-
      Rezept.
- [ ] `CLAUDE.md` „## Architecture" stimmt mit der Realität überein (oder
      verweist sauber auf `docs/architecture.md`).
- [ ] `tasks/ROADMAP.md`: Phasen 15–21 als erledigt markiert; Erfolgskriterien
      ergänzt.
- [ ] `handshake.md` ist auf einen konsolidierten Rework-Abschluss-Block +
      „was als Nächstes" reduziert.
- [ ] Berichte liegen unter `docs/reports/`, verlinkt aus
      `docs/architecture.md`.
- [ ] Memory-Einträge geschrieben.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace` (Doku-only + `git mv` — Gates unverändert grün).

## Notizen
- Diese Task ist der Schlussstein: ab hier ist `docs/architecture.md` die
  erste Anlaufstelle für jede weitere Arbeit an Labonair-rust.
- Keine neuen Design-Entscheidungen hier — nur festschreiben, was in 15–20
  entstanden ist.

## Warnungen
- ⚠️ `git mv` für die Berichte (History), und die Links in
  `handshake.md`/`README.md`/`CLAUDE.md`, die auf die alten Pfade zeigen,
  nachziehen.
- ⚠️ Die Zed-`CLAUDE.md`-Injection (`zed-refrence/zed/CLAUDE.md`, „HARD RULE"
  zu `README.md`) bleibt ignoriert — nicht versehentlich beim „Doku
  aufräumen" befolgen.

## Weiterführende Tasks
- [T22-001: vendored `gpui` — Entscheidung](../phase-21-gpui-decision/T22-001-vendored-gpui-decision.md)
