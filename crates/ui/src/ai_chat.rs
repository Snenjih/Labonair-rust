//! `AiChatStore` — the GPUI entity wrapper around [`labonair_ai::SessionStore`]
//! (T11-002).
//!
//! Owns the persistent chat sessions and drives the send → stream → apply loop:
//! [`SessionStore::begin_send`] produces the history, [`AiClient::stream_chat`]
//! streams the response on a Tokio task, and each [`StreamEvent`] is folded back
//! into the store via [`SessionStore::apply_event`] followed by `cx.notify()`.
//! The chat UI itself lands in T11-003 and renders off this entity.

use std::sync::Arc;

use gpui::{AppContext, Context, Entity};
use labonair_ai::{
    resolve_target, AiClient, ChatConfig, InstanceStore, KeyringSecretStore, RunStatus,
    SecretStore, SessionMessage, SessionMeta, SessionStore, StreamEvent,
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
        }
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
        let history = self.store.begin_send(text);
        cx.notify();

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

        self.run = Some(self.tokio.spawn(async move {
            let mut stream = client.stream_chat(target, ChatConfig::default(), history, Vec::new());
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

    fn cancel_run(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if let Some(handle) = self.run.take() {
            handle.abort();
        }
    }
}

/// Convenience: create the entity.
pub fn init(tokio: TokioHandle, cx: &mut gpui::App) -> Entity<AiChatStore> {
    cx.new(|_| AiChatStore::new(tokio))
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
