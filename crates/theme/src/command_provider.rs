//! Theme-owned command metadata.

use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu,
    SubmenuAction, SubmenuDescriptor, SubmenuItem, SubmenuSnapshot,
};

use crate::EditorThemeId;
use crate::ThemePreference;

#[derive(Clone, Copy, Debug, Default)]
pub struct ThemeCommandProvider;

pub fn color_mode_submenu(active: ThemePreference) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new("color-mode", "Color Mode", CommandSubmenu::ColorMode),
        items: [
            (ThemePreference::Dark, "dark", "Dark Mode"),
            (ThemePreference::Light, "light", "Light Mode"),
            (ThemePreference::System, "system", "System (Auto)"),
        ]
        .into_iter()
        .map(|(mode, id, title)| SubmenuItem {
            id: id.to_string(),
            title: title.to_string(),
            subtitle: None,
            active: mode == active,
            action: SubmenuAction::SetColorMode(id.to_string()),
            secondary: None,
        })
        .collect(),
    }
}

pub fn editor_themes_submenu(
    rows: impl IntoIterator<Item = (EditorThemeId, bool)>,
) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new(
            "editor-themes",
            "Editor Themes",
            CommandSubmenu::EditorTheme,
        ),
        items: rows
            .into_iter()
            .map(|(id, active)| SubmenuItem {
                id: id.slug().to_string(),
                title: theme_label(id.slug()),
                subtitle: None,
                active,
                action: SubmenuAction::SetEditorTheme(id.slug().to_string()),
                secondary: None,
            })
            .collect(),
    }
}

pub fn app_themes_submenu(
    rows: impl IntoIterator<Item = (String, String, bool)>,
) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new("app-themes", "App Themes", CommandSubmenu::Themes),
        items: rows
            .into_iter()
            .map(|(id, title, active)| SubmenuItem {
                action: SubmenuAction::SetAppTheme(id.clone()),
                id,
                title,
                subtitle: None,
                active,
                secondary: None,
            })
            .collect(),
    }
}

pub fn icon_themes_submenu(
    rows: impl IntoIterator<Item = (String, String, Option<String>, bool)>,
) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new(
            "icon-themes",
            "Icon Themes",
            CommandSubmenu::IconThemes,
        ),
        items: rows
            .into_iter()
            .map(|(id, title, subtitle, active)| SubmenuItem {
                action: SubmenuAction::SetIconTheme(id.clone()),
                id,
                title,
                subtitle,
                active,
                secondary: None,
            })
            .collect(),
    }
}

fn theme_label(slug: &str) -> String {
    slug.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl CommandProvider for ThemeCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::ChangeAppTheme, "Change App Theme…", "View")
                .with_icon(CommandIcon::Sparkles)
                .with_submenu(CommandSubmenu::Themes),
            CommandDescriptor::new(CommandId::ChangeIconTheme, "Change Icon Theme…", "View")
                .with_icon(CommandIcon::Palette)
                .with_submenu(CommandSubmenu::IconThemes),
            CommandDescriptor::new(CommandId::ChangeColorMode, "Change Color Mode…", "View")
                .with_icon(CommandIcon::ChevronDown)
                .with_submenu(CommandSubmenu::ColorMode),
            CommandDescriptor::new(CommandId::ChangeEditorTheme, "Change Editor Theme…", "View")
                .with_contexts(&[CommandContext::Editor])
                .with_icon(CommandIcon::Sparkles)
                .with_submenu(CommandSubmenu::EditorTheme),
        ]
    }
}
