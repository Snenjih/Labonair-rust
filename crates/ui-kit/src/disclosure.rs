//! Shared `Disclosure` primitive — a clickable chevron + label row that
//! toggles a collapsed/expanded flag the caller owns.
//!
//! Port of the Zed `Disclosure` component
//! (`zed-refrence/zed/crates/ui/src/components/disclosure.rs`) and of the
//! pattern hand-rolled in
//! `crates/settings-ui/src/panes/generic.rs::render_section_header`, which
//! drew the chevron as the ASCII arrows `▸`/`▾`. This module is the shared
//! replacement — real icon glyphs
//! ([`IconName::ChevronRight`]/[`IconName::ChevronDown`]) instead of text
//! arrows, same click-to-toggle contract (the state lives in the caller, this
//! only renders + fires `on_click`).
//!
//! ```ignore
//! disclosure("section-Terminal", "Terminal", collapsed, muted, fg)
//!     .on_click(cx.listener(move |this, _, _, cx| this.toggle_section(label, cx)))
//! ```

use gpui::{
    div, px, ElementId, Hsla, InteractiveElement, ParentElement, SharedString, Stateful, Styled,
};

use crate::icon::IconName;

/// Builds the disclosure row. `collapsed` selects the chevron direction;
/// `fg` is the hover text color, `muted` the resting one. Callers chain
/// `.on_click(..)` to flip their own collapsed flag.
pub fn disclosure(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    collapsed: bool,
    muted: Hsla,
    fg: Hsla,
) -> Stateful<gpui::Div> {
    let chevron = if collapsed {
        IconName::ChevronRight
    } else {
        IconName::ChevronDown
    };
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_1()
        .cursor_pointer()
        .text_size(px(11.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(muted)
        .hover(move |s| s.text_color(fg))
        .child(chevron.svg(muted).size(px(12.0)))
        .child(label.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chevron_direction_follows_collapsed() {
        // Smoke test: both directions build without panicking.
        let _ = disclosure("d1", "Section", true, gpui::black(), gpui::white());
        let _ = disclosure("d2", "Section", false, gpui::black(), gpui::white());
    }
}
