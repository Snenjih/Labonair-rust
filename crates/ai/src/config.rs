//! Provider + model catalog. Ported 1:1 from
//! `reference-src/src/modules/ai/config.ts` (values adjusted only where the
//! original relied on JS-SDK behaviour).

use serde::{Deserialize, Serialize};

/// Keyring service name — every AI secret is stored under this service.
pub const KEYRING_SERVICE: &str = "labonair-ai";

pub const DEFAULT_MODEL_ID: &str = "gpt-5.4-mini";

/// Supported provider families (BYOK cloud + local servers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderId {
    Openai,
    Anthropic,
    Google,
    Xai,
    Cerebras,
    Groq,
    Lmstudio,
    OpenaiCompatible,
    Deepseek,
    Mistral,
    Openrouter,
    Mlx,
    Ollama,
}

impl ProviderId {
    pub const ALL: [ProviderId; 13] = [
        ProviderId::Openai,
        ProviderId::Anthropic,
        ProviderId::Google,
        ProviderId::Xai,
        ProviderId::Cerebras,
        ProviderId::Groq,
        ProviderId::Lmstudio,
        ProviderId::OpenaiCompatible,
        ProviderId::Deepseek,
        ProviderId::Mistral,
        ProviderId::Openrouter,
        ProviderId::Mlx,
        ProviderId::Ollama,
    ];

    /// Stable string id (matches the TS `ProviderId` union / kebab-case).
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderId::Openai => "openai",
            ProviderId::Anthropic => "anthropic",
            ProviderId::Google => "google",
            ProviderId::Xai => "xai",
            ProviderId::Cerebras => "cerebras",
            ProviderId::Groq => "groq",
            ProviderId::Lmstudio => "lmstudio",
            ProviderId::OpenaiCompatible => "openai-compatible",
            ProviderId::Deepseek => "deepseek",
            ProviderId::Mistral => "mistral",
            ProviderId::Openrouter => "openrouter",
            ProviderId::Mlx => "mlx",
            ProviderId::Ollama => "ollama",
        }
    }

    pub fn from_id(s: &str) -> Option<ProviderId> {
        ProviderId::ALL.into_iter().find(|p| p.as_str() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            ProviderId::Openai => "OpenAI",
            ProviderId::Anthropic => "Anthropic",
            ProviderId::Google => "Google",
            ProviderId::Xai => "xAI",
            ProviderId::Cerebras => "Cerebras",
            ProviderId::Groq => "Groq",
            ProviderId::Lmstudio => "LM Studio",
            ProviderId::OpenaiCompatible => "OpenAI-compatible",
            ProviderId::Deepseek => "DeepSeek",
            ProviderId::Mistral => "Mistral",
            ProviderId::Openrouter => "OpenRouter",
            ProviderId::Mlx => "MLX (local)",
            ProviderId::Ollama => "Ollama (local)",
        }
    }

    /// Keyring account name for the (legacy) single-key-per-provider store.
    pub fn keyring_account(self) -> &'static str {
        match self {
            ProviderId::Openai => "openai-api-key",
            ProviderId::Anthropic => "anthropic-api-key",
            ProviderId::Google => "google-api-key",
            ProviderId::Xai => "xai-api-key",
            ProviderId::Cerebras => "cerebras-api-key",
            ProviderId::Groq => "groq-api-key",
            ProviderId::OpenaiCompatible => "openai-compatible-api-key",
            ProviderId::Deepseek => "deepseek-api-key",
            ProviderId::Mistral => "mistral-api-key",
            ProviderId::Openrouter => "openrouter-api-key",
            ProviderId::Lmstudio | ProviderId::Mlx | ProviderId::Ollama => "",
        }
    }

    /// Providers that run against a local server and need no API key.
    pub fn is_keyless(self) -> bool {
        matches!(
            self,
            ProviderId::Lmstudio | ProviderId::Mlx | ProviderId::Ollama
        )
    }

    pub fn needs_key(self) -> bool {
        !self.is_keyless()
    }

    /// Default base URL for local / custom providers (empty for cloud).
    pub fn default_base_url(self) -> &'static str {
        match self {
            ProviderId::Lmstudio => "http://localhost:1234/v1",
            ProviderId::OpenaiCompatible => "http://localhost:8080/v1",
            ProviderId::Mlx => "http://127.0.0.1:8080/v1",
            ProviderId::Ollama => "http://localhost:11434/v1",
            _ => "",
        }
    }

    /// Cloud API base URL used by the HTTP adapters.
    pub fn cloud_base_url(self) -> &'static str {
        match self {
            ProviderId::Openai => "https://api.openai.com/v1",
            ProviderId::Anthropic => "https://api.anthropic.com/v1",
            ProviderId::Google => "https://generativelanguage.googleapis.com/v1beta",
            ProviderId::Xai => "https://api.x.ai/v1",
            ProviderId::Cerebras => "https://api.cerebras.ai/v1",
            ProviderId::Groq => "https://api.groq.com/openai/v1",
            ProviderId::Deepseek => "https://api.deepseek.com/v1",
            ProviderId::Mistral => "https://api.mistral.ai/v1",
            ProviderId::Openrouter => "https://openrouter.ai/api/v1",
            _ => "",
        }
    }

    /// Which wire protocol the provider speaks.
    pub fn family(self) -> ProviderFamily {
        match self {
            ProviderId::Anthropic => ProviderFamily::Anthropic,
            ProviderId::Google => ProviderFamily::Google,
            _ => ProviderFamily::OpenAi,
        }
    }
}

/// The three request/stream formats we implement adapters for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderFamily {
    /// OpenAI `/chat/completions` — also LM Studio, Ollama, xAI, Groq, …
    OpenAi,
    /// Anthropic `/messages`.
    Anthropic,
    /// Google `generativelanguage` `:streamGenerateContent`.
    Google,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTag {
    Vision,
    Reasoning,
    Tools,
    Coding,
}

#[derive(Debug, Clone, Copy)]
pub struct ModelInfo {
    pub id: &'static str,
    pub provider: ProviderId,
    pub label: &'static str,
    pub hint: &'static str,
    pub context_limit: u32,
    pub tags: &'static [ModelTag],
}

use ModelTag::*;

/// The pre-configured model catalog (mirrors the ~21 static entries in
/// `config.ts::MODELS`; local providers expect a runtime model id).
pub const MODELS: &[ModelInfo] = &[
    ModelInfo {
        id: "gpt-5.4-mini",
        provider: ProviderId::Openai,
        label: "GPT-5.4 mini",
        hint: "Fast, default",
        context_limit: 400_000,
        tags: &[Tools],
    },
    ModelInfo {
        id: "gpt-5.5",
        provider: ProviderId::Openai,
        label: "GPT-5.5",
        hint: "Higher quality",
        context_limit: 1_050_000,
        tags: &[Tools, Coding],
    },
    ModelInfo {
        id: "gpt-5.3-codex",
        provider: ProviderId::Openai,
        label: "GPT-5.3 Codex",
        hint: "Coding",
        context_limit: 400_000,
        tags: &[Tools, Coding],
    },
    ModelInfo {
        id: "claude-haiku-4-5",
        provider: ProviderId::Anthropic,
        label: "Claude Haiku 4.5",
        hint: "Fast",
        context_limit: 200_000,
        tags: &[Tools],
    },
    ModelInfo {
        id: "claude-sonnet-4-6",
        provider: ProviderId::Anthropic,
        label: "Claude Sonnet 4.6",
        hint: "Balanced",
        context_limit: 200_000,
        tags: &[Tools, Coding],
    },
    ModelInfo {
        id: "claude-opus-4-7",
        provider: ProviderId::Anthropic,
        label: "Claude Opus 4.7",
        hint: "Best",
        context_limit: 200_000,
        tags: &[Reasoning, Tools, Coding],
    },
    ModelInfo {
        id: "gemini-3.1-pro",
        provider: ProviderId::Google,
        label: "Gemini 3.1 Pro",
        hint: "Best",
        context_limit: 1_000_000,
        tags: &[Tools, Vision],
    },
    ModelInfo {
        id: "gemini-3-flash",
        provider: ProviderId::Google,
        label: "Gemini 3 Flash",
        hint: "Fast",
        context_limit: 1_000_000,
        tags: &[Tools],
    },
    ModelInfo {
        id: "grok-4.20-reasoning",
        provider: ProviderId::Xai,
        label: "Grok 4.20 Reasoning",
        hint: "Reasoning",
        context_limit: 2_000_000,
        tags: &[Reasoning, Tools],
    },
    ModelInfo {
        id: "grok-4.20-non-reasoning",
        provider: ProviderId::Xai,
        label: "Grok 4.20",
        hint: "Fast",
        context_limit: 2_000_000,
        tags: &[Tools],
    },
    ModelInfo {
        id: "gpt-oss-120b",
        provider: ProviderId::Cerebras,
        label: "GPT-OSS 120B",
        hint: "Cerebras · ultra-fast",
        context_limit: 128_000,
        tags: &[Tools],
    },
    ModelInfo {
        id: "openai/gpt-oss-20b",
        provider: ProviderId::Groq,
        label: "GPT-OSS 20B",
        hint: "Groq · ultra-fast",
        context_limit: 128_000,
        tags: &[Tools],
    },
    ModelInfo {
        id: "lmstudio-local",
        provider: ProviderId::Lmstudio,
        label: "LM Studio (local)",
        hint: "Custom local model",
        context_limit: 32_000,
        tags: &[],
    },
    ModelInfo {
        id: "openai-compatible-custom",
        provider: ProviderId::OpenaiCompatible,
        label: "Custom Endpoint",
        hint: "OpenAI-compatible",
        context_limit: 128_000,
        tags: &[],
    },
    ModelInfo {
        id: "deepseek-chat",
        provider: ProviderId::Deepseek,
        label: "DeepSeek Chat",
        hint: "Strong coder",
        context_limit: 64_000,
        tags: &[Coding, Tools],
    },
    ModelInfo {
        id: "deepseek-reasoner",
        provider: ProviderId::Deepseek,
        label: "DeepSeek Reasoner",
        hint: "Reasoning",
        context_limit: 64_000,
        tags: &[Reasoning],
    },
    ModelInfo {
        id: "mistral-large-latest",
        provider: ProviderId::Mistral,
        label: "Mistral Large",
        hint: "Best",
        context_limit: 128_000,
        tags: &[Tools],
    },
    ModelInfo {
        id: "mistral-small-latest",
        provider: ProviderId::Mistral,
        label: "Mistral Small",
        hint: "Fast",
        context_limit: 128_000,
        tags: &[Tools],
    },
    ModelInfo {
        id: "openrouter-auto",
        provider: ProviderId::Openrouter,
        label: "OpenRouter Auto",
        hint: "Best available",
        context_limit: 128_000,
        tags: &[],
    },
    ModelInfo {
        id: "mlx-local",
        provider: ProviderId::Mlx,
        label: "MLX (local)",
        hint: "Apple Silicon",
        context_limit: 32_000,
        tags: &[],
    },
    ModelInfo {
        id: "ollama-local",
        provider: ProviderId::Ollama,
        label: "Ollama (local)",
        hint: "Custom local",
        context_limit: 32_000,
        tags: &[],
    },
];

/// Non-throwing catalog lookup (`None` for dynamic / user-supplied ids).
pub fn find_model(id: &str) -> Option<&'static ModelInfo> {
    MODELS.iter().find(|m| m.id == id)
}

/// Approximate context window (tokens) — conservative default for unknown ids.
pub fn model_context_limit(id: &str) -> u32 {
    find_model(id).map(|m| m.context_limit).unwrap_or(128_000)
}

/// True if the model preserves reasoning / thinking tokens across turns.
pub fn model_keeps_reasoning(id: &str) -> bool {
    find_model(id)
        .map(|m| m.tags.contains(&ModelTag::Reasoning))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_id_round_trips_through_str() {
        for p in ProviderId::ALL {
            assert_eq!(ProviderId::from_id(p.as_str()), Some(p));
        }
    }

    #[test]
    fn keyless_providers_have_no_account_and_a_base_url() {
        for p in ProviderId::ALL {
            if p.is_keyless() {
                assert_eq!(p.keyring_account(), "");
                assert!(!p.default_base_url().is_empty());
            }
        }
    }

    #[test]
    fn catalog_covers_all_cloud_providers() {
        for p in ProviderId::ALL {
            assert!(
                MODELS.iter().any(|m| m.provider == p),
                "no catalog model for {}",
                p.as_str()
            );
        }
    }

    #[test]
    fn context_limit_and_reasoning_lookup() {
        assert_eq!(model_context_limit("claude-opus-4-7"), 200_000);
        assert_eq!(model_context_limit("totally-unknown"), 128_000);
        assert!(model_keeps_reasoning("deepseek-reasoner"));
        assert!(!model_keeps_reasoning("gpt-5.4-mini"));
    }

    #[test]
    fn families_are_assigned() {
        assert_eq!(ProviderId::Anthropic.family(), ProviderFamily::Anthropic);
        assert_eq!(ProviderId::Google.family(), ProviderFamily::Google);
        assert_eq!(ProviderId::Ollama.family(), ProviderFamily::OpenAi);
    }
}
