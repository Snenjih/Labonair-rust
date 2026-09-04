//! [`ToastLayer`] — the workspace composition's second (and last) overlay
//! child (T17-005), alongside [`crate::modal_layer::ModalLayer`].
//!
//! Toasts are non-blocking and stacked. The stacking, per-severity
//! auto-dismiss (via `cx.background_executor().timer()`, never a thread sleep)
//! and manual close all already live in
//! [`labonair_notifications::NotificationCenter`]; this layer only observes it
//! and renders [`labonair_notifications::render_overlay`], so the shell's
//! `render` no longer has to.

use gpui::{Context, Entity, IntoElement, Render, Window};
use labonair_notifications::NotificationCenter;
use labonair_ui_kit::UiTheme;

/// Renders the stacked toast overlay from a [`NotificationCenter`].
pub struct ToastLayer<Th: UiTheme + 'static> {
    center: Entity<NotificationCenter>,
    theme: Entity<Th>,
}

impl<Th: UiTheme + 'static> ToastLayer<Th> {
    pub fn new(
        center: Entity<NotificationCenter>,
        theme: Entity<Th>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&center, |_, _, cx| cx.notify()).detach();
        Self { center, theme }
    }
}

impl<Th: UiTheme + 'static> Render for ToastLayer<Th> {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        labonair_notifications::render_overlay(&self.center, &self.theme, cx)
            .unwrap_or_else(|| gpui::div().into_any_element())
    }
}
