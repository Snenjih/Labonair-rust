//! Chat sessions, message model and send/stream orchestration (T11-002).
//!
//! Pure-Rust port of the reference app's
//! `reference-src/src/modules/ai/store/chatStore.ts` +
//! `reference-src/src/modules/ai/lib/sessions.ts`. This module is
//! UI-framework-agnostic: it owns the persistent conversation state and the
//! *state transitions* of a streaming response, but does not perform any I/O
//! for the stream itself — the caller (the GPUI `AiChatStore` entity in
//! `labonair-ui`) drives [`SessionStore::begin_send`] → [`AiClient`] →
//! [`SessionStore::apply_event`] and calls `cx.notify()` off [`revision`].
//!
//! [`AiClient`]: crate::client::AiClient
//! [`revision`]: SessionStore::revision

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::message::{ChatMessage, Role, ToolCall, Usage};
use crate::StreamEvent;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
}

// ── Message model ──────────────────────────────────────────────────────────

/// Lifecycle state of a single message, for UI rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    /// The assistant is still producing this message.
    Streaming,
    /// Complete and successful.
    Final,
    /// The response failed; `SessionMessage::error` holds the reason.
    Error,
}

/// State of a tool call the assistant requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolCallStatus {
    /// The JSON arguments are still streaming in.
    Streaming,
    /// Arguments complete; waiting for approval / execution (T11-004).
    AwaitingApproval,
    /// Executed; `result` holds the outcome text.
    Done,
    /// Execution failed; `result` holds the error text.
    Error,
}

/// A tool call attached to an assistant message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionToolCall {
    pub id: String,
    pub name: String,
    /// JSON arguments string (may be partial while `status == Streaming`).
    #[serde(default)]
    pub arguments: String,
    pub status: ToolCallStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

/// One message in a session's history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: String,
    pub role: Role,
    #[serde(default)]
    pub content: String,
    /// Reasoning / "thinking" text, kept separate from `content`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reasoning: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<SessionToolCall>,
    /// Set on `Role::Tool` messages — the call this result answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    pub status: MessageStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: u64,
}

impl SessionMessage {
    fn new(role: Role, content: impl Into<String>, status: MessageStatus) -> Self {
        SessionMessage {
            id: new_id("m"),
            role,
            content: content.into(),
            reasoning: String::new(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            status,
            error: None,
            created_at: now_ms(),
        }
    }

    /// Convert to the provider-agnostic [`ChatMessage`] for history replay.
    fn to_chat_message(&self) -> ChatMessage {
        ChatMessage {
            role: self.role,
            content: self.content.clone(),
            tool_calls: self
                .tool_calls
                .iter()
                .map(|t| ToolCall {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    arguments: t.arguments.clone(),
                })
                .collect(),
            tool_call_id: self.tool_call_id.clone(),
        }
    }
}

// ── Sessions ───────────────────────────────────────────────────────────────

/// Lightweight per-session metadata (the sidebar list rows).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
}

impl SessionMeta {
    fn fresh() -> Self {
        let ts = now_ms();
        SessionMeta {
            id: new_id("s"),
            title: NEW_CHAT_TITLE.to_string(),
            created_at: ts,
            updated_at: ts,
        }
    }
}

const NEW_CHAT_TITLE: &str = "New chat";

/// Coarse run state of the active session, mirrors the reference's
/// `AgentRunStatus`. Reset to `Idle` on a provider/key switch; the sessions
/// themselves are untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunStatus {
    #[default]
    Idle,
    /// Request sent, no tokens yet.
    Thinking,
    /// Tokens streaming in.
    Streaming,
    /// A tool call is complete and waiting for approval / execution.
    AwaitingApproval,
    /// The last run failed.
    Error,
}

// ── Persistence ────────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoreFile {
    #[serde(default)]
    sessions: Vec<SessionMeta>,
    #[serde(default)]
    active_id: Option<String>,
    #[serde(default)]
    messages: HashMap<String, Vec<SessionMessage>>,
}

/// The persistent chat store: sessions, their messages, the active id, and the
/// live [`RunStatus`] of the active session. Backed by a single JSON file
/// (`~/.config/labonair/labonair-sessions.json` by default).
///
/// Change notification is via [`SessionStore::revision`] — a counter bumped on
/// every mutation. Wrap this in a GPUI entity and call `cx.notify()` whenever
/// the revision advances.
#[derive(Debug)]
pub struct SessionStore {
    path: PathBuf,
    file: StoreFile,
    run_status: RunStatus,
    last_usage: Usage,
    revision: u64,
    autosave: bool,
}

impl SessionStore {
    /// Default location: `~/.config/labonair/labonair-sessions.json`.
    pub fn default_path() -> PathBuf {
        let dir = dirs::home_dir()
            .map(|h| h.join(".config").join("labonair"))
            .unwrap_or_else(|| PathBuf::from("."));
        let _ = std::fs::create_dir_all(&dir);
        dir.join("labonair-sessions.json")
    }

    /// Load from `path` (or start empty), then guarantee the invariant that
    /// there is always at least one session and a valid active id. A leading
    /// untitled "New chat" session is reused across restarts rather than
    /// stacking a fresh empty one every launch.
    pub fn load(path: impl Into<PathBuf>) -> SessionStore {
        let path = path.into();
        let file = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<StoreFile>(&s).ok())
            .unwrap_or_default();
        let mut store = SessionStore {
            path,
            file,
            run_status: RunStatus::Idle,
            last_usage: Usage::default(),
            revision: 0,
            autosave: true,
        };
        store.ensure_active();
        store
    }

    /// Open the store at [`SessionStore::default_path`].
    pub fn open_default() -> SessionStore {
        Self::load(Self::default_path())
    }

    /// Disable disk writes (tests). Re-enabling does not force a flush.
    pub fn set_autosave(&mut self, on: bool) {
        self.autosave = on;
    }

    fn ensure_active(&mut self) {
        // Drop orphan message blobs for sessions that no longer exist.
        let ids: Vec<String> = self.file.sessions.iter().map(|s| s.id.clone()).collect();
        self.file.messages.retain(|k, _| ids.contains(k));

        if self.file.sessions.is_empty() {
            let fresh = SessionMeta::fresh();
            self.file.active_id = Some(fresh.id.clone());
            self.file.sessions.push(fresh);
            self.persist_structure();
            return;
        }
        let active_ok = self
            .file
            .active_id
            .as_deref()
            .is_some_and(|id| self.file.sessions.iter().any(|s| s.id == id));
        if !active_ok {
            self.file.active_id = Some(self.file.sessions[0].id.clone());
            self.persist_structure();
        }
    }

    fn persist_structure(&self) {
        if !self.autosave {
            return;
        }
        self.write();
    }

    fn write(&self) {
        let json = match serde_json::to_string_pretty(&self.file) {
            Ok(j) => j,
            Err(err) => {
                tracing::warn!(%err, "failed to serialize chat sessions");
                return;
            }
        };
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = self.path.with_extension("json.tmp");
        if let Err(err) =
            std::fs::write(&tmp, json).and_then(|()| std::fs::rename(&tmp, &self.path))
        {
            tracing::warn!(%err, "failed to write chat sessions");
        }
    }

    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    // ── Read accessors ────────────────────────────────────────────────────

    /// Monotonic change counter — advances on every mutation.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn run_status(&self) -> RunStatus {
        self.run_status
    }

    pub fn last_usage(&self) -> Usage {
        self.last_usage
    }

    pub fn sessions(&self) -> &[SessionMeta] {
        &self.file.sessions
    }

    pub fn active_id(&self) -> Option<&str> {
        self.file.active_id.as_deref()
    }

    /// Messages of `id` (empty slice if the session has none yet).
    pub fn messages(&self, id: &str) -> &[SessionMessage] {
        self.file.messages.get(id).map_or(&[], |v| v.as_slice())
    }

    /// Messages of the active session.
    pub fn active_messages(&self) -> &[SessionMessage] {
        match self.file.active_id.as_deref() {
            Some(id) => self.messages(id),
            None => &[],
        }
    }

    // ── Session CRUD ──────────────────────────────────────────────────────

    /// Create a new empty session and make it active. Returns its id.
    pub fn new_session(&mut self) -> String {
        let meta = SessionMeta::fresh();
        let id = meta.id.clone();
        self.file.sessions.insert(0, meta);
        self.file.active_id = Some(id.clone());
        self.run_status = RunStatus::Idle;
        self.last_usage = Usage::default();
        self.persist_structure();
        self.bump();
        id
    }

    /// Switch the active session. No-op if `id` is unknown or already active.
    pub fn switch_session(&mut self, id: &str) {
        if self.file.active_id.as_deref() == Some(id) {
            return;
        }
        if !self.file.sessions.iter().any(|s| s.id == id) {
            return;
        }
        self.file.active_id = Some(id.to_string());
        // The freshly-focused session may itself hold an unfinished streaming
        // message from a previous run; treat it as settled for status purposes.
        self.run_status = if self.has_awaiting_tool_call(id) {
            RunStatus::AwaitingApproval
        } else {
            RunStatus::Idle
        };
        self.last_usage = Usage::default();
        self.persist_structure();
        self.bump();
    }

    /// Delete a session and its messages. If it was active, switch to the
    /// next remaining session; if none remain, create a fresh one.
    pub fn delete_session(&mut self, id: &str) {
        let existed = self.file.sessions.iter().any(|s| s.id == id);
        if !existed {
            return;
        }
        let was_active = self.file.active_id.as_deref() == Some(id);
        self.file.sessions.retain(|s| s.id != id);
        self.file.messages.remove(id);

        if self.file.sessions.is_empty() {
            let fresh = SessionMeta::fresh();
            self.file.active_id = Some(fresh.id.clone());
            self.file.sessions.push(fresh);
            self.run_status = RunStatus::Idle;
        } else if was_active {
            self.file.active_id = Some(self.file.sessions[0].id.clone());
            self.run_status = RunStatus::Idle;
            self.last_usage = Usage::default();
        }
        self.persist_structure();
        self.bump();
    }

    /// Rename a session (user-set title; auto-titling stops touching it).
    pub fn rename_session(&mut self, id: &str, title: impl Into<String>) {
        if let Some(s) = self.file.sessions.iter_mut().find(|s| s.id == id) {
            s.title = title.into();
            s.updated_at = now_ms();
            self.persist_structure();
            self.bump();
        }
    }

    // ── Provider / key switch ─────────────────────────────────────────────

    /// Reset the *in-memory* run state of the active chat — used when the
    /// active provider/key changes. Sessions and their persisted messages are
    /// deliberately left intact (a core Labonair principle); only the live run
    /// status, token counters and any still-streaming assistant message of the
    /// active session are settled.
    pub fn reset_active_run(&mut self) {
        self.run_status = RunStatus::Idle;
        self.last_usage = Usage::default();
        if let Some(id) = self.file.active_id.clone() {
            if let Some(msgs) = self.file.messages.get_mut(&id) {
                if let Some(last) = msgs.last_mut() {
                    if last.status == MessageStatus::Streaming {
                        last.status = MessageStatus::Final;
                    }
                    for tc in &mut last.tool_calls {
                        if tc.status == ToolCallStatus::Streaming {
                            tc.status = ToolCallStatus::AwaitingApproval;
                        }
                    }
                }
            }
            self.persist_messages(&id);
        }
        self.bump();
    }

    // ── Send / stream orchestration ───────────────────────────────────────

    /// Append a user message + an empty streaming assistant placeholder to the
    /// active session and return the conversation history to hand to
    /// [`AiClient::stream_chat`](crate::client::AiClient::stream_chat) (the
    /// placeholder itself is excluded).
    pub fn begin_send(&mut self, text: impl Into<String>) -> Vec<ChatMessage> {
        let id = match self.file.active_id.clone() {
            Some(id) => id,
            None => self.new_session(),
        };
        let msgs = self.file.messages.entry(id.clone()).or_default();
        msgs.push(SessionMessage::new(Role::User, text, MessageStatus::Final));
        let history: Vec<ChatMessage> = msgs.iter().map(SessionMessage::to_chat_message).collect();
        msgs.push(SessionMessage::new(
            Role::Assistant,
            "",
            MessageStatus::Streaming,
        ));

        self.run_status = RunStatus::Thinking;
        self.last_usage = Usage::default();
        self.retitle(&id);
        self.touch(&id);
        self.persist_messages(&id);
        self.persist_structure();
        self.bump();
        history
    }

    /// Fold one streaming event into the active session's trailing assistant
    /// message. Does **not** write to disk (called per token) — call
    /// [`SessionStore::finish_run`] once the stream ends.
    pub fn apply_event(&mut self, event: StreamEvent) {
        let Some(id) = self.file.active_id.clone() else {
            return;
        };
        let Some(msg) = self.file.messages.get_mut(&id).and_then(|m| m.last_mut()) else {
            return;
        };
        if msg.role != Role::Assistant {
            return;
        }
        match event {
            StreamEvent::TextDelta(t) => {
                msg.content.push_str(&t);
                self.run_status = RunStatus::Streaming;
            }
            StreamEvent::ReasoningDelta(t) => {
                msg.reasoning.push_str(&t);
                if self.run_status == RunStatus::Thinking {
                    self.run_status = RunStatus::Streaming;
                }
            }
            StreamEvent::ToolCallStart { id: tc_id, name } => {
                msg.tool_calls.push(SessionToolCall {
                    id: tc_id,
                    name,
                    arguments: String::new(),
                    status: ToolCallStatus::Streaming,
                    result: None,
                });
                self.run_status = RunStatus::Streaming;
            }
            StreamEvent::ToolCallDelta {
                id: tc_id,
                arguments_delta,
            } => {
                if let Some(tc) = msg.tool_calls.iter_mut().find(|t| t.id == tc_id) {
                    tc.arguments.push_str(&arguments_delta);
                }
            }
            StreamEvent::ToolCallEnd { id: tc_id } => {
                if let Some(tc) = msg.tool_calls.iter_mut().find(|t| t.id == tc_id) {
                    tc.status = ToolCallStatus::AwaitingApproval;
                }
                self.run_status = RunStatus::AwaitingApproval;
            }
            StreamEvent::Usage(u) => {
                self.last_usage = u;
            }
            StreamEvent::Done { .. } => {
                msg.status = MessageStatus::Final;
                self.settle_run_status(&id);
            }
            StreamEvent::Error(e) => {
                msg.status = MessageStatus::Error;
                msg.error = Some(e.to_string());
                self.run_status = RunStatus::Error;
            }
        }
        self.bump();
    }

    /// Finalize after the stream has ended (normally or via drop/abort): mark a
    /// still-`Streaming` trailing message as `Final` and persist the session.
    pub fn finish_run(&mut self) {
        let Some(id) = self.file.active_id.clone() else {
            return;
        };
        if let Some(msg) = self.file.messages.get_mut(&id).and_then(|m| m.last_mut()) {
            if msg.status == MessageStatus::Streaming {
                msg.status = MessageStatus::Final;
            }
        }
        if self.run_status != RunStatus::Error {
            self.settle_run_status(&id);
        }
        self.touch(&id);
        self.persist_messages(&id);
        self.persist_structure();
        self.bump();
    }

    /// User pressed "stop": settle the trailing assistant message where it is
    /// and return to idle. (The caller also cancels the [`ChatStream`].)
    ///
    /// [`ChatStream`]: crate::client::ChatStream
    pub fn stop(&mut self) {
        let Some(id) = self.file.active_id.clone() else {
            return;
        };
        if let Some(msg) = self.file.messages.get_mut(&id).and_then(|m| m.last_mut()) {
            if msg.status == MessageStatus::Streaming {
                msg.status = MessageStatus::Final;
            }
        }
        self.settle_run_status(&id);
        self.touch(&id);
        self.persist_messages(&id);
        self.bump();
    }

    /// Record a failure to start the run (e.g. no API key) on the trailing
    /// assistant placeholder.
    pub fn fail_run(&mut self, reason: impl Into<String>) {
        let Some(id) = self.file.active_id.clone() else {
            return;
        };
        if let Some(msg) = self.file.messages.get_mut(&id).and_then(|m| m.last_mut()) {
            if msg.role == Role::Assistant && msg.status == MessageStatus::Streaming {
                msg.status = MessageStatus::Error;
                msg.error = Some(reason.into());
            }
        }
        self.run_status = RunStatus::Error;
        self.persist_messages(&id);
        self.bump();
    }

    // ── internals ─────────────────────────────────────────────────────────

    fn settle_run_status(&mut self, session_id: &str) {
        self.run_status = if self.has_awaiting_tool_call(session_id) {
            RunStatus::AwaitingApproval
        } else {
            RunStatus::Idle
        };
    }

    fn has_awaiting_tool_call(&self, session_id: &str) -> bool {
        self.file.messages.get(session_id).is_some_and(|msgs| {
            msgs.iter().flat_map(|m| &m.tool_calls).any(|t| {
                matches!(
                    t.status,
                    ToolCallStatus::AwaitingApproval | ToolCallStatus::Streaming
                )
            })
        })
    }

    fn touch(&mut self, session_id: &str) {
        if let Some(s) = self.file.sessions.iter_mut().find(|s| s.id == session_id) {
            s.updated_at = now_ms();
        }
    }

    /// Auto-derive the title from the first user message while it is untitled.
    fn retitle(&mut self, session_id: &str) {
        let Some(meta) = self.file.sessions.iter().find(|s| s.id == session_id) else {
            return;
        };
        if !meta.title.is_empty() && meta.title != NEW_CHAT_TITLE {
            return;
        }
        let msgs = match self.file.messages.get(session_id) {
            Some(m) => m,
            None => return,
        };
        let next = derive_title(msgs);
        if let Some(meta) = self.file.sessions.iter_mut().find(|s| s.id == session_id) {
            if meta.title != next {
                meta.title = next;
                meta.updated_at = now_ms();
            }
        }
    }

    /// Persist a single session's message list. Structural metadata is written
    /// separately by [`SessionStore::persist_structure`]; both go to the same
    /// file, so in practice this writes the whole blob — callers must only
    /// invoke it at sensible points (send, finish, stop), never per token.
    fn persist_messages(&self, _session_id: &str) {
        if !self.autosave {
            return;
        }
        self.write();
    }
}

/// Derive a session title from its first non-empty user message: strip the
/// injected `<terminal-context>` / `<selection>` / `<file>` blocks, take the
/// first line, truncate to 40 chars. Mirrors `sessions.ts::deriveTitle`.
pub fn derive_title(messages: &[SessionMessage]) -> String {
    for m in messages {
        if m.role != Role::User {
            continue;
        }
        let text = strip_context_blocks(&m.content);
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        let first = text.lines().next().unwrap_or("").trim();
        return if first.chars().count() > 40 {
            let truncated: String = first.chars().take(40).collect();
            format!("{truncated}…")
        } else {
            first.to_string()
        };
    }
    NEW_CHAT_TITLE.to_string()
}

fn strip_context_blocks(text: &str) -> String {
    let mut out = text.to_string();
    for tag in ["terminal-context", "selection", "file"] {
        let open = format!("<{tag}");
        let close = format!("</{tag}>");
        while let Some(start) = out.find(&open) {
            let Some(rel_end) = out[start..].find(&close) else {
                break;
            };
            let end = start + rel_end + close.len();
            out.replace_range(start..end, "");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AiError;

    fn tmp() -> PathBuf {
        std::env::temp_dir().join(format!("labonair-sessions-{}.json", uuid::Uuid::new_v4()))
    }

    fn store() -> SessionStore {
        let mut s = SessionStore::load(tmp());
        s.set_autosave(false);
        s
    }

    #[test]
    fn starts_with_one_active_session() {
        let s = store();
        assert_eq!(s.sessions().len(), 1);
        assert_eq!(s.sessions()[0].title, "New chat");
        assert_eq!(s.active_id(), Some(s.sessions()[0].id.as_str()));
    }

    #[test]
    fn create_switch_delete_rename() {
        let mut s = store();
        let first = s.active_id().unwrap().to_string();

        let second = s.new_session();
        assert_eq!(s.active_id(), Some(second.as_str()));
        assert_eq!(s.sessions().len(), 2);

        s.switch_session(&first);
        assert_eq!(s.active_id(), Some(first.as_str()));
        s.switch_session("bogus");
        assert_eq!(s.active_id(), Some(first.as_str()));

        s.rename_session(&first, "Renamed");
        assert_eq!(
            s.sessions().iter().find(|x| x.id == first).unwrap().title,
            "Renamed"
        );

        // Deleting the active one falls back to a remaining session.
        s.delete_session(&first);
        assert_eq!(s.sessions().len(), 1);
        assert_eq!(s.active_id(), Some(second.as_str()));

        // Deleting the last one spawns a fresh session.
        s.delete_session(&second);
        assert_eq!(s.sessions().len(), 1);
        assert_ne!(s.active_id(), Some(second.as_str()));
    }

    #[test]
    fn send_produces_user_then_assistant_and_finalizes() {
        let mut s = store();
        let history = s.begin_send("hello world");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].role, Role::User);

        let msgs = s.active_messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, Role::User);
        assert_eq!(msgs[1].role, Role::Assistant);
        assert_eq!(msgs[1].status, MessageStatus::Streaming);
        assert_eq!(s.run_status(), RunStatus::Thinking);

        s.apply_event(StreamEvent::TextDelta("Hi".into()));
        s.apply_event(StreamEvent::TextDelta(" there".into()));
        assert_eq!(s.run_status(), RunStatus::Streaming);
        s.apply_event(StreamEvent::Usage(Usage {
            input_tokens: 3,
            output_tokens: 2,
            ..Usage::default()
        }));
        s.apply_event(StreamEvent::Done {
            finish_reason: "stop".into(),
        });
        s.finish_run();

        let msgs = s.active_messages();
        assert_eq!(msgs[1].content, "Hi there");
        assert_eq!(msgs[1].status, MessageStatus::Final);
        assert_eq!(s.run_status(), RunStatus::Idle);
        assert_eq!(s.last_usage().input_tokens, 3);
        // Title auto-derived from the first user message.
        assert_eq!(s.active_messages().len(), 2);
        assert_eq!(s.sessions()[0].title, "hello world");
    }

    #[test]
    fn title_strips_context_and_truncates() {
        let msgs = vec![SessionMessage::new(
            Role::User,
            "<terminal-context>noise</terminal-context>\nActual question here",
            MessageStatus::Final,
        )];
        assert_eq!(derive_title(&msgs), "Actual question here");

        let long = "x".repeat(60);
        let msgs = vec![SessionMessage::new(Role::User, long, MessageStatus::Final)];
        let t = derive_title(&msgs);
        assert_eq!(t.chars().count(), 41); // 40 + ellipsis
        assert!(t.ends_with('…'));
    }

    #[test]
    fn stop_settles_streaming_message() {
        let mut s = store();
        s.begin_send("q");
        s.apply_event(StreamEvent::TextDelta("partial".into()));
        s.stop();
        let msgs = s.active_messages();
        assert_eq!(msgs[1].content, "partial");
        assert_eq!(msgs[1].status, MessageStatus::Final);
        assert_eq!(s.run_status(), RunStatus::Idle);
    }

    #[test]
    fn tool_call_is_held_awaiting_approval() {
        let mut s = store();
        s.begin_send("run something");
        s.apply_event(StreamEvent::ToolCallStart {
            id: "t1".into(),
            name: "bash_run".into(),
        });
        s.apply_event(StreamEvent::ToolCallDelta {
            id: "t1".into(),
            arguments_delta: "{\"cmd\":\"ls\"}".into(),
        });
        s.apply_event(StreamEvent::ToolCallEnd { id: "t1".into() });
        s.apply_event(StreamEvent::Done {
            finish_reason: "tool_calls".into(),
        });
        s.finish_run();

        let tc = &s.active_messages()[1].tool_calls[0];
        assert_eq!(tc.name, "bash_run");
        assert_eq!(tc.arguments, "{\"cmd\":\"ls\"}");
        assert_eq!(tc.status, ToolCallStatus::AwaitingApproval);
        assert_eq!(s.run_status(), RunStatus::AwaitingApproval);
    }

    #[test]
    fn error_event_marks_message_error() {
        let mut s = store();
        s.begin_send("q");
        s.apply_event(StreamEvent::Error(AiError::Cancelled));
        assert_eq!(s.active_messages()[1].status, MessageStatus::Error);
        assert!(s.active_messages()[1].error.is_some());
        assert_eq!(s.run_status(), RunStatus::Error);
    }

    #[test]
    fn provider_switch_resets_run_but_keeps_sessions() {
        let mut s = store();
        let sid = s.active_id().unwrap().to_string();
        s.begin_send("keep me");
        s.apply_event(StreamEvent::TextDelta("half".into()));
        assert_eq!(s.run_status(), RunStatus::Streaming);

        s.reset_active_run();

        assert_eq!(s.run_status(), RunStatus::Idle);
        assert_eq!(s.active_id(), Some(sid.as_str()));
        assert_eq!(s.sessions().len(), 1);
        let msgs = s.active_messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "keep me");
        assert_eq!(msgs[1].content, "half");
        assert_eq!(msgs[1].status, MessageStatus::Final);
    }

    #[test]
    fn sessions_and_messages_survive_restart() {
        let path = tmp();
        let sid;
        {
            let mut s = SessionStore::load(&path);
            sid = s.active_id().unwrap().to_string();
            s.rename_session(&sid, "Persisted");
            let extra = s.new_session();
            s.switch_session(&sid);
            s.begin_send("remember this");
            s.apply_event(StreamEvent::TextDelta("answer".into()));
            s.apply_event(StreamEvent::Done {
                finish_reason: "stop".into(),
            });
            s.finish_run();
            let _ = extra;
        }
        {
            let s = SessionStore::load(&path);
            assert_eq!(s.sessions().len(), 2);
            let meta = s.sessions().iter().find(|x| x.id == sid).unwrap();
            assert_eq!(meta.title, "Persisted");
            let msgs = s.messages(&sid);
            assert_eq!(msgs.len(), 2);
            assert_eq!(msgs[0].content, "remember this");
            assert_eq!(msgs[1].content, "answer");
            assert_eq!(msgs[1].status, MessageStatus::Final);
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn revision_advances_on_mutation() {
        let mut s = store();
        let r0 = s.revision();
        s.new_session();
        let r1 = s.revision();
        assert!(r1 > r0);
        s.begin_send("x");
        assert!(s.revision() > r1);
    }
}
