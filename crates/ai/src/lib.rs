//! Labonair AI provider integration (Phase 10 — T11-001).
//!
//! A pure-Rust replacement for the reference app's Vercel-AI-SDK layer
//! (`reference-src/src/modules/ai/`). Provides:
//!
//! * [`config`] — the provider + model catalog (BYOK cloud + local servers).
//! * [`message`] — the provider-agnostic chat interface (messages, tools,
//!   generation config) and the streamed [`StreamEvent`] output.
//! * [`adapters`] — per-family HTTP request builders + SSE stream decoders
//!   (OpenAI `/chat/completions`, Anthropic `/messages`, Google
//!   `:streamGenerateContent`).
//! * [`secret_store`] — BYOK API-key storage in the OS keyring (never on disk).
//! * [`instances`] — provider-instance management + active model persistence.
//! * [`client`] — [`AiClient`], the cancellable streaming chat entry point.
//!
//! Chat sessions/history (T11-002), chat UI (T11-003) and the agent/tool loop
//! (T11-004) build on top of this crate.

pub mod adapters;
pub mod client;
pub mod config;
pub mod error;
pub mod instances;
pub mod message;
pub mod secret_store;
pub mod sessions;
pub mod sse;

pub use client::{resolve_target, AiClient, ChatStream, ResolvedTarget};
pub use config::{
    find_model, model_context_limit, model_keeps_reasoning, ModelInfo, ModelTag, ProviderFamily,
    ProviderId, DEFAULT_MODEL_ID, MODELS,
};
pub use error::AiError;
pub use instances::{make_model_ref, parse_model_ref, InstanceStore, ModelRef, ProviderInstance};
pub use message::{ChatConfig, ChatMessage, Role, StreamEvent, ToolCall, ToolDef, Usage};
pub use secret_store::{KeyringSecretStore, MemorySecretStore, SecretStore};
pub use sessions::{
    derive_title, MessageStatus, RunStatus, SessionMessage, SessionMeta, SessionStore,
    SessionToolCall, ToolCallStatus,
};
