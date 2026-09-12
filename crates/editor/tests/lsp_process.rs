use std::path::PathBuf;
use std::time::Duration;

use labonair_editor::{
    CancellationToken, DocumentVersion, EditorBuffer, Language, LanguageId, LanguageServerId,
    LanguageServerTimeoutPolicy, LanguageServiceEditCapability, LanguageServiceEditRequest,
    LanguageServiceEditResponse, LanguageServiceEvent, LanguageServiceRequest,
    LanguageServiceResponse, LocalLanguageServerConfig, LocalLanguageServiceEditRequest,
    LocalLanguageServiceProcess, LocalLanguageServiceRequest, LspPosition, LspRange,
    ProjectRootPolicy,
};

fn config() -> LocalLanguageServerConfig {
    config_with(Vec::new())
}

fn config_with(args: Vec<String>) -> LocalLanguageServerConfig {
    LocalLanguageServerConfig {
        command: env!("CARGO_BIN_EXE_labonair-editor-lsp-fixture").into(),
        args,
        env: Vec::new(),
        cwd: PathBuf::from("/tmp"),
        project_root_policy: ProjectRootPolicy::Trusted,
        language: LanguageId::new(Language::Rust),
        server: LanguageServerId::new("fixture"),
        timeouts: LanguageServerTimeoutPolicy {
            initialize: Duration::from_secs(2),
            request: Duration::from_secs(1),
            shutdown: Duration::from_secs(1),
        },
        max_frame_bytes: 1024 * 1024,
    }
}

#[tokio::test]
async fn process_handshake_sync_and_response_correlation_work() {
    let process = LocalLanguageServiceProcess::spawn(config())
        .await
        .expect("fixture starts");
    let mut events = process.subscribe();
    process
        .did_open(
            "file:///fixture".into(),
            LanguageId::new(Language::Rust),
            "fn f() {}".into(),
            DocumentVersion::new(1),
            1,
        )
        .await
        .expect("open");
    let diagnostics = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let LanguageServiceEvent::Diagnostics { version, .. } =
                events.recv().await.expect("event")
            {
                break version;
            }
        }
    })
    .await
    .expect("diagnostic event");
    assert_eq!(diagnostics, DocumentVersion::new(1));
    process
        .did_change(
            "file:///fixture",
            "fn changed() {}".into(),
            DocumentVersion::new(2),
            2,
        )
        .await
        .expect("change");
    let changed_diagnostics = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let LanguageServiceEvent::Diagnostics { version, .. } =
                events.recv().await.expect("event")
            {
                if version == DocumentVersion::new(2) {
                    break version;
                }
            }
        }
    })
    .await
    .expect("changed diagnostic event");
    assert_eq!(changed_diagnostics, DocumentVersion::new(2));
    let result = process
        .request(LocalLanguageServiceRequest {
            uri: "file:///fixture".into(),
            snapshot: labonair_editor::EditorBuffer::from_text("fn changed() {}").snapshot(),
            version: DocumentVersion::new(2),
            generation: 2,
            request: LanguageServiceRequest::Completion {
                position: labonair_editor::LspPosition {
                    line: 0,
                    character: 1,
                },
                trigger: None,
            },
            cancellation: CancellationToken::default(),
        })
        .await;
    assert!(
        matches!(result.response, Ok(LanguageServiceResponse::Completion(list)) if list.items[0].label == "fixture")
    );
    process.did_close("file:///fixture").await.expect("close");
    process.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn stale_versions_and_unadvertised_capabilities_are_typed() {
    let process = LocalLanguageServiceProcess::spawn(config_with(vec!["minimal".into()]))
        .await
        .expect("fixture starts");
    process
        .did_open(
            "file:///fixture".into(),
            LanguageId::new(Language::Rust),
            "f".into(),
            DocumentVersion::new(2),
            4,
        )
        .await
        .expect("open");
    let result = process
        .request(LocalLanguageServiceRequest {
            uri: "file:///fixture".into(),
            snapshot: labonair_editor::EditorBuffer::from_text("f").snapshot(),
            version: DocumentVersion::new(1),
            generation: 3,
            request: LanguageServiceRequest::Formatting,
            cancellation: CancellationToken::default(),
        })
        .await;
    assert!(matches!(
        result.response,
        Err(labonair_editor::LanguageServiceError::StaleDocument { .. })
    ));
    let result = process
        .request(LocalLanguageServiceRequest {
            uri: "file:///fixture".into(),
            snapshot: labonair_editor::EditorBuffer::from_text("f").snapshot(),
            version: DocumentVersion::new(2),
            generation: 4,
            request: LanguageServiceRequest::Formatting,
            cancellation: CancellationToken::default(),
        })
        .await;
    assert!(matches!(
        result.response,
        Err(
            labonair_editor::LanguageServiceError::UnsupportedCapability(
                labonair_editor::LanguageServiceCapability::Formatting
            )
        )
    ));
    process.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn process_edit_requests_map_range_formatting_and_organize_imports() {
    let process = LocalLanguageServiceProcess::spawn(config())
        .await
        .expect("fixture starts");
    process
        .did_open(
            "file:///fixture".into(),
            LanguageId::new(Language::Rust),
            "use z::Z;\nuse a::A;\n".into(),
            DocumentVersion::new(1),
            1,
        )
        .await
        .expect("open");
    let snapshot = EditorBuffer::from_text("use z::Z;\nuse a::A;\n").snapshot();
    let range = LspRange {
        start: LspPosition {
            line: 0,
            character: 0,
        },
        end: LspPosition {
            line: 1,
            character: 8,
        },
    };
    let formatted = process
        .request_edit(LocalLanguageServiceEditRequest {
            uri: "file:///fixture".into(),
            snapshot: snapshot.clone(),
            version: DocumentVersion::new(1),
            generation: 1,
            request: LanguageServiceEditRequest::FormatSelection { range },
            cancellation: CancellationToken::default(),
        })
        .await;
    assert!(matches!(
        formatted.response,
        Ok(LanguageServiceEditResponse::Formatting(ref edits))
            if edits.len() == 1 && edits[0].replacement == "range"
    ));

    let organized = process
        .request_edit(LocalLanguageServiceEditRequest {
            uri: "file:///fixture".into(),
            snapshot,
            version: DocumentVersion::new(1),
            generation: 1,
            request: LanguageServiceEditRequest::OrganizeImports,
            cancellation: CancellationToken::default(),
        })
        .await;
    assert!(matches!(
        organized.response,
        Ok(LanguageServiceEditResponse::OrganizeImports(ref edits))
            if edits.len() == 1 && edits[0].replacement == "organized"
    ));
    process.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn process_edit_capability_gating_is_typed() {
    let process = LocalLanguageServiceProcess::spawn(config_with(vec![
        "no-range-formatting".into(),
        "no-organize-imports".into(),
    ]))
    .await
    .expect("fixture starts");
    process
        .did_open(
            "file:///fixture".into(),
            LanguageId::new(Language::Rust),
            "use z::Z;\n".into(),
            DocumentVersion::new(1),
            1,
        )
        .await
        .expect("open");
    let snapshot = EditorBuffer::from_text("use z::Z;\n").snapshot();
    let result = process
        .request_edit(LocalLanguageServiceEditRequest {
            uri: "file:///fixture".into(),
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
                        character: 1,
                    },
                },
            },
            cancellation: CancellationToken::default(),
        })
        .await;
    assert!(matches!(
        result.response,
        Err(
            labonair_editor::LanguageServiceError::UnsupportedEditCapability(
                LanguageServiceEditCapability::FormatSelection
            )
        )
    ));
    let result = process
        .request_edit(LocalLanguageServiceEditRequest {
            uri: "file:///fixture".into(),
            snapshot,
            version: DocumentVersion::new(1),
            generation: 1,
            request: LanguageServiceEditRequest::OrganizeImports,
            cancellation: CancellationToken::default(),
        })
        .await;
    assert!(matches!(
        result.response,
        Err(
            labonair_editor::LanguageServiceError::UnsupportedEditCapability(
                LanguageServiceEditCapability::OrganizeImports
            )
        )
    ));
    process.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn process_edit_requests_preserve_stale_and_cancelled_safety() {
    let process = LocalLanguageServiceProcess::spawn(config())
        .await
        .expect("fixture starts");
    process
        .did_open(
            "file:///fixture".into(),
            LanguageId::new(Language::Rust),
            "fn f() {}".into(),
            DocumentVersion::new(2),
            4,
        )
        .await
        .expect("open");
    let snapshot = EditorBuffer::from_text("fn f() {}").snapshot();
    let stale = process
        .request_edit(LocalLanguageServiceEditRequest {
            uri: "file:///fixture".into(),
            snapshot: snapshot.clone(),
            version: DocumentVersion::new(1),
            generation: 3,
            request: LanguageServiceEditRequest::OrganizeImports,
            cancellation: CancellationToken::default(),
        })
        .await;
    assert!(matches!(
        stale.response,
        Err(labonair_editor::LanguageServiceError::StaleDocument { .. })
    ));

    let cancellation = CancellationToken::default();
    cancellation.cancel();
    let cancelled = process
        .request_edit(LocalLanguageServiceEditRequest {
            uri: "file:///fixture".into(),
            snapshot,
            version: DocumentVersion::new(2),
            generation: 4,
            request: LanguageServiceEditRequest::OrganizeImports,
            cancellation,
        })
        .await;
    assert!(matches!(
        cancelled.response,
        Err(labonair_editor::LanguageServiceError::Cancelled)
    ));
    process.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn cancellation_and_timeout_remove_pending_requests() {
    let process = LocalLanguageServiceProcess::spawn(config_with(vec!["slow".into()]))
        .await
        .expect("fixture starts");
    process
        .did_open(
            "file:///fixture".into(),
            LanguageId::new(Language::Rust),
            "f".into(),
            DocumentVersion::new(1),
            1,
        )
        .await
        .expect("open");
    let token = CancellationToken::default();
    token.cancel();
    let cancelled = process
        .request(LocalLanguageServiceRequest {
            uri: "file:///fixture".into(),
            snapshot: labonair_editor::EditorBuffer::from_text("f").snapshot(),
            version: DocumentVersion::new(1),
            generation: 1,
            request: LanguageServiceRequest::Hover {
                position: labonair_editor::LspPosition {
                    line: 0,
                    character: 0,
                },
            },
            cancellation: token,
        })
        .await;
    assert!(matches!(
        cancelled.response,
        Err(labonair_editor::LanguageServiceError::Cancelled)
    ));

    let mut timeout_config = config_with(vec!["slow".into()]);
    timeout_config.timeouts.request = Duration::from_millis(20);
    let process = LocalLanguageServiceProcess::spawn(timeout_config)
        .await
        .expect("fixture starts");
    process
        .did_open(
            "file:///fixture".into(),
            LanguageId::new(Language::Rust),
            "f".into(),
            DocumentVersion::new(1),
            1,
        )
        .await
        .expect("open");
    let timed_out = process
        .request(LocalLanguageServiceRequest {
            uri: "file:///fixture".into(),
            snapshot: labonair_editor::EditorBuffer::from_text("f").snapshot(),
            version: DocumentVersion::new(1),
            generation: 1,
            request: LanguageServiceRequest::Hover {
                position: labonair_editor::LspPosition {
                    line: 0,
                    character: 0,
                },
            },
            cancellation: CancellationToken::default(),
        })
        .await;
    assert!(matches!(
        timed_out.response,
        Err(labonair_editor::LanguageServiceError::RequestTimeout)
    ));
    process.shutdown().await.expect("shutdown");

    let process = LocalLanguageServiceProcess::spawn(config_with(vec!["slow".into()]))
        .await
        .expect("fixture starts");
    process
        .did_open(
            "file:///fixture".into(),
            LanguageId::new(Language::Rust),
            "f".into(),
            DocumentVersion::new(1),
            1,
        )
        .await
        .expect("open");
    let request_process = process.clone();
    let request = tokio::spawn(async move {
        request_process
            .request(LocalLanguageServiceRequest {
                uri: "file:///fixture".into(),
                snapshot: labonair_editor::EditorBuffer::from_text("f").snapshot(),
                version: DocumentVersion::new(1),
                generation: 1,
                request: LanguageServiceRequest::Hover {
                    position: labonair_editor::LspPosition {
                        line: 0,
                        character: 0,
                    },
                },
                cancellation: CancellationToken::default(),
            })
            .await
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    process
        .did_change(
            "file:///fixture",
            "changed".into(),
            DocumentVersion::new(2),
            2,
        )
        .await
        .expect("change");
    let stale = request.await.expect("request task");
    assert!(matches!(
        stale.response,
        Err(labonair_editor::LanguageServiceError::StaleDocument { .. })
    ));
    process.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn malformed_messages_recover_and_crashes_can_restart() {
    let process = LocalLanguageServiceProcess::spawn(config_with(vec!["malformed".into()]))
        .await
        .expect("fixture starts");
    let mut events = process.subscribe();
    process
        .did_open(
            "file:///fixture".into(),
            LanguageId::new(Language::Rust),
            "f".into(),
            DocumentVersion::new(1),
            1,
        )
        .await
        .expect("open");
    let mut malformed = false;
    let mut diagnostics = false;
    for _ in 0..4 {
        match tokio::time::timeout(Duration::from_secs(1), events.recv())
            .await
            .expect("event")
            .expect("event")
        {
            LanguageServiceEvent::MalformedFrame { .. } => malformed = true,
            LanguageServiceEvent::Diagnostics { .. } => diagnostics = true,
            _ => {}
        }
        if malformed && diagnostics {
            break;
        }
    }
    assert!(malformed && diagnostics);
    process.shutdown().await.expect("shutdown");

    let process = LocalLanguageServiceProcess::spawn(config_with(vec!["crash-after-open".into()]))
        .await
        .expect("fixture starts");
    let mut events = process.subscribe();
    process
        .did_open(
            "file:///fixture".into(),
            LanguageId::new(Language::Rust),
            "f".into(),
            DocumentVersion::new(1),
            1,
        )
        .await
        .expect("open");
    let crashed = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if matches!(
                events.recv().await.expect("event"),
                LanguageServiceEvent::Crashed { .. }
            ) {
                break;
            }
        }
    })
    .await;
    assert!(crashed.is_ok());
    process.restart().await.expect("restart");
    assert_eq!(
        process.lifecycle().await,
        labonair_editor::ProcessLifecycleState::Running
    );
    process.shutdown().await.expect("shutdown");
}
