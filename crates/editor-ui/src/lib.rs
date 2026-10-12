//! GPUI presentation and interaction for the Editor capability.
//!
//! Workspace creates and hosts [`EditorView`] instances, while the editor
//! owner keeps text behavior, rendering, settings application, and commands
//! in this sibling UI crate.

mod editor;
mod multibuffer;
mod status_item;
mod syntax_theme;

pub use editor::{EditorBufferSnapshot, EditorEvent, EditorSidePreviewContent, EditorView};
pub use multibuffer::{MultiBufferEvent, MultiBufferView};
pub use status_item::{status_item_registration, ActiveEditorProvider};
