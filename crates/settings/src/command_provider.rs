//! Settings-owned command metadata.

use gpui::App;

use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider,
};
use labonair_command_palette_runtime::CommandHandlerRegistry;

use crate::{Settings as _, SettingsStore, ThemeSettings};
use labonair_settings_content::editor::EditorModalMode;

#[derive(Clone, Copy, Debug, Default)]
pub struct SettingsCommandProvider;

fn toggle_editor_mode(current: EditorModalMode, target: EditorModalMode) -> EditorModalMode {
    if current == target {
        EditorModalMode::Normal
    } else {
        target
    }
}

fn toggle_setting_bool(key: &str, cx: &mut App) {
    if !cx.has_global::<SettingsStore>() {
        return;
    }
    let _ = cx
        .global_mut::<SettingsStore>()
        .update_user_settings(|c| match key {
            "zenModeShowHeader" => {
                c.appearance.zen_mode_show_header =
                    Some(!c.appearance.zen_mode_show_header.unwrap_or(true));
            }
            "zenModeShowStatusbar" => {
                c.appearance.zen_mode_show_statusbar =
                    Some(!c.appearance.zen_mode_show_statusbar.unwrap_or(true));
            }
            "editorWordWrap" => {
                c.editor.editor_word_wrap = Some(!c.editor.editor_word_wrap.unwrap_or(false));
            }
            "editorLineNumbers" => {
                c.editor.editor_line_numbers = Some(!c.editor.editor_line_numbers.unwrap_or(true));
            }
            "terminalCursorBlink" => {
                c.terminal.terminal_cursor_blink =
                    Some(!c.terminal.terminal_cursor_blink.unwrap_or(true));
            }
            "vimMode" | "helixMode" => {
                let current = c.editor.editor_modal_mode.unwrap_or_else(|| {
                    if c.editor.editor_vim_mode.unwrap_or(false) {
                        EditorModalMode::Vim
                    } else {
                        EditorModalMode::Normal
                    }
                });
                let target = if key == "vimMode" {
                    EditorModalMode::Vim
                } else {
                    EditorModalMode::Helix
                };
                c.editor.editor_modal_mode = Some(toggle_editor_mode(current, target));
            }
            _ => {}
        });
}

fn toggle_zen_mode(cx: &mut App) {
    let (show_header, show_statusbar) = ThemeSettings::try_get(cx)
        .map(|settings| {
            (
                settings.zen_mode_show_header(),
                settings.zen_mode_show_statusbar(),
            )
        })
        .unwrap_or((true, true));
    if cx.has_global::<SettingsStore>() {
        let _ = cx.global_mut::<SettingsStore>().update_user_settings(|c| {
            let next = !(show_header || show_statusbar);
            c.appearance.zen_mode_show_header = Some(next);
            c.appearance.zen_mode_show_statusbar = Some(next);
        });
    }
}

/// Register executable handlers for settings-owned toggle commands.
pub fn register_handlers(registry: &mut CommandHandlerRegistry) {
    registry
        .register(CommandId::ToggleZenMode, |_window, cx| toggle_zen_mode(cx))
        .expect("settings command handler must have a unique id");

    for (id, key) in [
        (CommandId::ToggleZenModeHeader, "zenModeShowHeader"),
        (CommandId::ToggleZenModeStatusbar, "zenModeShowStatusbar"),
        (CommandId::ToggleEditorWordWrap, "editorWordWrap"),
        (CommandId::ToggleLineNumbers, "editorLineNumbers"),
        (CommandId::ToggleCursorBlink, "terminalCursorBlink"),
        (CommandId::ToggleVimMode, "vimMode"),
        (CommandId::ToggleHelixMode, "helixMode"),
    ] {
        registry
            .register(id, move |_window, cx| toggle_setting_bool(key, cx))
            .expect("settings command handler must have a unique id");
    }
}

impl CommandProvider for SettingsCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        let commands = vec![
            CommandDescriptor::new(CommandId::OpenSettings, "Open Settings", "Application")
                .with_icon(CommandIcon::Edit),
            CommandDescriptor::new(
                CommandId::OpenProjectSettings,
                "Open Project Settings (.labonair/settings.json)",
                "Application",
            )
            .with_icon(CommandIcon::Edit),
            CommandDescriptor::new(
                CommandId::OpenSettingsJson,
                "Open Settings (JSON)",
                "Application",
            )
            .with_icon(CommandIcon::Edit),
            CommandDescriptor::new(CommandId::ToggleZenMode, "Toggle: Zen Mode", "Settings")
                .with_default_binding("cmd-shift-z", None)
                .with_icon(CommandIcon::Eye),
            CommandDescriptor::new(
                CommandId::ToggleZenModeHeader,
                "Toggle: Show Header Bar",
                "Settings",
            )
            .with_icon(CommandIcon::Eye),
            CommandDescriptor::new(
                CommandId::ToggleZenModeStatusbar,
                "Toggle: Show Status Bar",
                "Settings",
            )
            .with_icon(CommandIcon::Eye),
            CommandDescriptor::new(
                CommandId::ToggleEditorWordWrap,
                "Toggle: Editor Word Wrap",
                "Settings",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_icon(CommandIcon::ChevronDown),
            CommandDescriptor::new(
                CommandId::ToggleLineNumbers,
                "Toggle: Line Numbers",
                "Settings",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_icon(CommandIcon::Check),
            CommandDescriptor::new(
                CommandId::ToggleCursorBlink,
                "Toggle: Terminal Cursor Blink",
                "Settings",
            )
            .with_contexts(&[CommandContext::Terminal])
            .with_icon(CommandIcon::Eye),
            CommandDescriptor::new(CommandId::ToggleVimMode, "Toggle: Vim Mode", "Settings")
                .with_icon(CommandIcon::Check),
            CommandDescriptor::new(CommandId::ToggleHelixMode, "Toggle: Helix Mode", "Settings")
                .with_icon(CommandIcon::Check),
        ];

        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vim_and_helix_mode_toggles_are_mutually_exclusive() {
        assert_eq!(
            toggle_editor_mode(EditorModalMode::Normal, EditorModalMode::Vim),
            EditorModalMode::Vim
        );
        assert_eq!(
            toggle_editor_mode(EditorModalMode::Vim, EditorModalMode::Helix),
            EditorModalMode::Helix
        );
        assert_eq!(
            toggle_editor_mode(EditorModalMode::Helix, EditorModalMode::Helix),
            EditorModalMode::Normal
        );
    }
}
