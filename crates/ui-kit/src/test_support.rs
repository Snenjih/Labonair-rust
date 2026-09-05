//! Shared fixtures for the crate's unit tests.
//!
//! The primitives only *read* tokens, so their tests need no runtime store —
//! just a [`Palette`] snapshotted off a fixed [`Theme`].

use labonair_theme::Theme;

use crate::palette::Palette;
use crate::theme::UiTheme;

/// A bare [`UiTheme`] impl over a fixed [`Theme`].
pub(crate) struct TestTheme(pub Theme);

impl UiTheme for TestTheme {
    fn theme(&self) -> &Theme {
        &self.0
    }
}

/// The dark-theme [`Palette`] every primitive test styles against.
pub(crate) fn test_palette() -> Palette {
    Palette::from_theme(&TestTheme(Theme::dark()))
}
