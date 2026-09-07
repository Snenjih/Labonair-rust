//! Shared shell-integration protocol payloads for local and remote terminals.
//!
//! The scripts are deliberately UI- and transport-free. Local PTY spawning
//! and remote SSH bootstrap code both consume this one source of truth so
//! OSC 7/133 behavior cannot drift between terminal kinds.

pub const ZSHENV: &str = include_str!("scripts/zshenv.zsh");
pub const ZPROFILE: &str = include_str!("scripts/zprofile.zsh");
pub const ZLOGIN: &str = include_str!("scripts/zlogin.zsh");
pub const ZSHRC: &str = include_str!("scripts/zshrc.zsh");
pub const BASHRC: &str = include_str!("scripts/bashrc.bash");
