//! Workspace-owned command metadata.
//!
//! The workspace capability contributes its discoverable commands without
//! importing the palette UI or the application shell. Execution is still
//! connected by the composition root until command handlers move behind
//! capability-owned action services.

use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu,
    SubmenuAction, SubmenuDescriptor, SubmenuItem, SubmenuSnapshot,
};
use labonair_keymap::ShortcutId;

/// Stable command metadata contributed by the workspace capability.
#[derive(Clone, Copy, Debug, Default)]
pub struct WorkspaceCommandProvider;

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
                .with_icon(CommandIcon::Terminal),
            CommandDescriptor::new(CommandId::NewEditorTab, "New Editor Tab", "Layout")
                .with_shortcut(ShortcutId::TabNewEditor)
                .with_icon(CommandIcon::File),
            CommandDescriptor::new(CommandId::NewPreviewTab, "New Preview Tab", "Layout")
                .with_shortcut(ShortcutId::TabNewPreview)
                .with_icon(CommandIcon::File),
            CommandDescriptor::new(CommandId::Save, "Save", "Tab Actions")
                .with_icon(CommandIcon::Edit),
            CommandDescriptor::new(CommandId::CloseTab, "Close Current Tab", "Tab Actions")
                .with_shortcut(ShortcutId::TabClose)
                .with_icon(CommandIcon::Close),
            CommandDescriptor::new(CommandId::DuplicateTab, "Duplicate Tab", "Layout")
                .with_icon(CommandIcon::Copy),
            CommandDescriptor::new(CommandId::CloseOtherTabs, "Close Other Tabs", "Layout")
                .with_icon(CommandIcon::Close),
            CommandDescriptor::new(CommandId::NextTab, "Next Tab", "Tab Actions")
                .with_shortcut(ShortcutId::TabNext)
                .with_icon(CommandIcon::ChevronRight),
            CommandDescriptor::new(CommandId::PrevTab, "Previous Tab", "Tab Actions")
                .with_shortcut(ShortcutId::TabPrev)
                .with_icon(CommandIcon::ChevronRight),
            CommandDescriptor::new(CommandId::FocusNextPane, "Focus Next Pane", "Layout")
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
                .with_icon(CommandIcon::Plus),
            CommandDescriptor::new(CommandId::ZoomOut, "Zoom Out", "View")
                .with_shortcut(ShortcutId::ViewZoomOut)
                .with_icon(CommandIcon::Minus),
            CommandDescriptor::new(CommandId::ZoomReset, "Reset Zoom", "View")
                .with_shortcut(ShortcutId::ViewZoomReset)
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
                    .with_icon(CommandIcon::Terminal),
            );
        }

        commands.extend([
            CommandDescriptor::new(CommandId::SplitRight, "Split Pane Right", "Layout")
                .with_contexts(&[CommandContext::Terminal])
                .with_shortcut(ShortcutId::PaneSplitRight)
                .with_icon(CommandIcon::ChevronRight),
            CommandDescriptor::new(CommandId::SplitDown, "Split Pane Down", "Layout")
                .with_contexts(&[CommandContext::Terminal])
                .with_shortcut(ShortcutId::PaneSplitDown)
                .with_icon(CommandIcon::ChevronDown),
            CommandDescriptor::new(CommandId::ClosePane, "Close Active Pane", "Layout")
                .with_contexts(&[CommandContext::Terminal])
                .with_shortcut(ShortcutId::PaneClose)
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
