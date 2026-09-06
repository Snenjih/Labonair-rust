//! `labonair-ui-kit` — the shared design-system crate.
//!
//! Block A of the feature-parity audit (`vergleichsbericht-subagent-3.md`,
//! `-4.md`): the reference app (`reference-src/src/components/ui/**`) drives one
//! consistent visual language through a cva-based primitive set. The port had
//! none — every view hand-rolled its own `btn` and every "field" was a
//! focus-tracking `div`. This crate holds the shared replacements, built on
//! `gpui-component` where it fits and on the [`theme::UiTheme`] token accessor
//! where 1:1 reference values matter.
//!
//! T20-001 grew it from the four extracted primitives (`button`,
//! `context_menu`, `icon`, `text_field` + `popover`) into the full set the
//! views assemble from — see `docs/architecture.md` §"UI-Kit" for the
//! inventory that decided which primitives exist.
//!
//! ## Conventions
//!
//! * **Tokens, never literals.** Every primitive is styled from a
//!   [`Palette`] — a `Copy` snapshot of the `labonair-theme` tokens built once
//!   per render with [`Palette::from_theme`]. No primitive may hardcode a
//!   colour (Critical Rule 3). The two "one colour in, one line out" helpers
//!   ([`divider`], [`indicator`]) take a bare `Hsla` instead, since a whole
//!   palette would be noise.
//! * **`Size` / `Variant` enums.** Sizes are named `Xs`/`Sm`/`Md` (plus the
//!   reference's `Lg`/`Icon*` where `button.tsx` has them); variants follow the
//!   reference's cva variant names.
//! * **`RenderOnce`-shaped structs for >2 fields** — [`ListItem`],
//!   [`NumberField`], [`Checkbox`], [`SegmentedControl`], [`Banner`] are
//!   builder structs implementing [`gpui::IntoElement`]; everything smaller is
//!   a plain builder fn returning a `Div`/`Stateful<Div>` the caller keeps
//!   chaining on.
//! * **`disabled` dims to [`DISABLED_OPACITY`] and drops the handler**, so a
//!   disabled control is inert, not just faint.
//! * **State stays with the caller.** `Disclosure`, `Select`, `ToggleButton`
//!   and friends render a flag and fire `on_click`/`on_select`; they never own
//!   the flag. This is what lets them be rebuilt every frame inside a view's
//!   `render`.
//!
//! ## `gpui-component`
//!
//! `gpui-component` 0.5.1 ships many of these (checkbox, divider, kbd, select,
//! tab, …), but every one of them styles itself from **its own** `cx.theme()`
//! global, which the app never syncs to `labonair-theme` — wrapping them would
//! silently bypass our tokens. It is therefore used only where the *behaviour*
//! is the hard part and colours are incidental: [`InputState`]/[`field_input`]
//! (caret, selection, IME, undo), [`Badge`], [`Switch`], [`Tooltip`].

mod banner;
mod button;
mod checkbox;
mod context_menu;
mod density;
mod disclosure;
mod divider;
#[cfg(any(debug_assertions, feature = "gallery"))]
mod gallery;
mod git_change_row;
mod icon;
mod indicator;
mod kbd;
mod list;
mod number_field;
mod palette;
mod popover;
mod segmented;
mod select;
mod stack;
#[cfg(test)]
mod test_support;
mod text_field;
pub mod theme;
mod toggle;
mod tree_row;

pub use banner::{banner, Banner, Severity};
pub use button::{button, button_no_hover, ButtonSize, ButtonVariant, DISABLED_OPACITY};
pub use checkbox::{checkbox, Checkbox};
pub use context_menu::{context_menu, popover_menu, MenuClick, MenuItem};
pub use density::Density;
pub use disclosure::disclosure;
pub use divider::{divider, Axis};
#[cfg(any(debug_assertions, feature = "gallery"))]
pub use gallery::{open_gallery_window, Gallery};
pub use git_change_row::{git_change_row, GitChangeRow, StageState};
pub use icon::{
    chevron_icon_path, file_icon_path, folder_icon_path, icon_for_path, svg_path, IconName,
};
pub use indicator::{indicator, IndicatorSize};
pub use kbd::{kbd, kbd_row, keybinding_hint};
pub use list::{list_header, list_separator, ListItem};
pub use number_field::{number_field, step_value, NumberField};
pub use palette::Palette;
pub use popover::popover;
pub use segmented::{segmented_control, SegmentSize, SegmentVariant, SegmentedControl};
pub use select::{select_popover, select_trigger, selected_label, SelectOption};
pub use stack::{h_stack, v_stack};
pub use text_field::{field_input, text_field, InputEvent, InputState};
pub use theme::{ActiveThemeExt, UiTheme};
pub use toggle::{icon_toggle_button, toggle_base, ToggleSize, ToggleVariant};
pub use tree_row::{tree_row, TreeRow, TreeRowState, TREE_INDENT_STEP};

// gpui-component primitives re-exported where their behaviour (not their
// styling) is what we want. Kept here so call sites import from
// `labonair_ui_kit::*` and can be swapped without touching them.
pub use gpui_component::{badge::Badge, switch::Switch, tooltip::Tooltip};

/// Everything a view needs in one `use`.
///
/// ```ignore
/// use labonair_ui_kit::prelude::*;
/// ```
pub mod prelude {
    pub use crate::ActiveThemeExt;
    pub use crate::{
        banner, button, checkbox, chevron_icon_path, context_menu, disclosure, divider,
        file_icon_path, folder_icon_path, git_change_row, h_stack, icon_for_path,
        icon_toggle_button, indicator, kbd, kbd_row, keybinding_hint, list_header, list_separator,
        number_field, popover, popover_menu, segmented_control, select_popover, select_trigger,
        selected_label, svg_path, toggle_base, tree_row, v_stack,
    };
    pub use crate::{
        Axis, Badge, Banner, ButtonSize, ButtonVariant, Checkbox, Density, GitChangeRow, IconName,
        IndicatorSize, ListItem, MenuClick, MenuItem, NumberField, Palette, SegmentSize,
        SegmentVariant, SegmentedControl, SelectOption, Severity, StageState, Switch, ToggleSize,
        ToggleVariant, Tooltip, TreeRow, TreeRowState, UiTheme, DISABLED_OPACITY,
    };
}
