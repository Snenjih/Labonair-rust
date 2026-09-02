//! Labonair backend: SSH, SFTP, Git, filesystem, PTY, hosts, credentials,
//! snippets, secrets, themes, fonts, backgrounds, scrollback, terminal exec and
//! the MCP bridge. Ported in-process from `reference-src/src-tauri/src/modules/`
//! with all Tauri command/state/event wrappers stripped (see [`app`], [`events`]).

pub mod app;
pub mod events;
pub mod modules;

pub use app::{App, AppState};
pub use events::{AppEvent, EventBus, EventChannel, RawEvent};
pub use modules::errors::LabonairError;
pub use modules::errors::LabonairError as AppError;
pub use modules::errors::{ErrorCategory, RecoveryHint};
pub use modules::updater::{
    AvailableUpdate, SemVer, UpdateManifest, UpdatePlatform, CURRENT_VERSION,
    DEFAULT_UPDATE_ENDPOINT, UPDATE_TARGET,
};

pub type AppResult<T> = std::result::Result<T, AppError>;
