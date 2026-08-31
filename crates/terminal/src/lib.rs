//! Labonair terminal engine (alacritty_terminal) and GPUI renderer.
//!
//! * [`palette`] — theme → engine color bridge (T02-004).
//! * [`engine`] — render-free VTE emulation core (T03-001).
//! * [`session`] — local PTY-backed terminal sessions (T03-001).
//!
//! * [`render`] — pure grid → style-run batching + resize math for the GPUI
//!   renderer (T03-002). The GPUI element itself lives in `labonair-ui`.

pub mod engine;
pub mod palette;
pub mod render;
pub mod session;

pub use alacritty_terminal::grid::Scroll;
pub use alacritty_terminal::vte::ansi::{CursorShape, Rgb};
pub use engine::{
    RenderableCell, RenderableCursor, RenderableScreen, SelectionSpan, TermDimensions,
    TerminalEmulator, TerminalEvent, DEFAULT_SCROLLBACK_LINES,
};
pub use palette::{ansi_self_test, TerminalColors};
pub use render::{batch_runs, grid_size, RunStyle, StyledRun};
pub use session::{SessionOptions, TerminalSession};
