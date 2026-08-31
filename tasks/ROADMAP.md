# Labonair-rust — Roadmap

## Vision

Portierung von Labonair (Tauri v2 + React 19) zu einer reinen nativen Rust-App mit GPUI als UI-Framework — als **Hard Fork**: vollständig standalone, keine Verbindung (Symlink/Submodul/Pfad-Dependency) zum Original-Repo. Ziel ist eine 1:1 funktionsfähige Replik mit identischem Design und spürbar besserer Performance (kein WebView, kein IPC, direkter Prozess).

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

### Phase 12 — Settings & Preferences ·`/tasks/phase-12-settings/`
| Task | Titel | Abhängigkeit |
|---|---|---|
| **T13-001** | Einstellungen-Struktur & Preferences | T01-004, T01-001 |
| **T13-002** | Appearance- & Theme-Einstellungen | T13-001, T02-002/3/4 |
| **T13-003** | Terminal- & Editor-Einstellungen | T13-001, Phase 2/5 |
| **T13-004** | Shortcut-Konfiguration | T13-001, T12-002 |

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
6. **SSH-UI** zeigt Host-Manager und Verbindungs-Status; SSH-Terminals laufen.
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
