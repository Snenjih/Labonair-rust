//! Unified, human-readable error model for the AI subsystem.

use thiserror::Error;

/// Every failure the provider layer can surface. `Display` strings are meant to
/// be shown to the user directly (Settings → AI, chat error banner).
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AiError {
    #[error("No API key configured for {0}. Open Settings → AI to add one.")]
    MissingKey(String),

    #[error("Authentication failed for {provider} — check the API key. ({detail})")]
    Auth { provider: String, detail: String },

    #[error("{provider} rate limit hit — slow down or try again shortly. ({detail})")]
    RateLimit { provider: String, detail: String },

    #[error("{provider} rejected the request: {detail}")]
    BadRequest { provider: String, detail: String },

    #[error("{provider} had a server error: {detail}")]
    ServerError { provider: String, detail: String },

    #[error("Request to {provider} timed out.")]
    Timeout { provider: String },

    #[error("Could not reach {provider}: {detail}")]
    Network { provider: String, detail: String },

    #[error("Malformed streaming response from {provider}: {detail}")]
    Stream { provider: String, detail: String },

    #[error("Unknown model reference: {0}")]
    UnknownModel(String),

    #[error("No provider instance configured for {0}. Add one in Settings → AI.")]
    NoInstance(String),

    #[error("The response stream was cancelled.")]
    Cancelled,
}

impl AiError {
    /// Map an HTTP status + body snippet into a typed error.
    pub fn from_status(provider: &str, status: u16, body: &str) -> AiError {
        let detail = snippet(body);
        match status {
            401 | 403 => AiError::Auth {
                provider: provider.to_string(),
                detail,
            },
            429 => AiError::RateLimit {
                provider: provider.to_string(),
                detail,
            },
            400 | 404 | 422 => AiError::BadRequest {
                provider: provider.to_string(),
                detail,
            },
            500..=599 => AiError::ServerError {
                provider: provider.to_string(),
                detail,
            },
            _ => AiError::BadRequest {
                provider: provider.to_string(),
                detail: format!("HTTP {status}: {detail}"),
            },
        }
    }

    pub fn from_reqwest(provider: &str, e: &reqwest::Error) -> AiError {
        if e.is_timeout() {
            AiError::Timeout {
                provider: provider.to_string(),
            }
        } else {
            AiError::Network {
                provider: provider.to_string(),
                detail: e.to_string(),
            }
        }
    }
}

/// Trim a provider error body to something displayable — prefers the JSON
/// `error.message` field if present.
fn snippet(body: &str) -> String {
    let body = body.trim();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(msg) = v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .or_else(|| v.get("message").and_then(|m| m.as_str()))
        {
            return truncate(msg, 300);
        }
    }
    if body.is_empty() {
        "(no response body)".to_string()
    } else {
        truncate(body, 300)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_mapping() {
        assert!(matches!(
            AiError::from_status("openai", 401, "{}"),
            AiError::Auth { .. }
        ));
        assert!(matches!(
            AiError::from_status("openai", 429, ""),
            AiError::RateLimit { .. }
        ));
        assert!(matches!(
            AiError::from_status("openai", 400, ""),
            AiError::BadRequest { .. }
        ));
        assert!(matches!(
            AiError::from_status("openai", 503, ""),
            AiError::ServerError { .. }
        ));
    }

    #[test]
    fn extracts_json_error_message() {
        let e = AiError::from_status(
            "anthropic",
            400,
            r#"{"error":{"type":"invalid_request_error","message":"max_tokens too large"}}"#,
        );
        assert!(e.to_string().contains("max_tokens too large"));
    }

    #[test]
    fn empty_body_is_labelled() {
        let e = AiError::from_status("groq", 400, "");
        assert!(e.to_string().contains("no response body"));
    }
}
