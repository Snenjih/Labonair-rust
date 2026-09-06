//! `TreeRow` — a dense, flat, full-width hierarchical row (Zed-parity redesign
//! Phase 2, `docs/ui-comparison-zed-sidebar-status-bar.md` §8.6 / §10.4 / §12.4).
//!
//! Unlike [`ListItem`](crate::ListItem) — which is a card-like selectable row
//! with rounded corners and generous padding, used by command palettes and
//! settings — `TreeRow` is purpose-built for a *continuous* hierarchy: no
//! per-row corner radius, one density-derived height (~24px), 4px icon/label
//! gaps, a configurable indent step, and **independent visual channels** for
//! `selected` / `marked` / `focused` / `active_file` / `cut` / `drag_source` /
//! `drop_target` rather than one `is_selected` fill carrying every meaning.
//!
//! State stays with the caller: it passes a [`TreeRowState`] and click
//! handlers; the row never owns a flag. The handlers are plain
//! `Fn(.., &mut Window, &mut App)` closures (not `cx.listener`) so the row can
//! be built inside a `uniform_list` render closure, which only gets
//! `&mut Window, &mut App`.

use std::rc::Rc;

use gpui::{
    div, px, AnyElement, App, ClickEvent, Div, ElementId, Hsla, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, SharedString, Stateful, StatefulInteractiveElement,
    Styled, Window,
};

use crate::density::Density;
use crate::icon::IconName;
use crate::palette::Palette;
use crate::Tooltip;

/// The independent visual state inputs of a [`TreeRow`]. Every field is a
/// separate channel — a renderer assigns each one the *smallest* necessary
/// visual change instead of collapsing them into a single background fill.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TreeRowState {
    /// Part of the committed selection.
    pub selected: bool,
    /// Marked for a pending range/multi operation (lower amplitude than
    /// `selected`).
    pub marked: bool,
    /// Holds keyboard focus — draws a right-edge indicator, not a fill.
    pub focused: bool,
    /// This row is the file open in the active editor.
    pub active_file: bool,
    /// Pending clipboard *cut* — dimmed + tinted.
    pub cut: bool,
    /// Currently being dragged.
    pub drag_source: bool,
    /// A drag is hovering this row as a drop destination.
    pub drop_target: bool,
}

type ClickFn = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type MouseFn = Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;

/// A flat hierarchical row. Build with [`tree_row`].
pub struct TreeRow {
    id: ElementId,
    c: Palette,
    depth: usize,
    indent_step: f32,
    chevron: Option<IconName>,
    icon: Option<IconName>,
    /// Icon-theme asset path for the leading glyph; wins over `icon` when set.
    icon_path: Option<SharedString>,
    /// Icon-theme asset path for the disclosure chevron; wins over `chevron`.
    chevron_path: Option<SharedString>,
    label: SharedString,
    tooltip: Option<SharedString>,
    label_tint: Option<Hsla>,
    trailing: Option<AnyElement>,
    state: TreeRowState,
    indent_guides: bool,
    on_click: Option<ClickFn>,
    on_secondary_down: Option<MouseFn>,
    #[allow(clippy::type_complexity)]
    extra: Option<Box<dyn FnOnce(Stateful<Div>) -> Stateful<Div>>>,
}

/// The default indent, in logical pixels, applied per depth level. Callers
/// override with [`TreeRow::indent_step`].
pub const TREE_INDENT_STEP: f32 = 12.0;

/// Left padding applied before any indentation.
const TREE_BASE_INSET: f32 = 8.0;

/// Start a [`TreeRow`] with the given element id, palette snapshot and label.
pub fn tree_row(id: impl Into<ElementId>, c: Palette, label: impl Into<SharedString>) -> TreeRow {
    TreeRow {
        id: id.into(),
        c,
        depth: 0,
        indent_step: TREE_INDENT_STEP,
        chevron: None,
        icon: None,
        icon_path: None,
        chevron_path: None,
        label: label.into(),
        tooltip: None,
        label_tint: None,
        trailing: None,
        state: TreeRowState::default(),
        indent_guides: false,
        on_click: None,
        on_secondary_down: None,
        extra: None,
    }
}

impl TreeRow {
    /// Hierarchy depth (0 = root child). Drives left inset.
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// Per-depth indent, in logical pixels (default [`TREE_INDENT_STEP`]).
    pub fn indent_step(mut self, step: f32) -> Self {
        self.indent_step = step;
        self
    }

    /// Disclosure chevron (only for expandable rows). `None` keeps the slot for
    /// alignment but draws nothing.
    pub fn chevron(mut self, chevron: Option<IconName>) -> Self {
        self.chevron = chevron;
        self
    }

    /// Leading file / folder glyph.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Leading glyph as an icon-theme asset path (takes precedence over
    /// [`Self::icon`]). Used by the file tree, which resolves each entry
    /// against the active icon theme.
    pub fn icon_path(mut self, path: Option<SharedString>) -> Self {
        self.icon_path = path;
        self
    }

    /// Disclosure chevron as an icon-theme asset path (takes precedence over
    /// [`Self::chevron`]).
    pub fn chevron_path(mut self, path: Option<SharedString>) -> Self {
        self.chevron_path = path;
        self
    }

    /// Full path (or any longer identity) shown on hover — the label itself is
    /// ellipsised to the available width.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Override the label colour (e.g. a Git-status tint). Does not change the
    /// row height.
    pub fn label_tint(mut self, tint: Hsla) -> Self {
        self.label_tint = Some(tint);
        self
    }

    /// A trailing element (decoration / action) pinned to the right edge.
    pub fn trailing(mut self, el: impl IntoElement) -> Self {
        self.trailing = Some(el.into_any_element());
        self
    }

    /// The full visual-state bundle.
    pub fn state(mut self, state: TreeRowState) -> Self {
        self.state = state;
        self
    }

    /// Draw thin 1px vertical guides for each ancestor depth level. Overlay
    /// geometry — does not change the row height (§10.3 / Phase 3.4).
    pub fn indent_guides(mut self, on: bool) -> Self {
        self.indent_guides = on;
        self
    }

    /// Primary click.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Right mouse button down (context menu).
    pub fn on_secondary_down(
        mut self,
        handler: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_secondary_down = Some(Rc::new(handler));
        self
    }

    /// Escape hatch, applied last — drag sources / drop targets / one-off
    /// overrides the builder has no named method for. Must not call `.hover(..)`
    /// (the row already sets one; GPUI panics on a second).
    pub fn extra(mut self, f: impl FnOnce(Stateful<Div>) -> Stateful<Div> + 'static) -> Self {
        self.extra = Some(Box::new(f));
        self
    }
}

impl IntoElement for TreeRow {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let c = self.c;
        let d = Density::from_palette(&c);
        let inset = TREE_BASE_INSET + self.depth as f32 * self.indent_step;
        let st = self.state;

        let text_color = if st.cut {
            c.error
        } else {
            self.label_tint.unwrap_or(c.fg)
        };

        let mut row = div()
            .id(self.id)
            .relative()
            .flex()
            .items_center()
            .w_full()
            .h(d.tree_row_height())
            .pl(px(inset))
            .pr(px(TREE_BASE_INSET))
            .gap(d.row_inner_gap())
            .text_size(px(13.0))
            .text_color(text_color)
            .cursor_pointer();

        // Background channel — strongest meaning wins; hover only when the row
        // has no resting fill.
        if st.drop_target {
            // Drop target: the selection fill plus a primary inset ring so it
            // reads differently from a resting selection.
            row = row.bg(c.accent).child(
                div()
                    .absolute()
                    .inset_0()
                    .border_1()
                    .border_color(c.primary),
            );
        } else if st.selected {
            row = row.bg(c.accent);
        } else if st.marked {
            row = row.bg(c.accent.opacity(0.4));
        } else {
            let hover = c.accent.opacity(0.5);
            row = row.hover(move |s| s.bg(hover));
        }

        if st.cut {
            row = row.opacity(0.5);
        } else if st.drag_source {
            row = row.opacity(0.7);
        }

        // Indent guides — one 1px column per ancestor depth, as an absolute
        // overlay so the row height is untouched (§10.3).
        if self.indent_guides && self.depth > 0 {
            for level in 0..self.depth {
                let x = TREE_BASE_INSET + level as f32 * self.indent_step + self.indent_step / 2.0;
                row = row.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(px(x))
                        .w(px(1.0))
                        .bg(c.border),
                );
            }
        }

        // Disclosure slot — always present so labels line up whether or not the
        // row is expandable.
        row = row.child(
            div()
                .w(px(10.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .children(match self.chevron_path {
                    Some(p) => Some(
                        gpui::svg()
                            .path(p)
                            .size(px(12.0))
                            .flex_none()
                            .text_color(c.muted),
                    ),
                    None => self
                        .chevron
                        .map(|ch| ch.svg(c.muted).size(px(12.0)).flex_none()),
                }),
        );

        if let Some(p) = self.icon_path {
            row = row.child(
                gpui::svg()
                    .path(p)
                    .size(px(14.0))
                    .flex_none()
                    .text_color(c.muted),
            );
        } else if let Some(icon) = self.icon {
            row = row.child(icon.svg(c.muted).size(px(14.0)));
        }

        row = row.child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .child(self.label),
        );

        if let Some(trailing) = self.trailing {
            row = row.child(div().flex_none().child(trailing));
        }

        // Right-edge indicator (≤2px) — active-file beats focus.
        let indicator = if st.active_file {
            Some(c.primary)
        } else if st.focused {
            Some(c.ring)
        } else {
            None
        };
        if let Some(color) = indicator {
            row = row.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right_0()
                    .w(d.focus_indicator())
                    .bg(color),
            );
        }

        if let Some(tooltip) = self.tooltip {
            row = row.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx));
        }
        if let Some(h) = self.on_click {
            row = row.on_click(move |ev, w, cx| h(ev, w, cx));
        }
        if let Some(h) = self.on_secondary_down {
            row = row.on_mouse_down(MouseButton::Right, move |ev, w, cx| h(ev, w, cx));
        }
        if let Some(extra) = self.extra {
            row = extra(row);
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn builds_across_every_state_channel() {
        let c = test_palette();
        let states = [
            TreeRowState::default(),
            TreeRowState {
                selected: true,
                ..Default::default()
            },
            TreeRowState {
                marked: true,
                focused: true,
                ..Default::default()
            },
            TreeRowState {
                active_file: true,
                cut: true,
                ..Default::default()
            },
            TreeRowState {
                drag_source: true,
                drop_target: true,
                ..Default::default()
            },
        ];
        for st in states {
            let _ = tree_row("r", c, "name.rs")
                .depth(3)
                .indent_step(16.0)
                .chevron(Some(IconName::ChevronRight))
                .icon(IconName::FileCode)
                .tooltip("/abs/name.rs")
                .label_tint(c.warning)
                .trailing("2")
                .state(st)
                .indent_guides(true)
                .on_click(|_, _, _| {})
                .on_secondary_down(|_, _, _| {})
                .extra(|row| row.opacity(0.9))
                .into_element();
        }
    }

    /// Zed-parity Phase 5.5: the row must build at compact / default /
    /// comfortable density and at a deep indent without overflowing — the row
    /// is `w_full` and ellipsises its label, so a narrow dock cannot force a
    /// horizontal overflow. This is a build (not pixel) assertion.
    #[test]
    fn builds_at_every_density_and_deep_indent() {
        for density in [0.85_f32, 1.0, 1.15] {
            let mut c = test_palette();
            c.density = density;
            let d = Density::from_palette(&c);
            assert!(d.tree_row_height() > gpui::px(0.0));
            assert_eq!(
                d.focus_indicator(),
                gpui::px(2.0),
                "focus indicator stays 2px"
            );
            for depth in [0usize, 1, 8, 20] {
                let _ = tree_row("r", c, "a-very-long-file-name-that-must-ellipsise.rs")
                    .depth(depth)
                    .icon(IconName::FileCode)
                    .indent_guides(true)
                    .state(TreeRowState {
                        focused: true,
                        ..Default::default()
                    })
                    .into_element();
            }
        }
    }
}
