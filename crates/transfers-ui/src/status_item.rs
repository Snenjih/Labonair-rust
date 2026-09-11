//! Status-bar contribution for the transfer queue.
//!
//! The transfer UI owns both the queue dropdown and its status-bar badge. The
//! shell only inserts the typed contribution into the application registry.

use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, AnyElement, App, AppContext, ClickEvent, Context, Entity, InteractiveElement,
    IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled, Window,
};
use labonair_panel::{AnyStatusItemHandle, StatusItem, StatusItemRegistration, StatusSide};
use labonair_theme::store::ThemeStore;
use labonair_ui_kit::{icon_toggle_button, IconName, Palette};

use crate::TransfersView;

fn simple_bar_button<T: 'static>(
    key: &'static str,
    icon: IconName,
    palette: Palette,
    cx: &mut Context<T>,
    on_click: impl Fn(&mut T, &mut Window, &mut Context<T>) + 'static,
) -> AnyElement {
    icon_toggle_button(key, palette, icon, false)
        .tab_index(0)
        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
            on_click(this, window, cx);
        }))
        .into_any_element()
}

/// Status-bar badge exposing the transfer queue and its dropdown.
pub struct TransfersStatusItem {
    transfers: Entity<TransfersView>,
    theme: Entity<ThemeStore>,
}

impl TransfersStatusItem {
    pub fn new(
        transfers: Entity<TransfersView>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&transfers, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self { transfers, theme }
    }
}

impl Render for TransfersStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for TransfersStatusItem {
    fn id(&self) -> &'static str {
        "transfers"
    }

    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }

    fn order(&self) -> i32 {
        20
    }

    fn group(&self) -> u32 {
        1
    }

    fn is_empty(&self, cx: &App) -> bool {
        self.transfers.read(cx).total_count() == 0
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        // Keep the item visible while retained history exists, so completed
        // and failed jobs remain discoverable from the statusbar.
        let (total, active, open) = {
            let transfers = self.transfers.read(cx);
            (
                transfers.total_count(),
                transfers.active_count(),
                transfers.is_open(),
            )
        };
        if total == 0 {
            return div().into_any_element();
        }
        let palette = Palette::from_theme(self.theme.read(cx));
        let button = simple_bar_button(
            "bar-transfers",
            IconName::ArrowDownUp,
            palette,
            cx,
            |this, _window, cx| {
                this.transfers
                    .update(cx, |transfers, cx| transfers.toggle(cx));
            },
        );
        div()
            .relative()
            .child(button)
            .when(active > 0, |element| {
                element.child(
                    div()
                        .absolute()
                        .top(px(-2.0))
                        .right(px(-2.0))
                        .min_w(px(8.0))
                        .h(px(8.0))
                        .rounded_full()
                        .bg(palette.accent),
                )
            })
            .when(open, |element| element.child(self.transfers.clone()))
            .into_any_element()
    }
}

/// Build the transfer status-bar contribution for application composition.
///
/// The `TransferUiEvent::Completed` signal that refreshes the SFTP pane is
/// wired to Workspace by the composition root, not by this item — see
/// `docs/registries.md`.
pub fn status_item_registration(
    transfers: &Entity<TransfersView>,
    theme: &Entity<ThemeStore>,
    cx: &mut App,
) -> StatusItemRegistration {
    let item = cx.new(|cx| TransfersStatusItem::new(transfers.clone(), theme.clone(), cx));
    let handle = item.clone();
    StatusItemRegistration {
        id: item.read(cx).id(),
        default_side: item.read(cx).default_side(),
        order: item.read(cx).order(),
        group: item.read(cx).group(),
        build: Arc::new(move |_window, _cx| Arc::new(handle.clone()) as AnyStatusItemHandle),
    }
}
