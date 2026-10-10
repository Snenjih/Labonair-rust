//! Typed services consumed by the Settings UI.
//!
//! The Settings window is a value editor, not a backend integration hub.
//! These small contracts let the owning modules provide the few imperative
//! operations that a settings surface genuinely needs without exposing their
//! application state or storage types to this crate.

use gpui::{App, Window};
pub use labonair_settings::{
    SettingsSurface, SettingsSurfaceContribution, SettingsSurfaceId, SettingsSurfacePage,
    SettingsSurfaceRegistry,
};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

/// A service operation that can run away from the GPUI foreground thread.
pub type ServiceFuture<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'static>>;

/// System-font discovery contract used by the generic font-family field.
pub trait SystemFontService: Send + Sync {
    fn list(&self) -> ServiceFuture<Vec<String>>;
}

#[derive(Clone, Copy)]
pub enum SettingsFileTarget {
    User,
    Project,
}

type SettingsFileOpener = Rc<dyn Fn(SettingsFileTarget, &mut Window, &mut App)>;

/// The imperative services injected into a Settings window by the
/// composition root. Settings UI owns neither implementation.
#[derive(Clone)]
pub struct SettingsServices {
    pub fonts: Arc<dyn SystemFontService>,
    open_settings_file: SettingsFileOpener,
    surfaces: SettingsSurfaceRegistry,
}

impl SettingsServices {
    pub fn new(
        fonts: Arc<dyn SystemFontService>,
        open_settings_file: impl Fn(SettingsFileTarget, &mut Window, &mut App) + 'static,
        surfaces: SettingsSurfaceRegistry,
    ) -> Self {
        Self {
            fonts,
            open_settings_file: Rc::new(open_settings_file),
            surfaces,
        }
    }

    pub fn open_settings_file(
        &self,
        target: SettingsFileTarget,
        window: &mut Window,
        cx: &mut App,
    ) {
        (self.open_settings_file)(target, window, cx);
    }

    pub fn surfaces(&self) -> Vec<SettingsSurface> {
        self.surfaces.surfaces()
    }

    pub fn surface(&self, id: SettingsSurfaceId) -> Option<SettingsSurface> {
        self.surfaces.surface(id)
    }

    pub fn open_surface(&self, id: SettingsSurfaceId, cx: &mut App) -> Result<(), String> {
        self.surfaces.open(id, cx)
    }
}
