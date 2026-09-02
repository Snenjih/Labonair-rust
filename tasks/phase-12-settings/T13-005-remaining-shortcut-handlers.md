# T13-005: Restliche Shortcut-Handler (Tab-Index, Pane-Fokus, Zen-Mode)

## Status
⏳ Pending

## Phase
12 — Settings & Preferences

## Abhängigkeiten
T13-004 (Shortcut-Konfiguration), T04-002 (Split-Panes)

## Ziel
Die in T13-004 bewusst zurückgestellten Nicht-Menü-Shortcuts mit
Laufzeit-Dispatch versehen, sodass sie 1:1 der Referenz
(`reference-src/src/modules/shortcuts/lib/useShortcutHandlers.ts`) entsprechen.

## Kontext
Aufgedeckt / bestätigt beim T15-006-Audit. Betroffen (alle bereits als
rebindbare `ShortcutId` in `command_palette.rs` gelistet, `command_for_shortcut`
liefert aber `None`):

- `tab.selectTab1` … `tab.selectTab9` — Sprung zu Tab N (`Cmd+1..9`).
  Referenz: `useTabsStore.selectByIndex(n)`.
- `pane.focusNext` (`Cmd+]`) — Fokus zum nächsten Split-Leaf zyklisch
  (`collectLeafIds` → nächster Index, no-op bei Einzel-Pane).
- `view.zenMode` (`Cmd+Shift+Z`) — toggelt `zenModeShowHeader` +
  `zenModeShowStatusbar` (beide sichtbar → beide aus, sonst beide an).
  **Zwei neue Preferences** nötig (`zen_mode_show_header`,
  `zen_mode_show_statusbar`, Default `true`); `AppShell` blendet Header/Statusbar
  entsprechend aus. Command-Palette-Einträge dafür (siehe
  `useSettingsCommands.ts`: "Zen: Toggle Header/Statusbar/All").

## Anweisungen
1. `preferences.rs`: `zen_mode_show_header` / `zen_mode_show_statusbar` (bool,
   Default true) + Test.
2. `menu.rs`: `actions!` + `rebind!` für die 11 Shortcuts (keine Menü-Einträge —
   die Referenz hat auch keine).
3. `app_shell.rs`: `on_action`-Handler; `render` respektiert die Zen-Prefs;
   `workspace.rs`: `select_tab_by_index`, `focus_next_pane`.
4. `command_palette.rs`: `CommandId`-Varianten, `command_for_shortcut`-Mapping
   (die `None`-Testfälle anpassen), Dispatch in `app_shell.rs`.

## Akzeptanzkriterien
- [ ] `Cmd+1..9` aktiviert Tab N (bzw. no-op wenn nicht vorhanden)
- [ ] `Cmd+]` zykelt den Pane-Fokus in einem Split-Tab
- [ ] `Cmd+Shift+Z` toggelt Header + Statusbar; Zustand persistiert
- [ ] Command-Palette listet die Zen-Toggles
- [ ] `cargo check` + `clippy -D warnings` + `cargo test` grün

## Notizen
- Aus T15-006 ausgegliedert; mechanisch, kein GPUI-Blocker.
