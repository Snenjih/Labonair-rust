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

/// Runtime typography overrides driven by the Terminal / Editor / Appearance
/// settings (T13-003). An empty family or a non-positive size means "keep the
/// theme's value". Applied on top of the built-in and imported themes so every
/// `ThemeStore` typography accessor reflects the user's choice live.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FontOverrides {
    pub app_family: String,
    pub app_size: f32,
    pub editor_family: String,
    pub editor_size: f32,
    pub terminal_family: String,
    pub terminal_size: f32,
    pub terminal_line_height: f32,
}

impl FontOverrides {
    fn apply(&self, theme: &mut Theme) {
        let ty = &mut theme.typography;
        if !self.app_family.is_empty() {
            ty.app_font_family = self.app_family.clone();
        }
        if self.app_size > 0.0 {
            ty.app_font_size = self.app_size;
        }
        if !self.editor_family.is_empty() {
            ty.buffer_font_family = self.editor_family.clone();
        }
        if self.editor_size > 0.0 {
            ty.buffer_font_size = self.editor_size;
        }
        if !self.terminal_family.is_empty() {
            ty.terminal_font_family = self.terminal_family.clone();
        }
        if self.terminal_size > 0.0 {
            ty.terminal_font_size = self.terminal_size;
        }
        if self.terminal_line_height > 0.0 {
            ty.terminal_line_height = self.terminal_line_height;
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
    /// An imported user theme, resolved for the current mode + font overrides.
    /// When set it overrides the default theme (T02-003).
    custom: Option<Theme>,
    /// `custom` before the runtime font overrides are applied — the source the
    /// overridden `custom` is rebuilt from when the overrides change (T13-003).
    custom_base: Option<Theme>,
    /// The source file for `custom`, kept so the imported theme can be
    /// re-resolved for the other mode when the appearance changes.
    custom_file: Option<ThemeFile>,
    /// The editor syntax-highlighting colour scheme (T06-002).
    editor_theme: EditorThemeId,
    /// Runtime typography overrides from settings (T13-003).
    font_overrides: FontOverrides,
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
            custom_base: None,
            custom_file: None,
            editor_theme: EditorThemeId::default(),
            font_overrides: FontOverrides::default(),
        }
    }

    /// The active runtime typography overrides.
    pub fn font_overrides(&self) -> &FontOverrides {
        &self.font_overrides
    }

    /// Replace the runtime typography overrides (settings → live). Rebuilds the
    /// built-in themes and re-resolves any imported theme so every typography
    /// accessor reflects the new values. Re-renders only on a real change.
    pub fn set_font_overrides(&mut self, overrides: FontOverrides, cx: &mut Context<Self>) {
        if self.font_overrides == overrides {
            return;
        }
        self.font_overrides = overrides;
        self.light = Theme::light();
        self.dark = Theme::dark();
        self.font_overrides.apply(&mut self.light);
        self.font_overrides.apply(&mut self.dark);
        self.reresolve_custom();
        cx.notify();
    }

    /// The active editor syntax colour scheme.
    pub fn editor_theme(&self) -> EditorThemeId {
        self.editor_theme
    }

    /// Sets the editor syntax colour scheme. Re-renders only if it changed.
    pub fn set_editor_theme(&mut self, id: EditorThemeId, cx: &mut Context<Self>) {
        if self.editor_theme == id {
            return;
        }
        self.editor_theme = id;
        cx.notify();
    }

    /// Re-resolves the imported theme (if any) against the current mode, then
    /// re-applies the runtime font overrides.
    fn reresolve_custom(&mut self) {
        if let Some(file) = self.custom_file.clone() {
            let dark = self.mode() == ThemeMode::Dark;
            if let Ok((theme, _warnings)) = Theme::from_theme_file(&file, dark) {
                self.custom_base = Some(theme);
            }
        }
        self.rebuild_custom();
    }

    /// Rebuilds `custom` from `custom_base` with the current font overrides.
    fn rebuild_custom(&mut self) {
        self.custom = self.custom_base.clone().map(|mut t| {
            self.font_overrides.apply(&mut t);
            t
        });
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
        if self.custom_base == theme && self.custom_file.is_none() {
            return;
        }
        self.custom_base = theme;
        self.custom_file = None;
        self.rebuild_custom();
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
        self.custom_base = Some(theme);
        self.custom_file = Some(file);
        self.rebuild_custom();
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
        self.custom_base = None;
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

    /// `--destructive` / status `error` color (severity: error).
    pub fn status_error(&self) -> Hsla {
        self.theme().status.error
    }

    /// `--warning` status color (severity: warning).
    pub fn status_warning(&self) -> Hsla {
        self.theme().status.warning
    }

    /// `--info` status color (severity: info).
    pub fn status_info(&self) -> Hsla {
        self.theme().status.info
    }

    /// `--modified` status color (changed / dirty markers, diff "replaced").
    pub fn status_modified(&self) -> Hsla {
        self.theme().status.modified
    }

    /// `--success` status color (severity: success).
    pub fn status_success(&self) -> Hsla {
        self.theme().status.success
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

    /// Canonical hover/focus fill for interactive rows, menu items and ghost
    /// buttons. The reference `dropdown-menu` / `context-menu` / `select`
    /// components all use `focus:bg-accent` (deferred visual item **D1** from
    /// the T15-001 catalog — was an ad-hoc `fg.opacity(0.04..0.05)` tint).
    pub fn hover_fill(&self) -> Hsla {
        self.accent()
    }

    /// Canonical selected/active fill for list selection (Explorer rows,
    /// command-palette results). The reference `cmdk` command items use
    /// `data-selected:bg-muted` (**D1**).
    pub fn selected_fill(&self) -> Hsla {
        self.muted()
    }

    /// Scrollbar thumb color for panels that keep a visible scrollbar
    /// (Explorer, SFTP, Settings, Snippets, AI pickers). The reference
    /// `.themed-scrollbar` thumb is `color-mix(in oklch, --foreground 22%,
    /// transparent)` — i.e. the foreground at 22% alpha — rising to 34% on
    /// hover (deferred visual item **D2**).
    pub fn scrollbar_thumb(&self) -> Hsla {
        self.foreground().opacity(0.22)
    }

    pub fn scrollbar_thumb_hover(&self) -> Hsla {
        self.foreground().opacity(0.34)
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

    /// UI font size in pixels (`preferencesStore.appFontSize`).
    pub fn ui_font_size(&self) -> f32 {
        self.theme().typography.app_font_size
    }

    /// Code-editor font size in pixels (`preferencesStore.editorFontSize`).
    pub fn buffer_font_size(&self) -> f32 {
        self.theme().typography.buffer_font_size
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

/// The backdrop fill for modal overlays (command palette, dialogs, sheets,
/// picker popovers). The reference paints every `DialogOverlay` /
/// `AlertDialogOverlay` / `SheetOverlay` with the same `bg-black/30`
/// (`src/components/ui/{dialog,alert-dialog,sheet}.tsx`), independent of the
/// light/dark theme — so this is a fixed value, not a theme token.
pub fn modal_scrim() -> Hsla {
    gpui::black().opacity(0.30)
}

/// Visible-scrollbar track/thumb thickness, in px. Matches the reference
/// `.themed-scrollbar::-webkit-scrollbar { width: 10px }` (deferred visual
/// item **D2** from the T15-001 catalog).
pub const SCROLLBAR_SIZE: f32 = 10.0;

/// Popover/menu padding density (deferred visual item **D5**). Values are the
/// reference Tailwind classes converted to px:
/// - container: `dropdown-menu` / `context-menu` `p-1.5`, `command` list `p-1`
/// - item: `px-3 py-2` with `gap-2.5` (menu) / `gap-2` (command)
/// - popover shell: `popover` `p-4` with `gap-4`
pub mod menu_metrics {
    /// `dropdown-menu` / `context-menu` container `p-1.5`.
    pub const CONTAINER_PAD: f32 = 6.0;
    /// `cmdk` command list container `p-1`.
    pub const COMMAND_CONTAINER_PAD: f32 = 4.0;
    /// Menu / command item `px-3`.
    pub const ITEM_PAD_X: f32 = 12.0;
    /// Menu / command item `py-2`.
    pub const ITEM_PAD_Y: f32 = 8.0;
    /// Menu item `gap-2.5`.
    pub const ITEM_GAP: f32 = 10.0;
    /// `popover` shell `p-4` / `gap-4`.
    pub const POPOVER_PAD: f32 = 16.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[test]
    fn modal_scrim_matches_reference_dialog_overlay() {
        // reference-src `dialog.tsx` / `alert-dialog.tsx` / `sheet.tsx` all use
        // `bg-black/30` — one shared value, theme-independent.
        let s = modal_scrim();
        assert_eq!((s.h, s.s, s.l), (0.0, 0.0, 0.0));
        assert!((s.a - 0.30).abs() < 1e-6);
    }

    #[test]
    fn polish_metrics_match_reference_css() {
        // D2 — `.themed-scrollbar::-webkit-scrollbar { width: 10px }`
        assert_eq!(SCROLLBAR_SIZE, 10.0);
        // D5 — menu/popover padding density.
        assert_eq!(menu_metrics::CONTAINER_PAD, 6.0); // p-1.5
        assert_eq!(menu_metrics::ITEM_PAD_X, 12.0); // px-3
        assert_eq!(menu_metrics::ITEM_PAD_Y, 8.0); // py-2
        assert_eq!(menu_metrics::POPOVER_PAD, 16.0); // p-4
    }

    #[gpui::test]
    fn polish_fills_derive_from_the_active_theme(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| ThemeStore::new(WindowAppearance::Dark));
            let s = store.read(cx);
            // D1 — hover = accent, selection = muted (1:1 with the reference
            // `focus:bg-accent` / `data-selected:bg-muted`).
            assert_eq!(s.hover_fill(), s.accent());
            assert_eq!(s.selected_fill(), s.muted());
            // D2 — thumb is the foreground at 22% → 34% alpha.
            assert!((s.scrollbar_thumb().a - 0.22).abs() < 1e-6);
            assert!((s.scrollbar_thumb_hover().a - 0.34).abs() < 1e-6);
            assert_eq!(s.scrollbar_thumb().h, s.foreground().h);
        });
    }

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
    fn font_overrides_apply_live_and_revert(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| ThemeStore::new(WindowAppearance::Dark));
            store.update(cx, |s, cx| {
                let base_term = s.terminal_font_size();
                s.set_font_overrides(
                    FontOverrides {
                        terminal_size: base_term + 7.0,
                        editor_size: 21.0,
                        terminal_family: "Iosevka".to_string(),
                        ..FontOverrides::default()
                    },
                    cx,
                );
                assert_eq!(s.terminal_font_size(), base_term + 7.0);
                assert_eq!(s.buffer_font_size(), 21.0);
                assert_eq!(s.terminal_font().family.as_ref(), "Iosevka");

                // An imported theme keeps the overrides on top.
                s.set_custom_theme(Some(Theme::light()), cx);
                assert_eq!(s.terminal_font_size(), base_term + 7.0);

                s.set_font_overrides(FontOverrides::default(), cx);
                assert_eq!(s.terminal_font_size(), base_term);
            });
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
    fn community_theme_partial_import_round_trips_visually(cx: &mut TestAppContext) {
        // D6 — a user-imported community theme that only overrides a handful of
        // tokens must (a) apply exactly those, (b) leave every other token on
        // the built-in default, and (c) survive an export→re-import cycle
        // pixel-identically.
        const COMMUNITY: &str = r##"{
            "name": "Community Neon",
            "variants": {
                "dark":  { "mode": "dark",  "colors": { "primary": "#39ff14", "accent": "#1b1b1b" } },
                "light": { "mode": "light", "colors": { "primary": "#0a7d00", "accent": "#eaeaea" } }
            }
        }"##;
        cx.update(|cx| {
            let store = cx.new(|_| ThemeStore::new(WindowAppearance::Dark));
            store.update(cx, |s, cx| {
                let file = ThemeFile::from_json(COMMUNITY).unwrap();
                s.import_theme_file(file, cx).unwrap();
                assert_eq!(labonair_theme::to_rgb8(s.primary()), [0x39, 0xff, 0x14]);
                // Untouched token still equals the dark default.
                assert_eq!(
                    labonair_theme::to_rgb8(s.background()),
                    labonair_theme::to_rgb8(Theme::dark().core.background)
                );

                // Export → re-import must not drift any visible channel.
                let before = (
                    labonair_theme::to_rgb8(s.primary()),
                    labonair_theme::to_rgb8(s.accent()),
                    labonair_theme::to_rgb8(s.background()),
                    labonair_theme::to_rgb8(s.foreground()),
                );
                let json = s.active_theme_file("Community Neon").to_json().unwrap();
                s.import_theme_file(ThemeFile::from_json(&json).unwrap(), cx)
                    .unwrap();
                let after = (
                    labonair_theme::to_rgb8(s.primary()),
                    labonair_theme::to_rgb8(s.accent()),
                    labonair_theme::to_rgb8(s.background()),
                    labonair_theme::to_rgb8(s.foreground()),
                );
                assert_eq!(before, after);
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
