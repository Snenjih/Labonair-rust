//! `AiChatStore` — the GPUI entity wrapper around [`labonair_ai::SessionStore`]
//! (T11-002).
//!
//! Owns the persistent chat sessions and drives the send → stream → apply loop:
//! [`SessionStore::begin_send`] produces the history, [`AiClient::stream_chat`]
//! streams the response on a Tokio task, and each [`StreamEvent`] is folded back
//! into the store via [`SessionStore::apply_event`] followed by `cx.notify()`.
//! The chat UI itself lands in T11-003 and renders off this entity.
//!
//! Crate root (T16-008): this file is the `labonair-panel-ai` lib root. The
//! `theme` / `markdown` / `syntax_theme` shims keep the pre-split `crate::…`
//! paths resolving against their new home crates; `AgentAccessStore` stays in
//! `labonair-workspace` (shared with `Workspace`, T11-006) and is re-exported
//! here for the shell.

pub mod ai_composer;

pub use labonair_workspace::agent_access::{AgentAccessEntry, AgentAccessStore};

pub(crate) mod theme {
    pub use labonair_theme::store::*;
}

pub(crate) mod markdown {
    pub use labonair_workspace::markdown::*;
}

pub(crate) mod syntax_theme {
    pub use labonair_workspace::syntax_theme::*;
}

use std::sync::{Arc, Mutex};

use gpui::{AppContext, Context, Entity};
use labonair_ai::tools::{NativeHost, NoopSubagentRunner, SubagentRunner, Todo, TodoStatus};
use labonair_ai::{
    resolve_target, AiClient, ChatConfig, ChatMessage, InstanceStore, KeyringSecretStore,
    LiveBridge, NoLiveBridge, RunStatus, SecretStore, SessionMessage, SessionMeta, SessionStore,
    StreamEvent, TodoStore, ToolCall, ToolContext, ToolHost, ToolRegistry, Usage,
};
use tokio::runtime::Handle as TokioHandle;
use tokio::sync::mpsc;

/// GPUI entity holding the chat sessions + the active streaming run.
pub struct AiChatStore {
    store: SessionStore,
    instances: InstanceStore,
    secrets: Arc<dyn SecretStore>,
    client: AiClient,
    tokio: TokioHandle,
    /// The active model reference (`"<model>"` or `"<model>@<instanceId>"`).
    model_ref: String,
    /// Handle to the in-flight streaming task, if any.
    run: Option<tokio::task::JoinHandle<()>>,
    /// Bumped on every send / stop / provider switch; stale tasks whose
    /// generation no longer matches drop their events.
    generation: u64,
    // ── T11-004 agent / tool system ──────────────────────────────────────
    registry: Arc<ToolRegistry>,
    live: Arc<dyn LiveBridge>,
    host: Arc<dyn ToolHost>,
    todos: Arc<Mutex<TodoStore>>,
    subagents: Arc<dyn SubagentRunner>,
    /// Absolute paths read this session — the read-before-edit invariant.
    read_cache: Arc<Mutex<HashSet<String>>>,
    /// Approval-gated tool calls from the last model turn, awaiting the user.
    pending_calls: Vec<ToolCall>,
    /// Prompts queued with ⌘↵ — sent one at a time as each run completes.
    prompt_queue: Vec<String>,
    /// The active agent's instructions, prepended as a system message on every
    /// turn (AgentSwitcher — T16-019). Empty = no agent system prompt.
    agent_instructions: String,
    /// Plan mode — the agent proposes edits for review instead of applying them
    /// directly (reference `usePlanStore`). Toggled via `/plan` or the strip.
    plan_mode: bool,
    /// Queued file edits awaiting review while plan mode is active.
    plan_queue: Vec<PlanEdit>,
}

/// The mutating tools that plan mode diverts into the review queue.
pub const PLAN_MUTATING_TOOLS: &[&str] = &["write_file", "edit", "multi_edit", "create_directory"];

/// System-prompt block appended while plan mode is active (verbatim port of
/// `reference-src/src/modules/ai/lib/agent.ts` `planBlock`).
pub const PLAN_MODE_PROMPT: &str = "## PLAN MODE — ACTIVE\n\
Mutating tools (write_file, edit, multi_edit, create_directory) will queue their changes for the user to review as a single diff. \
Do NOT execute bash_run or bash_background while plan mode is active — restrict yourself to reads (read_file, grep, glob, list_directory) and the queued mutations. \
After queueing the full set of edits, stop and return a brief summary; do not continue acting until the user has accepted/rejected.";

/// A queued file mutation for plan review (`QueuedEdit` in the reference).
#[derive(Debug, Clone)]
pub struct PlanEdit {
    pub id: String,
    /// `write_file` | `edit` | `multi_edit` | `create_directory`.
    pub kind: String,
    pub path: String,
    /// File content before the edit (empty for new files / directories).
    pub original: String,
    /// Full file content after the edit (empty for `create_directory`).
    pub proposed: String,
    pub is_new: bool,
}

/// Build a [`PlanEdit`] by resolving a mutating tool call against the current
/// on-disk state (mirrors each tool's own replacement logic).
fn plan_edit_from_call(id: &str, name: &str, args: &serde_json::Value) -> Option<PlanEdit> {
    let path = args.get("path").and_then(|v| v.as_str())?.to_string();
    let exists = std::path::Path::new(&path).exists();
    let original = std::fs::read_to_string(&path).unwrap_or_default();
    let apply = |src: &str, old: &str, new: &str, all: bool| -> String {
        if all {
            src.replace(old, new)
        } else {
            src.replacen(old, new, 1)
        }
    };
    let (kind, proposed) = match name {
        "create_directory" => ("create_directory", String::new()),
        "write_file" => (
            "write_file",
            args.get("content")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        ),
        "edit" => {
            let old = args
                .get("old_string")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let new = args
                .get("new_string")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let all = args
                .get("replace_all")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            ("edit", apply(&original, old, new, all))
        }
        "multi_edit" => {
            let mut proposed = original.clone();
            if let Some(edits) = args.get("edits").and_then(|v| v.as_array()) {
                for e in edits {
                    let old = e
                        .get("old_string")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    let new = e
                        .get("new_string")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    let all = e
                        .get("replace_all")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    proposed = apply(&proposed, old, new, all);
                }
            }
            ("multi_edit", proposed)
        }
        _ => return None,
    };
    Some(PlanEdit {
        id: id.to_string(),
        kind: kind.to_string(),
        path,
        original,
        proposed,
        is_new: !exists,
    })
}

impl AiChatStore {
    /// Construct with the default on-disk stores and the OS keyring.
    pub fn new(tokio: TokioHandle) -> Self {
        Self::from_parts(
            SessionStore::open_default(),
            InstanceStore::open_default(),
            Arc::new(KeyringSecretStore),
            tokio,
        )
    }

    /// Construct from explicit parts (used by tests to inject temp stores and
    /// an in-memory secret store).
    pub fn from_parts(
        store: SessionStore,
        instances: InstanceStore,
        secrets: Arc<dyn SecretStore>,
        tokio: TokioHandle,
    ) -> Self {
        let model_ref = instances.active_model_ref();
        AiChatStore {
            store,
            instances,
            secrets,
            client: AiClient::new(),
            tokio,
            model_ref,
            run: None,
            generation: 0,
            registry: Arc::new(ToolRegistry::builtin()),
            live: Arc::new(NoLiveBridge),
            host: Arc::new(NativeHost),
            todos: Arc::new(Mutex::new(TodoStore::open_default())),
            subagents: Arc::new(NoopSubagentRunner),
            read_cache: Arc::new(Mutex::new(HashSet::new())),
            pending_calls: Vec::new(),
            prompt_queue: Vec::new(),
            agent_instructions: String::new(),
            plan_mode: false,
            plan_queue: Vec::new(),
        }
    }

    /// Queued plan-mode edits awaiting review.
    pub fn plan_queue(&self) -> &[PlanEdit] {
        &self.plan_queue
    }

    /// Reject a single queued edit.
    pub fn plan_reject(&mut self, id: &str, cx: &mut Context<Self>) {
        self.plan_queue.retain(|e| e.id != id);
        cx.notify();
    }

    /// Discard every queued edit.
    pub fn plan_discard_all(&mut self, cx: &mut Context<Self>) {
        self.plan_queue.clear();
        cx.notify();
    }

    /// Write every queued edit to disk (off-thread), then clear the queue.
    pub fn plan_apply_all(&mut self, cx: &mut Context<Self>) {
        let items = std::mem::take(&mut self.plan_queue);
        if items.is_empty() {
            return;
        }
        let (tx, rx) = tokio::sync::oneshot::channel::<Vec<String>>();
        self.tokio.spawn_blocking(move || {
            let mut errs = Vec::new();
            for it in &items {
                let res = if it.kind == "create_directory" {
                    std::fs::create_dir_all(&it.path)
                } else {
                    if let Some(parent) = std::path::Path::new(&it.path).parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    std::fs::write(&it.path, &it.proposed)
                };
                if let Err(e) = res {
                    errs.push(format!("{}: {e}", it.path));
                }
            }
            let _ = tx.send(errs);
        });
        cx.spawn(async move |this, cx| {
            let _ = rx.await;
            let _ = this.update(cx, |_this, cx| cx.notify());
        })
        .detach();
        cx.notify();
    }

    /// Plan-mode flag (reference `usePlanStore().active`).
    pub fn plan_mode(&self) -> bool {
        self.plan_mode
    }

    /// Toggle / set plan mode.
    pub fn set_plan_mode(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.plan_mode != on {
            self.plan_mode = on;
            cx.notify();
        }
    }

    /// Best-effort workspace directory for the `@`-file picker: the live
    /// bridge's root/cwd, else the process cwd.
    pub fn workspace_cwd(&self) -> Option<String> {
        self.live
            .workspace_root()
            .or_else(|| self.live.cwd())
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.display().to_string())
            })
    }

    /// Set the active agent's system instructions (used on the next turn).
    pub fn set_agent_instructions(&mut self, instructions: impl Into<String>) {
        self.agent_instructions = instructions.into();
    }

    /// Swap in a real live-bridge (active-terminal cwd + buffer). The default is
    /// a no-op bridge; the app-shell wires the workspace-backed one.
    pub fn set_live_bridge(&mut self, live: Arc<dyn LiveBridge>) {
        self.live = live;
    }

    fn tool_context(&self) -> ToolContext {
        let sid = self.store.active_id().unwrap_or_default().to_string();
        ToolContext::new(
            sid,
            self.live.clone(),
            self.host.clone(),
            self.todos.clone(),
            self.subagents.clone(),
        )
        .with_read_cache(self.read_cache.clone())
    }

    // ── Read accessors ───────────────────────────────────────────────────

    pub fn sessions(&self) -> &[SessionMeta] {
        self.store.sessions()
    }

    pub fn active_id(&self) -> Option<&str> {
        self.store.active_id()
    }

    pub fn active_messages(&self) -> &[SessionMessage] {
        self.store.active_messages()
    }

    pub fn run_status(&self) -> RunStatus {
        self.store.run_status()
    }

    pub fn model_ref(&self) -> &str {
        &self.model_ref
    }

    pub fn is_streaming(&self) -> bool {
        self.run.is_some()
    }

    /// Prompts waiting to be sent after the current run (⌘↵ enqueue).
    pub fn queued_prompts(&self) -> &[String] {
        &self.prompt_queue
    }

    /// The active session's agent todo list (TodoStrip).
    pub fn active_todos(&self) -> Vec<Todo> {
        let Some(id) = self.store.active_id() else {
            return Vec::new();
        };
        self.todos
            .lock()
            .map(|t| t.get(id).to_vec())
            .unwrap_or_default()
    }

    /// `true` when the active model has no usable credential — drives the
    /// composer "connect" banner.
    pub fn needs_connection(&self) -> bool {
        resolve_target(&self.model_ref, &self.instances, self.secrets.as_ref()).is_err()
    }

    /// Queue a prompt for after the current turn. If nothing is running, it is
    /// sent immediately.
    pub fn enqueue_prompt(&mut self, text: String, cx: &mut Context<Self>) {
        if self.is_streaming() {
            self.prompt_queue.push(text);
            cx.notify();
        } else {
            self.send(text, cx);
        }
    }

    /// Drop a queued prompt by index (QueueStrip "x").
    pub fn dequeue_prompt(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.prompt_queue.len() {
            self.prompt_queue.remove(idx);
            cx.notify();
        }
    }

    // ── Session management (each notifies) ───────────────────────────────

    pub fn new_session(&mut self, cx: &mut Context<Self>) -> String {
        self.cancel_run();
        let id = self.store.new_session();
        cx.notify();
        id
    }

    pub fn switch_session(&mut self, id: &str, cx: &mut Context<Self>) {
        self.cancel_run();
        self.store.switch_session(id);
        cx.notify();
    }

    pub fn delete_session(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.store.active_id() == Some(id) {
            self.cancel_run();
        }
        self.store.delete_session(id);
        cx.notify();
    }

    pub fn rename_session(&mut self, id: &str, title: impl Into<String>, cx: &mut Context<Self>) {
        self.store.rename_session(id, title);
        cx.notify();
    }

    /// Change the active provider/model. Cancels any in-flight run and resets
    /// the active session's live state (per Labonair's "provider switch resets
    /// the chat, sessions stay" rule) without deleting any session data.
    pub fn set_model_ref(&mut self, model_ref: impl Into<String>, cx: &mut Context<Self>) {
        let next = model_ref.into();
        if next == self.model_ref {
            return;
        }
        self.model_ref = next;
        self.cancel_run();
        let _ = self.instances.set_active_model_ref(&self.model_ref);
        self.store.reset_active_run();
        cx.notify();
    }

    // ── Send / stop ─────────────────────────────────────────────────────

    /// Append the user message and start streaming the assistant response.
    pub fn send(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        let text = text.into();
        if text.trim().is_empty() {
            return;
        }
        self.cancel_run();
        self.pending_calls.clear();
        self.read_cache.lock().unwrap().clear();
        let history = self.store.begin_send(text);
        cx.notify();
        self.spawn_stream(history, cx);
    }

    /// Start (or continue) a streaming model turn over `history`. Shared by
    /// [`AiChatStore::send`] and the post-approval agent continuation.
    fn spawn_stream(&mut self, history: Vec<ChatMessage>, cx: &mut Context<Self>) {
        let mut sys = self.agent_instructions.clone();
        if self.plan_mode {
            if !sys.is_empty() {
                sys.push_str("\n\n");
            }
            sys.push_str(PLAN_MODE_PROMPT);
        }
        let history =
            if sys.trim().is_empty() || history.iter().any(|m| matches!(m.role, Role::System)) {
                history
            } else {
                let mut with_sys = Vec::with_capacity(history.len() + 1);
                with_sys.push(ChatMessage::system(sys));
                with_sys.extend(history);
                with_sys
            };
        let target = match resolve_target(&self.model_ref, &self.instances, self.secrets.as_ref()) {
            Ok(t) => t,
            Err(err) => {
                self.store.fail_run(err.to_string());
                cx.notify();
                return;
            }
        };

        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        let (tx, mut rx) = mpsc::channel::<StreamEvent>(64);
        let client = self.client.clone();
        let tools = self.registry.tool_defs();

        self.run = Some(self.tokio.spawn(async move {
            let mut stream = client.stream_chat(target, ChatConfig::default(), history, tools);
            while let Some(ev) = stream.next().await {
                if tx.send(ev).await.is_err() {
                    break;
                }
            }
        }));

        cx.spawn(async move |this, cx| {
            while let Some(ev) = rx.recv().await {
                let terminal = matches!(ev, StreamEvent::Done { .. } | StreamEvent::Error(_));
                let ok = this
                    .update(cx, |this, cx| {
                        if this.generation != generation {
                            return;
                        }
                        this.store.apply_event(ev);
                        cx.notify();
                    })
                    .is_ok();
                if !ok || terminal {
                    break;
                }
            }
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                this.store.finish_run();
                this.run = None;
                this.dispatch_tool_calls(cx);
                // Drain one queued (⌘↵) prompt now the run is done and no tool
                // approvals are pending.
                if this.run.is_none()
                    && this.pending_calls.is_empty()
                    && !this.prompt_queue.is_empty()
                {
                    let next = this.prompt_queue.remove(0);
                    this.send(next, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// After a model turn ends: auto-run read-only tool calls immediately and
    /// hold approval-gated ones for the user. Once every call in the step is
    /// resolved, the run continues automatically.
    fn dispatch_tool_calls(&mut self, cx: &mut Context<Self>) {
        let pending = self.store.active_pending_tool_calls();
        if pending.is_empty() {
            return;
        }
        let mut auto = Vec::new();
        let mut queued_any = false;
        for (id, name, arguments) in pending {
            // Plan mode — mutating tools queue their change for review instead
            // of executing (reference `usePlanStore.enqueue`).
            if self.plan_mode && PLAN_MUTATING_TOOLS.contains(&name.as_str()) {
                let args: serde_json::Value =
                    serde_json::from_str(&arguments).unwrap_or_else(|_| serde_json::json!({}));
                if let Some(edit) = plan_edit_from_call(&id, &name, &args) {
                    let path = edit.path.clone();
                    self.plan_queue.push(edit);
                    self.store.record_tool_result(
                        &id,
                        &name,
                        format!("{{\"queued\":true,\"path\":{path:?}}}"),
                        false,
                    );
                    queued_any = true;
                    continue;
                }
            }
            let needs_approval = self
                .registry
                .get(&name)
                .map(|t| t.needs_approval())
                .unwrap_or(false);
            let call = ToolCall {
                id,
                name,
                arguments,
            };
            if needs_approval {
                self.pending_calls.push(call);
            } else {
                auto.push(call);
            }
        }
        if !auto.is_empty() {
            self.execute_calls(auto, cx);
        } else if queued_any && self.pending_calls.is_empty() {
            let history = self.store.begin_continue();
            self.spawn_stream(history, cx);
        }
        if queued_any {
            cx.notify();
        }
    }

    /// Execute `calls` off the UI thread, record each result into the store,
    /// and — when no approval-gated calls remain — continue the run.
    fn execute_calls(&mut self, calls: Vec<ToolCall>, cx: &mut Context<Self>) {
        let registry = self.registry.clone();
        let mut ctx = self.tool_context();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tokio.spawn_blocking(move || {
            let mut out = Vec::new();
            for c in calls {
                let args: serde_json::Value =
                    serde_json::from_str(&c.arguments).unwrap_or_else(|_| serde_json::json!({}));
                let result = match registry.get(&c.name) {
                    Some(tool) => tool.run(args, &mut ctx),
                    None => serde_json::json!({ "error": format!("unknown tool: {}", c.name) }),
                };
                out.push((c.id, c.name, result));
            }
            let _ = tx.send(out);
        });

        cx.spawn(async move |this, cx| {
            let Ok(results) = rx.await else { return };
            let _ = this.update(cx, |this, cx| {
                for (id, name, result) in results {
                    let is_error = result.get("error").is_some();
                    let text = serde_json::to_string(&result).unwrap_or_else(|_| "{}".into());
                    this.store.record_tool_result(&id, &name, text, is_error);
                }
                if this.pending_calls.is_empty() {
                    let history = this.store.begin_continue();
                    this.spawn_stream(history, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Stop the in-flight response and settle the partial message.
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        self.cancel_run();
        self.store.stop();
        cx.notify();
    }

    /// Token usage of the last run (0 until the first `Usage` event).
    pub fn last_usage(&self) -> Usage {
        self.store.last_usage()
    }

    /// Approve / reject a pending tool-call approval card. On approval the tool
    /// runs (off-thread); once every gated call is resolved the agent run
    /// continues automatically.
    pub fn resolve_tool_call(&mut self, tool_id: &str, approved: bool, cx: &mut Context<Self>) {
        if let Some(pos) = self.pending_calls.iter().position(|c| c.id == tool_id) {
            let call = self.pending_calls.remove(pos);
            if approved {
                self.execute_calls(vec![call], cx);
            } else {
                self.store.record_tool_result(
                    tool_id,
                    &call.name,
                    "{\"error\":\"Rejected by user.\",\"rejected\":true}",
                    true,
                );
                if self.pending_calls.is_empty() {
                    let history = self.store.begin_continue();
                    self.spawn_stream(history, cx);
                }
                cx.notify();
            }
        } else {
            // No tracked pending call (e.g. a test drove the store directly) —
            // fall back to the plain state transition.
            self.store.resolve_tool_call(tool_id, approved);
            cx.notify();
        }
    }

    #[cfg(test)]
    pub(crate) fn test_dispatch_tool_calls(&mut self, cx: &mut Context<Self>) {
        self.dispatch_tool_calls(cx);
    }

    fn cancel_run(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pending_calls.clear();
        if let Some(handle) = self.run.take() {
            handle.abort();
        }
    }
}

/// Convenience: create the entity.
pub fn init(tokio: TokioHandle, cx: &mut gpui::App) -> Entity<AiChatStore> {
    cx.new(|_| AiChatStore::new(tokio))
}

// ═══════════════════════════════════════════════════════════════════════════
// T11-003 — Chat UI & streaming markdown
// ═══════════════════════════════════════════════════════════════════════════

use std::collections::{HashMap, HashSet};

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, ClipboardItem, FocusHandle, Focusable, FontStyle, FontWeight,
    HighlightStyle, InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle,
    ScrollWheelEvent, SharedString, StatefulInteractiveElement, Styled, StyledText, Window,
};
use labonair_ai::{
    find_model, MessageStatus, ProviderId, Role, SessionToolCall, ToolCallStatus, MODELS,
};
use labonair_backend::modules::model_prefs::{self, ModelPrefs};
use labonair_editor::diff::{ChangeTag, Diff};
use labonair_editor::{Language, SyntaxHighlighter};

use crate::ai_composer::{
    apply_file_mention, detect_popup, filter_files, filter_slash, parse_slash,
    wrap_with_command_marker, ComposerPopup, SlashOutcome,
};
use crate::markdown::{parse_markdown, Inline, MdBlock};
use crate::syntax_theme::EditorPalette;
use crate::theme::ThemeStore;
use labonair_ui_kit::{
    button, disclosure, divider, field_input, segmented_control, toggle_base, Axis, ButtonSize,
    ButtonVariant, IconName, InputEvent, InputState, ListItem, Palette, SegmentSize, ToggleSize,
    ToggleVariant, DISABLED_OPACITY,
};

/// A composer attachment shown as a chip and embedded into the outgoing message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentKind {
    /// A selection captured from a terminal or editor.
    Selection,
    /// The contents of a text file.
    File,
    /// A referenced image (path only — vision payloads are a later pass).
    Image,
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub kind: AttachmentKind,
    /// Short label for the chip (source name / path).
    pub label: String,
    /// Embedded content (the file text / selection text; empty for images).
    pub content: String,
}

impl Attachment {
    fn embed(&self) -> String {
        match self.kind {
            AttachmentKind::Selection => {
                format!(
                    "<selection source=\"{}\">\n{}\n</selection>",
                    self.label, self.content
                )
            }
            AttachmentKind::File => {
                format!("<file path=\"{}\">\n{}\n</file>", self.label, self.content)
            }
            AttachmentKind::Image => format!("<image path=\"{}\"></image>", self.label),
        }
    }

    fn glyph(&self) -> IconName {
        match self.kind {
            AttachmentKind::Selection => IconName::Scissors,
            AttachmentKind::File => IconName::File,
            AttachmentKind::Image => IconName::Image,
        }
    }
}

/// Prepend the attachment blocks to `text` for the outgoing user message.
pub fn compose_message(text: &str, attachments: &[Attachment]) -> String {
    let mut out = String::new();
    for a in attachments {
        out.push_str(&a.embed());
        out.push_str("\n\n");
    }
    out.push_str(text);
    out
}

/// Expand `#handle` tokens in `text` against the directive store. Returns the
/// prose with matched tokens stripped, plus the `<directive>` blocks to prepend
/// to the outgoing message. Unknown tokens are left untouched. Port of
/// `expandDirectiveTokens` (`reference-src/src/modules/ai/lib/directives.ts`).
pub fn expand_directive_tokens(
    text: &str,
    directives: &[labonair_backend::modules::directives::Directive],
) -> (String, Vec<String>) {
    let chars: Vec<char> = text.chars().collect();
    let mut matched: Vec<(String, String)> = Vec::new();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let at_boundary = i == 0 || chars[i - 1].is_whitespace();
        if chars[i] == '#' && at_boundary {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '-') {
                j += 1;
            }
            // Trim trailing dashes off the handle (word boundary, like the JS `\b`).
            let mut end = j;
            while end > start && chars[end - 1] == '-' {
                end -= 1;
            }
            let handle: String = chars[start..end]
                .iter()
                .collect::<String>()
                .to_ascii_lowercase();
            let valid = !handle.is_empty() && !handle.starts_with('-');
            if valid {
                if let Some(d) = directives.iter().find(|d| d.handle == handle) {
                    if !matched.iter().any(|(id, _)| id == &d.id) {
                        matched.push((
                            d.id.clone(),
                            format!(
                                "<directive name=\"{}\">\n{}\n</directive>",
                                d.handle, d.content
                            ),
                        ));
                    }
                    i = end;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    let blocks = matched.into_iter().map(|(_, b)| b).collect();
    let cleaned = out
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    (cleaned, blocks)
}

/// True when the scroll viewport is pinned within `threshold` px of the bottom.
/// `offset_y` is the (non-positive) scroll offset, `max_h` the max scroll extent.
pub fn is_at_bottom(offset_y: f32, max_h: f32, threshold: f32) -> bool {
    if max_h <= 0.0 {
        return true;
    }
    (max_h - (-offset_y)) <= threshold
}

/// Icon for a tool-call chip, keyed on the tool name.
fn tool_icon(name: &str) -> IconName {
    let n = name.to_ascii_lowercase();
    if n.contains("read") || n.contains("open") {
        IconName::File
    } else if n.contains("write") || n.contains("edit") || n.contains("create") {
        IconName::Pencil
    } else if n.contains("bash") || n.contains("run") || n.contains("exec") || n.contains("shell") {
        IconName::Terminal
    } else if n.contains("search") || n.contains("grep") || n.contains("glob") || n.contains("find")
    {
        IconName::Search
    } else if n.contains("list") || n.contains("dir") {
        IconName::Folder
    } else {
        IconName::Zap
    }
}

/// A one-line summary of a tool call's arguments JSON — the first string-ish
/// value (path / command / query), else a compacted preview.
fn tool_summary(args_json: &str) -> String {
    if args_json.trim().is_empty() {
        return String::new();
    }
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(args_json)
    {
        for key in [
            "path",
            "file_path",
            "command",
            "query",
            "pattern",
            "cmd",
            "url",
        ] {
            if let Some(serde_json::Value::String(s)) = map.get(key) {
                return truncate(s, 120);
            }
        }
    }
    truncate(
        &args_json.split_whitespace().collect::<Vec<_>>().join(" "),
        120,
    )
}

fn run_status_label(status: RunStatus) -> Option<&'static str> {
    match status {
        RunStatus::Idle => None,
        RunStatus::Thinking => Some("Thinking\u{2026}"),
        RunStatus::Streaming => Some("Streaming\u{2026}"),
        RunStatus::AwaitingApproval => Some("Awaiting approval"),
        RunStatus::Error => Some("Error"),
    }
}

/// Map a fenced-code info string to an editor [`Language`] for highlighting.
fn fence_language(lang: Option<&str>) -> Language {
    let Some(raw) = lang else {
        return Language::PlainText;
    };
    let lower = raw.trim().to_ascii_lowercase();
    let ext = match lower.as_str() {
        "rust" | "rs" => "rs",
        "python" | "py" => "py",
        "javascript" | "js" | "node" => "js",
        "typescript" | "ts" => "ts",
        "tsx" => "tsx",
        "json" | "jsonc" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yml",
        "sh" | "bash" | "shell" | "zsh" | "console" => "sh",
        "go" | "golang" => "go",
        "c" => "c",
        "cpp" | "c++" | "cxx" => "cpp",
        "sql" => "sql",
        "java" => "java",
        "html" => "html",
        "css" => "css",
        "md" | "markdown" => "md",
        other => other,
    };
    Language::from_path(format!("snippet.{ext}"))
}

const CHAT_MIN_W: f32 = 240.0;

struct ChatColors {
    bg: gpui::Hsla,
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    border: gpui::Hsla,
    accent: gpui::Hsla,
    card: gpui::Hsla,
    user_bubble: gpui::Hsla,
    code_bg: gpui::Hsla,
    error: gpui::Hsla,
    link: gpui::Hsla,
}

/// ModelPicker tab (reference: All / Favorites / Recent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTab {
    All,
    Favorites,
    Recent,
}

impl ModelTab {
    fn label(self) -> &'static str {
        match self {
            ModelTab::All => "All",
            ModelTab::Favorites => "Favorites",
            ModelTab::Recent => "Recent",
        }
    }
}

/// The GPUI chat panel — renders off an [`AiChatStore`] and owns the composer.
pub struct AiChatView {
    store: Entity<AiChatStore>,
    theme: Entity<ThemeStore>,
    /// Real multi-line text input — created lazily on the first `render`
    /// (needs a `Window`). `None` before then / in headless tests.
    composer_input: Option<Entity<InputState>>,
    /// Seed text for the composer before the input exists (tests + first
    /// render). Once the input is live this stays empty.
    composer_seed: String,
    attachments: Vec<Attachment>,
    focus: FocusHandle,
    scroll: ScrollHandle,
    /// Auto-scroll to the newest message while the user is at the bottom.
    stick_bottom: bool,
    /// Session switcher dropdown open state.
    session_menu: bool,
    /// Model-picker dropdown open state (T16-019 — was a click-cycle).
    model_menu: bool,
    /// ModelPicker: active tab (All / Favorites / Recent).
    model_tab: ModelTab,
    /// ModelPicker: provider-rail filter (`None` = all providers).
    model_provider: Option<ProviderId>,
    /// ModelPicker: fuzzy search text.
    model_search: String,
    /// ModelPicker: lazily-built search input.
    model_search_input: Option<Entity<InputState>>,
    /// Favourites + recently-used model ids (persisted).
    model_prefs: ModelPrefs,
    /// Agent-switcher dropdown open state.
    agent_menu: bool,
    /// AI agents (builtin + user) + the active id (persisted via the backend).
    agents: Vec<labonair_backend::modules::agents::Agent>,
    active_agent_id: String,
    /// Tool-call chips the user has expanded (tool call id).
    expanded_tools: HashSet<String>,
    /// Reasoning blocks the user has expanded (message id).
    expanded_reasoning: HashSet<String>,
    /// Parsed-markdown cache keyed by message id → (content length, blocks).
    md_cache: HashMap<String, (usize, Vec<MdBlock>)>,
    /// Active `/` or `@` autocomplete popover (recomputed on every keystroke).
    composer_popup: Option<ComposerPopup>,
    /// Cached workspace file list (rel paths) for the `@`-file picker.
    popup_files: Vec<String>,
    /// AI⇄Shell toggle — when `true`, composed text runs in the active terminal
    /// instead of being sent to the model (reference `AiInputBar` shell mode).
    shell_mode: bool,
    /// Plan-review rows whose diff the user has expanded (plan-edit id).
    expanded_plan: HashSet<String>,
}

impl AiChatView {
    pub fn new(
        store: Entity<AiChatStore>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&store, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();

        use labonair_backend::modules::agents;
        let loaded = agents::load();
        let mut all = agents::builtin_agents();
        all.extend(loaded.custom);
        let active_agent_id = if all.iter().any(|a| a.id == loaded.active_id) {
            loaded.active_id
        } else {
            agents::default_active_id()
        };
        if let Some(a) = all.iter().find(|a| a.id == active_agent_id) {
            let instr = a.instructions.clone();
            store.update(cx, |s, _| s.set_agent_instructions(instr));
        }

        Self {
            store,
            theme,
            composer_input: None,
            composer_seed: String::new(),
            attachments: Vec::new(),
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            stick_bottom: true,
            session_menu: false,
            model_menu: false,
            model_tab: ModelTab::All,
            model_provider: None,
            model_search: String::new(),
            model_search_input: None,
            model_prefs: model_prefs::load(),
            agent_menu: false,
            agents: all,
            active_agent_id,
            expanded_tools: HashSet::new(),
            expanded_reasoning: HashSet::new(),
            md_cache: HashMap::new(),
            composer_popup: None,
            popup_files: Vec::new(),
            shell_mode: false,
            expanded_plan: HashSet::new(),
        }
    }

    /// Recompute the `/` `@` autocomplete popover from the current composer text.
    fn refresh_popup(&mut self, text: &str, cx: &mut Context<Self>) {
        let next = detect_popup(text);
        if let Some(ComposerPopup::File { query }) = &next {
            if !query.is_empty() {
                if let Some(root) = self.store.read(cx).workspace_cwd() {
                    self.popup_files = labonair_backend::modules::fs::search::fs_search(
                        root,
                        query.clone(),
                        Some(50),
                        Some(false),
                    )
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|h| !h.is_dir)
                    .map(|h| h.rel)
                    .collect();
                }
            } else {
                self.popup_files.clear();
            }
        }
        self.composer_popup = next;
        cx.notify();
    }

    /// On Enter with an open popover, complete from it instead of sending.
    /// Returns `true` when the keystroke was consumed.
    fn try_complete_from_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match self.composer_popup.clone() {
            Some(ComposerPopup::Slash { query }) => match filter_slash(&query).first() {
                Some(cmd) => {
                    self.run_slash(cmd.name, window, cx);
                    true
                }
                None => false,
            },
            Some(ComposerPopup::File { query }) => {
                match filter_files(&query, &self.popup_files, 20).first() {
                    Some(path) => {
                        self.insert_file_mention(path.clone(), window, cx);
                        true
                    }
                    None => false,
                }
            }
            None => false,
        }
    }

    fn insert_file_mention(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        let cur = self.composer_text(cx);
        let next = apply_file_mention(&cur, &path);
        if let Some(input) = self.composer_input.clone() {
            input.update(cx, |s, cx| s.set_value(&next, window, cx));
        } else {
            self.composer_seed = next;
        }
        self.composer_popup = None;
        cx.notify();
    }

    /// Execute a slash command by name (popover click or Enter).
    fn run_slash(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.composer_popup = None;
        let plan_now = self.store.read(cx).plan_mode();
        let (outcome, plan_next) = parse_slash(&format!("/{name}"), plan_now);
        self.store
            .update(cx, |s, cx| s.set_plan_mode(plan_next, cx));
        match outcome {
            SlashOutcome::SendPrompt { prompt, command } => {
                let body = wrap_with_command_marker(&prompt, command);
                self.clear_composer(Some(window), cx);
                self.stick_bottom = true;
                self.store.update(cx, |s, cx| s.send(body, cx));
            }
            SlashOutcome::Handled(_) => {
                self.clear_composer(Some(window), cx);
            }
            SlashOutcome::None => {}
        }
        cx.notify();
    }

    /// Switch the active agent — persists + pushes its instructions to the
    /// store for the next turn.
    fn set_agent(&mut self, id: String, cx: &mut Context<Self>) {
        if self.active_agent_id == id {
            self.agent_menu = false;
            cx.notify();
            return;
        }
        self.active_agent_id = id.clone();
        self.agent_menu = false;
        let instr = self
            .agents
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.instructions.clone())
            .unwrap_or_default();
        self.store
            .update(cx, |s, _| s.set_agent_instructions(instr));
        use labonair_backend::modules::agents;
        let custom: Vec<agents::Agent> = self
            .agents
            .iter()
            .filter(|a| !a.built_in)
            .cloned()
            .collect();
        let _ = agents::save(&custom, &id);
        cx.notify();
    }

    /// `(id, title, is_active)` for every chat session — command palette.
    pub fn session_choices(&self, cx: &App) -> Vec<(String, String, bool)> {
        let s = self.store.read(cx);
        let active = s.active_id().map(str::to_string);
        s.sessions()
            .iter()
            .map(|m| {
                (
                    m.id.clone(),
                    m.title.clone(),
                    Some(&m.id) == active.as_ref(),
                )
            })
            .collect()
    }

    /// Switch to a chat session by id (command palette).
    pub fn switch_to_session(&mut self, id: &str, cx: &mut Context<Self>) {
        self.store.update(cx, |s, cx| s.switch_session(id, cx));
        cx.notify();
    }

    fn active_agent_name(&self) -> String {
        self.agents
            .iter()
            .find(|a| a.id == self.active_agent_id)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Coder".to_string())
    }

    /// Attach a captured terminal/editor selection to the composer.
    pub fn attach_selection(
        &mut self,
        label: impl Into<String>,
        content: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.attachments.push(Attachment {
            kind: AttachmentKind::Selection,
            label: label.into(),
            content: content.into(),
        });
        cx.notify();
    }

    /// Attach a text file's contents (truncated) to the composer.
    pub fn attach_file(&mut self, path: impl Into<String>, cx: &mut Context<Self>) {
        let path = path.into();
        let content = std::fs::read_to_string(&path)
            .map(|s| s.chars().take(16_000).collect::<String>())
            .unwrap_or_default();
        self.attachments.push(Attachment {
            kind: AttachmentKind::File,
            label: path,
            content,
        });
        cx.notify();
    }

    /// Start a fresh AI session (menu "New AI Session").
    pub fn new_session(&mut self, cx: &mut Context<Self>) {
        self.store.update(cx, |s, cx| {
            s.new_session(cx);
        });
        cx.notify();
    }

    /// Discard the active session and start a new empty one (menu
    /// "Clear Current Chat"). Mirrors the reference `menu:clear_chat`.
    pub fn clear_active_chat(&mut self, cx: &mut Context<Self>) {
        self.store.update(cx, |s, cx| {
            if let Some(id) = s.active_id().map(str::to_string) {
                s.delete_session(&id, cx);
            }
            s.new_session(cx);
        });
        cx.notify();
    }

    /// Current composer text (from the live input, or the pre-render seed).
    fn composer_text(&self, cx: &App) -> String {
        match &self.composer_input {
            Some(input) => input.read(cx).value().to_string(),
            None => self.composer_seed.clone(),
        }
    }

    fn clear_composer(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        self.composer_seed.clear();
        if let (Some(input), Some(window)) = (self.composer_input.clone(), window) {
            input.update(cx, |s, cx| s.set_value("", window, cx));
        }
    }

    /// Lazily build the real multi-line text input (needs a `Window`, so this
    /// runs on the first `render`). Wires Enter → send, ⌘↵ → enqueue.
    fn ensure_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.composer_input.is_some() {
            return;
        }
        let seed = std::mem::take(&mut self.composer_seed);
        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx)
                .multi_line(true)
                .auto_grow(2, 8)
                .placeholder("Message Labonair\u{2026}");
            if !seed.is_empty() {
                s.set_value(seed, window, cx);
            }
            s
        });
        let view = cx.entity();
        window
            .subscribe(
                &input,
                cx,
                move |input, ev: &InputEvent, window, cx| match ev {
                    InputEvent::Change => {
                        let v = input.read(cx).value().to_string();
                        view.update(cx, |this, cx| this.refresh_popup(&v, cx));
                    }
                    InputEvent::PressEnter { secondary } => {
                        // Multi-line already inserted a newline — drop the last one
                        // and send (plain Enter) or enqueue (⌘↵).
                        let v = input.read(cx).value().to_string();
                        let trimmed = v.strip_suffix('\n').unwrap_or(&v).to_string();
                        input.update(cx, |s, cx| s.set_value(&trimmed, window, cx));
                        let enqueue = *secondary;
                        view.update(cx, |this, cx| {
                            if this.try_complete_from_popup(window, cx) {
                                return;
                            }
                            if enqueue {
                                this.enqueue(Some(window), cx);
                            } else {
                                this.send(Some(window), cx);
                            }
                        });
                    }
                    _ => {}
                },
            )
            .detach();
        self.composer_input = Some(input);
    }

    fn can_send(&self, cx: &App) -> bool {
        !self.store.read(cx).is_streaming()
            && (!self.composer_text(cx).trim().is_empty() || !self.attachments.is_empty())
    }

    fn send(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        let text = self.composer_text(cx).trim().to_string();
        if text.is_empty() && self.attachments.is_empty() {
            return;
        }
        // AI⇄Shell mode — run the text in the active terminal, don't message.
        if self.shell_mode && !text.is_empty() {
            cx.emit(AiChatEvent::RunInTerminal(text));
            self.composer_popup = None;
            self.clear_composer(window, cx);
            cx.notify();
            return;
        }
        // Intercept `/init` / `/plan` before treating the text as a message.
        let plan_now = self.store.read(cx).plan_mode();
        let (outcome, plan_next) = parse_slash(&text, plan_now);
        if !matches!(outcome, SlashOutcome::None) {
            self.composer_popup = None;
            self.store
                .update(cx, |s, cx| s.set_plan_mode(plan_next, cx));
            self.clear_composer(window, cx);
            if let SlashOutcome::SendPrompt { prompt, command } = outcome {
                let body = wrap_with_command_marker(&prompt, command);
                self.stick_bottom = true;
                self.store.update(cx, |s, cx| s.send(body, cx));
            }
            cx.notify();
            return;
        }
        let (text, directive_blocks) =
            expand_directive_tokens(&text, &labonair_backend::modules::directives::load());
        let mut body = compose_message(&text, &self.attachments);
        if !directive_blocks.is_empty() {
            body = format!("{}\n\n{}", directive_blocks.join("\n\n"), body);
        }
        self.clear_composer(window, cx);
        self.attachments.clear();
        self.stick_bottom = true;
        self.store.update(cx, |s, cx| s.send(body, cx));
        cx.notify();
    }

    /// ⌘↵ — queue the message as a follow-up turn instead of sending now.
    fn enqueue(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        let text = self.composer_text(cx).trim().to_string();
        if text.is_empty() {
            return;
        }
        self.store.update(cx, |s, cx| s.enqueue_prompt(text, cx));
        self.clear_composer(window, cx);
        cx.notify();
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        self.store.update(cx, |s, cx| s.stop(cx));
        cx.notify();
    }

    fn on_scroll(&mut self, _ev: &ScrollWheelEvent, _w: &mut Window, cx: &mut Context<Self>) {
        let off = self.scroll.offset().y;
        let max = self.scroll.max_offset().height;
        self.stick_bottom = is_at_bottom(f32::from(off), f32::from(max), 48.0);
        cx.notify();
    }

    fn colors(&self, cx: &App) -> ChatColors {
        let t = self.theme.read(cx);
        ChatColors {
            bg: t.background(),
            fg: t.foreground(),
            muted: t.muted_foreground(),
            border: t.border(),
            accent: t.accent(),
            card: t.card(),
            user_bubble: t.accent(),
            code_bg: t.muted(),
            error: t.status_error(),
            link: t.primary(),
        }
    }

    // ── rendering ─────────────────────────────────────────────────────────

    fn render_header(&self, c: &ChatColors, cx: &mut Context<Self>) -> impl IntoElement {
        let store = self.store.read(cx);
        let active_id = store.active_id().map(str::to_string);
        let title = store
            .sessions()
            .iter()
            .find(|s| Some(&s.id) == active_id.as_ref())
            .map(|s| s.title.clone())
            .unwrap_or_else(|| "New chat".to_string());
        let model_id = store
            .model_ref()
            .split('@')
            .next()
            .unwrap_or("")
            .to_string();
        let model_label = find_model(&model_id)
            .map(|m| m.label.to_string())
            .unwrap_or_else(|| model_id.clone());
        let status = run_status_label(store.run_status());
        let sessions: Vec<SessionMeta> = store.sessions().to_vec();

        div()
            .flex()
            .flex_col()
            .w_full()
            .flex_shrink_0()
            .px_2()
            .py_1p5()
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("ai-session-toggle")
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_xs()
                            .text_color(c.fg)
                            .child(
                                div()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(SharedString::from(title)),
                            )
                            .child(div().text_color(c.muted).child("\u{25be}"))
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                this.session_menu = !this.session_menu;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("ai-agent-pick")
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_1p5()
                            .py_0p5()
                            .rounded_sm()
                            .border_1()
                            .border_color(c.border)
                            .text_color(c.muted)
                            .text_size(px(10.0))
                            .hover(|s| s.border_color(c.accent).text_color(c.fg))
                            .child(IconName::Sparkles.svg(c.muted).size(px(11.0)))
                            .child(SharedString::from(self.active_agent_name()))
                            .child("\u{25be}")
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                this.agent_menu = !this.agent_menu;
                                this.model_menu = false;
                                this.session_menu = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("ai-model-pick")
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_1p5()
                            .py_0p5()
                            .rounded_sm()
                            .border_1()
                            .border_color(c.border)
                            .text_color(c.muted)
                            .text_size(px(10.0))
                            .hover(|s| s.border_color(c.accent).text_color(c.fg))
                            .child(SharedString::from(model_label))
                            .child("\u{25be}")
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                this.model_menu = !this.model_menu;
                                this.agent_menu = false;
                                this.session_menu = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("ai-new-session")
                            .size(px(20.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .text_color(c.muted)
                            .hover(|s| s.bg(c.border).text_color(c.fg))
                            .child("+")
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                this.store.update(cx, |s, cx| {
                                    s.new_session(cx);
                                });
                                this.session_menu = false;
                                cx.notify();
                            })),
                    ),
            )
            .when_some(status, |d, label| {
                d.child(
                    div()
                        .mt_1()
                        .text_size(px(10.0))
                        .text_color(if label == "Error" { c.error } else { c.muted })
                        .child(SharedString::from(label)),
                )
            })
            .when(self.session_menu, |d| {
                d.child(self.render_session_menu(&sessions, active_id.as_deref(), c, cx))
            })
            .when(self.agent_menu, |d| d.child(self.render_agent_menu(c, cx)))
            .when(self.model_menu, |d| d.child(self.render_model_menu(c, cx)))
    }

    fn render_agent_menu(&self, c: &ChatColors, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.active_agent_id.clone();
        div()
            .id("ai-agent-menu")
            .mt_1()
            .flex()
            .flex_col()
            .rounded_sm()
            .border_1()
            .border_color(c.border)
            .bg(c.card)
            .p_1()
            .children(self.agents.iter().map(|a| {
                let id = a.id.clone();
                let on = a.id == active;
                // T20-003: shared `ListItem` shell. `selected_fill` doubles as
                // the hover tint (the codebase-wide `ListItem` convention —
                // see hosts-ui/panel-snippets) rather than the pre-migration
                // row's separate hover-vs-selected colours.
                ListItem::new(
                    SharedString::from(format!("agent-{}", a.id)),
                    c.fg,
                    c.muted,
                    c.accent.opacity(0.15),
                )
                .selected(on)
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(c.fg)
                        .child(SharedString::from(a.name.clone())),
                )
                .child(
                    div()
                        .text_size(px(9.0))
                        .text_color(c.muted)
                        .child(SharedString::from(a.description.clone())),
                )
                .on_click(
                    cx.listener(move |this, _: &ClickEvent, _w, cx| this.set_agent(id.clone(), cx)),
                )
                .extra(|row| row.flex_col().items_start().px_2().py_1())
                .into_any_element()
            }))
    }

    /// Models visible under the current tab / provider / search filter.
    fn visible_models(&self) -> Vec<&'static labonair_ai::ModelInfo> {
        let q = self.model_search.trim().to_lowercase();
        let mut v: Vec<&'static labonair_ai::ModelInfo> = MODELS
            .iter()
            .filter(|m| {
                let tab_ok = match self.model_tab {
                    ModelTab::All => true,
                    ModelTab::Favorites => self.model_prefs.is_favorite(m.id),
                    ModelTab::Recent => self.model_prefs.recent.iter().any(|r| r == m.id),
                };
                let prov_ok = self.model_provider.is_none_or(|p| m.provider == p);
                let search_ok = q.is_empty()
                    || m.label.to_lowercase().contains(&q)
                    || m.id.to_lowercase().contains(&q)
                    || m.provider.label().to_lowercase().contains(&q);
                tab_ok && prov_ok && search_ok
            })
            .collect();
        if self.model_tab == ModelTab::Recent {
            v.sort_by_key(|m| {
                self.model_prefs
                    .recent
                    .iter()
                    .position(|r| r == m.id)
                    .unwrap_or(usize::MAX)
            });
        }
        v
    }

    fn select_model(&mut self, id: &'static str, cx: &mut Context<Self>) {
        self.store.update(cx, |s, cx| s.set_model_ref(id, cx));
        self.model_prefs.push_recent(id);
        let _ = model_prefs::save(&self.model_prefs);
        self.model_menu = false;
        cx.notify();
    }

    fn toggle_model_favorite(&mut self, id: &str, cx: &mut Context<Self>) {
        self.model_prefs.toggle_favorite(id);
        let _ = model_prefs::save(&self.model_prefs);
        cx.notify();
    }

    /// Lazily build the ModelPicker search input (needs a `Window`).
    fn ensure_model_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.model_search_input.is_some() {
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search models\u{2026}"));
        let view = cx.entity();
        window
            .subscribe(&input, cx, move |input, ev: &InputEvent, _w, cx| {
                if matches!(ev, InputEvent::Change) {
                    let v = input.read(cx).value().to_string();
                    view.update(cx, |this, cx| {
                        this.model_search = v;
                        cx.notify();
                    });
                }
            })
            .detach();
        self.model_search_input = Some(input);
    }

    fn render_model_menu(&self, c: &ChatColors, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::from_theme(self.theme.read(cx));
        let cur = self
            .store
            .read(cx)
            .model_ref()
            .split('@')
            .next()
            .unwrap_or("")
            .to_string();
        // Providers that actually have a catalog entry.
        let providers: Vec<ProviderId> = ProviderId::ALL
            .into_iter()
            .filter(|prov| MODELS.iter().any(|m| m.provider == *prov))
            .collect();
        let models = self.visible_models();

        div()
            .id("ai-model-menu")
            .mt_1()
            .flex()
            .flex_col()
            .w(px(380.0))
            .max_h(px(360.0))
            .rounded_sm()
            .border_1()
            .border_color(c.border)
            .bg(c.card)
            .overflow_hidden()
            // Search + tabs.
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_1p5()
                    .border_b_1()
                    .border_color(c.border)
                    .child(
                        div()
                            .rounded_sm()
                            .border_1()
                            .border_color(c.border)
                            .px_1p5()
                            .py_0p5()
                            .text_size(px(11.0))
                            .children(
                                self.model_search_input
                                    .as_ref()
                                    .map(|i| field_input(i).appearance(false)),
                            ),
                    )
                    .child(
                        // T20-003: shared `SegmentedControl` for the
                        // All/Favorites/Recent tab strip.
                        segmented_control("ai-model-tabs", p, self.model_tab.label())
                            .segment(ModelTab::All.label(), ModelTab::All.label())
                            .segment(ModelTab::Favorites.label(), ModelTab::Favorites.label())
                            .segment(ModelTab::Recent.label(), ModelTab::Recent.label())
                            .size(SegmentSize::Xs)
                            .on_select(cx.listener(|this, key: &SharedString, _w, cx| {
                                this.model_tab = match key.as_ref() {
                                    "Favorites" => ModelTab::Favorites,
                                    "Recent" => ModelTab::Recent,
                                    _ => ModelTab::All,
                                };
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    // Provider rail.
                    .child(
                        div()
                            .id("ai-model-providers")
                            .flex()
                            .flex_col()
                            .w(px(104.0))
                            .flex_shrink_0()
                            .border_r_1()
                            .border_color(c.border)
                            .overflow_y_scroll()
                            .p_1()
                            .child(
                                // T20-003: shared `ListItem` shell (see the
                                // agent-menu note above for the
                                // hover==selected-fill convention).
                                ListItem::new("mp-all", c.fg, c.muted, c.accent.opacity(0.15))
                                    .selected(self.model_provider.is_none())
                                    .child("All providers")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.model_provider = None;
                                        cx.notify();
                                    }))
                                    .extra(|row| row.text_size(px(10.0)).px_1p5().py_1())
                                    .into_any_element(),
                            )
                            .children(providers.into_iter().map(|prov| {
                                let on = self.model_provider == Some(prov);
                                ListItem::new(
                                    SharedString::from(format!("mp-{}", prov.as_str())),
                                    c.fg,
                                    c.muted,
                                    c.accent.opacity(0.15),
                                )
                                .selected(on)
                                .child(SharedString::from(prov.label()))
                                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                    this.model_provider = Some(prov);
                                    cx.notify();
                                }))
                                .extra(|row| row.text_size(px(10.0)).px_1p5().py_1())
                                .into_any_element()
                            })),
                    )
                    // Model list.
                    .child(
                        div()
                            .id("ai-model-list")
                            .flex_1()
                            .min_w_0()
                            .overflow_y_scroll()
                            .p_1()
                            .when(models.is_empty(), |d| {
                                d.child(
                                    div()
                                        .p_2()
                                        .text_size(px(10.0))
                                        .text_color(c.muted)
                                        .child("No models match"),
                                )
                            })
                            .children(models.into_iter().map(|m| {
                                let id = m.id;
                                let on = m.id == cur;
                                let fav = self.model_prefs.is_favorite(m.id);
                                let caps = {
                                    let ctx = if m.context_limit >= 1000 {
                                        format!("{}K ctx", m.context_limit / 1000)
                                    } else {
                                        format!("{} ctx", m.context_limit)
                                    };
                                    let tags = m
                                        .tags
                                        .iter()
                                        .map(|t| format!("{t:?}").to_lowercase())
                                        .collect::<Vec<_>>()
                                        .join(" \u{00b7} ");
                                    if tags.is_empty() {
                                        ctx
                                    } else {
                                        format!("{ctx} \u{00b7} {tags}")
                                    }
                                };
                                // T20-003: shared `ListItem` shell — the
                                // favourite-star toggle and the name/caps
                                // block keep their own independent click
                                // handlers as separate children (star toggles
                                // the favourite, the rest selects the model),
                                // exactly as before; `ListItem` only supplies
                                // the row chrome (selected tint, hover) around
                                // them. No dedicated star `IconName` exists in
                                // the ui-kit icon set, so this keeps the
                                // reference's ★/☆ glyph rather than inventing
                                // one — `icon_toggle_button` needs an
                                // `IconName`.
                                ListItem::new(
                                    SharedString::from(format!("model-row-{}", m.id)),
                                    c.fg,
                                    c.muted,
                                    c.accent.opacity(0.12),
                                )
                                .selected(on)
                                .child(
                                    div()
                                        .id(SharedString::from(format!("mfav-{}", m.id)))
                                        .text_size(px(11.0))
                                        .text_color(if fav { c.accent } else { c.muted })
                                        .child(if fav { "\u{2605}" } else { "\u{2606}" })
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, _w, cx| {
                                                this.toggle_model_favorite(id, cx)
                                            },
                                        )),
                                )
                                .child(
                                    div()
                                        .id(SharedString::from(format!("model-{}", m.id)))
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(c.fg)
                                                .child(SharedString::from(m.label)),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(9.0))
                                                .text_color(c.muted)
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .child(SharedString::from(caps)),
                                        )
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, _w, cx| {
                                                this.select_model(id, cx)
                                            },
                                        )),
                                )
                                .extra(|row| row.cursor_default())
                                .into_any_element()
                            })),
                    ),
            )
    }

    fn render_session_menu(
        &self,
        sessions: &[SessionMeta],
        active: Option<&str>,
        c: &ChatColors,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id("ai-session-menu")
            .mt_1()
            .flex()
            .flex_col()
            .gap_0p5()
            .max_h(px(220.0))
            .overflow_y_scroll()
            .rounded_sm()
            .border_1()
            .border_color(c.border)
            .bg(c.card)
            .p_1()
            .children(sessions.iter().map(|s| {
                let id = s.id.clone();
                let id2 = s.id.clone();
                let is_active = Some(s.id.as_str()) == active;
                // T20-003: shared `ListItem` shell. The row itself has no
                // click handler (the title and the delete glyph each carry
                // their own), so `.cursor_default()` keeps the pre-migration
                // arrow cursor over the row padding instead of `ListItem`'s
                // default pointer cursor.
                ListItem::new(SharedString::from(s.id.clone()), c.fg, c.muted, c.border)
                    .selected(is_active)
                    .child(
                        div()
                            .id(SharedString::from(format!("sess-title-{}", s.id)))
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_color(if is_active { c.fg } else { c.muted })
                            .child(SharedString::from(s.title.clone()))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.store.update(cx, |st, cx| st.switch_session(&id, cx));
                                this.session_menu = false;
                                cx.notify();
                            })),
                    )
                    .trailing(
                        div()
                            .id(SharedString::from(format!("sess-del-{}", s.id)))
                            .text_color(c.muted)
                            .hover(|s| s.text_color(c.error))
                            .child("\u{00d7}")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.store.update(cx, |st, cx| st.delete_session(&id2, cx));
                                cx.notify();
                            })),
                    )
                    .extra(|row| row.gap_1().text_size(px(11.0)).cursor_default())
                    .into_any_element()
            }))
    }

    fn render_messages(&mut self, c: &ChatColors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let messages: Vec<SessionMessage> = self.store.read(cx).active_messages().to_vec();
        let editor_theme = self.theme.read(cx).editor_theme();
        let palette = EditorPalette::resolve(editor_theme, self.theme.read(cx));

        if messages.is_empty() {
            return div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_1()
                .p_4()
                .text_center()
                .child(
                    div()
                        .text_xs()
                        .text_color(c.fg)
                        .child("Ask Labonair anything"),
                )
                .child(
                    div().text_size(px(10.0)).text_color(c.muted).child(
                        "Explain command output, fix errors, generate snippets, or run a task.",
                    ),
                )
                .into_any_element();
        }

        if self.stick_bottom {
            self.scroll.scroll_to_bottom();
        }

        // Prune cache entries for messages that no longer exist.
        let live: HashSet<&str> = messages.iter().map(|m| m.id.as_str()).collect();
        self.md_cache.retain(|k, _| live.contains(k.as_str()));

        let rows: Vec<gpui::AnyElement> = messages
            .iter()
            .map(|m| self.render_message(m, c, &palette, cx))
            .collect();

        div()
            .id("ai-messages")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .child(div().flex().flex_col().gap_3().p_3().children(rows))
            .into_any_element()
    }

    fn render_message(
        &mut self,
        m: &SessionMessage,
        c: &ChatColors,
        palette: &EditorPalette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match m.role {
            Role::User => self.render_user_message(m, c).into_any_element(),
            Role::System => div()
                .text_size(px(10.0))
                .text_color(c.muted)
                .child(SharedString::from(m.content.clone()))
                .into_any_element(),
            _ => self.render_assistant_message(m, c, palette, cx),
        }
    }

    fn render_user_message(&self, m: &SessionMessage, c: &ChatColors) -> impl IntoElement {
        let (chips, text) = split_context_blocks(&m.content);
        div().flex().flex_col().items_end().gap_1().child(
            div()
                .max_w(px(360.0))
                .rounded_md()
                .bg(c.user_bubble)
                .text_color(c.bg)
                .px_2p5()
                .py_1p5()
                .text_xs()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .when(!chips.is_empty(), |d| {
                            d.children(chips.iter().map(|chip| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .text_size(px(10.0))
                                    .opacity(0.85)
                                    .child(IconName::Paperclip.svg(c.muted).size(px(11.0)))
                                    .child(SharedString::from(chip.clone()))
                            }))
                        })
                        .when(!text.trim().is_empty(), |d| {
                            d.child(
                                div()
                                    .whitespace_normal()
                                    .child(SharedString::from(text.trim().to_string())),
                            )
                        }),
                ),
        )
    }

    fn render_assistant_message(
        &mut self,
        m: &SessionMessage,
        c: &ChatColors,
        palette: &EditorPalette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let cache_hit = self
            .md_cache
            .get(&m.id)
            .is_some_and(|(len, _)| *len == m.content.len());
        if !cache_hit {
            let blocks = parse_markdown(&m.content);
            self.md_cache
                .insert(m.id.clone(), (m.content.len(), blocks));
        }
        let blocks = self
            .md_cache
            .get(&m.id)
            .map(|(_, b)| b.clone())
            .unwrap_or_default();

        let reasoning = (!m.reasoning.is_empty()).then(|| {
            let expanded = self.expanded_reasoning.contains(&m.id);
            let id = m.id.clone();
            div()
                .flex()
                .flex_col()
                .gap_1()
                .rounded_sm()
                .border_1()
                .border_color(c.border)
                .bg(c.card)
                .p_1p5()
                .child(
                    // T20-003: shared `Disclosure` chevron+label row instead
                    // of the hand-rolled ASCII-arrow toggle.
                    disclosure(
                        SharedString::from(format!("reason-{}", m.id)),
                        "Thinking",
                        !expanded,
                        c.muted,
                        c.fg,
                    )
                    .text_size(px(10.0))
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            if !this.expanded_reasoning.remove(&id) {
                                this.expanded_reasoning.insert(id.clone());
                            }
                            cx.notify();
                        },
                    )),
                )
                .when(expanded, |d| {
                    d.child(
                        div()
                            .text_size(px(11.0))
                            .text_color(c.muted)
                            .whitespace_normal()
                            .child(SharedString::from(m.reasoning.clone())),
                    )
                })
        });

        let tool_cards: Vec<gpui::AnyElement> = m
            .tool_calls
            .iter()
            .map(|tc| self.render_tool_call(tc, c, cx))
            .collect();

        let error = (m.status == MessageStatus::Error)
            .then(|| m.error.clone())
            .flatten()
            .map(|e| {
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(c.error)
                    .bg(c.error.opacity(0.12))
                    .px_2()
                    .py_1p5()
                    .text_size(px(11.0))
                    .text_color(c.error)
                    .child(SharedString::from(e))
            });

        div()
            .flex()
            .flex_col()
            .gap_2()
            .text_xs()
            .text_color(c.fg)
            .when_some(reasoning, |d, r| d.child(r))
            .children(
                blocks
                    .iter()
                    .map(|b| self.render_block(b, c, palette, cx))
                    .collect::<Vec<_>>(),
            )
            .children(tool_cards)
            .when_some(error, |d, e| d.child(e))
            .into_any_element()
    }

    fn render_tool_call(
        &self,
        tc: &SessionToolCall,
        c: &ChatColors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let p = Palette::from_theme(self.theme.read(cx));
        let pending = matches!(
            tc.status,
            ToolCallStatus::AwaitingApproval | ToolCallStatus::Streaming
        );
        let (status_text, status_color) = match tc.status {
            ToolCallStatus::Streaming => ("streaming\u{2026}", c.muted),
            ToolCallStatus::AwaitingApproval => ("awaiting approval", c.accent),
            ToolCallStatus::Done => ("done", c.muted),
            ToolCallStatus::Error => ("rejected", c.error),
        };
        let id_ok = tc.id.clone();
        let id_no = tc.id.clone();
        let id_tog = tc.id.clone();
        let expanded = self.expanded_tools.contains(&tc.id);
        let icon = tool_icon(&tc.name);
        let has_detail = !tc.arguments.is_empty() || tc.result.is_some();
        let summary = tool_summary(&tc.arguments);
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .rounded_sm()
            .border_1()
            .border_color(if pending { c.accent } else { c.border })
            .bg(c.card)
            .p_2()
            .child(
                div()
                    .id(SharedString::from(format!("toolchip-{}", tc.id)))
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(has_detail, |d| {
                        d.on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            if !this.expanded_tools.remove(&id_tog) {
                                this.expanded_tools.insert(id_tog.clone());
                            }
                            cx.notify();
                        }))
                    })
                    .child(icon.svg(c.muted).size(px(12.0)))
                    .child(
                        div()
                            .font_family("mono")
                            .text_size(px(11.0))
                            .text_color(c.fg)
                            .child(SharedString::from(tc.name.clone())),
                    )
                    .when(!summary.is_empty() && !expanded, |d| {
                        d.child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_size(px(10.0))
                                .text_color(c.muted)
                                .child(SharedString::from(summary.clone())),
                        )
                    })
                    .when(summary.is_empty() || expanded, |d| d.child(div().flex_1()))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(status_color)
                            .child(status_text),
                    )
                    .when(has_detail, |d| {
                        d.child(
                            div()
                                .text_color(c.muted)
                                .text_size(px(10.0))
                                .child(if expanded { "\u{25be}" } else { "\u{25b8}" }),
                        )
                    }),
            )
            .when(expanded && !tc.arguments.is_empty(), |d| {
                d.child(
                    div()
                        .font_family("mono")
                        .text_size(px(10.0))
                        .text_color(c.muted)
                        .whitespace_normal()
                        .child(SharedString::from(truncate(&tc.arguments, 4000))),
                )
            })
            .when(expanded, |d| {
                d.when_some(tc.result.clone(), |d, r| {
                    d.child(
                        div()
                            .font_family("mono")
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .whitespace_normal()
                            .child(SharedString::from(truncate(&r, 4000))),
                    )
                })
            })
            .when(tc.status == ToolCallStatus::AwaitingApproval, |d| {
                d.child(
                    // T20-003: shared `Button` primitive.
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            button(
                                SharedString::from(format!("approve-{}", tc.id)),
                                p,
                                ButtonVariant::Default,
                                ButtonSize::Xs,
                            )
                            .child("Approve")
                            .on_click(cx.listener(
                                move |this, _: &ClickEvent, _w, cx| {
                                    this.store
                                        .update(cx, |s, cx| s.resolve_tool_call(&id_ok, true, cx));
                                    cx.notify();
                                },
                            )),
                        )
                        .child(
                            button(
                                SharedString::from(format!("reject-{}", tc.id)),
                                p,
                                ButtonVariant::Outline,
                                ButtonSize::Xs,
                            )
                            .child("Reject")
                            .on_click(cx.listener(
                                move |this, _: &ClickEvent, _w, cx| {
                                    this.store
                                        .update(cx, |s, cx| s.resolve_tool_call(&id_no, false, cx));
                                    cx.notify();
                                },
                            )),
                        ),
                )
            })
            .into_any_element()
    }

    fn render_block(
        &self,
        block: &MdBlock,
        c: &ChatColors,
        palette: &EditorPalette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match block {
            MdBlock::Heading { level, spans } => div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(match level {
                    1 => 15.0,
                    2 => 13.5,
                    _ => 12.5,
                }))
                .text_color(c.fg)
                .child(inline_text(spans, c))
                .into_any_element(),
            MdBlock::Paragraph(spans) => div()
                .whitespace_normal()
                .child(inline_text(spans, c))
                .into_any_element(),
            MdBlock::Quote(spans) => div()
                .border_l_2()
                .border_color(c.border)
                .pl_2()
                .text_color(c.muted)
                .child(inline_text(spans, c))
                .into_any_element(),
            // T20-001: shared `Divider` primitive.
            MdBlock::Rule => divider(Axis::Horizontal, c.border).into_any_element(),
            MdBlock::Bullets(items) => div()
                .flex()
                .flex_col()
                .gap_0p5()
                .children(items.iter().map(|it| {
                    div()
                        .flex()
                        .gap_1p5()
                        .child(div().text_color(c.muted).child("\u{2022}"))
                        .child(div().flex_1().whitespace_normal().child(inline_text(it, c)))
                }))
                .into_any_element(),
            MdBlock::Ordered(items) => div()
                .flex()
                .flex_col()
                .gap_0p5()
                .children(items.iter().map(|(n, it)| {
                    div()
                        .flex()
                        .gap_1p5()
                        .child(
                            div()
                                .text_color(c.muted)
                                .child(SharedString::from(format!("{n}."))),
                        )
                        .child(div().flex_1().whitespace_normal().child(inline_text(it, c)))
                }))
                .into_any_element(),
            MdBlock::Table { headers, rows } => div()
                .flex()
                .flex_col()
                .rounded_sm()
                .border_1()
                .border_color(c.border)
                .overflow_hidden()
                .child(table_row(headers, c, true))
                .children(rows.iter().map(|r| table_row(r, c, false)))
                .into_any_element(),
            MdBlock::Code { lang, text, .. } => {
                self.render_code_block(lang.as_deref(), text, c, palette, cx)
            }
        }
    }

    fn render_code_block(
        &self,
        lang: Option<&str>,
        text: &str,
        c: &ChatColors,
        palette: &EditorPalette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let language = fence_language(lang);
        let mut hl = SyntaxHighlighter::new(language);
        if hl.has_grammar() {
            hl.update(text, 0, 0..text.len());
        }
        let mut offset = 0usize;
        let lines: Vec<gpui::AnyElement> = text
            .split('\n')
            .map(|line| {
                let runs = if hl.has_grammar() {
                    hl.line_runs(line, offset)
                } else {
                    Vec::new()
                };
                offset += line.len() + 1;
                if runs.is_empty() {
                    div()
                        .child(SharedString::from(line.to_string()))
                        .into_any_element()
                } else {
                    let mut highlights: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
                    let mut byte = 0usize;
                    for run in &runs {
                        let len = run.text.len();
                        if let Some(kind) = run.kind {
                            highlights.push((
                                byte..byte + len,
                                HighlightStyle {
                                    color: Some(palette.color(kind)),
                                    ..Default::default()
                                },
                            ));
                        }
                        byte += len;
                    }
                    StyledText::new(SharedString::from(line.to_string()))
                        .with_highlights(highlights)
                        .into_any_element()
                }
            })
            .collect();

        let label = lang.map(|l| l.to_string());
        let copy = text.to_string();
        let is_shell = matches!(
            lang.map(|l| l.trim().to_ascii_lowercase()).as_deref(),
            Some("sh" | "bash" | "shell" | "zsh" | "console" | "shell-session" | "sh-session")
        );
        // A one-liner (or a short block) is safe to offer as a single "Run".
        let runnable = is_shell && text.lines().filter(|l| !l.trim().is_empty()).count() <= 8;
        let run_cmd = text.trim().to_string();

        div()
            .flex()
            .flex_col()
            .rounded_sm()
            .border_1()
            .border_color(c.border)
            .bg(c.code_bg)
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .py_0p5()
                    .border_b_1()
                    .border_color(c.border)
                    .text_size(px(9.0))
                    .text_color(c.muted)
                    .child(SharedString::from(
                        label.unwrap_or_else(|| "code".to_string()),
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(runnable, |d| {
                                d.child(
                                    div()
                                        .id("ai-code-run")
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .text_color(c.accent)
                                        .hover(|s| s.text_color(c.fg))
                                        .child(IconName::Terminal.svg(c.accent).size(px(10.0)))
                                        .child("Run")
                                        .on_click(cx.listener(move |_, _: &ClickEvent, _w, cx| {
                                            cx.emit(AiChatEvent::RunInTerminal(run_cmd.clone()));
                                        })),
                                )
                            })
                            .child(
                                div()
                                    .id("ai-code-copy")
                                    .child("Copy")
                                    .hover(|s| s.text_color(c.fg))
                                    .on_click(move |_, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            copy.clone(),
                                        ));
                                    }),
                            ),
                    ),
            )
            .child(
                div()
                    .font_family("mono")
                    .text_size(px(11.0))
                    .p_2()
                    .flex()
                    .flex_col()
                    .children(lines),
            )
            .into_any_element()
    }

    /// Plan-mode strip above the composer — shows the flag + pending count and
    /// an "Exit" toggle (reference `PlanModeStrip`).
    fn render_plan_strip(&self, c: &ChatColors, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::from_theme(self.theme.read(cx));
        let pending = self.store.read(cx).plan_queue().len();
        let label = if pending == 0 {
            "Plan mode — edits will be queued for review".to_string()
        } else {
            format!("Plan mode — {pending} change(s) pending review")
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_1p5()
            .py_0p5()
            .rounded_sm()
            .bg(c.accent.opacity(0.12))
            .border_1()
            .border_color(c.accent.opacity(0.4))
            .text_size(px(10.0))
            .text_color(c.fg)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(IconName::SquareCheck.svg(c.accent).size(px(11.0)))
                    .child(SharedString::from(label)),
            )
            .child(
                // T20-003: shared `Button` primitive.
                button("ai-plan-exit", p, ButtonVariant::Ghost, ButtonSize::Xs)
                    .child("Exit")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.store.update(cx, |s, cx| {
                            s.set_plan_mode(false, cx);
                            s.plan_discard_all(cx);
                        });
                        cx.notify();
                    })),
            )
    }

    /// Full-panel plan-review overlay (reference `PlanDiffReview`).
    fn render_plan_review(
        &self,
        c: &ChatColors,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let p = Palette::from_theme(self.theme.read(cx));
        let queue = self.store.read(cx).plan_queue().to_vec();
        if queue.is_empty() {
            return None;
        }
        let count = queue.len();
        let rows: Vec<gpui::AnyElement> = queue
            .iter()
            .map(|e| self.render_plan_row(e, c, cx))
            .collect();
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .flex_col()
                .bg(c.bg.opacity(0.97))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .py_2()
                        .border_b_1()
                        .border_color(c.border)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(c.fg)
                                        .child("Plan review"),
                                )
                                .child(div().text_size(px(10.0)).text_color(c.muted).child(
                                    SharedString::from(format!("{count} pending change(s)")),
                                )),
                        )
                        .child(
                            // T20-003: shared `Button` primitive.
                            div()
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .child(
                                    button(
                                        "ai-plan-discard",
                                        p,
                                        ButtonVariant::Outline,
                                        ButtonSize::Xs,
                                    )
                                    .child("Discard all")
                                    .on_click(cx.listener(
                                        |this, _: &ClickEvent, _w, cx| {
                                            this.store.update(cx, |s, cx| s.plan_discard_all(cx));
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "ai-plan-apply",
                                        p,
                                        ButtonVariant::Default,
                                        ButtonSize::Xs,
                                    )
                                    .child(SharedString::from(format!("Apply {count}")))
                                    .on_click(cx.listener(
                                        |this, _: &ClickEvent, _w, cx| {
                                            this.store.update(cx, |s, cx| s.plan_apply_all(cx));
                                            cx.notify();
                                        },
                                    )),
                                ),
                        ),
                )
                .child(
                    div()
                        .id("ai-plan-list")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_1p5()
                        .p_3()
                        .children(rows),
                )
                .into_any_element(),
        )
    }

    /// T20-003: the header row's expand/collapse click stays a hand-rolled
    /// `div` (not the shared `Disclosure` primitive) — `Disclosure` is a
    /// fixed chevron+single-line-label shape, while this header is a
    /// composite three-line block (filename+badge, path, diff stats)
    /// alongside a separately-clickable reject button, which `Disclosure`
    /// has no builder surface for. Only the reject action below is migrated,
    /// to the shared `Button`.
    fn render_plan_row(
        &self,
        e: &PlanEdit,
        c: &ChatColors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let p = Palette::from_theme(self.theme.read(cx));
        let is_dir = e.kind == "create_directory";
        let expanded = self.expanded_plan.contains(&e.id);
        let (added, removed) = plan_diff_stats(&e.original, &e.proposed);
        let id_toggle = e.id.clone();
        let id_reject = e.id.clone();
        let name = std::path::Path::new(&e.path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| e.path.clone());
        div()
            .flex()
            .flex_col()
            .rounded_md()
            .border_1()
            .border_color(c.border)
            .bg(c.card)
            .overflow_hidden()
            .child(
                div()
                    .id(SharedString::from(format!("plan-row-{}", e.id)))
                    .flex()
                    .items_start()
                    .gap_2()
                    .px_2p5()
                    .py_1p5()
                    .when(!is_dir, |d| {
                        d.on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            if !this.expanded_plan.remove(&id_toggle) {
                                this.expanded_plan.insert(id_toggle.clone());
                            }
                            cx.notify();
                        }))
                    })
                    .child(
                        div()
                            .text_color(c.muted)
                            .text_size(px(10.0))
                            .child(if is_dir {
                                " "
                            } else if expanded {
                                "\u{25be}"
                            } else {
                                "\u{25b8}"
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1p5()
                                    .font_family("mono")
                                    .text_size(px(11.0))
                                    .text_color(c.fg)
                                    .child(SharedString::from(name))
                                    .when(e.is_new && !is_dir, |d| {
                                        d.child(
                                            div()
                                                .text_size(px(9.0))
                                                .text_color(c.accent)
                                                .child("new"),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .font_family("mono")
                                    .text_size(px(9.0))
                                    .text_color(c.muted)
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(SharedString::from(e.path.clone())),
                            )
                            .child(if is_dir {
                                div()
                                    .text_size(px(9.0))
                                    .text_color(c.muted)
                                    .child("create directory")
                            } else {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .text_size(px(9.0))
                                    .child(
                                        div()
                                            .text_color(c.accent)
                                            .child(SharedString::from(format!("+{added}"))),
                                    )
                                    .child(
                                        div()
                                            .text_color(c.error)
                                            .child(SharedString::from(format!("-{removed}"))),
                                    )
                                    .child(
                                        div()
                                            .text_color(c.muted)
                                            .child(SharedString::from(e.kind.clone())),
                                    )
                            }),
                    )
                    .child(
                        button(
                            SharedString::from(format!("plan-reject-{}", e.id)),
                            p,
                            ButtonVariant::Ghost,
                            ButtonSize::IconXs,
                        )
                        .child(IconName::X.svg(c.muted).size(px(11.0)))
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _w, cx| {
                                this.store.update(cx, |s, cx| s.plan_reject(&id_reject, cx));
                                cx.notify();
                            },
                        )),
                    ),
            )
            .when(expanded && !is_dir, |d| {
                d.child(
                    div()
                        .border_t_1()
                        .border_color(c.border)
                        .bg(c.code_bg)
                        .px_2p5()
                        .py_2()
                        .child(plan_diff_preview(&e.original, &e.proposed, c)),
                )
            })
            .into_any_element()
    }

    /// The `/`-slash and `@`-file autocomplete popover.
    ///
    /// T20-003 documented exception: this stays an in-flow `div` (a normal
    /// child pushed above the composer input in `render_composer`'s flex
    /// column), NOT `labonair_ui_kit::popover_menu`. `popover_menu` is an
    /// `anchored().snap_to_window()` + `deferred(..)` overlay that needs a
    /// window-space anchor point (the trigger's bounds), which this popover
    /// has never tracked — it has always been positioned by ordinary layout
    /// flow, one flex child above the composer `div`. Wiring up bounds
    /// tracking just to swap the container would touch the exact
    /// focus/keystroke-routing path the task's warning calls out (Enter-to-
    /// complete via `try_complete_from_popup`, the `InputEvent::Change`
    /// subscription in `ensure_composer`) for no behavioural gain. Only the
    /// row rendering below is migrated, to the shared `ListItem`.
    fn render_composer_popup(
        &self,
        c: &ChatColors,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let popup = self.composer_popup.clone()?;
        let rows: Vec<gpui::AnyElement> = match &popup {
            ComposerPopup::Slash { query } => {
                let cmds = filter_slash(query);
                if cmds.is_empty() {
                    return None;
                }
                cmds.into_iter()
                    .map(|cmd| {
                        ListItem::new(
                            SharedString::from(format!("slash-{}", cmd.name)),
                            c.fg,
                            c.muted,
                            c.border,
                        )
                        .child(
                            div()
                                .font_family("mono")
                                .text_color(c.accent)
                                .child(cmd.invocation),
                        )
                        .child(div().text_color(c.muted).child(cmd.label))
                        .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                            this.run_slash(cmd.name, w, cx)
                        }))
                        .extra(|row| row.text_size(px(11.0)))
                        .into_any_element()
                    })
                    .collect()
            }
            ComposerPopup::File { query } => {
                let files = filter_files(query, &self.popup_files, 12);
                if files.is_empty() {
                    return None;
                }
                files
                    .into_iter()
                    .map(|path| {
                        let p = path.clone();
                        ListItem::new(
                            SharedString::from(format!("atfile-{path}")),
                            c.fg,
                            c.muted,
                            c.border,
                        )
                        .icon(IconName::File)
                        .child(SharedString::from(path))
                        .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                            this.insert_file_mention(p.clone(), w, cx)
                        }))
                        .extra(|row| row.text_size(px(11.0)))
                        .into_any_element()
                    })
                    .collect()
            }
        };
        Some(
            div()
                .id("ai-composer-popup")
                .flex()
                .flex_col()
                .max_h(px(200.0))
                .overflow_y_scroll()
                .rounded_sm()
                .border_1()
                .border_color(c.border)
                .bg(c.card)
                .p_1()
                .children(rows)
                .into_any_element(),
        )
    }

    fn render_composer(&self, c: &ChatColors, cx: &mut Context<Self>) -> impl IntoElement {
        // T20-001: the shared token snapshot the ui-kit primitives take.
        // `ChatColors` stays for the chat-specific semantic slots
        // (`user_bubble`, `code_bg`, `link`).
        let p = Palette::from_theme(self.theme.read(cx));
        let store = self.store.read(cx);
        let streaming = store.is_streaming();
        let queued: Vec<String> = store.queued_prompts().to_vec();
        let todos = store.active_todos();
        let needs_conn = store.needs_connection();

        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .w_full()
            .flex_shrink_0()
            .p_2()
            .border_t_1()
            .border_color(c.border)
            .when(!todos.is_empty(), |d| {
                let done = todos
                    .iter()
                    .filter(|t| t.status == TodoStatus::Completed)
                    .count();
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .p_1p5()
                        .rounded_sm()
                        .bg(c.card)
                        .border_1()
                        .border_color(c.border)
                        .child(
                            div()
                                .text_size(px(9.0))
                                .text_color(c.muted)
                                .child(SharedString::from(format!("TODO  {done}/{}", todos.len()))),
                        )
                        .children(todos.iter().map(|t| {
                            let (glyph, col) = match t.status {
                                TodoStatus::Completed => ("\u{2713}", c.muted),
                                TodoStatus::InProgress => ("\u{25b8}", c.accent),
                                TodoStatus::Pending => ("\u{25cb}", c.muted),
                            };
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .text_size(px(10.0))
                                .text_color(if t.status == TodoStatus::Completed {
                                    c.muted
                                } else {
                                    c.fg
                                })
                                .child(div().w(px(10.0)).text_color(col).child(glyph))
                                .child(SharedString::from(truncate(&t.title, 64)))
                        })),
                )
            })
            .when(needs_conn, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .px_1p5()
                        .py_0p5()
                        .rounded_sm()
                        .bg(c.error.opacity(0.10))
                        .border_1()
                        .border_color(c.error.opacity(0.4))
                        .text_size(px(10.0))
                        .text_color(c.error)
                        .child(IconName::Zap.svg(c.error).size(px(11.0)))
                        .child("No model connected — add a key in Settings → AI"),
                )
            })
            .when(self.store.read(cx).plan_mode(), |d| {
                d.child(self.render_plan_strip(c, cx))
            })
            .when(!self.attachments.is_empty(), |d| {
                d.child(div().flex().flex_wrap().gap_1().children(
                    self.attachments.iter().enumerate().map(|(i, a)| {
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_1p5()
                            .py_0p5()
                            .rounded_sm()
                            .border_1()
                            .border_color(c.border)
                            .bg(c.card)
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .child(a.glyph().svg(c.muted).size(px(12.0)))
                            .child(SharedString::from(truncate(&a.label, 28)))
                            .child(
                                div()
                                    .id(SharedString::from(format!("att-{i}")))
                                    .hover(|s| s.text_color(c.error))
                                    .child(IconName::X.svg(c.muted).size(px(12.0)))
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                        if i < this.attachments.len() {
                                            this.attachments.remove(i);
                                        }
                                        cx.notify();
                                    })),
                            )
                    }),
                ))
            })
            .when(!queued.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .children(queued.iter().enumerate().map(|(i, q)| {
                            // T20-003: `selected_fill` == the row's own
                            // resting background (`c.card`), matching the
                            // panel-snippets convention, so `ListItem`'s
                            // built-in hover tint is a no-op — this row never
                            // had a hover affordance, only the trailing "x".
                            ListItem::new(
                                SharedString::from(format!("queue-row-{i}")),
                                c.muted,
                                c.muted,
                                c.card,
                            )
                            .icon(IconName::CornerDownRight)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(SharedString::from(truncate(q, 60))),
                            )
                            .trailing(
                                div()
                                    .id(SharedString::from(format!("queue-x-{i}")))
                                    .hover(|s| s.text_color(c.error))
                                    .child(IconName::X.svg(c.muted).size(px(11.0)))
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                        this.store.update(cx, |s, cx| s.dequeue_prompt(i, cx));
                                    })),
                            )
                            .extra({
                                let (card, border) = (c.card, c.border);
                                move |row| {
                                    row.cursor_default()
                                        .bg(card)
                                        .border_1()
                                        .border_color(border)
                                        .text_size(px(10.0))
                                }
                            })
                            .into_any_element()
                        })),
                )
            })
            .when_some(self.render_composer_popup(c, cx), |d, p| d.child(p))
            .child(
                div()
                    .id("ai-composer")
                    .min_h(px(52.0))
                    .max_h(px(160.0))
                    .w_full()
                    .rounded_sm()
                    .border_1()
                    .border_color(c.border)
                    .bg(c.bg)
                    .px_2()
                    .py_1p5()
                    .text_xs()
                    .children(
                        self.composer_input
                            .as_ref()
                            .map(|input| field_input(input).appearance(false)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            // T20-001: the shared `ToggleButton` chrome
                            // (`toggle_base`) instead of a bespoke 9px pill —
                            // same pressed/hover contract as the statusbar
                            // panel toggles.
                            .child(
                                toggle_base(
                                    "ai-shell-toggle",
                                    p,
                                    ToggleVariant::Outline,
                                    ToggleSize::Xs,
                                    self.shell_mode,
                                    false,
                                )
                                .px_1p5()
                                .child(
                                    IconName::Terminal
                                        .svg(if self.shell_mode { c.accent } else { c.muted })
                                        .size(px(10.0)),
                                )
                                .child(if self.shell_mode { "Shell" } else { "AI" })
                                .on_click(cx.listener(
                                    |this, _: &ClickEvent, _w, cx| {
                                        this.shell_mode = !this.shell_mode;
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(div().text_size(px(9.0)).text_color(c.muted).child(
                                if self.shell_mode {
                                    "Enter to run in terminal"
                                } else {
                                    "Enter to send \u{00b7} \u{2318}\u{21a9} to queue"
                                },
                            ))
                            .child(
                                // Voice input — inert stub. TODO: needs a mic
                                // capture path + a local whisper transcription
                                // backend (no Rust crate wired yet); until then
                                // this stays visible-but-disabled so the
                                // composer layout matches the reference.
                                div()
                                    .id("ai-voice")
                                    .px_1()
                                    .rounded_sm()
                                    .opacity(labonair_ui_kit::DISABLED_OPACITY)
                                    .text_size(px(9.0))
                                    .text_color(c.muted)
                                    .child("voice (soon)"),
                            ),
                    )
                    // T20-003: shared `Button` primitive.
                    .child(if streaming {
                        button("ai-stop", p, ButtonVariant::Outline, ButtonSize::Xs)
                            .child("Stop")
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.stop(cx)))
                            .into_any_element()
                    } else {
                        let enabled = self.can_send(cx);
                        button("ai-send", p, ButtonVariant::Default, ButtonSize::Xs)
                            .when(!enabled, |d| d.opacity(DISABLED_OPACITY))
                            .child(if self.shell_mode { "Run" } else { "Send" })
                            .when(enabled, |d| {
                                d.on_click(
                                    cx.listener(|this, _: &ClickEvent, w, cx| {
                                        this.send(Some(w), cx)
                                    }),
                                )
                            })
                            .into_any_element()
                    }),
            )
    }
}

/// Events the AI panel raises for the app shell to service.
#[derive(Debug, Clone)]
pub enum AiChatEvent {
    /// Run a shell command in the active terminal — command-snippet "Run"
    /// button, or the AI⇄Shell composer mode.
    RunInTerminal(String),
}

impl gpui::EventEmitter<AiChatEvent> for AiChatView {}

impl Focusable for AiChatView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for AiChatView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_composer(window, cx);
        self.ensure_model_search(window, cx);
        let c = self.colors(cx);
        let header = self.render_header(&c, cx);
        let messages = self.render_messages(&c, cx);
        let composer = self.render_composer(&c, cx);
        let plan_review = self.render_plan_review(&c, cx);

        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .min_w(px(CHAT_MIN_W))
            .bg(c.bg)
            .text_color(c.fg)
            .child(header)
            .child(messages)
            .child(composer)
            .children(plan_review)
    }
}

// ── free helpers ──────────────────────────────────────────────────────────

/// `(added, removed)` line counts between two file contents.
fn plan_diff_stats(original: &str, proposed: &str) -> (usize, usize) {
    let d = Diff::compute(original, proposed);
    let mut add = 0;
    let mut del = 0;
    for l in &d.lines {
        match l.tag {
            ChangeTag::Insert => add += 1,
            ChangeTag::Delete => del += 1,
            ChangeTag::Equal => {}
        }
    }
    (add, del)
}

/// Compact +/- line preview for a plan-review row.
fn plan_diff_preview(original: &str, proposed: &str, c: &ChatColors) -> gpui::AnyElement {
    let d = Diff::compute(original, proposed);
    let changed: Vec<&labonair_editor::diff::DiffLine> = d
        .lines
        .iter()
        .filter(|l| l.tag != ChangeTag::Equal)
        .collect();
    if changed.is_empty() {
        return div()
            .text_size(px(10.0))
            .text_color(c.muted)
            .child("no line-level changes")
            .into_any_element();
    }
    const MAX: usize = 80;
    let rest = changed.len().saturating_sub(MAX);
    div()
        .font_family("mono")
        .text_size(px(10.5))
        .flex()
        .flex_col()
        .children(changed.iter().take(MAX).map(|l| {
            let (sign, col) = match l.tag {
                ChangeTag::Insert => ("+", c.accent),
                ChangeTag::Delete => ("-", c.error),
                ChangeTag::Equal => (" ", c.muted),
            };
            div()
                .flex()
                .gap_1()
                .text_color(col)
                .whitespace_nowrap()
                .child(sign)
                .child(SharedString::from(l.text.clone()))
        }))
        .when(rest > 0, |dv| {
            dv.child(
                div()
                    .text_size(px(9.0))
                    .text_color(c.muted)
                    .child(SharedString::from(format!("\u{2026} {rest} more"))),
            )
        })
        .into_any_element()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}\u{2026}")
    }
}

/// Split a user message into its `<selection>` / `<file>` / `<image>` context
/// chips (label strings) and the remaining prose.
pub fn split_context_blocks(content: &str) -> (Vec<String>, String) {
    let mut chips = Vec::new();
    let mut rest = content.to_string();
    for tag in ["selection", "file", "image"] {
        let open = format!("<{tag}");
        let close = format!("</{tag}>");
        while let Some(start) = rest.find(&open) {
            let Some(rel_end) = rest[start..].find(&close) else {
                break;
            };
            let end = start + rel_end + close.len();
            let block = &rest[start..end];
            let label = block
                .split_once('"')
                .and_then(|(_, r)| r.split_once('"'))
                .map(|(l, _)| l.to_string())
                .unwrap_or_else(|| tag.to_string());
            chips.push(label);
            rest.replace_range(start..end, "");
        }
    }
    (chips, rest)
}

fn inline_text(spans: &[Inline], c: &ChatColors) -> gpui::AnyElement {
    let mut text = String::new();
    let mut highlights: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
    for span in spans {
        let start = text.len();
        text.push_str(span.plain());
        let end = text.len();
        let style = match span {
            Inline::Text(_) => None,
            Inline::Bold(_) => Some(HighlightStyle {
                font_weight: Some(FontWeight::BOLD),
                ..Default::default()
            }),
            Inline::Italic(_) => Some(HighlightStyle {
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            }),
            Inline::Code(_) => Some(HighlightStyle {
                color: Some(c.fg),
                background_color: Some(c.code_bg),
                ..Default::default()
            }),
            Inline::Link { .. } => Some(HighlightStyle {
                color: Some(c.link),
                ..Default::default()
            }),
        };
        if let Some(style) = style {
            highlights.push((start..end, style));
        }
    }
    if highlights.is_empty() {
        div().child(SharedString::from(text)).into_any_element()
    } else {
        StyledText::new(SharedString::from(text))
            .with_highlights(highlights)
            .into_any_element()
    }
}

fn table_row(cells: &[Vec<Inline>], c: &ChatColors, header: bool) -> gpui::AnyElement {
    div()
        .flex()
        .border_b_1()
        .border_color(c.border)
        .children(cells.iter().map(|cell| {
            div()
                .flex_1()
                .min_w_0()
                .px_1p5()
                .py_1()
                .when(header, |d| d.font_weight(FontWeight::SEMIBOLD).bg(c.card))
                .child(inline_text(cell, c))
        }))
        .into_any_element()
}

/// [`Panel`](labonair_panel::Panel) wiring (T17-001).
///
/// The AI chat docks on the **right** at **380 px** — the reference pins the
/// assistant to the right edge, opposite the file tree, and 380 px is the
/// reference chat-column width (message bubbles + the composer). The composer
/// and message list are a vertical stack, so only side docks are valid. Dock
/// move/persistence is T17-002; [`set_position`] is a no-op until then.
impl labonair_panel::Panel for AiChatView {
    fn persistent_name() -> &'static str {
        "ai"
    }

    fn title(&self, _cx: &App) -> SharedString {
        "AI".into()
    }

    fn icon(&self) -> labonair_panel::PanelIcon {
        labonair_panel::PanelIcon::Ai
    }

    fn position(&self, _cx: &App) -> labonair_panel::DockPosition {
        labonair_panel::DockPosition::Right
    }

    fn position_is_valid(&self, position: labonair_panel::DockPosition) -> bool {
        matches!(
            position,
            labonair_panel::DockPosition::Left | labonair_panel::DockPosition::Right
        )
    }

    fn set_position(
        &mut self,
        _position: labonair_panel::DockPosition,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // T17-002 owns the dock model; nothing to persist here yet.
    }

    fn default_size(&self, _cx: &App) -> gpui::Pixels {
        px(380.0)
    }

    fn min_size(&self) -> Option<gpui::Pixels> {
        Some(px(320.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;
    use labonair_ai::MemorySecretStore;

    fn tmp() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("labonair-ui-chat-{}.json", uuid::Uuid::new_v4()))
    }

    fn make(cx: &mut TestAppContext) -> (Entity<AiChatStore>, tokio::runtime::Runtime) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handle = rt.handle().clone();
        let mut store = SessionStore::load(tmp());
        store.set_autosave(false);
        let instances = InstanceStore::load(tmp());
        let entity = cx.update(|cx| {
            cx.new(|_| {
                AiChatStore::from_parts(
                    store,
                    instances,
                    Arc::new(MemorySecretStore::default()),
                    handle,
                )
            })
        });
        (entity, rt)
    }

    #[gpui::test]
    fn session_ops_notify(cx: &mut TestAppContext) {
        let (entity, _rt) = make(cx);
        let notments = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let n2 = notments.clone();
        cx.update(|cx| {
            cx.observe(&entity, move |_, _| {
                n2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            })
            .detach();
        });

        let a = entity.update(cx, |this, cx| this.new_session(cx));
        cx.run_until_parked();
        entity.update(cx, |this, cx| this.rename_session(&a, "hello", cx));
        cx.run_until_parked();
        let b = entity.update(cx, |this, cx| this.new_session(cx));
        cx.run_until_parked();
        entity.update(cx, |this, cx| this.switch_session(&a, cx));
        cx.run_until_parked();
        entity.update(cx, |this, cx| this.delete_session(&b, cx));
        cx.run_until_parked();

        entity.read_with(cx, |this, _| {
            assert_eq!(this.active_id(), Some(a.as_str()));
            assert_eq!(
                this.sessions().iter().find(|s| s.id == a).unwrap().title,
                "hello"
            );
        });
        assert!(notments.load(std::sync::atomic::Ordering::SeqCst) >= 5);
    }

    fn make_view(cx: &mut TestAppContext) -> (Entity<AiChatView>, tokio::runtime::Runtime) {
        let (store, rt) = make(cx);
        let view = cx.update(|cx| {
            let theme = cx.new(|_| ThemeStore::new(gpui::WindowAppearance::Dark));
            cx.new(|cx| AiChatView::new(store, theme, cx))
        });
        (view, rt)
    }

    #[test]
    fn compose_message_embeds_attachments() {
        let atts = vec![
            Attachment {
                kind: AttachmentKind::Selection,
                label: "term".into(),
                content: "line".into(),
            },
            Attachment {
                kind: AttachmentKind::File,
                label: "a.rs".into(),
                content: "fn x() {}".into(),
            },
        ];
        let out = compose_message("explain", &atts);
        assert!(out.contains("<selection source=\"term\">\nline\n</selection>"));
        assert!(out.contains("<file path=\"a.rs\">\nfn x() {}\n</file>"));
        assert!(out.ends_with("explain"));
        // Title derivation strips the injected blocks.
        assert_eq!(labonair_ai::derive_title(&[]), "New chat");
    }

    #[test]
    fn expand_directive_tokens_splices_bodies() {
        use labonair_backend::modules::directives::Directive;
        let dirs = vec![Directive {
            id: "d1".into(),
            handle: "deploy".into(),
            name: "Deploy".into(),
            description: String::new(),
            content: "Run the deploy checklist.".into(),
        }];
        let (body, blocks) = expand_directive_tokens("please #deploy now #unknown", &dirs);
        assert_eq!(body, "please  now #unknown");
        assert_eq!(
            blocks,
            vec![
                "<directive name=\"deploy\">\nRun the deploy checklist.\n</directive>".to_string()
            ]
        );
        // No tokens → passthrough, no blocks.
        let (body, blocks) = expand_directive_tokens("nothing here", &dirs);
        assert_eq!(body, "nothing here");
        assert!(blocks.is_empty());
    }

    #[test]
    fn is_at_bottom_thresholds() {
        assert!(is_at_bottom(0.0, 0.0, 48.0));
        assert!(is_at_bottom(-960.0, 1000.0, 48.0));
        assert!(!is_at_bottom(-200.0, 1000.0, 48.0));
    }

    #[test]
    fn split_context_blocks_extracts_chips() {
        let (chips, rest) =
            split_context_blocks("<selection source=\"term\">\nhi\n</selection>\n\nWhat is this?");
        assert_eq!(chips, vec!["term".to_string()]);
        assert_eq!(rest.trim(), "What is this?");
    }

    #[gpui::test]
    fn composer_clears_on_send_and_attachments_manage(cx: &mut TestAppContext) {
        let (view, _rt) = make_view(cx);
        view.update(cx, |this, cx| {
            this.attach_selection("term", "ls -la", cx);
            this.attach_selection("editor", "let x = 1;", cx);
            assert_eq!(this.attachments.len(), 2);
            this.attachments.remove(0);
            assert_eq!(this.attachments.len(), 1);
            this.composer_seed = "why".into();
            assert!(this.can_send(cx));
            this.send(None, cx);
            assert!(this.composer_text(cx).is_empty());
            assert!(this.attachments.is_empty());
            // No key configured → run failed, and the embedded selection block
            // is part of the stored user message.
            let store = this.store.read(cx);
            assert_eq!(store.run_status(), RunStatus::Error);
            assert!(store.active_messages()[0].content.contains("<selection"));
        });
    }

    #[test]
    fn plan_edit_from_call_resolves_proposed_content() {
        let dir = std::env::temp_dir().join(format!("plan-src-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.txt");
        std::fs::write(&f, "one\ntwo\nthree\n").unwrap();
        let p = f.to_string_lossy().to_string();

        let e = plan_edit_from_call(
            "1",
            "edit",
            &serde_json::json!({ "path": p, "old_string": "two", "new_string": "TWO" }),
        )
        .unwrap();
        assert_eq!(e.proposed, "one\nTWO\nthree\n");
        assert!(!e.is_new);

        let e = plan_edit_from_call(
            "2",
            "write_file",
            &serde_json::json!({ "path": dir.join("new.txt").to_string_lossy(), "content": "x" }),
        )
        .unwrap();
        assert_eq!(e.proposed, "x");
        assert!(e.is_new);

        let e = plan_edit_from_call(
            "3",
            "multi_edit",
            &serde_json::json!({ "path": p, "edits": [
                { "old_string": "one", "new_string": "1" },
                { "old_string": "three", "new_string": "3" },
            ]}),
        )
        .unwrap();
        assert_eq!(e.proposed, "1\ntwo\n3\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn plan_mode_queues_mutating_tool(cx: &mut TestAppContext) {
        let (entity, _rt) = make(cx);
        let tmp = std::env::temp_dir().join(format!("plan-q-{}.txt", uuid::Uuid::new_v4()));
        entity.update(cx, |s, cx| {
            s.set_plan_mode(true, cx);
            s.send("write the file", cx);
            s.store.apply_event(StreamEvent::ToolCallStart {
                id: "w1".into(),
                name: "write_file".into(),
            });
            s.store.apply_event(StreamEvent::ToolCallDelta {
                id: "w1".into(),
                arguments_delta: format!(
                    "{{\"path\":{:?},\"content\":\"hello world\"}}",
                    tmp.to_string_lossy()
                ),
            });
            s.store
                .apply_event(StreamEvent::ToolCallEnd { id: "w1".into() });
            s.store.apply_event(StreamEvent::Done {
                finish_reason: "tool_calls".into(),
            });
            s.test_dispatch_tool_calls(cx);

            assert_eq!(s.plan_queue().len(), 1);
            assert_eq!(s.plan_queue()[0].kind, "write_file");
            assert_eq!(s.plan_queue()[0].proposed, "hello world");
            assert!(!tmp.exists(), "plan mode must not write to disk yet");

            s.plan_apply_all(cx);
            assert!(s.plan_queue().is_empty());
        });
        let mut wrote = None;
        for _ in 0..200 {
            cx.run_until_parked();
            if let Ok(s) = std::fs::read_to_string(&tmp) {
                if !s.is_empty() {
                    wrote = Some(s);
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(wrote.as_deref(), Some("hello world"));
        let _ = std::fs::remove_file(&tmp);
    }

    #[gpui::test]
    fn shell_mode_runs_in_terminal_not_model(cx: &mut TestAppContext) {
        let (view, _rt) = make_view(cx);
        let got: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let g2 = got.clone();
        cx.update(|cx| {
            cx.subscribe(&view, move |_, ev: &AiChatEvent, _| {
                let AiChatEvent::RunInTerminal(c) = ev;
                g2.lock().unwrap().push(c.clone());
            })
            .detach();
        });
        view.update(cx, |this, cx| {
            this.shell_mode = true;
            this.composer_seed = "ls -la".into();
            this.send(None, cx);
            assert!(this.store.read(cx).active_messages().is_empty());
            assert!(this.composer_text(cx).is_empty());
        });
        cx.run_until_parked();
        assert_eq!(got.lock().unwrap().as_slice(), &["ls -la".to_string()]);
    }

    #[gpui::test]
    fn tool_approval_card_resolves(cx: &mut TestAppContext) {
        let (view, _rt) = make_view(cx);
        view.update(cx, |this, cx| {
            this.store.update(cx, |s, cx| {
                s.send("run ls", cx);
                s.store
                    .apply_event(labonair_ai::StreamEvent::ToolCallStart {
                        id: "t1".into(),
                        name: "bash_run".into(),
                    });
                s.store
                    .apply_event(labonair_ai::StreamEvent::ToolCallEnd { id: "t1".into() });
                s.store.apply_event(labonair_ai::StreamEvent::Done {
                    finish_reason: "tool_calls".into(),
                });
            });
        });
        // begin_send with no key already failed the run; drive the card directly.
        view.update(cx, |this, cx| {
            this.store
                .update(cx, |s, cx| s.resolve_tool_call("t1", true, cx));
            let tc = &this.store.read(cx).active_messages()[1].tool_calls[0];
            assert_eq!(tc.status, ToolCallStatus::Done);
        });
    }

    #[gpui::test]
    fn model_picker_sets_ref(cx: &mut TestAppContext) {
        let (view, _rt) = make_view(cx);
        let before = view.read_with(cx, |v, cx| v.store.read(cx).model_ref().to_string());
        let other = MODELS
            .iter()
            .map(|m| m.id)
            .find(|id| !before.starts_with(id))
            .unwrap();
        view.update(cx, |v, cx| {
            v.store.update(cx, |s, cx| s.set_model_ref(other, cx))
        });
        let after = view.read_with(cx, |v, cx| v.store.read(cx).model_ref().to_string());
        assert_ne!(before, after);
    }

    #[gpui::test]
    fn model_picker_filters_by_tab_provider_search(cx: &mut TestAppContext) {
        let (view, _rt) = make_view(cx);
        view.update(cx, |v, _cx| {
            v.model_prefs = ModelPrefs::default();
            assert_eq!(v.visible_models().len(), MODELS.len());

            // Favorites tab: empty until something is starred.
            v.model_tab = ModelTab::Favorites;
            assert!(v.visible_models().is_empty());
            v.model_prefs.toggle_favorite(MODELS[0].id);
            assert_eq!(v.visible_models().len(), 1);

            // Recent tab tracks recency order.
            v.model_prefs.push_recent(MODELS[1].id);
            v.model_prefs.push_recent(MODELS[2].id);
            v.model_tab = ModelTab::Recent;
            assert_eq!(v.visible_models()[0].id, MODELS[2].id);

            // Provider rail + search narrow the All list.
            v.model_tab = ModelTab::All;
            v.model_provider = Some(MODELS[0].provider);
            assert!(v
                .visible_models()
                .iter()
                .all(|m| m.provider == MODELS[0].provider));
            v.model_provider = None;
            v.model_search = MODELS[0].label.to_string();
            assert!(v.visible_models().iter().any(|m| m.id == MODELS[0].id));
        });
    }

    #[gpui::test]
    fn send_without_key_records_error(cx: &mut TestAppContext) {
        let (entity, _rt) = make(cx);
        entity.update(cx, |this, cx| {
            this.send("hi there", cx);
            let msgs = this.active_messages();
            assert_eq!(msgs.len(), 2);
            assert_eq!(this.run_status(), RunStatus::Error);
            assert!(msgs[1].error.is_some());
            assert!(!this.is_streaming());
        });
    }
}
