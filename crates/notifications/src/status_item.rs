//! Statusbar presentation for the notification registry.
//!
//! This belongs to the notifications capability rather than the application
//! shell. The shell only registers the item in the global statusbar registry.

use std::collections::HashSet;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, AnyElement, App, ClickEvent, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Pixels, Point, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use labonair_panel::{StatusItem, StatusSide};
use labonair_theme::store::ThemeStore;
use labonair_ui_kit::{icon_toggle_button, ButtonSize, ButtonVariant, IconName, Palette};

use crate::NotificationCenter;

/// The notifications status item and its statusbar dropdown.
pub struct NotificationsStatusItem {
    center: Entity<NotificationCenter>,
    theme: Entity<ThemeStore>,
    open: Option<Point<Pixels>>,
    expanded: HashSet<u64>,
    focus: FocusHandle,
}

impl NotificationsStatusItem {
    pub fn new(
        center: Entity<NotificationCenter>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&center, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            center,
            theme,
            open: None,
            expanded: HashSet::new(),
            focus: cx.focus_handle(),
        }
    }
}

impl Focusable for NotificationsStatusItem {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for NotificationsStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for NotificationsStatusItem {
    fn id(&self) -> &'static str {
        "notifications"
    }

    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }

    fn order(&self) -> i32 {
        100
    }

    fn group(&self) -> u32 {
        2
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let count = self.center.read(cx).unread_count();
        let (fg, muted, accent, border) = {
            let theme = self.theme.read(cx);
            (
                theme.foreground(),
                theme.muted_foreground(),
                theme.accent(),
                theme.border(),
            )
        };
        let palette = Palette::from_theme(self.theme.read(cx));
        let bell = icon_toggle_button(
            "bar-notifications",
            palette,
            IconName::Bell,
            self.open.is_some(),
        )
        .track_focus(&self.focus)
        .key_context("StatusPopover")
        .relative()
        .when(count > 0, |button| {
            button.child(
                div()
                    .absolute()
                    .top(px(-2.0))
                    .right(px(-2.0))
                    .min_w(px(13.0))
                    .h(px(13.0))
                    .px(px(2.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(accent)
                    .text_color(fg)
                    .text_size(px(8.0))
                    .child(SharedString::from(count.to_string())),
            )
        })
        .on_click(cx.listener(|this, ev: &ClickEvent, window, cx| {
            if this.open.is_some() {
                this.open = None;
            } else {
                this.open = Some(ev.position());
                window.focus(&this.focus);
            }
            cx.notify();
        }))
        .on_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, _window, cx| {
            if this.open.is_some() && ev.keystroke.key == "escape" {
                this.open = None;
                cx.notify();
                cx.stop_propagation();
            }
        }));

        let Some(anchor) = self.open else {
            return bell.into_any_element();
        };

        let snapshots = self.center.read(cx).snapshots();
        let view = cx.entity();
        let dismiss = {
            let view = view.clone();
            move |_window: &mut Window, cx: &mut App| {
                view.update(cx, |item, cx| {
                    item.open = None;
                    cx.notify();
                });
            }
        };
        let content = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(border)
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(fg)
                    .child("Notifications")
                    .child(
                        labonair_ui_kit::button_no_hover(
                            "bar-notif-clear",
                            palette,
                            ButtonVariant::Ghost,
                            ButtonSize::Xs,
                        )
                        .text_color(muted)
                        .hover(|style| style.text_color(fg))
                        .child("Clear all")
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _window, cx| {
                                this.center.update(cx, |center, cx| center.clear_all(cx));
                                this.open = None;
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(
                div()
                    .id("bar-notifications-list")
                    .max_h(px(360.0))
                    .overflow_y_scroll()
                    .children(snapshots.into_iter().map(|snapshot| {
                        let id = snapshot.id;
                        let expanded = self.expanded.contains(&id);
                        let row_view = view.clone();
                        let mut row = div()
                            .id(SharedString::from(format!("bar-notification-{id}")))
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .px_3()
                            .py_1p5()
                            .border_b_1()
                            .border_color(border)
                            .hover(|style| style.bg(border))
                            .on_click(move |_: &ClickEvent, _window, cx| {
                                row_view.update(cx, |item, cx| {
                                    if !item.expanded.insert(id) {
                                        item.expanded.remove(&id);
                                    }
                                    item.center
                                        .update(cx, |center, cx| center.mark_read(id, cx));
                                    cx.notify();
                                });
                            })
                            .child(div().text_xs().text_color(fg).child(snapshot.title.clone()))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(muted)
                                    .child(snapshot.body.clone()),
                            );
                        if expanded {
                            if let Some(details) = snapshot.details.clone() {
                                row = row.child(
                                    div()
                                        .pt_1()
                                        .text_size(px(11.0))
                                        .text_color(muted)
                                        .child(details),
                                );
                            }
                            if let Some(label) = snapshot.action_label.clone() {
                                let action_view = view.clone();
                                row = row.child(
                                    labonair_ui_kit::button_no_hover(
                                        SharedString::from(format!("bar-notification-action-{id}")),
                                        palette,
                                        ButtonVariant::Ghost,
                                        ButtonSize::Xs,
                                    )
                                    .text_color(accent)
                                    .child(label)
                                    .on_click(
                                        move |_: &ClickEvent, window, cx| {
                                            action_view.update(cx, |item, cx| {
                                                item.center.update(cx, |center, cx| {
                                                    center.trigger_action(id, window, cx);
                                                });
                                            });
                                            cx.stop_propagation();
                                        },
                                    ),
                                );
                            }
                        }
                        row
                    })),
            )
            .into_any_element();

        div()
            .relative()
            .flex_shrink_0()
            .child(bell)
            .child(labonair_ui_kit::popover(
                anchor,
                px(300.0),
                Palette::from_theme(self.theme.read(cx)),
                dismiss,
                content,
            ))
            .into_any_element()
    }
}
