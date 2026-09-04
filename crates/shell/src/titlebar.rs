//! [`Titlebar`] — the app's top chrome: the tab strip, a transient inline
//! search box and the `⋯` app-menu dropdown.
//!
//! Extracted verbatim from `AppShell::render_header` / `render_app_menu` /
//! `render_search` in T17-006 so the shell root is pure composition. The
//! titlebar owns its own `zen_mode_show_header` reactivity (it renders an empty
//! element when the header is hidden) and its own search state. The full
//! titlebar redesign (`＋▾` menu, origin-badge search, single `[◉ ▾]` button)
//! is T18-001 — this is the behavior-preserving intermediate.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, Window,
};
use labonair_settings_ui::{open_settings_window, PreferencesStore, SettingsTab};
use labonair_ui_kit::IconName;

use crate::menu;
use crate::theme::ThemeStore;
use crate::workspace::Workspace;

const HEADER_H: f32 = 40.0;
/// Left inset reserved for the macOS traffic-light buttons.
const TRAFFIC_LIGHT_INSET: f32 = 78.0;

pub struct Titlebar {
    theme: Entity<ThemeStore>,
    prefs: Entity<PreferencesStore>,
    workspace: Entity<Workspace>,
    app_menu_open: bool,
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
            app_menu_open: false,
            search_open: false,
            search_query: String::new(),
            search_focus: cx.focus_handle(),
        }
    }

    /// `Cmd+F` fallback when the active pane is not an editor.
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

    fn render_app_menu(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (fg, muted, border, card) = (
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
            theme.card(),
        );
        let open = self.app_menu_open;

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
                    .id("app-menu")
                    .size(px(26.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child(IconName::Menu.svg(muted))
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.app_menu_open = !this.app_menu_open;
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
                        .child(item("Settings", "am-settings".into()).on_click(cx.listener(
                            |this, _: &ClickEvent, _window, cx| {
                                this.app_menu_open = false;
                                open_settings_window(None, cx);
                            },
                        )))
                        .child(item("Keyboard Shortcuts", "am-shortcuts".into()).on_click(
                            cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.app_menu_open = false;
                                window.dispatch_action(Box::new(menu::OpenShortcuts), cx);
                            }),
                        ))
                        .child(
                            item("Themes\u{2026}", "am-themes".into()).on_click(cx.listener(
                                |this, _: &ClickEvent, _window, cx| {
                                    this.app_menu_open = false;
                                    open_settings_window(Some(SettingsTab::Themes), cx);
                                },
                            )),
                        ),
                )
            })
    }

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
            return div();
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

        // T17-003: the titlebar carries no bar items — every former badge /
        // toggle is a `StatusItem` in the status bar now.
        div()
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
            .child(div().flex_1().min_w_0().children(tabs))
            .when(self.search_open, |d| d.child(self.render_search(cx)))
            .child(self.render_app_menu(cx))
    }
}

impl Focusable for Titlebar {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.search_focus.clone()
    }
}
