//! Asynchronous local Language Server Protocol process integration.
//!
//! This module is the Editor-owned adapter for local JSON-RPC over stdio. The
//! command is executed directly with argv (never through a shell), configured
//! environment values are passed only to the child, and a project root must be
//! explicitly trusted by the caller. Neither command lines nor environment
//! values are included in events or errors. All process and pipe operations
//! run on Tokio tasks; the synchronous [`LocalLanguageService`] trait remains
//! available for the syntax provider and is not used to drive this adapter.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{broadcast, oneshot, Mutex, Notify};
use tokio::time;

use crate::buffer::{BufferSnapshot, Edit};
use crate::language_services::{
    CancellationToken, CodeAction, CompletionItem, CompletionKind, CompletionList, Diagnostic,
    DiagnosticSeverity, DocumentVersion, FoldingRange, HoverContent, LanguageId, LanguageServerId,
    LanguageServiceCapabilities, LanguageServiceEditCapabilities, LanguageServiceEditRequest,
    LanguageServiceEditResponse, LanguageServiceError, LanguageServiceRequest,
    LanguageServiceResponse, LanguageServiceResult, LspPosition, LspRange, NavigationTarget,
    SemanticToken, SignatureHelp,
};
use crate::symbols::{DocumentSymbol, SymbolKind};

const DEFAULT_MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// Errors produced while recovering Content-Length framed messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameError {
    MissingContentLength,
    InvalidContentLength,
    BodyTooLarge(usize),
    HeaderTooLarge,
    TruncatedInput,
}

impl FrameError {
    pub fn to_language_service_error(&self) -> LanguageServiceError {
        LanguageServiceError::MalformedFrame(match self {
            Self::MissingContentLength => "missing Content-Length header".to_string(),
            Self::InvalidContentLength => "invalid Content-Length header".to_string(),
            Self::BodyTooLarge(length) => format!("Content-Length exceeds limit ({length} bytes)"),
            Self::HeaderTooLarge => "LSP header exceeds the configured limit".to_string(),
            Self::TruncatedInput => "truncated LSP frame at end of input".to_string(),
        })
    }
}

/// Incremental, recoverable LSP Content-Length framer.
#[derive(Debug)]
pub struct ContentLengthFramer {
    buffer: Vec<u8>,
    max_body_size: usize,
}

impl ContentLengthFramer {
    pub fn new(max_body_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_body_size,
        }
    }

    /// Adds bytes and returns every complete frame or recoverable frame error.
    /// A malformed header is discarded through its separator so a subsequent
    /// valid message can still be processed.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Result<Vec<u8>, FrameError>> {
        self.buffer.extend_from_slice(bytes);
        let mut frames = Vec::new();
        loop {
            if find_header_separator(&self.buffer).is_none()
                && self.buffer.len() > self.max_body_size.min(DEFAULT_MAX_FRAME_BYTES)
            {
                self.buffer.clear();
                frames.push(Err(FrameError::HeaderTooLarge));
                break;
            }
            let Some((separator, separator_len)) = find_header_separator(&self.buffer) else {
                break;
            };
            let body_start = separator + separator_len;
            let headers = &self.buffer[..separator];
            let Some(length_header) = headers.split(|byte| *byte == b'\n').find_map(|line| {
                let colon = line.iter().position(|byte| *byte == b':')?;
                let (name, value) = line.split_at(colon);
                if name.eq_ignore_ascii_case(b"content-length") {
                    Some(&value[1..])
                } else {
                    None
                }
            }) else {
                self.buffer.drain(..body_start);
                frames.push(Err(FrameError::MissingContentLength));
                continue;
            };
            let Ok(length_text) = std::str::from_utf8(length_header) else {
                self.buffer.drain(..body_start);
                frames.push(Err(FrameError::InvalidContentLength));
                continue;
            };
            let Ok(length) = length_text.trim().parse::<usize>() else {
                self.buffer.drain(..body_start);
                frames.push(Err(FrameError::InvalidContentLength));
                continue;
            };
            if length > self.max_body_size {
                self.buffer.drain(..body_start);
                frames.push(Err(FrameError::BodyTooLarge(length)));
                continue;
            }
            if self.buffer.len() < body_start.saturating_add(length) {
                break;
            }
            let body = self.buffer[body_start..body_start + length].to_vec();
            self.buffer.drain(..body_start + length);
            frames.push(Ok(body));
        }
        frames
    }

    /// Reports an incomplete header or body when the transport reaches EOF.
    pub fn finish(&mut self) -> Option<FrameError> {
        if self.buffer.is_empty() {
            None
        } else {
            self.buffer.clear();
            Some(FrameError::TruncatedInput)
        }
    }
}

impl Default for ContentLengthFramer {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_FRAME_BYTES)
    }
}

fn find_header_separator(bytes: &[u8]) -> Option<(usize, usize)> {
    let crlf = bytes.windows(4).position(|window| window == b"\r\n\r\n");
    let lf = bytes.windows(2).position(|window| window == b"\n\n");
    match (crlf, lf) {
        (Some(a), Some(b)) if a + 2 <= b => Some((a, 4)),
        (Some(a), _) => Some((a, 4)),
        (_, Some(b)) => Some((b, 2)),
        _ => None,
    }
}

/// Explicit policy for the local project root used as the server cwd and
/// `rootUri`. Untrusted roots are rejected before a process is spawned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectRootPolicy {
    Trusted,
    Untrusted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageServerTimeoutPolicy {
    pub initialize: Duration,
    pub request: Duration,
    pub shutdown: Duration,
}

impl Default for LanguageServerTimeoutPolicy {
    fn default() -> Self {
        Self {
            initialize: Duration::from_secs(10),
            request: Duration::from_secs(5),
            shutdown: Duration::from_secs(2),
        }
    }
}

/// Safe, argv-based configuration for one local language-server process.
///
/// `command` and `args` are passed directly to the child.  No shell is
/// involved, so arguments remain unambiguous even when they contain spaces or
/// shell metacharacters.
#[derive(Clone, Debug)]
pub struct ProcessServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
    pub project_root_policy: ProjectRootPolicy,
    pub language: LanguageId,
    pub server: LanguageServerId,
    pub timeouts: LanguageServerTimeoutPolicy,
    pub max_frame_bytes: usize,
}

/// Compatibility name for the public local adapter handle.  The canonical
/// configuration contract is [`ProcessServerConfig`].
pub type LocalLanguageServerConfig = ProcessServerConfig;

impl ProcessServerConfig {
    pub fn validate(&self) -> Result<(), LanguageServiceError> {
        if self.command.trim().is_empty() {
            return Err(LanguageServiceError::InvalidConfiguration(
                "language-server program is empty".into(),
            ));
        }
        if self.command.contains('\0') || self.cwd.to_string_lossy().contains('\0') {
            return Err(LanguageServiceError::InvalidConfiguration(
                "language-server program or cwd contains a NUL byte".into(),
            ));
        }
        if !self.cwd.is_absolute() {
            return Err(LanguageServiceError::InvalidConfiguration(
                "language-server cwd must be absolute".into(),
            ));
        }
        if self.project_root_policy != ProjectRootPolicy::Trusted {
            return Err(LanguageServiceError::InvalidConfiguration(
                "language-server project root is not trusted".into(),
            ));
        }
        if self.args.iter().any(|arg| arg.contains('\0')) {
            return Err(LanguageServiceError::InvalidConfiguration(
                "language-server arguments must not contain NUL bytes".into(),
            ));
        }
        if self
            .env
            .iter()
            .any(|(key, value)| key.is_empty() || key.contains('\0') || value.contains('\0'))
        {
            return Err(LanguageServiceError::InvalidConfiguration(
                "language-server environment contains an invalid entry".into(),
            ));
        }
        if self.timeouts.initialize.is_zero()
            || self.timeouts.request.is_zero()
            || self.timeouts.shutdown.is_zero()
        {
            return Err(LanguageServiceError::InvalidConfiguration(
                "language-server timeouts must be non-zero".into(),
            ));
        }
        if self.max_frame_bytes == 0 {
            return Err(LanguageServiceError::InvalidConfiguration(
                "language-server frame limit must be non-zero".into(),
            ));
        }
        Ok(())
    }
}

/// Operational and data events emitted by the Editor-owned process adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LanguageServiceEvent {
    Started {
        server: LanguageServerId,
        process_generation: u64,
    },
    Initialized {
        server: LanguageServerId,
        process_generation: u64,
        capabilities: LanguageServiceCapabilities,
    },
    Diagnostics {
        server: LanguageServerId,
        uri: String,
        version: DocumentVersion,
        generation: u64,
        diagnostics: Vec<Diagnostic>,
    },
    MalformedFrame {
        server: LanguageServerId,
        process_generation: u64,
        detail: String,
    },
    ProtocolEof {
        server: LanguageServerId,
        process_generation: u64,
    },
    Crashed {
        server: LanguageServerId,
        process_generation: u64,
        exit_code: Option<i32>,
    },
    Stopped {
        server: LanguageServerId,
        process_generation: u64,
    },
    Restarted {
        server: LanguageServerId,
        process_generation: u64,
    },
}

/// Lifecycle state of the process adapter, independent of the child process's
/// private implementation details.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessLifecycleState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Crashed,
}

pub struct LocalLanguageServiceRequest {
    pub uri: String,
    pub snapshot: BufferSnapshot,
    pub version: DocumentVersion,
    pub generation: u64,
    pub request: LanguageServiceRequest,
    pub cancellation: CancellationToken,
}

#[derive(Clone, Debug)]
pub struct LocalLanguageServiceEditRequest {
    pub uri: String,
    pub snapshot: BufferSnapshot,
    pub version: DocumentVersion,
    pub generation: u64,
    pub request: LanguageServiceEditRequest,
    pub cancellation: CancellationToken,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalLanguageServiceEditResult {
    pub server: LanguageServerId,
    pub version: DocumentVersion,
    pub generation: u64,
    pub response: Result<LanguageServiceEditResponse, LanguageServiceError>,
}

struct PendingRequest {
    sender: oneshot::Sender<Result<Value, LanguageServiceError>>,
}

struct DocumentState {
    language: LanguageId,
    version: DocumentVersion,
    generation: u64,
    text: String,
    diagnostics: Vec<Diagnostic>,
}

struct ProcessState {
    process_generation: u64,
    lifecycle: ProcessLifecycleState,
    writer: Option<Arc<Mutex<ChildStdin>>>,
    stop: Option<oneshot::Sender<()>>,
    ended: Option<Arc<Notify>>,
    pending: HashMap<u64, PendingRequest>,
    capabilities: LanguageServiceCapabilities,
    edit_capabilities: LanguageServiceEditCapabilities,
    semantic_token_types: Vec<String>,
    documents: HashMap<String, DocumentState>,
    stopping: bool,
}

struct ProcessInner {
    config: LocalLanguageServerConfig,
    events: broadcast::Sender<LanguageServiceEvent>,
    state: Mutex<ProcessState>,
    next_request_id: std::sync::atomic::AtomicU64,
}

/// A running local language-server process. Clone handles refer to the same
/// Editor-owned lifecycle and request correlation state.
#[derive(Clone)]
pub struct LocalLanguageServiceProcess {
    inner: Arc<ProcessInner>,
}

impl LocalLanguageServiceProcess {
    pub async fn spawn(config: LocalLanguageServerConfig) -> Result<Self, LanguageServiceError> {
        config.validate()?;
        let (events, _) = broadcast::channel(128);
        let process = Self {
            inner: Arc::new(ProcessInner {
                config,
                events,
                state: Mutex::new(ProcessState {
                    process_generation: 0,
                    lifecycle: ProcessLifecycleState::Stopped,
                    writer: None,
                    stop: None,
                    ended: None,
                    pending: HashMap::new(),
                    capabilities: LanguageServiceCapabilities {
                        diagnostics: true,
                        completion: false,
                        signature_help: false,
                        hover: false,
                        definition: false,
                        references: false,
                        rename: false,
                        code_actions: false,
                        semantic_tokens: false,
                        document_symbols: false,
                        folding_ranges: false,
                        formatting: false,
                    },
                    edit_capabilities: LanguageServiceEditCapabilities::NONE,
                    semantic_token_types: Vec::new(),
                    documents: HashMap::new(),
                    stopping: false,
                }),
                next_request_id: std::sync::atomic::AtomicU64::new(1),
            }),
        };
        if let Err(error) = process.start_session(1).await {
            let _ = process.stop().await;
            return Err(error);
        }
        Ok(process)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LanguageServiceEvent> {
        self.inner.events.subscribe()
    }

    pub fn language(&self) -> LanguageId {
        self.inner.config.language
    }

    pub fn server_id(&self) -> LanguageServerId {
        self.inner.config.server.clone()
    }

    pub async fn capabilities(&self) -> LanguageServiceCapabilities {
        self.inner.state.lock().await.capabilities
    }

    pub async fn edit_capabilities(&self) -> LanguageServiceEditCapabilities {
        self.inner.state.lock().await.edit_capabilities
    }

    /// Resolve the URI only when the process has one document at the requested
    /// revision.  The runtime uses this for its legacy edit request boundary,
    /// which intentionally does not expose a URI to Workspace.
    pub async fn unique_document_uri(
        &self,
        version: DocumentVersion,
        generation: u64,
    ) -> Option<String> {
        let state = self.inner.state.lock().await;
        let mut matches = state.documents.iter().filter(|(_, document)| {
            document.version == version && document.generation == generation
        });
        let (uri, _) = matches.next()?;
        if matches.next().is_none() {
            Some(uri.clone())
        } else {
            None
        }
    }

    pub async fn lifecycle(&self) -> ProcessLifecycleState {
        self.inner.state.lock().await.lifecycle
    }

    pub async fn did_open(
        &self,
        uri: String,
        language: LanguageId,
        text: String,
        version: DocumentVersion,
        generation: u64,
    ) -> Result<(), LanguageServiceError> {
        self.ensure_language(language)?;
        {
            let mut state = self.inner.state.lock().await;
            state.documents.insert(
                uri.clone(),
                DocumentState {
                    language,
                    version,
                    generation,
                    text: text.clone(),
                    diagnostics: Vec::new(),
                },
            );
        }
        let language_id = language.language().label().to_ascii_lowercase();
        self.send_notification(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": version.value(),
                    "text": text,
                }
            }),
        )
        .await
    }

    pub async fn did_change(
        &self,
        uri: &str,
        text: String,
        version: DocumentVersion,
        generation: u64,
    ) -> Result<(), LanguageServiceError> {
        {
            let mut state = self.inner.state.lock().await;
            let document = state.documents.get_mut(uri).ok_or_else(|| {
                LanguageServiceError::ProviderFailed("document was not opened".into())
            })?;
            if version <= document.version || generation <= document.generation {
                return Err(LanguageServiceError::StaleDocument {
                    expected: document.version,
                    received: version,
                });
            }
            document.version = version;
            document.generation = generation;
            document.text = text.clone();
            document.diagnostics.clear();
        }
        self.send_notification(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": uri, "version": version.value() },
                "contentChanges": [{ "text": text }],
            }),
        )
        .await
    }

    pub async fn did_close(&self, uri: &str) -> Result<(), LanguageServiceError> {
        self.inner.state.lock().await.documents.remove(uri);
        self.send_notification(
            "textDocument/didClose",
            json!({ "textDocument": { "uri": uri } }),
        )
        .await
    }

    pub async fn request(&self, input: LocalLanguageServiceRequest) -> LanguageServiceResult {
        let capability = input.request.capability();
        let current = self
            .inner
            .state
            .lock()
            .await
            .documents
            .get(&input.uri)
            .map(|document| (document.version, document.generation));
        let Some((version, generation)) = current else {
            return self.result(
                input.version,
                input.generation,
                Err(LanguageServiceError::ProviderFailed(
                    "document was not opened".into(),
                )),
            );
        };
        if version != input.version || generation != input.generation {
            return self.result(
                input.version,
                input.generation,
                Err(LanguageServiceError::StaleDocument {
                    expected: version,
                    received: input.version,
                }),
            );
        }
        if !self
            .inner
            .state
            .lock()
            .await
            .capabilities
            .supports(capability)
        {
            return self.result(
                input.version,
                input.generation,
                Err(LanguageServiceError::UnsupportedCapability(capability)),
            );
        }
        if input.cancellation.is_cancelled() {
            return self.result(
                input.version,
                input.generation,
                Err(LanguageServiceError::Cancelled),
            );
        }
        if matches!(input.request, LanguageServiceRequest::Diagnostics) {
            let diagnostics = self
                .inner
                .state
                .lock()
                .await
                .documents
                .get(&input.uri)
                .map(|document| document.diagnostics.clone())
                .unwrap_or_default();
            return self.result(
                input.version,
                input.generation,
                Ok(LanguageServiceResponse::Diagnostics(diagnostics)),
            );
        }
        let (method, params) = request_parts(&input.uri, &input.request);
        let response = match self
            .request_value(
                method,
                params,
                &input.cancellation,
                self.inner.config.timeouts.request,
            )
            .await
        {
            Ok(value) => {
                let current = self
                    .inner
                    .state
                    .lock()
                    .await
                    .documents
                    .get(&input.uri)
                    .map(|document| (document.version, document.generation));
                if current != Some((input.version, input.generation)) {
                    Err(LanguageServiceError::StaleDocument {
                        expected: current.map(|(version, _)| version).unwrap_or(input.version),
                        received: input.version,
                    })
                } else {
                    self.convert_response(&input.uri, &input.snapshot, &input.request, value)
                        .await
                }
            }
            Err(error) => Err(error),
        };
        self.result(input.version, input.generation, response)
    }

    pub async fn stop(&self) -> Result<(), LanguageServiceError> {
        let (stop, ended) = {
            let mut state = self.inner.state.lock().await;
            let Some(stop) = state.stop.take() else {
                return Ok(());
            };
            state.lifecycle = ProcessLifecycleState::Stopping;
            state.stopping = true;
            (stop, state.ended.clone())
        };
        let _ = stop.send(());
        if let Some(ended) = ended {
            time::timeout(self.inner.config.timeouts.shutdown, ended.notified())
                .await
                .map_err(|_| LanguageServiceError::RequestTimeout)?;
        }
        Ok(())
    }

    pub async fn restart(&self) -> Result<(), LanguageServiceError> {
        self.stop().await?;
        let generation = self
            .inner
            .state
            .lock()
            .await
            .process_generation
            .saturating_add(1);
        if let Err(error) = self.start_session(generation).await {
            let _ = self.stop().await;
            return Err(error);
        }
        let _ = self.inner.events.send(LanguageServiceEvent::Restarted {
            server: self.inner.config.server.clone(),
            process_generation: generation,
        });
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), LanguageServiceError> {
        let token = CancellationToken::default();
        let should_shutdown = self.inner.state.lock().await.writer.is_some();
        if should_shutdown {
            let _ = self
                .request_value(
                    "shutdown",
                    Value::Null,
                    &token,
                    self.inner.config.timeouts.shutdown,
                )
                .await;
            let _ = self.send_notification("exit", Value::Null).await;
        }
        self.stop().await
    }

    fn result(
        &self,
        version: DocumentVersion,
        generation: u64,
        response: Result<LanguageServiceResponse, LanguageServiceError>,
    ) -> LanguageServiceResult {
        LanguageServiceResult {
            server: self.inner.config.server.clone(),
            version,
            generation,
            response,
        }
    }

    fn ensure_language(&self, language: LanguageId) -> Result<(), LanguageServiceError> {
        if language == self.inner.config.language {
            Ok(())
        } else {
            Err(LanguageServiceError::UnsupportedLanguage(language))
        }
    }

    async fn start_session(&self, process_generation: u64) -> Result<(), LanguageServiceError> {
        let config = &self.inner.config;
        let mut command = Command::new(&config.command);
        command
            .args(&config.args)
            .envs(config.env.iter().map(|(key, value)| (key, value)))
            .current_dir(&config.cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn().map_err(|error| {
            LanguageServiceError::ProviderFailed(format!(
                "failed to start language server: {error}"
            ))
        })?;
        let stdin = child.stdin.take().ok_or_else(|| {
            LanguageServiceError::ProviderFailed("language server stdin was not piped".into())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            LanguageServiceError::ProviderFailed("language server stdout was not piped".into())
        })?;
        let ended = Arc::new(Notify::new());
        let stop_rx = {
            let (stop_tx, stop_rx) = oneshot::channel();
            let mut state = self.inner.state.lock().await;
            state.process_generation = process_generation;
            state.lifecycle = ProcessLifecycleState::Starting;
            state.writer = Some(Arc::new(Mutex::new(stdin)));
            state.stop = Some(stop_tx);
            state.ended = Some(ended);
            state.pending.clear();
            state.capabilities = LanguageServiceCapabilities {
                diagnostics: true,
                completion: false,
                signature_help: false,
                hover: false,
                definition: false,
                references: false,
                rename: false,
                code_actions: false,
                semantic_tokens: false,
                document_symbols: false,
                folding_ranges: false,
                formatting: false,
            };
            state.edit_capabilities = LanguageServiceEditCapabilities::NONE;
            state.semantic_token_types.clear();
            state.stopping = false;
            stop_rx
        };
        let _ = self.inner.events.send(LanguageServiceEvent::Started {
            server: self.inner.config.server.clone(),
            process_generation,
        });
        let reader_inner = self.inner.clone();
        tokio::spawn(
            async move { read_process_output(reader_inner, process_generation, stdout).await },
        );
        let waiter_inner = self.inner.clone();
        tokio::spawn(async move {
            watch_process(waiter_inner, process_generation, child, stop_rx).await
        });

        let root_uri = path_to_uri(&config.cwd);
        let initialized = self
            .request_value(
                "initialize",
                json!({
                    "processId": Value::Null,
                    "rootUri": root_uri,
                    "capabilities": {
                        "textDocument": {
                            "completion": { "completionItem": { "snippetSupport": true } },
                            "hover": {},
                            "publishDiagnostics": {},
                            "synchronization": { "dynamicRegistration": false },
                        },
                        "workspace": { "workspaceFolders": true },
                    },
                    "workspaceFolders": [{ "uri": path_to_uri(&config.cwd), "name": config.language.language().label() }],
                }),
                &CancellationToken::default(),
                config.timeouts.initialize,
            )
            .await?;
        let (capabilities, token_types) = parse_server_capabilities(&initialized);
        let edit_capabilities = parse_server_edit_capabilities(&initialized);
        {
            let mut state = self.inner.state.lock().await;
            state.capabilities = capabilities;
            state.edit_capabilities = edit_capabilities;
            state.semantic_token_types = token_types;
            state.lifecycle = ProcessLifecycleState::Running;
        }
        self.send_notification("initialized", json!({})).await?;
        let documents = self
            .inner
            .state
            .lock()
            .await
            .documents
            .iter()
            .map(|(uri, document)| {
                (
                    uri.clone(),
                    document.language,
                    document.version,
                    document.text.clone(),
                )
            })
            .collect::<Vec<_>>();
        for (uri, language, version, text) in documents {
            self.send_notification(
                "textDocument/didOpen",
                json!({
                    "textDocument": {
                        "uri": uri,
                        "languageId": language.language().label().to_ascii_lowercase(),
                        "version": version.value(),
                        "text": text,
                    }
                }),
            )
            .await?;
        }
        let _ = self.inner.events.send(LanguageServiceEvent::Initialized {
            server: self.inner.config.server.clone(),
            process_generation,
            capabilities,
        });
        Ok(())
    }

    pub async fn request_edit(
        &self,
        input: LocalLanguageServiceEditRequest,
    ) -> LocalLanguageServiceEditResult {
        let capability = input.request.capability();
        let current = self
            .inner
            .state
            .lock()
            .await
            .documents
            .get(&input.uri)
            .map(|document| (document.version, document.generation));
        let response = match current {
            None => Err(LanguageServiceError::ProviderFailed(
                "document was not opened".into(),
            )),
            Some((version, generation))
                if version != input.version || generation != input.generation =>
            {
                Err(LanguageServiceError::StaleDocument {
                    expected: version,
                    received: input.version,
                })
            }
            Some(_)
                if !self
                    .inner
                    .state
                    .lock()
                    .await
                    .edit_capabilities
                    .supports(capability) =>
            {
                Err(LanguageServiceError::UnsupportedEditCapability(capability))
            }
            Some(_) if input.cancellation.is_cancelled() => Err(LanguageServiceError::Cancelled),
            Some(_) => {
                let (method, params) =
                    edit_request_parts(&input.uri, &input.snapshot, &input.request);
                self.request_value(
                    method,
                    params,
                    &input.cancellation,
                    self.inner.config.timeouts.request,
                )
                .await
                .and_then(|value| {
                    self.convert_edit_response(&input.uri, &input.snapshot, &input.request, value)
                })
            }
        };
        let response = if response.is_ok() {
            let current = self
                .inner
                .state
                .lock()
                .await
                .documents
                .get(&input.uri)
                .map(|document| (document.version, document.generation));
            if current != Some((input.version, input.generation)) {
                Err(LanguageServiceError::StaleDocument {
                    expected: current.map(|(version, _)| version).unwrap_or(input.version),
                    received: input.version,
                })
            } else {
                response
            }
        } else {
            response
        };
        LocalLanguageServiceEditResult {
            server: self.inner.config.server.clone(),
            version: input.version,
            generation: input.generation,
            response,
        }
    }

    async fn request_value(
        &self,
        method: &str,
        params: Value,
        cancellation: &CancellationToken,
        timeout: Duration,
    ) -> Result<Value, LanguageServiceError> {
        let id = self
            .inner
            .next_request_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        let writer = {
            let mut state = self.inner.state.lock().await;
            let writer = state
                .writer
                .clone()
                .ok_or(LanguageServiceError::ProcessNotRunning)?;
            state.pending.insert(id, PendingRequest { sender });
            writer
        };
        let message = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        if let Err(error) = write_message(&writer, &message).await {
            self.inner.state.lock().await.pending.remove(&id);
            return Err(error);
        }
        self.wait_for_response(id, receiver, cancellation, timeout)
            .await
    }

    async fn send_notification(
        &self,
        method: &str,
        params: Value,
    ) -> Result<(), LanguageServiceError> {
        let writer = self
            .inner
            .state
            .lock()
            .await
            .writer
            .clone()
            .ok_or(LanguageServiceError::ProcessNotRunning)?;
        write_message(
            &writer,
            &json!({ "jsonrpc": "2.0", "method": method, "params": params }),
        )
        .await
    }

    async fn wait_for_response(
        &self,
        id: u64,
        mut receiver: oneshot::Receiver<Result<Value, LanguageServiceError>>,
        cancellation: &CancellationToken,
        timeout: Duration,
    ) -> Result<Value, LanguageServiceError> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                self.cancel_pending(id).await;
                return Err(LanguageServiceError::RequestTimeout);
            }
            let tick = remaining.min(Duration::from_millis(20));
            tokio::select! {
                result = &mut receiver => {
                    return result
                        .map_err(|_| LanguageServiceError::ProtocolEof)?;
                }
                _ = time::sleep(tick) => {
                    if cancellation.is_cancelled() {
                        self.cancel_pending(id).await;
                        return Err(LanguageServiceError::Cancelled);
                    }
                }
            }
        }
    }

    async fn cancel_pending(&self, id: u64) {
        let removed = self.inner.state.lock().await.pending.remove(&id).is_some();
        if removed {
            let _ = self
                .send_notification("$/cancelRequest", json!({ "id": id }))
                .await;
        }
    }

    async fn convert_response(
        &self,
        uri: &str,
        snapshot: &BufferSnapshot,
        request: &LanguageServiceRequest,
        value: Value,
    ) -> Result<LanguageServiceResponse, LanguageServiceError> {
        let response = match request {
            LanguageServiceRequest::Completion { .. } => {
                LanguageServiceResponse::Completion(parse_completion(&value))
            }
            LanguageServiceRequest::SignatureHelp { .. } => {
                LanguageServiceResponse::SignatureHelp(parse_signature_help(&value))
            }
            LanguageServiceRequest::Hover { .. } => {
                LanguageServiceResponse::Hover(parse_hover(&value))
            }
            LanguageServiceRequest::Definition { .. } => {
                LanguageServiceResponse::Definition(parse_locations(&value))
            }
            LanguageServiceRequest::References { .. } => {
                LanguageServiceResponse::References(parse_locations(&value))
            }
            LanguageServiceRequest::Rename { .. } => {
                LanguageServiceResponse::Rename(parse_workspace_edits(uri, snapshot, &value))
            }
            LanguageServiceRequest::CodeActions { .. } => {
                LanguageServiceResponse::CodeActions(parse_code_actions(uri, snapshot, &value))
            }
            LanguageServiceRequest::SemanticTokens => {
                let types = self.inner.state.lock().await.semantic_token_types.clone();
                LanguageServiceResponse::SemanticTokens(parse_semantic_tokens(
                    snapshot, &value, &types,
                ))
            }
            LanguageServiceRequest::DocumentSymbols => {
                LanguageServiceResponse::DocumentSymbols(parse_document_symbols(snapshot, &value))
            }
            LanguageServiceRequest::FoldingRanges => {
                LanguageServiceResponse::FoldingRanges(parse_folding_ranges(&value))
            }
            LanguageServiceRequest::Formatting => {
                LanguageServiceResponse::Formatting(parse_text_edits(uri, snapshot, &value))
            }
            LanguageServiceRequest::Diagnostics => LanguageServiceResponse::Diagnostics(Vec::new()),
        };
        Ok(response)
    }

    fn convert_edit_response(
        &self,
        uri: &str,
        snapshot: &BufferSnapshot,
        request: &LanguageServiceEditRequest,
        value: Value,
    ) -> Result<LanguageServiceEditResponse, LanguageServiceError> {
        Ok(match request {
            LanguageServiceEditRequest::FormatSelection { .. } => {
                LanguageServiceEditResponse::Formatting(parse_text_edits(uri, snapshot, &value))
            }
            LanguageServiceEditRequest::OrganizeImports => {
                LanguageServiceEditResponse::OrganizeImports(parse_organize_import_edits(
                    uri, snapshot, &value,
                )?)
            }
        })
    }
}

async fn write_message(
    writer: &Arc<Mutex<ChildStdin>>,
    message: &Value,
) -> Result<(), LanguageServiceError> {
    let body = serde_json::to_vec(message)
        .map_err(|error| LanguageServiceError::ProviderFailed(error.to_string()))?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut writer = writer.lock().await;
    writer
        .write_all(header.as_bytes())
        .await
        .map_err(|_| LanguageServiceError::ProcessNotRunning)?;
    writer
        .write_all(&body)
        .await
        .map_err(|_| LanguageServiceError::ProcessNotRunning)?;
    writer
        .flush()
        .await
        .map_err(|_| LanguageServiceError::ProcessNotRunning)
}

async fn read_process_output(
    inner: Arc<ProcessInner>,
    process_generation: u64,
    mut stdout: ChildStdout,
) {
    let mut framer = ContentLengthFramer::new(inner.config.max_frame_bytes);
    let mut bytes = [0_u8; 8192];
    loop {
        let count = match stdout.read(&mut bytes).await {
            Ok(0) => {
                if let Some(error) = framer.finish() {
                    let _ = inner.events.send(LanguageServiceEvent::MalformedFrame {
                        server: inner.config.server.clone(),
                        process_generation,
                        detail: format!("{error:?}"),
                    });
                }
                let _ = inner.events.send(LanguageServiceEvent::ProtocolEof {
                    server: inner.config.server.clone(),
                    process_generation,
                });
                fail_pending(&inner, LanguageServiceError::ProtocolEof).await;
                return;
            }
            Ok(count) => count,
            Err(_) => {
                fail_pending(&inner, LanguageServiceError::ProtocolEof).await;
                return;
            }
        };
        for frame in framer.push(&bytes[..count]) {
            let body = match frame {
                Ok(body) => body,
                Err(error) => {
                    let detail = format!("{error:?}");
                    let _ = inner.events.send(LanguageServiceEvent::MalformedFrame {
                        server: inner.config.server.clone(),
                        process_generation,
                        detail,
                    });
                    continue;
                }
            };
            let message = match serde_json::from_slice::<Value>(&body) {
                Ok(message) => message,
                Err(error) => {
                    let _ = inner.events.send(LanguageServiceEvent::MalformedFrame {
                        server: inner.config.server.clone(),
                        process_generation,
                        detail: error.to_string(),
                    });
                    continue;
                }
            };
            handle_message(&inner, process_generation, message).await;
        }
    }
}

async fn watch_process(
    inner: Arc<ProcessInner>,
    process_generation: u64,
    mut child: Child,
    mut stop_rx: oneshot::Receiver<()>,
) {
    let (stopped, status) = tokio::select! {
        status = child.wait() => (false, status.ok()),
        _ = &mut stop_rx => {
            let _ = child.kill().await;
            (true, child.wait().await.ok())
        }
    };
    let exit_code = status.and_then(|status| status.code());
    let (pending, ended, should_emit) = {
        let mut state = inner.state.lock().await;
        if state.process_generation != process_generation {
            return;
        }
        state.writer = None;
        state.stop = None;
        state.stopping = false;
        state.lifecycle = if stopped {
            ProcessLifecycleState::Stopped
        } else {
            ProcessLifecycleState::Crashed
        };
        let ended = state.ended.take();
        let pending = std::mem::take(&mut state.pending);
        (pending, ended, true)
    };
    for (_, pending) in pending {
        let error = if stopped {
            LanguageServiceError::ProcessNotRunning
        } else {
            LanguageServiceError::ProcessCrashed(exit_code)
        };
        let _ = pending.sender.send(Err(error));
    }
    if let Some(ended) = ended {
        ended.notify_one();
    }
    if should_emit {
        let event = if stopped {
            LanguageServiceEvent::Stopped {
                server: inner.config.server.clone(),
                process_generation,
            }
        } else {
            LanguageServiceEvent::Crashed {
                server: inner.config.server.clone(),
                process_generation,
                exit_code,
            }
        };
        let _ = inner.events.send(event);
    }
}

async fn fail_pending(inner: &Arc<ProcessInner>, error: LanguageServiceError) {
    let pending = std::mem::take(&mut inner.state.lock().await.pending);
    for (_, pending) in pending {
        let _ = pending.sender.send(Err(error.clone()));
    }
}

async fn handle_message(inner: &Arc<ProcessInner>, process_generation: u64, message: Value) {
    if inner.state.lock().await.process_generation != process_generation {
        return;
    }
    if let Some(id) = message.get("id").and_then(Value::as_u64) {
        if message.get("method").is_some() {
            let error = json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": "method not supported by Labonair" },
            });
            if let Some(writer) = inner.state.lock().await.writer.clone() {
                let _ = write_message(&writer, &error).await;
            }
            return;
        }
        let pending = inner.state.lock().await.pending.remove(&id);
        if let Some(pending) = pending {
            let response = if let Some(error) = message.get("error") {
                Err(LanguageServiceError::ProviderFailed(
                    error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("language-server request failed")
                        .to_string(),
                ))
            } else {
                Ok(message.get("result").cloned().unwrap_or(Value::Null))
            };
            let _ = pending.sender.send(response);
        }
        return;
    }
    if message.get("method").and_then(Value::as_str) != Some("textDocument/publishDiagnostics") {
        return;
    }
    let Some(params) = message.get("params") else {
        return;
    };
    let Some(uri) = params.get("uri").and_then(Value::as_str) else {
        return;
    };
    if let Some(version) = params.get("version").and_then(Value::as_u64) {
        let current = inner
            .state
            .lock()
            .await
            .documents
            .get(uri)
            .map(|document| document.version.value());
        if current != Some(version) {
            return;
        }
    }
    let diagnostics = parse_diagnostics(inner, params.get("diagnostics").unwrap_or(&Value::Null));
    let mut state = inner.state.lock().await;
    let Some(document) = state.documents.get_mut(uri) else {
        return;
    };
    document.diagnostics = diagnostics.clone();
    let _ = inner.events.send(LanguageServiceEvent::Diagnostics {
        server: inner.config.server.clone(),
        uri: uri.to_string(),
        version: document.version,
        generation: document.generation,
        diagnostics,
    });
    let _ = process_generation;
}

fn parse_server_capabilities(value: &Value) -> (LanguageServiceCapabilities, Vec<String>) {
    let capabilities = value.get("capabilities").unwrap_or(&Value::Null);
    let semantic = capabilities.get("semanticTokensProvider");
    let token_types = semantic
        .and_then(|provider| provider.get("legend"))
        .and_then(|legend| legend.get("tokenTypes"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    (
        LanguageServiceCapabilities {
            diagnostics: true,
            completion: advertised(capabilities.get("completionProvider")),
            signature_help: advertised(capabilities.get("signatureHelpProvider")),
            hover: advertised(capabilities.get("hoverProvider")),
            definition: advertised(capabilities.get("definitionProvider")),
            references: advertised(capabilities.get("referencesProvider")),
            rename: advertised(capabilities.get("renameProvider")),
            code_actions: advertised(capabilities.get("codeActionProvider")),
            semantic_tokens: semantic.is_some_and(|value| !value.is_null()),
            document_symbols: advertised(capabilities.get("documentSymbolProvider")),
            folding_ranges: advertised(capabilities.get("foldingRangeProvider")),
            formatting: advertised(capabilities.get("documentFormattingProvider")),
        },
        token_types,
    )
}

fn parse_server_edit_capabilities(value: &Value) -> LanguageServiceEditCapabilities {
    let capabilities = value.get("capabilities").unwrap_or(&Value::Null);
    let range_formatting = advertised(capabilities.get("documentRangeFormattingProvider"));
    let organize_imports = match capabilities.get("codeActionProvider") {
        Some(provider) if provider.as_bool() == Some(true) => true,
        Some(provider) if provider.is_object() => provider
            .get("codeActionKinds")
            .and_then(Value::as_array)
            .map(|kinds| {
                kinds.iter().any(|kind| {
                    kind.as_str()
                        .is_some_and(|kind| kind == "source" || kind == "source.organizeImports")
                })
            })
            .unwrap_or(true),
        _ => false,
    };
    LanguageServiceEditCapabilities {
        format_selection: range_formatting,
        organize_imports,
    }
}

fn advertised(value: Option<&Value>) -> bool {
    value.is_some_and(|value| value.as_bool().unwrap_or(!value.is_null()))
}

fn request_parts(uri: &str, request: &LanguageServiceRequest) -> (&'static str, Value) {
    let text_document = json!({ "uri": uri });
    match request {
        LanguageServiceRequest::Completion { position, trigger } => (
            "textDocument/completion",
            json!({ "textDocument": text_document, "position": position_value(*position), "context": trigger.map(|character| json!({ "triggerKind": 2, "triggerCharacter": character.to_string() })).unwrap_or(json!({ "triggerKind": 1 })) }),
        ),
        LanguageServiceRequest::SignatureHelp { position, trigger } => (
            "textDocument/signatureHelp",
            json!({ "textDocument": text_document, "position": position_value(*position), "context": trigger.map(|character| json!({ "triggerKind": 2, "triggerCharacter": character.to_string() })).unwrap_or(json!({ "triggerKind": 1 })) }),
        ),
        LanguageServiceRequest::Hover { position } => (
            "textDocument/hover",
            json!({ "textDocument": text_document, "position": position_value(*position) }),
        ),
        LanguageServiceRequest::Definition { position } => (
            "textDocument/definition",
            json!({ "textDocument": text_document, "position": position_value(*position) }),
        ),
        LanguageServiceRequest::References { position } => (
            "textDocument/references",
            json!({ "textDocument": text_document, "position": position_value(*position), "context": { "includeDeclaration": true } }),
        ),
        LanguageServiceRequest::Rename { position, new_name } => (
            "textDocument/rename",
            json!({ "textDocument": text_document, "position": position_value(*position), "newName": new_name }),
        ),
        LanguageServiceRequest::CodeActions { range } => (
            "textDocument/codeAction",
            json!({ "textDocument": text_document, "range": range_value(*range), "context": { "diagnostics": [] } }),
        ),
        LanguageServiceRequest::SemanticTokens => (
            "textDocument/semanticTokens/full",
            json!({ "textDocument": text_document }),
        ),
        LanguageServiceRequest::DocumentSymbols => (
            "textDocument/documentSymbol",
            json!({ "textDocument": text_document }),
        ),
        LanguageServiceRequest::FoldingRanges => (
            "textDocument/foldingRange",
            json!({ "textDocument": text_document }),
        ),
        LanguageServiceRequest::Formatting => (
            "textDocument/formatting",
            json!({ "textDocument": text_document, "options": FormattingOptions::DEFAULT.to_value() }),
        ),
        LanguageServiceRequest::Diagnostics => ("", Value::Null),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FormattingOptions {
    tab_size: usize,
    insert_spaces: bool,
}

impl FormattingOptions {
    const DEFAULT: Self = Self {
        tab_size: 4,
        insert_spaces: true,
    };

    fn to_value(self) -> Value {
        json!({
            "tabSize": self.tab_size,
            "insertSpaces": self.insert_spaces,
        })
    }
}

fn edit_request_parts(
    uri: &str,
    snapshot: &BufferSnapshot,
    request: &LanguageServiceEditRequest,
) -> (&'static str, Value) {
    let text_document = json!({ "uri": uri });
    match request {
        LanguageServiceEditRequest::FormatSelection { range } => (
            "textDocument/rangeFormatting",
            json!({
                "textDocument": text_document,
                "range": range_value(*range),
                "options": FormattingOptions::DEFAULT.to_value(),
            }),
        ),
        LanguageServiceEditRequest::OrganizeImports => (
            "textDocument/codeAction",
            json!({
                "textDocument": text_document,
                "range": range_value(LspRange::from_text_range(snapshot, crate::TextRange::new(0, snapshot.byte_len()))),
                "context": {
                    "diagnostics": [],
                    "only": ["source.organizeImports"],
                },
            }),
        ),
    }
}

fn position_value(position: LspPosition) -> Value {
    json!({ "line": position.line, "character": position.character })
}

fn range_value(range: LspRange) -> Value {
    json!({ "start": position_value(range.start), "end": position_value(range.end) })
}

fn path_to_uri(path: &std::path::Path) -> String {
    format!("file://{}", path.to_string_lossy())
}

fn parse_diagnostics(inner: &ProcessInner, value: &Value) -> Vec<Diagnostic> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let range = parse_range(item.get("range")?)?;
            Some(Diagnostic {
                range,
                severity: match item.get("severity").and_then(Value::as_u64).unwrap_or(1) {
                    2 => DiagnosticSeverity::Warning,
                    3 => DiagnosticSeverity::Information,
                    4 => DiagnosticSeverity::Hint,
                    _ => DiagnosticSeverity::Error,
                },
                message: item.get("message")?.as_str()?.to_string(),
                source: inner.config.server.clone(),
                code: item.get("code").map(value_string),
            })
        })
        .collect()
}

fn parse_range(value: &Value) -> Option<LspRange> {
    Some(LspRange {
        start: parse_position(value.get("start")?)?,
        end: parse_position(value.get("end")?)?,
    })
}

fn parse_position(value: &Value) -> Option<LspPosition> {
    Some(LspPosition {
        line: value.get("line")?.as_u64()? as usize,
        character: value.get("character")?.as_u64()? as usize,
    })
}

fn parse_completion(value: &Value) -> CompletionList {
    let (items, incomplete) = if let Some(object) = value.as_object() {
        (
            object
                .get("items")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            object
                .get("isIncomplete")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        )
    } else {
        (value.as_array().cloned().unwrap_or_default(), false)
    };
    CompletionList {
        items: items.iter().map(parse_completion_item).collect(),
        is_incomplete: incomplete,
    }
}

fn parse_completion_item(value: &Value) -> CompletionItem {
    let label = value_string(value.get("label").unwrap_or(&Value::Null));
    CompletionItem {
        insert_text: value
            .get("textEdit")
            .and_then(|edit| edit.get("newText"))
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                value
                    .get("insertText")
                    .and_then(Value::as_str)
                    .unwrap_or(&label)
            })
            .to_string(),
        kind: match value.get("kind").and_then(Value::as_u64).unwrap_or(1) {
            2 | 3 | 23 => CompletionKind::Function,
            4 | 7 | 8 | 9 | 13 | 16 | 17 | 18 | 19 | 20 | 21 | 22 | 25 => CompletionKind::Type,
            5 | 10 => CompletionKind::Field,
            6 | 12 => CompletionKind::Variable,
            14 => CompletionKind::Keyword,
            15 => CompletionKind::Snippet,
            _ => CompletionKind::Text,
        },
        label,
        detail: value
            .get("detail")
            .and_then(Value::as_str)
            .map(str::to_string),
        filter_text: value
            .get("filterText")
            .and_then(Value::as_str)
            .map(str::to_string),
        sort_text: value
            .get("sortText")
            .and_then(Value::as_str)
            .map(str::to_string),
        documentation: value.get("documentation").map(markup_content),
    }
}

fn parse_signature_help(value: &Value) -> Option<SignatureHelp> {
    let signature = value.get("signatures")?.as_array()?.first()?;
    Some(SignatureHelp {
        label: signature.get("label")?.as_str()?.to_string(),
        documentation: signature.get("documentation").map(markup_content),
    })
}

fn parse_hover(value: &Value) -> Option<HoverContent> {
    if value.is_null() {
        return None;
    }
    Some(HoverContent {
        contents: value
            .get("contents")
            .map(markup_content)
            .unwrap_or_default(),
        range: value.get("range").and_then(parse_range),
    })
}

fn markup_content(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    if let Some(value) = value.get("value").and_then(Value::as_str) {
        return value.to_string();
    }
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .map(markup_content)
            .collect::<Vec<_>>()
            .join("\n");
    }
    value_string(value)
}

fn parse_locations(value: &Value) -> Vec<NavigationTarget> {
    let values = value.as_array().cloned().unwrap_or_else(|| {
        if value.is_object() {
            vec![value.clone()]
        } else {
            Vec::new()
        }
    });
    values
        .iter()
        .filter_map(|item| {
            let range = item
                .get("range")
                .and_then(parse_range)
                .or_else(|| item.get("targetSelectionRange").and_then(parse_range))?;
            let selection_range = item
                .get("selectionRange")
                .and_then(parse_range)
                .or_else(|| item.get("targetSelectionRange").and_then(parse_range))
                .unwrap_or(range);
            Some(NavigationTarget {
                name: item
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("location")
                    .to_string(),
                range,
                selection_range,
            })
        })
        .collect()
}

fn parse_workspace_edits(uri: &str, snapshot: &BufferSnapshot, value: &Value) -> Vec<Edit> {
    let mut edits = parse_text_edits(
        uri,
        snapshot,
        value
            .get("changes")
            .and_then(|changes| changes.get(uri))
            .unwrap_or(&Value::Null),
    );
    if let Some(changes) = value.get("documentChanges").and_then(Value::as_array) {
        for change in changes {
            if change
                .get("uri")
                .or_else(|| {
                    change
                        .get("textDocument")
                        .and_then(|document| document.get("uri"))
                })
                .and_then(Value::as_str)
                == Some(uri)
            {
                edits.extend(parse_text_edits(
                    uri,
                    snapshot,
                    change.get("edits").unwrap_or(&Value::Null),
                ));
            }
        }
    }
    edits
}

fn parse_organize_import_edits(
    uri: &str,
    snapshot: &BufferSnapshot,
    value: &Value,
) -> Result<Vec<Edit>, LanguageServiceError> {
    let Some(actions) = value.as_array() else {
        return Ok(Vec::new());
    };
    let mut edits = Vec::new();
    let mut command_only = false;
    for action in actions {
        if let Some(edit) = action.get("edit") {
            edits.extend(parse_workspace_edits(uri, snapshot, edit));
        } else if action.get("command").is_some() {
            command_only = true;
        }
    }
    if edits.is_empty() && command_only {
        return Err(LanguageServiceError::ProviderFailed(
            "organize imports returned a command without a workspace edit".into(),
        ));
    }
    Ok(edits)
}

fn parse_text_edits(_uri: &str, snapshot: &BufferSnapshot, value: &Value) -> Vec<Edit> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(Edit::new(
                item.get("range")
                    .and_then(parse_range)?
                    .to_text_range(snapshot),
                item.get("newText")?.as_str()?,
            ))
        })
        .collect()
}

fn parse_code_actions(uri: &str, snapshot: &BufferSnapshot, value: &Value) -> Vec<CodeAction> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let edits = item
                .get("edit")
                .map(|edit| parse_workspace_edits(uri, snapshot, edit))
                .unwrap_or_default();
            Some(CodeAction {
                title: item.get("title")?.as_str()?.to_string(),
                kind: item
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("quickfix")
                    .to_string(),
                edits,
            })
        })
        .collect()
}

fn parse_semantic_tokens(
    snapshot: &BufferSnapshot,
    value: &Value,
    token_types: &[String],
) -> Vec<SemanticToken> {
    let Some(data) = value.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut line = 0usize;
    let mut character = 0usize;
    data.chunks(5)
        .filter_map(|chunk| {
            if chunk.len() != 5 {
                return None;
            }
            let delta_line = chunk[0].as_u64()? as usize;
            let delta_character = chunk[1].as_u64()? as usize;
            let length = chunk[2].as_u64()? as usize;
            line += delta_line;
            character = if delta_line == 0 {
                character + delta_character
            } else {
                delta_character
            };
            let start = LspPosition { line, character };
            let end = LspPosition {
                line,
                character: character + length,
            };
            Some(SemanticToken {
                range: LspRange { start, end },
                token_type: token_types
                    .get(chunk[3].as_u64()? as usize)
                    .cloned()
                    .unwrap_or_else(|| chunk[3].to_string()),
                modifiers: semantic_modifiers(chunk[4].as_u64().unwrap_or(0)),
            })
        })
        .filter(|token| token.range.to_text_range(snapshot).start <= snapshot.byte_len())
        .collect()
}

fn semantic_modifiers(mask: u64) -> Vec<String> {
    if mask == 0 {
        Vec::new()
    } else {
        vec![format!("bitmask:{mask}")]
    }
}

fn parse_document_symbols(snapshot: &BufferSnapshot, value: &Value) -> Vec<DocumentSymbol> {
    fn recurse(snapshot: &BufferSnapshot, values: &[Value], output: &mut Vec<DocumentSymbol>) {
        for value in values {
            let location = value.get("location").unwrap_or(value);
            let Some(range) = location.get("range").and_then(parse_range) else {
                continue;
            };
            let selection = value
                .get("selectionRange")
                .and_then(parse_range)
                .unwrap_or(range);
            let name = value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("symbol")
                .to_string();
            output.push(DocumentSymbol {
                name,
                kind: symbol_kind(value.get("kind").and_then(Value::as_u64)),
                line: range.start.line,
                range: range.to_text_range(snapshot),
                name_range: selection.to_text_range(snapshot),
            });
            if let Some(children) = value.get("children").and_then(Value::as_array) {
                recurse(snapshot, children, output);
            }
        }
    }
    let values = value.as_array().cloned().unwrap_or_default();
    let mut output = Vec::new();
    recurse(snapshot, &values, &mut output);
    output
}

fn symbol_kind(kind: Option<u64>) -> SymbolKind {
    match kind.unwrap_or(0) {
        2 => SymbolKind::Module,
        5 | 6 | 11 => SymbolKind::Class,
        12 | 13 | 23 => SymbolKind::Function,
        9 | 10 => SymbolKind::Constant,
        7 | 8 => SymbolKind::Type,
        _ => SymbolKind::Other,
    }
}

fn parse_folding_ranges(value: &Value) -> Vec<FoldingRange> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(FoldingRange {
                start_line: item.get("startLine")?.as_u64()? as usize,
                end_line: item.get("endLine")?.as_u64()? as usize,
                kind: item.get("kind").and_then(Value::as_str).map(str::to_string),
            })
        })
        .collect()
}

fn value_string(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_i64().map(|number| number.to_string()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EditorBuffer;

    fn snapshot(text: &str) -> BufferSnapshot {
        EditorBuffer::from_text(text).snapshot()
    }

    #[test]
    fn framing_handles_split_headers_and_bodies() {
        let mut framer = ContentLengthFramer::default();
        assert!(framer.push(b"Content-Length: 7\r\n\r").is_empty());
        assert_eq!(framer.push(b"\n{\"x\":1}").len(), 1);
    }

    #[test]
    fn malformed_header_recovers_for_next_frame() {
        let mut framer = ContentLengthFramer::default();
        let frames = framer.push(b"Nope: x\r\n\r\nContent-Length: 2\r\n\r\n{} ");
        assert!(matches!(
            frames.first(),
            Some(Err(FrameError::MissingContentLength))
        ));
        assert_eq!(frames.iter().filter(|frame| frame.is_ok()).count(), 1);
    }

    #[test]
    fn lsp_mapping_covers_common_values() {
        let snap = snapshot("fn main() {}\n");
        let completion = parse_completion(&json!([{ "label": "main", "kind": 3 }]));
        assert_eq!(completion.items[0].kind, CompletionKind::Function);
        let edits = parse_text_edits(
            "file:///x",
            &snap,
            &json!([{ "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 2 } }, "newText": "pub" }]),
        );
        assert_eq!(edits[0].replacement, "pub");
        assert_eq!(parse_locations(&json!({ "uri": "file:///x", "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 2 } } }))[0].range.start.line, 0);
    }

    #[test]
    fn advertised_capabilities_are_explicit() {
        let (capabilities, types) = parse_server_capabilities(
            &json!({ "capabilities": { "hoverProvider": true, "semanticTokensProvider": { "legend": { "tokenTypes": ["type"] } } } }),
        );
        assert!(capabilities.hover);
        assert!(!capabilities.completion);
        assert_eq!(types, vec!["type"]);
    }

    #[test]
    fn edit_request_mapping_uses_typed_ranges_and_protocol_options() {
        let snap = snapshot("use z::Z;\nuse a::A;\n");
        let range = LspRange {
            start: LspPosition {
                line: 0,
                character: 2,
            },
            end: LspPosition {
                line: 1,
                character: 8,
            },
        };
        let (method, params) = edit_request_parts(
            "file:///fixture",
            &snap,
            &LanguageServiceEditRequest::FormatSelection { range },
        );
        assert_eq!(method, "textDocument/rangeFormatting");
        assert_eq!(params["textDocument"]["uri"], "file:///fixture");
        assert_eq!(params["range"], range_value(range));
        assert_eq!(
            params["options"],
            json!({ "tabSize": 4, "insertSpaces": true })
        );

        let (method, params) = edit_request_parts(
            "file:///fixture",
            &snap,
            &LanguageServiceEditRequest::OrganizeImports,
        );
        assert_eq!(method, "textDocument/codeAction");
        assert_eq!(
            params["range"]["start"],
            json!({ "line": 0, "character": 0 })
        );
        assert_eq!(params["range"]["end"], json!({ "line": 2, "character": 0 }));
        assert_eq!(params["context"]["only"], json!(["source.organizeImports"]));
    }

    #[test]
    fn edit_capability_parsing_requires_the_advertised_operation() {
        let capabilities = parse_server_edit_capabilities(&json!({
            "capabilities": {
                "documentRangeFormattingProvider": true,
                "codeActionProvider": { "codeActionKinds": ["quickfix", "source.organizeImports"] }
            }
        }));
        assert!(capabilities.format_selection);
        assert!(capabilities.organize_imports);

        let capabilities = parse_server_edit_capabilities(&json!({
            "capabilities": {
                "documentRangeFormattingProvider": false,
                "codeActionProvider": { "codeActionKinds": ["quickfix"] }
            }
        }));
        assert!(!capabilities.format_selection);
        assert!(!capabilities.organize_imports);
    }
}
