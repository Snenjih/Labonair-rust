//! Background presentation contract (B02 boundary).
//!
//! The Workspace terminal view (and the app-shell window chrome) paint a
//! background image layer, but must not own image storage, import/delete
//! behavior, decoding, or persistence — that stays entirely in
//! `labonair-background`. [`BackgroundHost`] is the narrow, read-only
//! presentation capability a view actually needs: render the current layer
//! for a [`LayerScope`], and repaint when the underlying image or settings
//! change.
//!
//! `labonair-background` builds a [`BackgroundHost`] from its concrete
//! `BackgroundStore` entity and injects it at composition; `labonair-workspace`
//! never depends on `labonair-background` directly.

use std::rc::Rc;

use gpui::{AnyElement, App, Entity};

/// Which surface is asking for a background layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerScope {
    /// The whole app window.
    App,
    /// The terminal surface only.
    Terminal,
}

/// Zero-sized entity that `labonair-background` notifies whenever its
/// rendered layer may have changed. Consumers `cx.observe` this instead of
/// the concrete background store, so they never need to name it.
pub struct BackgroundPulse;

type LayerFn = Rc<dyn Fn(&App, LayerScope) -> Option<AnyElement>>;

/// Narrow, read-only background presentation capability.
#[derive(Clone)]
pub struct BackgroundHost {
    pulse: Entity<BackgroundPulse>,
    layer: LayerFn,
}

impl BackgroundHost {
    /// Build a host from its repaint pulse and layer-rendering callback. The
    /// composition root (`labonair-background::host`) wires these to the
    /// active `BackgroundStore`.
    pub fn new(
        pulse: Entity<BackgroundPulse>,
        layer: impl Fn(&App, LayerScope) -> Option<AnyElement> + 'static,
    ) -> Self {
        Self {
            pulse,
            layer: Rc::new(layer),
        }
    }

    /// The entity to `cx.observe` for repaint.
    pub fn pulse(&self) -> &Entity<BackgroundPulse> {
        &self.pulse
    }

    /// The rendered overlay for `scope`, or `None` when there is nothing to
    /// show (no image selected, or `scope` isn't targeted).
    pub fn layer(&self, cx: &App, scope: LayerScope) -> Option<AnyElement> {
        (self.layer)(cx, scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, IntoElement};

    #[gpui::test]
    fn host_forwards_layer_lookups(cx: &mut gpui::TestAppContext) {
        let pulse = cx.update(|cx| cx.new(|_| BackgroundPulse));
        let host = BackgroundHost::new(pulse, |_, scope| match scope {
            LayerScope::Terminal => None,
            LayerScope::App => Some(gpui::Empty.into_any_element()),
        });

        cx.update(|cx| {
            assert!(host.layer(cx, LayerScope::Terminal).is_none());
            assert!(host.layer(cx, LayerScope::App).is_some());
        });
    }
}
