//! Live-bridge: lazy access to the *currently active* terminal's context
//! (cwd + last N lines of the buffer) for the agent (T11-004).
//!
//! Port of `reference-src/src/modules/ai/lib/useAiLiveBridge.ts`'s `setLive`
//! callbacks. The bridge is queried **on demand** (not snapshotted at message
//! time) because the active tab can change while the agent is thinking.

/// The host-app hooks the agent uses to observe / drive the live UI.
///
/// Every method is called lazily, at the moment a tool or a message actually
/// needs the value — never cached by this crate.
pub trait LiveBridge: Send + Sync {
    /// cwd of the active terminal tab (resolves relative tool paths). `None`
    /// falls back to [`LiveBridge::workspace_root`] then the user's home.
    fn cwd(&self) -> Option<String> {
        None
    }
    /// Explorer / project root, for workspace-wide tools (grep, glob).
    fn workspace_root(&self) -> Option<String> {
        None
    }
    /// Last `max_lines` lines of the active terminal buffer, or `None` when the
    /// active tab is not a terminal.
    fn terminal_context(&self, _max_lines: usize) -> Option<String> {
        None
    }
    /// Session id of the active SSH terminal tab, or `None` for a local tab.
    fn active_ssh_tab_id(&self) -> Option<String> {
        None
    }
    /// Type raw text into the active terminal at the prompt *without* executing.
    /// Returns `false` when there is no active terminal to inject into.
    fn inject_into_active_pty(&self, _text: &str) -> bool {
        false
    }
    /// Send a command to the active shell (executes). Approval-gated by the
    /// caller. Returns `false` when there is no active terminal.
    fn send_to_active_terminal(&self, _command: &str) -> bool {
        false
    }
}

/// A bridge that knows nothing — the default before the app wires a real one.
pub struct NoLiveBridge;
impl LiveBridge for NoLiveBridge {}

/// A fixed bridge for tests / headless use.
#[derive(Default, Clone)]
pub struct StaticLiveBridge {
    pub cwd: Option<String>,
    pub workspace_root: Option<String>,
    pub terminal_buffer: Option<String>,
    pub ssh_tab_id: Option<String>,
}

impl LiveBridge for StaticLiveBridge {
    fn cwd(&self) -> Option<String> {
        self.cwd.clone()
    }
    fn workspace_root(&self) -> Option<String> {
        self.workspace_root.clone()
    }
    fn terminal_context(&self, max_lines: usize) -> Option<String> {
        let buf = self.terminal_buffer.as_deref()?;
        let lines: Vec<&str> = buf.lines().collect();
        let start = lines.len().saturating_sub(max_lines);
        Some(lines[start..].join("\n"))
    }
    fn active_ssh_tab_id(&self) -> Option<String> {
        self.ssh_tab_id.clone()
    }
}

/// Default number of terminal buffer lines fed to the model.
pub const TERMINAL_CONTEXT_LINES: usize = 200;

/// Build the `<terminal-context>` block prepended to a user message, or `None`
/// when there is no live terminal. Lazy: reads the bridge at call time.
pub fn terminal_context_block(bridge: &dyn LiveBridge) -> Option<String> {
    let buf = bridge.terminal_context(TERMINAL_CONTEXT_LINES)?;
    let cwd = bridge.cwd().unwrap_or_default();
    Some(format!(
        "<terminal-context cwd=\"{cwd}\">\n{}\n</terminal-context>",
        buf.trim_end()
    ))
}

/// Resolve a possibly-relative tool path against the live cwd (mirrors the
/// reference `resolvePath`). Absolute paths pass through unchanged.
pub fn resolve_path(raw: &str, cwd: Option<&str>) -> Result<String, String> {
    if raw.starts_with('/') || is_windows_abs(raw) {
        return Ok(raw.to_string());
    }
    let cwd = cwd.ok_or_else(|| {
        format!("cannot resolve relative path \"{raw}\": no active terminal cwd. Pass an absolute path.")
    })?;
    let sep = if cwd.contains('\\') && !cwd.contains('/') {
        '\\'
    } else {
        '/'
    };
    if cwd.ends_with(sep) {
        Ok(format!("{cwd}{raw}"))
    } else {
        Ok(format!("{cwd}{sep}{raw}"))
    }
}

fn is_windows_abs(p: &str) -> bool {
    let b = p.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_bridge_tails_buffer_lazily() {
        let b = StaticLiveBridge {
            cwd: Some("/tmp/work".into()),
            terminal_buffer: Some(
                (1..=10)
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            ..Default::default()
        };
        assert_eq!(b.terminal_context(3), Some("8\n9\n10".to_string()));
        let block = terminal_context_block(&b).unwrap();
        assert!(block.starts_with("<terminal-context cwd=\"/tmp/work\">"));
        assert!(block.contains("10"));
        assert!(block.ends_with("</terminal-context>"));
    }

    #[test]
    fn no_bridge_yields_nothing() {
        assert!(terminal_context_block(&NoLiveBridge).is_none());
        assert_eq!(NoLiveBridge.cwd(), None);
        assert!(!NoLiveBridge.inject_into_active_pty("x"));
    }

    #[test]
    fn resolve_path_rules() {
        assert_eq!(resolve_path("/abs/x", None).unwrap(), "/abs/x");
        assert_eq!(resolve_path("rel/x", Some("/root")).unwrap(), "/root/rel/x");
        assert_eq!(
            resolve_path("rel/x", Some("/root/")).unwrap(),
            "/root/rel/x"
        );
        assert!(resolve_path("rel/x", None).is_err());
        assert_eq!(resolve_path("C:\\a", None).unwrap(), "C:\\a");
    }
}
