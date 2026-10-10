//! Capability-owned links contributed to the Settings navigation.

use gpui::App;
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
        }
    }
}

type SettingsSurfaceOpener = Rc<dyn Fn(&mut App)>;

/// One owner contribution: discoverable page metadata plus its canonical
/// operation. Settings stores no feature state or persistence for the surface.
#[derive(Clone)]
pub struct SettingsSurfaceContribution {
    pub surface: SettingsSurface,
    open: SettingsSurfaceOpener,
}

impl SettingsSurfaceContribution {
    pub fn new(surface: SettingsSurface, open: impl Fn(&mut App) + 'static) -> Self {
        Self {
            surface,
            open: Rc::new(open),
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
