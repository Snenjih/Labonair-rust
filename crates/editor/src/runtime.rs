//! Editor-owned composition of syntax providers and optional local processes.
//!
//! [`LocalLanguageServiceProcess`] owns JSON-RPC and child-process I/O.  This
//! module owns the product decision about which provider serves a document,
//! its lifecycle projection, and the revision-safe background request entry
//! point used by a GPUI adapter.  Workspace never stores or dispatches LSP
//! state; it only injects a configured runtime into an editor view.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use tokio::runtime::Handle;

use crate::buffer::BufferSnapshot;
use crate::language::Language;
use crate::language_services::{
    default_registry, BackgroundRequest, DocumentVersion, LanguageId, LanguageServerId,
    LanguageServiceCapabilities, LanguageServiceEditRequest, LanguageServiceEditResponse,
    LanguageServiceError, LanguageServiceRegistry, LanguageServiceRequest, LanguageServiceResult,
};
use crate::lsp_process::{
    LanguageServiceEvent, LocalLanguageServiceEditRequest, LocalLanguageServiceProcess,
    LocalLanguageServiceRequest,
};

/// Presentation and request policy.  Executable paths are intentionally not
/// settings values: the application must inject a validated process handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageServiceRuntimePolicy {
    pub enabled: bool,
    pub diagnostics: bool,
    pub completion: bool,
    pub hover: bool,
    pub semantic_tokens: bool,
}

impl Default for LanguageServiceRuntimePolicy {
    fn default() -> Self {
        Self {
            // A local executable is never launched or selected implicitly.
            // Explicit process registration is the opt-in boundary.
            enabled: false,
            diagnostics: true,
            completion: true,
            hover: true,
            semantic_tokens: true,
        }
    }
}

impl LanguageServiceRuntimePolicy {
    fn permits(self, request: &LanguageServiceRequest) -> bool {
        match request.capability() {
            crate::LanguageServiceCapability::Diagnostics => self.diagnostics,
            crate::LanguageServiceCapability::Completion => self.completion,
            crate::LanguageServiceCapability::Hover => self.hover,
            crate::LanguageServiceCapability::SemanticTokens => self.semantic_tokens,
            _ => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageServiceRuntimeSource {
    LocalProcess,
    SyntaxFallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageServiceRuntimeStatus {
    Disabled,
    Starting,
    Running,
    Crashed,
    Fallback,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageServiceRuntimeSnapshot {
    pub language: LanguageId,
    pub server: LanguageServerId,
    pub source: LanguageServiceRuntimeSource,
    pub status: LanguageServiceRuntimeStatus,
    pub capabilities: LanguageServiceCapabilities,
    pub version: DocumentVersion,
    pub generation: u64,
    pub detail: Option<String>,
}

/// Revision-bound result for an Editor-owned edit request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageServiceEditResult {
    pub server: LanguageServerId,
    pub version: DocumentVersion,
    pub generation: u64,
    pub response: Result<LanguageServiceEditResponse, LanguageServiceError>,
}

/// Immutable input captured before an asynchronous edit request starts.
#[derive(Clone, Debug)]
pub struct BackgroundEditRequest {
    pub uri: String,
    pub language: LanguageId,
    pub snapshot: BufferSnapshot,
    pub version: DocumentVersion,
    pub generation: u64,
    pub request: LanguageServiceEditRequest,
    pub cancellation: crate::CancellationToken,
}

#[derive(Clone)]
struct DocumentRecord {
    version: DocumentVersion,
    generation: u64,
}

struct RuntimeInner {
    registry: Arc<LanguageServiceRegistry>,
    policy: Mutex<LanguageServiceRuntimePolicy>,
    processes: Mutex<BTreeMap<LanguageId, Arc<LocalLanguageServiceProcess>>>,
    states: Mutex<BTreeMap<LanguageId, RuntimeState>>,
    documents: Mutex<BTreeMap<String, DocumentRecord>>,
    tokio: Mutex<Option<Handle>>,
}

#[derive(Clone)]
struct RuntimeState {
    status: LanguageServiceRuntimeStatus,
    detail: Option<String>,
    capabilities: LanguageServiceCapabilities,
    server: LanguageServerId,
}

/// The single Editor-owned language-service manager for an editor surface.
#[derive(Clone)]
pub struct LanguageServiceRuntime {
    inner: Arc<RuntimeInner>,
}

impl Default for LanguageServiceRuntime {
    fn default() -> Self {
        Self::new(
            Arc::new(default_registry()),
            LanguageServiceRuntimePolicy::default(),
        )
    }
}

impl LanguageServiceRuntime {
    pub fn new(
        registry: Arc<LanguageServiceRegistry>,
        policy: LanguageServiceRuntimePolicy,
    ) -> Self {
        Self {
            inner: Arc::new(RuntimeInner {
                registry,
                policy: Mutex::new(policy),
                processes: Mutex::new(BTreeMap::new()),
                states: Mutex::new(BTreeMap::new()),
                documents: Mutex::new(BTreeMap::new()),
                tokio: Mutex::new(Handle::try_current().ok()),
            }),
        }
    }

    pub fn registry(&self) -> Arc<LanguageServiceRegistry> {
        Arc::clone(&self.inner.registry)
    }

    pub fn policy(&self) -> LanguageServiceRuntimePolicy {
        *self
            .inner
            .policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set_policy(&self, policy: LanguageServiceRuntimePolicy) {
        *self
            .inner
            .policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = policy;
    }

    pub fn set_presentation_policy(
        &self,
        diagnostics: bool,
        completion: bool,
        hover: bool,
        semantic_tokens: bool,
    ) {
        let mut policy = self.policy();
        policy.diagnostics = diagnostics;
        policy.completion = completion;
        policy.hover = hover;
        policy.semantic_tokens = semantic_tokens;
        self.set_policy(policy);
    }

    /// Inject the application Tokio handle used for process-backed requests.
    /// The handle is not feature state and is never persisted.
    pub fn set_tokio_handle(&self, handle: Handle) {
        *self
            .inner
            .tokio
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(handle);
    }

    /// Register an already-started process.  Starting a process remains an
    /// explicit application action; merely constructing the runtime never
    /// searches PATH or assumes a server is installed.
    pub fn register_process(
        &self,
        process: Arc<LocalLanguageServiceProcess>,
    ) -> Result<(), LanguageServiceError> {
        let language = process.language();
        let server = process.server_id();
        self.inner
            .processes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(language, Arc::clone(&process));
        self.set_state(
            language,
            RuntimeState {
                status: LanguageServiceRuntimeStatus::Running,
                detail: None,
                capabilities: LanguageServiceCapabilities::SYNTAX_DERIVED,
                server,
            },
        );
        // Registering a concrete, already-validated process is the explicit
        // opt-in. The default runtime remains disabled until this call.
        let mut policy = self.policy();
        policy.enabled = true;
        self.set_policy(policy);
        self.monitor_process(language, process);
        Ok(())
    }

    /// Spawn and register a configured local process on the caller's Tokio
    /// runtime.  The state is visible as `Starting` before the handshake.
    pub async fn spawn_process(
        &self,
        config: crate::ProcessServerConfig,
    ) -> Result<Arc<LocalLanguageServiceProcess>, LanguageServiceError> {
        if let Ok(handle) = Handle::try_current() {
            self.set_tokio_handle(handle);
        }
        let language = config.language;
        let server = config.server.clone();
        self.set_state(
            language,
            RuntimeState {
                status: LanguageServiceRuntimeStatus::Starting,
                detail: None,
                capabilities: LanguageServiceCapabilities::SYNTAX_DERIVED,
                server: server.clone(),
            },
        );
        let process = match LocalLanguageServiceProcess::spawn(config).await {
            Ok(process) => Arc::new(process),
            Err(error) => {
                self.set_state(
                    language,
                    RuntimeState {
                        status: LanguageServiceRuntimeStatus::Crashed,
                        detail: Some(error.to_string()),
                        capabilities: LanguageServiceCapabilities::SYNTAX_DERIVED,
                        server,
                    },
                );
                return Err(error);
            }
        };
        self.register_process(Arc::clone(&process))?;
        let capabilities = process.capabilities().await;
        self.set_state(
            language,
            RuntimeState {
                status: LanguageServiceRuntimeStatus::Running,
                detail: None,
                capabilities,
                server: process.server_id(),
            },
        );
        Ok(process)
    }

    pub fn snapshot(
        &self,
        language: Language,
        version: DocumentVersion,
        generation: u64,
    ) -> LanguageServiceRuntimeSnapshot {
        let language_id = LanguageId::new(language);
        let policy = self.policy();
        let syntax = self.inner.registry.provider_for(language_id);
        let process = self
            .inner
            .processes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&language_id)
            .cloned();
        let state = self
            .inner
            .states
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&language_id)
            .cloned();
        if !policy.enabled {
            return self.fallback_snapshot(
                language_id,
                syntax,
                LanguageServiceRuntimeStatus::Disabled,
                version,
                generation,
                None,
            );
        }
        if let (Some(_process), Some(state)) = (process, state) {
            if matches!(
                state.status,
                LanguageServiceRuntimeStatus::Starting | LanguageServiceRuntimeStatus::Running
            ) {
                return LanguageServiceRuntimeSnapshot {
                    language: language_id,
                    server: state.server,
                    source: LanguageServiceRuntimeSource::LocalProcess,
                    status: state.status,
                    capabilities: state.capabilities,
                    version,
                    generation,
                    detail: state.detail,
                };
            }
            if matches!(state.status, LanguageServiceRuntimeStatus::Crashed) {
                return self.fallback_snapshot(
                    language_id,
                    syntax,
                    state.status,
                    version,
                    generation,
                    state.detail,
                );
            }
        }
        self.fallback_snapshot(
            language_id,
            syntax,
            LanguageServiceRuntimeStatus::Fallback,
            version,
            generation,
            None,
        )
    }

    fn fallback_snapshot(
        &self,
        language: LanguageId,
        syntax: Option<Arc<dyn crate::LocalLanguageService>>,
        status: LanguageServiceRuntimeStatus,
        version: DocumentVersion,
        generation: u64,
        detail: Option<String>,
    ) -> LanguageServiceRuntimeSnapshot {
        let supported = syntax.is_some();
        let server = syntax
            .as_ref()
            .map(|provider| provider.id())
            .unwrap_or_else(|| LanguageServerId::new("none"));
        let capabilities = syntax
            .as_ref()
            .map(|provider| provider.capabilities())
            .unwrap_or(LanguageServiceCapabilities {
                diagnostics: false,
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
            });
        LanguageServiceRuntimeSnapshot {
            language,
            server,
            source: LanguageServiceRuntimeSource::SyntaxFallback,
            status: if supported {
                status
            } else {
                LanguageServiceRuntimeStatus::Unsupported
            },
            capabilities,
            version,
            generation,
            detail,
        }
    }

    pub fn open_document_in_background(
        &self,
        uri: String,
        language: LanguageId,
        snapshot: BufferSnapshot,
        version: DocumentVersion,
        generation: u64,
        callback: impl FnOnce(Result<(), LanguageServiceError>) + Send + 'static,
    ) -> JoinHandle<()> {
        self.inner
            .documents
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(
                uri.clone(),
                DocumentRecord {
                    version,
                    generation,
                },
            );
        let process = self.process_for(language);
        let handle = self.process_handle();
        thread::spawn(move || {
            let result = match (process, handle) {
                (Some(process), Some(handle)) => handle.block_on(process.did_open(
                    uri,
                    language,
                    snapshot.text(),
                    version,
                    generation,
                )),
                _ => Ok(()),
            };
            callback(result);
        })
    }

    pub fn change_document_in_background(
        &self,
        uri: String,
        language: LanguageId,
        snapshot: BufferSnapshot,
        version: DocumentVersion,
        generation: u64,
        callback: impl FnOnce(Result<(), LanguageServiceError>) + Send + 'static,
    ) -> JoinHandle<()> {
        let mut documents = self
            .inner
            .documents
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(document) = documents.get_mut(&uri) {
            document.version = version;
            document.generation = generation;
        } else {
            documents.insert(
                uri.clone(),
                DocumentRecord {
                    version,
                    generation,
                },
            );
        }
        drop(documents);
        let process = self.process_for(language);
        let handle = self.process_handle();
        thread::spawn(move || {
            let result = match (process, handle) {
                (Some(process), Some(handle)) => {
                    let text = snapshot.text();
                    let result = handle.block_on(process.did_change(
                        &uri,
                        text.clone(),
                        version,
                        generation,
                    ));
                    match result {
                        Err(LanguageServiceError::ProviderFailed(_)) => handle
                            .block_on(process.did_open(uri, language, text, version, generation)),
                        other => other,
                    }
                }
                _ => Ok(()),
            };
            callback(result);
        })
    }

    pub fn close_document_in_background(
        &self,
        uri: String,
        language: LanguageId,
        callback: impl FnOnce(Result<(), LanguageServiceError>) + Send + 'static,
    ) -> JoinHandle<()> {
        self.inner
            .documents
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&uri);
        let process = self.process_for(language);
        let handle = self.process_handle();
        thread::spawn(move || {
            let result = match (process, handle) {
                (Some(process), Some(handle)) => handle.block_on(process.did_close(&uri)),
                _ => Ok(()),
            };
            callback(result);
        })
    }

    pub async fn stop_process(&self, language: LanguageId) -> Result<(), LanguageServiceError> {
        let Some(process) = self.process_for(language) else {
            return Ok(());
        };
        process.stop().await
    }

    pub async fn restart_process(&self, language: LanguageId) -> Result<(), LanguageServiceError> {
        let Some(process) = self.process_for(language) else {
            return Err(LanguageServiceError::UnsupportedLanguage(language));
        };
        process.restart().await?;
        let capabilities = process.capabilities().await;
        self.set_state(
            language,
            RuntimeState {
                status: LanguageServiceRuntimeStatus::Running,
                detail: None,
                capabilities,
                server: process.server_id(),
            },
        );
        Ok(())
    }

    /// Dispatch every supported language-service request off the GPUI thread.
    /// Process failures transparently retry against the syntax provider, so a
    /// crashed or unavailable executable never removes basic editor behavior.
    pub fn request_in_background(
        &self,
        language: LanguageId,
        input: LocalLanguageServiceRequest,
        callback: impl FnOnce(LanguageServiceResult) + Send + 'static,
    ) -> JoinHandle<()> {
        let current_document = self
            .inner
            .documents
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&input.uri)
            .map(|document| (document.version, document.generation));
        if let Some((expected_version, expected_generation)) = current_document {
            if expected_version != input.version || expected_generation != input.generation {
                return thread::spawn(move || {
                    callback(LanguageServiceResult {
                        server: LanguageServerId::new("runtime"),
                        version: input.version,
                        generation: input.generation,
                        response: Err(LanguageServiceError::StaleDocument {
                            expected: expected_version,
                            received: input.version,
                        }),
                    });
                });
            }
        }
        let policy = self.policy();
        let process = policy.enabled.then(|| self.process_for(language)).flatten();
        let process_allowed = process.is_some() && policy.permits(&input.request);
        let process_handle = self.process_handle();
        let registry = Arc::clone(&self.inner.registry);
        let version = input.version;
        let generation = input.generation;
        let request = input.request.clone();
        let fallback_input = BackgroundRequest {
            language,
            snapshot: input.snapshot.clone(),
            version: input.version,
            generation: input.generation,
            request: input.request.clone(),
            cancellation: input.cancellation.clone(),
        };
        thread::spawn(move || {
            if !policy.permits(&request) {
                callback(LanguageServiceResult {
                    server: LanguageServerId::new("policy"),
                    version,
                    generation,
                    response: Err(LanguageServiceError::UnsupportedCapability(
                        request.capability(),
                    )),
                });
                return;
            }
            let mut result = None;
            if process_allowed {
                if let (Some(process), Some(handle)) = (process, process_handle) {
                    let process_result = handle.block_on(process.request(input));
                    if process_result.response.is_ok()
                        || matches!(
                            process_result.response,
                            Err(LanguageServiceError::UnsupportedCapability(_))
                                | Err(LanguageServiceError::Cancelled)
                                | Err(LanguageServiceError::StaleDocument { .. })
                        )
                    {
                        result = Some(process_result);
                    }
                }
            }
            let result = result.unwrap_or_else(|| LanguageServiceResult {
                server: registry
                    .provider_for(language)
                    .map(|provider| provider.id())
                    .unwrap_or_else(|| LanguageServerId::new("none")),
                version,
                generation,
                response: registry.request(
                    language,
                    &fallback_input.snapshot,
                    fallback_input.version,
                    fallback_input.request,
                    &fallback_input.cancellation,
                ),
            });
            callback(result);
        })
    }

    /// Run an Editor edit provider off the GPUI thread. A local process is
    /// selected only when it advertises the requested edit capability and has
    /// one unambiguous document at the captured revision. All other outcomes
    /// use the deterministic syntax provider.
    pub fn request_edit_in_background(
        &self,
        input: BackgroundEditRequest,
        callback: impl FnOnce(LanguageServiceEditResult) + Send + 'static,
    ) -> JoinHandle<()> {
        let policy = self.policy();
        let process = policy
            .enabled
            .then(|| self.process_for(input.language))
            .flatten();
        let process_handle = self.process_handle();
        let registry = Arc::clone(&self.inner.registry);
        thread::spawn(move || {
            let request = input.request.clone();
            if let (Some(process), Some(handle)) = (process, process_handle) {
                let process_result = handle.block_on(async {
                    if !process
                        .edit_capabilities()
                        .await
                        .supports(request.capability())
                    {
                        return None;
                    }
                    Some(
                        process
                            .request_edit(LocalLanguageServiceEditRequest {
                                uri: input.uri.clone(),
                                snapshot: input.snapshot.clone(),
                                version: input.version,
                                generation: input.generation,
                                request: request.clone(),
                                cancellation: input.cancellation.clone(),
                            })
                            .await,
                    )
                });
                if let Some(result) = process_result {
                    if result.response.is_ok()
                        || matches!(
                            result.response,
                            Err(LanguageServiceError::Cancelled)
                                | Err(LanguageServiceError::StaleDocument { .. })
                        )
                    {
                        callback(LanguageServiceEditResult {
                            server: result.server,
                            version: result.version,
                            generation: result.generation,
                            response: result.response,
                        });
                        return;
                    }
                }
            }
            let server = registry
                .provider_for(input.language)
                .map(|provider| provider.id())
                .unwrap_or_else(|| LanguageServerId::new("none"));
            let response = registry.request_edit(
                input.language,
                &input.snapshot,
                input.version,
                input.request,
                &input.cancellation,
            );
            callback(LanguageServiceEditResult {
                server,
                version: input.version,
                generation: input.generation,
                response,
            });
        })
    }

    fn process_for(&self, language: LanguageId) -> Option<Arc<LocalLanguageServiceProcess>> {
        self.inner
            .processes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&language)
            .cloned()
    }

    fn process_handle(&self) -> Option<Handle> {
        self.inner
            .tokio
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn set_state(&self, language: LanguageId, state: RuntimeState) {
        self.inner
            .states
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(language, state);
    }

    fn monitor_process(&self, language: LanguageId, process: Arc<LocalLanguageServiceProcess>) {
        let Some(handle) = self.process_handle() else {
            return;
        };
        let mut events = process.subscribe();
        let runtime = self.clone();
        handle.spawn(async move {
            while let Ok(event) = events.recv().await {
                runtime.apply_process_event(language, &event);
            }
        });
    }

    fn apply_process_event(&self, language: LanguageId, event: &LanguageServiceEvent) {
        let Some(previous) = self
            .inner
            .states
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&language)
            .cloned()
        else {
            return;
        };
        let mut next = previous;
        match event {
            LanguageServiceEvent::Started { .. } => {
                next.status = LanguageServiceRuntimeStatus::Starting;
                next.detail = None;
            }
            LanguageServiceEvent::Initialized { capabilities, .. } => {
                next.status = LanguageServiceRuntimeStatus::Running;
                next.capabilities = *capabilities;
                next.detail = None;
            }
            LanguageServiceEvent::Crashed { exit_code, .. } => {
                next.status = LanguageServiceRuntimeStatus::Crashed;
                next.detail = Some(format!("language server exited ({exit_code:?})"));
            }
            LanguageServiceEvent::ProtocolEof { .. } => {
                next.status = LanguageServiceRuntimeStatus::Crashed;
                next.detail = Some("language server closed its protocol stream".into());
            }
            LanguageServiceEvent::Stopped { .. } => {
                next.status = LanguageServiceRuntimeStatus::Fallback;
                next.detail = Some("language server is stopped".into());
            }
            LanguageServiceEvent::Restarted { .. } => {
                next.status = LanguageServiceRuntimeStatus::Starting;
                next.detail = None;
            }
            LanguageServiceEvent::MalformedFrame { detail, .. } => {
                next.detail = Some(detail.clone());
            }
            LanguageServiceEvent::Diagnostics { .. } => {}
        }
        self.set_state(language, next);
    }

    #[cfg(test)]
    fn document_record(&self, uri: &str) -> Option<(DocumentVersion, u64)> {
        self.inner
            .documents
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(uri)
            .map(|record| (record.version, record.generation))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CancellationToken, EditorBuffer, LanguageServiceResponse, LspPosition};

    fn input(request: LanguageServiceRequest) -> LocalLanguageServiceRequest {
        LocalLanguageServiceRequest {
            uri: "untitled://runtime-test".into(),
            snapshot: EditorBuffer::from_text("fn main() {}").snapshot(),
            version: DocumentVersion::new(1),
            generation: 1,
            request,
            cancellation: CancellationToken::default(),
        }
    }

    #[test]
    fn default_policy_is_safe_without_server_configuration() {
        let policy = LanguageServiceRuntimePolicy::default();
        assert!(!policy.enabled);
        assert!(policy.diagnostics);
        assert!(policy.completion);
        assert!(policy.hover);
        assert!(policy.semantic_tokens);
    }

    #[test]
    fn syntax_fallback_is_selected_without_a_process() {
        let runtime = LanguageServiceRuntime::default();
        let mut policy = runtime.policy();
        policy.enabled = true;
        runtime.set_policy(policy);
        let state = runtime.snapshot(Language::Rust, DocumentVersion::new(1), 1);
        assert_eq!(state.source, LanguageServiceRuntimeSource::SyntaxFallback);
        assert_eq!(state.status, LanguageServiceRuntimeStatus::Fallback);
        assert_eq!(state.server.as_str(), "syntax-rust");
    }

    #[test]
    fn disabled_policy_is_explicit_but_keeps_syntax_fallback() {
        let runtime = LanguageServiceRuntime::default();
        runtime.set_policy(LanguageServiceRuntimePolicy {
            enabled: false,
            ..Default::default()
        });
        let state = runtime.snapshot(Language::Rust, DocumentVersion::new(1), 1);
        assert_eq!(state.source, LanguageServiceRuntimeSource::SyntaxFallback);
        assert_eq!(state.status, LanguageServiceRuntimeStatus::Disabled);
    }

    #[test]
    fn document_sync_tracks_version_and_generation() {
        let runtime = LanguageServiceRuntime::default();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let handle = runtime.open_document_in_background(
            "untitled://runtime-test".into(),
            LanguageId::new(Language::Rust),
            EditorBuffer::from_text("fn main() {}").snapshot(),
            DocumentVersion::new(2),
            7,
            move |result| {
                let _ = sender.send(result.is_ok());
            },
        );
        handle.join().expect("fallback sync thread must finish");
        assert!(receiver.recv().expect("sync result"));
        assert_eq!(
            runtime.document_record("untitled://runtime-test"),
            Some((DocumentVersion::new(2), 7))
        );
    }

    #[test]
    fn requests_are_backgrounded_and_revision_safe() {
        let runtime = LanguageServiceRuntime::default();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let handle = runtime.request_in_background(
            LanguageId::new(Language::Rust),
            input(LanguageServiceRequest::Diagnostics),
            move |result| {
                sender.send(result).ok();
            },
        );
        handle.join().expect("request thread must finish");
        let result = receiver.recv().expect("request result");
        assert!(matches!(
            result.response,
            Ok(LanguageServiceResponse::Diagnostics(_))
        ));
        assert_eq!(result.version, DocumentVersion::new(1));
        let _ = LspPosition {
            line: 0,
            character: 0,
        };
    }
}
