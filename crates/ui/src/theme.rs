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

use gpui::{App, AppContext, Context, Entity, Global, Hsla, WindowAppearance};
use labonair_theme::{Animation, RadiusScale, Shadows, Theme};

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
    /// An imported user theme. When set it overrides the default theme
    /// regardless of the resolved mode (T02-003).
    custom: Option<Theme>,
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
            cx.notify();
        }
    }

    /// Activates an imported theme (`Some`) or clears it (`None`), falling back
    /// to the default theme for the resolved mode.
    pub fn set_custom_theme(&mut self, theme: Option<Theme>, cx: &mut Context<Self>) {
        if self.custom == theme {
            return;
        }
        self.custom = theme;
        cx.notify();
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

    pub fn radius(&self) -> RadiusScale {
        self.theme().radius
    }

    pub fn shadows(&self) -> &Shadows {
        &self.theme().shadows
    }

    pub fn animation(&self) -> &Animation {
        &self.theme().animation
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
