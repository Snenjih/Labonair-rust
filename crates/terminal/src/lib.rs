//! Labonair terminal engine (alacritty_terminal) and GPUI renderer.
//!
//! * [`palette`] — theme → engine color bridge (T02-004).
//! * [`engine`] — render-free VTE emulation core (T03-001).
//! * [`session`] — local PTY-backed terminal sessions (T03-001).
//! * [`input`] — keyboard/mouse → terminal byte-sequence mapping (T03-003).
//! * [`shell_integration`] — OSC 7 / OSC 133 shell-integration rc-file bootstrap
//!   + CWD / title / prompt-state tracking (T03-004).
//!
//! * [`render`] — pure grid → style-run batching + resize math for the GPUI
//!   renderer (T03-002). The GPUI element itself lives in `labonair-ui`.

pub mod engine;
pub mod input;
pub mod palette;
pub mod render;
pub mod session;
pub mod shell_integration;

pub use alacritty_terminal::grid::Scroll;
pub use alacritty_terminal::vte::ansi::{CursorShape, Rgb};
pub use engine::{
    ModeState, PromptPhase, RenderableCell, RenderableCursor, RenderableScreen, SelectionSpan,
    SessionMetadata, TermDimensions, TerminalEmulator, TerminalEvent, DEFAULT_SCROLLBACK_LINES,
};
pub use input::{
    key_to_bytes, mouse_report, paste_payload, wheel_action, Key, KeyInput, Modifiers, MouseButton,
    MouseEventKind, MouseInput, NamedKey, WheelAction, WheelInput,
};
pub use palette::{ansi_self_test, TerminalColors};
pub use render::{batch_runs, grid_size, RunStyle, StyledRun};
pub use session::{SessionOptions, TerminalContext, TerminalSession};
pub use shell_integration::Shell;
