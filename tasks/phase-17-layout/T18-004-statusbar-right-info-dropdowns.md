# T18-004: Statusbar rechts — Info-Dropdowns

## Status
📋 Geplant

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
- [ ] Alle Info-Items rendern rechts in einer festen, dokumentierten
      Default-Reihenfolge (via `order`).
- [ ] Einheitliches Dropdown-/Popover-Muster (Anker, Größe, Schließen-
      Verhalten) über alle Items.
- [ ] Notifications: Badge zählt ungelesen, Dropdown-Liste, „Alle löschen".
- [ ] CWD-Breadcrumb: Segmente + kollabierte Mitte + Segment-Menü +
      Subdir-Dropdown; folgt Tab-Wechsel.
- [ ] Transfers-Item nur bei Aktivität sichtbar/aktiv; Fortschritt im Dropdown.
- [ ] Updater-Zustände korrekt; Klick öffnet Updater-Modal.
- [ ] Agent-Access: Grant-Zähler + Revoke im Dropdown.
- [ ] Divider nur zwischen Gruppen.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

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
