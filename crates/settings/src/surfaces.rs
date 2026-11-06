//! Capability-owned links contributed to the Settings navigation.

use gpui::{AnyView, App, Window};
use std::rc::Rc;

/// Stable identity for a capability-owned surface linked from Settings.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SettingsSurfaceId(&'static str);

impl SettingsSurfaceId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Navigation index of the first owner page (Keymap, Hosts): the generated main
/// categories (General, Appearance, Themes, Editor, Languages & Tools,
/// Search & Files, Terminal) come first. `labonair-settings-ui`'s page tests pin
/// this to the number of generated pages.
pub const FIRST_OWNER_PAGE_INDEX: usize = 7;

/// How the Settings window presents an owner surface.
///
/// * [`Self::Link`] (default): the page shows the surface's title and
///   description with an action button that runs the owner's `open` callback
///   (the owner opens its own UI elsewhere).
/// * [`Self::Inline`]: the Settings window embeds the owner's view in the page
///   section, built on demand by the owner's inline builder
///   ([`SettingsSurfaceContribution::inline`]). The owner keeps all state and
///   persistence; Settings only hosts the view (ADR 0006). The `open` callback
///   stays valid as the fallback for hosts that cannot embed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SettingsSurfaceMode {
    #[default]
    Link,
    Inline,
}

/// Placement information contributed with an owner surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SettingsSurfacePage {
    pub slug: &'static str,
    pub title: &'static str,
    pub section: &'static str,
    /// Position in Settings navigation when this contribution creates a page.
    pub insert_at: Option<usize>,
}

impl SettingsSurfacePage {
    pub const fn section(slug: &'static str, title: &'static str, section: &'static str) -> Self {
        Self {
            slug,
            title,
            section,
            insert_at: None,
        }
    }

    pub const fn new_page(
        slug: &'static str,
        title: &'static str,
        section: &'static str,
        insert_at: usize,
    ) -> Self {
        Self {
            slug,
            title,
            section,
            insert_at: Some(insert_at),
        }
    }
}

/// Metadata and Settings-page placement supplied by a capability owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SettingsSurface {
    pub id: SettingsSurfaceId,
    pub title: &'static str,
    pub description: &'static str,
    pub action_label: &'static str,
    pub page: SettingsSurfacePage,
    pub mode: SettingsSurfaceMode,
    /// An inline surface that owns its whole page: the Settings window gives
    /// it the full content area instead of wrapping it in a section card, and
    /// the surface scrolls itself.
    pub full_page: bool,
}

impl SettingsSurface {
    pub const fn new(
        id: SettingsSurfaceId,
        title: &'static str,
        description: &'static str,
        action_label: &'static str,
        page: SettingsSurfacePage,
    ) -> Self {
        Self {
            id,
            title,
            description,
            action_label,
            page,
            mode: SettingsSurfaceMode::Link,
            full_page: false,
        }
    }

    /// Let an inline surface own its whole page (see [`Self::full_page`]).
    pub const fn full_page(mut self) -> Self {
        self.full_page = true;
        self
    }

    /// Mark the surface as embeddable inline (see [`SettingsSurfaceMode`]).
    pub const fn inline(mut self) -> Self {
        self.mode = SettingsSurfaceMode::Inline;
        self
    }
}

/// Builds the owner's view for an inline surface. Called when the Settings
/// window renders the section; the owner captures whatever entities it needs.
pub type InlineSurfaceBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyView>;

type SettingsSurfaceOpener = Rc<dyn Fn(&mut App)>;

/// One owner contribution: discoverable page metadata plus its canonical
/// operation. Settings stores no feature state or persistence for the surface.
#[derive(Clone)]
pub struct SettingsSurfaceContribution {
    pub surface: SettingsSurface,
    open: SettingsSurfaceOpener,
    inline: Option<InlineSurfaceBuilder>,
}

impl SettingsSurfaceContribution {
    pub fn new(surface: SettingsSurface, open: impl Fn(&mut App) + 'static) -> Self {
        Self {
            surface,
            open: Rc::new(open),
            inline: None,
        }
    }

    /// An inline-capable contribution: `build` produces the owner's view that
    /// the Settings window embeds, `open` remains the standalone entry point.
    pub fn inline(
        surface: SettingsSurface,
        open: impl Fn(&mut App) + 'static,
        build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        Self {
            surface: surface.inline(),
            open: Rc::new(open),
            inline: Some(Rc::new(build)),
        }
    }
}

/// Registry of owner surfaces contributed at application composition time.
#[derive(Clone, Default)]
pub struct SettingsSurfaceRegistry {
    contributions: Vec<SettingsSurfaceContribution>,
}

impl SettingsSurfaceRegistry {
    pub fn register(&mut self, contribution: SettingsSurfaceContribution) -> Result<(), String> {
        let surface = contribution.surface;
        if surface.id.as_str().trim().is_empty()
            || surface.title.trim().is_empty()
            || surface.description.trim().is_empty()
            || surface.action_label.trim().is_empty()
            || surface.page.slug.trim().is_empty()
            || surface.page.title.trim().is_empty()
            || surface.page.section.trim().is_empty()
        {
            return Err("Settings surface metadata must not be empty".to_owned());
        }
        if (surface.mode == SettingsSurfaceMode::Inline) != contribution.inline.is_some() {
            return Err(format!(
                "Settings surface `{}` must provide an inline builder exactly when it is inline",
                surface.id.as_str()
            ));
        }
        if self
            .contributions
            .iter()
            .any(|existing| existing.surface.id == surface.id)
        {
            return Err(format!(
                "duplicate Settings surface id `{}`",
                surface.id.as_str()
            ));
        }
        if self.contributions.iter().any(|existing| {
            existing.surface.page.slug == surface.page.slug
                && (existing.surface.page.title != surface.page.title
                    || existing.surface.page.insert_at != surface.page.insert_at)
        }) {
            return Err(format!(
                "Settings surface `{}` conflicts with page metadata for `{}`",
                surface.id.as_str(),
                surface.page.slug
            ));
        }
        self.contributions.push(contribution);
        Ok(())
    }

    pub fn surfaces(&self) -> Vec<SettingsSurface> {
        self.contributions
            .iter()
            .map(|contribution| contribution.surface)
            .collect()
    }

    pub fn surface(&self, id: SettingsSurfaceId) -> Option<SettingsSurface> {
        self.contributions
            .iter()
            .find(|contribution| contribution.surface.id == id)
            .map(|contribution| contribution.surface)
    }

    /// Build the embedded view of an inline surface, or `None` when the id is
    /// unknown or the surface is link-only.
    pub fn inline_view(
        &self,
        id: SettingsSurfaceId,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyView> {
        let build = self
            .contributions
            .iter()
            .find(|contribution| contribution.surface.id == id)?
            .inline
            .clone()?;
        Some(build(window, cx))
    }

    pub fn open(&self, id: SettingsSurfaceId, cx: &mut App) -> Result<(), String> {
        let contribution = self
            .contributions
            .iter()
            .find(|contribution| contribution.surface.id == id)
            .ok_or_else(|| format!("Settings surface `{}` is not registered", id.as_str()))?;
        (contribution.open)(cx);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contribution(id: &'static str) -> SettingsSurfaceContribution {
        SettingsSurfaceContribution::new(
            SettingsSurface::new(
                SettingsSurfaceId::new(id),
                "Test surface",
                "Test description",
                "Open…",
                SettingsSurfacePage::section("appearance", "Appearance", "Theme"),
            ),
            |_| {},
        )
    }

    #[test]
    fn inline_surfaces_require_a_builder_and_link_surfaces_must_not_have_one() {
        let surface = SettingsSurface::new(
            SettingsSurfaceId::new("inline"),
            "Inline surface",
            "Test description",
            "Open…",
            SettingsSurfacePage::section("themes", "Themes", "Background"),
        );
        // `inline()` without a builder is rejected.
        let mut registry = SettingsSurfaceRegistry::default();
        let missing_builder = SettingsSurfaceContribution::new(surface.inline(), |_| {});
        assert!(registry.register(missing_builder).is_err());

        // Plain link surfaces stay link-mode.
        assert_eq!(surface.mode, SettingsSurfaceMode::Link);
        registry
            .register(SettingsSurfaceContribution::new(surface, |_| {}))
            .unwrap();
        assert_eq!(registry.surfaces()[0].mode, SettingsSurfaceMode::Link);
    }

    #[gpui::test]
    fn inline_contribution_builds_its_view_on_demand(cx: &mut gpui::TestAppContext) {
        use gpui::{AppContext as _, IntoElement, Render};
        struct Probe;
        impl Render for Probe {
            fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
                gpui::Empty
            }
        }
        let surface = SettingsSurface::new(
            SettingsSurfaceId::new("inline"),
            "Inline surface",
            "Test description",
            "Open…",
            SettingsSurfacePage::section("themes", "Themes", "Background"),
        );
        let mut registry = SettingsSurfaceRegistry::default();
        registry
            .register(SettingsSurfaceContribution::inline(
                surface,
                |_| {},
                |_, cx| cx.new(|_| Probe).into(),
            ))
            .unwrap();
        assert_eq!(registry.surfaces()[0].mode, SettingsSurfaceMode::Inline);
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            assert!(registry
                .inline_view(SettingsSurfaceId::new("inline"), window, cx)
                .is_some());
            assert!(registry
                .inline_view(SettingsSurfaceId::new("other"), window, cx)
                .is_none());
        });
    }

    #[test]
    fn surface_registry_rejects_duplicate_ids() {
        let mut registry = SettingsSurfaceRegistry::default();
        registry.register(contribution("test")).unwrap();
        assert!(registry.register(contribution("test")).is_err());
        assert_eq!(registry.surfaces().len(), 1);
    }

    #[test]
    fn surface_registry_rejects_incomplete_page_metadata() {
        let mut registry = SettingsSurfaceRegistry::default();
        let contribution = SettingsSurfaceContribution::new(
            SettingsSurface::new(
                SettingsSurfaceId::new("missing-section"),
                "Test surface",
                "Test description",
                "Open…",
                SettingsSurfacePage::section("appearance", "Appearance", ""),
            ),
            |_| {},
        );
        assert!(registry.register(contribution).is_err());
        assert!(registry.surfaces().is_empty());
    }

    #[test]
    fn surface_registry_rejects_conflicting_page_metadata() {
        let mut registry = SettingsSurfaceRegistry::default();
        registry.register(contribution("first")).unwrap();
        let second = SettingsSurfaceContribution::new(
            SettingsSurface::new(
                SettingsSurfaceId::new("second"),
                "Second surface",
                "Second description",
                "Open…",
                SettingsSurfacePage::section("appearance", "Preferences", "Theme"),
            ),
            |_| {},
        );
        assert!(registry.register(second).is_err());
        assert_eq!(registry.surfaces().len(), 1);
    }
}
