# Vergleichsbericht — Zed-Referenz vs. Labonair-rust (pure Rust)

Erstellt: 2026-09-03
Vergleichsobjekte:

* **Zed** — `zed-refrence/zed/` (lokaler Checkout der Zed-Quellen, ~300 Crates)
* **Labonair-rust** — dieser Ordner, `crates/` (7 Crates, ~87k Zeilen Rust)

> Abgrenzung: Die bereits vorhandenen `vergleichsbericht-subagent-1..4.md`
> vergleichen den Port gegen `reference-src/` (die alte Tauri/React-App). Dieser
> Bericht ist **komplementär** und vergleicht ausschließlich gegen **Zed** als
> GPUI-Referenzarchitektur. Zed ist eine IDE, Labonair ein Terminal-/DevOps-Tool
> — verglichen werden *Bauweise und Muster*, nicht der Funktionsumfang.

---

## 0. TL;DR

| Dimension | Labonair-rust | Zed | Bewertung |
|---|---|---|---|
| Crate-Zerlegung | 7 Crates, `ui` ist Monolith (48k Zeilen, 40 Dateien) | ~300 Crates, jede Funktion/Panel/Provider eigener Crate | Zed deutlich sauberer; Labonair OK für die Größe, aber `ui` muss zerlegt werden |
| Root-Objekt | `AppShell` = God-Object mit ~20 `Entity`-Feldern, manuelles `observe()`-Verdrahten | `Workspace` + Trait-Registries (`Panel`, `StatusItemView`), keine zentrale Verdrahtung | Zed skaliert besser; Labonair-Muster bricht ab ~10 Panels |
| Panel-/Dock-System | fixe `enum SidebarPanel` (6 Varianten), 2 Dock-Slots, manuelle Resize-Handler | `Dock` (links/rechts/unten) × `Vec<Box<dyn Panel>>`, verschiebbar, zoombar, persistiert | Zed klar überlegen |
| Settings-Modell | **eine** handgeschriebene `Preferences`-Struct (~170 Felder), 1 JSON-Datei | `SettingsContent`-Baum + `Settings`-Trait + Layer-Merge (default→user→OS→profile→project→sprache) | Zed mächtiger; Labonair einfacher, aber ohne Layering/Projekt-Settings |
| Settings-UI | **paralleles** handgepflegtes `FIELDS: &[FieldDef]`-Array (131 Einträge) | UI generiert aus typisiertem Feld + Renderer-Registry (`SettingField<T> { pick: fn(&Content)->&T }`) | Labonair-Ansatz driftet (170 Struct-Felder vs. 131 UI-Felder) |
| Komponenten-Bibliothek | 5 Dateien (~880 Z.): `button`, `context_menu`, `icon`, `text_field` + 3 re-exportierte `gpui-component`-Primitives | eigener `ui`-Crate (Dutzende Primitives) + `component`-Crate mit Preview-Registry | Zed viel reicher; Labonair dünn, viel Hand-CSS pro View |
| Theme | oklch aus `globals.css` einmalig → `Theme`-Struct aus `Hsla`; 1 Custom-Theme importierbar | `theme`-Crate + `ThemeRegistry` + JSON-Theme-Familien + `theme_settings` + Icon-Themes | Zed: dynamisch, erweiterbar; Labonair: statisch, ausreichend |
| Keymap | fixes `enum ShortcutId` + `BTreeMap<slug,String>`-Overrides | `keymap.json` mit Kontexten, Chords, Basis-Keymaps (VSCode/JetBrains/…), Vim-Keymap, Validatoren | Zed: Daten-getrieben; Labonair: hartkodiert, keine Chords |
| GPUI-Bezug | `gpui = "0.2.2"` von crates.io (API-Deckel) | eigenes `gpui` (+ `gpui_macos`/`gpui_linux`/`gpui_wgpu` …) am Tip | Labonair durch Release-API limitiert (Multi-Window, Fensterlevel) |

**Kernaussage:** Die Rust-Portierung ist funktional weit, aber architektonisch
noch im „ein großer `ui`-Crate, ein großes Root-View"-Stadium. Zed zeigt das
Zielbild: Trait-basierte Registries statt Enums/God-Object, Settings als
typisierter Merge-Baum mit generierter UI, jede Feature-Einheit ein Crate.

---

## 1. Systemarchitektur / Aufbau

### 1.1 Crate-Layout

**Labonair-rust** (`Cargo.toml`):

```
crates/app       128 Z.   – nur main(): Tokio-Runtime, Backend-Init, ein Fenster
crates/ui      48 623 Z.   – ALLES an UI: app_shell, workspace, settings, explorer,
                             git, ai_chat, terminal-view, sftp, hosts, updater, …
crates/backend 21 970 Z.   – modules/{fs,ssh,sftp,pty,git,hosts,snippets,mcp,
                             settings,updater,bookmarks,…}
crates/ai       6 758 Z.   – Provider-Adapter, SSE-Streaming, Tool-Registry
crates/terminal 4 140 Z.   – alacritty-Engine, PTY, Render, Shell-Integration
crates/editor   4 039 Z.   – Buffer, TreeSitter-Syntax, Vim, Suche, Diff
crates/theme    1 748 Z.   – oklch→Hsla-Tokens, Font-Scan, Theme-Import
```

Der `ui`-Crate ist ein **Monolith**. Einzeldateien:
`settings.rs` 5 957, `workspace.rs` 4 076, `app_shell.rs` 2 983, `hosts.rs`,
`ai_chat.rs` … jeweils vierstellig. Kein `pane`/`dock`/`status_bar` als eigene
Einheit — alles liegt in `app_shell.rs` bzw. `workspace.rs`.

**Zed**: ~300 Crates. Relevante Muster:

* Jedes Panel = eigener Crate: `project_panel`, `outline_panel`, `git_ui`,
  `terminal_view`, `agent_ui`, `collab_ui`, …
* Settings in **fünf** Crates zerlegt: `settings`, `settings_content`,
  `settings_ui`, `settings_json`, `settings_macros`.
* `gpui` selbst zerlegt: `gpui`, `gpui_macos`, `gpui_linux`, `gpui_wgpu`,
  `gpui_web`, `gpui_tokio`, `gpui_macros`.
* Jeder LLM-Provider ein Crate: `anthropic`, `open_ai`, `ollama`, `mistral`,
  `bedrock`, `deepseek`, … (Labonair: alles in `crates/ai/src/adapters.rs`).
* Workspace-Bausteine getrennt: `workspace/src/{dock,pane,pane_group,
  status_bar,toolbar,modal_layer,toast_layer,notifications,persistence}.rs`.

**Warum unterschiedlich:** Zed ist ein Jahre altes Mehr-Team-Projekt mit
Compile-Zeit- und Ownership-Druck; die Crate-Grenzen erzwingen klare APIs.
Labonair-rust ist eine Ein-Personen-Portierung „Task für Task" — der `ui`-Crate
ist über die Roadmap organisch gewachsen. Für die aktuelle Größe noch
handhabbar, aber `app_shell.rs`/`settings.rs`/`workspace.rs` sind bereits an der
Schmerzgrenze.

### 1.2 Objekt-/Entity-Modell

Beide nutzen dasselbe GPUI-Fundament: `Entity<T>`, `Context<T>`, `cx.observe` /
`cx.subscribe` / `cx.emit`, `Render`-Trait, Actions.

**Labonair — `AppShell` als zentraler Koordinator** (`app_shell.rs:141`):

* Hält ~20 `Entity`-Felder direkt (`workspace`, `explorer`, `bookmarks`,
  `git_panel`, `snippets`, `ai_chat`, `command_palette`, `prefs`, `updater`,
  `agent_access`, `notifications`, `theme`, `background`, …).
* `AppShell::new` verdrahtet für **jedes** davon manuell
  `cx.observe(&x, |_,_,cx| cx.notify()).detach()` — ~10 identische Zeilen.
* `render()` beginnt mit `drain_pending_commands` / `drain_pending_bookmarks` /
  `drain_pending_ai` / `sync_live_bridge` — d.h. Sub-View-Events werden in
  `Vec<…>`-Puffer geschrieben und erst im nächsten `render` mit `&mut Window`
  abgearbeitet. Das ist ein **Workaround**, weil die Subscriptions ohne
  `Window` aufgesetzt sind.
* Backend = `labonair_backend::App`, per `.clone()` überall reingereicht;
  Tokio-Runtime wird mit `std::mem::forget(runtime)` am Leben gehalten
  (`main.rs:61`).
* Es gibt einen Broadcast-Event-Bus (`backend.events`, `AppEvent::from_raw`),
  der aktuell **nur geloggt** wird (`spawn_event_logger`, `main.rs:27`) — der
  Kommentar sagt „this is where the GPUI layer will later route events" (TODO).

**Zed — Registries statt God-Object:**

* `Workspace` ist Root, aber Panels werden über das `Panel`-Trait
  (`dock.rs:36`) registriert: `persistent_name`, `position`, `set_position`,
  `default_size`, `min_size`, `initial_size_state`, `PanelEvent`. Der Dock hält
  `Box<dyn Panel>` und weiß nichts über konkrete Panel-Typen.
* Status-Bar-Items implementieren `StatusItemView` (`status_bar.rs:44`) mit
  `set_active_pane_item` + `HideStatusItem` (jedes Item beschreibt selbst, wie
  es per Rechtsklick ausgeblendet wird und persistiert das über
  `update_settings_file`).
* Settings-Registrierung compile-time über `inventory` + `#[derive(RegisterSetting)]`
  — Features rufen `MySettings::register(cx)` und der `SettingsStore` sammelt
  sie ein (`settings_store.rs:407`).
* Kein zentrales manuelles `observe`-Verdrahten: `Settings::get(cx)` liest den
  gemergten Wert, `cx.observe_global::<SettingsStore>` benachrichtigt.

**Konsequenz:** In Zed kostet „neues Panel" = neuer Crate + `impl Panel`. In
Labonair kostet es: `enum SidebarPanel`-Variante + `label()` + `slug()` +
`from_slug()` + Arm in `render_panel_body` + evtl. Bar-Item + Feld in `AppShell`
+ `observe`-Zeile. Das Muster bricht mit steigender Panel-Zahl.

---

## 2. Layout / Fenster-Chrome

### 2.1 Labonair

`AppShell::render` (`app_shell.rs:2851`) baut eine Flex-Spalte:

```
┌ header (40px, optional via zen_mode_show_header) ────────────┐
│  Sidebar-Toggle · Titel · Inline-Suche · ⋯-App-Menü         │
├ body row (flex_1) ──────────────────────────────────────────┤
│ [left_slot]  │        Workspace (Tabs + Split-Panes)  │ [right_slot] │
├ statusbar (32px, optional) ────────────────────────────────┤
│  CWD-Breadcrumb · Pane-Count · Bar-Items                    │
└─ overlays: command_palette, bookmarks, updater, bar_menu,   ─┘
   crumb_menu, subdir_menu, toasts, background_layer
```

* **Dual-Dock**: `left_slot` / `right_slot` (`sidebar_slot::SidebarSlot`), je
  **ein** Panel aus `enum SidebarPanel { Explorer, Snippets, SourceControl,
  Tabs, Hosts, Ai }`. Kein Bottom-Dock. Kein Panel-Verschieben zwischen Slots.
* **Resize**: von Hand — `on_drag_move` + `SidebarResize(BarSide)` + throttled
  Persistenz (`SAVE_THROTTLE`).
* **Unibar / Bar-Items**: `bar_items::Placements`, persistiert als
  `barItemPlacements`-Blob; Items können zwischen Titlebar/Statusbar +
  links/rechts platziert und ausgeblendet werden. Das ist der einzige
  wirklich Zed-artige (daten-getriebene) Layout-Teil — aber laut
  `subagent-1.md` nur teilweise portiert.
* **Fenster-Chrome**: eine transparente Overlay-Titlebar (`appears_transparent`),
  macOS-Ampel-Position manuell gesetzt, `window_min_size` 720×480.
  `crates/ui/src/window_state.rs` persistiert Größe/Position.

### 2.2 Zed

* `Workspace` mit `left_dock`, `right_dock`, `bottom_dock` — je ein `Dock` mit
  **mehreren** Panels, aktivem Panel, Zoom (`PanelEvent::ZoomIn`), Resize
  (`RESIZE_HANDLE_SIZE`), Persistenz (`persistence/model.rs::DockData`).
* `pane_group.rs` — rekursiver Split-Baum (`PaneGroup` / `Member::Axis`), beliebig
  tief, mit `SplitDirection`.
* `status_bar.rs` — `left_items` / `right_items` als `Vec<Box<dyn StatusItemView>>`.
* `modal_layer.rs` + `toast_layer.rs` + `notifications.rs` als getrennte,
  wiederverwendbare Ebenen.
* `PlatformTitleBar` (eigener Crate `platform_title_bar`), plus
  `client_side_decorations` für Linux (Fensterrahmen selbst zeichnen).
* Panels sind per Rechtsklick zwischen den Docks verschiebbar
  (`position_is_valid` / `set_position`).

### 2.3 Bewertung Layout

| | Labonair | Zed |
|---|---|---|
| Bottom-Dock | ✗ (AI ist Panel im Seiten-Slot) | ✓ |
| Panels pro Dock | genau 1 | n, mit Umschalter |
| Panel zwischen Docks bewegen | ✗ | ✓ |
| Split-Layout | ✓ (`crates/ui/src/pane.rs`, 377 Z.) | ✓ (`pane_group.rs`, rekursiv, +Persistenz) |
| Daten-getriebene Bar-Items | ✓ (Unibar, teilw.) | ✓ (`StatusItemView` + `HideStatusItem`) |
| Modal/Toast als eigene Layer | Teils (Toasts ja, Modals ad-hoc) | ✓ eigene Crates/Module |

Labonair ist als **fokussiertes** 3-Zonen-Layout (Header/Body/Status) bewusst
schlanker — das ist für ein Terminal-Tool legitim. Die Schwäche ist nicht das
sichtbare Layout, sondern dass es **hartkodiert** statt über ein Panel-Trait
gebaut ist.

---

## 3. UI- / Komponentensystem

### 3.1 Labonair

`crates/ui/src/components/` (`mod.rs`):

```
button.rs        171 Z.  – button(), ButtonSize, ButtonVariant
context_menu.rs  386 Z.  – context_menu(), MenuItem, MenuClick
icon.rs          266 Z.  – IconName-Enum, Lucide-Glyph-Map, file_icon/folder_icon
text_field.rs     32 Z.  – dünner Wrapper um gpui-component Input
```

Re-exportiert exakt 3 Fremd-Primitives: `Badge`, `Switch`, `Tooltip` aus
`gpui-component`. Alles andere (Listen, Dropdowns, Dialoge, Disclosure, Tabs,
Tabellen, Keybinding-Hints) wird **pro View** aus `div()` + Tailwind-artigen
Stil-Methoden von Hand gebaut. Der `mod.rs`-Kommentar sagt selbst: „The port had
none — every view hand-rolled its own `btn`". Die jüngsten Commits
(`946023f` „unify buttons + breadcrumb menus") arbeiten das gerade nach.

Icons: `IconName`-Enum mit ~18 Lucide-Glyphen, `file_icon` mappt ~90
Dateiendungen. Fest eingebaut, nicht als Icon-Theme austauschbar.

### 3.2 Zed

* `crates/ui` — eigenständiges Design-System: `Button`/`IconButton`/`ButtonLike`,
  `List`/`ListItem`/`ListHeader`, `ContextMenu`, `PopoverMenu`, `DropdownMenu`,
  `Table`, `Disclosure`, `TreeViewItem`, `KeyBinding`/`KeybindingHint`,
  `Indicator`, `Banner`, `Scrollbars`, `Tooltip`, `Divider`, `Switch`, …
  plus `prelude.rs`, `styles/`, `traits/`, `utils/`.
* `crates/component` + `crates/component_preview` — Registrierung + Live-Preview
  aller Komponenten (Design-System-Gallery), damit visuelle Konsistenz prüfbar
  bleibt.
* `crates/icons` + `crates/file_icons` + Theme-Crate: **Icon-Themes** als JSON,
  zur Laufzeit umschaltbar.

### 3.3 Bewertung

Labonair verzichtet bewusst auf `gpui-component` als Voll-Framework und baut nur
das Nötigste. Das hält den Build klein, riskiert aber visuelle Drift (jede View
löst „Sekundär-Button mit Hover" leicht anders). Zed investiert stark in die
Primitive-Ebene + eine Preview-Seite. **Empfehlung** siehe §7.

---

## 4. Settings-System (Kern des Vergleichs)

### 4.1 Labonair — ein typisiertes Struct, eine Datei, parallele UI-Tabelle

**Modell** (`crates/backend/src/modules/settings/preferences.rs`):

* Eine `struct Preferences` mit **~170** `pub`-Feldern, gruppiert per Kommentar
  (`// ── General ──`, `// ── Appearance & Layout ──`, …).
* `#[serde(default, rename_all = "camelCase")]`, jedes Feld `#[serde(default)]`
  → alte Settings-Dateien laden immer.
* Persistenz: **eine** Datei `labonair-settings.json`, Key `"preferences"`.
  Andere Subsysteme legen weitere Keys **in dieselbe Datei**: `"editor"`
  (`settings/editor.rs`), `"mcp"` (`settings/mcp.rs`), `"barItemPlacements"`
  (`settings/mod.rs`).
* Korrupte Datei → `.bak` + Defaults (kann die App nicht „bricken").
* `PreferencesStore` (GPUI-Entity, `settings.rs:297`): `get() -> &Preferences`,
  `set_value(key, json_value)` — round-trippt `Preferences → serde_json::Value →
  Map → Preferences`, persistiert + `cx.notify()` + republiziert
  `GlobalPreferences`-Global. Module ohne Entity-Handle lesen
  `cx.global::<GlobalPreferences>()`.

**UI** (`crates/ui/src/settings.rs`, 5 957 Z.):

* Eigenes **OS-Fenster** (`open_settings_window` → `cx.open_window`), 860px breit,
  Höhe = 80 % Display, geklammert `[580,900]` — 1:1-Port der Tauri-Funktion.
* `CATEGORIES: &[&str]` = **10 hartkodierte** Strings (General, Appearance &
  Layout, Themes, Terminal, Editor, File Manager, Connections, Workspace,
  Shortcuts, AI).
* `FIELDS: &[FieldDef]` — **handgeschriebenes** statisches Array mit **131**
  Einträgen: `{ key, title, desc, category, FieldKind }`.
  `enum FieldKind { Switch, Int{min,max,step}, Float{…centi}, Select(&[&str]),
  FontFamily, Text }`.
* `SECTION_GROUPS` gruppiert Felder pro Kategorie; nicht gelistete Felder fallen
  in einen „leftover"-Block.
* Handgebaute Sonder-Panes: Theme-Grid, Shortcuts-Capture, AI
  Provider/Agents/Directives, MCP-Bridge-Pane.
* GPUI-0.2.2-Limits (im Modul-Doc dokumentiert): kein always-on-top, keine
  Max-Größe, kein Parent-Window → Referenz-Verhalten (Fenster hängt an Main,
  minimiert mit) unportierbar.

**Das strukturelle Problem:** `Preferences` hat ~170 Felder, `FIELDS` hat 131
Einträge. Beide werden **von Hand synchron gehalten**. Ein neues Feld erfordert:
Struct-Feld + `Default` + `FIELDS`-Eintrag + evtl. `SECTION_GROUPS` + evtl.
Verbraucher. Vergessene `FIELDS`-Einträge = Feld existiert, ist aber im UI
unsichtbar (genau das listet `subagent-2.md` abschnittsweise auf).

### 4.2 Zed — typisierter Merge-Baum + Trait + generierte UI

**Modell** (`crates/settings_content/`):

* `SettingsContent` = **ein großer typisierter Baum** aller Settings-Bereiche
  (`editor.rs`, `terminal.rs`, `theme.rs`, `project.rs`, `agent.rs`,
  `language.rs`, `language_model.rs`, `title_bar.rs`, `workspace.rs`, …).
* `merge_from.rs` — `trait MergeFrom`: eine Schicht wird über die andere gelegt.
* `fallible_options.rs` — einzelne Felder können ungültig sein, ohne die ganze
  Datei zu verwerfen.

**Store** (`crates/settings/src/settings_store.rs`, 3 300 Z.):

* `trait Settings { fn from_settings(content: &SettingsContent) -> Self; }` +
  `#[derive(RegisterSetting)]` + `inventory` → jede Feature-Einheit definiert
  ihr eigenes typisiertes Settings-Struct und registriert es.
* Layer-Merge in fester Reihenfolge:
  `default.json` (eingebettetes Asset, **voll dokumentiert**, dient als
  Schema-Doku) ← User-Settings ← Release-Channel-Overrides ← OS-Overrides
  (`for_os`) ← aktives Profil (`for_profile`) ← Projekt-/Ordner-Settings
  (`.zed/settings.json` pro Worktree, `LocalSettingsKind`) ← Sprach-spezifisch.
* Live-`fs`-Watching, JSON-**Schema-Generierung** (Editor-Autocomplete in der
  settings.json), `vscode_import.rs` (VSCode-Settings übernehmen).
* `settings_json` (eigener Crate) — **chirurgische** Text-Edits: `update_settings_file`
  ändert nur den betroffenen JSON-Knoten und **erhält Kommentare + Formatierung**
  des Users.

**UI** (`crates/settings_ui/`):

* `SettingsPage` → `Vec<SettingsPageItem>`:
  `SectionHeader | SettingItem | SubPageLink | DynamicItem | ActionLink`.
* `struct SettingField<T> { pick: fn(&SettingsContent) -> Option<&T>, .. }` +
  `SettingsFieldMetadata` — das Feld **weiß**, wie es aus dem Baum liest.
* `SettingFieldRenderer` — Renderer-Registry **pro Typ** (`bool → Switch`,
  Enum → `EnumVariantDropdown`, Zahl → `NumberField`, `String → SettingsInputField`,
  Theme → `theme_picker`, Font → `font_picker`, …). Neue Bool-Setting = **kein**
  neuer UI-Code.
* Volltext-Suche über **alle** Seiten; GUI und rohe `settings.json`
  gleichberechtigt und synchron.

**Keymap** ist bei Zed komplett separat: `keymap_file.rs` (2 802 Z.),
`keymap.json` mit Kontexten (`context = "Editor && vim_mode == normal"`),
Chords, Basis-Keymaps (`base_keymap_setting.rs`: VSCode/Atom/JetBrains/Sublime),
`vim.json`, Validatoren. Labonair: `enum ShortcutId` + `BTreeMap<slug,String>`
Overrides (`command_palette.rs`), keine Kontexte, keine Chords.

### 4.3 Settings — Direktvergleich

| Merkmal | Labonair-rust | Zed |
|---|---|---|
| Typisiertes Modell | ✓ (`Preferences`, 1 Struct) | ✓ (`SettingsContent`, Baum + n `Settings`-Structs) |
| Layering (default/user/OS/projekt/sprache) | ✗ (nur global) | ✓ 7 Schichten |
| Projekt-/Ordner-Settings | ✗ | ✓ `.zed/settings.json` pro Worktree |
| UI aus Modell generiert | ✗ (parallele `FIELDS`-Tabelle, manuell) | ✓ Renderer-Registry pro Typ |
| Rohe JSON editierbar + Kommentar-erhaltend | ✗ (nur GUI) | ✓ `settings_json` surgical edits |
| JSON-Schema / Autocomplete | ✗ | ✓ generiert |
| Settings-Suche | pro Kategorie/Query (`settings.rs:4200`) | ✓ global über alle Seiten |
| Keymap als Datei mit Kontexten/Chords | ✗ (`ShortcutId`-Enum + Map) | ✓ `keymap.json` |
| VSCode-Import | ✗ | ✓ |
| Fenster | eigenes OS-Fenster (860px), GPUI-0.2.2-limitiert | eigenes Fenster + In-Workspace-Seite |
| Persistenz-Ort | 1 Datei, mehrere Keys | `~/.config/zed/settings.json` + Projekt-Dateien |
| Robustheit | korrupt → `.bak` + Defaults | Feld-granulare `FallibleOption` |

**Wo Labonair besser ist:** drastisch einfacher zu verstehen; ein Blick in
`preferences.rs` zeigt *alle* Settings mit Typ und Default. Für ein Tool ohne
„Projekt"-Konzept ist Layering nicht zwingend.

**Wo Labonair verliert:** die parallele `FIELDS`-Tabelle ist eine dauerhafte
Fehlerquelle (Drift 170↔131), es gibt kein JSON-Editieren, kein Schema, keine
Chords/Kontexte in Keybinds, keine Ordner-Settings (für ein DevOps-Tool wäre
„pro Repo: SSH-Host X, Startlayout Y" durchaus wertvoll).

---

## 5. Terminal / Editor / Backend — kurzer Quervergleich

| Bereich | Labonair-rust | Zed |
|---|---|---|
| Terminal-Engine | `alacritty_terminal` 0.24, `crates/terminal` (4 140 Z.), zentral | `crates/terminal` (alacritty) + `terminal_view`, Nebenrolle |
| PTY | `portable-pty` | `portable-pty` |
| SSH | `russh`/`russh-sftp`, vollwertiger Host-Manager, Tunnel, Config-Parser | `remote`/`remote_server` (SSH-Remote-Dev), **kein** SFTP-Browser |
| SFTP-Browser, Transfer-Queue | ✓ (`backend/modules/sftp`, `ui/transfers.rs`) | ✗ |
| Credential-Vault | ✓ `keyring` (OS-Keychain), Secrets nie in SQLite | über `credentials_provider` (Login-Token) |
| Git | Git-CLI (`GitExecutor`), lokal + über SSH | `git`-Crate + `git_ui` + `git_hosting_providers` (libgit-frei via `gix` teils) |
| Editor | `crates/editor` 4 039 Z.: Buffer, TreeSitter, Vim, Suche, Diff. **Kein LSP** | Riesig: `multi_buffer`, `rope`, `sum_tree`, LSP, Edit-Prediction, Inline-Assist |
| AI | `crates/ai`: alle Provider in `adapters.rs`, Tool-Registry, Subagents, MCP-Server-Seite | `language_models` + Crate pro Provider, `agent`, `acp_thread`, `agent_servers` |
| SQLite | `rusqlite` (bundled) für Hosts/Creds/Snippets | `sqlez` (eigener Wrapper) für Workspace-Persistenz |

Der DevOps-Kern (SFTP, Host-Manager, Tunnel, Transfer-Queue, Snippets,
Credential-Vault, MCP-Bridge) ist **Labonair-Alleinstellung** — Zed hat davon
nichts. Umgekehrt hat Labonair kein LSP, keine Multi-Buffer, keine
Edit-Prediction.

---

## 6. Was ist besser / schlechter

### 6.1 Besser in Labonair-rust

1. **Begreifbarkeit** — 7 Crates, ein Settings-Struct, ein Root-View. Ein neuer
   Entwickler versteht das Grundgerüst an einem Tag.
2. **Fokus** — bewusst DevOps/Terminal: SFTP, Hosts, Tunnel, Creds, Snippets,
   MCP. Keine IDE-Ballast-Systeme (LSP-Locations, DAP, Collab, REPL, …).
3. **Build-Größe** — keine ~300 Crates, kein eigenes vendored `gpui`.
4. **Settings auf einen Blick** — `preferences.rs` ist die vollständige,
   typisierte Wahrheit; keine Suche durch fünf Crates.
5. **Robuste Fehlerbehandlung an den Rändern** — `.bak`-Rettung,
   `Result<T,String>` statt `unwrap`, Korrektheit vor „hübsch".

### 6.2 Schlechter / Risiken

1. **`ui` ist ein Monolith** — `settings.rs` 5 957, `workspace.rs` 4 076,
   `app_shell.rs` 2 983. Merge-Konflikte, schwer testbar, langsame
   Inkremental-Builds im `ui`-Crate.
2. **`AppShell` = God-Object** — ~20 Entity-Felder, manuelles `observe`-Boilerplate,
   `render()` mit `drain_pending_*`-Warteschlangen (Architektur-Geruch: Events
   werden gepuffert statt direkt verarbeitet).
3. **Panel-Set ist ein geschlossenes `enum`** — jedes neue Panel fasst
   `app_shell.rs` an mehreren Stellen an. Kein `Panel`-Trait, kein Bottom-Dock,
   Panels nicht zwischen Docks verschiebbar.
4. **Settings-UI driftet vom Modell** — 170 Felder vs. 131 handgepflegte
   `FIELDS`. Kein Generieren, kein Schema, kein JSON-Editor.
5. **Keybinds hartkodiert** — `ShortcutId`-Enum + flache Map, keine Kontexte,
   keine Chords (`Cmd-K Cmd-S`).
6. **Dünne Komponenten-Ebene** — 5 Dateien; viel Hand-CSS pro View, visuelle
   Drift-Gefahr, keine Component-Gallery zum Gegenprüfen.
7. **Toter/unfertiger Event-Bus** — `AppEvent` wird nur geloggt; entweder nutzen
   oder entfernen.
8. **GPUI-0.2.2-Deckel** — Multi-Window-Feinheiten (always-on-top, Parent,
   Max-Größe) fehlen, weil die crates.io-Release-API sie nicht hat. Zed umgeht
   das mit eigenem `gpui`.
9. **Ein-Crate-AI-Adapter** — alle Provider in `adapters.rs`; bei wachsender
   Provider-Zahl wird das wie Zeds Ansatz (Crate pro Provider) sinnvoller.

---

## 7. Verbesserungsempfehlungen (priorisiert)

### P0 — Struktur, jetzt sinnvoll

1. **`ui`-Monolith zerlegen.** Mindestens:
   `crates/workspace` (workspace + pane + pane_group + dock + status_bar),
   `crates/settings-ui` (aus `settings.rs`), je Panel ein eigenes Modul/Crate
   (`explorer`, `git-ui`, `hosts-ui`, `ai-ui`). Vorbild:
   `zed/crates/workspace/src/{dock,pane,pane_group,status_bar}.rs`.
   → `app_shell.rs` schrumpft auf reine Komposition.

2. **`Panel`-Trait + Registry statt `enum SidebarPanel`.** Dock hält
   `Vec<Box<dyn Panel>>`. Trait-Methoden 1:1 von `zed/crates/workspace/src/dock.rs`
   übernehmen: `persistent_name`, `position`, `set_position`, `default_size`,
   `min_size`, `PanelEvent`. Ermöglicht Bottom-Dock, mehrere Panels pro Dock,
   Verschieben, saubere Persistenz.

3. **Settings-UI aus dem Modell generieren.** Die parallele `FIELDS`-Tabelle
   ersetzen durch:
   * entweder ein `#[derive(SettingsUi)]`-Proc-Macro auf `Preferences`
     (Attribute `#[setting(title=…, desc=…, category=…, range=…)]`),
   * oder Zeds Muster: `SettingField { pick: fn(&Preferences) -> &T }` +
     Renderer-Registry pro Rust-Typ (`bool`, Enum, `u32`, `f32`, `String`).
   Ziel: neues `bool`-Feld ⇒ **null** UI-Code, kein Drift möglich.

### P1 — Settings-Reife

4. **Rohe `settings.json` editierbar machen**, kommentar-erhaltend. Zeds
   `settings_json`-Crate (`update_value_in_json_text`) ist klein und
   herauslösbar; gibt „Open Settings (JSON)" + schema-basierte Validierung.

5. **JSON-Schema generieren** (`schemars`, hat Labonair via `rusqlite`-Umfeld
   noch nicht als Dep) → Autocomplete/Doku, wenn P4 kommt.

6. **Keymap als Daten.** `keymap.json` mit Kontexten + Chords, `ShortcutId`-Enum
   nur noch als Default-Quelle. Reduziertes Vorbild: `keymap_file.rs`. Großer
   Brocken — nach P0.

7. **Optional: Ordner-Settings** (`.labonair/settings.json` pro geöffnetem
   Verzeichnis) für „pro Repo: Default-SSH-Host, Startlayout, Snippet-Set".
   Zeds `MergeFrom` + `LocalSettingsKind` als Blaupause. Nur wenn ein echter
   Use-Case da ist.

### P2 — UI-Politur

8. **Komponenten-Crate ausbauen.** Mehr `gpui-component`-Primitives hinter
   `crate::components::*` kapseln (List, Dropdown, Dialog, Table, Disclosure,
   KeybindingHint). Eine **Component-Gallery**-Debug-Seite (Vorbild
   `zed/crates/component_preview`) zum visuellen Abgleich mit `reference-src`.

9. **Icon-Themes** (JSON, umschaltbar) statt fest eingebautem `IconName` —
   niedrige Prio, aber billig an Zeds `file_icons`/`icon_theme` angelehnt.

10. **`StatusItemView`-artiges Trait für Bar-Items** — jedes Item beschreibt
    selbst Platzierung + „Hide"-Verhalten (`HideStatusItem`), statt zentraler
    `render_bar_item`-`match`-Kaskade in `app_shell.rs`.

### P3 — Architektur-Hygiene

11. **`drain_pending_*` eliminieren.** Ursache: Subscriptions ohne `Window`.
    Lösung: `cx.subscribe_in` / `update_in` mit gehaltenem `WindowHandle`, oder
    `window.defer(cx, …)`. Entfernt vier `Vec`-Puffer + vier Drain-Aufrufe pro
    `render`.

12. **`AppEvent`-Bus: nutzen oder streichen.** Wenn Backend→UI-Events geplant
    sind (z. B. SFTP-Transfer-Fortschritt, Host-Reachability), jetzt an eine
    `cx.subscribe`-Brücke hängen; sonst Dead Code entfernen.

13. **AI-Provider je Modul/Crate** (`ai/src/providers/{anthropic,openai,ollama}.rs`)
    sobald >3 Provider — Zeds Aufteilung zahlt sich bei Provider-spezifischen
    Quirks aus.

### P4 — Fundament (nur mit klarer Kosten/Nutzen-Rechnung)

14. **`gpui` vendored ziehen** (Pin auf Zed-Git-Rev, nur die `gpui*`-Crates als
    Path-Deps) um den 0.2.2-API-Deckel loszuwerden (Multi-Window,
    Fenster-Level, Client-Side-Decorations auf Linux). Achtung: laufende
    `gpui-component`-Kompatibilität + Update-Aufwand. Die CLAUDE.md-Regel „no
    submodule to external **Labonair** repo" betrifft Zed nicht — dennoch ein
    schwerer, dauerhafter Wartungsposten. Erst wenn ein konkretes Feature es
    erzwingt.

---

## 8. Fazit

Die Rust-Version ist **funktional auf einem guten Weg** und in ihrer
DevOps-Ausrichtung Zed sogar voraus (SFTP, Hosts, Tunnel, Creds, MCP). Der
Rückstand ist **architektonisch, nicht funktional**:

* **Größter Hebel:** `ui`-Monolith zerlegen + `Panel`-Trait einführen (P0-1/2).
  Das macht alles danach billiger.
* **Zweitgrößter Hebel:** Settings-UI aus dem `Preferences`-Modell generieren
  (P0-3) — beseitigt eine dauerhafte, schon jetzt sichtbare Drift-Fehlerquelle.
* Alles Weitere (JSON-Editor, Keymap-Datei, Ordner-Settings, Component-Gallery,
  vendored gpui) ist wertvoll, aber klar nachgelagert.

Zed ist nicht als Feature-Vorlage nützlich, sondern als **Muster-Katalog**:
Trait-Registries statt Enums, typisierter Merge-Baum + generierte UI statt
paralleler Tabellen, ein Crate pro Feature-Einheit. Genau diese Muster lassen
sich schrittweise übernehmen, ohne den schlanken Charakter von Labonair
aufzugeben.
