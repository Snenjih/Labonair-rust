//! Concrete russh-sftp integration for the UI-free SFTP capability.
//!
//! The crate owns SFTP session setup and the capability contract adapters.
//! Host lookup, secrets, and SSH transport state are supplied explicitly at
//! the composition boundary; this crate depends on no aggregate application
//! state.

pub mod connection;
pub mod contract;
