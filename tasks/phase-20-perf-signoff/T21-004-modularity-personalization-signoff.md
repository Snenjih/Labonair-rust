# T21-004: Modularitäts- & Personalisierungs-Abnahme

## Status
📋 Geplant

## Phase
20 — Performance & Modularitäts-Abnahme

## Abhängigkeiten
Alle Rework-Phasen (15–19) + T21-001..003

## Ziel
Die formale Abnahme des Architektur-Reworks gegen die neue Philosophie: eine
Checkliste, dass Modularität und Personalisierung wirklich greifen, plus ein
Regressions-Durchlauf der Feature-Parität gegen `reference-src` (nichts ist beim
Umbau verloren gegangen).

## Kontext
- Philosophie (T18-007, `docs/architecture.md`): Effizienz + Performance +
  Modularität für Personalisierung.
- Layout-Vertrag: Titlebar (Tabs + 1 Button), Workspace, Statusbar
  (Panel-Steuerung / Info-Dropdowns), Side Panels, Overlays.
- Parität-Referenz: `reference-src/src/modules/*` + `tasks/ROADMAP.md`
  „Erfolgskriterien" 1–21 + die frühere `T15-006`-Checkliste.
- Vergleichsberichte `vergleichsbericht-subagent-1..4.md` (Port vs.
  reference-src) — die dort gelisteten Lücken auf „durch den Rework
  geschlossen / weiterhin offen" prüfen.

## Anweisungen zur Umsetzung
1. **Modularitäts-Checkliste** (`docs/signoff-modularity.md`), jede Zeile
   nachweisbar (Test, Screenshot oder kurze Demo):
   - [ ] Neues Panel = neuer `panel-*`-Crate + `impl Panel` + 1 Zeile
     `register_builtin_panels` (Referenz: den Vorgang mit einem
     Wegwerf-„Hello-Panel" einmal durchspielen und wieder entfernen).
   - [ ] Neues Statusbar-Item = `impl StatusItem` + 1 `register`-Zeile.
   - [ ] Neues Kommando = 1 `register`-Zeile, taucht in Palette + ist
     keybindbar.
   - [ ] Neues `bool`-Setting = 1 Feld im `SettingsContent` (+ optional 1
     `SettingsPageItem`), 0 UI-Code.
   - [ ] `cargo-depgraph` zeigt keinen Zyklus; `check-crate-deps.sh` grün.
   - [ ] `app_shell.rs` < 400 Z.; kein `drain_pending_*`; keine
     `enum SidebarPanel`/`BarItemId`-Kaskade.
2. **Personalisierungs-Checkliste** (`docs/signoff-personalization.md`):
   - [ ] Statusbar-Item per Rechtsklick nach links/rechts + ausblenden;
     persistiert; über Personalisierungs-Seite + Palette rücknehmbar.
   - [ ] Panel per RMB zwischen Docks andocken (L/R/B); persistiert.
   - [ ] Panel-Toggle-Sichtbarkeit pro Panel schaltbar; persistiert.
   - [ ] Theme aus Registry wählbar (mehrere Familien, User-Ordner,
     Live-Reload).
   - [ ] Icon-Theme wählbar.
   - [ ] UI-Dichte (3 Stufen) + Font-/Radius-Skalen live.
   - [ ] `keymap.json` mit Kontexten + Chords editierbar, Live-Reload.
   - [ ] Projekt-`.labonair/settings.json` greift pro Ordner (erlaubte Keys).
   - [ ] `settings.json` roh editierbar, Kommentare bleiben, GUI ⇄ Datei live.
3. **Parität-Regressions-Durchlauf**: die `T15-006`/Erfolgskriterien-Liste
   Punkt für Punkt gegen den Rework-Stand testen (Terminal, Tabs, Explorer,
   Editor+Vim+Diff, SSH/Host-Manager, SFTP+Transfers, Git-UI, Git-Graph,
   AI-Chat+Tools+Plan+MCP, Snippets, Command-Palette, Session-Restore,
   Notifications, Hintergrundbilder, native Menüs, Auto-Updater). Ergebnis in
   `docs/signoff-parity.md`: je Punkt OK / Regression (mit Ticket) / bewusst
   geändert (mit Verweis auf den Layout-Vertrag).
4. **Bekannte, bewusste Abweichungen** dokumentieren (kein Regressions-Bug):
   Header-Suche → `Cmd+F`-Overlay; `⋯`-Menü → Titlebar-Button-Dropdown;
   Activity-Rail entfällt; Bar-Items nur noch Statusbar.
5. **Offene Punkte aus `vergleichsbericht-subagent-1..4.md`** durchgehen und
   markieren, was der Rework erledigt hat und was als Folge-Ticket bleibt.
6. **`handshake.md` + Memory**: Rework abgeschlossen, Sign-off-Dokumente
   verlinkt, Folge-Tickets gelistet.

## Akzeptanzkriterien
- [ ] `docs/signoff-modularity.md`, `docs/signoff-personalization.md`,
      `docs/signoff-parity.md` existieren, jede Zeile mit Nachweis
      (Test/Screenshot/Demo-Notiz).
- [ ] Alle Modularitäts-Checks bestanden (inkl. „Hello-Panel in 1 Zeile"-
      Demo).
- [ ] Alle Personalisierungs-Checks bestanden.
- [ ] Parität-Durchlauf: jeder ROADMAP-Erfolgskriteriums-Punkt ist OK oder
      hat ein Regressions-Ticket; bewusste Abweichungen sind als solche
      markiert (mit Layout-Vertrag-Verweis).
- [ ] Die in den `subagent-*`-Berichten gelisteten Lücken sind auf
      erledigt/offen klassifiziert.
- [ ] `handshake.md` + Memory aktualisiert.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Diese Task produziert vor allem Dokumente + eine ehrliche Bestandsaufnahme.
  Gefundene Regressionen werden als Tickets angelegt, nicht zwingend hier
  gefixt (außer Kleinkram).
- Die „Hello-Panel in 1 Zeile"-Demo ist der aussagekräftigste Modularitäts-
  Beweis — sauber durchführen und im PR zeigen.

## Warnungen
- ⚠️ Nicht die Checkliste weichspülen, damit sie grün wird. Ein ehrliches
  „Regression: X, Ticket #Y" ist wertvoller als ein geschöntes Sign-off.

## Weiterführende Tasks
- [T21-005: Architektur-Doku finalisieren](./T21-005-architecture-doc.md)
