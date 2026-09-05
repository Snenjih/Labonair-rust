//! `Banner` — a full-width info / warning / error strip.
//!
//! Port of `reference-src/src/components/ui/alert.tsx` (`alertVariants`):
//! `border-<sev>/50 bg-<sev>/10 text-<sev>` with a leading severity icon; the
//! `default` variant is the plain `bg-card text-card-foreground` note. Zed's
//! counterpart is `zed-refrence/zed/crates/ui/src/components/banner.rs`.
//!
//! Replaces the two hand-rolled strips in `crates/settings-ui/src/view.rs`
//! (the JSON syntax-error and schema-validation banners — which hardcoded
//! `gpui::red()` / `gpui::yellow()` instead of the theme's status tokens, a
//! Critical Rule 3 violation this primitive fixes). The remaining strips of
//! the same shape — `crates/workspace/src/views/editor.rs`'s external-change
//! banner and `crates/panel-explorer/src/panel_explorer.rs`'s clipboard strip
//! — move over in T20-002.
//!
//! ```ignore
//! banner(Severity::Error, c)
//!     .child("labonair-settings.json has a syntax error")
//!     .child(reload_button)
//! ```

use gpui::{div, prelude::FluentBuilder, px, AnyElement, Div, IntoElement, ParentElement, Styled};

use crate::icon::IconName;
use crate::palette::Palette;

/// Which status tokens the banner is tinted with.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Severity {
    /// `bg-card text-card-foreground` — a neutral note, no icon.
    #[default]
    Note,
    Info,
    Success,
    Warning,
    Error,
}

impl Severity {
    /// The leading icon, or `None` for [`Severity::Note`].
    pub fn icon(self) -> Option<IconName> {
        match self {
            Severity::Note => None,
            Severity::Info => Some(IconName::Info),
            Severity::Success => Some(IconName::CircleCheck),
            Severity::Warning => Some(IconName::Warning),
            Severity::Error => Some(IconName::CircleX),
        }
    }

    /// The token this severity draws its text/border/fill from.
    pub fn color(self, c: Palette) -> gpui::Hsla {
        match self {
            Severity::Note => c.card_fg,
            Severity::Info => c.info,
            Severity::Success => c.success,
            Severity::Warning => c.warning,
            Severity::Error => c.error,
        }
    }
}

/// A banner strip. Children are laid out in a row after the severity icon —
/// pass the message plus any action elements.
pub struct Banner {
    severity: Severity,
    c: Palette,
    /// Lay the children out in a column instead of a row (multi-line reports).
    stacked: bool,
    children: Vec<AnyElement>,
}

/// A [`Banner`] of the given severity.
pub fn banner(severity: Severity, c: Palette) -> Banner {
    Banner {
        severity,
        c,
        stacked: false,
        children: Vec::new(),
    }
}

impl Banner {
    /// Stack the children vertically (one line per validation finding) instead
    /// of laying them out in a row.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }
}

impl ParentElement for Banner {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl IntoElement for Banner {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let (c, sev) = (self.c, self.severity);
        let color = sev.color(c);
        div()
            .flex()
            .flex_row()
            .flex_shrink_0()
            .items_start()
            .gap(c.space(8.0))
            .w_full()
            .px(c.space(12.0))
            .py(c.space(6.0))
            .text_size(px(11.0))
            .text_color(color)
            .when(sev == Severity::Note, |d| d.bg(c.card))
            .when(sev != Severity::Note, |d| {
                d.bg(color.opacity(0.1))
                    .border_b_1()
                    .border_color(color.opacity(0.5))
            })
            .when_some(sev.icon(), |d, icon| {
                d.child(div().pt(c.space(1.0)).child(icon.svg(color).size(px(13.0))))
            })
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .when(self.stacked, |d| d.flex_col().gap(c.space(2.0)))
                    .when(!self.stacked, |d| {
                        d.flex_row().items_center().gap(c.space(8.0))
                    })
                    .children(self.children),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn builds_every_severity() {
        let c = test_palette();
        for sev in [
            Severity::Note,
            Severity::Info,
            Severity::Success,
            Severity::Warning,
            Severity::Error,
        ] {
            let _ = banner(sev, c).child("message").into_element();
            let _ = banner(sev, c).stacked(true).child("line").into_element();
        }
    }

    #[test]
    fn severity_colors_come_from_the_status_tokens() {
        let c = test_palette();
        assert_eq!(Severity::Error.color(c), c.error);
        assert_eq!(Severity::Warning.color(c), c.warning);
        assert_eq!(Severity::Info.color(c), c.info);
        assert_eq!(Severity::Success.color(c), c.success);
        assert_eq!(Severity::Note.color(c), c.card_fg);
        assert!(Severity::Note.icon().is_none());
        assert!(Severity::Error.icon().is_some());
    }
}
