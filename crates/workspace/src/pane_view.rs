//! Rendering of a tab's pane tree (R10-002, ADR 0005).
//!
//! The tree is laid out with flex: siblings share `extent - gaps` by their
//! flexes (basis 0, grow = weight), which is exactly the maths of
//! [`crate::tiling::solve`], so logic that needs pixel rectangles (split room,
//! focus moves, divider drag) can re-solve the same tree against the measured
//! pane area. Resize handles sit inside the gap strips. A single pane renders
//! full-bleed without frame or bar; from two panes on every pane gets a
//! [`pane_frame`] and a [`pane_bar`]. Zoom paints only the zoomed pane; the
//! rest of the tree keeps its layout state but is not rendered until zoom ends.

use gpui::prelude::FluentBuilder;
use gpui::{
    canvas, div, px, AnyElement, AppContext, ClickEvent, Context, DragMoveEvent,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point,
    Render, SharedString, StatefulInteractiveElement, Styled, Window,
};
use labonair_settings::Settings as _;
use labonair_ui_kit::{
    pane_bar, pane_frame, IconName, MotionExt, Palette, PaneAction, PaneLayoutStyle,
};

use crate::pane::{Member, PaneId, SplitAxis, SplitDirection};
use crate::tiling;
use crate::{DragGhost, PaneResize, Workspace};

/// Value carried while a pane is dragged by its bar. Wave 3 (drop zones,
/// tab/pane DnD) consumes it; this wave only defines the payload and the drag
/// source.
#[derive(Clone, Debug)]
pub struct DraggedPane {
    pub pane_id: PaneId,
    pub tab_id: u64,
    pub label: SharedString,
    pub icon: IconName,
}

/// Mini-chip drag image (icon and title) for [`DraggedPane`] and tab drags.
pub struct PaneDragChip {
    label: SharedString,
    icon: IconName,
    palette: Palette,
    /// Where the pointer grabbed the dragged element. GPUI places the drag
    /// image at the grab origin, so the chip is shifted by this much plus a
    /// small gap to sit just below-right of the pointer.
    grab: Point<Pixels>,
}

impl PaneDragChip {
    pub fn new(label: SharedString, icon: IconName, palette: Palette, grab: Point<Pixels>) -> Self {
        Self {
            label,
            icon,
            palette,
            grab,
        }
    }
}

/// Gap between the pointer and the drag chip.
const CHIP_OFFSET: f32 = 12.0;

impl Render for PaneDragChip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let c = self.palette;
        div()
            .ml(self.grab.x + px(CHIP_OFFSET))
            .mt(self.grab.y + px(CHIP_OFFSET))
            .flex()
            .items_center()
            .gap_1()
            .h(px(24.0))
            .px_2()
            .rounded(px(c.radius.md))
            .bg(c.popover)
            .border_1()
            .border_color(c.line_strong)
            .text_color(c.popover_fg)
            .text_xs()
            .child(self.icon.svg(c.muted))
            .child(self.label.clone())
    }
}

/// Everything one render pass of the tree needs.
struct TreeCtx {
    tab_id: u64,
    c: Palette,
    style: PaneLayoutStyle,
    gap: f32,
    active: Option<PaneId>,
    zoomed: Option<PaneId>,
    fresh: Option<PaneId>,
    /// `appearance.dimInactivePanes`: darken every pane but the active one.
    dim_inactive: bool,
    /// `appearance.paneHighlightColor`: outline drawn around the active pane.
    highlight: Option<gpui::Hsla>,
}

/// A flex child that takes `weight` of the free space (basis 0, no shrink).
fn weighted(mut cell: gpui::Div, weight: f32) -> gpui::Div {
    cell = cell.flex_basis(px(0.0));
    let style = cell.style();
    style.flex_grow = Some(weight.max(0.001) * 1000.0);
    style.flex_shrink = Some(0.0);
    cell
}

impl Workspace {
    /// The content of `tab_id`: its pane tree (or the single pane).
    pub(crate) fn render_pane_area(
        &mut self,
        tab_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.ensure_layout(tab_id, cx);
        let Some(layout) = self.layouts.get(&tab_id).cloned() else {
            return self.render_empty_surface(cx).into_any_element();
        };
        let Some(root) = layout.group.root.clone() else {
            return self.placeholder("Empty tab", cx).into_any_element();
        };
        let (style, metrics) = self.pane_style(cx);
        let c = Palette::from_theme(self.theme.read(cx));
        let area = self.pane_area.clone();
        let probe = canvas(
            move |bounds, _window, _cx| {
                area.set(Some(bounds));
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        let multi = layout.len() > 1;
        let zoomed = self
            .zoom
            .zoomed(tab_id)
            .filter(|z| layout.group.find_pane(*z));
        if !multi {
            let pane = root.first_leaf();
            let body = self.pane_body(pane, window, cx);
            let area = div()
                .relative()
                .size_full()
                .child(probe)
                .child(Self::pane_context(body));
            return self.with_pane_dnd(area, tab_id, cx).into_any_element();
        }

        let ctx = TreeCtx {
            tab_id,
            c,
            style,
            gap: metrics.gap,
            active: layout.active,
            zoomed,
            fresh: self.fresh_pane,
            dim_inactive: labonair_settings::ThemeSettings::try_get(cx)
                .is_some_and(|settings| settings.dim_inactive_panes()),
            highlight: self.theme.read(cx).metrics().pane_highlight,
        };
        // Culling: while a pane is zoomed the rest of the tree is not rendered
        // (the panes live on in their layout and items, only the elements go).
        let tree = match zoomed {
            Some(_) => None,
            None => Some(self.render_member(&root, &ctx, window, cx)),
        };
        // `occlude` keeps pointer events away from anything painted below.
        let overlay = zoomed.map(|pane| {
            let frame = self.render_pane_frame(pane, &ctx, window, cx);
            div()
                .absolute()
                .inset_0()
                .bg(c.bg)
                .occlude()
                .child(frame)
                .into_any_element()
        });
        let inner = div()
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(probe)
            .children(tree)
            .children(overlay)
            .on_drag_move(
                cx.listener(move |this, ev: &DragMoveEvent<PaneResize>, _window, cx| {
                    let drag = ev.drag(cx);
                    this.drag_divider(drag.axis_id, drag.boundary, ev.event.position, cx);
                }),
            );
        let inner = self.with_pane_dnd(inner, tab_id, cx);
        // Comfortable panes float: the whole area keeps a gap margin so the
        // outer frames do not touch the dock or titlebar.
        let margin = if style.is_framed() { metrics.gap } else { 0.0 };
        div()
            .size_full()
            .p(px(margin))
            .child(inner)
            .into_any_element()
    }

    /// Divider drag: re-solve the tree against the measured area and turn the
    /// pointer into the boundary fraction (gap- and minimum-size aware).
    fn drag_divider(
        &mut self,
        axis_id: PaneId,
        boundary: usize,
        pointer: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some((tab, _)) = self.active_pane(cx) else {
            return;
        };
        let (_, metrics) = self.pane_style(cx);
        let Some(layout) = self.layouts.get(&tab) else {
            return;
        };
        let solved = tiling::solve(&layout.group, self.pane_container_px(), metrics.gap);
        let Some(geometry) = solved.axis(axis_id) else {
            return;
        };
        let along = match geometry.axis {
            SplitAxis::Horizontal => f32::from(pointer.x),
            SplitAxis::Vertical => f32::from(pointer.y),
        };
        let min_px = metrics.min_along(geometry.axis);
        let Some(fraction) = geometry.boundary_fraction(boundary, along, min_px) else {
            return;
        };
        if let Some(layout) = self.layouts.get_mut(&tab) {
            if layout.set_boundary(axis_id, boundary, fraction) {
                cx.notify();
            }
        }
    }

    fn render_member(
        &mut self,
        node: &Member,
        ctx: &TreeCtx,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match node {
            Member::Pane(pane) => self.render_pane_frame(*pane, ctx, window, cx),
            Member::Axis(axis) => {
                let row = axis.axis == SplitAxis::Horizontal;
                let axis_id = axis.id;
                let n = axis.members.len();
                let mut container = div()
                    .flex()
                    .size_full()
                    .min_w_0()
                    .min_h_0()
                    .when(row, |d| d.flex_row())
                    .when(!row, |d| d.flex_col());
                for (i, member) in axis.members.iter().enumerate() {
                    let child = self.render_member(member, ctx, window, cx);
                    let weight = axis.flexes.get(i).copied().unwrap_or(1.0 / n as f32);
                    container =
                        container.child(weighted(div().min_w_0().min_h_0(), weight).child(child));
                    if i + 1 < n {
                        container = container.child(self.render_handle(axis_id, i, row, ctx, cx));
                    }
                }
                container.into_any_element()
            }
        }
    }

    /// The gap strip between two siblings with a wider invisible hit area.
    fn render_handle(
        &mut self,
        axis_id: PaneId,
        boundary: usize,
        row: bool,
        ctx: &TreeCtx,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = ctx.c;
        let gap = ctx.gap.max(1.0);
        let hit = gap + 6.0;
        let line = if ctx.style.is_framed() {
            gpui::transparent_black()
        } else {
            c.line_subtle
        };
        let hit_area = div()
            .id(SharedString::from(format!(
                "axis-handle-{axis_id}-{boundary}"
            )))
            .absolute()
            .when(row, |d| {
                d.top_0()
                    .bottom_0()
                    .left(px(-(hit - gap) / 2.0))
                    .w(px(hit))
                    .cursor_col_resize()
            })
            .when(!row, |d| {
                d.left_0()
                    .right_0()
                    .top(px(-(hit - gap) / 2.0))
                    .h(px(hit))
                    .cursor_row_resize()
            })
            .hover(move |s| s.bg(c.primary.opacity(0.35)))
            .on_drag(PaneResize { axis_id, boundary }, |_, _, _, cx| {
                cx.new(|_| DragGhost)
            })
            .on_click(cx.listener(move |this, ev: &ClickEvent, _window, cx| {
                if ev.click_count() >= 2 {
                    this.reset_axis(axis_id, cx);
                }
            }));
        div()
            .relative()
            .flex_shrink_0()
            .bg(line)
            .when(row, |d| d.w(px(gap)).h_full())
            .when(!row, |d| d.h(px(gap)).w_full())
            .child(hit_area)
            .into_any_element()
    }

    /// One pane: frame, bar and body.
    fn render_pane_frame(
        &mut self,
        pane: PaneId,
        ctx: &TreeCtx,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = ctx.c;
        let spec = self.pane_spec(pane, cx);
        let focused = ctx.active == Some(pane);
        let is_zoomed = ctx.zoomed == Some(pane);
        let body = self.pane_body(pane, window, cx);
        let body = Self::pane_context(body).into_any_element();
        let tab_id = ctx.tab_id;
        let ws = cx.entity();

        let action = |tip: &'static str,
                      run: fn(
            &mut Workspace,
            u64,
            PaneId,
            &mut Window,
            &mut Context<Workspace>,
        )| {
            let ws = ws.clone();
            PaneAction::new(tip).on_click(move |window, app| {
                ws.update(app, |this, cx| run(this, tab_id, pane, window, cx));
            })
        };
        let mut bar = pane_bar(
            SharedString::from(format!("pane-bar-{pane}")),
            c,
            spec.icon,
            spec.title.clone(),
        )
        .active(focused)
        .layout(ctx.style)
        .can_split(spec.kind.can_split())
        .split_horizontal(action("Split right", |this, _, pane, window, cx| {
            this.split_pane_dir(pane, SplitDirection::Right, false, window, cx)
        }))
        .split_vertical(action("Split down", |this, _, pane, window, cx| {
            this.split_pane_dir(pane, SplitDirection::Down, false, window, cx)
        }))
        .zoom(action(
            if is_zoomed {
                "Restore pane"
            } else {
                "Zoom pane"
            },
            |this, tab, pane, window, cx| this.toggle_zoom_of(tab, pane, window, cx),
        ))
        .zoomed(is_zoomed)
        .close(action("Close pane", |this, _, pane, window, cx| {
            this.close_pane_id(pane, window, cx)
        }));
        {
            let ws = ws.clone();
            bar = bar.on_double_click(move |window, app| {
                ws.update(app, |this, cx| {
                    this.toggle_zoom_of(tab_id, pane, window, cx)
                });
            });
        }
        if spec.dirty {
            bar = bar.leading(div().size(px(6.0)).rounded_full().bg(c.warning));
        }
        if let Some(trailing) = spec.trailing.clone() {
            bar = bar.trailing(
                div()
                    .truncate()
                    .text_xs()
                    .text_color(c.muted)
                    .child(trailing),
            );
        }
        let drag = DraggedPane {
            pane_id: pane,
            tab_id,
            label: spec.title.clone(),
            icon: spec.icon,
        };
        let bar = bar.render().on_drag(drag, move |drag, grab, _, cx| {
            let chip = PaneDragChip::new(drag.label.clone(), drag.icon, c, grab);
            cx.new(|_| chip)
        });

        // GPUI clips to rectangles, so square content would poke out of the
        // frame's rounded bottom corners; an inset equal to the corner's
        // sagitta keeps it inside (the bar already rounds its top corners).
        let inset = if ctx.style.is_framed() {
            px(c.radius.lg * 0.3)
        } else {
            px(0.0)
        };
        let body = div()
            .relative()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .px(inset)
            .pb(inset)
            .child(body);

        let frame = pane_frame(("pane-frame", pane), c, ctx.style, focused)
            .size_full()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    if this.focused_pane(tab_id) != pane {
                        this.activate_pane(pane, window, cx);
                    }
                }),
            )
            .child(bar)
            .child(body)
            .when(ctx.dim_inactive && !focused, |frame| {
                let radius = if ctx.style.is_framed() {
                    c.radius.lg
                } else {
                    0.0
                };
                frame.child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded(px(radius))
                        .bg(c.bg.opacity(0.45)),
                )
            })
            // An overlay, not a border: the outline must not shift the content.
            .when_some(ctx.highlight.filter(|_| focused), |frame, color| {
                let radius = if ctx.style.is_framed() {
                    c.radius.lg
                } else {
                    0.0
                };
                frame.child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded(px(radius))
                        .border_2()
                        .border_color(color),
                )
            });
        // Only the new pane fades in; fading the split source would blank it
        // for a frame (it is first mounted as a frame here, at opacity 0).
        if ctx.fresh == Some(pane) {
            frame
                .motion_fade_in(("pane-in", pane), c)
                .into_any_element()
        } else {
            frame.into_any_element()
        }
    }
}
