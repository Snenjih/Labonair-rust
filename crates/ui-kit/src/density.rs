//! Semantic UI-density tokens (Zed-parity redesign Phase 2, `docs/ui-comparison-zed-sidebar-status-bar.md`
//! §10.3).
//!
//! [`Palette`] already carries a raw `density` multiplier (T20-007). This module
//! turns it into a small *named* set of layout dimensions so feature renderers
//! stop hardcoding `h(px(22.0))` / `h(px(24.0))` and instead ask for
//! [`Density::tree_row_height`] etc. It is deliberately **not** a port of Zed's
//! `DynamicSpacing` enum — it is a flat accessor over the one multiplier.
//!
//! Rule (§10.3): only *rows, gaps and hit targets* scale with density. Hairline
//! borders and focus indicators stay 1–2 logical pixels at every density.

use gpui::{px, Pixels};

use crate::Palette;

/// The resolved density-aware layout dimensions for the active theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Density {
    scale: f32,
}

impl Density {
    /// Build from a raw spacing multiplier (`Palette::density`).
    pub fn new(scale: f32) -> Self {
        Self { scale }
    }

    /// Build from the per-render [`Palette`] snapshot.
    pub fn from_palette(c: &Palette) -> Self {
        Self { scale: c.density }
    }

    #[inline]
    fn s(&self, value: f32) -> Pixels {
        px(value * self.scale)
    }

    // ── scaled: rows / gaps / hit targets ────────────────────────────────────

    /// Outer padding of the status bar row.
    pub fn status_bar_padding(&self) -> Pixels {
        self.s(4.0)
    }

    /// Height of an interactive status-bar / toolbar control.
    pub fn control_height(&self) -> Pixels {
        self.s(22.0)
    }

    /// Height of one flat tree row (Explorer entry, Git change row).
    pub fn tree_row_height(&self) -> Pixels {
        self.s(24.0)
    }

    /// Height of a collapsible section header.
    pub fn section_header_height(&self) -> Pixels {
        self.s(22.0)
    }

    /// Gap between a row's icon and its label / between inline row atoms.
    pub fn row_inner_gap(&self) -> Pixels {
        self.s(4.0)
    }

    /// Height of a panel-owned tab bar / header strip.
    pub fn panel_header_height(&self) -> Pixels {
        self.s(28.0)
    }

    /// Invisible drag hit target overlaid on a dock boundary. Scales so it
    /// stays comfortably grabbable at higher densities.
    pub fn dock_resize_hit_target(&self) -> Pixels {
        self.s(6.0)
    }

    // ── fixed: hairlines / focus indicators (never scale, §10.3) ─────────────

    /// A structural hairline border.
    pub fn hairline(&self) -> Pixels {
        px(1.0)
    }

    /// The width of a row's right-edge focus / active indicator.
    pub fn focus_indicator(&self) -> Pixels {
        px(2.0)
    }

    /// The visible 1px dock boundary the resize target sits over.
    pub fn dock_boundary(&self) -> Pixels {
        px(1.0)
    }
}

impl Palette {
    /// The density-aware layout tokens for this palette snapshot.
    pub fn density_tokens(&self) -> Density {
        Density::from_palette(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn rows_and_gaps_scale_with_density() {
        let compact = Density::new(0.85);
        let default = Density::new(1.0);
        let comfortable = Density::new(1.15);

        assert!(compact.tree_row_height() < default.tree_row_height());
        assert!(default.tree_row_height() < comfortable.tree_row_height());
        assert!(compact.row_inner_gap() < comfortable.row_inner_gap());
        assert!(compact.section_header_height() < comfortable.section_header_height());
        assert!(compact.dock_resize_hit_target() < comfortable.dock_resize_hit_target());
    }

    #[test]
    fn borders_and_focus_indicators_do_not_scale() {
        let compact = Density::new(0.85);
        let comfortable = Density::new(1.15);
        assert_eq!(compact.hairline(), comfortable.hairline());
        assert_eq!(compact.focus_indicator(), comfortable.focus_indicator());
        assert_eq!(compact.dock_boundary(), comfortable.dock_boundary());
        assert_eq!(compact.focus_indicator(), px(2.0));
    }

    #[test]
    fn built_from_palette_matches_multiplier() {
        let mut c = test_palette();
        c.density = 1.0;
        assert_eq!(c.density_tokens().tree_row_height(), px(24.0));
        c.density = 0.85;
        assert_eq!(Density::from_palette(&c).tree_row_height(), px(24.0 * 0.85));
    }
}
