//! `labonair-panel` — the **contracts** shared by the workspace, the panel
//! feature crates and the shell.
//!
//! This crate exists to break the dependency cycle "panels need workspace
//! types, the workspace needs the panel trait". It holds only trait/enum
//! declarations and two small registry containers — no rendering, no
//! persistence, no feature code. Per `docs/architecture.md` §3 it must depend
//! on **no** workspace-track crate (`labonair-workspace`, `labonair-shell`) and
//! on **no** `labonair-panel-*` crate; its only dependencies are `gpui` and
//! `labonair-gpui-ext`.
//!
//! The types here are ported from Zed
//! (`zed-refrence/zed/crates/workspace/src/dock.rs` and
//! `.../status_bar.rs`, plus the extracted `zed-refrence/zed/crates/panel/`
//! crate) and reduced to what Labonair's fixed four-zone layout needs. They are
//! intentionally **unused** until T17-001/003 wire them into the workspace and
//! status bar.

mod dock;
mod status;

pub use dock::{
    AnyPanelHandle, DockPosition, Panel, PanelConstructor, PanelEvent, PanelHandle, PanelIcon,
    PanelRegistration, PanelRegistry,
};
pub use status::{
    AnyStatusItemHandle, StatusItem, StatusItemConstructor, StatusItemHandle, StatusItemHide,
    StatusItemRegistration, StatusItemRegistry, StatusSide,
};
