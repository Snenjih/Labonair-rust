# T12-003: Path-Bookmarks (Verzeichnis-Lesezeichen)

## Status
⏳ Pending

## Phase
11 — Snippets & Command-Palette

## Abhängigkeiten
T12-002 (Command-Palette & Shortcut-System), T13-001 (Preferences), Phase 4 (Explorer), Phase 6 (SSH-UI)

## Ziel
Portierung des Referenz-Moduls `reference-src/src/modules/bookmarks/` — schnelle
Sprungmarken auf gespeicherte lokale und Remote-Verzeichnispfade, per
Command/Popover erreichbar (`bookmarks.open`, Standard `Cmd+Shift+O`).

## Kontext
Aufgedeckt beim T15-006-Feature-Parität-Audit. Im Port existiert derzeit nur die
Shortcut-ID `bookmarks.open` (in `command_palette.rs` als rebindbar gelistet),
aber **keine Laufzeit-Implementierung** — `command_for_shortcut` liefert `None`.

Referenz-Bausteine:
- `store/pathBookmarksStore.ts` — `PathBookmark { id, path, label?, hostId? }`,
  `bookmarkKey(hostId, path)` (`"local"` für lokal), `computeAddBookmark`
  (dedupe pro `(hostId, path)`, Label-Update statt Zweitanlage),
  `computeRemoveByPath`, `isBookmarkOrphaned(bm, hosts)` (Host gelöscht →
  flaggen, nicht löschen). Persistenz in einer Bookmarks-JSON-Datei
  (`getStoragePaths`).
- `lib/filterBookmarksForContext.ts` — nur Bookmarks des aktiven Kontexts
  (lokal vs. konkreter Host) anzeigen.
- `lib/resolveBookmarkAction.ts` — Klick → Explorer auf den Pfad setzen
  (lokal) bzw. Remote-Explorer/SFTP für den Host.
- `components/BookmarksDropdown.tsx` / `BookmarkRow.tsx` — Popover-UI,
  Add/Remove, Orphan-Kennzeichnung.

## Anweisungen
1. `crates/backend` oder `crates/ui`: reines `path_bookmarks`-Modell (Add/Remove/
   Key/Orphan/Filter) als unit-getestete freie Funktionen, JSON-Persistenz in
   `<config_dir>/labonair/bookmarks.json`.
2. `crates/ui`: `BookmarksView`/Popover (GPUI), an Explorer + Host-Kontext
   gebunden; Add-Aktion aus dem Explorer-Kontextmenü ("Bookmark this folder").
3. `command_palette.rs`: `CommandId::OpenPathBookmarks`, `command_for_shortcut`
   für `ShortcutId::BookmarksOpen` auf diese Command mappen (den bestehenden
   `None`-Testfall anpassen), Dispatch in `app_shell.rs`.
4. Statusbar-Badge/Bar-Item wie in der Referenz (optional, wenn Bar-Item-System
   das trägt).

## Akzeptanzkriterien
- [ ] Lokale + Remote-Pfade lassen sich als Bookmark speichern und wieder öffnen
- [ ] Dedupe pro `(hostId, path)`, Orphan-Kennzeichnung bei gelöschtem Host
- [ ] `Cmd+Shift+O` / Command-Palette öffnet das Bookmark-Popover
- [ ] Persistenz über Neustart
- [ ] `cargo check` + `clippy -D warnings` + `cargo test` grün

## Notizen
- Aus T15-006 ausgegliedert; kein GPUI-Blocker, rein Umfang.
