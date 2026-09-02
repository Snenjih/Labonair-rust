//! Real text-input primitive.
//!
//! Wraps `gpui-component`'s `InputState` + `Input` element — a genuine text
//! field with caret, mouse selection, clipboard paste, IME composition and
//! undo/redo. Replaces the port's focus-tracking `div`s that pushed characters
//! one at a time via `on_key_down`.
//!
//! Usage:
//! ```ignore
//! // in a view's `new`:
//! let field = text_field(window, cx).placeholder("New name");
//! let field = cx.new(|cx| field);
//! // in `render`:
//! field_input(&self.field)
//! // reading:
//! self.field.read(cx).value()
//! ```

pub use gpui_component::input::{InputEvent, InputState};

use gpui::{Context, Entity, Window};
use gpui_component::input::Input;

/// Creates a single-line [`InputState`] ready to be stored in `cx.new(..)`.
pub fn text_field(window: &mut Window, cx: &mut Context<InputState>) -> InputState {
    InputState::new(window, cx)
}

/// Builds the renderable [`Input`] element bound to `state`.
pub fn field_input(state: &Entity<InputState>) -> Input {
    Input::new(state)
}
