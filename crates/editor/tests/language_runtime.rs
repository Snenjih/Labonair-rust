use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use labonair_editor::{
    BackgroundEditRequest, CancellationToken, DocumentVersion, EditorBuffer, Language, LanguageId,
    LanguageServerId, LanguageServerTimeoutPolicy, LanguageServiceEditRequest,
    LanguageServiceEditResponse, LanguageServiceRequest, LanguageServiceResponse,
    LanguageServiceRuntime, LanguageServiceRuntimeSource, LanguageServiceRuntimeStatus,
    LocalLanguageServerConfig, LocalLanguageServiceRequest, LspPosition, LspRange,
    ProjectRootPolicy,
};

fn config() -> LocalLanguageServerConfig {
    config_with_args(Vec::new())
}

fn config_with_args(args: Vec<String>) -> LocalLanguageServerConfig {
    LocalLanguageServerConfig {
        command: env!("CARGO_BIN_EXE_labonair-editor-lsp-fixture").into(),
        args,
        env: Vec::new(),
        cwd: PathBuf::from("/tmp"),
        project_root_policy: ProjectRootPolicy::Trusted,
        language: LanguageId::new(Language::Rust),
        server: LanguageServerId::new("runtime-fixture"),
        timeouts: LanguageServerTimeoutPolicy {
            initialize: Duration::from_secs(2),
            request: Duration::from_secs(1),
            shutdown: Duration::from_secs(1),
        },
        max_frame_bytes: 1024 * 1024,
    }
}

#[tokio::test]
async fn enabled_process_is_selected_and_syncs_requests() {
    let runtime = Arc::new(LanguageServiceRuntime::default());
    let process = runtime
        .spawn_process(config())
        .await
        .expect("fixture starts");
    let selected = runtime.snapshot(Language::Rust, DocumentVersion::new(1), 1);
    assert_eq!(selected.source, LanguageServiceRuntimeSource::LocalProcess);
    assert_eq!(selected.status, LanguageServiceRuntimeStatus::Running);
    assert!(selected.capabilities.completion);

    let snapshot = EditorBuffer::from_text("fn main() {}").snapshot();
    let (open_tx, open_rx) = tokio::sync::oneshot::channel();
    runtime.open_document_in_background(
        "file:///runtime.rs".into(),
        LanguageId::new(Language::Rust),
        snapshot.clone(),
        DocumentVersion::new(1),
        1,
        move |result| {
            let _ = open_tx.send(result);
        },
    );
    open_rx.await.expect("open callback").expect("didOpen");

    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    runtime.request_in_background(
        LanguageId::new(Language::Rust),
        LocalLanguageServiceRequest {
            uri: "file:///runtime.rs".into(),
            snapshot,
            version: DocumentVersion::new(1),
            generation: 1,
            request: LanguageServiceRequest::Completion {
                position: labonair_editor::LspPosition {
                    line: 0,
                    character: 1,
                },
                trigger: None,
            },
            cancellation: CancellationToken::default(),
        },
        move |result| {
            let _ = result_tx.send(result);
        },
    );
    let result = result_rx.await.expect("request callback");
    assert!(matches!(
        result.response,
        Ok(LanguageServiceResponse::Completion(list)) if list.items[0].label == "fixture"
    ));
    process.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn stale_runtime_requests_are_rejected_before_syntax_fallback() {
    let runtime = Arc::new(LanguageServiceRuntime::default());
    let snapshot = EditorBuffer::from_text("fn main() {}").snapshot();
    let (tx, rx) = tokio::sync::oneshot::channel();
    runtime.open_document_in_background(
        "file:///runtime.rs".into(),
        LanguageId::new(Language::Rust),
        snapshot.clone(),
        DocumentVersion::new(1),
        1,
        move |_| {},
    );
    runtime.change_document_in_background(
        "file:///runtime.rs".into(),
        LanguageId::new(Language::Rust),
        EditorBuffer::from_text("fn changed() {}").snapshot(),
        DocumentVersion::new(2),
        2,
        move |_| {},
    );
    runtime.request_in_background(
        LanguageId::new(Language::Rust),
        LocalLanguageServiceRequest {
            uri: "file:///runtime.rs".into(),
            snapshot,
            version: DocumentVersion::new(1),
            generation: 1,
            request: LanguageServiceRequest::Diagnostics,
            cancellation: CancellationToken::default(),
        },
        move |result| {
            let _ = tx.send(result);
        },
    );
    let result = rx.await.expect("stale callback");
    assert!(matches!(
        result.response,
        Err(labonair_editor::LanguageServiceError::StaleDocument { .. })
    ));
}

#[tokio::test]
async fn edit_runtime_uses_process_capability_and_falls_back_to_syntax() {
    let runtime = Arc::new(LanguageServiceRuntime::default());
    let process = runtime
        .spawn_process(config())
        .await
        .expect("fixture starts");
    let snapshot = EditorBuffer::from_text("fn f() {  \n}").snapshot();
    let (open_tx, open_rx) = tokio::sync::oneshot::channel();
    runtime.open_document_in_background(
        "file:///runtime.rs".into(),
        LanguageId::new(Language::Rust),
        snapshot.clone(),
        DocumentVersion::new(1),
        1,
        move |result| {
            let _ = open_tx.send(result);
        },
    );
    open_rx.await.expect("open callback").expect("open");
    let (tx, rx) = tokio::sync::oneshot::channel();
    runtime.request_edit_in_background(
        BackgroundEditRequest {
            uri: "file:///runtime.rs".into(),
            language: LanguageId::new(Language::Rust),
            snapshot: snapshot.clone(),
            version: DocumentVersion::new(1),
            generation: 1,
            request: LanguageServiceEditRequest::FormatSelection {
                range: LspRange {
                    start: LspPosition {
                        line: 0,
                        character: 0,
                    },
                    end: LspPosition {
                        line: 0,
                        character: 10,
                    },
                },
            },
            cancellation: CancellationToken::default(),
        },
        move |result| {
            let _ = tx.send(result);
        },
    );
    let result = rx.await.expect("process edit callback");
    assert!(matches!(
        result.response,
        Ok(LanguageServiceEditResponse::Formatting(ref edits))
            if edits.len() == 1 && edits[0].replacement == "range"
    ));
    process.shutdown().await.expect("shutdown");

    let runtime = Arc::new(LanguageServiceRuntime::default());
    let process = runtime
        .spawn_process(config_with_args(vec!["no-range-formatting".into()]))
        .await
        .expect("fixture starts");
    let snapshot = EditorBuffer::from_text("fn f() {  \n}").snapshot();
    let (open_tx, open_rx) = tokio::sync::oneshot::channel();
    runtime.open_document_in_background(
        "file:///fallback.rs".into(),
        LanguageId::new(Language::Rust),
        snapshot.clone(),
        DocumentVersion::new(1),
        1,
        move |result| {
            let _ = open_tx.send(result);
        },
    );
    open_rx.await.expect("open callback").expect("open");
    let (tx, rx) = tokio::sync::oneshot::channel();
    runtime.request_edit_in_background(
        BackgroundEditRequest {
            uri: "file:///fallback.rs".into(),
            language: LanguageId::new(Language::Rust),
            snapshot,
            version: DocumentVersion::new(1),
            generation: 1,
            request: LanguageServiceEditRequest::FormatSelection {
                range: LspRange {
                    start: LspPosition {
                        line: 0,
                        character: 0,
                    },
                    end: LspPosition {
                        line: 0,
                        character: 10,
                    },
                },
            },
            cancellation: CancellationToken::default(),
        },
        move |result| {
            let _ = tx.send(result);
        },
    );
    let result = rx.await.expect("fallback edit callback");
    assert!(matches!(
        result.response,
        Ok(LanguageServiceEditResponse::Formatting(ref edits)) if !edits.is_empty()
    ));
    assert_ne!(result.server, LanguageServerId::new("runtime-fixture"));
    process.shutdown().await.expect("shutdown");
}
