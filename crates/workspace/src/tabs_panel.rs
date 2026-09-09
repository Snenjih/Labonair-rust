//! The Tabs sidebar panel.
//!
//! When `tabsLocation == "sidebar"` the workspace tab strip moves out of the
//! titlebar and into this dock panel. The panel is a thin wrapper: all tab
//! behaviour (select / close / reorder / rename / context + new-tab menus)
//! stays owned by [`Workspace`], which renders the vertical list through
//! [`Workspace::render_tab_list_vertical`]. Membership in a dock is driven by
//! [`Workspace::sync_tabs_in_sidebar`], not by this view.

use std::sync::Arc;

use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Pixels, Render, SharedString, Styled, Window,
};
use labonair_panel::{DockPosition, Panel, PanelIcon};

use crate::theme::ThemeStore;
use crate::Workspace;

/// Dock panel that hosts the workspace tab list vertically.
pub struct TabsPanel {
    workspace: Entity<Workspace>,
    focus_handle: FocusHandle,
    position: DockPosition,
}

impl TabsPanel {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            workspace,
            focus_handle: cx.focus_handle(),
            position: DockPosition::Left,
        }
    }
}

impl Focusable for TabsPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TabsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = self
            .workspace
            .update(cx, |workspace, cx| workspace.render_tab_list_vertical(cx));
        div()
            .track_focus(&self.focus_handle)
            .key_context("TabsPanel")
            .size_full()
            .child(body)
    }
}

impl Panel for TabsPanel {
    fn persistent_name() -> &'static str {
        "tabs"
    }

    fn title(&self, _cx: &App) -> SharedString {
        "Tabs".into()
    }

    fn icon(&self) -> PanelIcon {
        PanelIcon::Tabs
    }

    fn position(&self, _cx: &App) -> DockPosition {
        self.position
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(position, DockPosition::Left | DockPosition::Right)
    }

    fn set_position(
        &mut self,
        position: DockPosition,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.position = position;
        cx.notify();
    }

    fn default_size(&self, _cx: &App) -> Pixels {
        px(240.0)
    }

    fn min_size(&self) -> Option<Pixels> {
        Some(px(180.0))
    }
}

/// Build the Tabs panel contribution for the workspace-owned panel registry.
/// Wired by the shell composition root alongside the other panel owners.
pub fn tabs_panel_registration(
    view: &Entity<TabsPanel>,
    cx: &App,
) -> labonair_panel::PanelRegistration {
    use labonair_panel::{AnyPanelHandle, PanelRegistration};

    let handle = view.clone();
    PanelRegistration {
        persistent_name: TabsPanel::persistent_name(),
        default_position: view.read(cx).position(cx),
        icon: view.read(cx).icon(),
        build: Arc::new(move |_window, _cx| Arc::new(handle.clone()) as AnyPanelHandle),
    }
}
