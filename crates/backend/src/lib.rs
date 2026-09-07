//! Native platform adapters and composition state for capabilities that have
//! not yet completed their extraction. Capability contracts live in their
//! canonical crates; the application root is the only intended constructor of
//! this package's aggregate state.

pub mod app;
pub mod events;
pub mod modules;

pub use app::{App, AppState};
pub use events::{AppEvent, EventBus, EventChannel, RawEvent};
