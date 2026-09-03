//! `labonair-gpui-ext` — shared GPUI prelude and helper glue.
//!
//! GPUI's own imports are verbose: nearly every view file in the port opens
//! with the same `use gpui::{div, px, App, AppContext, Context, Entity, …}` plus
//! `use gpui::prelude::*`. This crate collapses that into a single import path
//! so downstream crates (`labonair-ui-kit` and, later, the panel/workspace/shell
//! crates) pull the recurring surface from one place.
//!
//! It is a leaf crate: it depends only on `gpui` and adds no behaviour of its
//! own — it is pure re-export. Anything GPUI-specific but genuinely shared
//! (helper traits, newtypes) lands here as the rework proceeds; nothing
//! speculative is added ahead of a real second caller.

/// The recurring GPUI import surface.
///
/// Bring it in with `use labonair_gpui_ext::prelude::*;`. It re-exports
/// `gpui::prelude::*` (the element/builder traits, `FluentBuilder`, …) plus the
/// concrete types that showed up in a majority of the port's `use gpui::{…}`
/// lines.
pub mod prelude {
    pub use gpui::prelude::*;

    pub use gpui::{
        div, px, rems, AnyElement, App, AppContext, ClickEvent, Context, Div, Entity, EventEmitter,
        FocusHandle, Focusable, Global, Hsla, InteractiveElement, IntoElement, KeyDownEvent,
        MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Render, SharedString, Stateful,
        StatefulInteractiveElement, Styled, Task, Window,
    };
}
