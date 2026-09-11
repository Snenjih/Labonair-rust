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

use gpui::{div, px, Context, Div, Entity, Hsla, Styled, Window};
use gpui_component::input::Input;

/// Creates a single-line [`InputState`] ready to be stored in `cx.new(..)`.
pub fn text_field(window: &mut Window, cx: &mut Context<InputState>) -> InputState {
    InputState::new(window, cx)
}

/// Builds the renderable [`Input`] element bound to `state`.
pub fn field_input(state: &Entity<InputState>) -> Input {
    Input::new(state)
}

/// A static caret bar for the hand-rolled, single-`String` text fields that
/// predate `InputState` (a view-level `on_key_down` router editing a plain
/// `String`, documented across `command-palette`/`hosts-ui`/`panel-snippets`
/// as out of scope for a full `InputState` migration). These fields only
/// ever append/pop at the end of the string, so the caret always sits right
/// after the current value — callers should render it only while the field
/// actually has keyboard focus, otherwise the field looks alive when it
/// isn't.
pub fn caret(color: Hsla, height: f32) -> Div {
    div().flex_none().w(px(1.5)).h(px(height)).bg(color)
}
