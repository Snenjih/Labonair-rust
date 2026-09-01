//! The unified streaming chat client. Resolves a model reference against the
//! provider instances + keyring, dispatches to the right adapter, performs the
//! HTTP request with `reqwest`, and streams decoded [`StreamEvent`]s over an
//! mpsc channel. The stream is cancellable mid-flight.

use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::adapters::{build_request, StreamParser};
use crate::config::{find_model, ProviderFamily, ProviderId};
use crate::error::AiError;
use crate::instances::{parse_model_ref, resolve_instance, InstanceStore};
use crate::message::{ChatConfig, ChatMessage, StreamEvent, ToolDef};
use crate::secret_store::{get_instance_key, get_provider_key, SecretStore};
use crate::sse::SseDecoder;

/// Everything needed to talk to one provider for one request.
#[derive(Debug, Clone)]
pub struct ResolvedTarget {
    pub provider: ProviderId,
    pub family: ProviderFamily,
    pub base_url: String,
    pub api_key: Option<String>,
    /// The concrete model id sent on the wire.
    pub model: String,
}

/// Resolve a model ref (`"<model>"` or `"<model>@<instanceId>"`) into a target.
///
/// * Cloud providers require a key (from the instance, else the legacy
///   per-provider key). `openai-compatible` and keyless local providers don't.
/// * Local providers use the instance `local_model_id` (falling back to the
///   catalog placeholder) and the instance / default base URL.
pub fn resolve_target(
    model_ref: &str,
    instances: &InstanceStore,
    secrets: &dyn SecretStore,
) -> Result<ResolvedTarget, AiError> {
    let parsed = parse_model_ref(model_ref);
    // Determine the provider: from the catalog, or from the referenced instance.
    let instance = parsed
        .instance_id
        .as_deref()
        .and_then(|id| instances.instances().iter().find(|i| i.id == id).cloned());
    let provider = match (find_model(&parsed.model_def_id), &instance) {
        (Some(m), _) => m.provider,
        (None, Some(inst)) => inst.provider_id,
        (None, None) => return Err(AiError::UnknownModel(model_ref.to_string())),
    };

    let instance =
        instance.or_else(|| resolve_instance(provider, None, instances.instances()).cloned());

    let base_url = instance
        .as_ref()
        .and_then(|i| i.base_url.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let cloud = provider.cloud_base_url();
            if cloud.is_empty() {
                provider.default_base_url().to_string()
            } else {
                cloud.to_string()
            }
        });

    // Model id on the wire.
    let model = if provider.is_keyless() || provider == ProviderId::OpenaiCompatible {
        instance
            .as_ref()
            .and_then(|i| i.local_model_id.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| parsed.model_def_id.clone())
    } else {
        parsed.model_def_id.clone()
    };

    // API key.
    let api_key = if provider.needs_key() {
        let from_instance = instance
            .as_ref()
            .and_then(|i| get_instance_key(secrets, &i.id));
        let key = from_instance.or_else(|| get_provider_key(secrets, provider));
        if key.is_none() && provider != ProviderId::OpenaiCompatible {
            return Err(AiError::MissingKey(provider.label().to_string()));
        }
        key
    } else {
        None
    };

    Ok(ResolvedTarget {
        provider,
        family: provider.family(),
        base_url,
        api_key,
        model,
    })
}

/// A live streaming response. Drop it or call [`ChatStream::cancel`] to abort
/// the request; either way the underlying HTTP connection is closed.
pub struct ChatStream {
    rx: mpsc::Receiver<StreamEvent>,
    task: tokio::task::JoinHandle<()>,
}

impl ChatStream {
    /// Await the next event. `None` once the stream is fully consumed.
    pub async fn next(&mut self) -> Option<StreamEvent> {
        self.rx.recv().await
    }

    /// Abort the request immediately. Any partially-emitted assistant message
    /// is the caller's to finalize; no further events will arrive.
    pub fn cancel(&self) {
        self.task.abort();
    }
}

impl Drop for ChatStream {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Streaming chat client. Cheap to clone (wraps a `reqwest::Client`).
#[derive(Clone)]
pub struct AiClient {
    http: reqwest::Client,
}

impl Default for AiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AiClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(20))
            .build()
            .unwrap_or_default();
        AiClient { http }
    }

    /// Start a streaming completion. Events are delivered on the returned
    /// [`ChatStream`]; the last event is always exactly one `Done` or `Error`.
    pub fn stream_chat(
        &self,
        target: ResolvedTarget,
        config: ChatConfig,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDef>,
    ) -> ChatStream {
        let (tx, rx) = mpsc::channel(64);
        let http = self.http.clone();
        let task = tokio::spawn(async move {
            run_stream(http, target, config, messages, tools, tx).await;
        });
        ChatStream { rx, task }
    }
}

async fn run_stream(
    http: reqwest::Client,
    target: ResolvedTarget,
    config: ChatConfig,
    messages: Vec<ChatMessage>,
    tools: Vec<ToolDef>,
    tx: mpsc::Sender<StreamEvent>,
) {
    let provider_name = target.provider.as_str().to_string();
    let spec = build_request(
        target.family,
        &target.base_url,
        &target.model,
        target.api_key.as_deref(),
        &config,
        &messages,
        &tools,
    );

    let mut req = http.post(&spec.url).json(&spec.body);
    for (k, v) in &spec.headers {
        if k != "content-type" {
            req = req.header(k.as_str(), v.as_str());
        }
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            let _ = tx
                .send(StreamEvent::Error(AiError::from_reqwest(
                    &provider_name,
                    &e,
                )))
                .await;
            return;
        }
    };

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        let _ = tx
            .send(StreamEvent::Error(AiError::from_status(
                &provider_name,
                status,
                &body,
            )))
            .await;
        return;
    }

    let mut decoder = SseDecoder::new();
    let mut parser = StreamParser::new(target.family);
    let mut stream = resp.bytes_stream();
    let mut terminated = false;

    while let Some(chunk) = stream.next().await {
        let bytes = match chunk {
            Ok(b) => b,
            Err(e) => {
                let _ = tx
                    .send(StreamEvent::Error(AiError::from_reqwest(
                        &provider_name,
                        &e,
                    )))
                    .await;
                return;
            }
        };
        for sse in decoder.push(&bytes) {
            for out in parser.push_event(&sse) {
                terminated |= matches!(out, StreamEvent::Done { .. } | StreamEvent::Error(_));
                if tx.send(out).await.is_err() {
                    return; // receiver dropped — stop work
                }
            }
        }
    }

    if let Some(sse) = decoder.finish() {
        for out in parser.push_event(&sse) {
            terminated |= matches!(out, StreamEvent::Done { .. } | StreamEvent::Error(_));
            if tx.send(out).await.is_err() {
                return;
            }
        }
    }

    if !terminated {
        for out in parser.finish() {
            if tx.send(out).await.is_err() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_MODEL_ID;
    use crate::instances::make_model_ref;
    use crate::secret_store::{set_instance_key, MemorySecretStore};

    fn store(name: &str) -> InstanceStore {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "labonair-client-{name}-{}.json",
            uuid::Uuid::new_v4()
        ));
        InstanceStore::load(p)
    }

    #[test]
    fn resolve_requires_key_for_cloud_provider() {
        let secrets = MemorySecretStore::default();
        let instances = store("nokey");
        let err = resolve_target(DEFAULT_MODEL_ID, &instances, &secrets).unwrap_err();
        assert!(matches!(err, AiError::MissingKey(_)));
    }

    #[test]
    fn resolve_uses_instance_key_and_base_url() {
        let secrets = MemorySecretStore::default();
        let mut instances = store("withkey");
        let inst = instances.add(ProviderId::Openai).unwrap();
        set_instance_key(&secrets, &inst.id, "sk-abc").unwrap();
        let target = resolve_target(
            &make_model_ref("gpt-5.5", Some(&inst.id)),
            &instances,
            &secrets,
        )
        .unwrap();
        assert_eq!(target.provider, ProviderId::Openai);
        assert_eq!(target.api_key.as_deref(), Some("sk-abc"));
        assert_eq!(target.model, "gpt-5.5");
        assert_eq!(target.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn resolve_keyless_local_provider_uses_local_model_id() {
        let secrets = MemorySecretStore::default();
        let mut instances = store("ollama");
        let inst = instances.add(ProviderId::Ollama).unwrap();
        instances
            .update(&inst.id, |i| {
                i.local_model_id = Some("qwen2.5-coder".into())
            })
            .unwrap();
        let target = resolve_target(
            &make_model_ref("ollama-local", Some(&inst.id)),
            &instances,
            &secrets,
        )
        .unwrap();
        assert!(target.api_key.is_none());
        assert_eq!(target.model, "qwen2.5-coder");
        assert_eq!(target.base_url, "http://localhost:11434/v1");
    }

    #[tokio::test]
    async fn stream_reports_http_error_and_stays_consistent() {
        // No server on this port → connection error surfaces as one Error event.
        let client = AiClient::new();
        let target = ResolvedTarget {
            provider: ProviderId::Openai,
            family: ProviderFamily::OpenAi,
            base_url: "http://127.0.0.1:9".to_string(),
            api_key: Some("sk-x".into()),
            model: "gpt-5.5".into(),
        };
        let mut stream = client.stream_chat(
            target,
            ChatConfig::default(),
            vec![ChatMessage::user("hi")],
            vec![],
        );
        let first = stream.next().await.expect("one event");
        assert!(matches!(first, StreamEvent::Error(_)));
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn end_to_end_streams_openai_sse_over_http() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let _ = sock.read(&mut buf).await.unwrap();
            let body = concat!(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\n",
                "data: {\"choices\":[{\"delta\":{\"content\":\" there\"}}],\"finish_reason\":\"stop\"}\n\n",
                "data: {\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":2}}\n\n",
                "data: [DONE]\n\n",
            );
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            sock.write_all(resp.as_bytes()).await.unwrap();
            sock.flush().await.unwrap();
        });

        let client = AiClient::new();
        let target = ResolvedTarget {
            provider: ProviderId::Openai,
            family: ProviderFamily::OpenAi,
            base_url: format!("http://{addr}/v1"),
            api_key: Some("sk-x".into()),
            model: "gpt-5.5".into(),
        };
        let mut stream = client.stream_chat(
            target,
            ChatConfig::default(),
            vec![ChatMessage::user("hi")],
            vec![],
        );

        let mut text = String::new();
        let mut saw_usage = false;
        let mut finish = None;
        while let Some(ev) = stream.next().await {
            match ev {
                StreamEvent::TextDelta(t) => text.push_str(&t),
                StreamEvent::Usage(u) => saw_usage = u.input_tokens == 3,
                StreamEvent::Done { finish_reason } => finish = Some(finish_reason),
                StreamEvent::Error(e) => panic!("unexpected error: {e}"),
                _ => {}
            }
        }
        server.await.unwrap();
        assert_eq!(text, "Hi there");
        assert!(saw_usage);
        assert_eq!(finish.as_deref(), Some("stop"));
    }

    #[tokio::test]
    async fn cancel_stops_the_stream() {
        let client = AiClient::new();
        let target = ResolvedTarget {
            provider: ProviderId::Openai,
            family: ProviderFamily::OpenAi,
            base_url: "http://127.0.0.1:9".to_string(),
            api_key: Some("sk-x".into()),
            model: "m".into(),
        };
        let stream = client.stream_chat(
            target,
            ChatConfig::default(),
            vec![ChatMessage::user("hi")],
            vec![],
        );
        stream.cancel();
        drop(stream);
    }
}
