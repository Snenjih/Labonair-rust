//! GPUI presentation and interaction for the Editor capability.
//!
//! Workspace creates and hosts [`EditorView`] instances, while the editor
//! owner keeps text behavior, rendering, settings application, and commands
//! in this sibling UI crate.

mod editor;
mod multibuffer;
mod syntax_theme;

pub use editor::{EditorBufferSnapshot, EditorEvent, EditorView};
pub use multibuffer::{MultiBufferEvent, MultiBufferView};
