//! [`Titlebar`] — the app's top chrome, redesigned in T18-001.
//!
//! The titlebar now carries **only**:
//!
//! * **left** — the tab strip (`workspace.render_tab_bar`), which already owns
//!   its trailing `＋` new-tab button + `NewTabDropdownItems` port (Terminal /
//!   Editor / Preview / Git Graph · SSH ▸ / SFTP ▸ recent hosts · All hosts…).
//!   The `＋` button is part of the tab strip, so it does not count as a
//!   "second button" against the layout contract (`docs/architecture.md` §4).
//! * **right** — exactly one [`IconName::Ellipsis`] button that opens a small
//!   dropdown (`Settings…`, `Profile`, room for more).
//!
//! Gone from the old header: the app title, the `⋯` app-menu, the inline search
//! box (its `⌘F` fallback is kept here as a **provisional floating overlay**
//! until T18-002 gives search its own overlay), every bar item / badge (they
//! are `StatusItem`s since T17-003) and the sidebar toggle (a status-bar
//! control since T18-003).
//!
//! Window chrome: the whole titlebar background is a drag region
//! (`WindowControlArea::Drag` + a `start_window_move` on drag), and a
//! double-click zooms the window (`titlebar_double_click`) — same mechanism as
//! Zed's `platform_title_bar`. Interactive children handle their own clicks;
//! only a bare press-then-drag on empty background moves the window.
//!
//! `zen_mode_show_header` still gates the whole thing — when off, the titlebar
//! renders nothing and the OS window frame / traffic lights take over.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
    WindowControlArea,
};
use labonair_notifications::{notification_center, Notification};
use labonair_settings_ui::{open_settings_window, PreferencesStore};
use labonair_ui_kit::IconName;

use crate::theme::ThemeStore;
use crate::workspace::Workspace;

const HEADER_H: f32 = 40.0;
/// Left inset reserved for the macOS traffic-light buttons. Linux has no
/// traffic lights, so the tab strip starts flush there.
#[cfg(target_os = "macos")]
const TRAFFIC_LIGHT_INSET: f32 = 78.0;
#[cfg(not(target_os = "macos"))]
const TRAFFIC_LIGHT_INSET: f32 = 8.0;

pub struct Titlebar {
    theme: Entity<ThemeStore>,
    prefs: Entity<PreferencesStore>,
    workspace: Entity<Workspace>,
    /// The right-hand `Settings… / Profile` dropdown.
    menu_open: bool,
    /// Drag-to-move latch: set on a background press, consumed on the first
    /// move (→ `start_window_move`), cleared on release.
    should_move: bool,
    /// Provisional `⌘F` search overlay state — moves to a dedicated overlay in
    /// T18-002.
    search_open: bool,
    search_query: String,
    search_focus: FocusHandle,
}

impl Titlebar {
    pub fn new(
        theme: Entity<ThemeStore>,
        prefs: Entity<PreferencesStore>,
        workspace: Entity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe(&prefs, |_, _, cx| cx.notify()).detach();
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        Self {
            theme,
            prefs,
            workspace,
            menu_open: false,
            should_move: false,
            search_open: false,
            search_query: String::new(),
            search_focus: cx.focus_handle(),
        }
    }

    /// `Cmd+F` fallback when the active pane is not an editor.
    ///
    /// PROVISIONAL (T18-002): this opens a floating overlay attached to the
    /// titlebar. Once the dedicated search overlay lands, `act_find` binds
    /// there and this method + the `search_*` fields go away.
    pub fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = true;
        window.focus(&self.search_focus);
        cx.notify();
    }

    fn close_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = false;
        self.search_query.clear();
        self.workspace.update(cx, |w, cx| w.search_active("", cx));
        self.workspace.update(cx, |w, cx| w.focus(window, cx));
        cx.notify();
    }

    fn run_search(&mut self, cx: &mut Context<Self>) {
        let query = self.search_query.clone();
        self.workspace
            .update(cx, |w, cx| w.search_active(&query, cx));
    }

    fn on_search_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = &ks.modifiers;
        if m.control || m.alt {
            return;
        }
        if m.platform {
            // Let Cmd-F / Cmd-W etc. bubble; don't type them.
            return;
        }
        match ks.key.as_str() {
            "escape" => self.close_search(window, cx),
            "enter" => self.run_search(cx),
            "backspace" => {
                self.search_query.pop();
                self.run_search(cx);
                cx.notify();
            }
            key => {
                let ch = ks
                    .key_char
                    .clone()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                    .or_else(|| (key.chars().count() == 1).then(|| key.to_string()));
                if let Some(ch) = ch {
                    self.search_query.push_str(&ch);
                    self.run_search(cx);
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }

    /// The single right-hand icon button + its dropdown.
    ///
    /// `Settings…` is functional; `Profile` is a deliberate placeholder — a
    /// future account / profile surface hangs off this entry. The separator +
    /// this doc-comment mark where further entries slot in.
    fn render_account_menu(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (fg, muted, border, card) = (
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
            theme.card(),
        );
        let open = self.menu_open;

        let item = |label: &str, key: SharedString| {
            div()
                .id(key)
                .px_2()
                .py_1()
                .text_xs()
                .rounded_sm()
                .text_color(fg)
                .hover(|s| s.bg(border))
                .child(SharedString::from(label.to_string()))
        };

        div()
            .relative()
            .flex_shrink_0()
            .child(
                div()
                    .id("account-menu")
                    .size(px(26.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child(IconName::Ellipsis.svg(muted))
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.menu_open = !this.menu_open;
                        cx.notify();
                    })),
            )
            .when(open, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(30.0))
                        .right(px(0.0))
                        .w(px(208.0))
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .p_1()
                        .rounded_md()
                        .bg(card)
                        .border_1()
                        .border_color(border)
                        .child(item("Settings\u{2026}", "acc-settings".into()).on_click(
                            cx.listener(|this, _: &ClickEvent, _window, cx| {
                                this.menu_open = false;
                                open_settings_window(None, cx);
                            }),
                        ))
                        .child(div().my_1().h(px(1.0)).bg(border))
                        // Placeholder — future account / profile features hang
                        // off this entry. Add further items below the divider.
                        .child(item("Profile", "acc-profile".into()).on_click(cx.listener(
                            |this, _: &ClickEvent, _window, cx| {
                                this.menu_open = false;
                                notification_center(cx).update(cx, |c, cx| {
                                    c.push(
                                        Notification::info(
                                            "Profile",
                                            "Account & profile features are coming soon.",
                                        ),
                                        cx,
                                    );
                                });
                            },
                        ))),
                )
            })
    }

    /// Provisional `⌘F` search box, floated just under the titlebar (T18-002
    /// replaces it with a real overlay).
    fn render_search(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (bg, fg, muted, ring) = (
            theme.background(),
            theme.foreground(),
            theme.muted_foreground(),
            theme.accent(),
        );
        let is_terminal = self.workspace.read(cx).active_is_terminal(cx);
        let placeholder = if is_terminal {
            "Search terminal\u{2026}"
        } else {
            "Search\u{2026}"
        };
        let (text, color) = if self.search_query.is_empty() {
            (placeholder.to_string(), muted)
        } else {
            (self.search_query.clone(), fg)
        };

        div()
            .absolute()
            .top(px(HEADER_H + 6.0))
            .right(px(12.0))
            .id("header-search")
            .track_focus(&self.search_focus)
            .key_context("HeaderSearch")
            .flex()
            .items_center()
            .h(px(24.0))
            .w(px(240.0))
            .px_2()
            .rounded_md()
            .bg(bg)
            .border_1()
            .border_color(ring)
            .text_xs()
            .text_color(color)
            .child(SharedString::from(text))
            .on_key_down(cx.listener(Self::on_search_key))
    }
}

impl Render for Titlebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.prefs.read(cx).get().zen_mode_show_header {
            // `zen_mode_show_header == false`: no custom titlebar — the OS
            // frame / traffic lights take over. On macOS the window still uses
            // `appears_transparent`, so the tabs simply move into the Tabs
            // sidebar panel via `tabs_location` if the user wants them.
            return div().into_any_element();
        }

        let (toolbar, border) = {
            let theme = self.theme.read(cx);
            (theme.toolbar(), theme.border())
        };

        // `tabsLocation === "sidebar"` moves the tab strip out of the titlebar
        // and into the Tabs sidebar panel.
        let tabs_in_sidebar = self.prefs.read(cx).get().tabs_location == "sidebar";
        let tabs = (!tabs_in_sidebar).then(|| {
            self.workspace
                .update(cx, |w, cx| w.render_tab_bar(cx).into_any_element())
        });

        div()
            .id("titlebar")
            .relative()
            .flex()
            .items_center()
            .gap_2()
            .h(px(HEADER_H))
            .w_full()
            .flex_shrink_0()
            .pl(px(TRAFFIC_LIGHT_INSET))
            .pr_2()
            .bg(toolbar)
            .border_b_1()
            .border_color(border)
            // Whole background is a window drag region; a bare press-then-drag
            // starts a window move (Zed `platform_title_bar` mechanism). A
            // double-click zooms. Interactive children consume their own
            // clicks, so this only fires on empty background.
            .window_control_area(WindowControlArea::Drag)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _w, cx| {
                    this.should_move = true;
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _w, _cx| {
                    this.should_move = false;
                }),
            )
            .on_mouse_move(cx.listener(|this, _: &MouseMoveEvent, window, _cx| {
                if this.should_move {
                    this.should_move = false;
                    window.start_window_move();
                }
            }))
            .on_click(cx.listener(|_, ev: &ClickEvent, window, _cx| {
                if ev.click_count() == 2 {
                    window.titlebar_double_click();
                }
            }))
            .child(div().flex_1().min_w_0().children(tabs))
            .child(self.render_account_menu(cx))
            .when(self.search_open, |d| d.child(self.render_search(cx)))
            .into_any_element()
    }
}

impl Focusable for Titlebar {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.search_focus.clone()
    }
}
