//! `Indicator` — a small coloured status dot.
//!
//! The port draws this shape everywhere by hand:
//! `crates/hosts-ui/src/hosts.rs` (per-host reachability, 7px),
//! `crates/workspace/src/workspace.rs` (connected-session dot and the
//! unsaved-changes dot, 6px each) and several statusbar items. Zed keeps the
//! same primitive at
//! `zed-refrence/zed/crates/ui/src/components/indicator.rs`.
//!
//! ```ignore
//! indicator(IndicatorSize::Sm, c.success)
//! ```

use gpui::{div, px, Div, Hsla, Styled};

/// Dot diameter. `Xs`/`Sm`/`Md` mirrors the [`crate::ButtonSize`] naming.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum IndicatorSize {
    /// 6px — inline with 11–13px text (tab dirty markers).
    #[default]
    Xs,
    /// 7px — list rows (host reachability).
    Sm,
    /// 10px — standalone badges.
    Md,
}

impl IndicatorSize {
    fn diameter(self) -> f32 {
        match self {
            IndicatorSize::Xs => 6.0,
            IndicatorSize::Sm => 7.0,
            IndicatorSize::Md => 10.0,
        }
    }
}

/// A filled circle in `color`.
pub fn indicator(size: IndicatorSize, color: Hsla) -> Div {
    div()
        .flex_shrink_0()
        .size(px(size.diameter()))
        .rounded_full()
        .bg(color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_every_size() {
        for s in [IndicatorSize::Xs, IndicatorSize::Sm, IndicatorSize::Md] {
            let _ = indicator(s, gpui::black());
        }
    }

    #[test]
    fn sizes_are_ordered() {
        assert!(IndicatorSize::Xs.diameter() < IndicatorSize::Sm.diameter());
        assert!(IndicatorSize::Sm.diameter() < IndicatorSize::Md.diameter());
    }
}
