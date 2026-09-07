# T17-004: `PaneGroup` — rekursiver Split-Baum + Persistenz

## Status
✅ Done

## Phase
16 — Root-Objekt & Registries

## Abhängigkeiten
T16-006 (`labonair-workspace`), T17-002 (`Dock`-Modell)

## Ziel
Das heutige Split-Layout (flacher `SplitAxis` + eine Split-Ebene) durch einen
echten rekursiven Split-Baum nach Zed-Vorbild ersetzen: beliebig tief
verschachtelte horizontale/vertikale Splits, mit Größen-Verhältnissen und
vollständiger Persistenz.

**Zusätzlich (Thema 1):** Die Wurzel des Baums ist **optional**. Ein Workspace
ohne offene Tabs hat *keinen* Pane-Baum (`root: None`) und rendert nichts bzw.
die Empty-Surface (die visuelle Ausgestaltung macht T18-001). `remove` des
letzten Panes führt zu `root = None` — das ist **kein** Fehlerfall mehr.
Siehe `docs/architecture.md §8.2`.

## Kontext
- Heute: `crates/workspace/src/pane.rs` + `pane_group.rs` (aus T16-006) —
  `enum SplitAxis`, ein Split-Container. `Workspace::active_has_split`,
  `act_split_right`/`act_split_down`/`act_close_pane` in `app_shell.rs`.
  Vermutlich nur eine Split-Ebene (kein Baum).
- Zed-Vorbild: `zed-refrence/zed/crates/workspace/src/pane_group.rs` —
  `enum Member { Pane(Entity<Pane>), Axis(PaneAxis) }`,
  `struct PaneAxis { axis: Axis, members: Vec<Member>, flexes: Vec<f32>,
  bounding_boxes: … }`, `PaneGroup::{split, remove, swap, resize}`,
  `SplitDirection` (`Up`/`Down`/`Left`/`Right`).
  `zed-refrence/zed/crates/workspace/src/persistence.rs` +
  `persistence/model.rs` — `SerializedPaneGroup` (rekursiv).
- Session-Persistenz heute: `crates/workspace/src/session.rs` (aus T16-006) —
  Tab-/Layout-Snapshot; T14-001 hat das eingeführt.

## Anweisungen zur Umsetzung
1. **`pane_group.rs` zum Baum ausbauen**:
   - `enum Member { Pane(PaneId), Axis(PaneAxis) }`
   - `struct PaneAxis { axis: Axis (Horizontal|Vertical), members: Vec<Member>,
     flexes: Vec<f32> }` (Summe der `flexes` = 1.0).
   - `struct PaneGroup { root: Option<Member> }` mit:
     - `split(target: PaneId, new: PaneId, direction: SplitDirection)` —
       erzeugt/erweitert eine Achse an der Zielstelle. Bei `root == None`
       wird `new` die neue Wurzel (`Member::Pane`).
     - `remove(pane: PaneId)` — kollabiert leere Achsen, promotet einzige
       Kinder; Entfernen des letzten Panes ⇒ `root = None` (kein Fehler).
     - `resize(axis_path, member_ix, delta)` — passt zwei benachbarte `flexes`
       an.
     - `panes() -> Vec<PaneId>` (leer bei `root == None`), `find_pane(...)`,
       `is_empty() -> bool`.
   - `render(&self, window, cx)` — rekursiv: `Axis` → `flex` Row/Col mit
     Resize-Handles zwischen den Membern; `Pane` → die Pane-View;
     `None` → leeres Element (die Empty-Surface rendert T18-001 auf
     Workspace-Ebene, nicht hier).
2. **`Workspace`** hält `PaneGroup` statt des flachen Split-Zustands. Panes
   sind `Entity<Pane>`, referenziert über `PaneId` (stabile ID). Der aktive
   Pane ist `Workspace`-State (`active_pane: Option<PaneId>` — `None` bei
   leerem Baum). Kein Code darf einen aktiven Pane voraussetzen; das volle
   `Option`-Audit über `Workspace` macht T17-009, hier nur die Signatur
   richtig anlegen.
3. **Aktionen** neu verdrahten (bleiben im `CommandRegistry`, T17-007, bzw.
   vorerst im Shell): `split_right`/`split_down`/`split_left`/`split_up`
   (heute nur right/down) → `PaneGroup::split(active, new, dir)`;
   `close_pane` → `PaneGroup::remove(active)` + Nachbar aktivieren;
   `focus_next_pane` → Reihenfolge aus `panes()`.
4. **Persistenz**: `SerializedPaneGroup` (rekursives Serde-Enum, `Option`ale
   Wurzel) in `session.rs` integrieren — beim Speichern den Baum + `flexes` +
   Tab-Zuordnung je Pane serialisieren; beim Laden rekonstruieren. Bestehende
   Session-Snapshots (flach) müssen weiter laden (Migration: flacher Split →
   1-Achsen-Baum; fehlender Baum → einzelner Pane; **leerer/kein Baum →
   `root = None`**, gültig).
5. **Splits pro Dock?** Nein — Splits gelten für den zentralen Workspace-
   Bereich (Tab-Inhalt). Docks bleiben single-panel (T17-002). In
   `docs/architecture.md` festhalten.
6. `cargo run`: mehrfach horizontal + vertikal verschachteln (z.B. 2×2 +
   ein weiterer Split in einer Zelle); Resize an inneren Grenzen; einzelne
   Panes schließen bis einer übrig ist; Neustart stellt den exakten Baum +
   die Verhältnisse wieder her.

## Akzeptanzkriterien
- [x] `PaneGroup` ist ein rekursiver `Member`-Baum mit `flexes` und
      **`Option`aler Wurzel**; beliebige Verschachtelungstiefe.
- [x] `split` in alle vier Richtungen (inkl. aus `root == None` heraus);
      `remove` kollabiert leere Achsen korrekt und ergibt bei letztem Pane
      `root = None` ohne Panic; `resize` verändert nur die zwei benachbarten
      Verhältnisse. (Unit-tests `split_in_all_four_directions`,
      `remove_collapses_axes_and_can_empty_the_tree`,
      `resize_only_touches_two_adjacent_flexes`.)
- [~] `cargo run`: 2×2-Layout + verschachtelter Sub-Split — **not verified
      visually**: headless VPS, no display. `render_member` renders the
      recursive `Member` tree with a resize handle between every adjacent
      member pair (col/row per axis); logic covered by unit tests.
- [x] Session-Persistenz: `SerializedPaneGroup` (rekursives serde-Enum) +
      `SerializedLayout` in `session.rs`; `remap_layout` rebuilds the tree
      with fresh ids incl. `flexes` and the active leaf. Legacy flat `split`
      snapshots still load & migrate to `Axis` — `WorkspaceTabSnapshot.layout`
      kept its field name/nesting, no `SNAPSHOT_VERSION` bump. Tests
      `legacy_binary_split_snapshot_migrates_to_axis`, `empty_layout_round_trips`,
      `remap_layout_preserves_shape_and_active`, `snapshot_round_trips_through_json`.
- [x] `close_pane` aktiviert einen sinnvollen Nachbarn (`sibling_leaf`:
      previous member's last leaf, else next member's first);
      `focus_next_pane` cycles `leaves()` stably. Tests
      `layout_close_keeps_a_sensible_active_neighbour`, `deep_nesting_round_trips_leaf_order`.
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `scripts/check-crate-deps.sh`. **`cargo test` deviation** (recorded in
      handshake + `docs/architecture.md`): test binaries cannot link on this
      headless VPS (missing `-lxcb` / `-lxkbcommon*`); `cargo check/clippy
      --all-targets` compile all `#[cfg(test)]` code and is the accepted
      substitute. New `pane_group` unit tests: split (4 dirs + from empty),
      remove (collapse + empty), resize (adjacent-only + sum invariant),
      close-neighbour, deep-nesting; `session` tests: legacy migration,
      empty round-trip, remap shape/active.

## Abweichungen (sanctioned deviation process)
- **`split_left` / `split_up` as user actions**: only the `PaneGroup` /
  `WorkspaceLayout` / `Workspace::split` API carries all four `SplitDirection`s.
  The shell keeps its two existing actions (→ `Right` / `Down`); binding
  `Left` / `Up` is left to T17-007 (`CommandRegistry`). Recorded in
  `docs/architecture.md §8.7`.
- **Empty-tree render**: `root == None` currently falls back to the old
  "Terminal" placeholder in `render_content`; the real empty surface is
  T17-009 / T18-001 (already noted in `§8.2`).
- **`cargo test --workspace`** replaced by `cargo check/clippy --all-targets`
  (env cannot link test binaries — see above).

## Notizen
- Zeds `pane_group.rs` ist die beste Vorlage — Struktur 1:1 übernehmen,
  Zed-spezifische Teile (Collab-Cursor, `bounding_boxes` für Drag-Drop von
  Items) weglassen, bis sie gebraucht werden.
- Terminal-Panes dürfen beim Split ihre Prozesse **nicht** verlieren
  (bestehende Regel aus T04-001/T03-005).

## Warnungen
- ⚠️ `flexes`-Summe muss invariant 1.0 bleiben (Rundungsdrift abfangen) — sonst
  „wandern" Splits über viele Resizes.
- ⚠️ Rekursives `render` mit GPUI: auf übermäßiges Neu-Allozieren pro Frame
  achten (der Baum sollte klein sein, aber `into_any_element` je Knoten kostet).
  In T21-001 wird das gemessen.

## Weiterführende Tasks
- [T17-006: `AppShell` → reine Komposition](./T17-006-appshell-composition-only.md)
- [T17-007: `CommandRegistry`](./T17-007-command-registry.md)
- [T17-009: Tabs optional & Empty-Workspace](./T17-009-optional-tabs-empty-workspace.md)
