//! Workspace-owned status-bar contributions.
//!
//! Status items that expose workspace state or workspace-scoped agent access
//! belong here. The shell only inserts the resulting registrations into the
//! shared status-item registry.

use gpui::{
    canvas, div, px, AnyElement, App, AppContext, Bounds, ClickEvent, Context, Entity,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Pixels, Point, Render,
    SharedString, StatefulInteractiveElement, Styled, Window,
};
use labonair_mcp_core::SessionKind;
use labonair_panel::{StatusItem, StatusSide};
use labonair_theme::store::ThemeStore;
use labonair_ui_kit::{IconName, Palette};

use crate::agent_access::{AgentAccessEntry, AgentAccessStore};
use crate::Workspace;

/// Statusbar badge exposing the workspace-scoped MCP agent grants.
pub struct AgentAccessStatusItem {
    store: Entity<AgentAccessStore>,
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
    open: bool,
    /// The badge trigger's window-space bounds from the last paint; the
    /// dropdown drops from `bounds.bottom_left()`.
    trigger_bounds: Option<Bounds<Pixels>>,
    /// Click position that opened the dropdown — used only before the first
    /// paint records `trigger_bounds`.
    fallback_anchor: Point<Pixels>,
    focus: gpui::FocusHandle,
}

impl AgentAccessStatusItem {
    pub fn new(
        store: Entity<AgentAccessStore>,
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&store, |_, _, cx| cx.notify()).detach();
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            store,
            workspace,
            theme,
            open: false,
            trigger_bounds: None,
            fallback_anchor: Point::default(),
            focus: cx.focus_handle(),
        }
    }

    fn render_badge(
        &mut self,
        entries: Vec<AgentAccessEntry>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (fg, muted, border, accent) = {
            let t = self.theme.read(cx);
            (t.foreground(), t.muted_foreground(), t.border(), t.accent())
        };
        let count = entries.len();

        let badge = div()
            .id("agent-access-badge")
            .track_focus(&self.focus)
            .key_context("StatusPopover")
            .relative()
            .size(px(20.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .text_color(muted)
            .hover(|s| s.bg(border).text_color(fg))
            .child(IconName::Shield.svg(muted))
            .child({
                let weak = cx.weak_entity();
                canvas(
                    move |bounds, _window, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            if this.trigger_bounds != Some(bounds) {
                                this.trigger_bounds = Some(bounds);
                                cx.notify();
                            }
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full()
            })
            .child(
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
            .on_click(cx.listener(|this, ev: &ClickEvent, w, cx| {
                if this.open {
                    this.open = false;
                } else {
                    this.open = true;
                    this.fallback_anchor = ev.position();
                    w.focus(&this.focus);
                }
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                if this.open && ev.keystroke.key == "escape" {
                    this.open = false;
                    cx.notify();
                    cx.stop_propagation();
                }
            }));

        if !self.open {
            return div()
                .relative()
                .flex_shrink_0()
                .child(badge)
                .into_any_element();
        }
        let anchor = self
            .trigger_bounds
            .map(|b| b.bottom_left())
            .unwrap_or(self.fallback_anchor);

        let view = cx.entity();
        let dismiss = {
            let v = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.open = false;
                    cx.notify();
                })
            }
        };
        let content = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(border)
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(fg)
                    .child("AI Agent Access"),
            )
            .children(entries.into_iter().map(|entry| {
                let tab_id = entry.tab_id;
                let session_id = entry.session_id.clone();
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1p5()
                    .hover(|s| s.bg(border))
                    .child(
                        div()
                            .id(SharedString::from(format!("agent-jump-{tab_id}")))
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .text_color(fg)
                            .truncate()
                            .child(SharedString::from(entry.label.clone()))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.open = false;
                                this.workspace
                                    .update(cx, |w, cx| w.reveal_tab(tab_id, window, cx));
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("agent-revoke-{tab_id}")))
                            .px_1()
                            .rounded_sm()
                            .text_xs()
                            .text_color(muted)
                            .hover(|s| s.text_color(fg))
                            .child("\u{2715}")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                let session_id = session_id.clone();
                                this.store.update(cx, |s, cx| {
                                    s.set_grant(
                                        tab_id,
                                        session_id,
                                        false,
                                        String::new(),
                                        SessionKind::Ssh,
                                        None,
                                        cx,
                                    );
                                });
                                cx.notify();
                            })),
                    )
            }))
            .into_any_element();

        div()
            .relative()
            .flex_shrink_0()
            .child(badge)
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

impl Render for AgentAccessStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for AgentAccessStatusItem {
    fn id(&self) -> &'static str {
        "agent-access"
    }

    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }

    fn order(&self) -> i32 {
        30
    }

    fn group(&self) -> u32 {
        1
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let (enabled, entries) = {
            let aa = self.store.read(cx);
            (aa.bridge_enabled(), aa.entries())
        };
        if !enabled || entries.is_empty() {
            return div().into_any_element();
        }
        self.render_badge(entries, cx)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Workspace-derived informational items.
// ─────────────────────────────────────────────────────────────────────────────

/// Statusbar item showing the active editor cursor position.
pub struct CursorPositionStatusItem {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
}

impl CursorPositionStatusItem {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self { workspace, theme }
    }
}

impl Render for CursorPositionStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for CursorPositionStatusItem {
    fn id(&self) -> &'static str {
        "cursor-position"
    }

    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }

    fn order(&self) -> i32 {
        11
    }

    fn group(&self) -> u32 {
        0
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some((line, col)) = self.workspace.read(cx).active_editor_cursor(cx) else {
            return div().into_any_element();
        };
        let muted = self.theme.read(cx).muted_foreground();
        div()
            .text_size(px(11.0))
            .text_color(muted)
            .child(SharedString::from(format!("Ln {line}, Col {col}")))
            .into_any_element()
    }
}

/// Statusbar item exposing the active native preview URL.
pub struct PreviewUrlStatusItem {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
}

impl PreviewUrlStatusItem {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self { workspace, theme }
    }
}

impl Render for PreviewUrlStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for PreviewUrlStatusItem {
    fn id(&self) -> &'static str {
        "preview-url"
    }

    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }

    fn order(&self) -> i32 {
        12
    }

    fn group(&self) -> u32 {
        0
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(url) = self.workspace.read(cx).active_preview_url(cx) else {
            return div().into_any_element();
        };
        let (muted, foreground) = {
            let theme = self.theme.read(cx);
            (theme.muted_foreground(), theme.foreground())
        };
        let open = url.clone();
        div()
            .id("bar-preview-url")
            .flex()
            .items_center()
            .gap_1()
            .text_size(px(11.0))
            .text_color(muted)
            .hover(|style| style.text_color(foreground))
            .child(IconName::Globe.svg(muted).size(px(11.0)))
            .child(SharedString::from(
                url.strip_prefix("http://").unwrap_or(&url).to_string(),
            ))
            .on_click(cx.listener(move |_, _: &ClickEvent, _window, cx| {
                cx.open_url(&open);
            }))
            .into_any_element()
    }
}

/// Build the cursor-position status-bar contribution.
pub fn cursor_position_registration(
    workspace: &Entity<Workspace>,
    theme: &Entity<ThemeStore>,
    cx: &mut App,
) -> labonair_panel::StatusItemRegistration {
    let item = cx.new(|cx| CursorPositionStatusItem::new(workspace.clone(), theme.clone(), cx));
    let handle = item.clone();
    labonair_panel::StatusItemRegistration {
        id: item.read(cx).id(),
        default_side: item.read(cx).default_side(),
        order: item.read(cx).order(),
        group: item.read(cx).group(),
        build: std::sync::Arc::new(move |_window, _cx| {
            std::sync::Arc::new(handle.clone()) as labonair_panel::AnyStatusItemHandle
        }),
    }
}

/// Build the preview-URL status-bar contribution.
pub fn preview_url_registration(
    workspace: &Entity<Workspace>,
    theme: &Entity<ThemeStore>,
    cx: &mut App,
) -> labonair_panel::StatusItemRegistration {
    let item = cx.new(|cx| PreviewUrlStatusItem::new(workspace.clone(), theme.clone(), cx));
    let handle = item.clone();
    labonair_panel::StatusItemRegistration {
        id: item.read(cx).id(),
        default_side: item.read(cx).default_side(),
        order: item.read(cx).order(),
        group: item.read(cx).group(),
        build: std::sync::Arc::new(move |_window, _cx| {
            std::sync::Arc::new(handle.clone()) as labonair_panel::AnyStatusItemHandle
        }),
    }
}

/// Build the Agent Access status-item contribution for application composition.
pub fn status_item_registration(
    store: &Entity<AgentAccessStore>,
    workspace: &Entity<Workspace>,
    theme: &Entity<ThemeStore>,
    cx: &mut App,
) -> labonair_panel::StatusItemRegistration {
    let item = cx
        .new(|cx| AgentAccessStatusItem::new(store.clone(), workspace.clone(), theme.clone(), cx));
    let handle = item.clone();
    labonair_panel::StatusItemRegistration {
        id: item.read(cx).id(),
        default_side: item.read(cx).default_side(),
        order: item.read(cx).order(),
        group: item.read(cx).group(),
        build: std::sync::Arc::new(move |_window, _cx| {
            std::sync::Arc::new(handle.clone()) as labonair_panel::AnyStatusItemHandle
        }),
    }
}
