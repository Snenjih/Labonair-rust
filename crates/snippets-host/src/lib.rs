//! Snippet execution host contract (R08-003).
//!
//! The Snippets panel lives in `labonair-panel-snippets`. To run a snippet it
//! needs exactly four things from the surrounding application: inject text into
//! the active terminal, open a local terminal running a command, open an SSH
//! terminal running a command, and look up the active SSH session id for a
//! host (silent runs).
//!
//! Before this crate existed the panel held an `Entity<Workspace>` and called
//! Workspace methods directly, which coupled a panel to the whole workspace
//! feature. [`SnippetExecutionHost`] replaces that entity with a narrow set of
//! injected callbacks. The composition root builds the host from the active
//! Workspace; the panel never sees Workspace state.

use std::rc::Rc;

use gpui::{App, Window};

type InjectFn = Rc<dyn Fn(&str, &mut App)>;
type RunLocalFn = Rc<dyn Fn(Option<String>, String, &mut Window, &mut App)>;
type RunSshTerminalFn = Rc<dyn Fn(String, String, &mut Window, &mut App)>;
type SshSessionForHostFn = Rc<dyn Fn(&str, &App) -> Option<String>>;

/// Narrow composition contract between the Snippets panel and its host.
///
/// The host (normally the active Workspace) owns terminal and tab creation and
/// decides how each intent is fulfilled. The panel only expresses intent and
/// keeps snippet state, persistence, and execution-mode semantics itself.
#[derive(Clone)]
pub struct SnippetExecutionHost {
    inject: InjectFn,
    run_local: RunLocalFn,
    run_ssh_terminal: RunSshTerminalFn,
    ssh_session_for_host: SshSessionForHostFn,
}

impl SnippetExecutionHost {
    /// Build a host from its four intent callbacks. The composition root wires
    /// these to the active Workspace entity.
    pub fn new(
        inject: impl Fn(&str, &mut App) + 'static,
        run_local: impl Fn(Option<String>, String, &mut Window, &mut App) + 'static,
        run_ssh_terminal: impl Fn(String, String, &mut Window, &mut App) + 'static,
        ssh_session_for_host: impl Fn(&str, &App) -> Option<String> + 'static,
    ) -> Self {
        Self {
            inject: Rc::new(inject),
            run_local: Rc::new(run_local),
            run_ssh_terminal: Rc::new(run_ssh_terminal),
            ssh_session_for_host: Rc::new(ssh_session_for_host),
        }
    }

    /// A host that does nothing. Useful for headless views and tests that
    /// never exercise a run.
    pub fn disconnected() -> Self {
        Self::new(|_, _| {}, |_, _, _, _| {}, |_, _, _, _| {}, |_, _| None)
    }

    /// Append `command` to the active terminal (inject mode).
    pub fn inject_into_active_terminal(&self, command: &str, cx: &mut App) {
        (self.inject)(command, cx);
    }

    /// Open a local terminal tab (optionally rooted at `cwd`) running `command`.
    pub fn run_snippet_local(
        &self,
        cwd: Option<String>,
        command: String,
        window: &mut Window,
        cx: &mut App,
    ) {
        (self.run_local)(cwd, command, window, cx);
    }

    /// Open an SSH terminal tab to `host_id` running `command`.
    pub fn run_snippet_ssh_terminal(
        &self,
        host_id: String,
        command: String,
        window: &mut Window,
        cx: &mut App,
    ) {
        (self.run_ssh_terminal)(host_id, command, window, cx);
    }

    /// The active SSH session id for `host_id`, if a terminal tab is connected
    /// to it. Drives silent (non-terminal) SSH runs.
    pub fn ssh_session_for_host(&self, host_id: &str, cx: &App) -> Option<String> {
        (self.ssh_session_for_host)(host_id, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[gpui::test]
    fn callbacks_dispatch(cx: &mut gpui::TestAppContext) {
        let seen = Rc::new(RefCell::new(Vec::<String>::new()));
        let host = {
            let seen = seen.clone();
            SnippetExecutionHost::new(
                {
                    let seen = seen.clone();
                    move |c, _| seen.borrow_mut().push(format!("inject:{c}"))
                },
                {
                    let seen = seen.clone();
                    move |_, c, _, _| seen.borrow_mut().push(format!("local:{c}"))
                },
                |_, _, _, _| {},
                |host, _| (host == "h1").then(|| "sess-1".to_string()),
            )
        };

        cx.update(|cx| {
            host.inject_into_active_terminal("echo hi", cx);
            assert_eq!(
                host.ssh_session_for_host("h1", cx).as_deref(),
                Some("sess-1")
            );
            assert_eq!(host.ssh_session_for_host("other", cx), None);
        });
        assert_eq!(seen.borrow().as_slice(), ["inject:echo hi"]);
    }

    #[gpui::test]
    fn disconnected_is_inert(cx: &mut gpui::TestAppContext) {
        let host = SnippetExecutionHost::disconnected();
        cx.update(|cx| {
            host.inject_into_active_terminal("x", cx);
            assert_eq!(host.ssh_session_for_host("h", cx), None);
        });
    }
}
