//! `ai` area — AI chat defaults, provider endpoints, agent-shell behaviour.
//! Agents/directives (`labonair-backend::modules::{agents,directives}`) have
//! their own JSON-file stores and are out of scope here (they are
//! user-editable collections, not scalar settings).

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, crate::MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct AiContent {
    pub ai_enabled: Option<bool>,
    pub ai_max_agent_steps: Option<u32>,
    pub ai_terminal_context_lines: Option<u32>,
    pub ai_temperature: Option<f32>,
    pub ai_warn_destructive_commands: Option<bool>,
    pub ai_auto_open_mini_on_send: Option<bool>,
    pub ai_notify_on_headless_command: Option<bool>,
    pub ai_shell_max_timeout_secs: Option<u32>,
    pub ai_shell_max_output_kb: Option<u32>,
    pub default_model_id: Option<String>,
    pub custom_instructions: Option<String>,
    pub autocomplete_enabled: Option<bool>,
    pub autocomplete_provider: Option<String>,
    pub autocomplete_model_id: Option<String>,
    #[serde(rename = "lmstudioBaseURL")]
    pub lmstudio_base_url: Option<String>,
    pub lmstudio_chat_model_id: Option<String>,
    #[serde(rename = "openaiCompatibleBaseURL")]
    pub openai_compatible_base_url: Option<String>,
    pub openai_compatible_model_id: Option<String>,
    #[serde(rename = "mlxBaseURL")]
    pub mlx_base_url: Option<String>,
    pub mlx_chat_model_id: Option<String>,
    #[serde(rename = "ollamaBaseURL")]
    pub ollama_base_url: Option<String>,
    pub ollama_chat_model_id: Option<String>,
}

impl AiContent {
    pub fn defaults() -> Self {
        Self {
            ai_enabled: Some(true),
            ai_max_agent_steps: Some(24),
            ai_terminal_context_lines: Some(300),
            ai_temperature: Some(0.7),
            ai_warn_destructive_commands: Some(true),
            ai_auto_open_mini_on_send: Some(true),
            ai_notify_on_headless_command: Some(true),
            ai_shell_max_timeout_secs: Some(300),
            ai_shell_max_output_kb: Some(256),
            default_model_id: Some(String::new()),
            custom_instructions: Some(String::new()),
            autocomplete_enabled: Some(false),
            autocomplete_provider: Some("cerebras".to_string()),
            autocomplete_model_id: Some(String::new()),
            lmstudio_base_url: Some("http://localhost:1234/v1".to_string()),
            lmstudio_chat_model_id: Some(String::new()),
            openai_compatible_base_url: Some(String::new()),
            openai_compatible_model_id: Some(String::new()),
            mlx_base_url: Some("http://localhost:8080".to_string()),
            mlx_chat_model_id: Some(String::new()),
            ollama_base_url: Some("http://localhost:11434".to_string()),
            ollama_chat_model_id: Some(String::new()),
        }
    }
}
