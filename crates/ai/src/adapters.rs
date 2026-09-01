//! Per-family HTTP adapters: build the request for a provider and decode its
//! streaming response into [`StreamEvent`]s.
//!
//! Three wire formats are implemented, covering every provider in the catalog:
//! * **OpenAI** `/chat/completions` — openai, xai, cerebras, groq, deepseek,
//!   mistral, openrouter, lmstudio, openai-compatible, mlx, ollama.
//! * **Anthropic** `/messages`.
//! * **Google** `generativelanguage` `:streamGenerateContent`.

use serde_json::{json, Value};

use crate::config::ProviderFamily;
use crate::message::{ChatConfig, ChatMessage, Role, StreamEvent, ToolDef, Usage};
use crate::sse::SseEvent;

/// A fully-formed HTTP request for a provider.
#[derive(Debug, Clone)]
pub struct RequestSpec {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Value,
}

const ANTHROPIC_DEFAULT_MAX_TOKENS: u32 = 4096;

pub fn build_request(
    family: ProviderFamily,
    base_url: &str,
    model: &str,
    api_key: Option<&str>,
    config: &ChatConfig,
    messages: &[ChatMessage],
    tools: &[ToolDef],
) -> RequestSpec {
    let base = base_url.trim_end_matches('/');
    match family {
        ProviderFamily::OpenAi => build_openai(base, model, api_key, config, messages, tools),
        ProviderFamily::Anthropic => build_anthropic(base, model, api_key, config, messages, tools),
        ProviderFamily::Google => build_google(base, model, api_key, config, messages, tools),
    }
}

// ── OpenAI /chat/completions ───────────────────────────────────────────────

fn build_openai(
    base: &str,
    model: &str,
    api_key: Option<&str>,
    config: &ChatConfig,
    messages: &[ChatMessage],
    tools: &[ToolDef],
) -> RequestSpec {
    let msgs: Vec<Value> = messages
        .iter()
        .map(|m| {
            let role = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            let mut obj = json!({ "role": role, "content": m.content });
            if m.role == Role::Tool {
                if let Some(id) = &m.tool_call_id {
                    obj["tool_call_id"] = json!(id);
                }
            }
            if m.role == Role::Assistant && !m.tool_calls.is_empty() {
                obj["tool_calls"] = json!(m
                    .tool_calls
                    .iter()
                    .map(|tc| json!({
                        "id": tc.id,
                        "type": "function",
                        "function": { "name": tc.name, "arguments": tc.arguments },
                    }))
                    .collect::<Vec<_>>());
            }
            obj
        })
        .collect();

    let mut body = json!({
        "model": model,
        "messages": msgs,
        "stream": true,
        "stream_options": { "include_usage": true },
    });
    if let Some(t) = config.temperature {
        body["temperature"] = json!(t);
    }
    if let Some(m) = config.max_tokens {
        body["max_tokens"] = json!(m);
    }
    if !tools.is_empty() {
        body["tools"] = json!(tools
            .iter()
            .map(|t| json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                },
            }))
            .collect::<Vec<_>>());
    }

    let mut headers = vec![("content-type".to_string(), "application/json".to_string())];
    if let Some(k) = api_key.filter(|k| !k.is_empty()) {
        headers.push(("authorization".to_string(), format!("Bearer {k}")));
    }

    RequestSpec {
        url: format!("{base}/chat/completions"),
        headers,
        body,
    }
}

// ── Anthropic /messages ────────────────────────────────────────────────────

fn build_anthropic(
    base: &str,
    model: &str,
    api_key: Option<&str>,
    config: &ChatConfig,
    messages: &[ChatMessage],
    tools: &[ToolDef],
) -> RequestSpec {
    let system: String = messages
        .iter()
        .filter(|m| m.role == Role::System)
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    let msgs: Vec<Value> = messages
        .iter()
        .filter(|m| m.role != Role::System)
        .map(|m| match m.role {
            Role::Tool => json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": m.tool_call_id.clone().unwrap_or_default(),
                    "content": m.content,
                }],
            }),
            Role::Assistant => {
                let mut blocks = Vec::new();
                if !m.content.is_empty() {
                    blocks.push(json!({ "type": "text", "text": m.content }));
                }
                for tc in &m.tool_calls {
                    blocks.push(json!({
                        "type": "tool_use",
                        "id": tc.id,
                        "name": tc.name,
                        "input": serde_json::from_str::<Value>(&tc.arguments).unwrap_or(json!({})),
                    }));
                }
                json!({ "role": "assistant", "content": blocks })
            }
            _ => json!({
                "role": "user",
                "content": [{ "type": "text", "text": m.content }],
            }),
        })
        .collect();

    let mut body = json!({
        "model": model,
        "messages": msgs,
        "stream": true,
        "max_tokens": config.max_tokens.unwrap_or(ANTHROPIC_DEFAULT_MAX_TOKENS),
    });
    if !system.is_empty() {
        body["system"] = json!(system);
    }
    if let Some(t) = config.temperature {
        body["temperature"] = json!(t);
    }
    if !tools.is_empty() {
        body["tools"] = json!(tools
            .iter()
            .map(|t| json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.parameters,
            }))
            .collect::<Vec<_>>());
    }

    let mut headers = vec![
        ("content-type".to_string(), "application/json".to_string()),
        ("anthropic-version".to_string(), "2023-06-01".to_string()),
    ];
    if let Some(k) = api_key.filter(|k| !k.is_empty()) {
        headers.push(("x-api-key".to_string(), k.to_string()));
    }

    RequestSpec {
        url: format!("{base}/messages"),
        headers,
        body,
    }
}

// ── Google :streamGenerateContent ──────────────────────────────────────────

fn build_google(
    base: &str,
    model: &str,
    api_key: Option<&str>,
    config: &ChatConfig,
    messages: &[ChatMessage],
    tools: &[ToolDef],
) -> RequestSpec {
    let system: String = messages
        .iter()
        .filter(|m| m.role == Role::System)
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    let contents: Vec<Value> = messages
        .iter()
        .filter(|m| m.role != Role::System)
        .map(|m| match m.role {
            Role::Tool => json!({
                "role": "user",
                "parts": [{
                    "functionResponse": {
                        "name": m.tool_call_id.clone().unwrap_or_default(),
                        "response": { "content": m.content },
                    },
                }],
            }),
            Role::Assistant => {
                let mut parts = Vec::new();
                if !m.content.is_empty() {
                    parts.push(json!({ "text": m.content }));
                }
                for tc in &m.tool_calls {
                    parts.push(json!({
                        "functionCall": {
                            "name": tc.name,
                            "args": serde_json::from_str::<Value>(&tc.arguments).unwrap_or(json!({})),
                        },
                    }));
                }
                json!({ "role": "model", "parts": parts })
            }
            _ => json!({ "role": "user", "parts": [{ "text": m.content }] }),
        })
        .collect();

    let mut body = json!({ "contents": contents });
    if !system.is_empty() {
        body["systemInstruction"] = json!({ "parts": [{ "text": system }] });
    }
    let mut gen = json!({});
    if let Some(t) = config.temperature {
        gen["temperature"] = json!(t);
    }
    if let Some(m) = config.max_tokens {
        gen["maxOutputTokens"] = json!(m);
    }
    if gen.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
        body["generationConfig"] = gen;
    }
    if !tools.is_empty() {
        body["tools"] = json!([{
            "functionDeclarations": tools
                .iter()
                .map(|t| json!({
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                }))
                .collect::<Vec<_>>(),
        }]);
    }

    let key = api_key.unwrap_or("");
    RequestSpec {
        url: format!("{base}/models/{model}:streamGenerateContent?alt=sse&key={key}"),
        headers: vec![("content-type".to_string(), "application/json".to_string())],
        body,
    }
}

// ── Streaming decode ───────────────────────────────────────────────────────

/// Stateful decoder from provider SSE events to unified [`StreamEvent`]s.
pub enum StreamParser {
    OpenAi(OpenAiState),
    Anthropic(AnthropicState),
    Google(GoogleState),
}

impl StreamParser {
    pub fn new(family: ProviderFamily) -> Self {
        match family {
            ProviderFamily::OpenAi => StreamParser::OpenAi(OpenAiState::default()),
            ProviderFamily::Anthropic => StreamParser::Anthropic(AnthropicState::default()),
            ProviderFamily::Google => StreamParser::Google(GoogleState::default()),
        }
    }

    pub fn push_event(&mut self, ev: &SseEvent) -> Vec<StreamEvent> {
        match self {
            StreamParser::OpenAi(s) => s.push(ev),
            StreamParser::Anthropic(s) => s.push(ev),
            StreamParser::Google(s) => s.push(ev),
        }
    }

    /// Emit any terminal events if the stream closed without an explicit end
    /// marker (Google never sends one; OpenAI may drop `[DONE]`).
    pub fn finish(&mut self) -> Vec<StreamEvent> {
        match self {
            StreamParser::OpenAi(s) => s.finish(),
            StreamParser::Anthropic(s) => s.finish(),
            StreamParser::Google(s) => s.finish(),
        }
    }
}

/// `(json_key, field_accessor)` — maps a provider usage field onto [`Usage`].
type UsageField = (&'static str, fn(&mut Usage) -> &mut u32);

fn merge_usage(acc: &mut Usage, v: &Value, keys: &[UsageField]) {
    for (k, field) in keys {
        if let Some(n) = v.get(*k).and_then(|x| x.as_u64()) {
            *field(acc) = n as u32;
        }
    }
}

// ── OpenAI ─────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct OpenAiState {
    /// tool-call slot index -> (id, emitted_start)
    tools: Vec<(String, bool)>,
    finish_reason: Option<String>,
    usage: Usage,
    done: bool,
}

impl OpenAiState {
    fn push(&mut self, ev: &SseEvent) -> Vec<StreamEvent> {
        let mut out = Vec::new();
        let data = ev.data.trim();
        if data.is_empty() {
            return out;
        }
        if data == "[DONE]" {
            out.extend(self.close());
            return out;
        }
        let Ok(v) = serde_json::from_str::<Value>(data) else {
            return out;
        };
        if let Some(err) = v.get("error") {
            self.done = true;
            out.push(StreamEvent::Error(crate::AiError::Stream {
                provider: "openai".into(),
                detail: err
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("provider error")
                    .to_string(),
            }));
            return out;
        }
        if let Some(u) = v.get("usage").filter(|u| !u.is_null()) {
            merge_usage(
                &mut self.usage,
                u,
                &[
                    ("prompt_tokens", |x| &mut x.input_tokens),
                    ("completion_tokens", |x| &mut x.output_tokens),
                ],
            );
            if let Some(details) = u.get("prompt_tokens_details") {
                if let Some(c) = details.get("cached_tokens").and_then(|x| x.as_u64()) {
                    self.usage.cache_read_tokens = c as u32;
                }
            }
            out.push(StreamEvent::Usage(self.usage));
        }
        let Some(choice) = v.get("choices").and_then(|c| c.get(0)) else {
            return out;
        };
        if let Some(delta) = choice.get("delta") {
            if let Some(txt) = delta.get("content").and_then(|c| c.as_str()) {
                if !txt.is_empty() {
                    out.push(StreamEvent::TextDelta(txt.to_string()));
                }
            }
            for key in ["reasoning_content", "reasoning"] {
                if let Some(txt) = delta.get(key).and_then(|c| c.as_str()) {
                    if !txt.is_empty() {
                        out.push(StreamEvent::ReasoningDelta(txt.to_string()));
                    }
                }
            }
            if let Some(calls) = delta.get("tool_calls").and_then(|c| c.as_array()) {
                for call in calls {
                    let idx = call.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                    while self.tools.len() <= idx {
                        self.tools.push((String::new(), false));
                    }
                    if let Some(id) = call.get("id").and_then(|i| i.as_str()) {
                        if !id.is_empty() {
                            self.tools[idx].0 = id.to_string();
                        }
                    }
                    let func = call.get("function");
                    let name = func
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("");
                    if !self.tools[idx].1 && !name.is_empty() {
                        if self.tools[idx].0.is_empty() {
                            self.tools[idx].0 = format!("call_{idx}");
                        }
                        self.tools[idx].1 = true;
                        out.push(StreamEvent::ToolCallStart {
                            id: self.tools[idx].0.clone(),
                            name: name.to_string(),
                        });
                    }
                    if let Some(args) = func
                        .and_then(|f| f.get("arguments"))
                        .and_then(|a| a.as_str())
                    {
                        if !args.is_empty() && self.tools[idx].1 {
                            out.push(StreamEvent::ToolCallDelta {
                                id: self.tools[idx].0.clone(),
                                arguments_delta: args.to_string(),
                            });
                        }
                    }
                }
            }
        }
        if let Some(fr) = choice.get("finish_reason").and_then(|f| f.as_str()) {
            self.finish_reason = Some(fr.to_string());
        }
        out
    }

    fn close(&mut self) -> Vec<StreamEvent> {
        if self.done {
            return Vec::new();
        }
        self.done = true;
        let mut out = Vec::new();
        for (id, started) in &self.tools {
            if *started {
                out.push(StreamEvent::ToolCallEnd { id: id.clone() });
            }
        }
        out.push(StreamEvent::Done {
            finish_reason: self.finish_reason.clone().unwrap_or_else(|| "stop".into()),
        });
        out
    }

    fn finish(&mut self) -> Vec<StreamEvent> {
        self.close()
    }
}

// ── Anthropic ──────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct AnthropicState {
    /// content block index -> tool id (only for tool_use blocks)
    blocks: std::collections::HashMap<u64, String>,
    stop_reason: Option<String>,
    usage: Usage,
    done: bool,
}

impl AnthropicState {
    fn push(&mut self, ev: &SseEvent) -> Vec<StreamEvent> {
        let mut out = Vec::new();
        let Ok(v) = serde_json::from_str::<Value>(ev.data.trim()) else {
            return out;
        };
        let kind = ev
            .event
            .as_str()
            .to_string_or(v.get("type").and_then(|t| t.as_str()).unwrap_or(""));
        match kind.as_str() {
            "message_start" => {
                if let Some(u) = v.pointer("/message/usage") {
                    merge_usage(
                        &mut self.usage,
                        u,
                        &[
                            ("input_tokens", |x| &mut x.input_tokens),
                            ("output_tokens", |x| &mut x.output_tokens),
                            ("cache_read_input_tokens", |x| &mut x.cache_read_tokens),
                        ],
                    );
                    out.push(StreamEvent::Usage(self.usage));
                }
            }
            "content_block_start" => {
                let idx = v.get("index").and_then(|i| i.as_u64()).unwrap_or(0);
                let block = v.get("content_block");
                let btype = block.and_then(|b| b.get("type")).and_then(|t| t.as_str());
                if btype == Some("tool_use") {
                    let id = block
                        .and_then(|b| b.get("id"))
                        .and_then(|i| i.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = block
                        .and_then(|b| b.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    self.blocks.insert(idx, id.clone());
                    out.push(StreamEvent::ToolCallStart { id, name });
                }
            }
            "content_block_delta" => {
                let idx = v.get("index").and_then(|i| i.as_u64()).unwrap_or(0);
                let delta = v.get("delta");
                match delta.and_then(|d| d.get("type")).and_then(|t| t.as_str()) {
                    Some("text_delta") => {
                        if let Some(t) = delta.and_then(|d| d.get("text")).and_then(|t| t.as_str())
                        {
                            out.push(StreamEvent::TextDelta(t.to_string()));
                        }
                    }
                    Some("thinking_delta") => {
                        if let Some(t) = delta
                            .and_then(|d| d.get("thinking"))
                            .and_then(|t| t.as_str())
                        {
                            out.push(StreamEvent::ReasoningDelta(t.to_string()));
                        }
                    }
                    Some("input_json_delta") => {
                        if let (Some(id), Some(pj)) = (
                            self.blocks.get(&idx),
                            delta
                                .and_then(|d| d.get("partial_json"))
                                .and_then(|t| t.as_str()),
                        ) {
                            out.push(StreamEvent::ToolCallDelta {
                                id: id.clone(),
                                arguments_delta: pj.to_string(),
                            });
                        }
                    }
                    _ => {}
                }
            }
            "content_block_stop" => {
                let idx = v.get("index").and_then(|i| i.as_u64()).unwrap_or(0);
                if let Some(id) = self.blocks.remove(&idx) {
                    out.push(StreamEvent::ToolCallEnd { id });
                }
            }
            "message_delta" => {
                if let Some(sr) = v.pointer("/delta/stop_reason").and_then(|s| s.as_str()) {
                    self.stop_reason = Some(sr.to_string());
                }
                if let Some(u) = v.get("usage") {
                    merge_usage(
                        &mut self.usage,
                        u,
                        &[("output_tokens", |x| &mut x.output_tokens)],
                    );
                    out.push(StreamEvent::Usage(self.usage));
                }
            }
            "message_stop" => out.extend(self.close()),
            "error" => {
                self.done = true;
                out.push(StreamEvent::Error(crate::AiError::Stream {
                    provider: "anthropic".into(),
                    detail: v
                        .pointer("/error/message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("provider error")
                        .to_string(),
                }));
            }
            _ => {}
        }
        out
    }

    fn close(&mut self) -> Vec<StreamEvent> {
        if self.done {
            return Vec::new();
        }
        self.done = true;
        let mut out = Vec::new();
        for (_, id) in self.blocks.drain() {
            out.push(StreamEvent::ToolCallEnd { id });
        }
        out.push(StreamEvent::Done {
            finish_reason: self
                .stop_reason
                .clone()
                .unwrap_or_else(|| "end_turn".into()),
        });
        out
    }

    fn finish(&mut self) -> Vec<StreamEvent> {
        self.close()
    }
}

trait StrOr {
    fn to_string_or(&self, fallback: &str) -> String;
}
impl StrOr for &str {
    fn to_string_or(&self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self.to_string()
        }
    }
}

// ── Google ─────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct GoogleState {
    finish_reason: Option<String>,
    usage: Usage,
    tool_seq: u32,
    done: bool,
}

impl GoogleState {
    fn push(&mut self, ev: &SseEvent) -> Vec<StreamEvent> {
        let mut out = Vec::new();
        let data = ev.data.trim();
        if data.is_empty() || data == "[DONE]" {
            return out;
        }
        let Ok(v) = serde_json::from_str::<Value>(data) else {
            return out;
        };
        if let Some(err) = v.get("error") {
            self.done = true;
            out.push(StreamEvent::Error(crate::AiError::Stream {
                provider: "google".into(),
                detail: err
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("provider error")
                    .to_string(),
            }));
            return out;
        }
        if let Some(um) = v.get("usageMetadata") {
            merge_usage(
                &mut self.usage,
                um,
                &[
                    ("promptTokenCount", |x| &mut x.input_tokens),
                    ("candidatesTokenCount", |x| &mut x.output_tokens),
                    ("cachedContentTokenCount", |x| &mut x.cache_read_tokens),
                    ("thoughtsTokenCount", |x| &mut x.reasoning_tokens),
                ],
            );
            out.push(StreamEvent::Usage(self.usage));
        }
        if let Some(cand) = v.pointer("/candidates/0") {
            if let Some(parts) = cand.pointer("/content/parts").and_then(|p| p.as_array()) {
                for part in parts {
                    if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                        if !t.is_empty() {
                            let ev = if part
                                .get("thought")
                                .and_then(|b| b.as_bool())
                                .unwrap_or(false)
                            {
                                StreamEvent::ReasoningDelta(t.to_string())
                            } else {
                                StreamEvent::TextDelta(t.to_string())
                            };
                            out.push(ev);
                        }
                    }
                    if let Some(fc) = part.get("functionCall") {
                        self.tool_seq += 1;
                        let id = format!("call_{}", self.tool_seq);
                        let name = fc
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string();
                        let args = fc.get("args").cloned().unwrap_or(json!({}));
                        out.push(StreamEvent::ToolCallStart {
                            id: id.clone(),
                            name,
                        });
                        out.push(StreamEvent::ToolCallDelta {
                            id: id.clone(),
                            arguments_delta: args.to_string(),
                        });
                        out.push(StreamEvent::ToolCallEnd { id });
                    }
                }
            }
            if let Some(fr) = cand.get("finishReason").and_then(|f| f.as_str()) {
                self.finish_reason = Some(fr.to_string());
            }
        }
        out
    }

    fn close(&mut self) -> Vec<StreamEvent> {
        if self.done {
            return Vec::new();
        }
        self.done = true;
        vec![StreamEvent::Done {
            finish_reason: self.finish_reason.clone().unwrap_or_else(|| "STOP".into()),
        }]
    }

    fn finish(&mut self) -> Vec<StreamEvent> {
        self.close()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::ToolCall;

    fn frame(data: &str) -> SseEvent {
        SseEvent {
            event: String::new(),
            data: data.to_string(),
        }
    }
    fn ev(event: &str, data: &str) -> SseEvent {
        SseEvent {
            event: event.to_string(),
            data: data.to_string(),
        }
    }

    #[test]
    fn openai_request_shape() {
        let msgs = vec![
            ChatMessage::system("be brief"),
            ChatMessage::user("hi"),
            ChatMessage {
                role: Role::Assistant,
                content: String::new(),
                tool_calls: vec![ToolCall {
                    id: "c1".into(),
                    name: "read_file".into(),
                    arguments: "{\"path\":\"a\"}".into(),
                }],
                tool_call_id: None,
            },
            ChatMessage::tool_result("c1", "contents"),
        ];
        let tools = vec![ToolDef {
            name: "read_file".into(),
            description: "read".into(),
            parameters: json!({"type":"object"}),
        }];
        let r = build_request(
            ProviderFamily::OpenAi,
            "https://api.openai.com/v1/",
            "gpt-5.5",
            Some("sk-x"),
            &ChatConfig {
                temperature: Some(0.2),
                max_tokens: Some(100),
            },
            &msgs,
            &tools,
        );
        assert_eq!(r.url, "https://api.openai.com/v1/chat/completions");
        assert!(r
            .headers
            .iter()
            .any(|(k, v)| k == "authorization" && v == "Bearer sk-x"));
        assert_eq!(r.body["stream"], json!(true));
        assert_eq!(
            r.body["messages"][2]["tool_calls"][0]["function"]["name"],
            "read_file"
        );
        assert_eq!(r.body["messages"][3]["role"], "tool");
        assert_eq!(r.body["messages"][3]["tool_call_id"], "c1");
        assert_eq!(r.body["tools"][0]["type"], "function");
    }

    #[test]
    fn anthropic_request_extracts_system_and_defaults_max_tokens() {
        let msgs = vec![ChatMessage::system("sys"), ChatMessage::user("hi")];
        let r = build_request(
            ProviderFamily::Anthropic,
            "https://api.anthropic.com/v1",
            "claude-opus-4-7",
            Some("sk-ant"),
            &ChatConfig::default(),
            &msgs,
            &[],
        );
        assert_eq!(r.url, "https://api.anthropic.com/v1/messages");
        assert_eq!(r.body["system"], "sys");
        assert_eq!(r.body["max_tokens"], json!(ANTHROPIC_DEFAULT_MAX_TOKENS));
        assert_eq!(r.body["messages"][0]["content"][0]["type"], "text");
        assert!(r
            .headers
            .iter()
            .any(|(k, v)| k == "x-api-key" && v == "sk-ant"));
        assert!(r.headers.iter().any(|(k, _)| k == "anthropic-version"));
    }

    #[test]
    fn google_request_puts_key_in_query_and_maps_roles() {
        let msgs = vec![
            ChatMessage::system("sys"),
            ChatMessage::user("hi"),
            ChatMessage::assistant("hello"),
        ];
        let r = build_request(
            ProviderFamily::Google,
            "https://generativelanguage.googleapis.com/v1beta",
            "gemini-3-flash",
            Some("g-key"),
            &ChatConfig::default(),
            &msgs,
            &[],
        );
        assert!(r
            .url
            .contains("/models/gemini-3-flash:streamGenerateContent"));
        assert!(r.url.contains("alt=sse"));
        assert!(r.url.ends_with("key=g-key"));
        assert_eq!(r.body["systemInstruction"]["parts"][0]["text"], "sys");
        assert_eq!(r.body["contents"][1]["role"], "model");
    }

    #[test]
    fn openai_stream_text_and_tool_call() {
        let mut p = StreamParser::new(ProviderFamily::OpenAi);
        let mut got = Vec::new();
        got.extend(p.push_event(&frame(r#"{"choices":[{"delta":{"content":"Hel"}}]}"#)));
        got.extend(p.push_event(&frame(r#"{"choices":[{"delta":{"content":"lo"}}]}"#)));
        got.extend(p.push_event(&frame(
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c1","function":{"name":"grep","arguments":"{\"q\""}}]}}]}"#,
        )));
        got.extend(p.push_event(&frame(
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":":\"x\"}"}}]},"finish_reason":"tool_calls"}]}"#,
        )));
        got.extend(p.push_event(&frame(
            r#"{"usage":{"prompt_tokens":10,"completion_tokens":5}}"#,
        )));
        got.extend(p.push_event(&frame("[DONE]")));

        assert_eq!(got[0], StreamEvent::TextDelta("Hel".into()));
        assert_eq!(got[1], StreamEvent::TextDelta("lo".into()));
        assert_eq!(
            got[2],
            StreamEvent::ToolCallStart {
                id: "c1".into(),
                name: "grep".into()
            }
        );
        assert_eq!(
            got[3],
            StreamEvent::ToolCallDelta {
                id: "c1".into(),
                arguments_delta: "{\"q\"".into()
            }
        );
        assert!(matches!(got[5], StreamEvent::Usage(u) if u.input_tokens == 10));
        assert_eq!(
            got[got.len() - 2],
            StreamEvent::ToolCallEnd { id: "c1".into() }
        );
        assert!(
            matches!(&got[got.len() - 1], StreamEvent::Done { finish_reason } if finish_reason == "tool_calls")
        );
    }

    #[test]
    fn anthropic_stream_sequence() {
        let mut p = StreamParser::new(ProviderFamily::Anthropic);
        let mut got = Vec::new();
        got.extend(p.push_event(&ev(
            "message_start",
            r#"{"type":"message_start","message":{"usage":{"input_tokens":7}}}"#,
        )));
        got.extend(p.push_event(&ev(
            "content_block_start",
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text"}}"#,
        )));
        got.extend(p.push_event(&ev(
            "content_block_delta",
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#,
        )));
        got.extend(p.push_event(&ev(
            "content_block_start",
            r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"t1","name":"edit"}}"#,
        )));
        got.extend(p.push_event(&ev(
            "content_block_delta",
            r#"{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{}"}}"#,
        )));
        got.extend(p.push_event(&ev(
            "content_block_stop",
            r#"{"type":"content_block_stop","index":1}"#,
        )));
        got.extend(p.push_event(&ev(
            "message_delta",
            r#"{"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":3}}"#,
        )));
        got.extend(p.push_event(&ev("message_stop", r#"{"type":"message_stop"}"#)));

        assert!(matches!(got[0], StreamEvent::Usage(u) if u.input_tokens == 7));
        assert_eq!(got[1], StreamEvent::TextDelta("Hi".into()));
        assert_eq!(
            got[2],
            StreamEvent::ToolCallStart {
                id: "t1".into(),
                name: "edit".into()
            }
        );
        assert_eq!(
            got[3],
            StreamEvent::ToolCallDelta {
                id: "t1".into(),
                arguments_delta: "{}".into()
            }
        );
        assert_eq!(got[4], StreamEvent::ToolCallEnd { id: "t1".into() });
        assert!(
            matches!(&got[got.len()-1], StreamEvent::Done { finish_reason } if finish_reason == "tool_use")
        );
    }

    #[test]
    fn google_stream_text_then_close() {
        let mut p = StreamParser::new(ProviderFamily::Google);
        let mut got = Vec::new();
        got.extend(p.push_event(&frame(
            r#"{"candidates":[{"content":{"parts":[{"text":"hello"}]}}],"usageMetadata":{"promptTokenCount":4,"candidatesTokenCount":2}}"#,
        )));
        got.extend(p.push_event(&frame(
            r#"{"candidates":[{"content":{"parts":[{"functionCall":{"name":"glob","args":{"pattern":"*"}}}]},"finishReason":"STOP"}]}"#,
        )));
        got.extend(p.finish());
        assert!(matches!(got[0], StreamEvent::Usage(u) if u.output_tokens == 2));
        assert_eq!(got[1], StreamEvent::TextDelta("hello".into()));
        assert!(matches!(&got[2], StreamEvent::ToolCallStart { name, .. } if name == "glob"));
        assert_eq!(
            got[3],
            StreamEvent::ToolCallDelta {
                id: "call_1".into(),
                arguments_delta: "{\"pattern\":\"*\"}".into()
            }
        );
        assert_eq!(
            got[4],
            StreamEvent::ToolCallEnd {
                id: "call_1".into()
            }
        );
        assert!(matches!(&got[5], StreamEvent::Done { finish_reason } if finish_reason == "STOP"));
    }

    #[test]
    fn openai_error_frame_becomes_error_event() {
        let mut p = StreamParser::new(ProviderFamily::OpenAi);
        let got = p.push_event(&frame(r#"{"error":{"message":"boom"}}"#));
        assert!(matches!(&got[0], StreamEvent::Error(e) if e.to_string().contains("boom")));
    }
}
