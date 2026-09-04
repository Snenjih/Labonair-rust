# Labonair-rust — Roadmap

## Vision

Portierung von Labonair (Tauri v2 + React 19) zu einer reinen nativen Rust-App mit GPUI als UI-Framework — als **Hard Fork**: vollständig standalone, keine Verbindung (Symlink/Submodul/Pfad-Dependency) zum Original-Repo. Ziel ist eine 1:1 funktionsfähige Replik mit identischem Design und spürbar besserer Performance (kein WebView, kein IPC, direkter Prozess).

**Neue Philosophie (ab dem Architektur-Rework, Phasen 15–21):** „Der effizienteste Weg, seine Arbeit in Labonair fertig zu bekommen — mit maximaler Performance und Modularität für Personalisierung." Vier Prinzipien binden ab hier alle Tasks des Reworks (Details: `bericht-architektur-rework-roadmap.md` §1):

1. **Simple, feste Grundstruktur** — genau vier sichtbare Zonen (Titlebar, Workspace, Side Panels, Statusbar) + eine Overlay-Ebene, nicht mehr.
2. **Personalisierung ist erstklassig** — Statusbar-Items per Rechtsklick verschiebbar/ausblendbar, Panels zwischen Docks verschiebbar, Themes/Keymap editierbare Dateien, Settings global **und** pro Projektordner.
3. **Modularität im Code = Modularität im Produkt** — jede Feature-Einheit ein eigener Crate mit klarer API, Registrierung über Traits statt zentralem God-Object.
4. **Performance messbar** — kein Arbeiten pro Frame, das pro Event reicht; kein `cx.notify` ohne Zustandsänderung; Startup-/Build-Zeit dokumentiert.

Feature-Parität mit der Referenz-App bleibt Pflicht, ist ab hier aber das *Minimum*, nicht das Ziel: viele fokussierte Crates statt `ui`-Monolith, Trait-Registries statt God-Object, ein fester Layout-Vertrag mit erstklassiger Personalisierung. Maßgebliche Ziel-Architektur: [`docs/architecture.md`](../docs/architecture.md); Begründung: [`docs/adr/0001-crate-decomposition.md`](../docs/adr/0001-crate-decomposition.md).

## Feature-Parität (alles muss am Ende funktionieren)

**Kein Feature ist out-of-scope.** Alles, was Labonair heute kann, muss am Ende in der puren Rust-Version laufen — inklusive: Auto-Updater, Terminal-Hintergrundbilder, native macOS-Menüs (App-Menüleiste + Dock-Menü), MCP-Bridge (externe Agenten steuern SSH/lokale Tabs), Font-Handling, Notifications/Toasts.

**Einzige unvermeidbare Abweichung:** der In-App-URL-/Web-Preview-Tab (`reference-src/src/modules/preview/`) — GPUI kann keine WebView einbetten. Ersatz: nativer Markdown-Renderer + „im System-Browser öffnen". Sonst keine Abstriche.

## Tech-Stack (Ziel)

| Komponente | Heute (Labonair) | Ziel (Labonair-rust) |
|---|---|---|
| **UI-Framework** | React 19 + Tailwind CSS v4 | GPUI + gpui-component |
| **Terminal** | xterm.js (WebGL) | alacritty_terminal + GPUI-Renderer |
| **Editor** | CodeMirror 6 | TreeSitter-basierter Editor |
| **State Management** | Zustand (30 Stores) | GPUI Model/Entity-Pattern |
| **Styling** | Tailwind CSS (oklch-Tokens) | GPUI-Inline-Styling + Theme-Objekt |
| **Backend** | Tauri v2 (Rust) | Direkte Rust-Aufrufe (kein IPC) |
| **Plattform** | macOS + Linux (Tauri) | macOS (first), Linux (später) |

## Architektur-Prinzipien

1. **Kein IPC mehr** — Die Rust-Logik (SSH, SFTP, Git, PTY, SQLite, Keyring) läuft direkt im selben Binary. Kein `invoke()`, kein JSON-Serialisieren.
2. **Referenz-Ordner** — `reference-src/` (eingefrorene Kopie des Original-Source im Repo) ist die *einzige* Referenz. UI-Werte, Farben, Spacing, Verhalten werden dort abgelesen und in GPUI übersetzt. Keine externe Anbindung.
3. **Phasenbasiert** — Jede Phase ist ein eigenständig testbares Modul. Am Ende jeder Phase zeigt `cargo run` eine funktionsfähige (wenn auch unvollständige) App.
4. **Design-Parität** — Das Ziel ist 1:1 identisches Aussehen. Alle Farben, Fonts, Spacing, Radien, Shadows aus `globals.css` werden 1:1 in GPUI-Theme-Tokens übersetzt.

## Workflow (KI-Unterstützung + Nutzer-Testing)

Die Arbeit wird stark von KI (Claude Code / dieses Projekt) übernommen. Der Nutzer übernimmt das Testen:
1. Pro Task werden umfangreiche Anweisungen (siehe Task-Dateien) abgearbeitet.
2. Der Nutzer führt `cargo run` aus und vergleicht mit der Referenz-App (Screenshots/parallel).
3. Feedback wird zurückgespielt und iteriert.
4. Der Fokus liegt auf **visueller + funktionaler Parität** — die Logik ist bereits in Rust vorhanden.

## Phasen- und Task-Übersicht

Nummerierung: `T{NN}-{OOO}` wobei NN die Phase (01–15) und OOO die Task-Nummer in der Phase ist.

### Phase 00 — Projekt-Setup & Grundgerüst ·`/tasks/phase-00-setup/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T01-001** | Cargo-Workspace & Projektstruktur | — |
| **T01-002** | Backend-Logik aus reference-src extrahieren | T01-001 |
| **T01-003** | Referenz-Kopie verifizieren & Projekt-Doku | T01-001 |
| **T01-004** | Event-System & Logging | T01-001, T01-002 |
| **T01-005** | CI-Pipeline (cargo check/clippy/test/fmt auf macOS) | T01-001 |

### Phase 01 — Theme-System & Design-Tokens ·`/tasks/phase-01-theme/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T02-001** | Design-Tokens aus globals.css extrahieren | T01-001 |
| **T02-002** | Theme-Provider und Theme-Store | T02-001 |
| **T02-003** | Theme-Import/Export für Benutzer-Themes | T02-001, T02-002 |
| **T02-004** | Terminal-ANSI-Palette in Theme integrieren | T02-001 |
| **T02-005** | Font-Handling & Font-Bundling (GPUI) | T01-001 |
| **T02-006** | Terminal-Hintergrundbilder | T02-002, Phase 2 |

### Phase 02 — Terminal-Engine ·`/tasks/phase-02-terminal/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T03-001** | alacritty_terminal als Terminal-Engine einbinden | T01-001, T02-004 |
| **T03-002** | GPUI-Terminal-Renderer für Zellen bauen | T03-001, T02-004 |
| **T03-003** | Tastatur- und Maus-Mapping | T03-002 |
| **T03-004** | Shell-Integration und CWD-Tracking | T03-002 |
| **T03-005** | Lokale PTY-Sessions & Multi-Tab-Terminal | T03-001–004 |

### Phase 03 — App-Shell, Tab-System & Workspace-Layout ·`/tasks/phase-03-tabs-workspace/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T04-001** | Tab-Leiste & Tab-Verwaltung | T02-001/2 |
| **T04-002** | Split-Pane-Layout & Workspace | T04-001 |
| **T04-003** | App-Shell & Fensterchrome (Header, Statusbar, Sidebar-Container, Root-Coordinator) | T04-001, T04-002 |
| **T04-004** | Notifications / Toast-System | T04-003 |
| **T04-005** | Native macOS-Menüs (App-Menüleiste + Dock-Menü) | T04-003 |

### Phase 04 — File-Explorer ·`/tasks/phase-04-explorer/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T05-001** | Dateibaum & Explorer-Grundlagen | T04-002 |
| **T05-002** | Drag-and-Drop & erweiterte Dateiaktionen | T05-001 |

### Phase 05 — Code-Editor ·`/tasks/phase-05-editor/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T06-001** | Editor-Fundament & Datei-Öffnen/Speichern | T04-001/2, T05-001 |
| **T06-002** | Syntax-Highlighting & Sprach-Erkennung | T06-001 |
| **T06-003** | Vim-Modus | T06-001 |
| **T06-004** | Diff-Ansicht | T06-001 |
| **T06-005** | Editor Soft-Wrap + hörbare Terminal-Glocke (aus T15-006) | T06-001, T03-002 |

### Phase 06 — SSH-UI & Host-Manager ·`/tasks/phase-06-ssh-ui/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T07-001** | Host-Manager & SSH-Verbindung | T04-001/2 |
| **T07-002** | Jump-Hosts & Tunnel | T07-001 |
| **T07-003** | SSH-Config-Import/Export | T07-001 |

### Phase 07 — SFTP-Browser ·`/tasks/phase-07-sftp/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T08-001** | SFTP-Dateibrowser | T07-001, T04-001/2, T05-001 |
| **T08-002** | SFTP-Transfers (Upload/Download/Queue) | T08-001 |

### Phase 08 — Git-UI & Source-Control ·`/tasks/phase-08-git-ui/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T09-001** | Source-Control-Panel (Status & Staging) | T04-001/2, T05-001, T06-004 |
| **T09-002** | Branch-Verwaltung & Stash | T09-001 |

### Phase 09 — Git-Graph ·`/tasks/phase-09-git-graph/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T10-001** | Git-Graph-Rendering (Commit-Graph) | T04-001/2 |

### Phase 10 — AI-Chat-System ·`/tasks/phase-10-ai-chat/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T11-001** | AI-Provider-Integration (Multi-Provider BYOK) | T04-001 |
| **T11-002** | Chat-Store & Session-Verwaltung (AI) | T11-001 |
| **T11-003** | Chat-UI & Streaming-Markdown | T11-002 |
| **T11-004** | Agent/Tool-System & Live-Bridge | T11-001–003 |
| **T11-005** | MCP-Bridge — Server (rmcp/axum, OSC133-Capture, SSH+lokal-Parität) | T03-005, T07-001 |
| **T11-006** | MCP-Bridge — Grants-UI & Settings (Per-Tab-Opt-in, Block-Liste, Auto-Revoke) | T11-005, T13-001 |

### Phase 11 — Snippets & Command-Palette ·`/tasks/phase-11-snippets-palette/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T12-001** | Befehl-Snippets-System | T04-001, T07-001, T03-001 |
| **T12-002** | Command-Palette & Shortcut-System | T04-001 |
| **T12-003** | Path-Bookmarks (Verzeichnis-Lesezeichen, aus T15-006) | T12-002, T13-001 |

### Phase 12 — Settings & Preferences ·`/tasks/phase-12-settings/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T13-001** | Einstellungen-Struktur & Preferences | T01-004, T01-001 |
| **T13-002** | Appearance- & Theme-Einstellungen | T13-001, T02-002/3/4 |
| **T13-003** | Terminal- & Editor-Einstellungen | T13-001, Phase 2/5 |
| **T13-004** | Shortcut-Konfiguration | T13-001, T12-002 |
| **T13-005** | Restliche Shortcut-Handler (Tab-Index, Pane-Fokus, Zen-Mode, aus T15-006) | T13-004, T04-002 |

### Phase 13 — Session-Persistenz & Scrollback ·`/tasks/phase-13-session/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T14-001** | Session-Persistenz (Tabs/Layout) | Phase 3, T04-002, T03-005 |
| **T14-002** | Scrollback-Persistenz | T14-001, Phase 2, T03-001 |

### Phase 14 — Testing & Polish ·`/tasks/phase-14-testing/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T15-001** | Visuelle Paritäts-Verifikation (Design-Feinschliff) | Alle Phasen |
| **T15-002** | Fehlerbehandlung & Robustheit (app-weit) | Alle Backend-Subsysteme |
| **T15-003** | Cross-Platform- & Performance-Optimierung | Terminal, Layout, Kern |
| **T15-004** | Verpackung & Release (inkl. License-Audit: GPUI/ztracing-GPL) | Alle Phasen |
| **T15-005** | Auto-Updater (macOS, Sparkle o.ä. — Ersatz für tauri-plugin-updater) | T15-004 |
| **T15-006** | Feature-Parität-Abnahme (Checkliste gegen reference-src, alle Module) | Alle Phasen |

## Erfolgskriterien

1. **`cargo run`** startet die App auf macOS.
2. **Terminal** zeigt eine funktionsfähige Shell mit korrektem Design.
3. **Tabs** funktionieren (neu, schließen, wechseln, splitten).
4. **File-Explorer** zeigt den lokalen Dateibaum.
5. **Editor** öffnet Dateien mit Syntax-Highlighting (+ Vim optional).
6. **SSH-UI** — Hosts verbinden (Command-Palette-`Page::Hosts` + `＋▾`-Menü) und verwalten (Settings › Hosts, ab Rework); Verbindungs-Status sichtbar; SSH-Terminals laufen. (Bis Phase 18: noch als Host-Manager-Tab.)
7. **SFTP** zeigt Remote-Dateiliste und unterstützt Transfers.
8. **Git-UI** zeigt Staging, Diff, Branches, Stash.
9. **Git-Graph** rendert den Commit-Graph.
10. **AI-Chat** sendet Nachrichten an Provider und rendert Antworten (Tools + Genehmigung).
11. **Settings** erlaubt Theme-Wechsel und Preferences.
12. **Snippets** lassen sich lokal/SSH ausführen; Command-Palette funktioniert.
13. **Session-Restore** stellt Tabs/Layout/Scrollback wieder her.
14. **Design** ist 1:1 identisch mit Labonair (visueller Vergleich).
15. **Performance** ist messbar besser (Start, Rendering, Speicher) als die Referenz.
16. **App-Shell** — Header, Statusbar, Sidebar, native macOS-Menüleiste + Dock-Menü funktionieren.
17. **Notifications** — Toasts/Fehler werden app-weit angezeigt.
18. **Terminal-Hintergrundbilder** lassen sich setzen und rendern korrekt.
19. **MCP-Bridge** — eine externe Agent-CLI kann einen freigegebenen Tab steuern (list/run/read/send/open/close).
20. **Auto-Updater** prüft, lädt und installiert Updates auf macOS.
21. **Feature-Parität** — jedes Modul aus `reference-src/` ist abgehakt (T15-006). Einzige Abweichung: Web-Preview-Tab → nativer Markdown + System-Browser.

---

## Architektur-Rework (Phasen 15–21) — Zed-Architektur-Stil

Nach Erreichen der Feature-Parität (Phasen 00–14) folgt ein Umbau der internen
Architektur in Richtung des Zed-Musterkatalogs: viele fokussierte Crates statt
`ui`-Monolith, Trait-Registries statt God-Object, typisierter Settings-Merge-Baum
mit generierter UI, ein durchgängiges UI-Kit, ein Theme-/Icon-Theme-Registry, und
ein fester Layout-Vertrag mit erstklassiger Personalisierung.

**Neue Philosophie:** „Der effizienteste Weg, seine Arbeit in Labonair fertig zu
bekommen — mit maximaler Performance und Modularität für Personalisierung."
Feature-Parität ist ab hier das *Minimum*, nicht das Ziel.

**Layout-Vertrag:** Titlebar = nur Tabs + ein Icon-Button (Settings/Profile-
Dropdown). Workspace = Tab-Inhalt + rekursiver Split-Baum. Side Panels = Docks
links/rechts/unten. Statusbar = links Panel-Steuerung, rechts Info-Dropdowns
(Notifications-Badge, CWD, Updater, Transfers, Agent-Access), jedes Item per
Rechtsklick links/rechts/ausblendbar. Overlays nur über `ModalLayer`/`ToastLayer`.

Details: [`docs/architecture.md`](../docs/architecture.md),
Planungsbericht: [`bericht-architektur-rework-roadmap.md`](../bericht-architektur-rework-roadmap.md),
Vergleichsbericht: [`vergleichsbericht-zed-vs-rust.md`](../vergleichsbericht-zed-vs-rust.md).

**Workflow-Rework (Themen 1–3, in die Phasen 15–18 integriert — nach T16-005
vereinbart).** Erweitert den Plan, ohne Muster/Regeln/Graph zu brechen; jede
Abweichung ist in [`docs/architecture.md §8`](../docs/architecture.md)
festgehalten. Begründung: [`bericht-workflow-rework.md`](../bericht-workflow-rework.md).
1. **Tabs sind optional** — alle Tabs schließbar → leere Workspace-Fläche
   (Doppelklick → lokales Terminal); `TabKind::Home` entfällt (T17-009,
   T18-001).
2. **Host-Zugang ohne Host-Manager-Tab/-Panel** — Verbinden über
   Command-Palette-`Page::Hosts` (`Enter` = SSH, `Shift+Enter` = SFTP) +
   `＋▾`-Menü; Verwalten über **Settings › Hosts** (T16-007, T16-008, T17-001,
   T19-001, T19-010).
3. **Settings-Redesign** — Zed-Settings-System wie geplant, **plus**
   Design-Kontrakt (`docs/settings-guidelines.md`), Kategorie→Abschnitt-
   Disclosure-Navigation und ein erstklassiger Pfad für Custom-Top-Level-
   Kategorien (T19-000, T19-001, T19-004, T19-010).

### Phase 15 — Crate-Zerlegung & Fundament ·`/tasks/phase-15-crate-split/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T16-001** | ADR & Ziel-Crate-Graph festschreiben | — |
| **T16-002** | `labonair-gpui-ext` + `labonair-ui-kit` (Skeleton) | T16-001 |
| **T16-003** | `labonair-notifications` extrahieren | T16-002 |
| **T16-004** | `labonair-command-palette` extrahieren | T16-002, T16-003 |
| **T16-005** | `labonair-panel` Contracts-Crate | T16-001 |
| **T16-006** | `labonair-workspace` extrahieren | T16-002, T16-005 |
| **T16-007** | `labonair-settings-ui` extrahieren (+ Palette-`Page::Hosts` mit `Enter`/`Shift+Enter`, Thema 2) | T16-002, T16-004 |
| **T16-008** | Panel-Crates ausgliedern (explorer/scm/git-graph/snippets/ai) + `labonair-hosts-ui` (kein Panel, Thema 2) | T16-006, T16-005 |
| **T16-009** | `labonair-shell` + `labonair-app` schlank | T16-006–008 |
| **T16-010** | Build-Hygiene & Baseline (Dep-Regeln, Crate-Graph, Zeit-Baseline) | T16-009 |

### Phase 16 — Root-Objekt & Registries ·`/tasks/phase-16-registries/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T17-001** | `Panel`-Trait & `PanelRegistry` verdrahten (`enum SidebarPanel` weg; **kein Hosts-Panel**) | Phase 15 |
| **T17-002** | `Dock`-Modell (Links/Rechts/Unten), mehrere Panels je Dock | T17-001 |
| **T17-003** | `StatusItem`-Trait & `StatusItemRegistry` (`render_bar_item`-`match` weg) | T16-005, T17-001 |
| **T17-004** | `PaneGroup` — rekursiver Split-Baum + Persistenz (**`Option`ale Wurzel**) | T16-006, T17-002 |
| **T17-005** | `ModalLayer` + `ToastLayer` (Overlays entkoppeln, `drain_pending_*` weg) | T16-006, T16-003, T16-004 |
| **T17-006** | `AppShell` → reine Komposition (God-Object auflösen) | T17-002–005 |
| **T17-007** | `CommandRegistry` (Palette + Keymap teilen die Registry) | T16-004, T17-006 |
| **T17-008** | `AppEvent`-Bus entscheiden (nutzen oder streichen) | T17-006 |
| **T17-009** | Tabs optional — Empty-Workspace-State + `TabKind::Home`/Host-Manager-Tab weg (Thema 1) | T17-004, T17-006 |

### Phase 17 — Neues Layout & Statusbar-Personalisierung ·`/tasks/phase-17-layout/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T18-001** | Titlebar-Redesign — Tabs + `＋▾`-Neuer-Tab-Menü + rechter Icon-Button; Empty-Surface (Thema 1) | T17-006, T17-009 |
| **T18-002** | Suche als transientes Overlay (`Cmd+F`) | T17-005, T18-001 |
| **T18-003** | Statusbar links — Panel-Steuerung (Activity-Rail weg) | T17-002, T17-003 |
| **T18-004** | Statusbar rechts — Info-Dropdowns | T17-003, T18-003 |
| **T18-005** | Statusbar-Item-Personalisierung (RMB → links/rechts/ausblenden) | T18-004 |
| **T18-006** | Migrator `barItemPlacements` → `statusBarItemPlacements` | T18-005 |
| **T18-007** | Philosophie verankern + Personalisierungs-Settings-Seite | T18-005, T18-003 |

### Phase 18 — Settings-System Zed-Style ·`/tasks/phase-18-settings-core/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T19-000** | Settings-Design-Kontrakt festschreiben (`docs/settings-guidelines.md` + Critical Rule 9) (Thema 3) | T16-007 |
| **T19-001** | `labonair-settings-content` — typisierter Baum + `MergeFrom` (**`hosts` als eigener Top-Level-Bereich**) | T16-001, T16-007, T19-000 |
| **T19-002** | `SettingsStore` + Layer-Merge + `Settings`-Trait/Registrierung | T19-001 |
| **T19-003** | Projekt-/Ordner-Settings (`.labonair/settings.json`, Whitelist) | T19-002 |
| **T19-004** | Settings-UI aus dem Modell generieren (`FIELDS`-Array weg) + Kategorie→Abschnitt-Disclosure-Navigation + Custom-Top-Level-Pfad (Thema 3) | T19-002, T16-007, T19-000 |
| **T19-005** | Rohe `settings.json` editierbar (kommentar-erhaltend) | T19-002, T19-004 |
| **T19-006** | JSON-Schema-Generierung + Validierung | T19-001, T19-005 |
| **T19-007** | Globale Settings-Suche über alle Seiten | T19-004 |
| **T19-008** | Keymap als Datei mit Kontexten + Chords (`keymap.json`) | T17-007, T19-002 |
| **T19-009** | Settings-Migrator (`preferences`/`editor`/`mcp` → `SettingsContent`, Keybinds → `keymap.json`, SQLite-Hosts → `hosts.entries`) | T19-004, T19-008, T19-010 |
| **T19-010** | Settings › Hosts — Host-/Credential-Verwaltung als Top-Level-Kategorie; `TabKind::Hosts` weg (Thema 2) | T19-004, T16-008, T19-001 |

### Phase 19 — UI-Kit & Theme-System ·`/tasks/phase-19-ui-kit/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T20-001** | `ui-kit` Primitive-Set vervollständigen | T16-002 |
| **T20-002** | View-Migration Welle 1 (Terminal, Editor, Explorer, SCM) | T20-001 |
| **T20-003** | View-Migration Welle 2 (Hosts, Snippets, AI, SFTP, Git-Graph, Settings-UI) | T20-002 |
| **T20-004** | Component-Gallery (Debug-Fenster) | T20-001 |
| **T20-005** | `ThemeRegistry` + JSON-Theme-Familien | T16-009, T19-002 |
| **T20-006** | Icon-Themes (JSON, umschaltbar) | T20-005 |
| **T20-007** | `theme_settings`-Layer (Dichte, Font-Skalen, Radius) | T20-005, T19-002, T20-001 |

### Phase 20 — Performance & Modularitäts-Abnahme ·`/tasks/phase-20-perf-signoff/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T21-001** | Render-Pfad-Profiling & Frame-Hygiene | T17-006, T20-003 |
| **T21-002** | Build-Budget & Crate-Graph-Verifikation | T16-010, T19-009 |
| **T21-003** | Startup-Profiling (Zeit bis erstes Frame, Speicher) | T17-006, T19-002 |
| **T21-004** | Modularitäts- & Personalisierungs-Abnahme + Parität-Regression | Phasen 15–19, T21-001–003 |
| **T21-005** | Architektur-Doku finalisieren | T21-004 |

### Phase 21 — Decision-Gate ·`/tasks/phase-21-gpui-decision/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T22-001** | vendored `gpui` — Entscheidungs-Task (P4, Gate) | T21-004, T21-005 |

### Rework-Erfolgskriterien (zusätzlich zu 1–21)
22. **Modularität** — neues Panel / Statusbar-Item / Kommando / `bool`-Setting / Custom-Settings-Kategorie = je *eine* Registrierungszeile; Crate-Graph azyklisch; `app_shell.rs` < 400 Z., kein `drain_pending_*`.
23. **Layout-Vertrag** — Titlebar nur Tabs + `＋▾`-Menü + ein rechter Button; Statusbar links Panel-Steuerung / rechts Info-Dropdowns; Overlays nur `ModalLayer`/`ToastLayer`.
24. **Personalisierung** — Statusbar-Items per RMB links/rechts/aus (persistent); Panels zwischen Docks (L/R/B) verschiebbar; mehrere Themes + Icon-Themes + UI-Dichte; `keymap.json` mit Kontexten/Chords; Projekt-`.labonair/settings.json` greift.
25. **Settings-Modell** — eine typisierte `SettingsContent`-Quelle; UI generiert (kein paralleles `FIELDS`-Array); `settings.json` roh editierbar mit Kommentar-Erhalt; Migrator verliert keine Nutzerdaten; alle Seiten folgen `docs/settings-guidelines.md` (ein Navigations-Modell, Custom-Panes nur im Standard-Chrome).
26. **Performance** — Idle = 0 Kern-View-Renders; kein Pro-Frame-Recompute; inkrementeller `-p labonair-shell`-Build deutlich schneller als der alte `app_shell.rs`-Änderungsfall.
27. **Workflow-Rework (Themen 1–3)** — (a) alle Tabs schließbar → leere Fläche, Doppelklick öffnet lokales Terminal, `startup_tab = empty` greift; (b) der Host-Manager ist weder Tab noch Panel: Verbinden nur über Palette-`Page::Hosts` (`Enter`/`Shift+Enter`) + `＋▾`-Menü, Verwalten nur über Settings › Hosts; (c) „Hosts" ist eine erstklassige Custom-Top-Level-Settings-Kategorie.
