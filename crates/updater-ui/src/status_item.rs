//! Statusbar contribution for the updater UI.

use std::sync::Arc;

use gpui::{
    div, AnyElement, App, AppContext, ClickEvent, Context, Entity, IntoElement, Render,
    StatefulInteractiveElement, Window,
};
use labonair_panel::{AnyStatusItemHandle, StatusItem, StatusItemRegistration, StatusSide};
use labonair_theme::store::ThemeStore;
use labonair_ui_kit::{icon_button_builder, ButtonSize, IconName, IndicatorSize, Palette};

use crate::{UpdaterStatus, UpdaterView};

/// Statusbar badge exposing an available or active update.
pub struct UpdaterStatusItem {
    updater: Entity<UpdaterView>,
    theme: Entity<ThemeStore>,
}

impl UpdaterStatusItem {
    pub fn new(
        updater: Entity<UpdaterView>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&updater, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self { updater, theme }
    }
}

impl Render for UpdaterStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for UpdaterStatusItem {
    fn id(&self) -> &'static str {
        "updater"
    }

    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }

    fn order(&self) -> i32 {
        40
    }

    fn group(&self) -> u32 {
        1
    }

    fn is_empty(&self, cx: &App) -> bool {
        !matches!(
            self.updater.read(cx).status(),
            UpdaterStatus::Available(_) | UpdaterStatus::Downloading { .. } | UpdaterStatus::Ready
        )
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let ready = matches!(
            self.updater.read(cx).status(),
            UpdaterStatus::Available(_) | UpdaterStatus::Downloading { .. } | UpdaterStatus::Ready
        );
        if !ready {
            return div().into_any_element();
        }
        let palette = Palette::from_theme(self.theme.read(cx));
        icon_button_builder("bar-updater", palette, IconName::Download)
            .size(ButtonSize::IconXxs)
            .icon_color(palette.fg)
            .indicator(IndicatorSize::Xs, palette.primary)
            .tooltip("Update available")
            .render()
            .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                this.updater
                    .update(cx, |updater, cx| updater.open_dialog(cx));
            }))
            .into_any_element()
    }
}

/// Build the updater status-bar contribution.
pub fn registration(
    updater: &Entity<UpdaterView>,
    theme: &Entity<ThemeStore>,
    cx: &mut App,
) -> StatusItemRegistration {
    let item = cx.new(|cx| UpdaterStatusItem::new(updater.clone(), theme.clone(), cx));
    let handle = item.clone();
    StatusItemRegistration {
        id: item.read(cx).id(),
        default_side: item.read(cx).default_side(),
        order: item.read(cx).order(),
        group: item.read(cx).group(),
        build: Arc::new(move |_window, _cx| Arc::new(handle.clone()) as AnyStatusItemHandle),
    }
}
