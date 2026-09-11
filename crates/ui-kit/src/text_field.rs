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

use gpui::{div, px, Context, Div, Entity, Hsla, Styled, Task, Timer, Window};
use gpui_component::input::Input;
use std::time::Duration;

/// Creates a single-line [`InputState`] ready to be stored in `cx.new(..)`.
pub fn text_field(window: &mut Window, cx: &mut Context<InputState>) -> InputState {
    InputState::new(window, cx)
}

/// Builds the renderable [`Input`] element bound to `state`.
pub fn field_input(state: &Entity<InputState>) -> Input {
    Input::new(state)
}

/// A caret bar for the hand-rolled, single-`String` text fields that predate
/// `InputState` (a view-level `on_key_down` router editing a plain `String`,
/// documented across `command-palette`/`hosts-ui`/`panel-snippets` as out of
/// scope for a full `InputState` migration). These fields only ever
/// append/pop at the end of the string, so the caret always sits right after
/// the current value — callers should render it only while the field
/// actually has keyboard focus *and* [`BlinkCursor::visible`] is true,
/// otherwise the field looks alive when it isn't.
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
