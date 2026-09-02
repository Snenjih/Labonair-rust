//! App-wide error catalog (T15-002).
//!
//! [`LabonairError`] is the single structured error type the backend surfaces
//! to the UI layer. Every variant carries a human-readable detail string and,
//! through [`LabonairError::category`], [`LabonairError::user_message`] and
//! [`LabonairError::recovery`], maps to:
//!
//! * a coarse [`ErrorCategory`] (SSH, SFTP, filesystem, Git, AI, terminal,
//!   settings, network) so the UI can group / route failures consistently,
//! * a friendly, actionable message (never a raw error code or stack trace),
//! * an optional [`RecoveryHint`] the UI can turn into a button (reconnect,
//!   retry, resend, …).
//!
//! Raw transport / library error strings are funnelled through
//! [`LabonairError::classify`] which replaces the several near-identical
//! ad-hoc `to_lowercase().contains(...)` matchers that had accumulated in the
//! SSH and SFTP modules.

use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize, PartialEq)]
#[serde(tag = "code", content = "message")]
pub enum LabonairError {
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
    #[error("Network connection lost: {0}")]
    NetworkError(String),
    #[error("Host key verification failed: {0}")]
    HostKeyMismatch(String),
    #[error("I/O error: {0}")]
    IoError(String),
    #[error("Internal error: {0}")]
    Internal(String),

    // --- catalog additions (T15-002) ---
    /// A required session / connection is not established (or was already
    /// removed). Distinct from [`LabonairError::NetworkError`]: nothing failed
    /// on the wire, there simply is no live session to use.
    #[error("Not connected: {0}")]
    NotConnected(String),
    /// A referenced resource (file, host, credential, snippet, repo, …) does
    /// not exist.
    #[error("Not found: {0}")]
    NotFound(String),
    /// The OS or remote refused the operation for lack of permission.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    /// User-supplied input failed validation before any operation ran.
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    /// An operation exceeded its time budget.
    #[error("Timed out: {0}")]
    Timeout(String),
    /// The operation cannot proceed because of a conflicting state
    /// (merge conflict, name already taken, concurrent modification).
    #[error("Conflict: {0}")]
    Conflict(String),
}

/// Coarse grouping used by the UI to route / label a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCategory {
    Ssh,
    Sftp,
    Fs,
    Git,
    Ai,
    Terminal,
    Settings,
    Network,
    Other,
}

/// A follow-up action the UI can offer the user after a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryHint {
    /// Re-establish a dropped SSH/SFTP connection.
    Reconnect,
    /// Re-run the exact same operation (transient failure).
    Retry,
    /// Re-send the last AI prompt.
    Resend,
    /// Open a diagnostics view (e.g. `git status` / raw command output).
    Diagnose,
    /// Go back to the previous screen / undo the navigation.
    GoBack,
    /// Fix the highlighted form field and submit again.
    FixInput,
    /// Review the relevant settings pane.
    CheckSettings,
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

impl LabonairError {
    /// Classify a raw error string (from russh, russh-sftp, std::io, a shelled
    /// `git`, …) into the closest structured variant. The input string is
    /// preserved as the variant detail so no diagnostic information is lost.
    pub fn classify(msg: impl Into<String>) -> LabonairError {
        let s = msg.into();
        let lower = s.to_lowercase();

        if lower == "passphrase_required"
            || contains_any(
                &lower,
                &[
                    "authentication failed",
                    "not authenticated",
                    "auth fail",
                    "permission denied (publickey",
                    "incorrect passphrase",
                    "wrong passphrase",
                ],
            )
        {
            return LabonairError::AuthFailed(s);
        }

        if contains_any(
            &lower,
            &[
                "host key",
                "hostkey",
                "host-key",
                "key mismatch",
                "user rejected host",
                "not yet trusted",
            ],
        ) {
            return LabonairError::HostKeyMismatch(s);
        }

        if contains_any(&lower, &["timed out", "timeout", "deadline"]) {
            return LabonairError::Timeout(s);
        }

        if contains_any(
            &lower,
            &[
                "tcp connect",
                "network",
                "connection reset",
                "connection refused",
                "connection closed",
                "broken pipe",
                "no route to host",
                "transport read",
                "transport write",
                "unexpected eof",
                "end of file",
                "(eof)",
            ],
        ) {
            return LabonairError::NetworkError(s);
        }

        if lower.contains("no ssh session")
            || lower.contains("no sftp session")
            || lower.contains("not connected")
        {
            return LabonairError::NotConnected(s);
        }

        if lower.contains("permission denied") || lower.contains("access is denied") {
            return LabonairError::PermissionDenied(s);
        }

        if lower.contains("no such file")
            || lower.contains("not found")
            || lower.contains("does not exist")
            || lower.contains("cannot find")
        {
            return LabonairError::NotFound(s);
        }

        if lower.contains("conflict")
            || lower.contains("already exists")
            || lower.contains("would be overwritten")
        {
            return LabonairError::Conflict(s);
        }

        LabonairError::Internal(s)
    }

    /// The subsystem this failure belongs to.
    pub fn category(&self) -> ErrorCategory {
        match self {
            LabonairError::AuthFailed(_) | LabonairError::HostKeyMismatch(_) => ErrorCategory::Ssh,
            LabonairError::NetworkError(_) | LabonairError::Timeout(_) => ErrorCategory::Network,
            LabonairError::NotConnected(_) => ErrorCategory::Ssh,
            LabonairError::IoError(_)
            | LabonairError::NotFound(_)
            | LabonairError::PermissionDenied(_) => ErrorCategory::Fs,
            LabonairError::InvalidInput(_) => ErrorCategory::Settings,
            LabonairError::Conflict(_) => ErrorCategory::Git,
            LabonairError::Internal(_) => ErrorCategory::Other,
        }
    }

    /// A friendly, user-facing sentence: what went wrong and (where possible)
    /// what to do next. Never a bare error code or stack trace.
    pub fn user_message(&self) -> String {
        let (headline, hint) = match self {
            LabonairError::AuthFailed(d) => (
                format!("Authentication failed. {d}"),
                "Check the username, password, key file or passphrase for this host.",
            ),
            LabonairError::HostKeyMismatch(d) => (
                format!("The server's host key could not be verified. {d}"),
                "This can mean the server changed — or a man-in-the-middle. Verify the fingerprint before trusting it.",
            ),
            LabonairError::NetworkError(d) => (
                format!("The connection to the server was lost. {d}"),
                "Check your network and the host address, then reconnect.",
            ),
            LabonairError::Timeout(d) => (
                format!("The operation took too long and was aborted. {d}"),
                "The server may be slow or unreachable — try again.",
            ),
            LabonairError::NotConnected(d) => (
                format!("There is no active connection for this action. {d}"),
                "Reconnect to the host and try again.",
            ),
            LabonairError::PermissionDenied(d) => (
                format!("You don't have permission to do that. {d}"),
                "Check the file or remote permissions and ownership.",
            ),
            LabonairError::NotFound(d) => (
                format!("The item could not be found. {d}"),
                "It may have been moved or deleted since it was last loaded.",
            ),
            LabonairError::InvalidInput(d) => (
                format!("The input is not valid. {d}"),
                "Correct the highlighted field and try again.",
            ),
            LabonairError::Conflict(d) => (
                format!("The action conflicts with the current state. {d}"),
                "Resolve the conflict (or pick another name) and retry.",
            ),
            LabonairError::IoError(d) => (
                format!("A file operation failed. {d}"),
                "Check that the path exists and is writable.",
            ),
            LabonairError::Internal(d) => (
                format!("Something went wrong. {d}"),
                "If this keeps happening, check the logs for details.",
            ),
        };
        format!("{headline} {hint}")
    }

    /// The follow-up action the UI should offer, if any.
    pub fn recovery(&self) -> Option<RecoveryHint> {
        match self {
            LabonairError::NetworkError(_) | LabonairError::NotConnected(_) => {
                Some(RecoveryHint::Reconnect)
            }
            LabonairError::Timeout(_) => Some(RecoveryHint::Retry),
            LabonairError::AuthFailed(_) => Some(RecoveryHint::CheckSettings),
            LabonairError::HostKeyMismatch(_) => Some(RecoveryHint::Diagnose),
            LabonairError::Conflict(_) => Some(RecoveryHint::Diagnose),
            LabonairError::InvalidInput(_) => Some(RecoveryHint::FixInput),
            LabonairError::NotFound(_) => Some(RecoveryHint::GoBack),
            LabonairError::PermissionDenied(_)
            | LabonairError::IoError(_)
            | LabonairError::Internal(_) => None,
        }
    }
}

impl From<russh::Error> for LabonairError {
    fn from(e: russh::Error) -> Self {
        LabonairError::classify(e.to_string())
    }
}

impl From<russh_sftp::client::error::Error> for LabonairError {
    fn from(e: russh_sftp::client::error::Error) -> Self {
        LabonairError::classify(e.to_string())
    }
}

impl From<std::io::Error> for LabonairError {
    fn from(e: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match e.kind() {
            ErrorKind::NotFound => LabonairError::NotFound(e.to_string()),
            ErrorKind::PermissionDenied => LabonairError::PermissionDenied(e.to_string()),
            ErrorKind::TimedOut => LabonairError::Timeout(e.to_string()),
            ErrorKind::AlreadyExists => LabonairError::Conflict(e.to_string()),
            ErrorKind::ConnectionReset
            | ErrorKind::ConnectionRefused
            | ErrorKind::ConnectionAborted
            | ErrorKind::BrokenPipe
            | ErrorKind::NotConnected => LabonairError::NetworkError(e.to_string()),
            _ => LabonairError::IoError(e.to_string()),
        }
    }
}

impl From<rusqlite::Error> for LabonairError {
    fn from(e: rusqlite::Error) -> Self {
        match e {
            rusqlite::Error::QueryReturnedNoRows => {
                LabonairError::NotFound("the requested row does not exist".to_string())
            }
            other => LabonairError::Internal(other.to_string()),
        }
    }
}

impl From<serde_json::Error> for LabonairError {
    fn from(e: serde_json::Error) -> Self {
        LabonairError::Internal(e.to_string())
    }
}

impl From<LabonairError> for String {
    fn from(e: LabonairError) -> String {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_conversion_produces_not_found_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = LabonairError::from(io_err);
        assert!(matches!(err, LabonairError::NotFound(_)));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn io_error_permission_denied_maps_to_permission_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "nope");
        assert!(matches!(
            LabonairError::from(io_err),
            LabonairError::PermissionDenied(_)
        ));
    }

    #[test]
    fn io_error_broken_pipe_maps_to_network_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken pipe");
        assert!(matches!(
            LabonairError::from(io_err),
            LabonairError::NetworkError(_)
        ));
    }

    #[test]
    fn rusqlite_no_rows_maps_to_not_found() {
        let err = LabonairError::from(rusqlite::Error::QueryReturnedNoRows);
        assert!(matches!(err, LabonairError::NotFound(_)));
    }

    #[test]
    fn rusqlite_other_error_conversion_produces_internal_variant() {
        let db_err = rusqlite::Error::InvalidQuery;
        let err = LabonairError::from(db_err);
        assert!(matches!(err, LabonairError::Internal(_)));
    }

    #[test]
    fn error_serializes_with_code_tag_and_message_content() {
        let err = LabonairError::AuthFailed("bad password".to_string());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "AuthFailed");
        assert_eq!(json["message"], "bad password");
    }

    #[test]
    fn each_variant_produces_distinct_code_field() {
        let variants: &[(&str, LabonairError)] = &[
            ("AuthFailed", LabonairError::AuthFailed("x".into())),
            ("NetworkError", LabonairError::NetworkError("x".into())),
            (
                "HostKeyMismatch",
                LabonairError::HostKeyMismatch("x".into()),
            ),
            ("IoError", LabonairError::IoError("x".into())),
            ("Internal", LabonairError::Internal("x".into())),
            ("NotConnected", LabonairError::NotConnected("x".into())),
            ("NotFound", LabonairError::NotFound("x".into())),
            (
                "PermissionDenied",
                LabonairError::PermissionDenied("x".into()),
            ),
            ("InvalidInput", LabonairError::InvalidInput("x".into())),
            ("Timeout", LabonairError::Timeout("x".into())),
            ("Conflict", LabonairError::Conflict("x".into())),
        ];
        for (expected_code, err) in variants {
            let json = serde_json::to_value(err).unwrap();
            assert_eq!(json["code"].as_str().unwrap(), *expected_code);
        }
    }

    #[test]
    fn partial_eq_works_for_same_variants() {
        assert_eq!(
            LabonairError::AuthFailed("x".into()),
            LabonairError::AuthFailed("x".into())
        );
    }

    #[test]
    fn partial_eq_distinguishes_different_variants() {
        assert_ne!(
            LabonairError::AuthFailed("x".into()),
            LabonairError::Internal("x".into())
        );
    }

    #[test]
    fn classify_recognizes_auth_failures() {
        for m in [
            "Authentication failed",
            "not authenticated",
            "passphrase_required",
            "Permission denied (publickey).",
            "incorrect passphrase for key",
        ] {
            assert!(
                matches!(LabonairError::classify(m), LabonairError::AuthFailed(_)),
                "{m:?} should be AuthFailed"
            );
        }
    }

    #[test]
    fn classify_recognizes_host_key_problems() {
        for m in [
            "Host key mismatch",
            "user rejected host key",
            "server key is not yet trusted",
        ] {
            assert!(
                matches!(
                    LabonairError::classify(m),
                    LabonairError::HostKeyMismatch(_)
                ),
                "{m:?} should be HostKeyMismatch"
            );
        }
    }

    #[test]
    fn classify_recognizes_network_and_timeout() {
        assert!(matches!(
            LabonairError::classify("TCP connect error: Connection refused"),
            LabonairError::NetworkError(_)
        ));
        assert!(matches!(
            LabonairError::classify("Broken pipe (os error 32)"),
            LabonairError::NetworkError(_)
        ));
        assert!(matches!(
            LabonairError::classify("operation timed out"),
            LabonairError::Timeout(_)
        ));
    }

    #[test]
    fn classify_recognizes_not_connected_permission_not_found_conflict() {
        assert!(matches!(
            LabonairError::classify("no SSH session for this host — reconnect and try again"),
            LabonairError::NotConnected(_)
        ));
        assert!(matches!(
            LabonairError::classify("Permission denied"),
            LabonairError::PermissionDenied(_)
        ));
        assert!(matches!(
            LabonairError::classify("No such file or directory"),
            LabonairError::NotFound(_)
        ));
        assert!(matches!(
            LabonairError::classify("Automatic merge failed; fix conflicts"),
            LabonairError::Conflict(_)
        ));
    }

    #[test]
    fn classify_falls_back_to_internal() {
        assert!(matches!(
            LabonairError::classify("something weird happened"),
            LabonairError::Internal(_)
        ));
    }

    #[test]
    fn classify_preserves_the_original_detail_string() {
        let raw = "Authentication failed for user 'root'";
        assert_eq!(
            LabonairError::classify(raw).to_string(),
            format!("Authentication failed: {raw}")
        );
    }

    #[test]
    fn categories_are_stable_per_variant() {
        assert_eq!(
            LabonairError::AuthFailed("x".into()).category(),
            ErrorCategory::Ssh
        );
        assert_eq!(
            LabonairError::NetworkError("x".into()).category(),
            ErrorCategory::Network
        );
        assert_eq!(
            LabonairError::NotFound("x".into()).category(),
            ErrorCategory::Fs
        );
        assert_eq!(
            LabonairError::Conflict("x".into()).category(),
            ErrorCategory::Git
        );
        assert_eq!(
            LabonairError::InvalidInput("x".into()).category(),
            ErrorCategory::Settings
        );
    }

    #[test]
    fn user_message_is_friendly_and_carries_detail() {
        let msg = LabonairError::NetworkError("Connection reset by peer".into()).user_message();
        assert!(msg.contains("connection to the server was lost"));
        assert!(msg.contains("Connection reset by peer"));
        assert!(msg.contains("reconnect"));
        // never leak the serde code tag
        assert!(!msg.contains("NetworkError"));
    }

    #[test]
    fn every_variant_has_a_non_empty_user_message() {
        let all = [
            LabonairError::AuthFailed("d".into()),
            LabonairError::NetworkError("d".into()),
            LabonairError::HostKeyMismatch("d".into()),
            LabonairError::IoError("d".into()),
            LabonairError::Internal("d".into()),
            LabonairError::NotConnected("d".into()),
            LabonairError::NotFound("d".into()),
            LabonairError::PermissionDenied("d".into()),
            LabonairError::InvalidInput("d".into()),
            LabonairError::Timeout("d".into()),
            LabonairError::Conflict("d".into()),
        ];
        for e in all {
            assert!(e.user_message().len() > 20, "{e:?}");
        }
    }

    #[test]
    fn recovery_hints_match_expectations() {
        assert_eq!(
            LabonairError::NetworkError("x".into()).recovery(),
            Some(RecoveryHint::Reconnect)
        );
        assert_eq!(
            LabonairError::NotConnected("x".into()).recovery(),
            Some(RecoveryHint::Reconnect)
        );
        assert_eq!(
            LabonairError::Timeout("x".into()).recovery(),
            Some(RecoveryHint::Retry)
        );
        assert_eq!(
            LabonairError::InvalidInput("x".into()).recovery(),
            Some(RecoveryHint::FixInput)
        );
        assert_eq!(LabonairError::Internal("x".into()).recovery(), None);
    }
}
