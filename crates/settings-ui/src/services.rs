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

/// Whether the detached session host (ADR 0009) is up, as the Settings status
/// block reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundHostStatus {
    NotRunning,
    Running,
    /// A host built for another protocol is running; `sessions` end when it is
    /// restarted.
    Incompatible {
        build_id: String,
        sessions: u32,
    },
}

/// One session living in the background host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundHostSession {
    pub id: u64,
    pub label: Option<String>,
    pub cwd: Option<String>,
    /// Foreground job when it is not the shell.
    pub foreground: Option<String>,
    pub attached: bool,
    pub exited: bool,
}

/// What the status block shows; one poll of the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundHostSnapshot {
    pub status: BackgroundHostStatus,
    pub sessions: Vec<BackgroundHostSession>,
}

/// The imperative side of the "Background host" status block. Every call may
/// block on the host's socket: implementations must not do that on the calling
/// thread (the futures run on the tokio runtime).
pub trait BackgroundHostService: Send + Sync {
    fn snapshot(&self) -> ServiceFuture<BackgroundHostSnapshot>;
    fn kill(&self, id: u64) -> ServiceFuture<()>;
    /// Kill every session and stop the host.
    fn stop_all(&self) -> ServiceFuture<()>;
    /// Stop an incompatible host (ending its sessions) and start a fresh one.
    fn restart(&self) -> ServiceFuture<()>;
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
    background_host: Option<Arc<dyn BackgroundHostService>>,
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
            background_host: None,
        }
    }

    /// Provide the background-host status block (unix composition roots only;
    /// without it the block is not shown).
    pub fn with_background_host(mut self, service: Arc<dyn BackgroundHostService>) -> Self {
        self.background_host = Some(service);
        self
    }

    pub fn background_host(&self) -> Option<Arc<dyn BackgroundHostService>> {
        self.background_host.clone()
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

    /// The embedded view of an inline-capable owner surface (see
    /// [`SettingsSurfaceMode::Inline`]); `None` for link-only or unknown ids.
    pub fn inline_surface(
        &self,
        id: SettingsSurfaceId,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<gpui::AnyView> {
        self.surfaces.inline_view(id, window, cx)
    }

    pub fn open_surface(&self, id: SettingsSurfaceId, cx: &mut App) -> Result<(), String> {
        self.surfaces.open(id, cx)
    }
}
