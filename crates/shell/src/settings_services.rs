//! Composition adapters for the small imperative services used by Settings UI.
//!
//! Settings UI owns value rendering and persistence. The composition layer
//! supplies platform discovery services without exposing the backend facade.

use std::sync::Arc;

use labonair_settings_ui::{ServiceFuture, SettingsServices, SystemFontService};

struct BackendSystemFontService;

impl SystemFontService for BackendSystemFontService {
    fn list(&self) -> ServiceFuture<Vec<String>> {
        Box::pin(labonair_backend::modules::fonts::fonts_list_system())
    }
}

pub(crate) fn settings_services() -> SettingsServices {
    SettingsServices::new(Arc::new(BackendSystemFontService))
}
