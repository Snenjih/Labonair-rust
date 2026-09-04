# T17-005: `ModalLayer` + `ToastLayer`

## Status
✅ Done

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
- [x] `ModalLayer` + `ToastLayer` existieren in `labonair-workspace`, mit
      `trait ModalView` (`crates/workspace/src/modal_layer.rs`,
      `crates/workspace/src/toast_layer.rs`).
- [x] Command-Palette und Bookmarks laufen als `ModalView` — über
      shell-lokale Wrapper-Newtypes (`CommandPaletteModal`, `BookmarksModal`,
      `UpdaterModal` in `app_shell.rs`), weil `labonair-command-palette` /
      `labonair-panel-explorer` nicht auf `labonair-workspace` zeigen dürfen
      (Zyklus) und die Orphan-Rule `impl ModalView` woanders verbietet.
      `pending_commands`, `pending_bookmarks`, `drain_pending_commands`,
      `drain_pending_bookmarks` sind entfernt; Picks laufen jetzt direkt über
      `cx.subscribe_in` (in `AppShell::new` aufgesetzt).
- [x] `AppShell::render` endet mit genau zwei Overlay-Kindern
      (`.child(self.modal_layer.clone()).child(self.toast_layer.clone())`).
- [x] Fokus-Trap: `ModalLayer` fokussiert beim Öffnen den Modal-Focus-Handle
      (`cx.defer_in` → `window.focus`), setzt `on_focus_out` +
      `dismiss_on_focus_lost` auf. Für die drei `render_bare`-Modals liefern
      die Views selbst Scrim + `Esc` + Overlay-Klick (Palette hatte beides,
      Bookmarks-Overlay-Klick in T17-005 ergänzt). Der generische
      Non-Bare-Pfad (Backdrop + `on_mouse_down`→`hide_modal` + zentriertes
      Panel) ist implementiert für künftige Modals (T18-002). Visuelle
      `cargo run`-Prüfung: siehe unten.
- [x] Toasts stapeln + auto-dismissen — via
      `cx.background_executor().timer()` in `NotificationCenter` (schon vor
      T17-005 vorhanden); `ToastLayer` beobachtet + rendert nur.
- [~] `cargo run`: **auf diesem Headless-VPS nicht möglich** (kein Display).
      Kontextmenüs (Bar/Breadcrumb) nicht angefasst — bleiben `StatusItem`/
      `PopoverMenu`-lokal wie vom Task gefordert.
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `scripts/check-crate-deps.sh` (20 Crates, 87 Kanten, azyklisch, keine
      neue Kante). `cargo test --workspace` kann auf diesem VPS keine
      Test-Binaries linken (fehlende X11-Dev-Libs); `cargo check/clippy
      --all-targets` kompiliert alle `#[cfg(test)]`-Module — der im Projekt
      akzeptierte Ersatz.

## Abweichungen (T17-005)
Festgehalten in `docs/architecture.md §8.8`:
1. Command-Palette / Path-Bookmarks / Updater-Dialog behalten ihren eigenen
   Scrim + Positionierung (`ModalView::render_bare() == true`); der
   `ModalLayer` hostet sie für Lifecycle + Fokus, rendert sie unverändert.
   Der generische Backdrop-Pfad im `ModalLayer` bleibt für neue Modals.
2. `impl ModalView` sitzt auf shell-lokalen Wrapper-Newtypes, nicht auf den
   View-Typen selbst (Zyklus + Orphan-Rule).
3. Updater + Bookmarks sind „driven" Modals (`sync_updater_modal` /
   `sync_bookmarks_modal` in `render` spiegeln `dialog_open` / `is_open`);
   nur die Palette nutzt `open_modal` / `hide_modal` direkt.

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
