# T04-005: Native macOS-Menüs (App-Menüleiste + Dock-Menü)

## Status
✅ Done

## Phase
3 — App-Shell, Tab-System & Workspace-Layout

## Abhängigkeiten
T04-003 (App-Shell)

## Ziel
Parität zu `dock_menu.rs` + `menu_sync.rs` des Originals: eine vollständige native macOS-
Menüleiste (App / Datei / Bearbeiten / Ansicht / Fenster / Hilfe …) und ein Dock-Kontextmenü,
deren Einträge mit dem App-Zustand synchron sind (z.B. „Neuer Tab", „Split", zuletzt genutzte
Hosts, aktivierte Toggles).

## Kontext
Referenz:
- `reference-src/src-tauri/src/modules/dock_menu.rs` — Dock-Kontextmenü-Einträge + Handler.
- `reference-src/src-tauri/src/modules/menu_sync.rs` — hält die native Menüleiste mit dem
  Frontend-Zustand synchron (Enable/Disable, Häkchen, dynamische Listen).
- `reference-src/src/modules/shortcuts/shortcuts.ts` — Menü-Einträge spiegeln Shortcuts;
  Handler-IDs (`tab.new`, `ai.toggle`, …).
- `reference-src/src-tauri/tauri.conf.json` / `menu`-Aufbau in `lib.rs`.

## Anweisungen
1. GPUI-Menü-API klären (`gpui::Menu`, `MenuItem`, `cx.set_menus(...)` — in gpui/Zed-Source
   verifizieren; Zed hat eine vollständige macOS-Menüleiste als Vorlage).
2. Menüstruktur 1:1 aus dem Original übernehmen (Reihenfolge, Trenner, Untermenüs,
   Tastenkürzel-Anzeige). Jeder Eintrag löst eine GPUI-Action aus.
3. Actions an dieselben Handler binden, die auch die Shortcut-Registry nutzt (T12-002) —
   nur eine Wahrheit für „was macht `tab.new`".
4. Dynamische Teile: „Fenster"-Liste, zuletzt geöffnete Hosts/Ordner, aktive Toggles mit
   Häkchen — Sync-Mechanismus wie `menu_sync.rs` (Menü bei Zustandsänderung neu setzen).
5. Dock-Menü: die Einträge aus `dock_menu.rs` (z.B. „Neues Terminal", schnelle Host-Verbindung).
6. „Über Labonair", „Einstellungen…" (öffnet Settings, Phase 12), „Nach Updates suchen…"
   (Hook für T15-005) verdrahten.

## Akzeptanzkriterien
- [ ] Vollständige macOS-Menüleiste, Struktur/Reihenfolge/Kürzel wie im Original
- [ ] Menü-Einträge lösen dieselben Actions aus wie die Shortcuts
- [ ] Enable/Disable + Häkchen folgen dem App-Zustand (Demo: „Split" ist disabled ohne Tab)
- [ ] Dock-Kontextmenü mit den Original-Einträgen funktioniert
- [ ] „Einstellungen…" und „Nach Updates suchen…" sind vorhanden (dürfen anfangs Platzhalter sein)
- [ ] `cargo check` + `clippy -- -D warnings` grün

## Notizen
- Menü-Sync kann simpel sein: bei relevanten Zustandsänderungen `cx.set_menus()` neu aufrufen.
- Einträge für noch nicht existierende Features (SFTP, AI) als disabled anlegen, später aktivieren.

## Warnungen
- ⚠️ GPUI-Menü-API ist Plattform-spezifisch — Linux-Menüs später (kein Blocker für macOS-first).
- ⚠️ Keine erfundene API — Zed-`crates/zed/src/zed/app_menus.rs` als konkrete Vorlage lesen.
