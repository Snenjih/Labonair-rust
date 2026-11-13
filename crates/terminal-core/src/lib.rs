//! GPUI-free terminal core: the alacritty-based emulator, local PTY sessions,
//! scrollback persistence and shell-integration bootstrap.
//!
//! This crate must stay free of GPUI so the `labonair-host` daemon can depend
//! on it. The app-facing crate `labonair-terminal` re-exports everything here.
//!
//! * [`palette`] — engine color model ([`TerminalColors`]).
//! * [`engine`] — render-free VTE emulation core.
//! * [`session`] — local PTY-backed terminal sessions + the remote-feed seam.
//! * [`scrollback`] — compressed scrollback persistence.
//! * [`shell_integration`] — OSC 7 / OSC 133 rc-file bootstrap.

pub mod engine;
pub mod palette;
pub mod scrollback;
pub mod session;
pub mod shell_integration;

pub use alacritty_terminal::grid::Scroll;
pub use alacritty_terminal::vte::ansi::{CursorShape, Rgb};
pub use engine::{
    EmulatorConfig, ModeState, PromptPhase, RenderableCell, RenderableCursor, RenderableScreen,
    SelectionSpan, SessionMetadata, TermDimensions, TerminalEmulator, TerminalEvent,
    DEFAULT_SCROLLBACK_LINES,
};
pub use palette::{ansi_self_test, TerminalColors};
pub use session::{
    ForegroundProcess, RemoteFeed, RemoteResizer, RemoteSession, RemoteWriter, SessionAccess,
    SessionOptions, TerminalContext, TerminalSession,
};
pub use shell_integration::Shell;
