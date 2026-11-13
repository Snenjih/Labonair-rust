//! Labonair terminal engine (alacritty_terminal) and GPUI renderer.
//!
//! * [`palette`] — theme → engine color bridge (T02-004).
//! * [`engine`] — render-free VTE emulation core (T03-001).
//! * [`session`] — local PTY-backed terminal sessions (T03-001).
//! * [`host`] — session-creation seam ([`SessionHost`]) for the detached host.
//! * [`daemon_host`] — the [`SessionHost`] that talks to the detached host daemon.
//! * [`daemon_ssh`] — the [`SessionHost`] of an SSH pane hosted by the daemon.
//! * [`registry`] — multi-session registry + tab-system seam (T03-005).
//! * [`input`] — keyboard/mouse → terminal byte-sequence mapping (T03-003).
//! * [`shell_integration`] — OSC 7 / OSC 133 shell-integration rc-file bootstrap
//!   + CWD / title / prompt-state tracking (T03-004).
//!
//! * [`render`] — pure grid → style-run batching + resize math for the GPUI
//!   renderer (T03-002). The GPUI element itself lives in `labonair-ui`.

pub mod command_provider;
pub mod daemon_host;
pub mod daemon_ssh;
pub mod host;
pub mod input;
pub mod palette;
pub mod registry;
pub mod render;
pub mod resume;

// GPUI-free engine, PTY session, scrollback and shell integration live in
// `labonair-terminal-core` (shared with the detached session host daemon);
// re-exported so every `labonair_terminal::{engine, session, ...}` path holds.
pub use labonair_terminal_core::{engine, scrollback, session, shell_integration};

pub use daemon_host::{DaemonHost, HostState};
pub use daemon_ssh::HostedSshSession;
#[cfg(unix)]
pub use daemon_ssh::SshHostedHost;
pub use engine::{
    EmulatorConfig, ModeState, PromptPhase, RenderableCell, RenderableCursor, RenderableScreen,
    SelectionSpan, SessionMetadata, TermDimensions, TerminalEmulator, TerminalEvent,
    DEFAULT_SCROLLBACK_LINES,
};
pub use host::{
    BackgroundSession, HostedControl, HostedLink, HostedSession, InProcessHost, SessionHost,
};
pub use input::{
    key_to_bytes, mouse_report, paste_payload, wheel_action, Key, KeyInput, Modifiers, MouseButton,
    MouseEventKind, MouseInput, NamedKey, WheelAction, WheelInput,
};
pub use labonair_terminal_core::{CursorShape, Rgb, Scroll};
pub use palette::{ansi_self_test, TerminalColors, TerminalColorsExt};
pub use registry::{SessionBackend, SessionHandle, SessionId, SessionStatus, TerminalRegistry};
pub use render::{batch_runs, grid_size, RunStyle, StyledRun};
pub use session::{
    ForegroundProcess, RemoteFeed, RemoteResizer, RemoteSession, RemoteWriter, SessionAccess,
    SessionOptions, TerminalContext, TerminalSession,
};
pub use shell_integration::Shell;
