//! Shared primitive/component layer.
//!
//! Block A of the feature-parity audit (`vergleichsbericht-subagent-3.md`,
//! `-4.md`): the reference app (`reference-src/src/components/ui/**`) drives one
//! consistent visual language through a cva-based primitive set. The port had
//! none — every view hand-rolled its own `btn` and every "field" was a
//! focus-tracking `div`. This module provides the shared replacements, built on
//! `gpui-component` where it fits and on the local [`crate::theme::ThemeStore`]
//! tokens where 1:1 reference values matter (button radii/heights, icon sizes).

mod button;
mod context_menu;
mod icon;
mod text_field;

pub use button::{button, ButtonSize, ButtonVariant, DISABLED_OPACITY};
pub use context_menu::{context_menu, MenuClick, MenuItem};
pub use icon::{file_icon, folder_icon, IconName};
pub use text_field::{field_input, text_field, InputEvent, InputState};

// gpui-component primitives re-exported for later blocks (Select, Dropdown,
// Dialog, Tooltip, Switch, Badge, ContextMenu). Kept here so call sites import
// from `crate::components::*` and can be swapped without touching them.
pub use gpui_component::{badge::Badge, switch::Switch, tooltip::Tooltip};
