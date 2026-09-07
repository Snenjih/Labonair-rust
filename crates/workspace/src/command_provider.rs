//! Workspace-owned command metadata.
//!
//! The workspace capability contributes its discoverable commands and their
//! executable handlers without importing the palette UI or application shell.

use gpui::Entity;
use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu,
    SubmenuAction, SubmenuDescriptor, SubmenuItem, SubmenuSnapshot,
};
use labonair_command_palette_runtime::CommandHandlerRegistry;
use labonair_keymap::ShortcutId;

use crate::context::WorkspaceTransition;
use crate::Workspace;

/// Stable command metadata contributed by the workspace capability.
#[derive(Clone, Copy, Debug, Default)]
pub struct WorkspaceCommandProvider;

/// Register executable handlers for commands owned by Workspace.
///
/// The callbacks capture only the Workspace entity and therefore remain
/// usable by any composition root. The shell supplies the active window and
/// application context when dispatching; it does not define these behaviors.
pub fn register_handlers(registry: &mut CommandHandlerRegistry, workspace: &Entity<Workspace>) {
    macro_rules! register {
        ($id:expr, $handler:expr) => {
            registry
                .register($id, $handler)
                .expect("workspace command handler must have a unique id");
        };
    }

    let workspace_handle = workspace.clone();
    register!(CommandId::NewTerminalTab, move |window, cx| {
        workspace_handle.update(cx, |workspace, cx| workspace.new_terminal_tab(window, cx));
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::NewEditorTab, move |window, cx| {
        workspace_handle.update(cx, |workspace, cx| workspace.new_editor_tab(window, cx));
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::NewPreviewTab, move |window, cx| {
        workspace_handle.update(cx, |workspace, cx| workspace.new_preview_tab(window, cx));
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::Save, move |_window, cx| {
        workspace_handle.update(cx, |workspace, cx| workspace.save_active(cx));
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::CloseTab, move |window, cx| {
        workspace_handle.update(cx, |workspace, cx| workspace.close_active(window, cx));
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::DuplicateTab, move |window, cx| {
        workspace_handle.update(cx, |workspace, cx| {
            workspace.duplicate_active_tab(window, cx)
        });
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::CloseOtherTabs, move |window, cx| {
        workspace_handle.update(cx, |workspace, cx| workspace.close_other_tabs(window, cx));
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::NextTab, move |window, cx| {
        workspace_handle.update(cx, |workspace, cx| workspace.cycle(true, window, cx));
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::PrevTab, move |window, cx| {
        workspace_handle.update(cx, |workspace, cx| workspace.cycle(false, window, cx));
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::FocusNextPane, move |window, cx| {
        workspace_handle.update(cx, |workspace, cx| workspace.focus_next_pane(window, cx));
    });

    for (id, index) in [
        (CommandId::SelectTab1, 0),
        (CommandId::SelectTab2, 1),
        (CommandId::SelectTab3, 2),
        (CommandId::SelectTab4, 3),
        (CommandId::SelectTab5, 4),
        (CommandId::SelectTab6, 5),
        (CommandId::SelectTab7, 6),
        (CommandId::SelectTab8, 7),
        (CommandId::SelectTab9, 8),
    ] {
        let workspace_handle = workspace.clone();
        register!(id, move |window, cx| {
            workspace_handle.update(cx, |workspace, cx| {
                workspace.select_tab_by_index(index, window, cx)
            });
        });
    }

    let workspace_handle = workspace.clone();
    register!(CommandId::SplitRight, move |window, cx| {
        workspace_handle.update(cx, |workspace, cx| {
            workspace.split(crate::pane::SplitDirection::Right, window, cx)
        });
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::SplitDown, move |window, cx| {
        workspace_handle.update(cx, |workspace, cx| {
            workspace.split(crate::pane::SplitDirection::Down, window, cx)
        });
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::ClosePane, move |window, cx| {
        workspace_handle.update(cx, |workspace, cx| workspace.close_pane(window, cx));
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::OpenProject, move |_window, cx| {
        workspace_handle.update(cx, |workspace, cx| workspace.request_open_project(cx));
    });

    let workspace_handle = workspace.clone();
    register!(CommandId::ReturnToStandalone, move |_window, cx| {
        workspace_handle.update(cx, |workspace, cx| {
            workspace.apply_transition(WorkspaceTransition::ReturnToStandalone, cx);
        });
    });
}

pub fn zoom_submenu() -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new("zoom", "Font Size", CommandSubmenu::Zoom),
        items: [
            (CommandId::ZoomIn, "Increase Font Size"),
            (CommandId::ZoomOut, "Decrease Font Size"),
            (CommandId::ZoomReset, "Reset Font Size"),
        ]
        .into_iter()
        .map(|(id, title)| SubmenuItem {
            id: id.action_name().to_string(),
            title: title.to_string(),
            subtitle: None,
            active: false,
            action: SubmenuAction::RunCommand(id),
            secondary: None,
        })
        .collect(),
    }
}

/// Build the workspace-owned open-tab snapshot without exposing workspace
/// entities to the palette UI.
pub fn tabs_submenu(rows: impl IntoIterator<Item = (u64, String, String)>) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new("tabs", "Open Tabs", CommandSubmenu::Tabs),
        items: rows
            .into_iter()
            .map(|(id, title, kind)| SubmenuItem {
                id: id.to_string(),
                title,
                subtitle: Some(kind),
                active: false,
                action: SubmenuAction::SwitchToTab(id),
                secondary: None,
            })
            .collect(),
    }
}

pub fn hidden_status_items_submenu(
    rows: impl IntoIterator<Item = (String, String)>,
) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new(
            "status-bar-hidden",
            "Hidden Status Bar Items",
            CommandSubmenu::StatusBarHidden,
        ),
        items: rows
            .into_iter()
            .map(|(id, title)| SubmenuItem {
                action: SubmenuAction::ShowStatusBarItem(id.clone()),
                id,
                title,
                subtitle: None,
                active: false,
                secondary: None,
            })
            .collect(),
    }
}

/// User-facing labels for the workspace status-bar registry.
pub fn status_item_label(id: &str) -> &'static str {
    match id {
        "dock-buttons-left" => "Left Dock Buttons",
        "dock-buttons-right" => "Right Dock Buttons",
        "dock-buttons-bottom" => "Bottom Dock Buttons",
        "notifications" => "Notifications",
        "cwd" => "CWD Breadcrumb",
        "cursor-position" => "Cursor Position",
        "preview-url" => "Preview URL",
        "updater" => "Updater",
        "transfers" => "Transfers",
        "agent-access" => "Agent Access",
        _ => "Status Bar Item",
    }
}

impl CommandProvider for WorkspaceCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        let mut commands = vec![
            CommandDescriptor::new(CommandId::OpenProject, "Open Project…", "Workspace")
                .with_icon(CommandIcon::Folder),
            CommandDescriptor::new(
                CommandId::ReturnToStandalone,
                "Return to Standalone",
                "Workspace",
            )
            .with_icon(CommandIcon::Folder),
            CommandDescriptor::new(CommandId::NewTerminalTab, "New Terminal Tab", "Layout")
                .with_shortcut(ShortcutId::TabNew)
                .with_default_binding("cmd-t", None)
                .with_icon(CommandIcon::Terminal),
            CommandDescriptor::new(CommandId::NewEditorTab, "New Editor Tab", "Layout")
                .with_shortcut(ShortcutId::TabNewEditor)
                .with_default_binding("cmd-e", None)
                .with_icon(CommandIcon::File),
            CommandDescriptor::new(CommandId::NewPreviewTab, "New Preview Tab", "Layout")
                .with_shortcut(ShortcutId::TabNewPreview)
                .with_default_binding("cmd-shift-p", None)
                .with_icon(CommandIcon::File),
            CommandDescriptor::new(CommandId::Save, "Save", "Tab Actions")
                .with_icon(CommandIcon::Edit),
            CommandDescriptor::new(CommandId::CloseTab, "Close Current Tab", "Tab Actions")
                .with_shortcut(ShortcutId::TabClose)
                .with_default_binding("cmd-w", None)
                .with_icon(CommandIcon::Close),
            CommandDescriptor::new(CommandId::DuplicateTab, "Duplicate Tab", "Layout")
                .with_icon(CommandIcon::Copy),
            CommandDescriptor::new(CommandId::CloseOtherTabs, "Close Other Tabs", "Layout")
                .with_icon(CommandIcon::Close),
            CommandDescriptor::new(CommandId::NextTab, "Next Tab", "Tab Actions")
                .with_shortcut(ShortcutId::TabNext)
                .with_default_binding("ctrl-tab", None)
                .with_icon(CommandIcon::ChevronRight),
            CommandDescriptor::new(CommandId::PrevTab, "Previous Tab", "Tab Actions")
                .with_shortcut(ShortcutId::TabPrev)
                .with_default_binding("ctrl-shift-tab", None)
                .with_icon(CommandIcon::ChevronRight),
            CommandDescriptor::new(CommandId::FocusNextPane, "Focus Next Pane", "Layout")
                .with_shortcut(ShortcutId::PaneFocusNext)
                .with_default_binding("cmd-]", None)
                .with_icon(CommandIcon::ChevronRight),
            CommandDescriptor::new(CommandId::SwitchTab, "Switch Tab…", "Layout")
                .with_icon(CommandIcon::Terminal)
                .with_submenu(CommandSubmenu::Tabs),
            CommandDescriptor::new(CommandId::AdjustFontSize, "Adjust Font Size…", "Layout")
                .with_contexts(&[CommandContext::Terminal, CommandContext::Editor])
                .with_icon(CommandIcon::ChevronDown)
                .with_submenu(CommandSubmenu::Zoom),
            CommandDescriptor::new(CommandId::ToggleSidebar, "Toggle File Explorer", "View")
                .with_shortcut(ShortcutId::SidebarToggle)
                .with_default_binding("cmd-b", None)
                .with_icon(CommandIcon::PanelLeft),
            CommandDescriptor::new(
                CommandId::ShowStatusBarItem,
                "Statusbar: Show Hidden Item…",
                "View",
            )
            .with_icon(CommandIcon::Eye)
            .with_submenu(CommandSubmenu::StatusBarHidden),
            CommandDescriptor::new(CommandId::ZoomIn, "Zoom In", "View")
                .with_shortcut(ShortcutId::ViewZoomIn)
                .with_default_binding("cmd-=", None)
                .with_icon(CommandIcon::Plus),
            CommandDescriptor::new(CommandId::ZoomOut, "Zoom Out", "View")
                .with_shortcut(ShortcutId::ViewZoomOut)
                .with_default_binding("cmd--", None)
                .with_icon(CommandIcon::Minus),
            CommandDescriptor::new(CommandId::ZoomReset, "Reset Zoom", "View")
                .with_shortcut(ShortcutId::ViewZoomReset)
                .with_default_binding("cmd-0", None)
                .with_icon(CommandIcon::Refresh),
        ];

        for (id, index, shortcut) in [
            (CommandId::SelectTab1, 1, ShortcutId::TabSelect1),
            (CommandId::SelectTab2, 2, ShortcutId::TabSelect2),
            (CommandId::SelectTab3, 3, ShortcutId::TabSelect3),
            (CommandId::SelectTab4, 4, ShortcutId::TabSelect4),
            (CommandId::SelectTab5, 5, ShortcutId::TabSelect5),
            (CommandId::SelectTab6, 6, ShortcutId::TabSelect6),
            (CommandId::SelectTab7, 7, ShortcutId::TabSelect7),
            (CommandId::SelectTab8, 8, ShortcutId::TabSelect8),
            (CommandId::SelectTab9, 9, ShortcutId::TabSelect9),
        ] {
            commands.push(
                CommandDescriptor::new(id, format!("Select Tab {index}"), "Tab Actions")
                    .with_shortcut(shortcut)
                    .with_default_binding(format!("cmd-{index}"), None)
                    .with_icon(CommandIcon::Terminal),
            );
        }

        commands.extend([
            CommandDescriptor::new(CommandId::SplitRight, "Split Pane Right", "Layout")
                .with_contexts(&[CommandContext::Terminal])
                .with_shortcut(ShortcutId::PaneSplitRight)
                .with_default_binding("cmd-d", Some(CommandContext::Terminal))
                .with_icon(CommandIcon::ChevronRight),
            CommandDescriptor::new(CommandId::SplitDown, "Split Pane Down", "Layout")
                .with_contexts(&[CommandContext::Terminal])
                .with_shortcut(ShortcutId::PaneSplitDown)
                .with_default_binding("cmd-shift-d", Some(CommandContext::Terminal))
                .with_icon(CommandIcon::ChevronDown),
            CommandDescriptor::new(CommandId::ClosePane, "Close Active Pane", "Layout")
                .with_contexts(&[CommandContext::Terminal])
                .with_shortcut(ShortcutId::PaneClose)
                .with_default_binding("cmd-shift-w", Some(CommandContext::Terminal))
                .with_icon(CommandIcon::Close),
        ]);

        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_provider_owns_project_lifecycle_commands() {
        let commands = WorkspaceCommandProvider.commands();
        assert!(commands
            .iter()
            .any(|command| command.id == CommandId::OpenProject));
        assert!(commands
            .iter()
            .any(|command| command.id == CommandId::ReturnToStandalone));
        assert!(commands
            .iter()
            .any(|command| command.id == CommandId::SwitchTab));
        assert!(commands
            .iter()
            .any(|command| command.id == CommandId::SplitRight));
    }
}
