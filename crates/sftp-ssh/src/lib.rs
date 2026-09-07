//! Concrete russh-sftp integration for the UI-free SFTP capability.
//!
//! The crate owns SFTP session setup and the capability contract adapters.
//! Host lookup, secrets, and SSH transport state are supplied explicitly at
//! the composition boundary; no application-wide backend facade is required.

pub mod connection;
pub mod contract;
