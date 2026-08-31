//! Labonair terminal engine (alacritty_terminal) and GPUI renderer.
//!
//! * [`palette`] — theme → engine color bridge (T02-004).
//! * [`engine`] — render-free VTE emulation core (T03-001).
//! * [`session`] — local PTY-backed terminal sessions (T03-001).
//!
//! The GPUI cell renderer lands in T03-002 and consumes
//! [`engine::RenderableScreen`].

pub mod engine;
pub mod palette;
pub mod session;

pub use alacritty_terminal::grid::Scroll;
pub use engine::{
    RenderableCell, RenderableCursor, RenderableScreen, TermDimensions, TerminalEmulator,
    TerminalEvent, DEFAULT_SCROLLBACK_LINES,
};
pub use palette::{ansi_self_test, TerminalColors};
pub use session::{SessionOptions, TerminalSession};
