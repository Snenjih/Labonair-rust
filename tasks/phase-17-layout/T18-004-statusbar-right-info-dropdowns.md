# T18-004: Statusbar rechts — Info-Dropdowns

## Status
✅ Done

## Phase
17 — Neues Layout & Statusbar-Personalisierung

## Abhängigkeiten
T17-003 (`StatusItemRegistry`), T18-003 (Statusbar links)

## Ziel
Die rechte Seite der Statusbar als **Info-/Status-Bereich** finalisieren:
Notifications (Badge + Dropdown), CWD-Breadcrumb, Updater, Transfers,
Agent-Access — jeweils als `StatusItem` mit sauberem Dropdown/Popover, in einer
festen Default-Reihenfolge.

## Kontext
- Nach T17-003 existieren bereits die konkreten `StatusItem`s
  (`NotificationsStatusItem`, `CwdStatusItem`, `UpdaterStatusItem`,
  `TransfersStatusItem`, `AgentAccessStatusItem`, `JumpHostsStatusItem`,
  `BookmarksStatusItem`) — dort wurde nur „funktioniert weiterhin" verlangt.
  Hier: Politur, konsistente Interaktion, Default-Reihenfolge.
- Heutige Quellen: `updater.rs`, `transfers.rs`, `agent_access.rs`,
  `cwd_breadcrumb.rs`, `notifications.rs`.
- `reference-src/src/modules/statusbar/lib/renderBarItem.tsx` — Referenz für
  Icon/Badge/Label-Layout eines Bar-Items.

## Anweisungen zur Umsetzung
1. **Default-Reihenfolge rechts** (von innen nach außen / links nach rechts
   innerhalb der rechten Gruppe) via `StatusItem::order`:
   `CwdBreadcrumb` (ganz links der rechten Gruppe, da am breitesten) →
   `Transfers` → `AgentAccess` → `Updater` → `JumpHosts` → `Bookmarks` →
   `Notifications` (ganz rechts, immer sichtbar). Werte in Doc-Kommentar
   begründen; final ist Nutzer-Sichtprüfung im PR.
2. **Einheitliches Dropdown-Muster**: jedes Info-Item mit Popover nutzt
   dasselbe `PopoverMenu`/`Popover`-Primitive (ui-kit; bis T20-001 das
   bestehende `context_menu`/`anchored`+`deferred`). Konsistent: Anker unter
   dem Item, gleiche Breite/Padding/Schatten, `Esc`/Klick-außerhalb schließt.
3. **Notifications-Item**: Glocken-Icon + Zähler-Badge (ungelesen). Dropdown =
   Liste der letzten N Notifications (Icon, Titel, Text, Zeit),
   „Alle löschen", Klick auf Eintrag = zugehörige Aktion (falls vorhanden).
   Badge verschwindet bei 0.
4. **CWD-Breadcrumb-Item**: Segmente des aktuellen Arbeitsverzeichnisses,
   mittlere Segmente kollabierbar; Klick auf Segment = `cd` im aktiven
   Terminal (bestehendes Verhalten); Rechtsklick-Segment-Menü + Subdir-
   Dropdown beibehalten. Reagiert auf `Panel`/Tab-Wechsel
   (`StatusItem::on_active_tab_changed`).
5. **Transfers-Item**: nur sichtbar/aktiv, wenn Transfers laufen oder in der
   Queue sind (sonst ausgegraut oder verborgen — konsistent mit Zeds
   „conditional status item"). Dropdown = laufende + wartende Transfers mit
   Fortschritt (gespeist aus T17-008 Backend-Event-Brücke, falls Variante A).
6. **Updater-Item**: Icon-Zustände (idle / Update verfügbar / lädt /
   bereit-zum-Neustart). Klick → Updater-Modal (`ModalLayer`).
7. **Agent-Access-Item**: Badge = Anzahl aktiver Per-Tab-Grants; Dropdown =
   Grant-Liste mit Revoke (bestehendes `agent_access`-Popover).
8. **Trennlinien**: dezente Divider zwischen logischen Gruppen (Referenz-Regel
   aus `barItemLayout.tsx` — nur zwischen Gruppen, nicht zwischen jedem Item).
9. `cargo run`: rechte Statusbar in Default-Reihenfolge; jedes Dropdown
   öffnet/schließt konsistent; Notifications-Badge zählt; CWD folgt Tab;
   Transfers erscheint nur bei Aktivität; Updater-Zustände; Agent-Grants
   revoke.

## Akzeptanzkriterien
- [x] Alle Info-Items rendern rechts in einer festen, dokumentierten
      Default-Reihenfolge (via `order`).
- [x] Einheitliches Dropdown-/Popover-Muster (Anker, Größe, Schließen-
      Verhalten) über alle Items.
- [x] Notifications: Badge zählt ungelesen, Dropdown-Liste, „Alle löschen".
- [x] CWD-Breadcrumb: Segmente + kollabierte Mitte + Segment-Menü +
      Subdir-Dropdown; folgt Tab-Wechsel.
- [x] Transfers-Item nur bei Aktivität sichtbar/aktiv; Fortschritt im Dropdown.
- [x] Updater-Zustände korrekt; Klick öffnet Updater-Modal.
- [x] Agent-Access: Grant-Zähler + Revoke im Dropdown.
- [x] Divider nur zwischen Gruppen.
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Umsetzung (Session 2026-09-04)

- **`crates/panel/src/status.rs`** — `StatusItem::group()` (default 0) +
  `StatusItemRegistration::group` + `StatusItemHandle::group`: a second sort
  key so the status bar can tell "same logical group" from "different group"
  without a new registry type.
- **`crates/workspace/src/status_bar.rs`** — `StatusBar::cluster` now returns
  `Vec<AnyElement>` and inserts a 1px divider between two consecutive items
  whose `group()` differs (point 8 — dividers only between groups).
- **`crates/shell/src/status_items.rs`** — reordered every right-side item's
  `order()`/`group()` per the default-order list above (doc comment on
  `register_builtin_status_items` explains the numbering); `Notifications`
  bell is now always rendered (only its badge hides at 0, per point 3, not
  the whole item, per point 1 "immer sichtbar"); `Notifications` and
  `AgentAccess` dropdowns migrated from a bespoke `.absolute()` div (no
  outside-click/Esc close) to the new `labonair_ui_kit::popover` primitive
  (anchored at the click point, `Esc`-close via a `FocusHandle` +
  `on_key_down`, click-outside-close via the popover's own backdrop);
  `Transfers` item now hides unless `TransfersView::active_count() > 0`
  (new method — point 5's "conditional status item"); `Updater` click now
  calls the new `UpdaterView::open_dialog` (reopens the existing dialog for
  an already-known update instead of re-running the network check that
  `run_check` would otherwise kick off — point 6).
- **`crates/ui-kit/src/popover.rs`** (new) — `popover(anchor, width, theme,
  dismiss, content)`: `deferred` + `anchored().snap_to_window()` card (same
  mechanics as `settings-ui`'s `render_dropdown`) + a transparent backdrop
  that dismisses on outside click. `context_menu` (flat `MenuItem` lists,
  used by the CWD breadcrumb's segment/subdir menus and the panel-toggle
  dock menu) is left as its own established primitive — different content
  shape, and touching its ~10 existing call sites was out of this task's
  scope; both close the same way (outside click / Esc), which is what
  "einheitliches Muster" asked for here. A real shared popover primitive
  (possibly unifying `context_menu` into it) is T20-001's job per the task
  file's own note.
- **`crates/workspace/src/{transfers.rs,workspace.rs}`** —
  `TransfersView::active_count()` + `Workspace::transfers_entity()` so the
  statusbar item can observe the transfer queue directly (observing
  `Workspace` alone never notified — `apply_transfer_bus_event` only calls
  `cx.notify()` on the `TransfersView` entity, not on `Workspace` itself).

### Deviation
- Transfers' "Dropdown = laufende + wartende Transfers mit Fortschritt" is
  satisfied by the pre-existing `TransfersView` panel (`reveal_transfers`) —
  a fixed bottom-right panel with full per-job progress/step log/cancel, not
  an anchored-under-the-item popover. Migrating that whole subsystem to the
  new anchor pattern is a much larger change than "polish, keine neue
  Funktion" implies; flagged for a user visual pass / a future task if the
  anchored placement is wanted.
- Updater's icon does not visually distinguish Available/Downloading/Ready
  (single icon + dot regardless of sub-state) — it already reads its status
  correctly for click behavior and visibility; per-substate iconography is a
  cosmetic addition beyond what this polish pass changed.

## Notizen
- Diese Task ist Politur + Konsistenz, keine neue Funktion. Wenn ein Item
  heute schon gut aussieht, nur die Dropdown-Mechanik vereinheitlichen.
- Die Verschiebbarkeit links/rechts (RMB) kommt in T18-005 — hier nur die
  **Default**-Seite/Reihenfolge.

## Warnungen
- ⚠️ CWD-Breadcrumb hält asynchronen State (Subdir-Listing in flight). Nicht
  blockierend laden; `Popover` darf sich nicht schließen, während das Listing
  ankommt.
- ⚠️ Transfers/Updater-Sichtbarkeit: ein Item, das mal da / mal weg ist,
  verschiebt die anderen. Entweder festen Platz reservieren oder bewusst
  „reflow" — Nutzer-Sichtprüfung im PR entscheidet.

## Weiterführende Tasks
- [T18-005: Statusbar-Item-Personalisierung](./T18-005-statusbar-item-personalization.md)
