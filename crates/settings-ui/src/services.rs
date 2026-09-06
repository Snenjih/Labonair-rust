//! Typed services consumed by the Settings UI.
//!
//! The Settings window is a value editor, not a backend integration hub.
//! These small contracts let the owning modules provide the few imperative
//! operations that a settings surface genuinely needs without exposing their
//! application state or storage types to this crate.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A service operation that can run away from the GPUI foreground thread.
pub type ServiceFuture<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'static>>;

/// System-font discovery contract used by the generic font-family field.
pub trait SystemFontService: Send + Sync {
    fn list(&self) -> ServiceFuture<Vec<String>>;
}

/// The imperative services injected into a Settings window by the
/// composition root. Settings UI owns neither implementation.
#[derive(Clone)]
pub struct SettingsServices {
    pub fonts: Arc<dyn SystemFontService>,
}

impl SettingsServices {
    pub fn new(fonts: Arc<dyn SystemFontService>) -> Self {
        Self { fonts }
    }
}
