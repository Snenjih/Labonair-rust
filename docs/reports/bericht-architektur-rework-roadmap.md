# Bericht & Roadmap — Labonair-rust: Umbau in Zed-Architektur-Stil

Erstellt: 2026-09-03
Grundlage: `vergleichsbericht-zed-vs-rust.md` (Zed-Referenz vs. `crates/`)
Status: **Planungsbericht** — noch keine Implementierung. Nach Freigabe werden
daraus die Task-Dateien unter `tasks/phase-15..21/` geschrieben.

Nutzer-Entscheidungen (in diese Roadmap eingearbeitet):

* **Titlebar-Button:** einfaches Dropdown mit `Settings…` + `Profile` (Platzhalter),
  bewusst erweiterbar für bereits geplante Features.
* **Reihenfolge:** Rework läuft **sofort, vor** der restlichen Feature-Parität
  (Voice/Whisper, T15-006-Abnahme) — auf sauberem Fundament.
* **Settings-Layering:** volles Zed-Layering inkl. Projekt-/Ordner-Settings
  (`.labonair/settings.json`).
* **vendored gpui (P4):** bleibt **offene Entscheidung** mit Gate am Ende.

---

## 1. Neue Philosophie

> **„Der effizienteste Weg, seine Arbeit in Labonair fertig zu bekommen —
> mit maximaler Performance und Modularität für Personalisierung."**

Konkret bindend für alle Tasks dieses Reworks:

1. **Simple, feste Grundstruktur.** Die App besteht aus genau vier sichtbaren
   Zonen + einer Overlay-Ebene — mehr nicht:
   * **Titlebar** — nur Tabs, rechts **ein** Icon-Button (Dropdown: Settings /
     Profile / künftige Einträge).
   * **Workspace** — Tab-Inhalt + rekursives Split-Layout.
   * **Side Panels** — Docks links / rechts / unten, je mehrere umschaltbare Panels.
   * **Statusbar** — links die **Panel-Steuerung** (Toggles je Panel), rechts die
     **Info-Dropdowns** (Notifications mit Badge, CWD-Breadcrumb, Updater,
     Transfers, Agent-Access).
   * **Modal/Overlay-Ebene** — Command-Palette, Dialoge, transiente Suche, Toasts.
2. **Personalisierung ist erstklassig.** Statusbar-Items sind per **Rechtsklick →
   Kontextmenü** zwischen links/rechts verschiebbar und ausblendbar. Panels sind
   zwischen Docks verschiebbar. Themes und Keymap sind editierbare Dateien.
   Settings gelten global **und** pro Projektordner.
3. **Modularität im Code = Modularität im Produkt.** Jede Feature-Einheit ist ein
   eigener Crate mit klarer API. Neue Panels / Statusbar-Items / Settings
   registrieren sich über Traits — kein zentrales God-Object fasst sie an.
4. **Performance messbar.** Kein Arbeiten pro Frame, das pro Event reicht; kein
   `cx.notify` ohne Zustandsänderung; Startup- und Build-Zeit werden vor/nach
   dem Rework dokumentiert.

Diese Philosophie ersetzt in `tasks/ROADMAP.md` (Abschnitt „Vision") die reine
Parität-Formulierung — Parität bleibt Pflicht, ist aber ab hier das *Minimum*,
nicht das Ziel.

---

## 2. Ziel-Architektur

### 2.1 Crate-Graph (7 → ~22 Crates)

```
labonair-app            (bin)   – nur main(): Runtime, Backend-Init, Fenster-Bootstrap

── Fundament ──────────────────────────────────────────────────────────────
labonair-gpui-ext               – prelude-Re-Exports, GPUI-Helfer-Traits, Shared-Newtypes
labonair-ui-kit                 – Design-System: Button, IconButton, List, Dropdown,
                                  Select, Dialog, Popover, ContextMenu, Disclosure,
                                  Table, Tabs, Tooltip, Divider, Indicator, Badge,
                                  Banner, KeybindingHint, Kbd, Icon/IconName, file_icon
labonair-theme        (erweitert) – ThemeRegistry, JSON-Theme-Familien, Icon-Themes,
                                  theme_settings-Layer (Dichte/Font/Radius)
labonair-notifications          – NotificationCenter + Toast-Rendering
labonair-command-palette        – Palette-UI + Command-/Keybind-Modell

── Settings-Track ─────────────────────────────────────────────────────────
labonair-settings-content       – typisierter SettingsContent-Baum + MergeFrom
labonair-settings               – SettingsStore (Layer-Merge), Settings-Trait +
                                  Registrierung, keymap.json-Loader, JSON-Surgical-Edit,
                                  Schema-Generierung
labonair-settings-ui            – Settings-Fenster, Seiten, generierte Feld-Renderer

── Workspace-Track ───────────────────────────────────────────────────────
labonair-panel                  – Contracts: Panel-Trait, PanelRegistry,
                                  StatusItem-Trait, StatusItemRegistry (bricht Zyklus)
labonair-workspace              – Workspace, Pane, PaneGroup (Split-Baum), Dock (L/R/B),
                                  StatusBar-Host, ModalLayer, ToastLayer-Host, Persistenz
labonair-shell                  – AppShell: komponiert Titlebar + Docks + Workspace +
                                  StatusBar + ModalLayer. Dünn, kein Feature-Code.

── Panels (je 1 Crate) ───────────────────────────────────────────────────
labonair-panel-explorer  · labonair-panel-scm  · labonair-panel-git-graph
labonair-panel-hosts     · labonair-panel-snippets  · labonair-panel-ai

── Unverändert (evtl. später eigener Split) ──────────────────────────────
labonair-terminal (Engine) · labonair-editor · labonair-backend · labonair-ai
```

**Abhängigkeitsregeln (in einem ADR festgeschrieben, T16-001):**

* `labonair-panel` hängt von nichts aus dem Workspace-Track ab → bricht den
  Zyklus „Panels brauchen Workspace-Typen, Workspace braucht Panel-Trait".
* Panel-Crates hängen von `panel` + `ui-kit` + `backend` + `theme` — **nie**
  voneinander, **nie** von `shell`.
* `shell` hängt von `workspace` + `panel` + allen `panel-*` + `settings-ui` —
  und ist der einzige Ort, der konkrete Panels kennt (Registrierung).
* `backend` / `ai` / `terminal` / `editor` hängen von **keinem** UI-Crate.

### 2.2 Registries statt God-Object

| Heute (`AppShell`) | Neu |
|---|---|
| ~20 `Entity`-Felder, manuelles `cx.observe(&x, …).detach()` je Feld | `PanelRegistry` + `StatusItemRegistry` + `CommandRegistry`; `shell` hält nur die Registries + Workspace |
| `enum SidebarPanel { … 6 Varianten }` + `label`/`slug`/`from_slug`/`render_panel_body`-Arm | `trait Panel` (Port `zed/crates/workspace/src/dock.rs`), Panel-Crate ruft `registry.register::<MyPanel>()` |
| `render_bar_item`-`match`-Kaskade über `BarItemId` | `trait StatusItem { fn placement(); fn hide(); fn render(); }` (Port `status_bar.rs` + `HideStatusItem`) |
| Riesige `.on_action(cx.listener(Self::act_*))`-Kette im `render` | `CommandRegistry` — Command-Palette **und** Keymap teilen sich dieselben Handler |
| `render()` startet mit `drain_pending_commands/bookmarks/ai` + `sync_live_bridge` | Events via `cx.subscribe_in` / `window.defer` direkt verarbeitet — keine Frame-Puffer |
| `AppEvent`-Broadcast-Bus wird nur geloggt | an `cx.subscribe`-Brücke (Backend→UI: Transfer-Progress, Host-Reachability) — oder ersatzlos gestrichen |

### 2.3 Layout-Vertrag (verbindlich)

```
┌─ Titlebar ────────────────────────────────────────────────────────────────┐
│  [Tab] [Tab] [Tab] [+]                                            [◉ ▾]     │  ← nur Tabs + 1 Button
├─ Docks + Workspace ──────────────────────────────────────────────────────┤
│ ┌ left dock ┐                                              ┌ right dock ┐ │
│ │  Panel    │            Workspace (Split-Baum)            │   Panel    │ │
│ └───────────┘                                              └────────────┘ │
│ ┌ bottom dock ─────────────────────────────────────────────────────────┐ │
│ │  Panel                                                                │ │
│ └──────────────────────────────────────────────────────────────────────┘ │
├─ Statusbar ──────────────────────────────────────────────────────────────┤
│ [Explorer][SCM][Git][Hosts][Snippets][AI]  ·············  [⟳][CWD ▸][🔔³] │
│  └─ Panel-Toggles (links, Default) ──────┘   └─ Info-Dropdowns (rechts) ──┘│
└──────────────────────────────────────────────────────────────────────────┘
   Overlay-Ebene: Command-Palette · Dialoge · Cmd+F-Suche · Toasts
```

* **Titlebar-Button `[◉ ▾]`** → Dropdown: `Settings…`, `Profile` (Platzhalter),
  Trenner, Platz für geplante Einträge. Ersetzt das alte `⋯`-Menü.
* **Header-Inline-Suche entfällt** aus der Titlebar → transientes Overlay per
  `Cmd+F` (Overlay-Ebene, keine permanente Fläche).
* **44px-„Activity-Rail"** (in `subagent-1.md` als Erfindung markiert) **entfällt**
  — Panel-Wechsel läuft über die Statusbar-Toggles.
* **Jedes Statusbar-Item** ist ein `StatusItem`: Rechtsklick → `Nach links` /
  `Nach rechts` / `Ausblenden`. Persistenz `statusBarItemPlacements`
  (`{ itemId: { side, hidden } }`) — der Titlebar-Scope des alten
  `barItemPlacements` fällt weg.
* **macOS-Menüleiste** bleibt nativ erhalten (Parität); der Titlebar-Dropdown ist
  der plattformübergreifende + auffindbare Zweitweg.

### 2.4 Muster-Katalog — was wir 1:1 von Zed abschauen

| Bereich | Zed-Quelle | Übernahme |
|---|---|---|
| Panel/Dock | `crates/workspace/src/dock.rs`, `crates/panel` | `Panel`-Trait (`position`, `set_position`, `default_size`, `min_size`, `PanelEvent`), `DockPosition` |
| Statusbar-Items | `crates/workspace/src/status_bar.rs` | `StatusItemView` + `HideStatusItem` (Item beschreibt Ausblenden selbst) |
| Split-Layout | `crates/workspace/src/pane_group.rs` | rekursiver `Member::Axis`-Baum + Persistenz |
| Overlay-Ebenen | `crates/workspace/src/{modal_layer,toast_layer}.rs` | eigene wiederverwendbare Layer-Typen |
| Settings-Modell | `crates/settings_content/*`, `merge_from.rs` | typisierter Baum + `MergeFrom` |
| Settings-Store | `crates/settings/src/settings_store.rs`, `settings_macros` | `Settings`-Trait + `RegisterSetting`-Derive + `inventory`, Layer-Merge |
| JSON-Edit | `crates/settings_json` (`update_value_in_json_text`) | kommentar-/format-erhaltende Surgical-Edits |
| Settings-UI-Gen | `crates/settings_ui/src/settings_ui.rs`, `page_data.rs` | `SettingField<T>{ pick }` + `SettingFieldRenderer`-Registry pro Typ, `SettingsPageItem` |
| Keymap | `crates/settings/src/keymap_file.rs`, `base_keymap_setting.rs`, `assets/keymaps/*` | `keymap.json` mit Kontexten + Chords, Validatoren |
| Theme | `crates/theme/src/registry.rs`, `theme.rs`, `icon_theme.rs`, `crates/theme_settings` | `ThemeRegistry`, JSON-Familien, Icon-Themes, Dichte-Layer |
| UI-Kit + Gallery | `crates/ui/src/components/*`, `crates/component`, `crates/component_preview` | Primitive-Set + Live-Preview-Seite |

---

## 3. Roadmap — neue Phasen 15–21

Nummerierung folgt dem Bestand: Phase `NN` → Tasks `T{NN+1}-{OOO}`.
Jede Phase endet mit grünen Gates (`cargo fmt --check`, `check`,
`clippy -D warnings`, `test`) und `handshake.md`-Update.

### Phase 15 — Crate-Zerlegung & Fundament  ·  `tasks/phase-15-crate-split/`  ·  **P0-1**

Ziel: den `ui`-Monolithen (48k Z., `settings.rs` 5 957, `workspace.rs` 4 076,
`app_shell.rs` 2 983) in fokussierte Crates zerlegen. **Reine Moves, null
Verhaltensänderung**, Gate grün nach jedem Crate.

| Task | Titel | Kern-Ziel | Abh. |
|---|---|---|---|
| **T16-001** | ADR + Ziel-Crate-Graph | Abhängigkeitsregeln (§2.1) + Layout-Vertrag (§2.3) als `docs/architecture.md` festschreiben; ROADMAP-Vision auf neue Philosophie umstellen | — |
| **T16-002** | `labonair-gpui-ext` + `labonair-ui-kit` (Skeleton) | `crates/ui/src/components/*` → eigener Crate, prelude-Crate anlegen, Re-Exports; alle Call-Sites umbiegen | T16-001 |
| **T16-003** | `labonair-notifications` extrahieren | aus `ui/notifications.rs`; von Panels + Shell nutzbar | T16-002 |
| **T16-004** | `labonair-command-palette` extrahieren | aus `ui/command_palette.rs` + Shortcut-/Keybind-Modell | T16-002 |
| **T16-005** | `labonair-panel` Contracts-Crate | leere `Panel`/`StatusItem`-Traits + Registry-Typen (Signaturen von Zed), noch ungenutzt — bricht künftige Zyklen | T16-001 |
| **T16-006** | `labonair-workspace` extrahieren | `workspace.rs` + `pane.rs` + Tab-Content-Views; `pane_group` als eigenes Modul angelegt | T16-002, T16-005 |
| **T16-007** | `labonair-settings-ui` extrahieren | `ui/settings.rs` → eigener Crate; `FIELDS`/`SECTION_GROUPS` bleiben vorerst unverändert | T16-002 |
| **T16-008** | Panel-Crates ausgliedern | `panel-explorer`, `panel-scm`, `panel-git-graph`, `panel-hosts`, `panel-snippets`, `panel-ai` — je aus der zugehörigen `ui/*.rs` | T16-006 |
| **T16-009** | `labonair-shell` + `labonair-app` schlank | `AppShell` → `shell`-Crate; `crates/ui` wird Rest-Fassade oder entfällt; `app` nur noch Bootstrap | T16-006, T16-007, T16-008 |
| **T16-010** | Build-Hygiene + Baseline | per-Crate-clippy in CI, Crate-Graph azyklisch prüfen (`cargo-depgraph`), `cargo check`-Zeit als Baseline dokumentieren | T16-009 |

### Phase 16 — Root-Objekt & Registries  ·  `tasks/phase-16-registries/`  ·  **P0-2, P3**

Ziel: God-Object → Registries + dünne Shell; Dock-System mit mehreren Panels je
Dock + Bottom-Dock; Overlay-Ebenen.

| Task | Titel | Kern-Ziel | Abh. |
|---|---|---|---|
| **T17-001** | `Panel`-Trait + `PanelRegistry` | Port `dock.rs`; `enum SidebarPanel` entfernen, Panels registrieren sich | Phase 15 |
| **T17-002** | `Dock`-Modell (L/R/B) | mehrere Panels je Dock, aktives Panel, Zoom, Resize, Persistenz (`DockData`-Äquiv.); **Bottom-Dock neu** | T17-001 |
| **T17-003** | `StatusItem`-Trait + `StatusItemRegistry` | self-describing `placement()` + `hide()` (Port `HideStatusItem`); ersetzt `render_bar_item`-`match` | T17-001 |
| **T17-004** | `PaneGroup` rekursiver Split-Baum | Port `pane_group.rs` + Split-Persistenz | T17-001 |
| **T17-005** | `ModalLayer` + `ToastLayer` | Workspace-Layer; Command-Palette / Bookmarks / Updater ziehen in `ModalLayer` | T17-001 |
| **T17-006** | `AppShell` → reine Komposition | Titlebar + Docks + Workspace + StatusBar + Modal/Toast; `drain_pending_*` raus (`subscribe_in`/`defer`); ~20 Felder → Registries | T17-002, T17-003, T17-005 |
| **T17-007** | `CommandRegistry` | `.on_action`-Kette → registrierte Command-Handler; Palette + Keymap teilen die Registry | T17-005 |
| **T17-008** | `AppEvent`-Bus entscheiden | an `cx.subscribe`-Brücke hängen (Backend→UI-Events) **oder** streichen — Entscheidung + Umsetzung | T17-006 |

### Phase 17 — Neues Layout & Statusbar-Personalisierung  ·  `tasks/phase-17-layout/`  ·  **Philosophie + Layout-Vertrag**

Ziel: den Layout-Vertrag (§2.3) umsetzen.

| Task | Titel | Kern-Ziel | Abh. |
|---|---|---|---|
| **T18-001** | Titlebar-Redesign | nur Tabs + **ein** Icon-Button rechts → Dropdown (`Settings…`, `Profile` Platzhalter, erweiterbar). Inline-Suche + `⋯`-Menü + Aktions-Buttons raus. Ampel-Inset bleibt | Phase 16 |
| **T18-002** | Suche als Overlay | `Cmd+F` → transientes Such-Overlay in der Modal/Overlay-Ebene, keine permanente Chrome-Fläche | T17-005, T18-001 |
| **T18-003** | Statusbar links: Panel-Steuerung | Toggles für **alle** registrierten Panels (aus `PanelRegistry`), aktives hervorgehoben; 44px-Rail entfernen | T17-002, T17-003 |
| **T18-004** | Statusbar rechts: Info-Dropdowns | Notifications (Badge-Dropdown), CWD-Breadcrumb, Updater, Transfers, Agent-Access — alle als `StatusItem` | T17-003 |
| **T18-005** | Rechtsklick-Personalisierung | Kontextmenü je Statusbar-Item: `Nach links` / `Nach rechts` / `Ausblenden`; Persistenz `statusBarItemPlacements` | T18-003, T18-004 |
| **T18-006** | Migrator `barItemPlacements` | altes Schema (Titlebar+Statusbar) → `statusBarItemPlacements`; Titlebar-Items auf Statusbar-Default abbilden; einmalig, idempotent | T18-005 |
| **T18-007** | Philosophie + Personalisierungs-Seite | Philosophie in `ROADMAP.md` + `CLAUDE.md` verankern; Settings-Seite „Personalisierung": Statusbar-Layout + Panel-Sichtbarkeit editierbar (spiegelt RMB-Menü) | T18-005 |

### Phase 18 — Settings-System Zed-Style  ·  `tasks/phase-18-settings-core/`  ·  **P0-3, P1**

Ziel: typisierter Merge-Baum + generierte UI + JSON-Editor + Keymap-Datei +
Projekt-Settings. Die parallele `FIELDS`-Tabelle verschwindet.

| Task | Titel | Kern-Ziel | Abh. |
|---|---|---|---|
| **T19-001** | `labonair-settings-content` | typisierter `SettingsContent`-Baum (general/appearance/terminal/editor/file_manager/connections/workspace/ai) + `MergeFrom`-Trait; `Preferences` wird abgeleitet/ersetzt | Phase 15 |
| **T19-002** | `SettingsStore` + Layer-Merge | default → user → OS → (projekt) → sprache; `Settings`-Trait + Registrierung (`inventory`/Derive, Port `settings_macros`); Live-fs-Watch | T19-001 |
| **T19-003** | Projekt-/Ordner-Settings | `.labonair/settings.json` pro geöffnetem Verzeichnis in den Merge einklinken (`LocalSettingsKind`-Äquiv.); Use-Case: pro Repo Default-Host / Startlayout / Snippet-Set | T19-002 |
| **T19-004** | Settings-UI aus Modell generieren | `SettingField<T>{ pick }` + `SettingFieldRenderer`-Registry pro Rust-Typ (bool→Switch, enum→Dropdown, Zahl→NumberField, String→Input, Sondertypen→Custom); `FIELDS` + `SECTION_GROUPS` **löschen** | T19-002, T16-007 |
| **T19-005** | Rohe `settings.json` editierbar | Port `settings_json::update_value_in_json_text` (kommentar-/format-erhaltend); „Open Settings (JSON)"; GUI + JSON gleichberechtigt | T19-002 |
| **T19-006** | JSON-Schema-Generierung | `schemars` für alle Bereiche → Validierung jetzt, Editor-Autocomplete später | T19-001 |
| **T19-007** | Globale Settings-Suche | Fuzzy-Index über alle Seiten (Port `settings_ui`) | T19-004 |
| **T19-008** | Keymap als Datei | `keymap.json` mit Kontexten + Chords; Loader/Validator (reduzierter Port `keymap_file.rs`); `enum ShortcutId` nur noch Default-Quelle | T17-007 |
| **T19-009** | Settings-Migrator | `labonair-settings.json` (`preferences`/`editor`/`mcp`) → `SettingsContent`; Keybind-Overrides → `keymap.json`; einmalig, Backup | T19-004, T19-008 |

### Phase 19 — UI-Kit & Theme-System  ·  `tasks/phase-19-ui-kit/`  ·  **P2 + Theme**

Ziel: ein Primitive-Set, überall genutzt; Theme/Icon-Registry.

| Task | Titel | Kern-Ziel | Abh. |
|---|---|---|---|
| **T20-001** | `ui-kit` Primitive-Set vervollständigen | List/ListItem/ListHeader, Dropdown/Select, Dialog, Popover, Disclosure, Table, Tabs, SegmentedControl, Tooltip, Divider, Indicator/Badge, Banner, KeybindingHint, Kbd — auf `gpui-component` wo vorhanden, sonst selbst; einheitliche Token-Anbindung | Phase 15 |
| **T20-002** | View-Migration Welle 1 | Terminal-View, Editor-View, Explorer, SCM auf `ui-kit`; hand-rolled `div`/`btn`/Field raus | T20-001 |
| **T20-003** | View-Migration Welle 2 | Hosts, Snippets, AI-Chat, SFTP, Git-Graph, Settings-UI | T20-002 |
| **T20-004** | Component-Gallery | Debug-Fenster/-Route mit allen Primitives in allen Zuständen (Idee `component_preview`); Abgleich mit `reference-src` | T20-001 |
| **T20-005** | `ThemeRegistry` + JSON-Theme-Familien | mehrere Themes, Laufzeit-Umschaltung, User-Theme-Ordner (Port `theme/src/registry.rs`); aus „ein Custom-Theme" wird Registry | Phase 15 |
| **T20-006** | Icon-Themes | `IconName` + `file_icon`-Map → JSON-Icon-Theme, umschaltbar (Port `file_icons`/`icon_theme`) | T20-005 |
| **T20-007** | `theme_settings`-Layer | UI-Dichte / Font-Skalen / Corner-Radius als Theme-Overrides statt Einzel-Prefs; an Settings-Baum anbinden | T20-005, T19-002 |

### Phase 20 — Performance & Modularitäts-Abnahme  ·  `tasks/phase-20-perf-signoff/`

| Task | Titel | Kern-Ziel | Abh. |
|---|---|---|---|
| **T21-001** | Render-Pfad-Profiling | unnötige `cx.notify` + per-Frame-Allokationen (`build_palette_data`, `sync_live_bridge`) reduzieren; Messwerte dokumentieren | Phase 17 |
| **T21-002** | Build-Budget | `check`/`clippy`-Zeit vorher/nachher; Crate-Graph azyklisch (`cargo-depgraph`) | Phase 15 |
| **T21-003** | Startup-Profiling | Zeit bis erstes Frame + Speicher-Baseline; Vergleich zu ROADMAP-Erfolgskriterium 15 | Phase 17 |
| **T21-004** | Modularitäts-/Personalisierungs-Abnahme | Checkliste gegen die Philosophie (Panels frei anordbar, Statusbar-Items L/R, Themes/Keymap als Datei, Projekt-Settings greifen) + Regressions-Durchlauf Parität vs. `reference-src` | alle Rework-Phasen |
| **T21-005** | Architektur-Doku | `docs/architecture.md` finalisieren (Crate-Graph, Registries, Layout-Vertrag, Settings-Schichten); `handshake.md` konsolidieren | T21-004 |

### Phase 21 — Decision-Gate  ·  `tasks/phase-21-gpui-decision/`  ·  **P4**

| Task | Titel | Kern-Ziel | Abh. |
|---|---|---|---|
| **T22-001** | vendored `gpui` — Entscheidung | Kriterien sammeln (welche geplanten Features brauchen Multi-Window / Fensterlevel / CSD-Linux), Aufwand + `gpui-component`-Kompatibilität prüfen, Empfehlung schreiben. **Nur umsetzen, wenn ein konkretes Feature es erzwingt.** | Phase 20 |

---

## 4. Sequencing & Migrationsstrategie

1. **Phase 15 zuerst und am Stück.** Reine Datei-Moves + Re-Exports, kein
   Logikeingriff. Nach jedem extrahierten Crate: alle vier Gates grün + Commit.
   So bleibt jederzeit `cargo run` lauffähig und Reverts sind billig.
2. **Traits vor Umbau.** `labonair-panel` (T16-005) entsteht als leerer
   Contracts-Crate, bevor irgendein Panel darauf umzieht — verhindert
   Zyklen und großflächige Rückbauten.
3. **Ein Feature-Flag für das neue Layout.** Phase 17 hinter
   `--features new-shell` entwickeln; altes `AppShell`-Rendering bleibt bis zur
   Abnahme parallel lauffähig (dann gelöscht).
4. **Settings-Migrator ist Pflicht, nicht optional.** Bestehende
   `labonair-settings.json` (`preferences` / `editor` / `mcp` /
   `barItemPlacements`) muss nach dem Rework weiter laden — T19-009 + T18-006
   schreiben einmalige, idempotente Migratoren mit `.bak`.
5. **View-Migration inkrementell.** T20-002/003 pro View, nie „alle auf einmal";
   jede migrierte View wird visuell gegen `reference-src` geprüft.
6. **Parität-Reste danach.** Voice/Whisper + T15-006-Abnahme laufen nach Phase 21
   auf dem neuen Fundament (Nutzer-Entscheidung).

---

## 5. Risiken & Gegenmaßnahmen

| Risiko | Gegenmaßnahme |
|---|---|
| Großer Move kollidiert mit in-flight-Arbeit | Rework auf eigenem Branch, Phase 15 in wenigen Tagen am Stück, kein Parallel-Feature |
| `gpui-component` 0.5.1 exportiert nicht alle Primitives | `ui-kit` kapselt: vorhandenes wrappen, Rest selbst bauen — Call-Sites sehen nur `labonair_ui_kit::*` |
| Zirkuläre Crate-Deps (Panel ↔ Workspace) | `labonair-panel` Contracts-Crate ohne Workspace-Deps; T16-010 prüft Azyklizität in CI |
| `inventory`/`linkme`-Registrierung plattformabhängig | früh auf macOS **und** Linux verifizieren (T19-002) |
| Statusbar trägt jetzt Panel-Toggles → UX-Bruch ggü. Referenz | bewusste Abweichung laut Philosophie; T18-003/007 mit Nutzer-Sichtprüfung |
| Settings-Migrator verliert Nutzerdaten | idempotent + `.bak` + Round-Trip-Test (alt → neu → alt) |
| GPUI-0.2.2-Limits blockieren Layout-Ziel | Layout-Vertrag kommt mit einem Fenster aus; Multi-Window erst in T22-001 |
| Scope-Kriechen (jede Phase zieht „nur schnell noch…") | Task-Akzeptanzkriterien halten sich strikt an §2; Erweiterungen = neue Tasks |

---

## 6. Mapping — deine Vorgaben → Tasks

| Deine Vorgabe | Tasks |
|---|---|
| Crate-Zerlegung deutlich mehr/besser | Phase 15 (T16-001 … T16-010) |
| Root-Objekt verbessern, mehrere Registries statt God-Object | T17-001, T17-003, T17-006, T17-007 |
| Panel-/Dock-System (Zed-Stil) | T17-001, T17-002, T17-004 |
| Settings überarbeiten | Phase 18 komplett (T19-001 … T19-009) |
| Komponenten überall integrieren, einheitliches UI-System | T20-001, T20-002, T20-003, T20-004 |
| Theme-System | T20-005, T20-006, T20-007 |
| App = Header + Workspace + Statusbar + Side Panels (+ Modals), simple Struktur | T17-005, T17-006, T18-001 |
| Statusbar-Items per Rechtsklick links/rechts wählbar | T18-005 (+ Migrator T18-006, Settings-Seite T18-007) |
| Neue Philosophie verankern | §1 dieses Berichts + T16-001, T18-007 |
| Titlebar: nur Tabs + 1 Icon-Button rechts (Settings/Profile-Dropdown) | T18-001 |
| Statusbar: Panel-Steuerung + Info-Dropdowns (Notifications/Badges/CWD) | T18-003, T18-004 |
| Empfehlungen P0-1 / P0-2 / P0-3 | Phase 15 / Phase 16 / T19-004 |
| Empfehlung P1 (JSON-Editor, Schema, Keymap-Datei, Projekt-Settings) | T19-003, T19-005, T19-006, T19-008 |
| Empfehlung P2 (UI-Kit, Gallery) | T20-001 … T20-004 |
| Empfehlung P3 (`drain_pending_*` weg, Event-Bus klären) | T17-006, T17-008 |
| Empfehlung P4 (vendored gpui) | T22-001 (Gate, offen) |

---

## 7. Offene Detail-Entscheidungen (nicht blockierend — Default gesetzt)

1. **SFTP-Browser:** bleibt Tab-View (kein Dock-Panel). Default: **Tab-View
   lassen**; als Panel nachrüstbar.
2. **Panel-Crates einzeln vs. ein `labonair-panels`-Sammelcrate:** Default
   **einzeln** (max. Modularität); reversibel, falls Compile-Overhead stört.
3. **macOS-Menüleiste parallel zum Titlebar-Dropdown:** Default **beide** — native
   Menüleiste (Parität) + Dropdown (plattformübergreifend/auffindbar).
4. **`labonair-terminal-view` / `editor-view` aus `workspace` herauslösen:**
   Default **vorerst in `workspace`**; eigener Crate nur bei Bedarf (kein
   Blocker).

---

## 8. Nächster Schritt

Nach deiner Freigabe dieses Berichts:

1. `tasks/ROADMAP.md` um die Phasen 15–21 + die neue Philosophie erweitern.
2. Die Task-Dateien `tasks/phase-15-*/T16-*.md` … `tasks/phase-21-*/T22-001.md`
   im bestehenden Format anlegen (`## Status` = `📋 Geplant`, `## Ziel`,
   `## Kontext` mit Zed-Datei-Pointern aus §2.4, `## Anweisungen`,
   `## Akzeptanzkriterien`, `## Warnungen`).
3. Mit **T16-001** starten (ADR + `docs/architecture.md` + ROADMAP-Vision).
