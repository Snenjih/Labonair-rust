# Bericht: Workflow-Rework (Themen 1–3) — Tab-Verhalten, Host-Zugang, Settings

**Status:** normativ für die Phasen 15–18. Ergänzt den Architektur-Rework
([`bericht-architektur-rework-roadmap.md`](./bericht-architektur-rework-roadmap.md),
[`docs/architecture.md`](./docs/architecture.md)). Nach T16-005 vereinbart, während
T16-006 lief — deshalb landete der Palette-Anteil (konzeptionell ≤ T16-004) in
T16-007.

Die drei Themen **erweitern** den geplanten Umbau, ohne seine Muster
(PanelRegistry, StatusItemRegistry, SettingsStore, `MergeFrom`), die
Abhängigkeitsregeln oder den azyklischen Crate-Graph zu brechen. Jede Abweichung
vom T16-001-Plan steht in [`docs/architecture.md §8`](./docs/architecture.md).

---

## Thema 1 — Tabs sind nicht mehr Pflicht (Empty State)

**Heute:** `tabs.rs::close` bricht bei `len() <= 1` ab; `TabKind::Home` ist
unschließbar und rendert den Host-Manager (`workspace.rs:3457`). Damit ist immer
mindestens ein Tab offen.

**Ziel:** Alle Tabs schließbar → leerer Workspace mit einer Empty-Surface
(zentrierter Shortcut-Hinweis; **Doppelklick → lokales Terminal**;
**Datei-Drop → Editor-Tab**). Wiederöffnen über das `＋▾`-Menü in der Titlebar
oder die Command-Palette. Session-Restore stellt den letzten Zustand wieder her —
war er leer, bleibt er leer; `startup_tab` bekommt den Wert `empty`.

**Umsetzung:**
- `T17-004` — `PaneGroup { root: Option<Member> }`; Entfernen des letzten Panes ⇒
  `root = None` (kein Fehler).
- `T17-006` — `AppShell::render` komponiert auch bei 0 Tabs sauber.
- `T17-009` (neu) — `TabKind::Home` streichen; `Option<ActiveTab>`-Audit über
  `labonair-workspace`; `close_all` bis 0; `startup_tab = empty`; Platzhalter-
  Empty-Surface + Doppelklick-Handler. Der Host-Manager bleibt hier interim als
  normaler, schließbarer `TabKind::Hosts`-Tab.
- `T18-001` — endgültige Empty-Surface-Optik + `＋▾`-Neuer-Tab-Menü
  (Terminal/Editor/Preview/Git-Graph + SSH/SFTP-Recent-Submenüs).

**Bewertung:** reine Erweiterung. Hauptaufwand ist das `Option`-Audit in
`workspace.rs`, kein Feature-Risiko.

---

## Thema 2 — SSH-Hosts öffnen ohne Host-Manager

**Heute:** Der Host-Manager ist der `Home`-Tab; zusätzlich gibt es eine kompakte
`SidebarPanel::Hosts`-Liste. Die Command-Palette hat bereits `Page::HostsSsh` /
`Page::HostsSftp` + `ConnectSsh`/`OpenSftp`.

**Ziel — Zweiteilung:**
- **Verbinden:** eine `Page::Hosts` in der Command-Palette (eine Zeile pro Host,
  `Enter` = SSH, `Shift+Enter` = SFTP, Footer-Hinweisleiste), Quick-Connect-Zeilen
  am Palette-Root, `＋▾`-Submenü, native Menüleiste. `Cmd+Shift+N` öffnet die
  Hosts-Seite.
- **Verwalten:** eine erstklassige Top-Level-Settings-Kategorie **„Hosts"** —
  anlegen/bearbeiten/löschen/duplizieren, Credentials (OS-Keychain, **nie** in
  JSON), Jump-Hosts, Tunnel, SSH-Config-Import/Export, Verfügbarkeits-Polling.

**Architektur-Verfeinerung (in `docs/architecture.md §8.1` festgehalten):**
`labonair-panel-hosts` **entfällt** aus dem Ziel-Graph. Der View-Code
(`hosts.rs`, `ssh_connection.rs`) wandert in **`labonair-hosts-ui`** — ein reines
View-Crate, **kein** `labonair-panel-*`, ohne `impl Panel`, ohne
`labonair-workspace`-Kante (Tab-Öffnen per Callback). `labonair-settings-ui`
hängt davon ab und bettet die Verwaltungs-Seite ein. Die Palette bekommt die
Hostliste + Connect-Callbacks als injizierte Daten (kein
`command-palette → hosts-ui`-Edge). `enum SidebarPanel` inkl. `Hosts` und der
`TabKind::Home`-Host-Tab werden gelöscht.

**Umsetzung:** `T16-007` (Palette-`Page::Hosts` + Sekundäraktion), `T16-008`
(`labonair-hosts-ui`), `T17-001` (kein Hosts-Panel), `T17-009` (interimer
`TabKind::Hosts`), `T19-001` (`hosts: HostsContent`), `T19-010` (Settings ›
Hosts, `TabKind::Hosts` weg), `T19-009` (SQLite-Hosts → `hosts.entries` +
Keychain).

**Bewertung:** Erweiterung + eine vom T16-001-Plan ausdrücklich erlaubte
Graph-Verfeinerung. Muster/Regeln/Azyklizität bleiben. Einziger nicht rein
mechanischer Punkt: die `hosts-ui`-Tab-Aufrufe hinter Callbacks legen.

---

## Thema 3 — Settings-Redesign + fester Design-Kontrakt

**Ziel:** Der geplante Zed-Style-Umbau (typisierter `SettingsContent`-Baum,
`SettingsStore` mit Layer-Merge, `Settings`-Trait-Registrierung, UI aus dem
Modell generiert) bleibt **unverändert**. Darauf gesetzt:

1. **Navigations-Modell:** links Top-Level-Kategorie, rechts die Seite mit
   aufklappbaren Abschnitts-Überschriften (Disclosure) + Scroll-Spy;
   große Kategorien mit `SubPageLink`-Unter-Seiten.
2. **„Hosts" als eigene Top-Level-Kategorie** — wie „Themes". Ermöglicht über
   einen **sanktionierten erstklassigen Pfad** für Custom-Top-Level-Kategorien
   (`AREAS`-Eintrag mit `kind = Custom` + `render_fn`), **kein** Eingriff in die
   Feld-Registry. Themes, Hosts, Shortcuts, AI, MCP, Personalisierung laufen
   alle darüber und rendern **im Standard-Seiten-Chrome**.
3. **Design-Kontrakt** `docs/settings-guidelines.md` (`T19-000`, **vor**
   T19-001): ein Navigations-Modell; jede Einstellung ist ein typisiertes
   `SettingsContent`-Feld mit Metadaten; UI aus dem Typ generiert; Custom-Panes
   nur für echte Nicht-Feld-UIs und immer im Standard-Chrome; Herkunfts-Badge +
   Reset je Feld; globale Suche; Deep-Links je Kategorie/Abschnitt; Copy-Regeln.
   Verankert als Critical Rule 9 in `CLAUDE.md`.

**Umsetzung:** `T19-000` (Kontrakt-Doku), `T19-001` (`hosts` + `AREAS` mit
`kind`), `T19-004` (Disclosure-Navigation + Custom-Top-Level-Pfad), `T19-010`
(Hosts als erste neue Custom-Kategorie).

**Bewertung:** reine Erweiterung des Phase-18-Umbaus. Der Kontrakt ist die
geschriebene Zieldefinition, gegen die T19-004 implementiert — verhindert das
Tauri-Drift-Problem.

---

## Geänderte / neue Tasks (Übersicht)

| Task | Art | Kern |
|---|---|---|
| `docs/architecture.md §8` | geändert | Deviations-Abschnitt (drei Themen) |
| `T16-007` | erweitert | Palette-`Page::Hosts` + `Shift+Enter`-Sekundäraktion + Footer-Hints |
| `T16-008` | geändert | `labonair-panel-hosts` → `labonair-hosts-ui` (kein Panel) |
| `T17-001` | geändert | fünf Panels; kein Hosts-Panel; `SidebarPanel::Hosts`-Liste weg |
| `T17-002` | unverändert | (liest aus Registry) |
| `T17-004` | erweitert | `PaneGroup { root: Option<Member> }` |
| `T17-006` | erweitert | Render-Pfad für 0 Tabs |
| `T17-009` | **neu** | Tabs optional, `Option<ActiveTab>`-Audit, `TabKind::Home` weg |
| `T18-001` | erweitert | `＋▾`-Menü + Empty-Surface-Optik + `Cmd+Shift+N`-Retarget |
| `T18-003` | geändert | fünf Panel-Toggles statt sechs |
| `T19-000` | **neu** | Settings-Design-Kontrakt (`docs/settings-guidelines.md`) |
| `T19-001` | erweitert | `hosts: HostsContent` (Top-Level, ohne Secrets); `AREAS` mit `kind` |
| `T19-004` | erweitert | Disclosure-Navigation + Scroll-Spy + `SubPageLink` + Custom-Top-Level-Pfad |
| `T19-009` | erweitert | SQLite-Hosts → `hosts.entries` + Keychain |
| `T19-010` | **neu** | Settings › Hosts; `TabKind::Hosts` endgültig weg |
| `tasks/ROADMAP.md` | geändert | Tabellen 15–18, Workflow-Rework-Abschnitt, Erfolgskriterium 27 |

Kein bereits gebauter Code wird zurückgebaut — alles sind Task-Text-Schärfungen
vor der Umsetzung.
