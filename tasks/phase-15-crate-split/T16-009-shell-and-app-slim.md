# T16-009: `labonair-shell` + `labonair-app` schlank

## Status
📋 Geplant

## Phase
15 — Crate-Zerlegung & Fundament

## Abhängigkeiten
T16-006 (`labonair-workspace`), T16-007 (`labonair-settings-ui`), T16-008 (Panel-Crates)

## Ziel
Den `AppShell` in einen eigenen dünnen Crate `labonair-shell` ziehen und
`crates/ui` auflösen (oder auf eine reine Fassade reduzieren). `crates/app`
bleibt nur noch Bootstrap. Nach dieser Task ist der `ui`-Monolith Geschichte;
`app_shell.rs` ist zwar noch groß (Umbau folgt in T17-006), lebt aber isoliert.

## Kontext
- Heute: `crates/ui/src/app_shell.rs` (2 983 Z.), plus verbleibende
  `crates/ui/src/`-Dateien nach T16-002…008: `lib.rs`, `assets.rs`,
  `menu.rs`, `theme.rs` (`ThemeStore`), `syntax_theme.rs`, `background.rs`,
  `bar_items.rs`, `sidebar_slot.rs`, `cwd_breadcrumb.rs`, `window_state.rs`,
  `updater.rs`, `transfers.rs`, `bell.rs`, `markdown.rs`, `tabs.rs`.
- `crates/ui/src/lib.rs` exportiert heute `Assets`, `AppShell`, `init_fonts`,
  `init_theme`, `init_background`, `init_notifications`, `init_menus`,
  `window_state`.
- `crates/app/src/main.rs` (128 Z.) — Tokio-Runtime, Backend-Init,
  `Application::new().with_assets(labonair_ui::Assets).run(...)`,
  `cx.open_window(...)`, `AppShell::new(...)`, `Root::new(...)`, `init_menus`.
- Zed-Vorbild: `zed-refrence/zed/crates/zed/src/main.rs` (dünner Bin) +
  `zed-refrence/zed/crates/workspace` + `zed-refrence/zed/crates/title_bar`.

## Anweisungen zur Umsetzung
1. **`crates/shell/` anlegen** (`labonair-shell`, `src/shell.rs` Lib-Root).
2. Verschieben nach `labonair-shell`:
   - `app_shell.rs` → `src/app_shell.rs` (unverändert — Umbau ist T17-006).
   - `menu.rs` → `src/menu.rs` (native macOS-Menüs + `apply_keybinds`).
   - `bar_items.rs`, `sidebar_slot.rs`, `cwd_breadcrumb.rs` → `src/` (werden
     in Phase 17 abgelöst/umgebaut, aber gehören zur Shell-Komposition).
   - `updater.rs`, `transfers.rs`, `bell.rs` → `src/` (Shell-nahe Overlays;
     ziehen in Phase 17 ggf. in Statusbar-Items).
   - `window_state.rs` → `src/window_state.rs`.
   - `assets.rs` + Asset-Ordner-Verweis → `src/assets.rs`.
3. **Theme-Heimat entscheiden**: `theme.rs` (`ThemeStore`, `ThemePreference`,
   `GlobalTheme`, `init_theme`), `syntax_theme.rs`, `background.rs` — diese
   sind von vielen Crates genutzt. Optionen:
   a) in `labonair-theme` ziehen (bevorzugt, macht `theme` zum vollen
      Theme-Crate — passt zu Phase 19), oder
   b) einen `labonair-theme-store`-Crate.
   Entscheidung in `docs/architecture.md` nachziehen; hier umsetzen. `ui-kit`
   und alle Panels hängen dann an der neuen Theme-Heimat.
4. **`markdown.rs`, `tabs.rs`**: `markdown` (Renderer) → `labonair-ui-kit`
   (wiederverwendbares Primitive) **oder** `labonair-workspace::views`
   (nur vom Preview-Tab genutzt) — nach realer Nutzung entscheiden.
   `tabs.rs` (Tab-Leisten-UI) → `labonair-workspace` (gehört zum Tab-System)
   **oder** `labonair-shell` (Titlebar-Komposition, Layout-Vertrag). Default:
   Tab-Leiste zur Shell (die Titlebar besitzt sie), Tab-*Store* zum Workspace.
5. **`crates/ui` auflösen**: wenn nach 1–4 nichts mehr übrig ist, den Crate
   entfernen (Member raus, Verzeichnis löschen). Bleibt ein Rest, wird
   `crates/ui` eine reine Re-Export-Fassade mit `#[deprecated]`-Hinweis im
   `lib.rs` und einem TODO, sie in der nächsten Task ganz zu tilgen.
6. **`crates/app/src/main.rs`** anpassen: `labonair_ui::` → `labonair_shell::`
   bzw. die neuen Crate-Pfade (`labonair_shell::AppShell`,
   `labonair_shell::Assets`, `labonair_theme::init_theme`,
   `labonair_shell::init_menus`, …). `main.rs` bleibt inhaltlich gleich
   (kein Logik-Umbau).
7. `cargo run`: vollständige App startet identisch — Fenster, Menüleiste,
   Header, Statusbar, Sidebar, alle Panels, Settings-Fenster.

## Akzeptanzkriterien
- [ ] `crates/shell/` existiert; `AppShell` + `menu` + Shell-nahe Overlays
      leben dort.
- [ ] `crates/ui` ist entweder gelöscht **oder** eine leere, als
      `#[deprecated]` markierte Fassade ohne eigene Logik.
- [ ] Die Theme-Store-Heimat ist entschieden, umgesetzt und in
      `docs/architecture.md` dokumentiert; alle Nutzer importieren von dort.
- [ ] `crates/app/src/main.rs` kompiliert nur mit geänderten Import-Pfaden,
      keine Logikzeile geändert.
- [ ] `cargo run`: App startet und verhält sich identisch zu vor Phase 15
      (End-to-End-Sichtprüfung: Fenster, Menüs, alle Panels, Settings,
      Session-Restore).
- [ ] `cargo tree -p labonair-app` zeigt den erwarteten Graphen aus
      `docs/architecture.md` (kein `labonair-ui` mehr, außer als deprecated
      Fassade).
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Diese Task darf mehrere Commits umfassen (Theme-Heimat, Shell-Move,
  ui-Auflösung, main.rs) — Gate nach jedem grün halten.
- `app_shell.rs` bleibt bewusst groß und „hässlich". Es geht hier nur um die
  Isolation; die Diät ist T17-006.

## Warnungen
- ⚠️ Asset-Pfade: `with_assets(Assets)` + `include_dir!`/`RustEmbed`-artige
  Makros haben oft relative Pfade zum Crate-Root. Beim Verschieben von
  `assets.rs` die Pfade prüfen (`cargo run` bricht sonst erst zur Laufzeit).
- ⚠️ Native Menüs (`menu.rs`) registrieren macOS-`Menu`/`MenuItem` +
  Accelerators über GPUI — `init_menus(cx)` muss weiter genau einmal aus
  `main.rs` nach dem ersten `open_window` laufen.
- ⚠️ Falls `crates/ui` als Fassade bleibt: keine neue Abhängigkeit *auf* die
  Fassade zulassen — nur der Bin darf sie übergangsweise nutzen.

## Weiterführende Tasks
- [T16-010: Build-Hygiene + Baseline](./T16-010-build-hygiene-baseline.md)
- [T17-006: `AppShell` → reine Komposition](../phase-16-registries/T17-006-appshell-composition-only.md)
