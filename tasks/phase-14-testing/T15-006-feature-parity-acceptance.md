# T15-006: Feature-Parität-Abnahme

## Status
⏳ Pending

## Phase
14 — Testing & Polish

## Abhängigkeiten
Alle Phasen

## Ziel
Systematische Abnahme, dass **jedes** Feature/Modul aus `reference-src/` in der Rust-Version
existiert und funktioniert. Ergebnis ist eine abgehakte Checkliste + eine dokumentierte Liste
bewusster Abweichungen.

## Kontext
Der User hat explizit festgelegt: alles, was Labonair heute kann, muss am Ende laufen. Diese
Task ist das Kontroll-Gate vor Release.

## Anweisungen
1. Vollständige Modul-Inventur gegen `reference-src/`:
   - Frontend: alle Ordner in `reference-src/src/modules/` (terminal, editor, explorer, tabs,
     header, statusbar, shortcuts, theme, command-palette, git-graph, source-control, snippets,
     session, settings, notifications, preview, search, updater, ai, hosts, sftp, …).
   - Backend: alle Ordner in `reference-src/src-tauri/src/modules/` (ssh, sftp, git, fs, pty,
     hosts, credentials, snippets, secrets, themes, backgrounds, scrollback, settings, shell,
     terminal_exec, mcp, fonts, dock_menu, menu_sync, errors).
2. Für jedes Modul: Roadmap-Task(s) zuordnen, Status prüfen, in der echten App gegentesten
   (Nutzer führt `cargo run` aus, vergleicht mit dem Original-Verhalten).
3. Checkliste in dieser Datei pflegen (Modul → Task(s) → ✅/offen → Notiz).
4. Bewusste Abweichungen dokumentieren:
   - `preview/` (In-App-Web-Preview) → GPUI hat keine WebView → nativer Markdown-Renderer +
     „im System-Browser öffnen". Verifizieren, dass der Ersatz die realen Nutzungsfälle abdeckt.
   - Weitere, falls unterwegs entstanden.
5. Lücken → neue Task-Dateien in der passenden Phase anlegen und abarbeiten, bevor abgehakt wird.
6. IPC-Command-Abgleich: die ~150 Commands aus `reference-src/src-tauri/src/lib.rs`
   (`generate_handler!`) durchgehen — jede dahinterliegende Fähigkeit muss als in-process-
   Funktion existieren.

## Akzeptanzkriterien
- [ ] Jedes Frontend-Modul aus `reference-src/src/modules/` ist zugeordnet + abgehakt oder als
      dokumentierte Abweichung erfasst
- [ ] Jedes Backend-Modul aus `reference-src/src-tauri/src/modules/` ist portiert + verifiziert
- [ ] Jede Fähigkeit hinter den `generate_handler!`-Commands hat ein in-process-Äquivalent
- [ ] Abweichungs-Liste vollständig und vom Nutzer bestätigt
- [ ] Keine offenen „TODO/stub"-Pfade in Kern-Features
- [ ] `cargo check` + `clippy -- -D warnings` + `cargo test` grün; volle manuelle Test-Runde bestanden

## Notizen
- Diese Task ist iterativ — sie kann mehrfach „In Progress" werden, während Lücken geschlossen werden.
- Enge Kopplung mit T15-001 (visuelle Parität) und T15-002 (Robustheit).

## Warnungen
- ⚠️ Nicht abhaken „weil der Code existiert" — nur nach echtem Gegentest in der laufenden App.
