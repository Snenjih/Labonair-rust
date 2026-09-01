//! GPUI diff view (T06-004).
//!
//! [`DiffView`] renders the difference between two text contents — computed
//! by [`labonair_editor::Diff`] (line-based Myers) — as either a **unified**
//! (single column, `+`/`-` prefixed) or **side-by-side** (old left, new
//! right) layout. Colours come from the active [`ThemeStore`] semantic
//! palette (`success` for insertions, `error` for deletions, `modified` for
//! changed lines, `info` for hunk headers) and re-resolve on theme change.
//!
//! The component is deliberately caller-agnostic: [`DiffView::set_content`]
//! takes two strings and a label, and [`DiffView::on_stage_hunk`] exposes a
//! per-hunk action hook that the Git UI (Phase 8) and AI diff (Phase 10)
//! wire up — the staging logic itself lives in those phases.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, SharedString, Styled, Window,
};
use labonair_editor::diff::{side_by_side, ChangeTag, Diff, Hunk, RowKind};

use crate::theme::ThemeStore;

/// How the diff is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLayout {
    /// One column, deletions then insertions, `+`/`-` prefixed.
    Unified,
    /// Two columns: old on the left, new on the right.
    Split,
}

/// Per-hunk action callback (Phase 8 hunk staging). The `usize` is the index
/// into [`DiffView::diff`]`.hunks`.
type StageHunkFn = dyn Fn(usize, &mut Window, &mut App);

/// A reusable diff pane. Feed it two texts via [`Self::set_content`].
pub struct DiffView {
    theme: Entity<ThemeStore>,
    focus_handle: FocusHandle,
    title: SharedString,
    diff: Diff,
    layout: DiffLayout,
    /// Index into `diff.hunks` of the hunk to highlight (hunk navigation).
    active_hunk: usize,
    /// Optional per-hunk action hook, wired by the Git phase. The `usize` is
    /// the index into [`Self::diff`]`.hunks`.
    on_stage_hunk: Option<Box<StageHunkFn>>,
}

impl DiffView {
    pub fn new(theme: Entity<ThemeStore>, cx: &mut Context<Self>) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            theme,
            focus_handle: cx.focus_handle(),
            title: SharedString::default(),
            diff: Diff::default(),
            layout: DiffLayout::Unified,
            active_hunk: 0,
            on_stage_hunk: None,
        }
    }

    /// Replaces the compared content. `title` labels the header (e.g. a path).
    pub fn set_content(
        &mut self,
        old: &str,
        new: &str,
        title: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.diff = Diff::compute(old, new);
        self.title = title.into();
        self.active_hunk = 0;
        cx.notify();
    }

    pub fn diff(&self) -> &Diff {
        &self.diff
    }

    pub fn layout(&self) -> DiffLayout {
        self.layout
    }

    pub fn active_hunk(&self) -> usize {
        self.active_hunk
    }

    pub fn set_layout(&mut self, layout: DiffLayout, cx: &mut Context<Self>) {
        if self.layout != layout {
            self.layout = layout;
            cx.notify();
        }
    }

    pub fn toggle_layout(&mut self, cx: &mut Context<Self>) {
        let next = match self.layout {
            DiffLayout::Unified => DiffLayout::Split,
            DiffLayout::Split => DiffLayout::Unified,
        };
        self.set_layout(next, cx);
    }

    /// Moves the hunk highlight to the next hunk (clamped).
    pub fn next_hunk(&mut self, cx: &mut Context<Self>) {
        if self.active_hunk + 1 < self.diff.hunks.len() {
            self.active_hunk += 1;
            cx.notify();
        }
    }

    /// Moves the hunk highlight to the previous hunk (clamped).
    pub fn prev_hunk(&mut self, cx: &mut Context<Self>) {
        if self.active_hunk > 0 {
            self.active_hunk -= 1;
            cx.notify();
        }
    }

    /// Wires a per-hunk action callback (Phase 8 hunk staging). The index
    /// passed to `f` matches [`Self::diff`]`.hunks`.
    pub fn on_stage_hunk(&mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) {
        self.on_stage_hunk = Some(Box::new(f));
    }

    fn on_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        match ev.keystroke.key.as_str() {
            "j" | "down" => self.next_hunk(cx),
            "k" | "up" => self.prev_hunk(cx),
            "s" => self.toggle_layout(cx),
            _ => {}
        }
    }
}

impl Focusable for DiffView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DiffView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let bg = theme.background();
        let fg = theme.foreground();
        let muted = theme.muted_foreground();
        let border = theme.border();
        let card = theme.card();
        let insert = theme.status_success();
        let delete = theme.status_error();
        let modified = theme.status_modified();
        let info = theme.status_info();
        let font = theme.buffer_font();
        let font_px = theme.buffer_font_size();
        let line_h = (font_px * 1.5).ceil().max(1.0);

        let (ins, del) = self.diff.stats();
        let hunk_count = self.diff.hunks.len();
        let active = self.active_hunk;
        let is_split = self.layout == DiffLayout::Split;

        let btn = |label: SharedString, on: bool| {
            div()
                .px(px(6.0))
                .py(px(1.0))
                .rounded_sm()
                .text_size(px(font_px * 0.85))
                .text_color(if on { fg } else { muted })
                .when(on, |d| d.bg(card))
                .child(label)
        };

        let header = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .h(px(line_h + 6.0))
            .px(px(8.0))
            .border_b_1()
            .border_color(border)
            .bg(card)
            .text_size(px(font_px * 0.9))
            .text_color(fg)
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(if self.title.is_empty() {
                        SharedString::from("Diff")
                    } else {
                        self.title.clone()
                    }),
            )
            .child(
                div()
                    .text_color(insert)
                    .child(SharedString::from(format!("+{ins}"))),
            )
            .child(
                div()
                    .text_color(delete)
                    .child(SharedString::from(format!("-{del}"))),
            )
            .when(hunk_count > 0, |d| {
                d.child(div().text_color(muted).child(SharedString::from(format!(
                    "hunk {}/{}",
                    active + 1,
                    hunk_count
                ))))
                .child(btn(SharedString::from("\u{2191}"), false))
                .child(btn(SharedString::from("\u{2193}"), false))
            })
            .child(btn(SharedString::from("Unified"), !is_split))
            .child(btn(SharedString::from("Split"), is_split));

        let mut body = div()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .font(font.clone())
            .text_size(px(font_px))
            .line_height(px(line_h));

        if self.diff.hunks.is_empty() {
            body = body.child(
                div()
                    .p(px(12.0))
                    .text_color(muted)
                    .text_size(px(font_px * 0.9))
                    .child(SharedString::from("No changes")),
            );
        } else {
            let gutter = (self.diff.lines.len() + 1).to_string().len().max(3) as f32;
            let gutter_w = gutter * font_px * 0.6 + 10.0;
            for (idx, hunk) in self.diff.hunks.iter().enumerate() {
                let is_active = idx == active;
                // Ellipsis divider for the skipped context before this hunk.
                body = body.child(
                    div()
                        .px(px(8.0))
                        .bg(info.opacity(0.08))
                        .text_color(info)
                        .text_size(px(font_px * 0.85))
                        .child(SharedString::from(hunk.header())),
                );
                let rows = if is_split {
                    render_split_hunk(hunk, gutter_w, line_h, fg, muted, insert, delete, modified)
                } else {
                    render_unified_hunk(hunk, gutter_w, line_h, fg, muted, insert, delete)
                };
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .when(is_active, |d| d.border_l_2().border_color(modified))
                        .children(rows),
                );
            }
            body = body.child(
                div()
                    .px(px(8.0))
                    .text_color(muted)
                    .text_size(px(font_px * 0.8))
                    .child(SharedString::from("\u{2026}")),
            );
        }

        div()
            .key_context("DiffView")
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .size_full()
            .bg(bg)
            .text_color(fg)
            .on_key_down(cx.listener(Self::on_key))
            .child(header)
            .child(body)
    }
}

#[allow(clippy::too_many_arguments)]
fn render_unified_hunk(
    hunk: &Hunk,
    gutter_w: f32,
    line_h: f32,
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    insert: gpui::Hsla,
    delete: gpui::Hsla,
) -> Vec<gpui::AnyElement> {
    hunk.lines
        .iter()
        .map(|line| {
            let (sign, color, tint) = match line.tag {
                ChangeTag::Equal => (' ', fg, None),
                ChangeTag::Insert => ('+', insert, Some(insert)),
                ChangeTag::Delete => ('-', delete, Some(delete)),
            };
            let num = |n: Option<usize>| n.map(|n| n.to_string()).unwrap_or_default();
            div()
                .flex()
                .h(px(line_h))
                .items_center()
                .when_some(tint, |d, c| d.bg(c.opacity(0.12)))
                .child(gutter(num(line.old_line), gutter_w, muted))
                .child(gutter(num(line.new_line), gutter_w, muted))
                .child(
                    div()
                        .w(px(gutter_w.min(16.0)))
                        .flex_shrink_0()
                        .text_color(color)
                        .child(SharedString::from(sign.to_string())),
                )
                .child(
                    div()
                        .flex_1()
                        .whitespace_nowrap()
                        .text_color(color)
                        .child(SharedString::from(display_text(&line.text))),
                )
                .into_any_element()
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn render_split_hunk(
    hunk: &Hunk,
    gutter_w: f32,
    line_h: f32,
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    insert: gpui::Hsla,
    delete: gpui::Hsla,
    modified: gpui::Hsla,
) -> Vec<gpui::AnyElement> {
    side_by_side(hunk)
        .into_iter()
        .map(|row| {
            let (left_tint, right_tint) = match row.kind {
                RowKind::Context => (None, None),
                RowKind::Delete => (Some(delete), None),
                RowKind::Insert => (None, Some(insert)),
                RowKind::Replace => (Some(modified), Some(modified)),
            };
            let cell = |c: Option<labonair_editor::SideCell>, tint: Option<gpui::Hsla>| {
                let (num, text) = c.map(|c| (c.line.to_string(), c.text)).unwrap_or_default();
                let color = tint.unwrap_or(fg);
                div()
                    .flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .h(px(line_h))
                    .items_center()
                    .when_some(tint, |d, c| d.bg(c.opacity(0.12)))
                    .child(gutter(num, gutter_w, muted))
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_color(color)
                            .child(SharedString::from(display_text(&text))),
                    )
            };
            div()
                .flex()
                .child(cell(row.left, left_tint))
                .child(
                    div()
                        .w(px(1.0))
                        .h(px(line_h))
                        .flex_shrink_0()
                        .bg(muted.opacity(0.3)),
                )
                .child(cell(row.right, right_tint))
                .into_any_element()
        })
        .collect()
}

fn gutter(num: String, w: f32, color: gpui::Hsla) -> impl IntoElement {
    div()
        .w(px(w))
        .flex_shrink_0()
        .px(px(4.0))
        .text_color(color)
        .child(SharedString::from(num))
}

/// Render an empty line as a single space so the row keeps its height.
fn display_text(text: &str) -> String {
    if text.is_empty() {
        " ".to_string()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext};

    fn setup(cx: &mut TestAppContext) -> Entity<DiffView> {
        cx.update(|cx| {
            let theme = cx.new(|_| ThemeStore::new(gpui::WindowAppearance::Light));
            cx.new(|cx| DiffView::new(theme, cx))
        })
    }

    #[gpui::test]
    fn set_content_computes_hunks(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.set_content("a\nb\nc\n", "a\nB\nc\n", "file.txt", cx);
                assert_eq!(v.diff().hunks.len(), 1);
                assert_eq!(v.diff().stats(), (1, 1));
            });
        });
    }

    #[gpui::test]
    fn layout_toggles(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                assert_eq!(v.layout(), DiffLayout::Unified);
                v.toggle_layout(cx);
                assert_eq!(v.layout(), DiffLayout::Split);
                v.toggle_layout(cx);
                assert_eq!(v.layout(), DiffLayout::Unified);
            });
        });
    }

    #[gpui::test]
    fn hunk_navigation_is_clamped(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                let old: String = (1..=30).map(|i| format!("l{i}\n")).collect();
                let mut lines: Vec<String> = (1..=30).map(|i| format!("l{i}")).collect();
                lines[2] = "X".into();
                lines[27] = "Y".into();
                v.set_content(&old, &format!("{}\n", lines.join("\n")), "f", cx);
                assert_eq!(v.diff().hunks.len(), 2);
                assert_eq!(v.active_hunk(), 0);
                v.prev_hunk(cx);
                assert_eq!(v.active_hunk(), 0);
                v.next_hunk(cx);
                assert_eq!(v.active_hunk(), 1);
                v.next_hunk(cx);
                assert_eq!(v.active_hunk(), 1);
            });
        });
    }

    #[gpui::test]
    fn stage_hunk_hook_can_be_wired(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, _cx| {
                assert!(v.on_stage_hunk.is_none());
                v.on_stage_hunk(|_idx, _window, _app| {});
                assert!(v.on_stage_hunk.is_some());
            });
        });
    }
}
