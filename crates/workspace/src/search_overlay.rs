//! [`SearchOverlay`] — the `Cmd+F` search overlay (T18-002).
//!
//! Replaces the titlebar's provisional inline search (T18-001) and the
//! editor's own in-buffer find bar with a single transient overlay hosted by
//! the [`crate::modal_layer::ModalLayer`]. It is a **bare** [`ModalView`]: the
//! layer moves keyboard focus into it but paints no scrim and does not
//! `occlude()` the rest of the window, so the active tab keeps scrolling while
//! the overlay is open.
//!
//! Routing is driven entirely by [`Workspace::active_search_target`] /
//! [`Workspace::search_set`] / [`Workspace::search_step`] /
//! [`Workspace::search_end`] — the overlay itself only owns the query input,
//! the case-sensitivity toggle and the last match count, and never touches
//! the editor / terminal search engines directly.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, ClickEvent, Context, DismissEvent, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Window,
};
use labonair_ui_kit::{field_input, text_field, InputEvent, InputState};
use std::sync::Mutex;

use crate::modal_layer::ModalView;
use crate::theme::ThemeStore;
use crate::{SearchTarget, Workspace};

/// The last query typed into any search overlay, pre-filled (and replacing an
/// editor-selection seed when none exists) the next time one opens.
static LAST_QUERY: Mutex<String> = Mutex::new(String::new());

fn last_query() -> String {
    LAST_QUERY.lock().map(|q| q.clone()).unwrap_or_default()
}

fn save_last_query(query: &str) {
    if let Ok(mut slot) = LAST_QUERY.lock() {
        *slot = query.to_string();
    }
}

pub struct SearchOverlay {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
    input: Entity<InputState>,
    target: SearchTarget,
    case_sensitive: bool,
    count: (usize, usize),
    _input_sub: Subscription,
}

impl SearchOverlay {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let target = workspace.read(cx).active_search_target(cx);
        let seed = workspace
            .read(cx)
            .search_seed(cx)
            .unwrap_or_else(last_query);

        let input = cx.new(|cx| {
            text_field(window, cx)
                .placeholder("Search\u{2026}")
                .default_value(seed.clone())
        });
        let input_sub = cx.subscribe(
            &input,
            |this: &mut Self, _input, ev: &InputEvent, cx| match ev {
                InputEvent::Change => this.run_search(cx),
                InputEvent::PressEnter { secondary } => this.step(!secondary, cx),
                _ => {}
            },
        );

        let mut this = Self {
            workspace,
            theme,
            input,
            target,
            case_sensitive: false,
            count: (0, 0),
            _input_sub: input_sub,
        };
        if !seed.is_empty() && target != SearchTarget::Unavailable {
            this.run_search(cx);
        }
        this
    }

    fn run_search(&mut self, cx: &mut Context<Self>) {
        let query = self.input.read(cx).value().to_string();
        save_last_query(&query);
        let case_sensitive = self.case_sensitive;
        self.count = self
            .workspace
            .update(cx, |w, cx| w.search_set(&query, case_sensitive, cx))
            .unwrap_or((0, 0));
        cx.notify();
    }

    fn step(&mut self, forward: bool, cx: &mut Context<Self>) {
        self.count = self
            .workspace
            .update(cx, |w, cx| w.search_step(forward, cx))
            .unwrap_or((0, 0));
        cx.notify();
    }

    fn toggle_case(&mut self, cx: &mut Context<Self>) {
        self.case_sensitive = !self.case_sensitive;
        self.run_search(cx);
    }
}

impl EventEmitter<DismissEvent> for SearchOverlay {}

impl Focusable for SearchOverlay {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input.read(cx).focus_handle(cx)
    }
}

impl ModalView for SearchOverlay {
    fn on_dismiss(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.search_end(cx));
    }

    /// No backdrop, no centering — a small box over the top-right of the
    /// active tab content that does not steal mouse input from it.
    fn render_bare(&self) -> bool {
        true
    }
}

impl Render for SearchOverlay {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (bg, fg, muted, border, accent) = (
            theme.card(),
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
            theme.accent(),
        );
        let unavailable = self.target == SearchTarget::Unavailable;
        let (cur, total) = self.count;
        let count_label = if total == 0 {
            "No results".to_string()
        } else {
            format!("{cur}/{total}")
        };

        // T20-003: a text-glyph mini-button ("Aa"/"↑"/"↓"/"✕") — `ui-kit`'s
        // `icon_toggle_button`/`button` primitives only accept an `IconName`
        // from the closed Lucide set, not an arbitrary glyph string, so
        // swapping in a real primitive would mean adding new icon assets for
        // a one-off search-bar affordance; documented exception.
        let icon_btn = |id: &'static str, glyph: &str, on: bool| {
            div()
                .id(id)
                .px_1()
                .rounded_sm()
                .text_xs()
                .text_color(if on { accent } else { muted })
                .hover(|s| s.bg(border).text_color(fg))
                .child(glyph.to_string())
        };

        div()
            .id("search-overlay")
            .absolute()
            .top(px(48.0))
            .right(px(16.0))
            .occlude()
            .flex()
            .items_center()
            .gap_1p5()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(bg)
            .border_1()
            .border_color(border)
            .when(unavailable, |d| {
                d.child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child("Search is not available for this tab"),
                )
            })
            .when(!unavailable, |d| {
                d.child(field_input(&self.input).w(px(220.0)))
                    .child(
                        icon_btn("search-case", "Aa", self.case_sensitive).on_click(
                            cx.listener(|this, _: &ClickEvent, _w, cx| this.toggle_case(cx)),
                        ),
                    )
                    .child(
                        div()
                            .min_w(px(44.0))
                            .text_xs()
                            .text_color(muted)
                            .child(count_label.clone()),
                    )
                    .child(
                        icon_btn("search-prev", "\u{2191}", false).on_click(
                            cx.listener(|this, _: &ClickEvent, _w, cx| this.step(false, cx)),
                        ),
                    )
                    .child(
                        icon_btn("search-next", "\u{2193}", false).on_click(
                            cx.listener(|this, _: &ClickEvent, _w, cx| this.step(true, cx)),
                        ),
                    )
            })
            .child(
                icon_btn("search-close", "\u{2715}", false)
                    .on_click(cx.listener(|_this, _: &ClickEvent, _w, cx| cx.emit(DismissEvent))),
            )
            .on_key_down(cx.listener(|_this, ev: &KeyDownEvent, _w, cx| {
                if ev.keystroke.key == "escape" {
                    cx.emit(DismissEvent);
                    cx.stop_propagation();
                }
            }))
    }
}
