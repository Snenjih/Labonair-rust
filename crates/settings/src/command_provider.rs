//! Settings-owned command metadata.

use gpui::App;

use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu,
    PaletteAction, SubmenuAction, SubmenuDescriptor, SubmenuItem, SubmenuSnapshot,
};
use labonair_command_palette_runtime::{CommandHandlerRegistry, PaletteActionHandlerRegistry};

use crate::{Settings as _, SettingsStore, ThemeSettings};
use labonair_settings_content::editor::EditorModalMode;
use labonair_settings_content::SettingsContent;

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

/// One boolean setting exposed as a `Toggle: …` palette row. `get` reads the
/// merged value (defaults are always populated), `set` writes the user layer.
struct BoolToggle {
    id: CommandId,
    title: &'static str,
    section: &'static str,
    contexts: &'static [CommandContext],
    icon: CommandIcon,
    get: fn(&SettingsContent) -> bool,
    set: fn(&mut SettingsContent, bool),
}

macro_rules! bool_toggle {
    ($id:ident, $title:literal, $section:literal, $contexts:expr, $area:ident . $field:ident, $default:literal) => {
        BoolToggle {
            id: CommandId::$id,
            title: $title,
            section: $section,
            contexts: $contexts,
            icon: CommandIcon::Check,
            get: |c| c.$area.$field.unwrap_or($default),
            set: |c, value| c.$area.$field = Some(value),
        }
    };
}

const EDITOR: &[CommandContext] = &[CommandContext::Editor];
const TERMINAL: &[CommandContext] = &[CommandContext::Terminal, CommandContext::SshTerminal];

/// The settings reachable from the palette without opening the Settings page.
/// One row per entry: add a setting here (plus its `CommandId`) and the
/// descriptor, handler and ON/OFF state follow.
const BOOL_TOGGLES: &[BoolToggle] = &[
    bool_toggle!(
        ToggleDimInactivePanes,
        "Toggle: Dim Inactive Panes",
        "View",
        &[],
        appearance.dim_inactive_panes,
        false
    ),
    bool_toggle!(
        TogglePaneHighlight,
        "Toggle: Pane Highlight",
        "View",
        &[],
        appearance.pane_highlight,
        false
    ),
    bool_toggle!(
        ToggleReduceMotion,
        "Toggle: Reduce Motion",
        "View",
        &[],
        appearance.reduce_motion,
        false
    ),
    bool_toggle!(
        ToggleTranslucentSurfaces,
        "Toggle: Translucent Surfaces",
        "View",
        &[],
        appearance.translucent_surfaces,
        false
    ),
    bool_toggle!(
        ToggleRelativeLineNumbers,
        "Toggle: Relative Line Numbers",
        "Editor",
        EDITOR,
        editor.editor_relative_line_numbers,
        false
    ),
    bool_toggle!(
        ToggleIndentGuides,
        "Toggle: Indentation Guides",
        "Editor",
        EDITOR,
        editor.editor_indentation_guides,
        true
    ),
    bool_toggle!(
        ToggleStickyContext,
        "Toggle: Sticky Scroll",
        "Editor",
        EDITOR,
        editor.editor_sticky_context,
        false
    ),
    bool_toggle!(
        ToggleBracketMatching,
        "Toggle: Bracket Matching",
        "Editor",
        EDITOR,
        editor.editor_bracket_matching,
        true
    ),
    bool_toggle!(
        ToggleGitGutter,
        "Toggle: Git Gutter",
        "Editor",
        EDITOR,
        editor.editor_git_gutter,
        true
    ),
    bool_toggle!(
        ToggleGitBlame,
        "Toggle: Git Blame",
        "Editor",
        EDITOR,
        editor.editor_git_blame_status,
        false
    ),
    bool_toggle!(
        ToggleDiagnostics,
        "Toggle: Diagnostics",
        "Editor",
        EDITOR,
        editor.editor_diagnostics,
        true
    ),
    bool_toggle!(
        ToggleErrorLens,
        "Toggle: Error Lens",
        "Editor",
        EDITOR,
        editor.editor_error_lens,
        false
    ),
    bool_toggle!(
        ToggleSemanticTokens,
        "Toggle: Semantic Highlighting",
        "Editor",
        EDITOR,
        editor.editor_semantic_tokens,
        true
    ),
    bool_toggle!(
        ToggleFormatOnSave,
        "Toggle: Format on Save",
        "Editor",
        EDITOR,
        editor.editor_format_on_save,
        false
    ),
    bool_toggle!(
        ToggleAutoSave,
        "Toggle: Auto Save",
        "Editor",
        EDITOR,
        editor.editor_auto_save,
        false
    ),
    bool_toggle!(
        ToggleTrimTrailingWhitespace,
        "Toggle: Trim Trailing Whitespace",
        "Editor",
        EDITOR,
        editor.editor_trim_trailing_whitespace,
        false
    ),
    bool_toggle!(
        ToggleCopyOnSelect,
        "Toggle: Copy on Select",
        "Terminal",
        TERMINAL,
        terminal.terminal_copy_on_select,
        false
    ),
    bool_toggle!(
        ToggleRightClickPastes,
        "Toggle: Right-Click Pastes",
        "Terminal",
        TERMINAL,
        terminal.terminal_right_click_pastes,
        false
    ),
    bool_toggle!(
        ToggleTerminalBell,
        "Toggle: Terminal Bell",
        "Terminal",
        TERMINAL,
        terminal.terminal_bell,
        true
    ),
    bool_toggle!(
        ToggleConfirmCloseTerminalTab,
        "Toggle: Confirm Closing Terminal Tabs",
        "Terminal",
        TERMINAL,
        terminal.confirm_close_terminal_tab,
        true
    ),
];

/// Current value of a table-driven toggle (`None` for any other command), so
/// the palette can show its ON/OFF state.
pub fn bool_toggle_state(id: CommandId, cx: &App) -> Option<bool> {
    let toggle = BOOL_TOGGLES.iter().find(|toggle| toggle.id == id)?;
    let store = cx.try_global::<SettingsStore>()?;
    Some((toggle.get)(store.merged()))
}

fn flip_bool_toggle(toggle: &BoolToggle, cx: &mut App) {
    if !cx.has_global::<SettingsStore>() {
        return;
    }
    let next = !(toggle.get)(cx.global::<SettingsStore>().merged());
    let _ = cx
        .global_mut::<SettingsStore>()
        .update_user_settings(|c| (toggle.set)(c, next));
}

/// The enumerated settings the palette can write through
/// [`SubmenuAction::SetChoice`]. Returns `false` for an unknown key/value.
pub fn set_choice(setting: &str, value: &str, cx: &mut App) -> bool {
    use labonair_settings_content::terminal::CursorStyle;

    if !cx.has_global::<SettingsStore>() {
        return false;
    }
    let cursor = match value {
        "block" => Some(CursorStyle::Block),
        "bar" => Some(CursorStyle::Bar),
        "underline" => Some(CursorStyle::Underline),
        _ => None,
    };
    let store = cx.global_mut::<SettingsStore>();
    if setting == "uiDensity" && matches!(value, "compact" | "default" | "comfortable") {
        let _ = store.update_user_settings(|c| c.appearance.ui_density = Some(value.to_string()));
        true
    } else if let (true, Some(style)) = (setting == "terminalCursorStyle", cursor) {
        let _ = store.update_user_settings(|c| c.terminal.terminal_cursor_style = Some(style));
        true
    } else {
        false
    }
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

    for toggle in BOOL_TOGGLES {
        registry
            .register(toggle.id, move |_window, cx| flip_bool_toggle(toggle, cx))
            .expect("settings command handler must have a unique id");
    }

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

/// Register the dynamic palette rows owned by Settings (`SetChoice`).
pub fn register_palette_action_handlers(registry: &mut PaletteActionHandlerRegistry) {
    registry
        .register("settings", |action, _window, cx| match action {
            PaletteAction::Submenu(SubmenuAction::SetChoice { setting, value }) => {
                set_choice(setting, value, cx)
            }
            _ => false,
        })
        .expect("settings palette action handler must have a unique owner");
}

/// Rows for the Settings-owned picker sub-pages. `current` is the active wire
/// value.
pub fn choice_submenu(
    submenu: CommandSubmenu,
    setting: &str,
    title: &str,
    current: &str,
) -> SubmenuSnapshot {
    let choices: &[(&str, &str)] = match setting {
        "uiDensity" => &[
            ("compact", "Compact"),
            ("default", "Default"),
            ("comfortable", "Comfortable"),
        ],
        "terminalCursorStyle" => &[
            ("block", "Block"),
            ("bar", "Bar"),
            ("underline", "Underline"),
        ],
        _ => &[],
    };
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new(setting, title, submenu),
        items: choices
            .iter()
            .map(|(value, label)| SubmenuItem {
                id: (*value).to_string(),
                title: (*label).to_string(),
                subtitle: None,
                active: *value == current,
                action: SubmenuAction::SetChoice {
                    setting: setting.to_string(),
                    value: (*value).to_string(),
                },
                secondary: None,
            })
            .collect(),
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
            CommandDescriptor::new(CommandId::ChangeUiDensity, "Change UI Density…", "View")
                .with_icon(CommandIcon::PanelTop)
                .with_submenu(CommandSubmenu::UiDensity),
            CommandDescriptor::new(
                CommandId::ChangeTerminalCursorStyle,
                "Change Terminal Cursor Style…",
                "Terminal",
            )
            .with_contexts(TERMINAL)
            .with_icon(CommandIcon::Terminal)
            .with_submenu(CommandSubmenu::TerminalCursorStyle),
        ];
        let toggles = BOOL_TOGGLES.iter().map(|toggle| {
            CommandDescriptor::new(toggle.id, toggle.title, toggle.section)
                .with_contexts(toggle.contexts)
                .with_icon(toggle.icon)
        });
        commands.into_iter().chain(toggles).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bool_toggles_are_unique_and_round_trip_through_the_content_tree() {
        let mut ids = std::collections::HashSet::new();
        for toggle in BOOL_TOGGLES {
            assert!(ids.insert(toggle.id), "{:?} listed twice", toggle.id);
            let mut content = SettingsContent::defaults();
            let before = (toggle.get)(&content);
            (toggle.set)(&mut content, !before);
            assert_eq!((toggle.get)(&content), !before, "{:?}", toggle.id);
        }
    }

    #[test]
    fn every_bool_toggle_has_a_descriptor() {
        let descriptors = SettingsCommandProvider.commands();
        for toggle in BOOL_TOGGLES {
            assert!(
                descriptors.iter().any(|d| d.id == toggle.id),
                "{:?}",
                toggle.id
            );
        }
    }

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
