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
    apply_macos_update, current_app_bundle, download_update, fetch_manifest, manifest_update,
    record_check_now, relaunch, should_auto_check, verify_update, AvailableUpdate,
    DownloadProgress, SemVer, UpdateManifest, UpdatePlatform, CHECK_INTERVAL, CURRENT_VERSION,
    DEFAULT_UPDATE_ENDPOINT, UPDATE_PUBLIC_KEY, UPDATE_TARGET,
};

pub type AppResult<T> = std::result::Result<T, AppError>;
