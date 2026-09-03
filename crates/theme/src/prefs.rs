//! Theme-selection preference enums.
//!
//! [`ThemePreference`] (System / Light / Dark) and [`EditorThemeId`] (the
//! editor syntax colour scheme) are the user-facing choices that drive the
//! runtime theme store. They live here — below every UI crate — so that
//! `labonair-command-palette` and the settings track can name them without
//! depending on `crates/ui`. The runtime store (`ThemeStore`) re-exports them
//! from `crate::theme` so existing `crate::theme::` paths keep working.

/// The theme preference the user picked in settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemePreference {
    /// Follow the operating-system appearance.
    #[default]
    System,
    /// Always use the light theme.
    Light,
    /// Always use the dark theme.
    Dark,
}

/// The editor colour scheme the user picked. `Auto` derives syntax colours
/// from the active app theme's tokens (so it follows light/dark and imported
/// themes); the named schemes mirror Labonair's CodeMirror theme set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorThemeId {
    #[default]
    Auto,
    Atomone,
    Aura,
    Copilot,
    GithubDark,
    GithubLight,
    Nord,
    TokyoNight,
    XcodeDark,
    XcodeLight,
}

impl EditorThemeId {
    /// The stable slug used in settings / theme files.
    pub fn slug(&self) -> &'static str {
        match self {
            EditorThemeId::Auto => "auto",
            EditorThemeId::Atomone => "atomone",
            EditorThemeId::Aura => "aura",
            EditorThemeId::Copilot => "copilot",
            EditorThemeId::GithubDark => "github-dark",
            EditorThemeId::GithubLight => "github-light",
            EditorThemeId::Nord => "nord",
            EditorThemeId::TokyoNight => "tokyo-night",
            EditorThemeId::XcodeDark => "xcode-dark",
            EditorThemeId::XcodeLight => "xcode-light",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Some(match slug {
            "auto" => EditorThemeId::Auto,
            "atomone" => EditorThemeId::Atomone,
            "aura" => EditorThemeId::Aura,
            "copilot" => EditorThemeId::Copilot,
            "github-dark" => EditorThemeId::GithubDark,
            "github-light" => EditorThemeId::GithubLight,
            "nord" => EditorThemeId::Nord,
            "tokyo-night" => EditorThemeId::TokyoNight,
            "xcode-dark" => EditorThemeId::XcodeDark,
            "xcode-light" => EditorThemeId::XcodeLight,
            _ => return None,
        })
    }

    pub const ALL: [EditorThemeId; 10] = [
        EditorThemeId::Auto,
        EditorThemeId::Atomone,
        EditorThemeId::Aura,
        EditorThemeId::Copilot,
        EditorThemeId::GithubDark,
        EditorThemeId::GithubLight,
        EditorThemeId::Nord,
        EditorThemeId::TokyoNight,
        EditorThemeId::XcodeDark,
        EditorThemeId::XcodeLight,
    ];
}
