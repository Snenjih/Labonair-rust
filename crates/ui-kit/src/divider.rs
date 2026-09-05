//! Shared `Divider` primitive.
//!
//! Port of `reference-src/src/components/ui/separator.tsx`:
//! `shrink-0 bg-border`, `h-px w-full` horizontal / `w-px self-stretch`
//! vertical. Zed's counterpart is
//! `zed-refrence/zed/crates/ui/src/components/divider.rs`.
//!
//! Replaces the `div().h(px(1.0)).bg(border)` / `div().w(px(1.0)).bg(border)`
//! one-offs hand-rolled across `crates/shell/src/titlebar.rs`,
//! `crates/panel-ai/src/panel_ai.rs` (`MdBlock::Rule`),
//! `crates/workspace/src/views/preview.rs` (`MdBlock::Rule`) and
//! `crates/workspace/src/views/sftp.rs` (the pane splitter).
//!
//! ```ignore
//! div().child(divider(Axis::Horizontal, c.border))
//! ```

use gpui::{div, px, Div, Hsla, Styled};

/// Orientation of a [`divider`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// A 1px line in `color`, full-width (horizontal) or full-height (vertical).
/// The caller places it in a flex row/column and controls surrounding margin.
pub fn divider(axis: Axis, color: Hsla) -> Div {
    let el = div().flex_shrink_0().bg(color);
    match axis {
        Axis::Horizontal => el.h(px(1.0)).w_full(),
        Axis::Vertical => el.w(px(1.0)).h_full(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_both_axes() {
        let _ = divider(Axis::Horizontal, gpui::black());
        let _ = divider(Axis::Vertical, gpui::black());
    }
}
