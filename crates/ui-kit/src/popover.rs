//! Shared anchored dropdown/popover primitive for statusbar-style info items
//! (T18-004).
//!
//! [`context_menu`](crate::context_menu) covers flat `MenuItem` lists
//! anchored at a click point; the statusbar info items (Notifications,
//! Agent-Access, …) need the same anchor/dismiss mechanics around arbitrary
//! rich content (badges, list rows, "Clear all"). A shared positioner measures
//! the rendered card, flips it when the opposite side has more room, then
//! clamps it to a consistent viewport margin. The deferred layer escapes
//! ancestor clipping, and the card owns outside-click dismissal.

use gpui::{
    deferred, div, point, px, size, AnimationExt, AnyElement, App, Bounds, Display, Element,
    GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, KeyDownEvent, LayoutId,
    Length, ParentElement, Pixels, Point, Position, Size, Style, Styled, Window,
};
use std::rc::Rc;

use crate::animation::fade_in;
use crate::palette::Palette;

/// Position one or more popup children after measuring their actual bounds.
/// This keeps flip/clamp decisions consistent for menus, selects, and rich
/// popovers instead of relying on each caller to estimate panel dimensions.
pub(crate) struct PopupPositioner {
    anchor: Point<Pixels>,
    margin: Pixels,
    children: Vec<AnyElement>,
}

pub(crate) fn anchored_popup(anchor: Point<Pixels>, c: Palette) -> PopupPositioner {
    PopupPositioner {
        anchor,
        margin: c.space(8.0),
        children: Vec::new(),
    }
}

impl ParentElement for PopupPositioner {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl IntoElement for PopupPositioner {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PopupPositioner {
    type RequestLayoutState = Vec<LayoutId>;
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let child_layout_ids = self
            .children
            .iter_mut()
            .map(|child| child.request_layout(window, cx))
            .collect::<Vec<_>>();
        let viewport = window.viewport_size();
        let margin = self.margin + window.client_inset().unwrap_or(px(0.0));
        let max_size = max_popup_size(viewport, margin);
        let style = Style {
            display: Display::Flex,
            position: Position::Absolute,
            max_size,
            ..Style::default()
        };
        let layout_id = window.request_layout(style, child_layout_ids.iter().copied(), cx);
        (layout_id, child_layout_ids)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child_layout_ids: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if child_layout_ids.is_empty() {
            return;
        }

        let mut child_min = point(Pixels::MAX, Pixels::MAX);
        let mut child_max = Point::default();
        for child_layout_id in child_layout_ids.iter() {
            let child_bounds = window.layout_bounds(*child_layout_id);
            child_min = child_min.min(&child_bounds.origin);
            child_max = child_max.max(&child_bounds.bottom_right());
        }
        let panel_size: Size<Pixels> = (child_max - child_min).into();
        let margin = self.margin + window.client_inset().unwrap_or(px(0.0));
        let origin = popup_origin(self.anchor, panel_size, window.viewport_size(), margin);
        let offset = point(
            (origin.x - bounds.origin.x).round(),
            (origin.y - bounds.origin.y).round(),
        );
        window.with_element_offset(offset, |window| {
            for child in &mut self.children {
                child.prepaint(window, cx);
            }
        });
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _child_layout_ids: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        for child in &mut self.children {
            child.paint(window, cx);
        }
    }
}

fn popup_origin(
    anchor: Point<Pixels>,
    panel: Size<Pixels>,
    viewport: Size<Pixels>,
    margin: Pixels,
) -> Point<Pixels> {
    let x = f32::from(anchor.x);
    let y = f32::from(anchor.y);
    let width = f32::from(panel.width);
    let height = f32::from(panel.height);
    let viewport_width = f32::from(viewport.width);
    let viewport_height = f32::from(viewport.height);
    let margin = f32::from(margin);

    let right_space = viewport_width - margin - x;
    let left_space = x - margin;
    let below_space = viewport_height - margin - y;
    let above_space = y - margin;

    let mut origin_x = x;
    let mut origin_y = y;
    if width > right_space && left_space > right_space {
        origin_x = x - width;
    }
    if height > below_space && above_space > below_space {
        origin_y = y - height;
    }

    let max_x = (viewport_width - margin - width).max(margin);
    let max_y = (viewport_height - margin - height).max(margin);
    point(
        px(origin_x.clamp(margin, max_x)),
        px(origin_y.clamp(margin, max_y)),
    )
}

fn max_popup_size(viewport: Size<Pixels>, margin: Pixels) -> Size<Length> {
    size(
        Length::from((viewport.width - margin - margin).max(px(0.0))),
        Length::from((viewport.height - margin - margin).max(px(0.0))),
    )
}

/// Build a dropdown card anchored at `anchor` (window coordinates — pass the
/// trigger's bottom-left point so the card opens below the item). Clicks
/// outside the card and Escape while the popup content has focus dismiss it.
pub fn popover(
    anchor: Point<Pixels>,
    width: Pixels,
    c: Palette,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    content: AnyElement,
) -> AnyElement {
    let (card_bg, border) = (c.popover, c.border);
    let dismiss = Rc::new(dismiss);
    let dismiss_outside = dismiss.clone();
    let dismiss_escape = dismiss.clone();

    let card = anchored_popup(anchor, c).child(
        div()
            .occlude()
            .w(width)
            .flex()
            .flex_col()
            .rounded_md()
            .bg(card_bg)
            .border_1()
            .border_color(border)
            .shadow_lg()
            .child(content)
            .on_mouse_down_out(move |_, window, cx| dismiss_outside(window, cx))
            .on_key_down(
                move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
                    if event.keystroke.key.as_str() == "escape" {
                        dismiss_escape(window, cx);
                        cx.stop_propagation();
                    }
                },
            )
            .with_animation("popover-fade", fade_in(c), |el, delta| el.opacity(delta)),
    );

    deferred(card).with_priority(200).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_popups_flip_to_the_opposite_corner_when_it_fits() {
        let origin = popup_origin(
            point(px(790.0), px(590.0)),
            size(px(200.0), px(120.0)),
            size(px(800.0), px(600.0)),
            px(8.0),
        );

        assert_eq!(origin, point(px(590.0), px(470.0)));
    }

    #[test]
    fn trigger_popups_flip_above_when_the_below_space_is_insufficient() {
        let origin = popup_origin(
            point(px(400.0), px(590.0)),
            size(px(180.0), px(200.0)),
            size(px(800.0), px(600.0)),
            px(8.0),
        );

        assert_eq!(origin, point(px(400.0), px(390.0)));
    }

    #[test]
    fn anchored_popups_flip_left_when_the_right_side_is_too_narrow() {
        let origin = popup_origin(
            point(px(790.0), px(100.0)),
            size(px(180.0), px(120.0)),
            size(px(800.0), px(600.0)),
            px(8.0),
        );

        assert_eq!(origin, point(px(610.0), px(100.0)));
    }

    #[test]
    fn edge_collisions_clamp_to_the_shared_viewport_margin() {
        let origin = popup_origin(
            point(px(2.0), px(4.0)),
            size(px(200.0), px(100.0)),
            size(px(800.0), px(600.0)),
            px(8.0),
        );

        assert_eq!(origin, point(px(8.0), px(8.0)));
    }

    #[test]
    fn short_viewports_constrain_popup_size_to_the_usable_area() {
        assert_eq!(
            max_popup_size(size(px(300.0), px(180.0)), px(8.0)),
            size(Length::from(px(284.0)), Length::from(px(164.0))),
        );
        assert_eq!(
            max_popup_size(size(px(12.0), px(10.0)), px(8.0)),
            size(Length::from(px(0.0)), Length::from(px(0.0))),
        );
    }
}
