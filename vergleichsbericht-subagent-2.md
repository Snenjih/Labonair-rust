# Vergleichsbericht — Subagent 2: Settings-System (end to end)

Audit der Rust/GPUI-Portierung des Settings-Systems gegen die eingefrorene Referenz
(`reference-src/`). Analyse only — keine Quelldateien geändert.

Referenz-Dateien:
- `reference-src/src/settings/SettingsApp.tsx`, `main.tsx`, `sections/*.tsx`, `components/*.tsx`
- `reference-src/src/modules/settings/{definitions.ts,preferences.ts,store.ts,openSettingsWindow.ts,useThemeStore.ts,lib/*}`
- `reference-src/src-tauri/src/lib.rs` (`open_settings_window`, `settings_window_size`)
- `reference-src/src-tauri/src/modules/settings/mod.rs`

Port-Dateien:
- `crates/ui/src/settings.rs` (2730 Zeilen — `PreferencesStore`, `SettingsView`, `FIELDS`)
- `crates/backend/src/modules/settings/{mod.rs,preferences.rs,editor.rs,mcp.rs}`
- `crates/ui/src/app_shell.rs` (`act_open_settings` → `SettingsView::toggle`)
- `crates/ui/src/menu.rs` (`OpenSettings`, `cmd-,`), `crates/ui/src/command_palette.rs` (`CommandId::OpenSettings`)
- `crates/app/src/main.rs` (`cx.open_window` — einziges Fenster)

---

## 0. Zusammenfassung der drei Beschwerden

| # | Beschwerde | Bestätigt? | Kernbefund |
|---|---|---|---|
| 1 | Settings öffnet als In-App-Panel, nicht als eigenes OS-Fenster | **JA** | `SettingsView` ist ein modales Overlay in `AppShell` (`self.settings.update(cx, \|s, cx\| s.toggle(...))`, `app_shell.rs:434`). Die Referenz baut ein separates `WebviewWindow` mit Label `"settings"` (`lib.rs:170`), 860 px breit, Höhe = 80 % Monitorhöhe (clamp 580–900), `always_on_top`, `parent(main)`, close = hide. GPUI kann Fenster (`cx.open_window` wird in `crates/app/src/main.rs:67` bereits benutzt) — es wurde für Settings nur nie gemacht. Der Code-Kommentar in `settings.rs:5` gibt das offen zu: „GPUI has no child-window story wired here yet". |
| 2 | UI visuell inkongruent, Kategorien vermischt, Navigation falsch | **JA** | Der Port rendert eine flache Liste `FIELDS` (~44 Zeilen), gefiltert per `active_cat`, mit primitiven Controls (Switch als Div, Select = **Click-to-Cycle** statt Dropdown, Int = `−`/`+`-Stepper, Text = Inline-Edit). Keine Sub-Section-Header, keine bedingten Zeilen, keine Slider, kein FontPicker, kein About-Hero, kein Theme-Karten-Grid, keine Background-Image-Verwaltung, kein Bar-Item-Layout, keine Provider/Agents/Directives-UI, kein Suchfeld-Ergebnis-Layout nach Kategorie. Sidebar-Taxonomie weicht ab (siehe §2). |
| 3 | 100+ einzelne Settings fehlen | **JA** | Referenz `Preferences` (store.ts) = **~165 Keys** + `SETTING_DEFINITIONS` = **95 dokumentierte Rows**. Port `Preferences` (preferences.rs) = **46 Felder**, davon nur **~44 in der UI** sichtbar (`FIELDS`). **> 110 Preference-Keys fehlen komplett im Rust-Modell**, weitere ~10 existieren nur als Modell-Feld ohne UI. Zusätzlich 3 Port-Erfindungen ohne Referenz (`terminalOpacity`, `editorRelativeLineNumbers`, `editorTheme:"auto"`) und 1 Key-Rename-Bug (`vimMode` → `editorVimMode`). |

---

## 1. Separate-Window-Mechanismus & GPUI-Replikation

### Referenz (Tauri)

`openSettingsWindow.ts`:
```ts
export type SettingsTab =
  | "general" | "appearance" | "themes" | "terminal" | "file-manager"
  | "editor" | "remote-connections" | "workspace" | "shortcuts"
  | "models" | "agents" | "ai" | "directives" | "security" | "about";
export async function openSettingsWindow(tab?: SettingsTab): Promise<void> {
  await invoke("open_settings_window", { tab: tab ?? null });
}
```

`src-tauri/src/lib.rs`:
- `open_settings_window(app, tab)` — baut `WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("settings.html?tab=…"))`.
- Größe: `settings_window_size()` → Breite fix **860** logical px, Höhe = `monitor_logical_h * 0.8` clamp **[580, 900]**; `min_inner_size(720, 480)`, `max_inner_size(1400, 900)`, `resizable(true)`, `always_on_top(true)`.
- `parent(main_window)` → minimiert/schließt mit dem Hauptfenster.
- macOS: `TitleBarStyle::Overlay` + `hidden_title(true)`. Linux: `decorations(false)` + `transparent(true)` (eigene Titelbar).
- **Fenster wird bei „Schließen" nur versteckt** (`api.prevent_close(); window.hide()`), nicht zerstört — nächstes Öffnen ist instant.
- Bereits offen → `set_size` + `center` + `show` + `set_focus` + `window.emit("labonair:settings-tab", tab)`.
- Menü: `MenuItem::with_id(app, "settings", "Settings...", true, Some("CmdOrCtrl+,"))` (`lib.rs:233`); Menü-Handler `"settings" | "open_settings_2"` ruft `open_settings_window(app, None)`, `"open_settings_ai"` ruft mit `Some("ai")` (`lib.rs:512-521`).
- `SettingsApp.tsx` liest den Ziel-Tab aus `window.location` (`?tab=`) **und** hört auf das `labonair:settings-tab`-Event; Deep-Link-Aliasse: `models|agents|connections|directives → ai`, `bookmarks|command-palette|source-control → workspace`, `layout → appearance`.

### Port (GPUI) — Ist-Zustand

- Kein Settings-Fenster. `SettingsView` lebt als `Entity<SettingsView>` in `AppShell` (`app_shell.rs:138`), wird in `AppShell::new` bei `app_shell.rs:280` erzeugt und als Overlay über der Workspace gerendert.
- Öffnen: `menu::OpenSettings` (`cmd-,`, `menu.rs:172`) → `AppShell::act_open_settings` → `SettingsView::toggle`. Ebenso `CommandId::OpenSettings` („Open Settings", `command_palette.rs:411`).
- Kein Deep-Link zu einem Tab (weder Menü „Settings → AI" noch Palette).
- `SettingsView::on_key` schluckt **alle** Tastatureingaben in ein Suchfeld/Inline-Editor, solange offen — modaler Trap.

### Replikationsplan für GPUI

GPUI unterstützt Multi-Window nativ über `cx.open_window(WindowOptions { … }, |window, cx| …)` — im Port bereits verwendet (`crates/app/src/main.rs:67`). Vorgehen:

1. **Neues Modul `crates/ui/src/settings_window.rs`** (oder `SettingsView` behalten, nur die Präsentation wechseln). `SettingsWindow`-Handle im `AppShell` oder in einem `Global` halten, damit „bereits offen" erkannt wird.
2. `open_settings_window(tab: Option<SettingsTab>, cx: &mut App)`:
   - Wenn `Some(handle)` in Global und `handle.is_active(cx)` → `handle.update(cx, |w, _| w.activate())` + Tab setzen; sonst
   - `let bounds = settings_bounds(cx)` (Breite 860, Höhe `0.8 * display_h` clamp 580..900 — `cx.displays()` / `window.display(cx)` für die Monitorgröße).
   - `cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), window_min_size: Some(size(px(720.), px(480.))), titlebar: Some(TitlebarOptions { title: Some("Settings".into()), appears_transparent: true /* macOS overlay */, .. }), is_movable: true, kind: WindowKind::Normal, .. }, move \|window, cx\| cx.new(\|cx\| SettingsRoot::new(prefs, theme, background, backend, tokio, tab, window, cx)))`.
   - `SettingsRoot` teilt sich `Entity<PreferencesStore>`, `Entity<ThemeStore>`, `Entity<BackgroundStore>`, `Backend`, `TokioHandle` mit dem Hauptfenster (dieselben Entities → Änderungen propagieren beidseitig; `PreferencesStore` publiziert bereits `GlobalPreferences`).
3. **Close = hide** nachbilden: GPUI zerstört das Fenster bei Close. Entweder akzeptieren (State liegt in geteilten Entities, Neubau ist billig) oder `window.on_should_close(|_, _| { hide statt close })` falls verfügbar; ansonsten Fenster-Handle behalten und beim nächsten Open neu bauen.
4. **`cmd-,`** bleibt in `menu.rs`, ruft statt `SettingsView::toggle` das neue `open_settings_window(None, cx)`. Menü-Eintrag „Settings → AI…" ergänzen (Deep-Link `Some(SettingsTab::Ai)`).
5. **macOS**: `TitlebarOptions { appears_transparent: true, traffic_light_position: Some(point(px(19.), px(19.))) }` + eigener Header-Strip (`h-11`, zentrierter Titel „Settings"), analog `SettingsApp.tsx:159`.
6. Das modale `on_key`-Handling entfällt — echtes Fenster hat eigenen Fokus-Scope; die einzelnen Felder brauchen echte `TextInput`/Dropdown-Komponenten (siehe `gpui-component`).

---

## 2. Section-Taxonomie & Sidebar-Navigation

### Referenz — 10 Sidebar-Einträge (`SettingsApp.tsx:62`), Reihenfolge fix

| Sidebar-Label | id | Component | Enthält (Sub-Sections) |
|---|---|---|---|
| General | `general` | `GeneralSection` | About-Hero (Icon, Version, Updater-Button, Links Report/GitHub/Website) · Startup · Quit · Session Restore: Scrollback Persistence · Security · Accessibility · Notifications |
| Appearance & Layout | `appearance` | `AppearanceSection` | Color scheme (3 Kacheln) · Background image (Grid + Import + Drag&Drop + Kontextmenü) · Opacity/Blur/Tint (Slider) · Interface (Corner radius, Host card size — Slider) · Layout (Tab bar location, Sidebar tab info line ToggleGroup, Group by folder, Group single tabs, Show header bar, Show status bar) · Typography (FontPicker, size, line-height) · **Titlebar & Statusbar Items** (`BarItemLayoutSettings` — 4 Gruppen Badges/Panels/Info/AI, je Item Bar+Side+Hidden, „Always show badges", Reset) |
| Themes | `themes` | `ThemeMarketplace` | Installed/Community-Tabs · Search · Import JSON · New Theme (Inline-Creator) · Open Folder · Contribute · Theme-Karten mit Preview/Activate/Delete |
| Terminal | `terminal` | `TerminalSection` | Shell (shell path, default cwd, new tab inherits cwd, confirm close) · Font (family/size/weight/letter-spacing/line-height) · Cursor (style/blink/blink interval) · Layout (pane header/footer) · Composer & Blocks (composer, history popup, argument completion, block terminal, auto-collapse) · Rendering (WebGL) · Bell · Buffer (scrollback) · Input (copy on select, right-click pastes, word separators) · Scrolling (sensitivity, fast-scroll modifier) |
| Editor | `editor` | `EditorSection` | Keybindings (Vim) · Theme (syntax) · Font (family, line-height) · Behaviour (format on save, auto save + delay, tab size) · Indentation (indent with tabs) · Files (max file size) · Display (line numbers, word wrap, bracket matching, cursor position, selection stats, outline, indentation guides) · On Save (trim whitespace, insert final newline) · AI Completion (autocomplete debounce) |
| File Manager | `file-manager` | `FileManagerSection` | Browsing (show hidden, `..` up-folder, explorer hidden by default) · Columns (Size/Modified/Permissions/Type) · Remote Editing (show transfers, max remote file size) · Transfers (concurrent, on name conflict, chunk size, on file error) |
| Connections | `remote-connections` | `ConnectionsSection` | Host Availability (ping interval) · SSH Terminal Sessions (connect timeout, auto-reconnect + delay + max attempts) · Remote File Browsing (explorer poll interval, auto-reconnect, idle timeout, max idle sessions, max cached scopes) · **AI Agent Bridge (MCP)** (enable, port, max command timeout, auto-revoke, notify on activity, setup command + regenerate token) |
| Workspace | `workspace` | `WorkspaceSection` | **Bookmarks** (enable, 4 Row-Actions, primary click behavior, show badge) · **Command Palette** (blur Slider, opacity Slider, open position, animation speed, show recent, history size, search mode, close on outside click) · **Source Control** (refresh interval) |
| Shortcuts | `shortcuts` | `KeyboardShortcutsSection` | Filter · Reset all · Gruppen aus `SHORTCUT_GROUPS`, Klick-to-Record je Zeile, Konflikt-Erkennung + Override, „Set to none" |
| AI | `ai` | `AiSection` | Defaults (Chat model Dropdown, Autocomplete: enable+provider+model id) · **Providers** (`ProviderInstanceCard`, Add-Provider-Dropdown, per-instance API-Keys) · General (Disable AI, Warn on destructive) · Behaviour (auto-open mini, notify on headless, max agent steps, temperature, terminal context lines, max command timeout, max command output) · **Agents** (Custom instructions Textarea, Built-in + Custom Agent-Karten, Agent-Editor-Dialog) · **Directives** (`#handle`-Liste, Directive-Editor-Dialog) |

Standalone-Section-Dateien, die **nicht** von `SettingsApp.tsx` importiert werden (Legacy — Inhalt in Ai/Appearance gefaltet, aber als Deep-Link-Ziele referenziert): `ModelsSection.tsx`, `AgentsSection.tsx`, `DirectivesSection.tsx`, `LayoutSection.tsx` (nur `BarItemLayoutSettings` re-exportiert).

Zusätzliche Referenz-Mechanik:
- **Globale Suche** über `SETTING_DEFINITIONS` (`SettingsApp.tsx:148`): Ergebnisse gruppiert nach `SettingCategory`, mit funktionierenden Switch/Select/Custom-Controls und „Open in …"-Links.
- **`SETTING_DEFINITIONS`** (`definitions.ts`) ist eine parallele, deklarative Registry mit 95 Rows: `id` (= PrefKey), `label`, `description`, `category` (10 `SettingCategory`-Werte, andere Namen als die Sidebar!), `controlType` (`Switch|Select|Input|NumberInput|Custom`), `options`, `targetTab`, `linkLabel`. Diese `SettingCategory`-Enum: `General | Appearance & Layout | Terminal | Editor | Command Palette | File Manager | Connections | Source Control | AI | Bookmarks`.

### Port — 10 Kategorien (`settings.rs:173`), flach

```rust
pub const CATEGORIES: &[&str] = &[
    "General", "Appearance", "Terminal", "Editor", "File Manager",
    "Command Palette", "Source Control", "AI",
    "Keyboard Shortcuts",      // KEYBOARD
    "AI Agent Bridge",         // AGENT_BRIDGE
];
```

Abweichungen zur Referenz-Sidebar:
- **Fehlt: „Themes"** — kein Sidebar-Eintrag. Theme-Verwaltung ist stattdessen unter „Appearance" versteckt (`refresh_themes`, `activate_theme`, `import_theme`, `export_theme`, `delete_theme` in `settings.rs`), aber ohne Karten-Grid/Community-Tab/Preview.
- **Fehlt: „Connections"** als eigener Eintrag — SSH/Explorer-Settings existieren gar nicht (siehe §3). Nur der MCP-Teil überlebt als eigene Top-Level-Kategorie **„AI Agent Bridge"** (in der Referenz eine Sub-Section von „Connections").
- **Fehlt: „Workspace"-Gruppierung** — „Command Palette" und „Source Control" sind Top-Level-Kategorien (Referenz: Sub-Sections von „Workspace"). „Bookmarks" fehlt komplett.
- **„Appearance & Layout"** → im Port nur „Appearance", und praktisch leer (3 Felder: `appFontFamily`, `appFontSize`, `reduceMotion`). Kein Layout, keine Background-Bilder, keine Bar-Items, kein Color-Scheme-Picker (nur `theme` unter „General"), keine `appLineHeight`-UI.
- **Reihenfolge**: Port zieht `theme` nach „General" statt „Appearance".
- Keine Sub-Section-Header innerhalb einer Kategorie — alle Felder einer Kategorie sind eine ununterbrochene Liste (das ist die „vermischt"-Wahrnehmung).
- Kategorie-Suche: `self.search` filtert Feld-Titel, aber es gibt **kein** nach-Kategorie-gruppiertes Ergebnis-Layout wie `SearchResults`.

---

## 3. Vollständige Preference-Matrix (Referenz-Row → Port-Status)

Legende: **OK** = vorhanden & inhaltlich korrekt · **WRONG** = vorhanden, aber falscher Key / falscher Default / falscher Control / falsche Range · **MISSING** = im Rust-`Preferences`-Modell nicht existent · **MODEL-ONLY** = Feld im Modell, aber keine UI-Row.

Port-Modell-Felder (`preferences.rs`, camelCase serialisiert): `theme, restoreWindowState, defaultStartupTab, notifyOnErrors, confirmQuitWithSsh, checkForUpdates, sessionRestore, appFontSize, appLineHeight, appFontFamily, reduceMotion, zenModeShowHeader, zenModeShowStatusbar, terminalShell, terminalFontFamily, terminalFontSize, terminalScrollback, sessionScrollbackLines, scrollbackMaxSizeMb, scrollbackRetentionDays, terminalCursorStyle, terminalCursorBlink, terminalCopyOnSelect, terminalBell, terminalOpacity, editorFontFamily, editorFontSize, editorTabSize, editorWordWrap, editorLineNumbers, editorRelativeLineNumbers, editorIndentWithTabs, editorFormatOnSave, editorVimMode, editorTheme, sftpShowHiddenFiles, sftpFontSize, sftpMaxConcurrentTransfers, commandPaletteSearchMode, commandPaletteShowRecent, gitStatusPollIntervalMs, aiEnabled, aiMaxAgentSteps, aiTerminalContextLines, aiWarnDestructiveCommands, keybinds` (46).

MCP-Prefs (`mcp.rs`, `McpPrefs`): `bridge_port, max_command_timeout_secs, auto_revoke_minutes` (+ notify?) — separat, nicht Teil von `Preferences`.
Editor-Prefs (`editor.rs`, `EditorPrefs`): interne Vim-Optionen `hlsearch/incsearch/smartcase` etc., nicht in der Settings-UI.

### 3.1 General

| Referenz-Row (label) | PrefKey | Control | Default | Port-Status | Anmerkung |
|---|---|---|---|---|---|
| Launch at login | `autostart` | Switch | false | **MISSING** | kein Feld im Modell; Referenz koppelt an `plugin-autostart` OS-State |
| Restore window position & size | `restoreWindowState` | Switch | true | **OK** | |
| Session restore | `sessionRestore` | Switch | false (ref) / **true (port)** | **WRONG (default)** | Port-Default `true`, Referenz `false` |
| Scrollback history (session) | `sessionScrollbackLines` | Select 200/500/1000/2000/5000/0 | 1000 (ref) / **5000 (port)** | **WRONG (default + control)** | Port: Int-Stepper 0..100000 step 500, in Kategorie „Terminal" statt „General"; Default 5000 statt 1000; bedingte Anzeige (nur wenn `sessionRestore`) fehlt |
| Check for updates on launch | `checkForUpdates` | Switch | true | **OK** | |
| Default opening tab | `defaultStartupTab` | Select host-manager/terminal | host-manager (ref) / **terminal (port)** | **WRONG (default)** | Port-Default `Terminal`, Referenz `HostManager` |
| Startup terminal count | `startupTerminalCount` | Select 1/2/3 | 1 | **MISSING** | |
| Max scrollback size (MB) | `scrollbackMaxSizeMb` | NumberInput 1–50 | 10 (ref) / **5 (port)** | **WRONG (default)** | Port Int 1..100 step 1, Default 5 |
| Scrollback retention | `scrollbackRetentionDays` | Select 0/7/30/90/365 | 0 (ref) / **14 (port)** | **WRONG (default + control)** | Port Int 0..365, Default 14 |
| Encrypt stored credentials | `credentialEncryption` | Switch | false | **MISSING** | Referenz ruft `secrets_set_encryption_enabled` |
| Confirm quit with active SSH connections | `confirmQuitWithSsh` | Switch | true | **OK** | |
| Reduce motion | `reduceMotion` | Switch | false | **OK** (aber in Port-Kategorie „Appearance") | Referenz: „General" |
| Notify on errors | `notifyOnErrors` | Switch | false (ref) / **true (port)** | **WRONG (default)** | |
| About-Hero (App-Icon, Name, Version/Build-String, Updater-Button mit Status, Links: Report a problem / GitHub / Website) | — | Custom | — | **MISSING** | komplett |

### 3.2 Appearance & Layout

| Referenz-Row | PrefKey | Control | Default | Port-Status |
|---|---|---|---|---|
| Color theme (JSON theme picker) | `appTheme` | Custom → Themes | "default" | **MISSING** (kein `appTheme`-Feld; Theme-Aktivierung läuft über `ThemeStore`, nicht persistierte Preference) |
| (per-theme variant overrides) | `themeVariantOverrides` | — | {} | **MISSING** |
| Color scheme | `theme` | Select system/light/dark | system | **OK** (aber unter „General" + Select = Cycle statt 3 Kacheln) |
| UI font family | `appFontFamily` | Input / FontPicker | `"Inter Variable", sans-serif` | **WRONG (default)** — Port-Default `""`; kein FontPicker |
| UI font size | `appFontSize` | NumberInput 10–20 | 13 | **OK** (Port Range 9..24) |
| UI line height | `appLineHeight` | NumberInput 1–2 step 0.05 | 1.5 | **MODEL-ONLY** — Feld existiert, keine UI-Row (kein Float-Control im Port) |
| Background image | `backgroundImage` | Input + Grid/Import/DnD | "" | **MISSING** (kein Feld; `BackgroundStore` existiert, aber nicht als persistierte Preference hier) |
| Wallpaper opacity | `backgroundOpacity` | NumberInput/Slider 0–100 | 30 | **MISSING** |
| Image blur | `backgroundBlur` | Slider 0–20 | 0 | **MISSING** |
| Tint color | `backgroundTintColor` | color input | "#000000" | **MISSING** |
| Color tint (opacity) | `backgroundTintOpacity` | Slider 0–100 | 0 | **MISSING** |
| Corner radius | `appCornerRadius` | NumberInput/Slider 0–20 | 5 | **MISSING** |
| Host card size | `hmCardScale` | Slider 85–150 | 100 | **MISSING** |
| Show header bar (zen) | `zenModeShowHeader` | Switch | true | **MODEL-ONLY** — Feld da, keine Settings-UI-Row (nur via `view.zenMode`-Command T13-005) |
| Show status bar (zen) | `zenModeShowStatusbar` | Switch | true | **MODEL-ONLY** — dito |
| Tab bar location | `tabsLocation` | Select titlebar/sidebar | titlebar | **MISSING** |
| Sidebar tab info line | `sidebarTabInfoLine` | Custom (ToggleGroup, max 2 von path/connection/host/uptime/transfer/busy) | [] | **MISSING** |
| Group sidebar tabs by folder | `sidebarGroupByFolder` | Switch | false | **MISSING** |
| Group single tabs too | `sidebarGroupSingleTabs` | Switch | false | **MISSING** |
| Customize titlebar/statusbar/panel layout | `barItemPlacements` | Custom (`BarItemLayoutSettings`) | `DEFAULT_BAR_ITEM_PLACEMENTS` | **MISSING** in Preferences-Modell — Rust hat nur `settings_set_bar_item_placement` (backend `mod.rs`) für roh-JSON, aber keine Settings-UI und keinen typisierten Zugriff |
| Always show badges | `badgesAlwaysVisible` | Switch | true | **MISSING** |
| (migration gate) | `barLayoutMigrated` | — | false | **MISSING** (evtl. bewusst) |
| titlebar icons position | `titlebarsIconsPosition` | (legacy, via bar items) | "auto" | **MISSING** |

### 3.3 Terminal

| Referenz-Row | PrefKey | Control | Default | Port-Status |
|---|---|---|---|---|
| Shell path | `terminalShell` | Input | "" | **OK** |
| Default working directory | `terminalDefaultPath` | Input | "" | **MISSING** |
| New tab inherits current directory | `newTabInheritsCwd` | Switch | true | **MISSING** |
| Confirm before closing terminal tab | `confirmCloseTerminalTab` | Switch | false | **MISSING** |
| Terminal font family | `terminalFontFamily` | FontPicker | `"JetBrains Mono", SFMono-Regular, Menlo, monospace` | **WRONG (default)** — Port `"JetBrains Mono"` |
| Terminal font size | `terminalFontSize` | NumberInput 8–32 | 14 (ref) / **13 (port)** | **WRONG (default)** |
| Font weight | `terminalFontWeight` | Select normal/medium/bold | "normal" | **MISSING** |
| Letter spacing | `terminalLetterSpacing` | NumberInput −2..10 step 0.5 | 0 | **MISSING** |
| Line height | `terminalLineHeight` | NumberInput 0.8–2 step 0.05 | 1.05 | **MISSING** |
| Cursor style | `terminalCursorStyle` | Select block/underline/bar | bar (ref) / **block (port)** | **WRONG (default)** |
| Cursor blink | `terminalCursorBlink` | Switch | true | **OK** |
| Cursor blink interval | `terminalCursorBlinkInterval` | NumberInput 200–2000 | 1000 | **MISSING** |
| Command composer | `terminalComposerEnabled` | Switch | false | **MISSING** |
| History popup | `terminalComposerHistoryPopup` | Switch | false | **MISSING** |
| Argument completion | `terminalComposerArgumentCompletion` | Switch | true | **MISSING** |
| Block terminal | `terminalBlocksEnabled` | Switch | false | **MISSING** |
| Auto-collapse blocks for full-screen apps | `terminalBlocksAutoCollapseOnAltScreen` | Switch | true | **MISSING** |
| Terminal bell | `terminalBell` | Switch | false | **OK** |
| Show pane headers | `terminalShowPaneHeader` | Switch | false | **MISSING** |
| Show pane footer | `terminalShowPaneFooter` | Switch | false | **MISSING** |
| Use WebGL renderer | `terminalUseWebGL` | Switch | true | **MISSING** (in GPUI evtl. N/A — trotzdem dokumentieren) |
| Scrollback buffer | `terminalScrollback` | NumberInput 500–50000 step 500 | 5000 (ref) / **10000 (port)** | **WRONG (default + range)** — Port 1000..200000 |
| Copy on select | `terminalCopyOnSelect` | Switch | false | **OK** |
| Right-click pastes | `terminalRightClickPastes` | Switch | false | **MISSING** |
| Word separators | `terminalWordSeparator` | Input | `" ()[]{}',\"`"` | **MISSING** |
| Scroll sensitivity | `terminalScrollSensitivity` | NumberInput 1–10 | 1 | **MISSING** |
| Fast scroll modifier | `terminalFastScrollModifier` | Select none/alt/ctrl/shift | "alt" | **MISSING** |
| — (Port-Erfindung) | `terminalOpacity` | Int 20–100 | 100 | **PORT-ONLY** — keine Referenz-Row (Referenz-Analogon wäre `backgroundOpacity` fürs ganze UI) |

### 3.4 Editor

| Referenz-Row | PrefKey | Control | Default | Port-Status |
|---|---|---|---|---|
| Vim mode | `vimMode` | Switch | false | **WRONG (key rename)** — Port serialisiert als `editorVimMode`, Referenz-Key ist `vimMode`. Bricht Kompatibilität mit `labonair-settings.json` |
| Syntax theme | `editorTheme` | Select (9 Themes: atomone/aura/copilot/github-dark/github-light/nord/tokyo-night/xcode-dark/xcode-light) | "atomone" (ref) / **"auto" (port)** | **WRONG (default + options)** — Port fügt nicht-Referenz-Wert `"auto"` hinzu, Default `"auto"` |
| Editor font family | `editorFontFamily` | FontPicker | `"JetBrains Mono", SFMono-Regular, Menlo, monospace` | **WRONG (default)** — Port `"JetBrains Mono"` |
| Editor line height | `editorLineHeight` | NumberInput 1.0–3.0 step 0.05 | 1.55 | **MISSING** |
| Indent with tabs | `editorIndentWithTabs` | Switch | false | **OK** |
| Trim trailing whitespace | `editorTrimTrailingWhitespace` | Switch | false | **MISSING** |
| Insert final newline | `editorInsertFinalNewline` | Switch | false | **MISSING** |
| Autocomplete debounce (ms) | `editorAutocompleteDebounceMs` | NumberInput 50–2000 | 350 | **MISSING** |
| Auto save | `editorAutoSave` | Select off/afterDelay/onFocusChange | "off" | **MISSING** |
| Auto save delay | `editorAutoSaveDelay` | NumberInput 100–60000 | 1000 | **MISSING** |
| Tab size | `editorTabSize` | Select 2/4/8 | 2 (ref) / **4 (port)** | **WRONG (default + control)** — Port Int 2..8 step 2 |
| Line numbers | `editorLineNumbers` | Switch | true | **OK** |
| Word wrap | `editorWordWrap` | Switch | false | **OK** |
| Bracket matching | `editorBracketMatching` | Switch | true | **MISSING** |
| Cursor position (statusbar) | `editorShowCursorPosition` | Switch | true | **MISSING** |
| Selection stats | `editorShowSelectionStats` | Switch | true | **MISSING** |
| Outline panel | `editorShowOutline` | Switch | false | **MISSING** |
| Format on Save | `editorFormatOnSave` | Switch | false | **OK** |
| Indentation guides | `editorIndentationGuides` | Switch | true | **MISSING** |
| Max file size (MB) | `editorMaxFileSizeMb` | NumberInput 1–100 | 10 | **MISSING** |
| Editor font size | `editorFontSize` | NumberInput 8–32 | 13 | **OK** |
| — (Port-Erfindung) | `editorRelativeLineNumbers` | Switch | false | **PORT-ONLY** — keine Referenz-Row (Vim-`relativenumber`; Referenz hat das nur intern in `EditorPrefs`) |

### 3.5 File Manager

| Referenz-Row | PrefKey | Control | Default | Port-Status |
|---|---|---|---|---|
| Show hidden files | `sftpShowHiddenFiles` | Switch | false | **OK** |
| Show '..' up-folder entry | `sftpShowUpFolder` | Switch | true | **MISSING** |
| Explorer: Show hidden files by default | `explorerShowHiddenByDefault` | Switch | false | **MISSING** |
| Show Size column | `sftpColumnSize` | Switch | true | **MISSING** |
| Show Modified column | `sftpColumnModified` | Switch | true | **MISSING** |
| Show Permissions column | `sftpColumnPermissions` | Switch | true | **MISSING** |
| Show Type column | `sftpColumnType` | Switch | false | **MISSING** |
| Show remote edit transfers | `sftpRemoteEditShowTransfers` | Switch | true | **MISSING** |
| Max remote file size (MB) | `sftpMaxRemoteFileSizeMb` | NumberInput 1–100 | 5 | **MISSING** |
| Concurrent transfers | `sftpMaxConcurrentTransfers` | NumberInput 1–6 | 2 (ref) / **3 (port)** | **WRONG (default + range)** — Port 1..16 |
| On name conflict | `sftpDefaultConflictResolution` | Select ask/overwrite/skip | "ask" | **MISSING** |
| Transfer chunk size (KB) | `sftpChunkSizeKb` | NumberInput 16–1024 step 16 | 64 | **MISSING** |
| On file error in folder transfers | `sftpOnFolderFileError` | Select ask/skip/abort | "ask" | **MISSING** |
| (font size für File-Browser) | `sftpFontSize` | (nicht in definitions.ts, aber in store.ts) | 13 | **OK** — Port hat `sftpFontSize` als Feld+Row |

### 3.6 Connections (SSH / Explorer / Host Availability) — **komplett fehlend im Port**

| Referenz-Row | PrefKey | Control | Default | Port-Status |
|---|---|---|---|---|
| Ping interval | `hostPingInterval` | Select 10/30/60/120/300/0 | 60 | **MISSING** |
| Connect timeout (s) | `sshConnectTimeoutSecs` | NumberInput 3–60 | 10 | **MISSING** |
| Auto-reconnect SSH sessions | `sshAutoReconnect` | Switch | false | **MISSING** |
| Reconnect delay (s) | `sshAutoReconnectDelay` | NumberInput 1–30 | 5 | **MISSING** |
| Max reconnect attempts | `sshAutoReconnectMaxAttempts` | NumberInput 1–10 | 3 | **MISSING** |
| Explorer: Remote refresh interval | `explorerRemotePollInterval` | Select 10/20/30/60/0 | 20 | **MISSING** |
| Explorer: Auto-reconnect remote sessions | `explorerAutoReconnect` | Switch | false | **MISSING** |
| Explorer: Idle session timeout (min) | `explorerIdleSessionTimeoutMin` | NumberInput 1–30 | 5 | **MISSING** |
| Explorer: Max cached remote sessions | `explorerMaxIdleSessions` | NumberInput 1–10 | 3 | **MISSING** |
| Explorer: Max cached remote folders | `explorerMaxCachedRemoteScopes` | NumberInput 1–20 | 5 | **MISSING** |

### 3.7 AI Agent Bridge (MCP) — Referenz: Sub-Section von „Connections"; Port: eigene Top-Level-Kategorie

| Referenz-Row | PrefKey | Control | Default | Port-Status |
|---|---|---|---|---|
| Enable agent bridge | `mcpBridgeEnabled` | Switch | false | **OK** (in `McpPrefs`, eigene Kategorie) |
| Agent bridge port | `mcpBridgePort` | NumberInput 1024–65535 | 47823 | **OK** (`McpPrefs.bridge_port`) |
| Agent bridge max command timeout (s) | `mcpMaxCommandTimeoutSecs` | NumberInput 5–3600 | 300 | **OK** (`McpPrefs.max_command_timeout_secs`) |
| Agent bridge auto-revoke (min) | `mcpAutoRevokeMinutes` | NumberInput 0–1440 | 0 | **OK** (`McpPrefs.auto_revoke_minutes`) |
| Notify on agent activity | `mcpNotifyOnActivity` | Switch | false | **PARTIAL** — prüfen ob in `McpPrefs`; nicht in `Preferences` |
| Setup command + Regenerate token | — | Custom | — | **PARTIAL** — Port hat `mcp_token`/`mcp_regenerate_token`, UI-Darstellung unklar |

> Anmerkung: In der Referenz ist der „enabled"-State prozessweit via `useAgentAccessStore.bridgeEnabled` geteilt. Port-seitig sind das getrennte `McpPrefs` — nicht Teil von `Preferences`, also nicht über den generischen `set_value`-Pfad und nicht in der globalen Suche.

### 3.8 Source Control

| Referenz-Row | PrefKey | Control | Default | Port-Status |
|---|---|---|---|---|
| Source Control refresh interval | `gitStatusPollIntervalMs` | NumberInput 2000–30000 step 500 | 5000 (ref) / **3000 (port)** | **WRONG (default + range)** — Port 500..30000 |

### 3.9 Command Palette (Referenz: Sub-Section von „Workspace")

| Referenz-Row | PrefKey | Control | Default | Port-Status |
|---|---|---|---|---|
| Background blur | `commandPaletteBlur` | Slider 0–20 | 4 | **MISSING** |
| Palette opacity | `commandPaletteOpacity` | Slider 60–100 | 95 | **MISSING** |
| Open position | `commandPalettePosition` | Select top/high/center | "top" | **MISSING** |
| Animation speed | `commandPaletteAnimation` | Select fast/normal/slow/none | "normal" | **MISSING** |
| Show recent commands | `commandPaletteShowRecent` | Switch | true | **OK** |
| Recent history size | `commandPaletteHistorySize` | NumberInput 3–20 | 5 | **MISSING** |
| Search mode | `commandPaletteSearchMode` | Select contains/startsWith/fuzzy | "contains" | **OK** |
| Close on outside click | `commandPaletteCloseOnOverlayClick` | Switch | true | **MISSING** |

### 3.10 Bookmarks (Referenz: eigene `SettingCategory`, UI unter „Workspace") — **komplett fehlend im Port**

| Referenz-Row | PrefKey | Control | Default | Port-Status |
|---|---|---|---|---|
| Enable path bookmarks | `bookmarksEnabled` | Switch | true | **MISSING** |
| Open in new terminal | `bookmarksActionNewTerminal` | Switch | true | **MISSING** |
| Open in current terminal | `bookmarksActionCurrentTerminal` | Switch | true | **MISSING** |
| Open in current SFTP manager | `bookmarksActionCurrentSftp` | Switch | true | **MISSING** |
| Open in new SFTP tab | `bookmarksActionNewSftp` | Switch | true | **MISSING** |
| Primary click opens | `bookmarksPrimaryClickBehavior` | Select current/new | "current" | **MISSING** |
| Show bookmark count badge | `bookmarksShowBadge` | Switch | true | **MISSING** |

> Anmerkung MEMORY: T12-003 („path bookmarks") gilt lt. `MEMORY.md` als „done" mit `crates/backend/src/modules/bookmarks/` + `crates/ui/src/bookmarks.rs`. Die **7 Bookmark-Preferences** wurden dabei aber **nicht** ins `Preferences`-Modell aufgenommen — die Bookmark-Feature-Toggles fehlen in Settings.

### 3.11 AI (Defaults / Providers / Behaviour / Agents / Directives)

| Referenz-Row | PrefKey | Control | Default | Port-Status |
|---|---|---|---|---|
| AI features / Disable AI | `aiEnabled` | Switch | true | **OK** (Port zeigt es positiv „Enable AI", Referenz-Section „Disable AI" invertiert — kosmetisch) |
| Warn on destructive commands | `aiWarnDestructiveCommands` | Switch | true | **OK** |
| Max agent steps | `aiMaxAgentSteps` | NumberInput 5–50 | 24 (ref) / **12 (port)** | **WRONG (default + range)** — Port 1..50 |
| Temperature | `aiTemperature` | NumberInput 0–1 step 0.1 | 0.7 | **MISSING** (kein Float-Control im Port) |
| Terminal context lines | `aiTerminalContextLines` | NumberInput 50–1000 step 50 | 300 (ref) / **200 (port)** | **WRONG (default + range)** — Port 0..2000 |
| Max command timeout (s) | `aiShellMaxTimeoutSecs` | NumberInput 30–1800 | 300 | **MISSING** |
| Max command output (KB) | `aiShellMaxOutputKb` | NumberInput 64–2048 | 256 | **MISSING** |
| Auto-open Mini window on send | `aiAutoOpenMiniOnSend` | Switch | true | **MISSING** |
| Notify on headless/background commands | `aiNotifyOnHeadlessCommand` | Switch | true | **MISSING** |
| Editor autocomplete | `autocompleteEnabled` | Switch | false | **MISSING** |
| Autocomplete provider | `autocompleteProvider` | Select | "cerebras" | **MISSING** |
| Autocomplete model ID | `autocompleteModelId` | Input | `DEFAULT_AUTOCOMPLETE_MODEL.cerebras` | **MISSING** |
| Chat model / default model | `defaultModelId` | Custom (Dropdown) | `DEFAULT_MODEL_ID` | **MISSING** |
| Custom instructions | `customInstructions` | Input/Textarea | "" | **MISSING** |
| LM Studio: Base URL | `lmstudioBaseURL` | Input | `LMSTUDIO_DEFAULT_BASE_URL` | **MISSING** |
| LM Studio: Model ID | `lmstudioChatModelId` | Input | "" | **MISSING** |
| OpenAI-compatible: Base URL | `openaiCompatibleBaseURL` | Input | `OPENAI_COMPATIBLE_DEFAULT_BASE_URL` | **MISSING** |
| OpenAI-compatible: Model ID | `openaiCompatibleModelId` | Input | "" | **MISSING** |
| MLX: Base URL / Model ID | `mlxBaseURL` / `mlxChatModelId` | Input | `MLX_DEFAULT_BASE_URL` / "" | **MISSING** |
| Ollama: Base URL / Model ID | `ollamaBaseURL` / `ollamaChatModelId` | Input | `OLLAMA_DEFAULT_BASE_URL` / "" | **MISSING** |
| **Providers** (`ProviderInstanceCard`, API-Keys per Instance, Add-Provider-Dropdown) | (Store `providersStore` + Keychain) | Custom | — | **MISSING** |
| **Agents** (Built-in + Custom, Agent-Editor-Dialog: Icon/Name/Description/Instructions) | (Store `agentsStore`) | Custom | `BUILTIN_AGENTS` | **MISSING** |
| **Directives** (`#handle`-Liste, Editor-Dialog: Handle/Name/Description/Content) | (Store `directivesStore`) | Custom | [] | **MISSING** |

### 3.12 Status Bar toggles (Referenz `Preferences`, migriert in `barItemPlacements`) — **alle fehlend**

`statusBarShowExplorerButton`, `statusBarShowSnippetsButton`, `statusBarShowSourceControlButton`, `statusBarShowTabsButton`, `statusBarShowCwdBreadcrumb`, `statusBarShowPreviewUrl`, `statusBarShowAiControls` — alle default `true`, alle **MISSING**.

### 3.13 Sidebar / Host-Manager State (Referenz `Preferences`, kein `SETTING_DEFINITIONS`-Row, aber persistiert)

`sidebarPosition`, `sidebarOpen`, `sidebarActivePanel`, `sidebarRightOpen`, `sidebarRightActivePanel`, `sidebarWidth`, `sidebarRightWidth`, `hmLayout`, `hmSort`, `hmCardScale` — alle **MISSING** aus dem `Preferences`-Modell (der Port hält Sidebar-State evtl. woanders in `AppShell`, aber nicht persistiert über `Preferences`).

### 3.14 Keyboard Shortcuts

| Aspekt | Referenz | Port |
|---|---|---|
| Store | `useKeybindsStore` (`overrides`), separat vom `Preferences`-Store | `Preferences.keybinds: BTreeMap<String,String>` (`""` = disabled) — **anders modelliert, aber funktional äquivalent** |
| UI | `KeyboardShortcutsSection` — Filter, „Reset all", Gruppen aus `SHORTCUT_GROUPS`, Klick-to-Record, Live-Modifier-Anzeige, Konflikt-Erkennung (`findConflict`) mit „Override", „Set to none", Reserved-Detektion | Port hat `capture_keybind`/`overwrite_keybind`/`resolve_conflict`/`Conflict::{Reserved,Shortcut}` + `recording`/`kb_conflict` State in `SettingsView` → **weitgehend portiert** (T13-004). Gruppierung/Filter-Parität nicht verifiziert. |
| Status | **OK (grob)** — bester portierter Bereich |

---

## 4. Zahlen

- Referenz `Preferences` (store.ts): **~165 Keys** (inkl. Sidebar/HM-State, StatusBar-Toggles, AI-Provider-URLs).
- Referenz `SETTING_DEFINITIONS` (definitions.ts): **95 deklarative Rows** über 10 `SettingCategory`.
- Referenz Section-Rows tatsächlich gerendert (inkl. bedingter/Custom): **~110** (Sections enthalten mehr als definitions.ts, z. B. Background-Grid, About-Hero, Providers/Agents/Directives, Bar-Items).
- Port `Preferences` (preferences.rs): **46 Felder**, davon
  - **~44 als UI-Row** (`FIELDS`),
  - **2–4 MODEL-ONLY** (`appLineHeight`, `zenModeShowHeader`, `zenModeShowStatusbar`, `keybinds`),
  - **3 PORT-ONLY-Erfindungen** (`terminalOpacity`, `editorRelativeLineNumbers`, `editorTheme:"auto"`),
  - **1 Key-Bug** (`editorVimMode` statt `vimMode`).
- **Fehlende Preference-Keys im Rust-Modell: ~120** (`autostart`, `startupTerminalCount`, `credentialEncryption`, alle `background*`, `appCornerRadius`, `appTheme`, `themeVariantOverrides`, alle `terminalComposer*`/`terminalBlocks*`, `terminalDefaultPath`, `newTabInheritsCwd`, `confirmCloseTerminalTab`, `terminalFontWeight`, `terminalLetterSpacing`, `terminalLineHeight`, `terminalCursorBlinkInterval`, `terminalRightClickPastes`, `terminalWordSeparator`, `terminalScrollSensitivity`, `terminalFastScrollModifier`, `terminalUseWebGL`, `terminalShowPaneHeader/Footer`, alle `editor*` außer den 11 vorhandenen, alle `sftp*`/`explorer*` außer 3, alle `ssh*`, `hostPingInterval`, alle `commandPalette*` außer 2, alle `bookmarks*`, alle AI-Provider/Agents/Directives/`ai*` außer 3, alle `statusBar*`, alle `sidebar*`/`hm*`, `tabsLocation`, `sidebarTabInfoLine`, `sidebarGroup*`, `barItemPlacements`, `badgesAlwaysVisible`, `titlebarsIconsPosition`).

---

## 5. Konkreter Fix-Plan (datei-für-datei)

### 5.1 `crates/backend/src/modules/settings/preferences.rs` — `Preferences`-Modell erweitern

Alle folgenden Felder ergänzen (camelCase-Serde-Key = Referenz-Key; `#[serde(default)]` ist bereits auf Struct-Ebene gesetzt). Defaults exakt aus `store.ts::DEFAULT_PREFERENCES` übernehmen. Enums als eigene `#[serde(rename_all=…)]`-Typen wie die vorhandenen (`ThemePref`, `CursorStyle`).

**General**: `autostart: bool = false`, `startup_terminal_count: u8 = 1` (1..3), `credential_encryption: bool = false`.
Fixes: `session_restore` default **false**, `default_startup_tab` default **HostManager**, `notify_on_errors` default **false**, `session_scrollback_lines` default **1000**, `scrollback_max_size_mb` default **10**, `scrollback_retention_days` default **0**.

**Appearance & Layout**: `app_theme: String = "default"`, `theme_variant_overrides: BTreeMap<String, ThemeVariant>` (`{light?,dark?}`), `background_image: String = ""`, `background_opacity: u32 = 30`, `background_blur: u32 = 0`, `background_tint_color: String = "#000000"`, `background_tint_opacity: u32 = 0`, `app_corner_radius: u32 = 5`, `hm_card_scale: u32 = 100`, `tabs_location: TabsLocation = Titlebar`, `sidebar_tab_info_line: Vec<SidebarTabInfo> = []` (max 2), `sidebar_group_by_folder: bool = false`, `sidebar_group_single_tabs: bool = false`, `badges_always_visible: bool = true`, `bar_item_placements: BTreeMap<String, BarItemPlacement>` (typisieren: `{ bar: "titlebar"|"statusbar", side: "left"|"right", hidden: bool, itemId: String }` — Default aus einer Rust-Portierung von `DEFAULT_BAR_ITEM_PLACEMENTS`).
Fixes: `app_font_family` default `"\"Inter Variable\", sans-serif"`.

**Terminal**: `terminal_default_path: String = ""`, `new_tab_inherits_cwd: bool = true`, `confirm_close_terminal_tab: bool = false`, `terminal_font_weight: FontWeight = Normal`, `terminal_letter_spacing: f32 = 0.0`, `terminal_line_height: f32 = 1.05`, `terminal_cursor_blink_interval: u32 = 1000`, `terminal_right_click_pastes: bool = false`, `terminal_word_separator: String = " ()[]{}',\"`"`, `terminal_scroll_sensitivity: u32 = 1`, `terminal_fast_scroll_modifier: ScrollModifier = Alt`, `terminal_show_pane_header: bool = false`, `terminal_show_pane_footer: bool = false`, `terminal_use_webgl: bool = true` (oder bewusst weglassen, in `## Notizen` dokumentieren), `terminal_composer_enabled: bool = false`, `terminal_composer_history_popup: bool = false`, `terminal_composer_argument_completion: bool = true`, `terminal_blocks_enabled: bool = false`, `terminal_blocks_auto_collapse_on_alt_screen: bool = true`.
Fixes: `terminal_font_family` default voller CSS-Stack, `terminal_font_size` default **14**, `terminal_cursor_style` default **Bar**, `terminal_scrollback` default **5000** + range 500..50000. `terminal_opacity` (PORT-ONLY) entweder entfernen oder als bewusste GPUI-Ergänzung in `## Notizen` rechtfertigen.

**Editor**: `vim_mode` **umbenennen** von `editorVimMode` → serde-Key `vimMode` (Kompatibilität mit `labonair-settings.json`!). `editor_line_height: f32 = 1.55`, `editor_trim_trailing_whitespace: bool = false`, `editor_insert_final_newline: bool = false`, `editor_autocomplete_debounce_ms: u32 = 350`, `editor_auto_save: EditorAutoSave = Off`, `editor_auto_save_delay: u32 = 1000`, `editor_bracket_matching: bool = true`, `editor_show_cursor_position: bool = true`, `editor_show_selection_stats: bool = true`, `editor_show_outline: bool = false`, `editor_indentation_guides: bool = true`, `editor_max_file_size_mb: u32 = 10`.
Fixes: `editor_tab_size` default **2** (Select 2/4/8), `editor_theme` default **"atomone"**, `"auto"` aus den Options streichen (oder als Port-Erweiterung dokumentieren), `editor_font_family` default voller CSS-Stack. `editor_relative_line_numbers` (PORT-ONLY) in `## Notizen` als Vim-Ergänzung rechtfertigen oder streichen.

**File Manager**: `sftp_show_up_folder: bool = true`, `explorer_show_hidden_by_default: bool = false`, `sftp_column_size: bool = true`, `sftp_column_modified: bool = true`, `sftp_column_permissions: bool = true`, `sftp_column_type: bool = false`, `sftp_remote_edit_show_transfers: bool = true`, `sftp_max_remote_file_size_mb: u32 = 5`, `sftp_default_conflict_resolution: ConflictResolution = Ask`, `sftp_chunk_size_kb: u32 = 64`, `sftp_on_folder_file_error: FolderFileError = Ask`.
Fix: `sftp_max_concurrent_transfers` default **2**, range 1..6.

**Connections** (neu): `host_ping_interval: u32 = 60`, `ssh_connect_timeout_secs: u32 = 10`, `ssh_auto_reconnect: bool = false`, `ssh_auto_reconnect_delay: u32 = 5`, `ssh_auto_reconnect_max_attempts: u32 = 3`, `explorer_remote_poll_interval: u32 = 20`, `explorer_auto_reconnect: bool = false`, `explorer_idle_session_timeout_min: u32 = 5`, `explorer_max_idle_sessions: u32 = 3`, `explorer_max_cached_remote_scopes: u32 = 5`.

**Source Control**: fix `git_status_poll_interval_ms` default **5000**, range 2000..30000.

**Command Palette**: `command_palette_blur: u32 = 4`, `command_palette_opacity: u32 = 95`, `command_palette_position: PalettePosition = Top`, `command_palette_animation: PaletteAnimation = Normal`, `command_palette_history_size: u32 = 5`, `command_palette_close_on_overlay_click: bool = true`.

**Bookmarks** (neu): `bookmarks_enabled: bool = true`, `bookmarks_action_new_terminal: bool = true`, `bookmarks_action_current_terminal: bool = true`, `bookmarks_action_current_sftp: bool = true`, `bookmarks_action_new_sftp: bool = true`, `bookmarks_primary_click_behavior: BookmarkClick = Current`, `bookmarks_show_badge: bool = true`.

**AI**: `default_model_id: String`, `custom_instructions: String = ""`, `ai_temperature: f32 = 0.7`, `ai_shell_max_timeout_secs: u32 = 300`, `ai_shell_max_output_kb: u32 = 256`, `ai_auto_open_mini_on_send: bool = true`, `ai_notify_on_headless_command: bool = true`, `autocomplete_enabled: bool = false`, `autocomplete_provider: String = "cerebras"`, `autocomplete_model_id: String`, `lmstudio_base_url`, `lmstudio_chat_model_id`, `openai_compatible_base_url`, `openai_compatible_model_id`, `mlx_base_url`, `mlx_chat_model_id`, `ollama_base_url`, `ollama_chat_model_id`.
Fixes: `ai_max_agent_steps` default **24**, range 5..50; `ai_terminal_context_lines` default **300**, range 50..1000.
(Provider-Instanzen + API-Keys + Agents + Directives = eigene Backend-Module wie Referenz `providersStore`/`agentsStore`/`directivesStore` + Keychain; nicht in `Preferences`.)

**Status Bar**: `status_bar_show_explorer_button` … `status_bar_show_ai_controls` (7×, default true) — oder direkt über `bar_item_placements` lösen wie die Referenz nach der Migration.

**Sidebar/HM-State**: `sidebar_position`, `sidebar_open`, `sidebar_active_panel`, `sidebar_right_open`, `sidebar_right_active_panel`, `sidebar_width` (clamp 130..450), `sidebar_right_width`, `hm_layout`, `hm_sort` — persistierbar machen (Referenz-Parität; kein UI-Row nötig).

Ranges/Clamps in `load_from` bzw. per Field-Validator nachziehen (Referenz `store.ts::loadPreferences` clampt viele Werte).

### 5.2 `crates/backend/src/modules/settings/mcp.rs`

`McpPrefs` um `notify_on_activity: bool = false` ergänzen (Referenz `mcpNotifyOnActivity`). Prüfen ob `mcp_regenerate_token` in der UI erreichbar ist.

### 5.3 `crates/backend/src/modules/settings/` — neue Module (Referenz-Parität)

- `bar_items.rs` — Portierung von `reference-src/src/modules/settings/lib/barItems.ts` (`BarItemId`, `BarItemPlacement`, `DEFAULT_BAR_ITEM_PLACEMENTS`, `migrateBarItemPlacements`). Der backend `mod.rs::settings_set_bar_item_placement` ist schon da, braucht aber ein typisiertes Gegenstück.
- `providers.rs` / `agents.rs` / `directives.rs` — für AI-Section-Parität (Provider-Instanzen mit Keychain-Keys, Custom Agents, Directives). Referenz: `src/modules/ai/store/*`.
- `themes.rs` / `useThemeStore`-Portierung — Installed/Community-Theme-Liste (`crates/ui/src/settings.rs` hat `scan_themes` rudimentär; für „Themes"-Section braucht es Community-Fetch von `github.com/Snenjih/labonair-themes` + `ThemeMeta`).

### 5.4 `crates/ui/src/settings.rs` — von Modal-Overlay zu Fenster + echte Section-Struktur

1. **Fenster**: `SettingsView` → `SettingsRoot` als eigenständiger Fenster-Root (siehe §1). `AppShell` hält kein `Entity<SettingsView>` mehr, sondern ein optionales Fenster-Handle in einem `Global<SettingsWindow>`. `act_open_settings` ruft `settings_window::open(None, cx)`.
2. **`SettingsTab`-Enum** portieren (`openSettingsWindow.ts`): 15 Varianten + Deep-Link-Aliasse. Menü „Settings…" (`None`) und „Settings → AI…" (`Ai`). Palette-Command optional pro Tab.
3. **Section-Taxonomie** an die Referenz-Sidebar angleichen: `CATEGORIES` ersetzen durch die 10 Referenz-Einträge (`General, Appearance & Layout, Themes, Terminal, Editor, File Manager, Connections, Workspace, Shortcuts, AI`). Icons wie `SIDEBAR_ITEMS` in `SettingsApp.tsx`.
4. **Sub-Section-Struktur pro Pane** — statt einer flachen `FIELDS`-Liste eine geschachtelte Definition: `Section { label, groups: [Group { label, rows: [Row] }] }`. Bedingte Rows unterstützen (`show_if: fn(&Preferences)->bool`) — z. B. `sessionScrollbackLines` nur wenn `sessionRestore`, `terminalCursorBlinkInterval` nur wenn `terminalCursorBlink`, MCP-Detail-Rows nur wenn `mcpBridgeEnabled`, SSH-Reconnect-Detail nur wenn `sshAutoReconnect`.
5. **Echte Controls** (aktuell alles Div-Hacks):
   - `Switch` → `gpui-component`-Switch.
   - `Select` → echtes Dropdown-Menü statt Click-to-Cycle (`cycle_select` ist der Hauptgrund für „fühlt sich falsch an").
   - `Int`/`Number` → Stepper **plus** direkter Zahleneingabe; Float-Variante ergänzen (`appLineHeight`, `editorLineHeight`, `terminalLineHeight`, `terminalLetterSpacing`, `aiTemperature`).
   - `Slider` → neuer Control-Typ (Background opacity/blur/tint, Corner radius, Host card size, Command-Palette blur/opacity).
   - `FontPicker` → Portierung von `src/modules/fonts` (System-Font-Scan gibt es Rust-seitig lt. `SettingsApp.tsx:109` schon: `useSystemFontsStore` ≈ Rust `OnceLock`-Cache).
   - `ToggleGroup` (Sidebar tab info line), `color input` (tint color).
6. **Custom-Panes**:
   - **General**: About-Hero (App-Icon `crates/…/assets`, `env!("CARGO_PKG_VERSION")`, Plattform/Arch, Updater-Button an `crate::updater`, 3 Links).
   - **Appearance**: Color-Scheme-3-Kachel-Picker, Background-Image-Grid (`BackgroundStore` + Import-Dialog + Delete), Slider-Block, `BarItemLayoutSettings`-Port (4 Gruppen, je Item Bar/Side/Hidden).
   - **Themes**: Karten-Grid (Installed/Community-Tabs), Import/New/Open-Folder/Contribute.
   - **Connections**: 4 Sub-Sections inkl. der jetzigen „AI Agent Bridge"-Pane (aus Top-Level zurück nach „Connections" verschieben).
   - **Workspace**: Bookmarks + Command Palette + Source Control zusammenführen.
   - **AI**: Defaults (Model-Dropdown), Providers (`ProviderInstanceCard`-Port), Behaviour, Agents (Karten + Editor-Dialog), Directives (Liste + Editor-Dialog), Custom-Instructions-Textarea.
7. **`FIELDS` → `SETTING_DEFINITIONS`-Äquivalent**: eine zentrale deklarative Registry (key/label/desc/category/control/options) als Single Source für (a) die Section-Rows, (b) die **globale Suche** (nach Kategorie gruppierte Ergebnisliste wie `SearchResults`).
8. **`set_pref`**: bleibt generisch (`PreferencesStore::set_value` per JSON-Key ist gut), aber Side-Effect-Propagation erweitern (Referenz `applySettingChange` in `SettingsApp.tsx:251` hat ~90 Fälle): u. a. `reduceMotion`, `appCornerRadius`, `background*` → Theme/Layout live; `hostPingInterval`/`ssh*`/`explorer*` → jeweilige Worker; `mcp*` → MCP-Bridge.

### 5.5 `crates/ui/src/menu.rs`

- „Settings…" bleibt `cmd-,`, Ziel = neues Fenster.
- Zweiten Eintrag „Settings → AI…" ergänzen (Referenz `open_settings_ai`).
- Prüfen: `apply_keybinds` muss auch im Settings-Fenster greifen (geteiltes `GlobalPreferences`).

### 5.6 `crates/app/src/main.rs`

Kein Zwang, aber: die Settings-Fenster-Öffnung braucht Zugriff auf die geteilten Entities (`PreferencesStore`, `ThemeStore`, `BackgroundStore`, `Backend`, `TokioHandle`). Diese liegen aktuell in `AppShell`; entweder als `Global` hochziehen oder das Fenster von `AppShell` aus öffnen (`window_handle` in `AppShell` cachen).

---

## 6. Priorisierte Fix-Liste

**P0 — Datenmodell & Kompatibilität (blockiert alles andere)**
1. `preferences.rs`: `editorVimMode` → serde-Key `vimMode` zurückbenennen (Kompat-Bug).
2. `preferences.rs`: alle **falschen Defaults** korrigieren (`sessionRestore`, `defaultStartupTab`, `notifyOnErrors`, `terminalFontSize`, `terminalCursorStyle`, `terminalScrollback`, `editorTabSize`, `editorTheme`, `sftpMaxConcurrentTransfers`, `gitStatusPollIntervalMs`, `aiMaxAgentSteps`, `aiTerminalContextLines`, `scrollbackMaxSizeMb`, `scrollbackRetentionDays`, `sessionScrollbackLines`, Font-Family-Stacks).
3. `preferences.rs`: die ~120 fehlenden Keys ergänzen (§5.1) — mindestens als Modell-Felder mit korrekten Defaults, damit `labonair-settings.json` verlustfrei roundtrippt.
4. PORT-ONLY-Felder (`terminalOpacity`, `editorRelativeLineNumbers`, `editorTheme:"auto"`) entscheiden: streichen oder in `## Notizen` als bewusste GPUI-Erweiterung dokumentieren.

**P1 — Separates Fenster**
5. `settings_window.rs` + `cx.open_window`, Größe/Verhalten wie `settings_window_size`/`open_settings_window` (§1). `SettingsTab`-Enum + Deep-Links.
6. Modales `on_key`-Trap entfernen; echte Fokus-Scopes.

**P2 — Section-Struktur & Navigation**
7. Sidebar-Taxonomie an die 10 Referenz-Einträge angleichen (Themes-Eintrag ergänzen, Connections ergänzen, Command Palette/Source Control/Bookmarks unter „Workspace", AI Agent Bridge zurück unter „Connections").
8. Geschachtelte Section→Group→Row-Definition mit Sub-Section-Headern und bedingten Rows.
9. Zentrale deklarative Setting-Registry + globale Suche mit kategorisiertem Ergebnis-Layout.

**P3 — Echte Controls**
10. Select-Dropdown statt Click-to-Cycle; Slider-Control; Float-Number-Control; direkte Zahleneingabe.
11. FontPicker-Port (System/Custom-Fonts).

**P4 — Custom-Panes**
12. General: About-Hero + Updater.
13. Appearance: Color-Scheme-Kacheln, Background-Image-Grid, Slider-Block, `BarItemLayoutSettings`-Port.
14. Themes-Section: Karten-Grid + Community-Tab.
15. Connections-Section: Host/SSH/Explorer-Rows (setzt P0-Modellfelder + Worker-Verdrahtung voraus).
16. Workspace-Section: Bookmarks + Command Palette + Source Control.
17. AI-Section: Defaults/Providers/Behaviour/Agents/Directives (setzt neue Backend-Module voraus, §5.3).

**P5 — Feinschliff**
18. `set_pref` Side-Effect-Propagation für alle neuen Keys (Referenz `applySettingChange`).
19. Visuelle Parität: Header-Strip „Settings", 208 px Sidebar mit Icons + Suchfeld, „Open settings.json"-Button, Spacing/`text-[11.5px]`-Skala, `max-w-[580px]` Content-Spalte.
20. Keyboard-Shortcuts-Section: Gruppen/Filter-Parität gegen `SHORTCUT_GROUPS` verifizieren.
