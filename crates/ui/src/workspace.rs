//! Workspace shell: tab bar + tab content area (T04-001).
//!
//! [`Workspace`] owns the [`TabStore`], the shared [`TerminalRegistry`] and the
//! per-tab content views. It renders the tab bar (open / close / switch /
//! drag-reorder / context actions) and, below it, the content view for the
//! active tab — a live [`TerminalView`] for `Workspace` tabs, a placeholder for
//! kinds whose content arrives in later phases.
//!
//! Session lifecycle: a new `Workspace` tab spawns a PTY session in the
//! registry (inheriting the previous tab's cwd); closing the tab calls
//! [`TerminalRegistry::close`] so no shell process is orphaned. Switching tabs
//! never touches the registry, so background terminals keep running.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, ClickEvent, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement,
    Render, SharedString, StatefulInteractiveElement, Styled, Task, Window,
};
use labonair_terminal::{SessionOptions, TermDimensions, TerminalColors, TerminalRegistry};

use crate::background::BackgroundStore;
use crate::tabs::{Tab, TabKind, TabStore};
use crate::terminal::TerminalView;
use crate::theme::ThemeStore;

/// Interval for syncing terminal cwd/title into their tab labels.
const META_SYNC_INTERVAL: Duration = Duration::from_millis(400);

/// Value carried by a tab drag.
struct DraggedTab {
    id: u64,
    label: SharedString,
}

/// Drag image shown while a tab is being reordered.
struct TabDragPreview {
    label: SharedString,
}

impl Render for TabDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .text_xs()
            .rounded_md()
            .bg(gpui::rgba(0x00000099))
            .text_color(gpui::white())
            .child(self.label.clone())
    }
}

/// The tabbed workspace shell.
pub struct Workspace {
    registry: Arc<TerminalRegistry>,
    tabs: Entity<TabStore>,
    theme: Entity<ThemeStore>,
    background: Entity<BackgroundStore>,
    /// Content view per `Workspace` tab id — kept alive across tab switches so
    /// scrollback position and focus survive.
    terminals: HashMap<u64, Entity<TerminalView>>,
    /// Tab id whose close is awaiting unsaved-changes confirmation.
    confirm_close: Option<u64>,
    /// Open tab context menu: `(tab id, anchor position)`.
    context_menu: Option<(u64, gpui::Point<gpui::Pixels>)>,
    focus_handle: FocusHandle,
    _meta_sync: Task<()>,
}

impl Workspace {
    pub fn new(
        registry: Arc<TerminalRegistry>,
        theme: Entity<ThemeStore>,
        background: Entity<BackgroundStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let tabs = cx.new(|_| TabStore::new());
        cx.observe(&tabs, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe(&background, |_, _, cx| cx.notify()).detach();

        let meta_sync = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(META_SYNC_INTERVAL).await;
            if this.update(cx, |this, cx| this.sync_meta(cx)).is_err() {
                break;
            }
        });

        let mut this = Self {
            registry,
            tabs,
            theme,
            background,
            terminals: HashMap::new(),
            confirm_close: None,
            context_menu: None,
            focus_handle: cx.focus_handle(),
            _meta_sync: meta_sync,
        };
        // First tab.
        this.open_terminal_tab(window, cx);
        this
    }

    /// The tab store (for later phases / command palette wiring).
    pub fn tab_store(&self) -> &Entity<TabStore> {
        &self.tabs
    }

    fn theme_colors(&self, cx: &App) -> TerminalColors {
        TerminalColors::from_theme(self.theme.read(cx).theme())
    }

    /// Spawn a new local terminal session and open a workspace tab for it.
    fn open_terminal_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cwd = self.active_terminal(cx).and_then(|v| v.read(cx).cwd());
        let colors = self.theme_colors(cx);
        let options = SessionOptions {
            working_directory: cwd.clone(),
            ..SessionOptions::default()
        };
        let session_id = match self
            .registry
            .create(colors, TermDimensions::new(80, 24), options)
        {
            Ok(id) => id,
            Err(err) => {
                tracing::error!(%err, "failed to spawn terminal session for new tab");
                return;
            }
        };
        let Some(handle) = self.registry.handle(session_id) else {
            return;
        };
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open_workspace(session_id, cwd, cx));
        let theme = self.theme.clone();
        let background = self.background.clone();
        let view = cx.new(|cx| TerminalView::new(handle, theme, background, window, cx));
        self.terminals.insert(tab_id, view);
    }

    fn active_terminal(&self, cx: &App) -> Option<Entity<TerminalView>> {
        let id = self.tabs.read(cx).active_id();
        self.terminals.get(&id).cloned()
    }

    fn select_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.update(cx, |s, cx| s.set_active(id, cx));
        self.focus_active(window, cx);
    }

    fn focus_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(view) = self.active_terminal(cx) {
            view.read(cx).focus(window);
        } else {
            window.focus(&self.focus_handle);
        }
    }

    fn cycle_tab(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.update(cx, |s, cx| s.cycle(forward, cx));
        self.focus_active(window, cx);
    }

    /// Request closing a tab. Editor tabs with unsaved changes first ask for
    /// confirmation; everything else (and terminals) closes immediately, with
    /// the session torn down.
    fn request_close(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let needs_confirm = self
            .tabs
            .read(cx)
            .get(id)
            .map(Tab::needs_close_confirm)
            .unwrap_or(false);
        if needs_confirm && self.confirm_close != Some(id) {
            self.confirm_close = Some(id);
            cx.notify();
            return;
        }
        self.do_close(id, window, cx);
    }

    fn do_close(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.confirm_close == Some(id) {
            self.confirm_close = None;
        }
        if let Some(removed) = self.tabs.update(cx, |s, cx| s.close(id, cx)) {
            self.retire_tab(&removed);
        }
        self.focus_active(window, cx);
    }

    fn close_others(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let removed = self.tabs.update(cx, |s, cx| s.close_others(id, cx));
        for tab in &removed {
            self.retire_tab(tab);
        }
        self.confirm_close = None;
        self.focus_active(window, cx);
    }

    fn close_by_kind(&mut self, kind: TabKind, window: &mut Window, cx: &mut Context<Self>) {
        let removed = self.tabs.update(cx, |s, cx| s.close_by_kind(kind, cx));
        for tab in &removed {
            self.retire_tab(tab);
        }
        self.confirm_close = None;
        self.focus_active(window, cx);
    }

    /// Tear down a removed tab's backing resources.
    fn retire_tab(&mut self, tab: &Tab) {
        if let Some(session_id) = tab.data.session_id {
            self.registry.close(session_id);
        }
        self.terminals.remove(&tab.id);
    }

    fn sync_meta(&mut self, cx: &mut Context<Self>) {
        let updates: Vec<(u64, Option<String>, Option<String>)> = self
            .terminals
            .iter()
            .map(|(id, view)| {
                let v = view.read(cx);
                (*id, v.cwd(), v.shell_title())
            })
            .collect();
        self.tabs.update(cx, |store, cx| {
            for (id, cwd, title) in updates {
                store.sync_workspace_meta(id, cwd, title, cx);
            }
        });
    }

    // ── Rendering ───────────────────────────────────────────────────────────

    fn render_tab(&self, tab: &Tab, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (fg, muted, accent, border) = (
            theme.foreground(),
            theme.muted_foreground(),
            theme.accent(),
            theme.border(),
        );
        let id = tab.id;
        let active = self.tabs.read(cx).active_id() == id;
        let total = self.tabs.read(cx).len();
        let closable = total > 1 && tab.kind != TabKind::Home;
        let label = SharedString::from(tab.label());

        let close_btn = div()
            .id(("tab-close", id))
            .px_1()
            .rounded_sm()
            .text_color(muted)
            .hover(|s| s.bg(border).text_color(fg))
            .child("\u{2715}")
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                this.request_close(id, window, cx);
            }));

        div()
            .id(("tab", id))
            .flex()
            .items_center()
            .gap_1p5()
            .h(px(28.0))
            .px_2()
            .rounded_md()
            .text_xs()
            .whitespace_nowrap()
            .cursor_pointer()
            .text_color(if active { fg } else { muted })
            .when(active, |d| d.bg(accent))
            .when(!active, |d| d.hover(|s| s.bg(border)))
            .child(div().text_color(muted).child(tab.kind.indicator()))
            .child(
                div()
                    .max_w(px(180.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .when(tab.kind == TabKind::Editor && tab.peek, |d| d.italic())
                    .child(label.clone()),
            )
            .when(tab.kind == TabKind::Editor && tab.dirty, |d| {
                d.child(div().size(px(6.0)).rounded_full().bg(fg).opacity(0.7))
            })
            .when(closable, |d| d.child(close_btn))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.select_tab(id, window, cx);
            }))
            .when(closable, |d| {
                d.on_mouse_down(
                    MouseButton::Middle,
                    cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                        this.request_close(id, window, cx);
                    }),
                )
            })
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                    this.context_menu = Some((id, ev.position));
                    cx.notify();
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
            .drag_over::<DraggedTab>(move |style, _, _, _| style.border_l_2().border_color(fg))
            .on_drop(cx.listener(move |this, dragged: &DraggedTab, _window, cx| {
                this.tabs.update(cx, |s, cx| s.reorder(dragged.id, id, cx));
            }))
    }

    fn render_tab_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (bg, muted, fg, border) = (
            theme.background(),
            theme.muted_foreground(),
            theme.foreground(),
            theme.border(),
        );
        let tabs = self.tabs.read(cx).tabs().to_vec();

        div()
            .flex()
            .items_center()
            .gap_1()
            .h(px(36.0))
            .w_full()
            .flex_shrink_0()
            .px_2()
            .bg(bg)
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .id("tab-strip")
                    .flex()
                    .items_center()
                    .gap_0p5()
                    .min_w_0()
                    .overflow_x_scroll()
                    .children(tabs.iter().map(|t| self.render_tab(t, cx))),
            )
            .child(
                div()
                    .id("tab-new")
                    .flex_shrink_0()
                    .size(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child("+")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.open_terminal_tab(window, cx);
                        this.focus_active(window, cx);
                    })),
            )
    }

    fn render_content(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let active = self.tabs.read(cx).active().cloned();
        let Some(active) = active else {
            return div().size_full().into_any_element();
        };

        match active.kind {
            TabKind::Workspace => {
                if let Some(view) = self.terminals.get(&active.id) {
                    div().size_full().child(view.clone()).into_any_element()
                } else {
                    self.placeholder("Terminal", cx).into_any_element()
                }
            }
            other => self
                .placeholder(other.default_title(), cx)
                .into_any_element(),
        }
    }

    fn placeholder(&self, title: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.background())
            .text_color(theme.muted_foreground())
            .text_sm()
            .child(SharedString::from(format!(
                "{title} — coming in a later phase"
            )))
    }

    fn render_confirm(&mut self, id: u64, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (card, fg, border, accent, muted) = (
            theme.card(),
            theme.foreground(),
            theme.border(),
            theme.accent(),
            theme.muted_foreground(),
        );
        let label = self
            .tabs
            .read(cx)
            .get(id)
            .map(Tab::label)
            .unwrap_or_default();

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::rgba(0x00000080))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .rounded_lg()
                    .bg(card)
                    .border_1()
                    .border_color(border)
                    .text_color(fg)
                    .child(SharedString::from(format!(
                        "Discard unsaved changes to \u{201c}{label}\u{201d}?"
                    )))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .justify_end()
                            .child(
                                div()
                                    .id("confirm-cancel")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .text_color(muted)
                                    .hover(|s| s.bg(border).text_color(fg))
                                    .child("Cancel")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.confirm_close = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("confirm-discard")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(accent)
                                    .text_color(fg)
                                    .hover(|s| s.opacity(0.85))
                                    .child("Discard")
                                    .on_click(cx.listener(
                                        move |this, _: &ClickEvent, window, cx| {
                                            this.do_close(id, window, cx);
                                        },
                                    )),
                            ),
                    ),
            )
    }

    fn render_context_menu(
        &mut self,
        id: u64,
        pos: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (card, fg, border, muted) = (
            theme.card(),
            theme.foreground(),
            theme.border(),
            theme.muted_foreground(),
        );
        let kind = self.tabs.read(cx).get(id).map(|t| t.kind);

        let item = |label: &str, key: &'static str| {
            div()
                .id(key)
                .px_3()
                .py_1()
                .text_xs()
                .rounded_sm()
                .text_color(fg)
                .hover(|s| s.bg(border))
                .child(SharedString::from(label.to_string()))
        };

        // Full-window catcher: any click dismisses the menu.
        div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _w, cx| {
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .absolute()
                    .left(pos.x)
                    .top(pos.y)
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .p_1()
                    .min_w(px(160.0))
                    .rounded_md()
                    .bg(card)
                    .border_1()
                    .border_color(border)
                    .text_color(muted)
                    .child(item("Close", "close").on_click(cx.listener(
                        move |this, _: &ClickEvent, window, cx| {
                            this.context_menu = None;
                            this.request_close(id, window, cx);
                        },
                    )))
                    .child(item("Close Others", "others").on_click(cx.listener(
                        move |this, _: &ClickEvent, window, cx| {
                            this.context_menu = None;
                            this.close_others(id, window, cx);
                        },
                    )))
                    .when_some(kind, |el, kind| {
                        el.child(item("Close All Of This Type", "kind").on_click(cx.listener(
                            move |this, _: &ClickEvent, window, cx| {
                                this.context_menu = None;
                                this.close_by_kind(kind, window, cx);
                            },
                        )))
                    }),
            )
    }
}

impl Focusable for Workspace {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tab_bar = self.render_tab_bar(cx);
        let content = self.render_content(cx);
        let confirm = self
            .confirm_close
            .map(|id| self.render_confirm(id, cx).into_any_element());
        let context_menu = self
            .context_menu
            .map(|(id, pos)| self.render_context_menu(id, pos, cx).into_any_element());

        div()
            .track_focus(&self.focus_handle)
            .key_context("Workspace")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .on_key_down(cx.listener(Self::on_key_down))
            .child(tab_bar)
            .child(div().flex_1().min_h_0().child(content))
            .children(confirm)
            .children(context_menu)
    }
}

impl Workspace {
    fn on_key_down(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = &ks.modifiers;
        // Cmd-based tab shortcuts (full configurability lands in Phase 12).
        if !m.platform || m.control || m.alt {
            return;
        }
        match (m.shift, ks.key.as_str()) {
            (false, "t") => {
                self.open_terminal_tab(window, cx);
                self.focus_active(window, cx);
                cx.stop_propagation();
            }
            (false, "w") => {
                let id = self.tabs.read(cx).active_id();
                self.request_close(id, window, cx);
                cx.stop_propagation();
            }
            (true, "]") | (false, "}") => {
                self.cycle_tab(true, window, cx);
                cx.stop_propagation();
            }
            (true, "[") | (false, "{") => {
                self.cycle_tab(false, window, cx);
                cx.stop_propagation();
            }
            _ => {}
        }
    }
}
