//! Labonair terminal engine (alacritty_terminal) and GPUI renderer.
//!
//! Populated by Phase 02 (T03-*). T02-004 adds the theme → engine color bridge
//! ([`palette`]).

pub mod palette;

pub use palette::{ansi_self_test, TerminalColors};
