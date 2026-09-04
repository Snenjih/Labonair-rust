//! Native macOS menu bar + Dock context menu (T04-005).
//!
//! Ported from `reference-src/src-tauri/src/lib.rs` (`build_menu`),
//! `reference-src/src-tauri/src/modules/dock_menu.rs` and
//! `reference-src/src-tauri/src/modules/menu_sync.rs`.
//!
//! Every menu entry is a GPUI [`Action`](gpui::Action). The actual behaviour
//! lives in one place ([`crate::app_shell::AppShell`] for the window/workspace
//! entries, a few app-global handlers here for App-menu items) so a menu click
//! and its keyboard shortcut always run the identical code path — the same
//! "one source of truth" the reference achieved via `nativeMenuSync.ts`.
//!
//! Enable/disable follows the app state automatically: macOS calls GPUI's
//! `validate_menu_item` on every menu open, which resolves to
//! `App::is_action_available` against the live focus dispatch tree. `AppShell`
//! only registers the `Split Pane …` / `Close Pane` handlers when the active
//! tab actually has a (splittable / split) workspace, so those items grey out
//! on their own — no explicit `set_menus` re-sync needed.
//!
//! Items for features that don't exist yet (SSH, SFTP, AI, editor, host
//! manager, zoom) have no handler and therefore render disabled, per the task
//! note; their phase wires a handler in and they light up.

use gpui::{
    actions, Action, App, DummyKeyboardMapper, KeyBinding, KeyBindingContextPredicate, Keystroke,
    Menu, MenuItem, OsAction,
};
use std::rc::Rc;

use labonair_notifications::{notification_center, Notification};
use labonair_settings::keymap::EffectiveBinding;

/// `AskAboutSelection` is defined in `labonair-workspace` (so `views::terminal`
/// can dispatch it without a dependency cycle, T16-006); re-exported here so the
/// menu / keybind wiring below and `crate::menu::AskAboutSelection` still work.
pub use labonair_workspace::AskAboutSelection;

actions!(
    labonair,
    [
        // ── File ──────────────────────────────────────────────────────────
        NewTerminalTab,
        NewSshTab,
        NewSftpTab,
        NewPreviewTab,
        NewEditorTab,
        Save,
        CloseTab,
        ClosePane,
        // ── Edit (OS-backed) ──────────────────────────────────────────────
        Undo,
        Redo,
        Cut,
        Copy,
        Paste,
        SelectAll,
        // ── View ──────────────────────────────────────────────────────────
        ToggleSidebar,
        ToggleAiPanel,
        ZoomIn,
        ZoomOut,
        ResetZoom,
        ToggleFullScreen,
        ToggleZenMode,
        // Temporary T17-002 dock debug shortcuts (no menu entry).
        DebugCyclePanelDock,
        DebugToggleDockZoom,
        // ── Tab index jumps / pane focus (no menu entry — parity with the
        //    reference, which only exposes these as shortcuts, T13-005) ──────
        SelectTab1,
        SelectTab2,
        SelectTab3,
        SelectTab4,
        SelectTab5,
        SelectTab6,
        SelectTab7,
        SelectTab8,
        SelectTab9,
        FocusNextPane,
        // ── Terminal ──────────────────────────────────────────────────────
        SplitPaneRight,
        SplitPaneDown,
        Find,
        // ── Connections ───────────────────────────────────────────────────
        OpenHostManager,
        NewSshConnection,
        NewQuickSsh,
        // ── AI ────────────────────────────────────────────────────────────
        NewAiSession,
        ClearChat,
        AiSettings,
        // ── Window ────────────────────────────────────────────────────────
        Minimize,
        ZoomWindow,
        OpenShortcuts,
        OpenKeymapJson,
        OpenPathBookmarks,
        CommandPalette,
        NextTab,
        PrevTab,
        // ── App ───────────────────────────────────────────────────────────
        About,
        OpenSettings,
        CheckForUpdates,
        HideApp,
        HideOthers,
        ShowAll,
        Quit,
    ]
);

/// Register app-global menu handlers, key bindings, the menu bar and the Dock
/// menu. Call once, after the main window is open.
pub fn init(cx: &mut App) {
    // Key bindings are applied by `AppShell` via [`apply_keybinds`] so they
    // reflect the user's persisted shortcut overrides (T13-004). `AppShell`
    // is constructed before this runs, so we must not re-bind defaults here.

    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
    cx.on_action(|_: &HideApp, cx: &mut App| cx.hide());
    cx.on_action(|_: &HideOthers, cx: &mut App| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx: &mut App| cx.unhide_other_apps());
    cx.on_action(|_: &About, cx: &mut App| {
        toast(
            cx,
            "About Labonair",
            concat!(
                "Version ",
                env!("CARGO_PKG_VERSION"),
                " \u{2014} a native terminal, SSH & SFTP client with integrated AI. labonair.app"
            ),
        )
    });
    // `OpenSettings` / `AiSettings` are handled by `AppShell` (open the
    // dedicated settings OS window, deep-linking to the AI tab for `AiSettings`
    // — T16-009).
    // `CheckForUpdates` is handled by `AppShell` (drives the auto-updater,
    // T15-005) so the menu item, the command-palette entry and any shortcut
    // share one code path.

    cx.set_menus(app_menus());

    #[cfg(target_os = "macos")]
    cx.set_dock_menu(dock_menu());
}

fn toast(cx: &mut App, title: &'static str, body: &'static str) {
    notification_center(cx).update(cx, |center, cx| {
        center.push(Notification::info(title, body), cx);
    });
}

/// Rebind all key bindings from the merged `keymap.json` effective bindings
/// (T19-008 — replaces the old `KeybindMap`-based `apply_keybinds`). Called by
/// `crate::keymap_loader` at startup and on every live-reload, so an edited
/// `keymap.json` takes effect with no restart. GPUI derives the native
/// menu-item accelerator hints from the same keymap, so the menu stays in
/// sync automatically.
pub fn apply_keymap(cx: &mut App, effective: &[EffectiveBinding]) {
    cx.clear_key_bindings();
    cx.bind_keys(fixed_bindings());
    cx.bind_keys(bindings_from_keymap(effective));
}

/// Fixed / OS-reserved accelerators — never customizable, so they never go
/// through `keymap.json` at all (mirrors the pre-T19-008 hardcoded list).
fn fixed_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-s", Save, None),
        KeyBinding::new("cmd-shift-n", NewSshConnection, None),
        KeyBinding::new("ctrl-cmd-f", ToggleFullScreen, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-m", Minimize, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", HideApp, None),
    ]
}

/// Resolve a `keymap.json` action name (`CommandId::action_name()`) to the
/// concrete `menu::` GPUI `Action` it dispatches, for the subset of commands
/// that have a key-bindable menu action today. `None` for an action name with
/// no concrete `Action` type (e.g. a palette-only sub-page navigator) — the
/// caller skips + logs those instead of failing the whole keymap.
fn action_for(name: &str) -> Option<Box<dyn Action>> {
    Some(match name {
        "command_palette::Toggle" => Box::new(CommandPalette),
        "settings::OpenShortcuts" => Box::new(OpenShortcuts),
        "zed::OpenKeymap" => Box::new(OpenKeymapJson),
        "tab::NewTerminal" => Box::new(NewTerminalTab),
        "tab::NewPreview" => Box::new(NewPreviewTab),
        "tab::NewEditor" => Box::new(NewEditorTab),
        "tab::NewSsh" => Box::new(NewSshTab),
        "tab::NewSftp" => Box::new(NewSftpTab),
        "tab::Close" => Box::new(CloseTab),
        "tab::Next" => Box::new(NextTab),
        "tab::Prev" => Box::new(PrevTab),
        "tab::Select1" => Box::new(SelectTab1),
        "tab::Select2" => Box::new(SelectTab2),
        "tab::Select3" => Box::new(SelectTab3),
        "tab::Select4" => Box::new(SelectTab4),
        "tab::Select5" => Box::new(SelectTab5),
        "tab::Select6" => Box::new(SelectTab6),
        "tab::Select7" => Box::new(SelectTab7),
        "tab::Select8" => Box::new(SelectTab8),
        "tab::Select9" => Box::new(SelectTab9),
        "pane::SplitRight" => Box::new(SplitPaneRight),
        "pane::SplitDown" => Box::new(SplitPaneDown),
        "pane::Close" => Box::new(ClosePane),
        "pane::FocusNext" => Box::new(FocusNextPane),
        "search::Toggle" => Box::new(Find),
        "sidebar::Toggle" => Box::new(ToggleSidebar),
        "view::ToggleZenMode" => Box::new(ToggleZenMode),
        "view::ZoomIn" => Box::new(ZoomIn),
        "view::ZoomOut" => Box::new(ZoomOut),
        "view::ZoomReset" => Box::new(ResetZoom),
        "view::ToggleFullScreen" => Box::new(ToggleFullScreen),
        "ai::TogglePanel" => Box::new(ToggleAiPanel),
        "ai::AskSelection" => Box::new(AskAboutSelection),
        "ai::NewSession" => Box::new(NewAiSession),
        "ai::ClearChat" => Box::new(ClearChat),
        "bookmarks::Open" => Box::new(OpenPathBookmarks),
        "connections::OpenHostManager" => Box::new(OpenHostManager),
        "connections::NewSshConnection" => Box::new(NewSshConnection),
        "connections::NewQuickSsh" => Box::new(NewQuickSsh),
        "settings::Open" => Box::new(OpenSettings),
        "settings::OpenAi" => Box::new(AiSettings),
        "app::CheckForUpdates" => Box::new(CheckForUpdates),
        "debug::CyclePanelDock" => Box::new(DebugCyclePanelDock),
        "debug::ToggleDockZoom" => Box::new(DebugToggleDockZoom),
        _ => return None,
    })
}

/// Turn the merged effective keymap into real `gpui::KeyBinding`s. Every
/// keystroke token and context predicate is pre-validated with the same
/// fallible parsers GPUI itself uses internally, so the panicking
/// `KeyBinding::new` is only ever called on data already proven to parse —
/// an entry that fails validation (bad keystroke, bad context, unknown
/// action) is skipped with a `tracing::warn!` instead of crashing the app.
fn bindings_from_keymap(effective: &[EffectiveBinding]) -> Vec<KeyBinding> {
    let mut out = Vec::with_capacity(effective.len());
    for binding in effective {
        let Some(action) = action_for(&binding.action) else {
            continue; // no concrete Action for this command — not key-bindable yet.
        };
        if binding
            .keystrokes
            .split_whitespace()
            .any(|token| Keystroke::parse(token).is_err())
        {
            tracing::warn!(
                keystrokes = %binding.keystrokes,
                "keymap.json: unparseable keystroke, skipping binding"
            );
            continue;
        }
        let context_predicate = match &binding.context {
            None => None,
            Some(ctx) => match KeyBindingContextPredicate::parse(ctx) {
                Ok(pred) => Some(Rc::new(pred)),
                Err(_) => {
                    tracing::warn!(context = %ctx, "keymap.json: unparseable context, skipping binding");
                    continue;
                }
            },
        };
        match KeyBinding::load(
            binding.keystrokes.as_str(),
            action,
            context_predicate,
            false,
            None,
            &DummyKeyboardMapper,
        ) {
            Ok(kb) => out.push(kb),
            Err(e) => tracing::warn!(
                keystrokes = %binding.keystrokes,
                error = ?e,
                "keymap.json: failed to load binding, skipping"
            ),
        }
    }
    out
}

/// The full menu bar, structure/order/labels 1:1 with the reference
/// `build_menu`.
fn app_menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "Labonair".into(),
            items: vec![
                MenuItem::action("About Labonair", About),
                MenuItem::action("Settings\u{2026}", OpenSettings),
                // Not in the reference menu — a deliberate T15-005 addition
                // (the app ships its own macOS auto-updater).
                MenuItem::action("Check for Updates\u{2026}", CheckForUpdates),
                MenuItem::separator(),
                MenuItem::action("Hide Labonair", HideApp),
                MenuItem::action("Hide Others", HideOthers),
                MenuItem::action("Show All", ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quit Labonair", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Terminal Tab", NewTerminalTab),
                MenuItem::action("New SSH Tab", NewSshTab),
                MenuItem::action("New SFTP Tab", NewSftpTab),
                MenuItem::action("New Preview Tab", NewPreviewTab),
                MenuItem::action("New Editor Tab", NewEditorTab),
                MenuItem::separator(),
                MenuItem::action("Close Tab", CloseTab),
                MenuItem::action("Close Pane", ClosePane),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, OsAction::Undo),
                MenuItem::os_action("Redo", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, OsAction::Cut),
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
                MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Sidebar", ToggleSidebar),
                MenuItem::action("Toggle AI Panel", ToggleAiPanel),
                MenuItem::separator(),
                MenuItem::action("Zoom In", ZoomIn),
                MenuItem::action("Zoom Out", ZoomOut),
                MenuItem::action("Reset Zoom", ResetZoom),
                MenuItem::separator(),
                MenuItem::action("Toggle Full Screen", ToggleFullScreen),
            ],
        },
        Menu {
            name: "Terminal".into(),
            items: vec![
                MenuItem::action("Split Pane Right", SplitPaneRight),
                MenuItem::action("Split Pane Down", SplitPaneDown),
                MenuItem::separator(),
                MenuItem::action("Find\u{2026}", Find),
            ],
        },
        Menu {
            name: "Connections".into(),
            items: vec![
                MenuItem::action("Open Host Manager", OpenHostManager),
                MenuItem::separator(),
                MenuItem::action("New SSH Connection\u{2026}", NewSshConnection),
                MenuItem::action("New Quick SSH\u{2026}", NewQuickSsh),
            ],
        },
        Menu {
            name: "AI".into(),
            items: vec![
                MenuItem::action("Toggle AI Panel", ToggleAiPanel),
                MenuItem::action("New AI Session", NewAiSession),
                MenuItem::action("Ask about Selection", AskAboutSelection),
                MenuItem::separator(),
                MenuItem::action("Clear Current Chat", ClearChat),
                MenuItem::separator(),
                MenuItem::action("AI Settings\u{2026}", AiSettings),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Minimize", Minimize),
                MenuItem::action("Zoom", ZoomWindow),
                MenuItem::separator(),
                MenuItem::action("Keyboard Shortcuts", OpenShortcuts),
                MenuItem::action("Settings", OpenSettings),
                MenuItem::separator(),
                MenuItem::action("Next Tab", NextTab),
                MenuItem::action("Previous Tab", PrevTab),
            ],
        },
    ]
}

/// Dock right-click menu — entries 1:1 with `dock_menu.rs`.
#[cfg(target_os = "macos")]
fn dock_menu() -> Vec<MenuItem> {
    vec![
        MenuItem::action("New Terminal Tab", NewTerminalTab),
        MenuItem::action("New SSH Connection\u{2026}", NewSshConnection),
        MenuItem::separator(),
        MenuItem::action("Open Host Manager", OpenHostManager),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every default-keymap binding either resolves to a concrete `Action`
    /// and loads, or is a deliberate no-op (no menu action yet) — either way
    /// `bindings_from_keymap` must never panic.
    #[test]
    fn default_keymap_bindings_load() {
        let default = labonair_settings::keymap::parse_keymap_jsonc(
            labonair_settings::keymap::default_asset(),
        )
        .unwrap();
        let effective = labonair_settings::keymap::merge_keymaps(&[(
            labonair_settings::keymap::KeybindSource::Default,
            &default,
        )]);
        let loaded = bindings_from_keymap(&effective);
        // Every action referenced in the shipped default asset has a concrete
        // `menu::` Action in `action_for` (this test would fail loudly via
        // count mismatch if a new default binding's action were forgotten
        // there).
        assert_eq!(loaded.len(), effective.len());
    }

    #[test]
    fn unresolvable_action_is_skipped_not_panicking() {
        let file = labonair_settings::keymap::parse_keymap_jsonc(
            r#"[{ "bindings": { "cmd-shift-y": "bogus::DoesNotExist" } }]"#,
        )
        .unwrap();
        let effective = labonair_settings::keymap::merge_keymaps(&[(
            labonair_settings::keymap::KeybindSource::User,
            &file,
        )]);
        assert!(bindings_from_keymap(&effective).is_empty());
    }

    #[test]
    fn menu_bar_matches_reference_structure() {
        let menus = app_menus();
        let names: Vec<_> = menus.iter().map(|m| m.name.as_ref()).collect();
        assert_eq!(
            names,
            [
                "Labonair",
                "File",
                "Edit",
                "View",
                "Terminal",
                "Connections",
                "AI",
                "Window",
            ]
        );
        // GPUI wires the live window list into a submenu named exactly "Window".
        assert!(menus.iter().any(|m| m.name.as_ref() == "Window"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn dock_menu_has_reference_entries() {
        assert_eq!(dock_menu().len(), 4);
    }
}
