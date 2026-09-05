//! `Kbd` key chips and the `KeybindingHint` (label + chips) pair.
//!
//! Port of `reference-src/src/components/ui/kbd.tsx` (`Kbd` + `KbdGroup`):
//! `h-5.5 min-w-5.5 rounded-lg bg-muted px-1.5 text-xs text-muted-foreground`,
//! grouped with `gap-1`. Zed's equivalents are
//! `zed-refrence/zed/crates/ui/src/components/keybinding.rs` (`KeyBinding`) and
//! `.../keybinding_hint.rs`.
//!
//! Before this module the port hand-rolled the chip in
//! `crates/command-palette/src/palette.rs` (a private `kbd` fn) and again as
//! bordered boxes in `crates/settings-ui/src/panes/shortcuts.rs`.
//!
//! ```ignore
//! kbd_row(["\u{2318}", "K"], c)                 // just the chips
//! keybinding_hint("Toggle Sidebar", ["\u{2318}", "B"], c)   // label + chips
//! ```

use gpui::{div, px, Div, ParentElement, SharedString, Styled};

use crate::palette::Palette;

/// One key chip (`⌘`, `⇧`, `K`, `Esc`, …).
pub fn kbd(label: impl Into<SharedString>, c: Palette) -> Div {
    div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .min_w(px(18.0))
        .px(px(4.0))
        .py(px(1.0))
        .rounded(px(c.radius.sm))
        .bg(c.muted_bg)
        .text_size(px(10.0))
        .text_color(c.muted)
        .child(label.into())
}

/// A `KbdGroup`: several [`kbd`] chips in a `gap-1` row.
pub fn kbd_row<S: Into<SharedString>>(keys: impl IntoIterator<Item = S>, c: Palette) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(2.0))
        .children(keys.into_iter().map(|k| kbd(k, c)))
}

/// A `KeybindingHint`: a muted label followed by its key chips — the shape used
/// for "press ⌘K to …" affordances and the palette footer.
pub fn keybinding_hint<S: Into<SharedString>>(
    label: impl Into<SharedString>,
    keys: impl IntoIterator<Item = S>,
    c: Palette,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .text_size(px(10.0))
        .text_color(c.muted)
        .child(label.into())
        .child(kbd_row(keys, c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn chips_and_hints_build() {
        let c = test_palette();
        let _ = kbd("\u{2318}", c);
        let _ = kbd_row(["\u{2318}", "K"], c);
        let _ = kbd_row(Vec::<SharedString>::new(), c);
        let _ = keybinding_hint("Open", ["\u{2318}", "P"], c);
    }
}
