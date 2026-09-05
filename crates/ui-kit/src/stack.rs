//! Flex-layout shorthands.
//!
//! `div().flex().flex_col()` / `div().flex().flex_row().items_center()` are by
//! far the most repeated constructs in the port (>400 occurrences across the
//! view crates). Zed keeps the same two helpers in
//! `zed-refrence/zed/crates/ui/src/styles/stack.rs`; `labonair-gpui-ext` has no
//! layout helpers, so they live here with the rest of the design system.
//!
//! ```ignore
//! v_stack().gap_2().child(header).child(body)
//! h_stack().gap_1().child(icon).child(label)
//! ```

use gpui::{div, Div, Styled};

/// A vertical flex column.
pub fn v_stack() -> Div {
    div().flex().flex_col()
}

/// A horizontal flex row, vertically centred (the shape every
/// icon-next-to-label row wants).
pub fn h_stack() -> Div {
    div().flex().flex_row().items_center()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::ParentElement;

    #[test]
    fn stacks_build() {
        let _ = v_stack().child("a");
        let _ = h_stack().child("a");
    }
}
