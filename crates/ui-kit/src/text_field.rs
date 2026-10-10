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

use gpui::{
    div, prelude::FluentBuilder, px, Context, Div, ElementId, Entity, Hsla, InteractiveElement,
    IntoElement, ParentElement, Stateful, Styled, Task, Timer, Window,
};
use gpui_component::input::Input;
use std::time::Duration;

use crate::palette::Palette;

/// Creates a single-line [`InputState`] ready to be stored in `cx.new(..)`.
pub fn text_field(window: &mut Window, cx: &mut Context<InputState>) -> InputState {
    InputState::new(window, cx)
}

/// Builds the renderable [`Input`] element bound to `state`.
pub fn field_input(state: &Entity<InputState>) -> Input {
    Input::new(state)
}

/// Builds the shared text editor inside a caller-owned input surface.
///
/// The owner composes the surrounding border, background, radius, and focus
/// treatment; this function keeps the native editor itself consistent across
/// settings, search, rename, keymap, and commit inputs.
pub fn text_input(state: &Entity<InputState>, c: Palette) -> Input {
    field_input(state)
        .appearance(false)
        .bordered(false)
        .focus_bordered(false)
        .h_full()
        .w_full()
        .text_color(c.fg)
        .text_size(px(12.0 * c.ui_font_scale))
}

/// Visual state for the shared text-field frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextFieldState {
    #[default]
    Normal,
    Focused,
    Invalid,
    Disabled,
}

/// Shared geometry for text fields used in different UI densities.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextFieldSize {
    /// Compact fields for dense panels and inline forms.
    Compact,
    /// Standard fields for settings, dialogs, and primary forms.
    #[default]
    Standard,
}

/// Shared text-field frame. The feature still owns the `InputState`, value,
/// validation, and commit behavior.
pub fn text_field_surface(
    id: impl Into<ElementId>,
    c: Palette,
    state: TextFieldState,
    input: impl IntoElement,
) -> Stateful<Div> {
    text_field_surface_sized(id, c, state, TextFieldSize::Standard, input)
}

/// Shared text-field frame with an explicit compact or standard control size.
pub fn text_field_surface_sized(
    id: impl Into<ElementId>,
    c: Palette,
    state: TextFieldState,
    size: TextFieldSize,
    input: impl IntoElement,
) -> Stateful<Div> {
    let (height, horizontal_padding, vertical_padding) = match size {
        TextFieldSize::Compact => (24.0, 6.0, 2.0),
        TextFieldSize::Standard => (32.0, 8.0, 4.0),
    };
    text_control_surface(id, c, state, size)
        .h(c.control_space(height))
        .items_center()
        .px(c.control_space(horizontal_padding))
        .py(c.control_space(vertical_padding))
        .child(input)
}

/// Shared growing text-area frame for multiline inputs such as commit
/// messages. The input owns its editing state and preferred height; this frame
/// provides the same border, surface, radius, padding, and focus treatment as
/// single-line fields.
pub fn text_area_surface(
    id: impl Into<ElementId>,
    c: Palette,
    state: TextFieldState,
    input: impl IntoElement,
) -> Stateful<Div> {
    text_control_surface(id, c, state, TextFieldSize::Standard)
        .flex_1()
        .min_h(c.control_space(96.0))
        .items_start()
        .px(c.control_space(10.0))
        .py(c.control_space(8.0))
        .child(input)
}

fn text_control_surface(
    id: impl Into<ElementId>,
    c: Palette,
    state: TextFieldState,
    size: TextFieldSize,
) -> Stateful<Div> {
    let (border, background) = match state {
        TextFieldState::Normal => (c.border, c.input),
        TextFieldState::Focused => (c.ring, c.input),
        TextFieldState::Invalid => (c.error, c.input),
        TextFieldState::Disabled => (c.border, c.muted_bg),
    };

    div()
        .id(id)
        .min_w_0()
        .flex()
        .rounded(px(match size {
            TextFieldSize::Compact => c.radius.sm,
            TextFieldSize::Standard => c.radius.md,
        }))
        .border_1()
        .border_color(border)
        .bg(background)
        .when(state != TextFieldState::Disabled, |field| {
            field.focus(|style| style.border_1().border_color(c.ring))
        })
}

/// A caret bar for the hand-rolled, single-`String` text fields that predate
/// `InputState` (a view-level `on_key_down` router editing a plain `String`,
/// documented across `command-palette`/`hosts-ui`/`panel-snippets` as out of
/// scope for a full `InputState` migration). These fields only ever
/// append/pop at the end of the string, so the caret always sits right after
/// the current value — callers should render it only while the field
/// actually has keyboard focus *and* [`BlinkCursor::visible`] is true,
/// otherwise the field looks alive when it isn't.
///
/// `height` should be close to the field's own ambient `text_size`/`text_sm`/
/// `text_xs` value, not the row's full height — `gpui_component`'s own
/// `Input` cursor uses `0.85 * line_height` (≈ the font size), and a caret
/// noticeably taller than the text it sits next to reads as oversized.
pub fn caret(color: Hsla, height: f32) -> Div {
    div().flex_none().w(px(1.5)).h(px(height)).bg(color)
}

static BLINK_INTERVAL: Duration = Duration::from_millis(500);
static BLINK_PAUSE_DELAY: Duration = Duration::from_millis(300);

/// Drives the on/off blink cycle for [`caret`] on hand-rolled text fields —
/// a small port of `gpui_component`'s own (private) `input::BlinkCursor`,
/// since that type isn't exported. One instance is shared per host view
/// (only one hand-rolled field is ever focused at a time), created with
/// `cx.new(|_| BlinkCursor::new())` and observed with `cx.observe` so the
/// host re-renders on every tick.
///
/// - [`BlinkCursor::start`] when the field gains focus.
/// - [`BlinkCursor::stop`] when it loses focus.
/// - [`BlinkCursor::pause`] on every keystroke, so the caret snaps solid and
///   re-syncs its blink phase instead of possibly being mid-blink-off while
///   the user is actively typing.
pub struct BlinkCursor {
    visible: bool,
    paused: bool,
    epoch: usize,
    _task: Task<()>,
}

impl BlinkCursor {
    pub fn new() -> Self {
        Self {
            visible: false,
            paused: false,
            epoch: 0,
            _task: Task::ready(()),
        }
    }

    pub fn start(&mut self, cx: &mut Context<Self>) {
        self.blink(self.epoch, cx);
    }

    pub fn stop(&mut self, cx: &mut Context<Self>) {
        self.epoch = 0;
        self.visible = false;
        cx.notify();
    }

    /// Snap solid, then resume blinking after a short delay — call this on
    /// every keystroke so the caret doesn't look dead mid-blink while typing.
    pub fn pause(&mut self, cx: &mut Context<Self>) {
        self.paused = true;
        self.visible = true;
        cx.notify();

        let epoch = self.next_epoch();
        self._task = cx.spawn(async move |this, cx| {
            Timer::after(BLINK_PAUSE_DELAY).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| {
                    this.paused = false;
                    this.blink(epoch, cx);
                })
                .ok();
            }
        });
    }

    pub fn visible(&self) -> bool {
        self.paused || self.visible
    }

    fn next_epoch(&mut self) -> usize {
        self.epoch += 1;
        self.epoch
    }

    fn blink(&mut self, epoch: usize, cx: &mut Context<Self>) {
        if self.paused || epoch != self.epoch {
            self.visible = true;
            return;
        }
        self.visible = !self.visible;
        cx.notify();

        let epoch = self.next_epoch();
        self._task = cx.spawn(async move |this, cx| {
            Timer::after(BLINK_INTERVAL).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |this, cx| this.blink(epoch, cx)).ok();
            }
        });
    }
}

impl Default for BlinkCursor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_control_surfaces_build_every_state() {
        let palette = crate::test_support::test_palette();
        for state in [
            TextFieldState::Normal,
            TextFieldState::Focused,
            TextFieldState::Invalid,
            TextFieldState::Disabled,
        ] {
            let _ = text_field_surface("field", palette, state, div().child("value"));
            let _ = text_field_surface_sized(
                "compact-field",
                palette,
                state,
                TextFieldSize::Compact,
                div().child("value"),
            );
            let _ = text_area_surface("area", palette, state, div().child("value"));
        }
    }
}
