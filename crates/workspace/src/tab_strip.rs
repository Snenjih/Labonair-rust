//! Tab strip rendering: the titlebar strip, the vertical sidebar list and the
//! per-tab block (R10-002).

use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, point, px, Animation, AnimationExt, AppContext, Context, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, SharedString, StatefulInteractiveElement, Styled,
};
use labonair_settings::{Settings as _, WorkspaceSettings};
use labonair_ui_kit::{
    caret, divider, icon_button_builder, tab_item, Axis, ButtonSize, IconButtonShape, IconName,
    Palette, TabLayout,
};

use crate::tabs::{Tab, TabDiagnosticStatus, TabGitStatus, TabKind};
use crate::{DraggedTab, TabDragPreview, Workspace, TITLEBAR_OFFSET};

impl Workspace {
    /// Render one tab row. `sidebar == true` lays the row out for the vertical
    /// Tabs sidebar panel (full width, click-anchored menus, top-edge drop
    /// indicator); `false` is the horizontal titlebar strip.
    pub(crate) fn render_tab(
        &self,
        tab: &Tab,
        sidebar: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let fg = theme.foreground();
        let c = Palette::from_theme(theme);
        let id = tab.id;
        let active = self.tabs.read(cx).active_id() == id;
        let label = SharedString::from(tab.label());
        let status_color = (tab.kind == TabKind::Editor
            && WorkspaceSettings::try_get(cx)
                .is_some_and(|settings| settings.show_editor_tab_git_status()))
        .then_some(tab.git_status)
        .flatten()
        .map(|status| match status {
            TabGitStatus::Added => c.success,
            TabGitStatus::Modified => c.warning,
            TabGitStatus::Deleted => c.error,
        });
        let diagnostic_color = (tab.kind == TabKind::Editor
            && WorkspaceSettings::try_get(cx)
                .is_some_and(|settings| settings.show_editor_tab_diagnostics()))
        .then_some(tab.diagnostic_status)
        .flatten()
        .map(|status| match status {
            TabDiagnosticStatus::Error => c.error,
            TabDiagnosticStatus::Warning => c.warning,
        });

        // D4 — tab-entrance animation (`@keyframes labonair-tab-in` in the
        // reference `globals.css`: fade + `scale(0.86) → 1` over `--dur-base`
        // with `--ease-premium`). GPUI 0.2.2 `Div` can't take a scale
        // transform, so we animate opacity only. Reduce-motion mirrors the
        // reference's `animation-duration: 0.01ms` clamp rather than dropping
        // the keyframe entirely.
        // T20-007: `reduce_motion` + timings now come from the metric-aware
        // `ActiveTheme` — `ThemeStore::animation()` already reports zero
        // durations when reduce-motion is on. GPUI's `with_animation` can
        // divide by a zero duration, so we still clamp to a 10µs floor here
        // (mirrors the reference `animation-duration: 0.01ms`).
        let (ease, dur_base, reduce_motion) = {
            let store = self.theme.read(cx);
            let a = store.animation();
            (
                a.ease_premium,
                a.dur_base,
                store.active_theme().reduce_motion(),
            )
        };
        let tab_in_dur = if reduce_motion || dur_base.is_zero() {
            Duration::from_micros(10)
        } else {
            dur_base
        };

        let label_content = match self.rename_tab.as_ref().filter(|(rid, _)| *rid == id) {
            Some((_, buf)) => div()
                .track_focus(&self.rename_focus)
                .key_context("TabRename")
                .on_key_down(cx.listener(Self::on_rename_key))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseDownEvent, window, cx| {
                        window.focus(&this.rename_focus);
                        cx.stop_propagation();
                    }),
                )
                .min_w(px(80.0))
                .max_w(px(180.0))
                .px_1()
                .rounded_sm()
                .border_1()
                .border_color(theme.primary())
                .flex()
                .items_center()
                .child(SharedString::from(buf.clone()))
                .when(self.rename_blink.read(cx).visible(), |d| {
                    d.child(caret(fg, 12.0))
                })
                .into_any_element(),
            None => div().child(label.clone()).into_any_element(),
        };

        let view = cx.entity();
        let mut tab_item = tab_item(format!("workspace-tab-{id}"), c, label.clone());
        if sidebar || tab.kind != TabKind::Editor {
            tab_item = tab_item.leading(tab.kind.indicator());
        }
        tab_item = tab_item
            .layout(if sidebar {
                TabLayout::Vertical
            } else {
                TabLayout::Horizontal
            })
            .selected(active)
            .dirty(tab.kind == TabKind::Editor && tab.dirty)
            .peek(tab.kind == TabKind::Editor && tab.peek)
            .pinned(tab.pinned)
            .status_color(status_color)
            .status_marker(diagnostic_color)
            .label_content(label_content)
            .on_activate({
                let view = view.clone();
                move |window, app| {
                    view.update(app, |this, cx| this.select_tab(id, window, cx));
                }
            })
            .on_navigate({
                let view = view.clone();
                move |forward, window, app| {
                    view.update(app, |this, cx| this.cycle_from_tab(id, forward, window, cx));
                }
            })
            .on_double_click({
                let view = view.clone();
                move |window, app| {
                    view.update(app, |this, cx| {
                        if this.tabs.read(cx).get(id).is_some_and(|tab| tab.peek) {
                            this.tabs
                                .update(cx, |store, cx| store.set_peek(id, false, cx));
                        } else {
                            this.begin_tab_rename(id, window, cx);
                        }
                    });
                }
            });
        if !tab.pinned {
            tab_item = tab_item.on_close(move |window, app| {
                view.update(app, |this, cx| this.request_close(id, window, cx));
            });
        }
        let row = tab_item
            .render()
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                    let y = if sidebar {
                        ev.position.y
                    } else {
                        px(TITLEBAR_OFFSET)
                    };
                    this.context_menu = Some((id, point(ev.position.x, y)));
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    if !this.tabs.read(cx).get(id).is_some_and(|tab| tab.pinned) {
                        this.request_close(id, window, cx);
                    }
                }),
            )
            .on_drag(
                DraggedTab {
                    id,
                    label: label.clone(),
                },
                |dragged, _, _, cx| {
                    cx.new(|_| TabDragPreview {
                        label: dragged.label.clone(),
                    })
                },
            )
            .drag_over::<DraggedTab>(move |style, _, _, _| {
                let style = if sidebar {
                    style.border_t_2()
                } else {
                    style.border_l_2()
                };
                style.border_color(fg)
            })
            .on_drop(cx.listener(move |this, dragged: &DraggedTab, _window, cx| {
                this.tabs.update(cx, |s, cx| s.reorder(dragged.id, id, cx));
            }))
            .with_animation(
                ("tab-in", id),
                Animation::new(tab_in_dur).with_easing(move |t| ease.eval(t)),
                |el, delta| el.opacity(delta),
            );
        row
    }

    /// The tab strip. Rendered inside `AppShell`'s single overlay titlebar
    /// (the header) between the left/right bar-item buckets — mirrors the
    /// reference `Header.tsx` where the `TabBar` lives in the titlebar.
    pub fn render_tab_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (muted, fg, border) = (theme.muted_foreground(), theme.foreground(), theme.border());
        let c = Palette::from_theme(theme);
        let active_space_id = self.spaces.read(cx).active_id();
        // Only the active Space's tabs show in the titlebar strip — every
        // other Space's tabs stay alive (T20-008) but out of sight until
        // switched to. The sidebar's `render_tab_list_vertical` intentionally
        // shows every Space's tabs at once, grouped into sections instead.
        let tabs: Vec<Tab> = self
            .tabs
            .read(cx)
            .tabs_in_space(active_space_id)
            .into_iter()
            .cloned()
            .collect();
        let pinned_tabs: Vec<Tab> = tabs.iter().filter(|tab| tab.pinned).cloned().collect();
        let unpinned_tabs: Vec<Tab> = tabs.iter().filter(|tab| !tab.pinned).cloned().collect();

        let active_space = self.spaces.read(cx).active().cloned();
        let (show_nav_history_buttons, show_tab_bar_buttons) = WorkspaceSettings::try_get(cx)
            .map(|settings| {
                (
                    settings.show_nav_history_buttons(),
                    settings.show_tab_bar_buttons(),
                )
            })
            .unwrap_or((true, true));
        let editor_history = self.active_editor_history_state(cx);
        let can_cycle_tabs = tabs.len() > 1;
        let (can_go_back, can_go_forward) =
            editor_history.unwrap_or((can_cycle_tabs, can_cycle_tabs));
        let history_buttons_active = editor_history.is_some();
        let workspace = cx.entity();
        let previous = icon_button_builder("tab-previous", c, IconName::ArrowLeft)
            .size(ButtonSize::IconLg)
            .shape(IconButtonShape::Square)
            .disabled(!can_go_back)
            .tooltip(if history_buttons_active {
                "Back"
            } else {
                "Previous Tab"
            })
            .render()
            .when(can_go_back, |button| {
                button.on_click({
                    let workspace = workspace.clone();
                    move |_, window, app| {
                        workspace.update(app, |this, cx| this.navigate_tab_bar(false, window, cx));
                    }
                })
            });
        let next = icon_button_builder("tab-next", c, IconName::ArrowRight)
            .size(ButtonSize::IconLg)
            .shape(IconButtonShape::Square)
            .disabled(!can_go_forward)
            .tooltip(if history_buttons_active {
                "Forward"
            } else {
                "Next Tab"
            })
            .render()
            .when(can_go_forward, |button| {
                button.on_click({
                    let workspace = workspace.clone();
                    move |_, window, app| {
                        workspace.update(app, |this, cx| this.navigate_tab_bar(true, window, cx));
                    }
                })
            });
        let new_tab = icon_button_builder("tab-new", c, IconName::PlusBold)
            .size(ButtonSize::IconSm)
            .shape(IconButtonShape::Square)
            .tooltip("New Tab")
            .render()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, _window, cx| {
                    this.new_tab_menu = Some(point(ev.position.x, px(TITLEBAR_OFFSET)));
                    this.new_tab_submenu = None;
                    this.context_menu = None;
                    cx.notify();
                }),
            );

        let spaces_trigger = div()
            .id("spaces-trigger")
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_1()
            .h(px(24.0))
            .px_1p5()
            .rounded_md()
            .cursor_pointer()
            .hover(|s| s.bg(border))
            .child(IconName::Box.svg(muted).size(px(13.0)))
            .when_some(active_space, |d, space| {
                d.child(
                    div()
                        .max_w(px(96.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_xs()
                        .text_color(fg)
                        .child(space.name),
                )
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, _window, cx| {
                    this.spaces_menu = Some(point(ev.position.x, px(TITLEBAR_OFFSET)));
                    this.new_tab_menu = None;
                    this.context_menu = None;
                    cx.notify();
                }),
            );

        div()
            .flex()
            .items_center()
            .gap_1()
            .h(px(32.0))
            .w_full()
            .flex_shrink_0()
            .bg(c.toolbar_bg)
            .when(show_nav_history_buttons, |bar| {
                bar.child(
                    div()
                        .id("tab-navigation")
                        .flex()
                        .items_center()
                        .h_full()
                        .border_r_1()
                        .border_color(border)
                        .child(previous)
                        .child(next),
                )
            })
            .when(show_tab_bar_buttons, |bar| bar.child(spaces_trigger))
            .child(
                div()
                    .id("tab-strip")
                    .flex()
                    .items_center()
                    .gap_0()
                    .min_w_0()
                    .h_full()
                    .overflow_x_scroll()
                    .children(pinned_tabs.iter().map(|t| self.render_tab(t, false, cx)))
                    .when(
                        !pinned_tabs.is_empty() && !unpinned_tabs.is_empty(),
                        |strip| strip.child(div().mx_1().w(px(1.0)).h(px(16.0)).bg(border)),
                    )
                    .children(unpinned_tabs.iter().map(|t| self.render_tab(t, false, cx))),
            )
            .when(show_tab_bar_buttons, |bar| bar.child(new_tab))
    }

    /// The vertical tab list shown in the Tabs sidebar panel when
    /// `tabsLocation == "sidebar"`. Same behaviour as [`Workspace::render_tab_bar`]
    /// (select / close / reorder / rename / context menu), stacked in a column
    /// with a "New Tab" footer. The new-tab / context menus themselves are
    /// rendered by [`Workspace::render`], anchored at the click position.
    pub fn render_tab_list_vertical(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let (muted, fg, border) = {
            let theme = self.theme.read(cx);
            (theme.muted_foreground(), theme.foreground(), theme.border())
        };
        let spaces = self.spaces.read(cx).spaces().to_vec();
        let tabs = self.tabs.read(cx).tabs().to_vec();

        // Grouped into a collapsible section per Space (T20-008) — the same
        // data and row component the Spaces popover uses, just expanded
        // inline instead of in a floating card.
        let mut groups: Vec<gpui::AnyElement> = Vec::with_capacity(spaces.len() * 2);
        let last_space_idx = spaces.len().saturating_sub(1);
        for (idx, space) in spaces.iter().enumerate() {
            let space_tabs: Vec<&Tab> = tabs.iter().filter(|t| t.space_id == space.id).collect();
            groups.push(
                self.render_space_header(space, space_tabs.len(), false, cx)
                    .into_any_element(),
            );
            if !self.collapsed_spaces.contains(&space.id) {
                groups.extend(
                    space_tabs
                        .into_iter()
                        .map(|t| self.render_tab(t, true, cx).into_any_element()),
                );
            }
            if idx != last_space_idx {
                groups.push(divider(Axis::Horizontal, border).my_1().into_any_element());
            }
        }
        // Recent Projects lives in the "New Space" dropdown (`sidebar-new-space`)
        // here, not inline — the titlebar Spaces popover still shows it inline
        // since it has no equivalent creation dropdown of its own.

        div()
            .flex()
            .flex_col()
            .min_h_0()
            .h_full()
            .w_full()
            .child(
                div()
                    .id("sidebar-tab-list")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .overflow_y_scroll()
                    .p_1p5()
                    .children(groups),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(border)
                    .p_1p5()
                    .flex()
                    .justify_end()
                    .child(
                        div()
                            .id("sidebar-new-space")
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .text_color(muted)
                            .cursor_pointer()
                            .hover(|s| s.bg(border).text_color(fg))
                            .child(IconName::FolderAdd.svg(muted).size(px(15.0)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                                    this.new_space_menu = Some(ev.position);
                                    this.new_space_search.clear();
                                    window.focus(&this.new_space_search_focus);
                                    cx.notify();
                                }),
                            ),
                    ),
            )
            .into_any_element()
    }
}
