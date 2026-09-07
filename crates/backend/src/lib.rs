//! Native platform adapters for capabilities that have not yet completed their
//! extraction. Capability contracts live in their canonical crates; the
//! application composition state is owned by `labonair-shell`.

pub mod events;
pub mod modules;

pub use events::{EventBus, EventChannel, RawEvent};
