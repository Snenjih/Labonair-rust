//! `GitChangeRow` — a dense version-control entry (Zed-parity redesign Phase 2,
//! `docs/ui-comparison-zed-sidebar-status-bar.md` §9.5 / §10.4 / §12.5).
//!
//! Distinct from [`ListItem`](crate::ListItem): a tri-state staging *control*
//! (checkbox: Unstaged / Staged / PartiallyStaged) whose meaning is independent
//! of the semantic status icon/tint, file identity with a full-path fallback,
//! and contextual actions that stay hidden until the row is hovered.
//!
//! State stays with the caller. Handlers are plain
//! `Fn(.., &mut Window, &mut App)` closures so the row can be built inside a
//! `uniform_list` render closure.

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, ClickEvent, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, SharedString, StatefulInteractiveElement, Styled,
    Window,
};

use crate::density::Density;
use crate::icon::IconName;
use crate::palette::Palette;

/// The staging state of a file or an aggregate (section / directory).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageState {
    /// Nothing staged.
    Unstaged,
    /// Fully staged.
    Staged,
    /// Some children staged, some not (aggregate rows only).
    PartiallyStaged,
}

impl StageState {
    /// The checkbox is "on" only when fully staged.
    fn is_checked(self) -> bool {
        matches!(self, StageState::Staged)
    }
}

type ToggleFn = Rc<dyn Fn(&bool, &mut Window, &mut App)>;
type ClickFn = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type MouseFn = Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;

/// A dense VCS change row. Build with [`git_change_row`].
pub struct GitChangeRow {
    id: SharedString,
    c: Palette,
    depth: usize,
    stage: StageState,
    status_glyph: SharedString,
    status_color: gpui::Hsla,
    label: SharedString,
    secondary: Option<SharedString>,
    icon: Option<IconName>,
    selected: bool,
    tooltip: Option<SharedString>,
    actions: Option<AnyElement>,
    on_toggle_stage: Option<ToggleFn>,
    on_click: Option<ClickFn>,
    on_secondary_down: Option<MouseFn>,
}

const BASE_INSET: f32 = 8.0;
const INDENT_STEP: f32 = 12.0;

/// Start a [`GitChangeRow`]. `id` doubles as the hover-group name, so it must be
/// unique within the list.
pub fn git_change_row(
    id: impl Into<SharedString>,
    c: Palette,
    stage: StageState,
    label: impl Into<SharedString>,
) -> GitChangeRow {
    GitChangeRow {
        id: id.into(),
        c,
        depth: 0,
        stage,
        status_glyph: SharedString::default(),
        status_color: c.muted,
        label: label.into(),
        secondary: None,
        icon: None,
        selected: false,
        tooltip: None,
        actions: None,
        on_toggle_stage: None,
        on_click: None,
        on_secondary_down: None,
    }
}

impl GitChangeRow {
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// The semantic status badge (`M`, `A`, `D`, `?`, …) and its tint — shown
    /// independently of the staging checkbox.
    pub fn status(mut self, glyph: impl Into<SharedString>, color: gpui::Hsla) -> Self {
        self.status_glyph = glyph.into();
        self.status_color = color;
        self
    }

    /// A muted secondary line / suffix (the directory part of the path — the
    /// full-path fallback so identity is never reduced to an ambiguous tail).
    pub fn secondary(mut self, secondary: impl Into<SharedString>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Contextual actions — rendered invisibly and revealed on row hover.
    pub fn actions(mut self, actions: impl IntoElement) -> Self {
        self.actions = Some(actions.into_any_element());
        self
    }

    /// Fires with the desired new staged state when the checkbox is clicked.
    pub fn on_toggle_stage(
        mut self,
        handler: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_stage = Some(Rc::new(handler));
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn on_secondary_down(
        mut self,
        handler: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_secondary_down = Some(Rc::new(handler));
        self
    }
}

impl IntoElement for GitChangeRow {
    type Element = gpui::Stateful<gpui::Div>;

    fn into_element(self) -> Self::Element {
        let c = self.c;
        let d = Density::from_palette(&c);
        let inset = BASE_INSET + self.depth as f32 * INDENT_STEP;
        let checked = self.stage.is_checked();
        let partial = matches!(self.stage, StageState::PartiallyStaged);

        // Tri-state staging control.
        let box_border = if checked || partial {
            c.primary
        } else {
            gpui::transparent_black()
        };
        let box_bg = if checked {
            c.primary
        } else if partial {
            c.primary.opacity(0.4)
        } else {
            c.input.opacity(0.9)
        };
        let mut check = div()
            .id(SharedString::from(format!("{}::stage", self.id)))
            .flex_none()
            .size(px(15.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(c.radius.sm))
            .border_1()
            .border_color(box_border)
            .bg(box_bg)
            .when(checked, |x| {
                x.child(IconName::SquareCheck.svg(c.primary_fg).size(px(10.0)))
            })
            .when(partial, |x| {
                x.child(IconName::Minus.svg(c.primary_fg).size(px(10.0)))
            });
        if let Some(handler) = self.on_toggle_stage.clone() {
            let next = !checked;
            check = check.cursor_pointer().on_click(move |_, w, cx| {
                cx.stop_propagation();
                handler(&next, w, cx);
            });
        }

        let actions = self.actions.map(|el| {
            div()
                .flex_none()
                .invisible()
                .group_hover(self.id.clone(), |s| s.visible())
                .child(el)
        });

        let mut row = div()
            .id(self.id.clone())
            .group(self.id.clone())
            .relative()
            .flex()
            .items_center()
            .w_full()
            .h(d.tree_row_height())
            .pl(px(inset))
            .pr(px(BASE_INSET))
            .gap(d.row_inner_gap())
            .text_size(px(13.0))
            .text_color(c.fg)
            .cursor_pointer();

        if self.selected {
            row = row.bg(c.accent);
        } else {
            let hover = c.accent.opacity(0.5);
            row = row.hover(move |s| s.bg(hover));
        }

        row = row
            .child(check)
            .child(
                div()
                    .w(px(12.0))
                    .flex_none()
                    .text_size(px(11.0))
                    .text_color(self.status_color)
                    .child(self.status_glyph),
            )
            .when_some(self.icon, |x, icon| {
                x.child(icon.svg(c.muted).size(px(14.0)))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_baseline()
                    .gap(px(4.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(self.label)
                    .when_some(self.secondary, |x, sec| {
                        x.child(div().text_size(px(11.0)).text_color(c.muted).child(sec))
                    }),
            )
            .children(actions);

        if let Some(tooltip) = self.tooltip {
            row = row
                .tooltip(move |window, cx| crate::Tooltip::new(tooltip.clone()).build(window, cx));
        }
        if let Some(h) = self.on_click {
            row = row.on_click(move |ev, w, cx| h(ev, w, cx));
        }
        if let Some(h) = self.on_secondary_down {
            row = row.on_mouse_down(MouseButton::Right, move |ev, w, cx| h(ev, w, cx));
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn builds_in_every_stage_state() {
        let c = test_palette();
        for stage in [
            StageState::Unstaged,
            StageState::Staged,
            StageState::PartiallyStaged,
        ] {
            for selected in [true, false] {
                let _ = git_change_row("git-row-1", c, stage, "file.rs")
                    .depth(2)
                    .status("M", c.warning)
                    .secondary("src/deep")
                    .icon(IconName::FileCode)
                    .selected(selected)
                    .tooltip("src/deep/file.rs")
                    .actions("x")
                    .on_toggle_stage(|_, _, _| {})
                    .on_click(|_, _, _| {})
                    .on_secondary_down(|_, _, _| {})
                    .into_element();
            }
        }
    }

    /// Zed-parity Phase 5.5: builds at compact / default / comfortable density
    /// and deep indent. The row is `w_full` with an ellipsised identity, so a
    /// minimum-width Git dock cannot force horizontal overflow.
    #[test]
    fn builds_at_every_density_and_deep_indent() {
        for density in [0.85_f32, 1.0, 1.15] {
            let mut c = test_palette();
            c.density = density;
            for depth in [0usize, 3, 12] {
                for stage in [
                    StageState::Unstaged,
                    StageState::Staged,
                    StageState::PartiallyStaged,
                ] {
                    let _ = git_change_row("g", c, stage, "deeply/nested/module/file-name.rs")
                        .depth(depth)
                        .status("M", c.warning)
                        .secondary("deeply/nested/module")
                        .on_toggle_stage(|_, _, _| {})
                        .into_element();
                }
            }
        }
    }
}
