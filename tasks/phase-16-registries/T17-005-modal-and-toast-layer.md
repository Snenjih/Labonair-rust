# T17-005: `ModalLayer` + `ToastLayer`

## Status
📋 Geplant

## Phase
16 — Root-Objekt & Registries

## Abhängigkeiten
T16-006 (`labonair-workspace`), T16-003 (`labonair-notifications`), T16-004 (`labonair-command-palette`)

## Ziel
Die heute in `AppShell::render` einzeln zusammengesteckten Overlays
(Command-Palette, Bookmarks, Updater, Bar-Menüs, Breadcrumb-Menüs, Toasts) durch
zwei wiederverwendbare Layer nach Zed-Vorbild ersetzen: `ModalLayer` (ein
aktiver, fokus-fangender Modal) und `ToastLayer` (gestapelte, nicht-blockierende
Toasts). Sie sind Teil des `Workspace`/der Shell-Komposition — der Layout-Vertrag
nennt sie als einzige „Overlay-Ebene".

## Kontext
- Heute: `crates/shell/src/app_shell.rs::render` — am Ende:
  `.child(self.command_palette.clone()) .child(self.bookmarks.clone())
  .child(self.updater.clone()) .children(bar_menu) .children(crumb_menu)
  .children(subdir_menu) .children(toasts)`. Jedes ist ein handverdrahtetes
  `deferred`/`anchored` Konstrukt. Palette-/Bookmark-Events werden über
  `pending_commands`/`pending_bookmarks`-Vec + `drain_*` im nächsten Render
  verarbeitet.
- Zed-Vorbild:
  `zed-refrence/zed/crates/workspace/src/modal_layer.rs` — `ModalLayer`,
  `trait ModalView: Focusable + Render`, `Workspace::toggle_modal`,
  `dismiss_on_focus_lost`, Fokus-Trap.
  `zed-refrence/zed/crates/workspace/src/toast_layer.rs` — `ToastLayer`,
  gestapelte Toasts mit Auto-Dismiss.
- `labonair-notifications::render_overlay` liefert heute die Toasts.

## Anweisungen zur Umsetzung
1. **`crates/workspace/src/modal_layer.rs`**:
   - `trait ModalView: Focusable + Render { fn on_dismiss(&mut self, cx); }`.
   - `struct ModalLayer { active: Option<AnyModalHandle> }` als Entity.
   - `Workspace::toggle_modal::<M>(build: impl FnOnce(&mut Window, &mut Context<M>) -> M)`
     — öffnet/ersetzt den Modal, fängt Fokus, `Esc`/Klick-außerhalb/
     Fokusverlust schließt (konfigurierbar).
   - `render` — zentriertes/angepinntes Panel mit halbtransparentem Backdrop.
2. **`crates/workspace/src/toast_layer.rs`**:
   - `struct ToastLayer` — hält die aktive Toast-Liste (oder delegiert an
     `labonair-notifications::NotificationCenter` und rendert nur).
   - Positionierung, Stapelung, Auto-Dismiss-Timer (GPUI-Executor-Timer, **kein**
     `std::thread::sleep`).
3. **Migrationen**:
   - **Command-Palette** → `ModalView`. `Workspace::toggle_command_palette`
     ruft `toggle_modal`. Der `pending_commands`-Vec + `drain_pending_commands`
     entfällt: die Palette emittet `PaletteEvent`, `Workspace` (bzw. der
     `CommandRegistry` in T17-007) verarbeitet es direkt via `cx.subscribe`
     mit `&mut Window` (`cx.subscribe_in`).
   - **Bookmarks** → `ModalView`; `pending_bookmarks` + `drain` entfällt analog.
   - **Bar-Menüs / Breadcrumb-Segment-Menü / Subdir-Dropdown** → das sind
     Kontextmenüs, keine Modals. Sie ziehen mit dem jeweiligen `StatusItem`
     (T17-003) um bzw. nutzen ein `PopoverMenu`-Primitive (T20-001). Nicht in
     den `ModalLayer` zwingen.
   - **Updater**-Dialog → `ModalView` (die „Update verfügbar"-Karte);
     der Statusbar-Updater-Indicator bleibt ein `StatusItem`.
   - **Toasts** → `ToastLayer`.
4. **`AppShell::render`**: die sechs `.child(...)`/`.children(...)`-Overlay-
   Zeilen werden zu genau zwei: `.child(modal_layer) .child(toast_layer)`.
5. **`drain_pending_*` entfernen**: nach dieser Task sollen
   `drain_pending_commands` und `drain_pending_bookmarks` weg sein
   (`drain_pending_ai` folgt in T17-007/T17-006, `sync_live_bridge` in T17-006).
6. `cargo run`: `Cmd+Shift+P` Palette (öffnen, tippen, Enter, `Esc`);
   Path-Bookmarks-Modal; „Update verfügbar"-Dialog; mehrere Toasts gestapelt
   mit Auto-Dismiss; Klick außerhalb schließt den Modal.

## Akzeptanzkriterien
- [ ] `ModalLayer` + `ToastLayer` existieren in `labonair-workspace`, mit
      `trait ModalView`.
- [ ] Command-Palette und Bookmarks laufen als `ModalView`; `pending_commands`,
      `pending_bookmarks`, `drain_pending_commands`, `drain_pending_bookmarks`
      sind entfernt.
- [ ] `AppShell::render` endet mit genau zwei Overlay-Kindern
      (`modal_layer`, `toast_layer`).
- [ ] Fokus-Trap: bei offenem Modal gehen Tastatureingaben an den Modal;
      `Esc` und Klick-außerhalb schließen ihn.
- [ ] Toasts stapeln, auto-dismissen über GPUI-Timer (kein Thread-Sleep).
- [ ] `cargo run`: alle migrierten Overlays funktionieren; Kontextmenüs
      (Bar/Breadcrumb) weiterhin korrekt (jetzt über Popover/StatusItem).
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- `ModalLayer` hält **einen** aktiven Modal (wie Zed). Kein Modal-Stack.
- Die transiente `Cmd+F`-Suche (T18-002) wird ebenfalls ein `ModalView`
  (oder ein leichtes Overlay) — deshalb die API generisch halten.

## Warnungen
- ⚠️ `cx.subscribe_in` braucht den `Window`. Sicherstellen, dass die
  Subscriptions dort aufgesetzt werden, wo ein `&mut Window` vorliegt
  (`Workspace::new(..., window, cx)`), sonst ist man wieder beim
  `pending_*`-Puffer.
- ⚠️ Backdrop-Klick darf nicht an das darunterliegende UI durchschlagen
  (`on_mouse_down` am Backdrop mit `cx.stop_propagation()`).

## Weiterführende Tasks
- [T17-006: `AppShell` → reine Komposition](./T17-006-appshell-composition-only.md)
- [T18-002: Suche als Overlay](../phase-17-layout/T18-002-search-overlay.md)
