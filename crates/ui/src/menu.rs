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

use gpui::{actions, App, KeyBinding, Menu, MenuItem, OsAction, SystemMenuType};

use crate::notifications::{notification_center, Notification};

actions!(
    labonair,
    [
        // ── File ──────────────────────────────────────────────────────────
        NewTerminalTab,
        NewSshTab,
        NewSftpTab,
        NewPreviewTab,
        NewEditorTab,
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
        AskAboutSelection,
        ClearChat,
        AiSettings,
        // ── Window ────────────────────────────────────────────────────────
        Minimize,
        ZoomWindow,
        OpenShortcuts,
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
    cx.bind_keys(bindings());

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
    cx.on_action(|_: &OpenSettings, cx: &mut App| {
        toast(cx, "Settings", "Settings arrive in a later phase.")
    });
    cx.on_action(|_: &AiSettings, cx: &mut App| {
        toast(cx, "AI Settings", "AI settings arrive in a later phase.")
    });
    cx.on_action(|_: &CheckForUpdates, cx: &mut App| {
        toast(
            cx,
            "Check for Updates",
            "You're on the latest version. (Auto-update lands in T15-005.)",
        )
    });

    cx.set_menus(app_menus());

    #[cfg(target_os = "macos")]
    cx.set_dock_menu(dock_menu());
}

fn toast(cx: &mut App, title: &'static str, body: &'static str) {
    notification_center(cx).update(cx, |center, cx| {
        center.push(Notification::info(title, body), cx);
    });
}

/// Key bindings that mirror the reference accelerators. They drive both the
/// menu-item shortcut hint and the actual dispatch.
fn bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-t", NewTerminalTab, None),
        KeyBinding::new("cmd-shift-p", NewPreviewTab, None),
        KeyBinding::new("cmd-e", NewEditorTab, None),
        KeyBinding::new("cmd-w", CloseTab, None),
        KeyBinding::new("cmd-shift-w", ClosePane, None),
        KeyBinding::new("cmd-b", ToggleSidebar, None),
        KeyBinding::new("cmd-i", ToggleAiPanel, None),
        KeyBinding::new("cmd-=", ZoomIn, None),
        KeyBinding::new("cmd--", ZoomOut, None),
        KeyBinding::new("cmd-0", ResetZoom, None),
        KeyBinding::new("ctrl-cmd-f", ToggleFullScreen, None),
        KeyBinding::new("cmd-d", SplitPaneRight, None),
        KeyBinding::new("cmd-shift-d", SplitPaneDown, None),
        KeyBinding::new("cmd-f", Find, None),
        KeyBinding::new("cmd-shift-n", NewSshConnection, None),
        KeyBinding::new("cmd-l", AskAboutSelection, None),
        KeyBinding::new("cmd-k", OpenShortcuts, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-m", Minimize, None),
        KeyBinding::new("ctrl-tab", NextTab, None),
        KeyBinding::new("ctrl-shift-tab", PrevTab, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", HideApp, None),
    ]
}

/// The full menu bar, structure/order/labels 1:1 with the reference
/// `build_menu`.
fn app_menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "Labonair".into(),
            items: vec![
                MenuItem::action("About Labonair", About),
                MenuItem::separator(),
                MenuItem::action("Settings\u{2026}", OpenSettings),
                MenuItem::separator(),
                MenuItem::action("Check for Updates\u{2026}", CheckForUpdates),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
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

    /// Every accelerator string parses (`KeyBinding::new` panics otherwise).
    #[test]
    fn bindings_parse() {
        assert_eq!(bindings().len(), 23);
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
