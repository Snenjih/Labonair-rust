//! Path-bookmarks popover (T12-003).
//!
//! Port of `reference-src/src/modules/bookmarks/components/BookmarksDropdown.tsx`
//! and `BookmarkRow.tsx`. The pure model (add/remove/key/orphan/filter) and JSON
//! persistence live in `labonair_backend::modules::bookmarks`; this is the GPUI
//! overlay: a context-filtered, host-grouped list with per-row remove and
//! orphan flagging, plus an "add the current folder" action. Opened by the
//! `bookmarks.open` shortcut (`Cmd+Shift+O`) / command palette.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement,
    Render, SharedString, StatefulInteractiveElement, Styled, Window,
};
use labonair_backend::modules::bookmarks as model;
use labonair_backend::modules::bookmarks::{BookmarkContext, PathBookmark};

use crate::theme::ThemeStore;
use crate::workspace::Workspace;
use crate::ExplorerView;

/// Emitted when the user picks a bookmark. `AppShell` resolves the jump.
#[derive(Clone, Debug)]
pub enum BookmarkEvent {
    /// Navigate the local explorer to this path.
    OpenLocal(String),
    /// Open (or focus) an SFTP browser for this host.
    OpenRemote { host_id: String, path: String },
}

pub struct BookmarksView {
    theme: Entity<ThemeStore>,
    workspace: Entity<Workspace>,
    explorer: Entity<ExplorerView>,
    open: bool,
    bookmarks: Vec<PathBookmark>,
    focus: FocusHandle,
}

impl EventEmitter<BookmarkEvent> for BookmarksView {}

/// Emitted so a hosting [`ModalLayer`](labonair_workspace::modal_layer::ModalLayer)
/// can drop the popover when it closes itself (Esc / overlay click / a pick).
impl EventEmitter<DismissEvent> for BookmarksView {}

impl BookmarksView {
    pub fn new(
        theme: Entity<ThemeStore>,
        workspace: Entity<Workspace>,
        explorer: Entity<ExplorerView>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            theme,
            workspace,
            explorer,
            open: false,
            bookmarks: model::load(),
            focus: cx.focus_handle(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.close(cx);
        } else {
            self.open(window, cx);
        }
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.bookmarks = model::load();
        self.open = true;
        window.focus(&self.focus);
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        let was_open = self.open;
        self.open = false;
        if was_open {
            cx.emit(DismissEvent);
        }
        cx.notify();
    }

    /// The `(host_id, path)` the "add current folder" action targets, given the
    /// active tab. `None` when there is no resolvable path context.
    fn add_target(&self, cx: &App) -> Option<(Option<String>, String)> {
        let ws = self.workspace.read(cx);
        match ws.active_host_id(cx) {
            Some(host_id) => ws.active_cwd(cx).map(|p| (Some(host_id), p)),
            None => self
                .explorer
                .read(cx)
                .root()
                .map(|p| (None, p.to_string_lossy().to_string())),
        }
    }

    fn context(&self, cx: &App) -> BookmarkContext {
        let ws = self.workspace.read(cx);
        match ws.active_host_id(cx) {
            Some(host_id) => {
                if ws.active_is_terminal(cx) {
                    BookmarkContext::Host(host_id)
                } else {
                    BookmarkContext::Sftp(host_id)
                }
            }
            None if self.explorer.read(cx).root().is_some() => BookmarkContext::Local,
            None => BookmarkContext::None,
        }
    }

    fn persist(&mut self, next: Vec<PathBookmark>, cx: &mut Context<Self>) {
        if let Err(message) = model::save(&next) {
            labonair_notifications::notification_center(cx).update(cx, |c, cx| {
                c.push(
                    labonair_notifications::Notification::error("Bookmark save failed", message),
                    cx,
                )
            });
            return;
        }
        self.bookmarks = next;
        cx.notify();
    }

    fn add_current(&mut self, cx: &mut Context<Self>) {
        let Some((host_id, path)) = self.add_target(cx) else {
            return;
        };
        if let Some(next) =
            model::compute_add_bookmark(&self.bookmarks, host_id.as_deref(), &path, None)
        {
            self.persist(next, cx);
        }
    }

    fn remove(&mut self, id: String, cx: &mut Context<Self>) {
        let next = model::compute_remove_by_id(&self.bookmarks, &id);
        self.persist(next, cx);
    }

    fn pick(&mut self, bm: PathBookmark, cx: &mut Context<Self>) {
        self.close(cx);
        match bm.host_id {
            Some(host_id) => cx.emit(BookmarkEvent::OpenRemote {
                host_id,
                path: bm.path,
            }),
            None => cx.emit(BookmarkEvent::OpenLocal(bm.path)),
        }
    }

    fn on_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.open && ev.keystroke.key == "escape" {
            self.close(cx);
            cx.stop_propagation();
        }
    }
}

impl Focusable for BookmarksView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for BookmarksView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div();
        }

        let t = self.theme.read(cx);
        let (fg, muted, border, card) =
            (t.foreground(), t.muted_foreground(), t.border(), t.card());
        let sel_fill = t.selected_fill();

        let hosts = self.workspace.read(cx).known_hosts(cx);
        let host_ids: Vec<String> = hosts.iter().map(|(id, _)| id.clone()).collect();
        let ctx = self.context(cx);
        let sections = model::filter_for_context(&ctx, &self.bookmarks, &hosts);
        let can_add = self.add_target(cx).is_some();

        let mut list = div().flex().flex_col().py(px(4.0)).max_h(px(360.0));
        let total: usize = sections.iter().map(|s| s.bookmarks.len()).sum();
        if total == 0 {
            list = list.child(
                div()
                    .px(px(12.0))
                    .py(px(10.0))
                    .text_size(px(11.0))
                    .text_color(muted)
                    .child("No bookmarks yet"),
            );
        }
        for section in &sections {
            if section.bookmarks.is_empty() {
                continue;
            }
            list = list.child(
                div()
                    .px(px(12.0))
                    .pt(px(6.0))
                    .pb(px(2.0))
                    .text_size(px(9.0))
                    .text_color(muted)
                    .child(SharedString::from(section.title.to_uppercase())),
            );
            for bm in &section.bookmarks {
                let orphaned = model::is_bookmark_orphaned(bm, &host_ids);
                let label = bm.label.clone().unwrap_or_else(|| bm.path.clone());
                let bm_pick = bm.clone();
                let id_remove = bm.id.clone();
                let row_id = SharedString::from(format!("bookmark-row-{}", bm.id));
                let label_id = SharedString::from(format!("bookmark-label-{}", bm.id));
                let remove_id = SharedString::from(format!("bookmark-remove-{}", bm.id));
                list = list.child(
                    div()
                        .id(row_id)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.0))
                        .mx(px(4.0))
                        .px(px(crate::theme::menu_metrics::ITEM_PAD_X))
                        .h(px(26.0))
                        .rounded_sm()
                        .text_size(px(12.0))
                        .text_color(fg)
                        .hover(|s| s.bg(sel_fill))
                        .child(
                            div()
                                .id(label_id)
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(SharedString::from(label))
                                .when(orphaned, |d| {
                                    d.child(
                                        div()
                                            .text_size(px(9.0))
                                            .text_color(muted)
                                            .child("host removed"),
                                    )
                                })
                                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                    this.pick(bm_pick.clone(), cx)
                                })),
                        )
                        .child(
                            div()
                                .id(remove_id)
                                .px(px(4.0))
                                .text_size(px(12.0))
                                .text_color(muted)
                                .hover(|s| s.text_color(fg))
                                .child("\u{00d7}")
                                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                    this.remove(id_remove.clone(), cx)
                                })),
                        ),
                );
            }
        }

        div()
            .absolute()
            .inset_0()
            .flex()
            .justify_center()
            .pt(px(80.0))
            .bg(crate::theme::modal_scrim())
            .track_focus(&self.focus)
            .key_context("BookmarksPopover")
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _w, cx| this.close(cx)),
            )
            .child(
                div()
                    .occlude()
                    .w(px(420.0))
                    .max_h(px(440.0))
                    .flex()
                    .flex_col()
                    .rounded_md()
                    .bg(card)
                    .border_1()
                    .border_color(border)
                    .child(
                        div()
                            .h(px(32.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .px(px(12.0))
                            .border_b_1()
                            .border_color(border)
                            .text_size(px(12.0))
                            .text_color(fg)
                            .child("Path Bookmarks")
                            .when(can_add, |d| {
                                d.child(
                                    div()
                                        .id("bookmark-add-current")
                                        .text_size(px(11.0))
                                        .text_color(muted)
                                        .hover(|s| s.text_color(fg))
                                        .child("+ Add current folder")
                                        .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                            this.add_current(cx)
                                        })),
                                )
                            }),
                    )
                    .child(list),
            )
    }
}
