//! Runtime theme provider and store (T02-002).
//!
//! [`ThemeStore`] is a GPUI entity and the single source of truth for which
//! [`Theme`] is currently active. It holds the user's [`ThemePreference`]
//! (System / Light / Dark), the [`ThemeMode`] resolved from the system
//! appearance, the default light/dark themes (built once from the design tokens
//! in `labonair-theme`), and an optional imported custom theme (T02-003).
//!
//! Access from UI components goes through the [`GlobalTheme`] global — see
//! [`active_theme`] / [`theme_store`]. Components must never hold their own
//! theme state.

use gpui::{
    font, App, AppContext, Context, Entity, Font, FontFallbacks, FontFeatures, FontWeight, Global,
    Hsla, WindowAppearance,
};
use labonair_theme::{Animation, MonoFontWeight, RadiusScale, Shadows, Theme, ThemeFile};

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

/// The concrete appearance mode after resolving [`ThemePreference::System`]
/// against the current system appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
}

impl ThemeMode {
    /// Maps a GPUI [`WindowAppearance`] onto a light/dark mode.
    pub fn from_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => ThemeMode::Light,
            WindowAppearance::Dark | WindowAppearance::VibrantDark => ThemeMode::Dark,
        }
    }
}

/// Central theme state. Created once at startup and stored as a GPUI entity;
/// exposed app-wide via [`GlobalTheme`].
pub struct ThemeStore {
    preference: ThemePreference,
    /// Light/dark as reported by the system, regardless of `preference`.
    system_mode: ThemeMode,
    /// Default themes, built once from the design tokens — never recomputed.
    light: Theme,
    dark: Theme,
    /// An imported user theme, resolved for the current mode. When set it
    /// overrides the default theme (T02-003).
    custom: Option<Theme>,
    /// The source file for `custom`, kept so the imported theme can be
    /// re-resolved for the other mode when the appearance changes.
    custom_file: Option<ThemeFile>,
}

impl ThemeStore {
    /// Builds the store from the initial system appearance.
    pub fn new(appearance: WindowAppearance) -> Self {
        Self {
            preference: ThemePreference::default(),
            system_mode: ThemeMode::from_appearance(appearance),
            light: Theme::light(),
            dark: Theme::dark(),
            custom: None,
            custom_file: None,
        }
    }

    /// Re-resolves the imported theme (if any) against the current mode.
    fn reresolve_custom(&mut self) {
        let Some(file) = self.custom_file.clone() else {
            return;
        };
        let dark = self.mode() == ThemeMode::Dark;
        if let Ok((theme, _warnings)) = Theme::from_theme_file(&file, dark) {
            self.custom = Some(theme);
        }
    }

    /// The mode after applying the preference (System falls back to the
    /// system appearance).
    pub fn mode(&self) -> ThemeMode {
        match self.preference {
            ThemePreference::System => self.system_mode,
            ThemePreference::Light => ThemeMode::Light,
            ThemePreference::Dark => ThemeMode::Dark,
        }
    }

    pub fn preference(&self) -> ThemePreference {
        self.preference
    }

    /// Whether a custom user theme is currently active.
    pub fn has_custom_theme(&self) -> bool {
        self.custom.is_some()
    }

    /// The active theme: the custom theme if one is set, otherwise the default
    /// theme for the resolved mode. Cheap — no allocation.
    pub fn theme(&self) -> &Theme {
        if let Some(custom) = &self.custom {
            return custom;
        }
        match self.mode() {
            ThemeMode::Light => &self.light,
            ThemeMode::Dark => &self.dark,
        }
    }

    /// Sets the user preference. Triggers a re-render only if it changed.
    pub fn set_preference(&mut self, preference: ThemePreference, cx: &mut Context<Self>) {
        if self.preference == preference {
            return;
        }
        self.preference = preference;
        self.reresolve_custom();
        cx.notify();
    }

    /// Called when GPUI reports a system appearance change. Only re-renders if
    /// the resolved active theme actually changes (i.e. preference is System).
    pub fn set_system_appearance(&mut self, appearance: WindowAppearance, cx: &mut Context<Self>) {
        let mode = ThemeMode::from_appearance(appearance);
        if self.system_mode == mode {
            return;
        }
        let was_following = self.preference == ThemePreference::System;
        self.system_mode = mode;
        if was_following {
            self.reresolve_custom();
            cx.notify();
        }
    }

    /// Directly sets a resolved custom theme (`Some`) or clears it (`None`).
    /// Clears any imported [`ThemeFile`] source — use [`Self::import_theme_file`]
    /// to keep mode-following behaviour.
    pub fn set_custom_theme(&mut self, theme: Option<Theme>, cx: &mut Context<Self>) {
        if self.custom == theme && self.custom_file.is_none() {
            return;
        }
        self.custom = theme;
        self.custom_file = None;
        cx.notify();
    }

    /// Imports a user [`ThemeFile`], resolves it for the current mode and
    /// activates it. The file is validated first; a half-parsed theme is never
    /// set active. Returns any non-fatal warnings (unknown tokens, unparseable
    /// color values that fell back to defaults).
    pub fn import_theme_file(
        &mut self,
        file: ThemeFile,
        cx: &mut Context<Self>,
    ) -> Result<Vec<String>, String> {
        file.validate()?;
        let dark = self.mode() == ThemeMode::Dark;
        let (theme, warnings) = Theme::from_theme_file(&file, dark)?;
        self.custom = Some(theme);
        self.custom_file = Some(file);
        cx.notify();
        Ok(warnings)
    }

    /// Clears the active custom theme, reverting to the built-in light/dark
    /// theme for the resolved mode.
    pub fn clear_custom_theme(&mut self, cx: &mut Context<Self>) {
        if self.custom.is_none() && self.custom_file.is_none() {
            return;
        }
        self.custom = None;
        self.custom_file = None;
        cx.notify();
    }

    /// Serializes the active theme into a reusable [`ThemeFile`] for export.
    pub fn active_theme_file(&self, name: impl Into<String>) -> ThemeFile {
        self.theme().to_theme_file(name, "")
    }

    // --- Convenience accessors for UI components -------------------------

    pub fn background(&self) -> Hsla {
        self.theme().core.background
    }

    pub fn foreground(&self) -> Hsla {
        self.theme().core.foreground
    }

    pub fn card(&self) -> Hsla {
        self.theme().core.card
    }

    pub fn muted(&self) -> Hsla {
        self.theme().core.muted
    }

    pub fn muted_foreground(&self) -> Hsla {
        self.theme().core.muted_foreground
    }

    pub fn border(&self) -> Hsla {
        self.theme().core.border
    }

    pub fn primary(&self) -> Hsla {
        self.theme().core.primary
    }

    pub fn accent(&self) -> Hsla {
        self.theme().core.accent
    }

    /// `--toolbar-background` — the window header / titlebar surface.
    pub fn toolbar(&self) -> Hsla {
        self.theme().surface.toolbar
    }

    /// `--title-bar-background`.
    pub fn title_bar(&self) -> Hsla {
        self.theme().surface.title_bar
    }

    /// `--status-bar-background` — the bottom status bar surface.
    pub fn status_bar(&self) -> Hsla {
        self.theme().surface.status_bar
    }

    /// `--sidebar` background.
    pub fn sidebar_bg(&self) -> Hsla {
        self.theme().sidebar.background
    }

    /// `--sidebar-border`.
    pub fn sidebar_border(&self) -> Hsla {
        self.theme().sidebar.border
    }

    /// `--sidebar-foreground`.
    pub fn sidebar_fg(&self) -> Hsla {
        self.theme().sidebar.foreground
    }

    pub fn radius(&self) -> RadiusScale {
        self.theme().radius
    }

    pub fn shadows(&self) -> &Shadows {
        &self.theme().shadows
    }

    pub fn animation(&self) -> &Animation {
        &self.theme().animation
    }

    /// GPUI [`Font`] for UI text (Inter Variable + system fallbacks).
    pub fn ui_font(&self) -> Font {
        let t = &self.theme().typography;
        Font {
            fallbacks: Some(FontFallbacks::from_fonts(t.ui_font_fallback.clone())),
            ..font(t.app_font_family.clone())
        }
    }

    /// GPUI [`Font`] for the code editor buffer (JetBrains Mono, ligatures per theme).
    pub fn buffer_font(&self) -> Font {
        let t = &self.theme().typography;
        self.mono_font(&t.buffer_font_family, MonoFontWeight::Normal)
    }

    /// GPUI [`Font`] for the terminal, honoring the configured weight + ligatures.
    pub fn terminal_font(&self) -> Font {
        let t = &self.theme().typography;
        self.mono_font(&t.terminal_font_family, t.terminal_font_weight)
    }

    fn mono_font(&self, family: &str, weight: MonoFontWeight) -> Font {
        let t = &self.theme().typography;
        let features = if t.font_ligatures {
            FontFeatures::default()
        } else {
            FontFeatures::disable_ligatures()
        };
        Font {
            features,
            fallbacks: Some(FontFallbacks::from_fonts(t.mono_font_fallback.clone())),
            weight: match weight {
                MonoFontWeight::Normal => FontWeight::NORMAL,
                MonoFontWeight::Medium => FontWeight::MEDIUM,
                MonoFontWeight::Bold => FontWeight::BOLD,
            },
            ..font(family.to_string())
        }
    }

    /// Terminal font size in pixels (`preferencesStore.terminalFontSize`).
    pub fn terminal_font_size(&self) -> f32 {
        self.theme().typography.terminal_font_size
    }

    /// Terminal line-height multiple (`preferencesStore.terminalLineHeight`).
    pub fn terminal_line_height(&self) -> f32 {
        self.theme().typography.terminal_line_height
    }
}

/// Registers the bundled font assets ([`labonair_theme::embedded_fonts`]) with
/// GPUI's text system so the UI / terminal / editor render with Inter Variable
/// and JetBrains Mono regardless of what is installed on the system. Call once
/// at startup, before opening the window.
pub fn init_fonts(cx: &App) {
    if let Err(err) = cx.text_system().add_fonts(labonair_theme::embedded_fonts()) {
        // Non-fatal: GPUI falls back to system fonts.
        eprintln!("labonair-ui: failed to register bundled fonts: {err}");
    }
}

/// App-wide handle to the [`ThemeStore`] entity.
pub struct GlobalTheme(pub Entity<ThemeStore>);

impl Global for GlobalTheme {}

/// Creates the [`ThemeStore`] from the given appearance and installs it as the
/// [`GlobalTheme`]. Returns the entity so the caller can observe it / wire up
/// system appearance changes.
pub fn init(appearance: WindowAppearance, cx: &mut App) -> Entity<ThemeStore> {
    let store = cx.new(|_cx| ThemeStore::new(appearance));
    cx.set_global(GlobalTheme(store.clone()));
    store
}

/// The [`ThemeStore`] entity from the global. Panics if [`init`] has not run.
pub fn theme_store(cx: &App) -> Entity<ThemeStore> {
    cx.global::<GlobalTheme>().0.clone()
}

/// The currently active [`Theme`] from the global. Panics if [`init`] has not run.
pub fn active_theme(cx: &App) -> &Theme {
    cx.global::<GlobalTheme>().0.read(cx).theme()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn preference_resolves_to_the_right_theme(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| ThemeStore::new(WindowAppearance::Dark));
            store.update(cx, |s, cx| {
                assert!(s.theme().is_dark, "system Dark → dark theme");

                s.set_preference(ThemePreference::Light, cx);
                assert_eq!(s.mode(), ThemeMode::Light);
                assert!(!s.theme().is_dark);

                s.set_preference(ThemePreference::Dark, cx);
                assert!(s.theme().is_dark);

                s.set_preference(ThemePreference::System, cx);
                assert!(s.theme().is_dark, "back to System, system is Dark");
            });
        });
    }

    #[gpui::test]
    fn follows_system_appearance_when_preference_is_system(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| ThemeStore::new(WindowAppearance::Dark));
            store.update(cx, |s, cx| {
                assert!(s.theme().is_dark);
                s.set_system_appearance(WindowAppearance::Light, cx);
                assert_eq!(s.mode(), ThemeMode::Light);
                assert!(!s.theme().is_dark);

                // With an explicit preference, system changes don't move the theme.
                s.set_preference(ThemePreference::Dark, cx);
                s.set_system_appearance(WindowAppearance::VibrantLight, cx);
                assert!(s.theme().is_dark);
            });
        });
    }

    #[gpui::test]
    fn custom_theme_overrides_then_falls_back(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| ThemeStore::new(WindowAppearance::Dark));
            store.update(cx, |s, cx| {
                assert!(s.theme().is_dark);
                s.set_custom_theme(Some(Theme::light()), cx);
                assert!(s.has_custom_theme());
                assert!(!s.theme().is_dark, "custom theme overrides resolved mode");

                s.set_custom_theme(None, cx);
                assert!(!s.has_custom_theme());
                assert!(s.theme().is_dark, "falls back to the dark default");
            });
        });
    }

    #[gpui::test]
    fn accessors_read_from_the_active_theme(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| ThemeStore::new(WindowAppearance::Light));
            let s = store.read(cx);
            let t = s.theme();
            assert_eq!(s.background(), t.core.background);
            assert_eq!(s.foreground(), t.core.foreground);
            assert_eq!(s.card(), t.core.card);
            assert_eq!(s.muted(), t.core.muted);
            assert_eq!(s.muted_foreground(), t.core.muted_foreground);
            assert_eq!(s.border(), t.core.border);
            assert_eq!(s.primary(), t.core.primary);
            assert_eq!(s.accent(), t.core.accent);
            assert_eq!(s.radius(), t.radius);
            assert_eq!(s.shadows(), &t.shadows);
            assert_eq!(s.animation(), &t.animation);
        });
    }

    #[gpui::test]
    fn font_accessors_build_expected_gpui_fonts(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| ThemeStore::new(WindowAppearance::Dark));
            let s = store.read(cx);

            let ui = s.ui_font();
            assert_eq!(ui.family.as_ref(), labonair_theme::UI_FONT_FAMILY);
            assert!(ui
                .fallbacks
                .as_ref()
                .unwrap()
                .fallback_list()
                .contains(&"sans-serif".to_string()));

            let term = s.terminal_font();
            assert_eq!(term.family.as_ref(), labonair_theme::MONO_FONT_FAMILY);
            assert_eq!(term.weight, FontWeight::NORMAL);
            // Ligatures on by default → calt is not disabled.
            assert_ne!(term.features, FontFeatures::disable_ligatures());
            assert!(term
                .fallbacks
                .as_ref()
                .unwrap()
                .fallback_list()
                .contains(&"Menlo".to_string()));

            assert_eq!(
                s.buffer_font().family.as_ref(),
                labonair_theme::MONO_FONT_FAMILY
            );
            assert_eq!(s.terminal_font_size(), 14.0);
            assert_eq!(s.terminal_line_height(), 1.05);
        });
    }

    const SAMPLE_THEME: &str = r##"{
        "name": "Sample",
        "variants": {
            "dark":  { "mode": "dark",  "colors": { "primary": "#ff0000" } },
            "light": { "mode": "light", "colors": { "primary": "#0000ff" } }
        }
    }"##;

    #[gpui::test]
    fn import_theme_file_activates_and_follows_mode(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| ThemeStore::new(WindowAppearance::Dark));
            store.update(cx, |s, cx| {
                let file = ThemeFile::from_json(SAMPLE_THEME).unwrap();
                let warnings = s.import_theme_file(file, cx).unwrap();
                assert!(warnings.is_empty(), "{warnings:?}");
                assert!(s.has_custom_theme());
                assert_eq!(labonair_theme::to_rgb8(s.primary()), [0xff, 0x00, 0x00]);

                // Switching the resolved mode re-resolves the imported theme.
                s.set_preference(ThemePreference::Light, cx);
                assert_eq!(labonair_theme::to_rgb8(s.primary()), [0x00, 0x00, 0xff]);

                s.clear_custom_theme(cx);
                assert!(!s.has_custom_theme());
                assert_eq!(s.primary(), Theme::light().core.primary);
            });
        });
    }

    #[gpui::test]
    fn import_rejects_invalid_file_without_activating(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| ThemeStore::new(WindowAppearance::Dark));
            store.update(cx, |s, cx| {
                let bad = ThemeFile::from_json(
                    r#"{ "name": "OnlyDark", "variants": { "d": { "mode": "dark", "colors": {} } } }"#,
                )
                .unwrap();
                assert!(s.import_theme_file(bad, cx).is_err());
                assert!(!s.has_custom_theme(), "invalid theme must not become active");
            });
        });
    }

    #[gpui::test]
    fn export_then_import_round_trips(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| ThemeStore::new(WindowAppearance::Dark));
            store.update(cx, |s, cx| {
                let exported = s.active_theme_file("Exported");
                let json = exported.to_json().unwrap();
                let reparsed = ThemeFile::from_json(&json).unwrap();
                let warnings = s.import_theme_file(reparsed, cx).unwrap();
                assert!(warnings.is_empty(), "{warnings:?}");
                // Same colors as the dark default we exported from.
                assert_eq!(
                    labonair_theme::to_rgb8(s.background()),
                    labonair_theme::to_rgb8(Theme::dark().core.background)
                );
            });
        });
    }

    #[gpui::test]
    fn global_exposes_the_active_theme(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = init(WindowAppearance::Light, cx);
            assert!(!active_theme(cx).is_dark);
            store.update(cx, |s, cx| s.set_preference(ThemePreference::Dark, cx));
            assert!(active_theme(cx).is_dark);
            assert_eq!(theme_store(cx).entity_id(), store.entity_id());
        });
    }
}
