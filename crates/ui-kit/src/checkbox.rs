//! `Checkbox` — a square, checked/unchecked toggle with an optional label.
//!
//! Port of `reference-src/src/components/ui/checkbox.tsx`: `size-4`,
//! `rounded-[5px]`, transparent border over `bg-input/90`, checked flips to
//! `border-primary bg-primary text-primary-foreground` with a tick glyph;
//! `disabled:opacity-50`. Zed's counterpart is
//! `zed-refrence/zed/crates/ui/src/components/checkbox.rs`.
//!
//! Replaces the two hand-rolled `SquareCheck`/`Square` icon pairs in
//! `crates/hosts-ui/src/hosts.rs` (SSH-config import list, host export list).
//!
//! ```ignore
//! checkbox("row-1", c, selected)
//!     .label("id_ed25519")
//!     .on_click(cx.listener(|this, checked: &bool, _w, cx| this.set(*checked, cx)))
//! ```

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder, px, App, ClickEvent, Div, ElementId, InteractiveElement,
    IntoElement, ParentElement, SharedString, Stateful, StatefulInteractiveElement, Styled, Window,
};

use crate::icon::IconName;
use crate::palette::Palette;
use crate::DISABLED_OPACITY;

/// A checkbox. Build with [`checkbox`].
pub struct Checkbox {
    id: ElementId,
    c: Palette,
    checked: bool,
    disabled: bool,
    label: Option<SharedString>,
    #[allow(clippy::type_complexity)]
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

/// A [`Checkbox`] in `checked` state.
pub fn checkbox(id: impl Into<ElementId>, c: Palette, checked: bool) -> Checkbox {
    Checkbox {
        id: id.into(),
        c,
        checked,
        disabled: false,
        label: None,
        on_click: None,
    }
}

impl Checkbox {
    /// A trailing label, click-through to the box.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Dim + inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Fires with the *new* checked state.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// The box on its own, without the row wrapper — for call sites that place
    /// the tick inside their own selectable row.
    pub fn box_only(self) -> Div {
        let c = self.c;
        div()
            .flex_shrink_0()
            .size(c.space(16.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(c.radius.sm))
            .border_1()
            .border_color(if self.checked {
                c.primary
            } else {
                gpui::transparent_black()
            })
            .bg(if self.checked {
                c.primary
            } else {
                c.input.opacity(0.9)
            })
            .when(self.checked, |d| {
                d.child(IconName::SquareCheck.svg(c.primary_fg).size(px(12.0)))
            })
    }
}

impl IntoElement for Checkbox {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let (c, id, disabled, next) = (self.c, self.id.clone(), self.disabled, !self.checked);
        let (label, handler) = (self.label.clone(), self.on_click.clone());
        div()
            .id(id)
            .flex()
            .items_center()
            .gap_2()
            .text_color(c.fg)
            .when(disabled, |d| d.opacity(DISABLED_OPACITY))
            .when(!disabled, |d| d.cursor_pointer())
            .child(self.box_only())
            .children(label)
            .when(!disabled, move |d| match handler {
                Some(h) => d.on_click(move |_: &ClickEvent, w, cx| h(&next, w, cx)),
                None => d,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn builds_in_every_state() {
        let c = test_palette();
        for checked in [true, false] {
            for disabled in [true, false] {
                let _ = checkbox("cb", c, checked)
                    .label("Label")
                    .disabled(disabled)
                    .on_click(|_, _, _| {})
                    .into_element();
            }
        }
    }

    #[test]
    fn box_only_drops_the_row_wrapper() {
        let _ = checkbox("cb", test_palette(), true).box_only();
    }
}
