//! The provider-agnostic chat interface: conversation input, tool definitions,
//! generation config, and the streamed output events. Adapters map these to and
//! from each provider's concrete wire format.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// One message in the conversation history.
///
/// * `System`/`User`/`Assistant` carry `content` text.
/// * `Assistant` may additionally carry `tool_calls` it requested.
/// * `Tool` carries the result of a previous tool call (`tool_call_id` + text).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(text: impl Into<String>) -> Self {
        Self::text(Role::System, text)
    }
    pub fn user(text: impl Into<String>) -> Self {
        Self::text(Role::User, text)
    }
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::text(Role::Assistant, text)
    }
    pub fn tool_result(tool_call_id: impl Into<String>, text: impl Into<String>) -> Self {
        ChatMessage {
            role: Role::Tool,
            content: text.into(),
            tool_calls: Vec::new(),
            tool_call_id: Some(tool_call_id.into()),
        }
    }
    fn text(role: Role, text: impl Into<String>) -> Self {
        ChatMessage {
            role,
            content: text.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }
}

/// A tool call requested by the assistant. `arguments` is a JSON string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// A tool the model may call. `parameters` is a JSON-Schema object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Generation knobs. `None` = provider default.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChatConfig {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// Rolling token accounting for a single request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub reasoning_tokens: u32,
}

/// Incremental output from a streaming response. A well-formed stream ends with
/// exactly one `Done` **or** one `Error`.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// A chunk of visible assistant text.
    TextDelta(String),
    /// A chunk of reasoning / thinking text (kept separate from `TextDelta`).
    ReasoningDelta(String),
    /// A tool call has started; `arguments` will arrive via `ToolCallDelta`.
    ToolCallStart { id: String, name: String },
    /// More characters of the in-progress tool call's JSON arguments.
    ToolCallDelta { id: String, arguments_delta: String },
    /// The tool call's arguments are complete.
    ToolCallEnd { id: String },
    /// Updated token usage (may fire multiple times; last one wins).
    Usage(Usage),
    /// Terminal success event.
    Done { finish_reason: String },
    /// Terminal failure event.
    Error(crate::AiError),
}
