# T17-007: `CommandRegistry`

## Status
✅ Done

## Phase
16 — Root-Objekt & Registries

## Abhängigkeiten
T16-004 (`labonair-command-palette`), T17-006 (`AppShell` → reine Komposition)

## Ziel
Die große `.on_action(cx.listener(Self::act_*))`-Kette in `AppShell` (~50
Einträge) auflösen. Kommandos werden zentral in einer `CommandRegistry`
registriert; **Command-Palette und Keymap teilen sich dieselbe Registry**. Ein
neues Kommando = ein `register(...)`-Aufruf, nicht ein Enum-Arm + Handler +
`.on_action`-Zeile + Palette-Eintrag an vier Stellen.

## Kontext
- Heute: `app_shell.rs` — `enum menu::Action`-artige Actions
  (`SelectTabN`-Makro, `act_new_terminal_tab`, `act_split_right`,
  `act_toggle_ai_panel`, `act_open_settings`, `act_command_palette`, …).
  `build_palette_data(cx)` baut die Palette-Einträge (`CommandId` →
  `PaletteData`) getrennt von den Action-Handlern. `menu.rs` +
  `apply_keybinds` mappen Keybinds auf dieselben Actions.
- `labonair-command-palette` (T16-004): `CommandId`, `PaletteData`,
  `PaletteChoice`, `PaletteEvent`, `KeybindMap`, `ShortcutId`,
  `effective_binding`.
- Zed-Vorbild: `zed-refrence/zed/crates/command_palette/` +
  `zed-refrence/zed/crates/command_palette_hooks/` — Kommandos sind GPUI-
  Actions; die Palette listet *alle* registrierten Actions im aktuellen
  Kontext; `zed-refrence/zed/crates/settings/src/keymap_file.rs` bindet
  Chords an Action-Namen.

## Anweisungen zur Umsetzung
1. **`CommandRegistry`** in `labonair-command-palette` (oder neuem
   `labonair-commands`):
   - `struct Command { id: CommandId, title: SharedString, category:
     SharedString, default_key: Option<&'static str>, run: CommandFn }`
     wobei `CommandFn = Rc<dyn Fn(&mut Workspace, &mut Window, &mut Context<Workspace>)>`
     (oder ein Enum-Dispatch, falls `Workspace` nicht ohne Zyklus importierbar
     ist — dann `run: fn()` + Action-Emission).
   - `register(cmd)`, `iter()`, `by_id(id)`, `visible_in(context)` (Kontext-
     Filter, z.B. „nur wenn aktiver Tab = Terminal").
   - Als Feld im `Workspace` oder GPUI-Global (konsistent zu den anderen
     Registries).
2. **Kommando-Definitionen**: eine `register_builtin_commands(registry)` in
   `labonair-shell` — alle heutigen `act_*` als `Command`-Einträge mit
   `title` + `category` + `default_key`. Kategorien z.B. „Tabs", „Panels",
   „Terminal", „View", „AI", „Window", „Settings".
3. **Palette speist sich aus der Registry**: `build_palette_data` entfällt —
   die Palette liest `registry.visible_in(current_context)` und rendert
   Titel + Keybind (`effective_binding`). Auswahl → `command.run(...)`.
4. **Keymap bindet an `CommandId`**: `apply_keybinds` / `menu.rs` mappen
   Keystrokes auf `CommandId` statt auf GPUI-Actions direkt; der Dispatch
   läuft über `registry.by_id(id).run(...)`. (Die Datei-basierte `keymap.json`
   kommt in T19-008 — hier bleibt die `ShortcutId`/`KeybindMap`-Quelle, aber
   der Ziel-Dispatch ist die Registry.)
5. **`AppShell`**: die `.on_action`-Kette schrumpft auf die paar echten
   GPUI-Fenster-Actions; alles andere ist Command. `select_tab_action!`-Makro
   → sechs, äh neun `Command`-Einträge mit Index-Closure.
6. **Native Menüs** (`menu.rs`): Menüpunkte lösen `CommandId` aus (ein
   `dispatch_command(id)`), statt je einen eigenen Action-Typ zu haben — wo
   sinnvoll; macOS-`Menu`-Items brauchen ggf. weiter echte Actions für die
   Accelerator-Anzeige, dann Action → Command-Bridge.
7. `cargo run`: Command-Palette listet alle Kommandos mit korrektem Keybind;
   jedes ausführbar; Keybinds wirken; native Menüpunkte funktionieren;
   Kontext-Filter (Split nur bei Terminal) greift.

## Akzeptanzkriterien
- [x] `CommandRegistry` existiert; `register_builtin_commands` ist die einzige
      Stelle, an der Kommandos definiert werden.
      `> Deviation:` der Registry-**Typ** liegt in `labonair-shell`
      (`crates/shell/src/commands.rs`), nicht in `labonair-command-palette` /
      einem neuen `labonair-commands` — `CommandFn` braucht `&mut AppShell`
      (Panel-Entities aus §8.4/§8.9), das ist nur in `labonair-shell` benennbar.
      Palette + Keymap teilen die Registry über die gemeinsame `CommandId`.
      Festgehalten in `docs/architecture.md` §8.10.
- [x] `build_palette_data` rendert aus der Registry inkl.
      `effective_binding`-Keybind-Anzeige (`effective_keys`, override-aware).
      `> Deviation:` `build_palette_data` ist **verschlankt**, nicht entfernt.
      Pref/Theme-Skalare (`color_mode`, `editor_theme`, `font_size`, 9 Toggles)
      + Keybind-Overrides → `PalettePrefs`-Reads. Nur die panel-/workspace-/
      settings-gespeisten Auswahllisten (`snippet_choices`, `session_choices`,
      `branch_choices`, Hosts, Symbole, App-Themes) laufen weiter über
      `set_data` (sonst Crate-Zyklus). `PaletteData`: 12 → 7 Felder.
      Festgehalten in §8.10.
- [x] Keymap-Dispatch und Menü-Dispatch laufen über `CommandId` → Registry
      (`attach_action_handlers` = Action→`CommandId`-Bridge;
      `handle_palette_event`'s `Run(id)` → `dispatch_command`).
- [x] `app_shell.rs` hat nur noch **3** echte Fenster-`.on_action`s
      (`ToggleFullScreen`, `Minimize`, `ZoomWindow`).
- [x] Kontext-Filter: `Split Right/Down` erscheint/aktiv nur bei Terminal-Tab;
      `Close Pane` nur bei vorhandenem Split (unverändert `if can_split` /
      `if has_split` in `attach_action_handlers`; Registry `contexts` +
      `visible_in` spiegeln es).
- [x] Ein neues Kommando = 1 `register`-Aufruf: `CommandId::DuplicateTab`
      (kein Menü, kein Keybind) ist mit genau einer `r.register(...)`-Zeile
      verdrahtet und erscheint über den Palette-`COMMANDS`-Eintrag; ein Keybind
      wäre 1 `menu::`-Action + 1 Bridge-Zeile.
- [x] `cargo run`: bisher unverändert — jede Menü-/Keybind-Action bridged auf
      dieselbe `CommandId`; `ZoomIn`/`OpenShortcuts`/Navigator-IDs bleiben
      No-ops wie zuvor (`> Deviation:` §8.10, Punkt 3).
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `scripts/check-crate-deps.sh` (87 Edges, azyklisch, kein neuer Edge).
      `cargo test --workspace` kann auf diesem headless-VPS nicht linken
      (fehlende X11-Dev-Libs); `check`/`clippy --all-targets` haben den
      gesamten `#[cfg(test)]`-Code (inkl. der neuen `commands.rs`-Tests)
      kompiliert — projektanerkannter Ersatz.

## Notizen
- Wenn `CommandFn` `&mut Workspace` nicht sauber typisieren kann (Zyklus
  `command-palette` → `workspace`), dann `run` eine GPUI-Action emittieren
  lassen, die `Workspace` per `.on_action` einmalig zentral behandelt — immer
  noch eine Registry, nur mit Action-Zwischenschritt. In `docs/architecture.md`
  festhalten, welcher Weg gewählt wurde.
- `drain_pending_ai` (Rest aus T17-006) hier final beseitigen: der
  „Run in terminal"-Button des AI-Panels löst ein `Command` aus.

## Warnungen
- ⚠️ GPUI-Actions, die an macOS-Menü-Accelerators hängen, brauchen stabile
  Typen für die Tastenkürzel-Anzeige — nicht alle blind durch Closures
  ersetzen; die Action→Command-Bridge ist der sichere Weg.
- ⚠️ Kontext-Filter dürfen nicht pro Frame den ganzen Workspace abfragen —
  den `current_context` bei Tab-/Pane-Wechsel cachen.

## Weiterführende Tasks
- [T17-008: `AppEvent`-Bus entscheiden](./T17-008-appevent-bus-decision.md)
- [T19-008: Keymap als Datei mit Kontexten](../phase-18-settings-core/T19-008-keymap-file-with-contexts.md)
