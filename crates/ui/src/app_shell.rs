//! [`AppShell`] — the app's root coordinator (T04-003).
//!
//! Mirrors `reference-src/src/app/components/AppShell.tsx`: it is *only*
//! composition. It wires together, top to bottom:
//!
//! * a **header** bar (`bg-toolbar`, `h-10`) — sidebar toggle, app title,
//!   inline search, menu affordance;
//! * a **body** row — an optional, resizable left sidebar (a panel switcher
//!   rail + the active panel's content, both slots for later phases) next to
//!   the [`Workspace`] (tab bar + split-pane content, from T04-001/002);
//! * a **status bar** (`bg-status-bar`, `h-8`) — the active pane's cwd
//!   breadcrumb on the left, pane count + (empty) connection / AI slots on
//!   the right.
//!
//! No feature logic lives here — the header's inline search is forwarded to
//! [`Workspace::search_active`], the breadcrumb reads [`Workspace::active_cwd`],
//! etc. Window geometry is persisted via [`crate::window_state`] so the window
//! size/position survive a restart (full session persistence is T14-001).

use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, Bounds, ClickEvent, Context, DragMoveEvent, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Pixels, Render,
    SharedString, StatefulInteractiveElement, Styled, Window, WindowBounds,
};
use labonair_terminal::TerminalRegistry;

use crate::background::{BackgroundStore, LayerScope};
use crate::theme::ThemeStore;
use crate::window_state;
use crate::workspace::Workspace;

const HEADER_H: f32 = 40.0;
const STATUS_H: f32 = 32.0;
/// Left inset reserved for the macOS traffic-light buttons.
const TRAFFIC_LIGHT_INSET: f32 = 78.0;
/// Panel-switcher rail width.
const RAIL_W: f32 = 44.0;
const SIDEBAR_DEFAULT: f32 = 260.0;
const SIDEBAR_MIN: f32 = 180.0;
const SIDEBAR_MAX: f32 = 520.0;
/// Sidebar resize-handle thickness.
const HANDLE: f32 = 6.0;
/// Minimum interval between window-geometry writes.
const SAVE_THROTTLE: Duration = Duration::from_millis(1000);

/// A dockable sidebar panel. Later phases register their panel by adding a
/// variant here + an arm in [`AppShell::render_panel_body`]; the switcher rail
/// and toggle logic then pick it up automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPanel {
    Explorer,
    Snippets,
    SourceControl,
    GitGraph,
    Ai,
}

impl SidebarPanel {
    /// All panels, in rail order.
    const ALL: [SidebarPanel; 5] = [
        SidebarPanel::Explorer,
        SidebarPanel::Snippets,
        SidebarPanel::SourceControl,
        SidebarPanel::GitGraph,
        SidebarPanel::Ai,
    ];

    fn label(self) -> &'static str {
        match self {
            SidebarPanel::Explorer => "Explorer",
            SidebarPanel::Snippets => "Snippets",
            SidebarPanel::SourceControl => "Source Control",
            SidebarPanel::GitGraph => "Git Graph",
            SidebarPanel::Ai => "AI",
        }
    }

    /// A single glyph for the switcher rail (real icons arrive with the panels).
    fn glyph(self) -> &'static str {
        match self {
            SidebarPanel::Explorer => "\u{1F4C1}",
            SidebarPanel::Snippets => "\u{2702}",
            SidebarPanel::SourceControl => "\u{2325}",
            SidebarPanel::GitGraph => "\u{26D3}",
            SidebarPanel::Ai => "\u{2728}",
        }
    }
}

/// Value carried while dragging the sidebar's edge handle.
struct SidebarResize;

struct DragGhost;

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// The root view: window chrome around the [`Workspace`].
pub struct AppShell {
    theme: Entity<ThemeStore>,
    background: Entity<BackgroundStore>,
    workspace: Entity<Workspace>,
    sidebar_open: bool,
    sidebar_width: f32,
    active_panel: SidebarPanel,
    search_open: bool,
    search_query: String,
    search_focus: FocusHandle,
    focus_handle: FocusHandle,
    last_saved: Option<(Bounds<Pixels>, Instant)>,
}

impl AppShell {
    pub fn new(
        theme: Entity<ThemeStore>,
        background: Entity<BackgroundStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe(&background, |_, _, cx| cx.notify()).detach();

        let registry = Arc::new(TerminalRegistry::new());
        let workspace =
            cx.new(|cx| Workspace::new(registry, theme.clone(), background.clone(), window, cx));
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();

        // Persist the final window geometry on close (the throttled per-render
        // save covers force-quit within the last second).
        window.on_window_should_close(cx, |window, _cx| {
            if let WindowBounds::Windowed(bounds) = window.window_bounds() {
                window_state::save(bounds);
            }
            true
        });

        Self {
            theme,
            background,
            workspace,
            sidebar_open: true,
            sidebar_width: SIDEBAR_DEFAULT,
            active_panel: SidebarPanel::Explorer,
            search_open: false,
            search_query: String::new(),
            search_focus: cx.focus_handle(),
            focus_handle: cx.focus_handle(),
            last_saved: None,
        }
    }

    /// The workspace view (for later command-palette / menu wiring).
    pub fn workspace(&self) -> &Entity<Workspace> {
        &self.workspace
    }

    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        cx.notify();
    }

    fn select_panel(&mut self, panel: SidebarPanel, cx: &mut Context<Self>) {
        if self.active_panel == panel && self.sidebar_open {
            self.sidebar_open = false;
        } else {
            self.active_panel = panel;
            self.sidebar_open = true;
        }
        cx.notify();
    }

    fn set_sidebar_width(&mut self, width: f32, cx: &mut Context<Self>) {
        let clamped = (width - RAIL_W).clamp(SIDEBAR_MIN, SIDEBAR_MAX);
        if (clamped - self.sidebar_width).abs() > 0.5 {
            self.sidebar_width = clamped;
            cx.notify();
        }
    }

    fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn on_key_down(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = &ks.modifiers;
        if !m.platform || m.control || m.alt || m.shift {
            return;
        }
        match ks.key.as_str() {
            "b" => {
                self.toggle_sidebar(cx);
                cx.stop_propagation();
            }
            "f" => {
                self.open_search(window, cx);
                cx.stop_propagation();
            }
            _ => {}
        }
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

    /// Save the window geometry at most once per [`SAVE_THROTTLE`].
    fn maybe_persist_geometry(&mut self, window: &Window) {
        let WindowBounds::Windowed(bounds) = window.window_bounds() else {
            return;
        };
        let now = Instant::now();
        let stale = match self.last_saved {
            None => true,
            Some((last, at)) => {
                now.duration_since(at) >= SAVE_THROTTLE && bounds_differ(last, bounds)
            }
        };
        if stale {
            window_state::save(bounds);
            self.last_saved = Some((bounds, now));
        }
    }

    // ── Rendering ───────────────────────────────────────────────────────────

    fn render_header(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (toolbar, fg, muted, border) = (
            theme.toolbar(),
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
        );

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
            .child(
                div()
                    .id("sidebar-toggle")
                    .size(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child("\u{2630}")
                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                        this.toggle_sidebar(cx);
                    })),
            )
            .child(div().text_xs().text_color(fg).child("Labonair"))
            .child(div().flex_1())
            .when(self.search_open, |d| d.child(self.render_search(cx)))
            .child(
                div()
                    .id("app-menu")
                    .size(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child("\u{22EF}"),
            )
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

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (sidebar_bg, sidebar_fg, sidebar_border, accent, muted) = (
            theme.sidebar_bg(),
            theme.sidebar_fg(),
            theme.sidebar_border(),
            theme.accent(),
            theme.muted_foreground(),
        );
        let active = self.active_panel;

        let rail = div()
            .w(px(RAIL_W))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .py_2()
            .border_r_1()
            .border_color(sidebar_border)
            .children(SidebarPanel::ALL.into_iter().map(|panel| {
                let is_active = panel == active;
                div()
                    .id(SharedString::from(panel.label()))
                    .size(px(30.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(if is_active { sidebar_fg } else { muted })
                    .when(is_active, |d| d.bg(accent))
                    .when(!is_active, |d| d.hover(|s| s.bg(sidebar_border)))
                    .child(panel.glyph())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.select_panel(panel, cx);
                    }))
            }));

        let panel = div()
            .w(px(self.sidebar_width))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(sidebar_bg)
            .text_color(sidebar_fg)
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_xs()
                    .text_color(muted)
                    .child(SharedString::from(active.label().to_uppercase())),
            )
            .child(self.render_panel_body(active, cx));

        let handle = div()
            .id("sidebar-handle")
            .w(px(HANDLE))
            .h_full()
            .flex_shrink_0()
            .cursor_col_resize()
            .bg(sidebar_border)
            .hover(|s| s.bg(accent))
            .on_drag(SidebarResize, |_, _, _, cx| cx.new(|_| DragGhost));

        div()
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_row()
            .child(rail)
            .child(panel)
            .child(handle)
    }

    /// Placeholder body for each sidebar panel. Later phases replace the arm
    /// for their panel with the real view.
    fn render_panel_body(&self, panel: SidebarPanel, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = self.theme.read(cx).muted_foreground();
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .px_3()
            .text_center()
            .text_xs()
            .text_color(muted)
            .child(SharedString::from(format!(
                "{} \u{2014} coming in a later phase",
                panel.label()
            )))
    }

    fn render_statusbar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = self.workspace.read(cx);
        let cwd = workspace.active_cwd(cx);
        let panes = workspace.active_pane_count(cx);
        let label = workspace.active_tab_label(cx);

        let theme = self.theme.read(cx);
        let (status_bar, fg, muted, border) = (
            theme.status_bar(),
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
        );

        let breadcrumb = match cwd.as_deref().map(display_path) {
            Some(path) => {
                let segments: Vec<SharedString> = path
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .map(|s| SharedString::from(s.to_string()))
                    .collect();
                let last = segments.len().saturating_sub(1);
                div()
                    .flex()
                    .items_center()
                    .min_w_0()
                    .when(path.starts_with('/'), |d| {
                        d.child(div().text_color(muted).child("/"))
                    })
                    .children(segments.into_iter().enumerate().map(|(i, seg)| {
                        div()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .text_color(if i == last { fg } else { muted })
                                    .child(seg),
                            )
                            .when(i != last, |d| {
                                d.child(div().px_0p5().text_color(muted).child("/"))
                            })
                    }))
                    .into_any_element()
            }
            None => div()
                .text_color(muted)
                .child(SharedString::from(label))
                .into_any_element(),
        };

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .h(px(STATUS_H))
            .w_full()
            .flex_shrink_0()
            .px_3()
            .bg(status_bar)
            .border_t_1()
            .border_color(border)
            .text_color(muted)
            .child(
                div()
                    .flex()
                    .min_w_0()
                    .flex_1()
                    .items_center()
                    .gap_1()
                    .overflow_hidden()
                    .child(breadcrumb),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_2()
                    // Connection / jump-host / AI badge slots stay empty until
                    // their phases (06 / 10) provide data.
                    .when(panes > 1, |d| {
                        d.child(SharedString::from(format!("{panes} panes")))
                    }),
            )
    }
}

impl Focusable for AppShell {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.maybe_persist_geometry(window);

        let bg = self.theme.read(cx).background();
        let background_layer = self.background.read(cx).layer(LayerScope::App);
        let header = self.render_header(cx);
        let sidebar = self
            .sidebar_open
            .then(|| self.render_sidebar(cx).into_any_element());
        let statusbar = self.render_statusbar(cx);
        let workspace = self.workspace.clone();

        div()
            .track_focus(&self.focus_handle)
            .key_context("AppShell")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg)
            .text_xs()
            .on_key_down(cx.listener(Self::on_key_down))
            .child(header)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_row()
                    .children(sidebar)
                    .child(div().flex_1().min_w_0().child(workspace))
                    .on_drag_move(cx.listener(
                        |this, ev: &DragMoveEvent<SidebarResize>, _window, cx| {
                            let w = f32::from(ev.event.position.x - ev.bounds.origin.x);
                            this.set_sidebar_width(w, cx);
                        },
                    )),
            )
            .child(statusbar)
            .children(background_layer)
    }
}

/// Substitute the user's home directory with `~` for display.
fn display_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir().and_then(|p| p.to_str().map(str::to_string)) {
        if path == home {
            return "~".to_string();
        }
        if let Some(rest) = path.strip_prefix(&format!("{home}/")) {
            return format!("~/{rest}");
        }
    }
    path.to_string()
}

fn bounds_differ(a: Bounds<Pixels>, b: Bounds<Pixels>) -> bool {
    let d = |x: Pixels, y: Pixels| (f32::from(x) - f32::from(y)).abs() > 2.0;
    d(a.origin.x, b.origin.x)
        || d(a.origin.y, b.origin.y)
        || d(a.size.width, b.size.width)
        || d(a.size.height, b.size.height)
}
