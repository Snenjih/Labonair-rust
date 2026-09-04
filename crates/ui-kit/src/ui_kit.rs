//! `labonair-ui-kit` — the shared design-system crate.
//!
//! Block A of the feature-parity audit (`vergleichsbericht-subagent-3.md`,
//! `-4.md`): the reference app (`reference-src/src/components/ui/**`) drives one
//! consistent visual language through a cva-based primitive set. The port had
//! none — every view hand-rolled its own `btn` and every "field" was a
//! focus-tracking `div`. This crate holds the shared replacements, built on
//! `gpui-component` where it fits and on the [`theme::UiTheme`] token accessor
//! where 1:1 reference values matter (button radii/heights, icon sizes).
//!
//! It was extracted verbatim from `crates/ui/src/components/` in T16-002 — the
//! move is behaviour-neutral. The primitive **expansion** (Select, Dropdown,
//! Dialog, Table, markdown renderer, …) happens later in Phase 20.

mod button;
mod context_menu;
mod icon;
mod popover;
mod text_field;
pub mod theme;

pub use button::{button, ButtonSize, ButtonVariant, DISABLED_OPACITY};
pub use context_menu::{context_menu, MenuClick, MenuItem};
pub use icon::{file_icon, folder_icon, IconName};
pub use popover::popover;
pub use text_field::{field_input, text_field, InputEvent, InputState};
pub use theme::UiTheme;

// gpui-component primitives re-exported for later blocks (Select, Dropdown,
// Dialog, Tooltip, Switch, Badge, ContextMenu). Kept here so call sites import
// from `labonair_ui_kit::*` and can be swapped without touching them.
pub use gpui_component::{badge::Badge, switch::Switch, tooltip::Tooltip};
