//! Radio-style preview tiles (R10-002): a mini-window mock drawn from theme
//! colours above a label, used for colour scheme, interface style, pane layout
//! and theme pickers.
//!
//! Tiles are always presented as a [`preview_tile_group`]: one selected key, an
//! `on_select` callback, and arrow-key navigation between tiles (Left/Up =
//! previous, Right/Down = next, wrapping; Enter/Space activate). The owner keeps
//! the selected value.
//!
//! ```ignore
//! preview_tile_group("scheme", c, current_key)
//!     .tile(preview_tile("auto", "Auto", PreviewKind::ColorScheme(ColorSchemeChoice::Auto)))
//!     .tile(preview_tile("light", "Light", PreviewKind::ColorScheme(ColorSchemeChoice::Light)))
//!     .tile(preview_tile("dark", "Dark", PreviewKind::ColorScheme(ColorSchemeChoice::Dark)))
//!     .on_select(cx.listener(|this, key: &SharedString, _, cx| this.set_scheme(key, cx)))
//! ```
//!
//! The selected tile has a 2px `primary` ring and a bold label. The ring is a
//! constant 2px border (transparent when idle), so selecting never shifts
//! layout. A keyboard-focused unselected tile shows the focus ring colour.

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder, px, App, ClickEvent, Div, FontWeight, Hsla, InteractiveElement,
    IntoElement, ParentElement, Pixels, SharedString, StatefulInteractiveElement, Styled, Window,
};
use labonair_theme_tokens::Theme;

use crate::palette::Palette;
use crate::pane_bar::PaneLayoutStyle;
use crate::segmented::adjacent_segment_index;
use crate::DISABLED_OPACITY;

/// Mock width, before density scaling.
const MOCK_WIDTH: f32 = 112.0;
/// Mock height, before density scaling.
const MOCK_HEIGHT: f32 = 72.0;

/// Which colour scheme a [`PreviewKind::ColorScheme`] tile stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSchemeChoice {
    /// Half light, half dark: follows the system.
    Auto,
    Light,
    Dark,
}

/// Which surface material a [`PreviewKind::InterfaceStyle`] tile stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfaceStyleChoice {
    /// Opaque surfaces.
    Classic,
    /// Translucent surfaces over the window blur.
    Glassy,
}

/// What a tile's mini-window depicts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewKind {
    ColorScheme(ColorSchemeChoice),
    InterfaceStyle(InterfaceStyleChoice),
    PaneLayout(PaneLayoutStyle),
    /// A theme family: left half its light variant, right half its dark one.
    /// Supply both with [`PreviewTile::colors`].
    Theme,
}

/// The handful of colours a mini-window mock is drawn from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileMock {
    pub bg: Hsla,
    pub chrome: Hsla,
    pub sidebar: Hsla,
    pub fg: Hsla,
    pub muted: Hsla,
    pub line: Hsla,
    pub accent: Hsla,
}

impl TileMock {
    /// The mock of the active theme.
    pub fn from_palette(c: &Palette) -> Self {
        Self {
            bg: c.bg,
            chrome: c.toolbar_bg,
            sidebar: c.sidebar,
            fg: c.fg,
            muted: c.muted,
            line: c.line_subtle,
            accent: c.primary,
        }
    }

    /// The mock of any theme (for example a registry variant, or the built-in
    /// light and dark themes for colour-scheme tiles).
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            bg: theme.core.background,
            chrome: theme.surface.toolbar,
            sidebar: theme.sidebar.background,
            fg: theme.core.foreground,
            muted: theme.core.muted_foreground,
            line: theme.border.subtle,
            accent: theme.core.primary,
        }
    }
}

/// One tile: a stable `key`, a visible `label` and what it depicts.
pub struct PreviewTile {
    key: SharedString,
    label: SharedString,
    kind: PreviewKind,
    colors: Option<(TileMock, TileMock)>,
}

/// Describe a tile. It is rendered by a [`PreviewTileGroup`].
pub fn preview_tile(
    key: impl Into<SharedString>,
    label: impl Into<SharedString>,
    kind: PreviewKind,
) -> PreviewTile {
    PreviewTile {
        key: key.into(),
        label: label.into(),
        kind,
        colors: None,
    }
}

impl PreviewTile {
    /// The light and dark mock colours. Required for [`PreviewKind::Theme`];
    /// optional for the colour-scheme kinds (default: the built-in themes).
    /// Other kinds draw from the active palette.
    pub fn colors(mut self, light: TileMock, dark: TileMock) -> Self {
        self.colors = Some((light, dark));
        self
    }
}

type SelectHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A row of [`PreviewTile`]s acting as one radio group.
pub struct PreviewTileGroup {
    id: SharedString,
    c: Palette,
    selected: SharedString,
    tiles: Vec<PreviewTile>,
    disabled: bool,
    on_select: Option<SelectHandler>,
}

/// Start a tile group whose selected tile is `selected` (a tile key).
pub fn preview_tile_group(
    id: impl Into<SharedString>,
    c: Palette,
    selected: impl Into<SharedString>,
) -> PreviewTileGroup {
    PreviewTileGroup {
        id: id.into(),
        c,
        selected: selected.into(),
        tiles: Vec::new(),
        disabled: false,
        on_select: None,
    }
}

impl PreviewTileGroup {
    pub fn tile(mut self, tile: PreviewTile) -> Self {
        self.tiles.push(tile);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the chosen tile's key by click, Enter/Space or arrow keys.
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// The key of the selected tile, or `None` when it matches no tile.
    pub fn selection(&self) -> Option<&SharedString> {
        selected_index(self.tiles.iter().map(|t| &t.key), &self.selected)
            .map(|index| &self.tiles[index].key)
    }
}

/// Index of `selected` among `keys`.
pub fn selected_index<'a>(
    keys: impl IntoIterator<Item = &'a SharedString>,
    selected: &str,
) -> Option<usize> {
    keys.into_iter().position(|key| key.as_ref() == selected)
}

/// The tile an arrow key moves to from `index` among `len` tiles (wrapping).
/// Left/Up step back, Right/Down step forward; other keys do nothing.
pub fn step_selection(index: usize, len: usize, key: &str) -> Option<usize> {
    let forward = match key {
        "right" | "down" => true,
        "left" | "up" => false,
        _ => return None,
    };
    adjacent_segment_index(index, len, forward)
}

impl IntoElement for PreviewTileGroup {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let Self {
            id,
            c,
            selected,
            tiles,
            disabled,
            on_select,
        } = self;
        let keys: Vec<SharedString> = tiles.iter().map(|tile| tile.key.clone()).collect();
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(c.space(16.0))
            .when(disabled, |group| group.opacity(DISABLED_OPACITY))
            .children(tiles.into_iter().enumerate().map(|(index, tile)| {
                let on = tile.key == selected;
                let click = on_select.clone();
                let keyed = on_select.clone();
                let click_key = tile.key.clone();
                let keys = keys.clone();
                let (light, dark) = tile_colors(&tile, &c);
                let ring = if on { c.primary } else { c.ring };
                div()
                    .id(SharedString::from(format!("{id}-{}", tile.key)))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(c.space(6.0))
                    .child(
                        div()
                            .p(px(2.0))
                            .border_2()
                            .rounded(px(c.radius.lg))
                            .border_color(if on {
                                c.primary
                            } else {
                                gpui::transparent_black()
                            })
                            .child(mock(&c, tile.kind, light, dark)),
                    )
                    .child(
                        div()
                            .text_size(c.control_space(12.0))
                            .text_color(if on { c.fg } else { c.muted })
                            .when(on, |label| label.font_weight(FontWeight::BOLD))
                            .child(tile.label),
                    )
                    .when(!disabled, move |tile| {
                        tile.cursor_pointer()
                            .tab_index(0)
                            .focus(move |style| style.bg(ring.opacity(0.12)))
                            .when_some(click, move |tile, handler| {
                                tile.on_click(move |_: &ClickEvent, window, cx| {
                                    handler(&click_key, window, cx)
                                })
                            })
                            .on_key_down(move |event, window, cx| {
                                let key = event.keystroke.key.as_str();
                                if matches!(key, "enter" | "space") {
                                    if let Some(handler) = &keyed {
                                        handler(&keys[index], window, cx);
                                    }
                                    cx.stop_propagation();
                                    return;
                                }
                                let Some(next) = step_selection(index, keys.len(), key) else {
                                    return;
                                };
                                if let Some(handler) = &keyed {
                                    handler(&keys[next], window, cx);
                                }
                                if matches!(key, "right" | "down") {
                                    window.focus_next();
                                } else {
                                    window.focus_prev();
                                }
                                cx.stop_propagation();
                            })
                    })
            }))
    }
}

/// The light/dark mock colours of `tile`.
fn tile_colors(tile: &PreviewTile, c: &Palette) -> (TileMock, TileMock) {
    if let Some(colors) = tile.colors {
        return colors;
    }
    match tile.kind {
        PreviewKind::ColorScheme(_) | PreviewKind::Theme => (
            TileMock::from_theme(&Theme::light()),
            TileMock::from_theme(&Theme::dark()),
        ),
        PreviewKind::InterfaceStyle(_) | PreviewKind::PaneLayout(_) => {
            let current = TileMock::from_palette(c);
            (current, current)
        }
    }
}

/// The mini-window for `kind`.
fn mock(c: &Palette, kind: PreviewKind, light: TileMock, dark: TileMock) -> Div {
    let (width, height) = (c.space(MOCK_WIDTH), c.space(MOCK_HEIGHT));
    let frame = div()
        .relative()
        .flex_shrink_0()
        .w(width)
        .h(height)
        .overflow_hidden()
        .rounded(px(c.radius.md))
        .border_1()
        .border_color(c.line_subtle);
    match kind {
        PreviewKind::ColorScheme(ColorSchemeChoice::Light) => {
            frame.child(window_mock(c, width, height, light, None, None))
        }
        PreviewKind::ColorScheme(ColorSchemeChoice::Dark) => {
            frame.child(window_mock(c, width, height, dark, None, None))
        }
        PreviewKind::ColorScheme(ColorSchemeChoice::Auto) | PreviewKind::Theme => {
            split_mock(frame, c, width, height, light, dark)
        }
        PreviewKind::InterfaceStyle(style) => {
            frame.child(window_mock(c, width, height, light, Some(style), None))
        }
        PreviewKind::PaneLayout(layout) => {
            frame.child(window_mock(c, width, height, light, None, Some(layout)))
        }
    }
}

/// Left half `light`, right half `dark`: two clipped columns of the same mock.
fn split_mock(
    frame: Div,
    c: &Palette,
    width: Pixels,
    height: Pixels,
    light: TileMock,
    dark: TileMock,
) -> Div {
    let half = width / 2.0;
    let side = |colors: TileMock, shift: Pixels| {
        div()
            .relative()
            .flex_shrink_0()
            .w(half)
            .h_full()
            .overflow_hidden()
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left(shift)
                    .child(window_mock(c, width, height, colors, None, None)),
            )
    };
    frame
        .flex()
        .flex_row()
        .child(side(light, px(0.0)))
        .child(side(dark, -half))
}

/// A tiny application window: titlebar, sidebar and content, drawn from
/// `colors`. `style` and `layout` vary the material and the pane arrangement.
fn window_mock(
    c: &Palette,
    width: Pixels,
    height: Pixels,
    colors: TileMock,
    style: Option<InterfaceStyleChoice>,
    layout: Option<PaneLayoutStyle>,
) -> Div {
    let glassy = style == Some(InterfaceStyleChoice::Glassy);
    // Glassy surfaces let a tinted backdrop show through.
    let surface = |color: Hsla| if glassy { color.opacity(0.55) } else { color };
    let line = |w: f32, color: Hsla| {
        div()
            .h(c.space(3.0))
            .w(c.space(w))
            .rounded(px(c.radius.sm))
            .bg(color)
    };
    let titlebar = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(c.space(3.0))
        .flex_shrink_0()
        .h(c.space(11.0))
        .px(c.space(5.0))
        .bg(surface(colors.chrome))
        .children((0..3).map(|_| {
            div()
                .size(c.space(3.0))
                .rounded_full()
                .bg(colors.muted.opacity(0.7))
        }));
    let sidebar = div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .gap(c.space(4.0))
        .w(c.space(28.0))
        .p(c.space(5.0))
        .bg(surface(colors.sidebar))
        .child(line(16.0, colors.fg.opacity(0.55)))
        .child(line(12.0, colors.muted.opacity(0.5)))
        .child(line(14.0, colors.muted.opacity(0.5)));
    let content = match layout {
        Some(PaneLayoutStyle::Comfortable) => div()
            .flex()
            .flex_row()
            .flex_1()
            .gap(c.space(3.0))
            .p(c.space(3.0))
            .bg(colors.chrome)
            .children((0..2).map(|_| pane_block(c, colors, true))),
        Some(PaneLayoutStyle::Compact) => div()
            .flex()
            .flex_row()
            .flex_1()
            .bg(colors.bg)
            .child(pane_block(c, colors, false))
            .child(div().w(px(1.0)).h_full().bg(colors.line))
            .child(pane_block(c, colors, false)),
        None => div()
            .flex()
            .flex_col()
            .flex_1()
            .gap(c.space(4.0))
            .p(c.space(6.0))
            .bg(if glassy {
                colors.bg.opacity(0.7)
            } else {
                colors.bg
            })
            .child(line(40.0, colors.fg.opacity(0.7)))
            .child(line(52.0, colors.muted.opacity(0.5)))
            .child(line(34.0, colors.muted.opacity(0.5)))
            .child(
                div()
                    .h(c.space(5.0))
                    .w(c.space(20.0))
                    .rounded(px(c.radius.sm))
                    .bg(colors.accent),
            ),
    };
    div()
        .flex()
        .flex_col()
        .w(width)
        .h(height)
        .bg(if glassy {
            colors.accent.opacity(0.35)
        } else {
            colors.bg
        })
        .child(titlebar)
        .child(
            div()
                .flex()
                .flex_row()
                .flex_1()
                .child(sidebar)
                .child(content),
        )
}

/// One pane of a layout mock: filled, optionally rounded with a ring.
fn pane_block(c: &Palette, colors: TileMock, framed: bool) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .gap(c.space(3.0))
        .p(c.space(4.0))
        .bg(colors.bg)
        .when(framed, |pane| {
            pane.rounded(px(c.radius.sm))
                .border_1()
                .border_color(colors.line)
        })
        .child(
            div()
                .h(c.space(3.0))
                .w(c.space(22.0))
                .rounded(px(c.radius.sm))
                .bg(colors.fg.opacity(0.6)),
        )
        .child(
            div()
                .h(c.space(3.0))
                .w(c.space(16.0))
                .rounded(px(c.radius.sm))
                .bg(colors.muted.opacity(0.5)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    fn every_kind() -> Vec<PreviewKind> {
        vec![
            PreviewKind::ColorScheme(ColorSchemeChoice::Auto),
            PreviewKind::ColorScheme(ColorSchemeChoice::Light),
            PreviewKind::ColorScheme(ColorSchemeChoice::Dark),
            PreviewKind::InterfaceStyle(InterfaceStyleChoice::Classic),
            PreviewKind::InterfaceStyle(InterfaceStyleChoice::Glassy),
            PreviewKind::PaneLayout(PaneLayoutStyle::Comfortable),
            PreviewKind::PaneLayout(PaneLayoutStyle::Compact),
            PreviewKind::Theme,
        ]
    }

    #[test]
    fn group_builds_every_kind_selected_idle_and_disabled() {
        let c = test_palette();
        for disabled in [false, true] {
            for selected in ["0", "none"] {
                let mut group = preview_tile_group("tiles", c, selected).disabled(disabled);
                for (index, kind) in every_kind().into_iter().enumerate() {
                    let tile = preview_tile(index.to_string(), format!("Tile {index}"), kind);
                    group = group.tile(if kind == PreviewKind::Theme {
                        tile.colors(TileMock::from_palette(&c), TileMock::from_palette(&c))
                    } else {
                        tile
                    });
                }
                let _ = group.on_select(|_, _, _| {}).into_element();
            }
        }
        // An empty group is valid.
        let _ = preview_tile_group("empty", c, "x").into_element();
    }

    #[test]
    fn selection_model_resolves_the_selected_tile() {
        let c = test_palette();
        let group = preview_tile_group("g", c, "dark")
            .tile(preview_tile(
                "light",
                "Light",
                PreviewKind::ColorScheme(ColorSchemeChoice::Light),
            ))
            .tile(preview_tile(
                "dark",
                "Dark",
                PreviewKind::ColorScheme(ColorSchemeChoice::Dark),
            ));
        assert_eq!(group.selection().map(SharedString::as_ref), Some("dark"));

        // A selection matching no tile reports nothing instead of the first.
        let none = preview_tile_group("g", c, "sepia").tile(preview_tile(
            "light",
            "Light",
            PreviewKind::ColorScheme(ColorSchemeChoice::Light),
        ));
        assert!(none.selection().is_none());

        let keys: Vec<SharedString> = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(selected_index(&keys, "c"), Some(2));
        assert_eq!(selected_index(&keys, "z"), None);
    }

    #[test]
    fn arrow_keys_step_and_wrap_like_a_radio_group() {
        assert_eq!(step_selection(0, 3, "right"), Some(1));
        assert_eq!(step_selection(0, 3, "down"), Some(1));
        assert_eq!(step_selection(2, 3, "right"), Some(0));
        assert_eq!(step_selection(0, 3, "left"), Some(2));
        assert_eq!(step_selection(1, 3, "up"), Some(0));
        assert_eq!(step_selection(0, 3, "tab"), None);
        assert_eq!(step_selection(0, 0, "right"), None);
        assert_eq!(step_selection(5, 3, "right"), None);
    }

    #[test]
    fn mock_colors_come_from_theme_and_palette() {
        let c = test_palette();
        let from_palette = TileMock::from_palette(&c);
        assert_eq!(from_palette.bg, c.bg);
        let (light, dark) = (
            TileMock::from_theme(&Theme::light()),
            TileMock::from_theme(&Theme::dark()),
        );
        assert_ne!(light.bg, dark.bg);
    }
}
