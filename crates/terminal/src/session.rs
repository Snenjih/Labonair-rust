//! Local PTY-backed terminal session (T03-001).
//!
//! Ties a real shell (via [`portable_pty`]) to a [`TerminalEmulator`]. A
//! dedicated OS thread does the blocking PTY reads and feeds the parser; the UI
//! thread only ever locks the emulator to read a [`RenderableScreen`] snapshot
//! or to write user input. Terminal events reach the UI over an
//! [`mpsc`](std::sync::mpsc) channel — the I/O thread never touches UI state.

use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use alacritty_terminal::grid::Scroll;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};

use crate::engine::{
    ModeState, RenderableScreen, SessionMetadata, TermDimensions, TerminalEmulator, TerminalEvent,
};
use crate::TerminalColors;

const READ_BUF: usize = 16 * 1024;

/// Options for spawning a local shell session.
#[derive(Debug, Clone, Default)]
pub struct SessionOptions {
    /// Shell program to run. `None` → the user's `$SHELL` (fallback `/bin/zsh`).
    pub shell: Option<String>,
    /// Extra arguments for the shell.
    pub args: Vec<String>,
    /// Working directory for the shell.
    pub working_directory: Option<String>,
    /// Extra environment variables.
    pub env: Vec<(String, String)>,
    /// Start the shell in block-terminal mode (`LABONAIR_BLOCKS=1`). Fixed for
    /// the shell's lifetime.
    pub blocks: bool,
    /// Optional command written to the shell as input right after spawn
    /// (analogous to Labonair's "startup snippet"). The shell stays interactive
    /// afterwards — this is fed as if the user typed it, not `exec`'d.
    pub startup_command: Option<String>,
    /// Scrollback history depth. `None` → engine default (T13-003).
    pub scrollback: Option<usize>,
    /// Default cursor shape until a program overrides it (T13-003).
    pub cursor_shape: Option<crate::CursorShape>,
    /// Whether the default cursor blinks (T13-003).
    pub cursor_blink: Option<bool>,
}

/// A read of the active terminal for the AI companion: working directory,
/// process title and the tail of the visible buffer (T03-004, consumed in
/// Phase 10).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TerminalContext {
    pub cwd: Option<String>,
    pub title: Option<String>,
    pub lines: Vec<String>,
}

/// A running local terminal session.
pub struct TerminalSession {
    emulator: Arc<Mutex<TerminalEmulator>>,
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    events: Receiver<TerminalEvent>,
    dimensions: TermDimensions,
    shell_pid: Option<u32>,
    reader_thread: Option<JoinHandle<()>>,
}

impl TerminalSession {
    /// Spawn a shell in a new PTY sized to `dimensions`, wired to a
    /// theme-colored emulator.
    pub fn spawn(
        colors: TerminalColors,
        dimensions: TermDimensions,
        options: SessionOptions,
    ) -> Result<Self, String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: dimensions.screen_lines as u16,
                cols: dimensions.columns as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("openpty failed: {e}"))?;

        let shell = options.shell.clone().unwrap_or_else(default_shell);
        let mut cmd = CommandBuilder::new(&shell);
        cmd.args(&options.args);
        // Load Labonair shell integration (OSC 7 + OSC 133 emitters, env).
        crate::shell_integration::configure(&mut cmd, &shell, options.blocks);
        if let Some(dir) = &options.working_directory {
            cmd.cwd(dir);
        }
        for (k, v) in &options.env {
            cmd.env(k, v);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("failed to spawn shell {shell:?}: {e}"))?;
        drop(pair.slave);

        let killer = child.clone_killer();
        let shell_pid = child.process_id();
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("pty reader clone failed: {e}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("pty writer take failed: {e}"))?;
        let writer = Arc::new(Mutex::new(writer));

        let (event_tx, events): (Sender<TerminalEvent>, Receiver<TerminalEvent>) = channel();
        let mut emu_cfg = crate::EmulatorConfig::default();
        if let Some(sb) = options.scrollback {
            emu_cfg.scrollback = sb;
        }
        if let Some(shape) = options.cursor_shape {
            emu_cfg.cursor_shape = shape;
        }
        if let Some(blink) = options.cursor_blink {
            emu_cfg.cursor_blink = blink;
        }
        let emulator = Arc::new(Mutex::new(TerminalEmulator::new_with(
            colors,
            dimensions,
            event_tx.clone(),
            emu_cfg,
        )));
        // Seed the working directory so the UI has one before the first prompt.
        {
            let initial_cwd = options.working_directory.clone().or_else(|| {
                std::env::current_dir()
                    .ok()
                    .map(|p| p.to_string_lossy().into_owned())
            });
            if let (Some(cwd), Ok(mut guard)) = (initial_cwd, emulator.lock()) {
                guard.set_initial_cwd(cwd);
            }
        }

        let reader_thread = {
            let emulator = Arc::clone(&emulator);
            let writer = Arc::clone(&writer);
            let event_tx = event_tx.clone();
            thread::Builder::new()
                .name("labonair-pty-reader".into())
                .spawn(move || {
                    let mut buf = [0u8; READ_BUF];
                    loop {
                        match reader.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                let mut guard = match emulator.lock() {
                                    Ok(g) => g,
                                    Err(_) => break,
                                };
                                let extra = guard.feed(&buf[..n]);
                                let replies = guard.take_pty_output();
                                drop(guard);
                                for ev in extra {
                                    let _ = event_tx.send(ev);
                                }
                                // Service any PtyWrite the emulator queued
                                // (DA/DSR replies) without waiting on the UI.
                                if !replies.is_empty() {
                                    if let Ok(mut w) = writer.lock() {
                                        let _ = w.write_all(&replies);
                                        let _ = w.flush();
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    let _ = event_tx.send(TerminalEvent::Exit);
                })
                .map_err(|e| format!("failed to spawn pty reader thread: {e}"))?
        };

        // Feed the optional startup command as if typed. The PTY buffers it
        // until the shell starts reading, so ordering vs. shell init is safe.
        if let Some(cmd) = &options.startup_command {
            let trimmed = cmd.trim_end_matches(['\r', '\n']);
            if !trimmed.is_empty() {
                if let Ok(mut w) = writer.lock() {
                    let _ = w.write_all(trimmed.as_bytes());
                    let _ = w.write_all(b"\n");
                    let _ = w.flush();
                }
            }
        }

        Ok(Self {
            emulator,
            master: pair.master,
            writer,
            killer,
            events,
            dimensions,
            shell_pid,
            reader_thread: Some(reader_thread),
        })
    }

    /// The shell's process id, if the platform reports one.
    pub fn shell_pid(&self) -> Option<u32> {
        self.shell_pid
    }

    /// True while a foreground job other than the shell itself owns the tty —
    /// i.e. the shell handed its process group off to a running command
    /// (`vim`, `less`, `sleep`, …). Used to decide whether closing a tab
    /// should warn the user first (Labonair's KeepTerminal / "process still
    /// running" behaviour). Unix-only; always `false` elsewhere.
    #[cfg(unix)]
    pub fn has_foreground_job(&self) -> bool {
        let Some(shell_pid) = self.shell_pid else {
            return false;
        };
        matches!(
            self.master.process_group_leader(),
            Some(pid) if pid > 0 && pid as u32 != shell_pid
        )
    }

    #[cfg(not(unix))]
    pub fn has_foreground_job(&self) -> bool {
        false
    }

    /// Politely ask the shell (and its foreground job, which shares the
    /// process group) to exit by sending `SIGHUP` — exactly what a real
    /// terminal emulator does when its window closes. The hard `SIGKILL`
    /// fallback and thread join happen in [`Drop`], so dropping the session
    /// right after this call can never hang.
    pub fn terminate(&self) {
        #[cfg(unix)]
        if let Some(pid) = self.shell_pid {
            // Negative pid → signal the whole process group so a foreground
            // job started by the shell gets the hangup too.
            unsafe {
                libc::kill(-(pid as libc::pid_t), libc::SIGHUP);
                libc::kill(pid as libc::pid_t, libc::SIGHUP);
            }
        }
    }

    /// Current grid size in cells.
    pub fn dimensions(&self) -> TermDimensions {
        self.dimensions
    }

    /// Write user input bytes to the shell.
    pub fn write(&self, bytes: &[u8]) -> Result<(), String> {
        let mut writer = self.writer.lock().map_err(|_| "pty writer poisoned")?;
        writer.write_all(bytes).map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())
    }

    /// Resize both the emulator grid and the underlying PTY.
    pub fn resize(&mut self, dimensions: TermDimensions) -> Result<(), String> {
        self.dimensions = dimensions;
        self.master
            .resize(PtySize {
                rows: dimensions.screen_lines as u16,
                cols: dimensions.columns as u16,
                pixel_width: dimensions
                    .cell_width
                    .saturating_mul(dimensions.columns as u16),
                pixel_height: dimensions
                    .cell_height
                    .saturating_mul(dimensions.screen_lines as u16),
            })
            .map_err(|e| format!("pty resize failed: {e}"))?;
        self.emulator
            .lock()
            .map_err(|_| "emulator poisoned")?
            .resize(dimensions);
        Ok(())
    }

    /// Scroll the viewport within scrollback.
    pub fn scroll(&self, scroll: Scroll) -> Result<(), String> {
        self.emulator
            .lock()
            .map_err(|_| "emulator poisoned")?
            .scroll(scroll);
        Ok(())
    }

    /// Swap the theme palette (light/dark switch).
    pub fn set_colors(&self, colors: TerminalColors) -> Result<(), String> {
        self.emulator
            .lock()
            .map_err(|_| "emulator poisoned")?
            .set_colors(colors);
        Ok(())
    }

    /// Snapshot of the input-relevant terminal modes (cursor/keypad/mouse/paste).
    pub fn mode_state(&self) -> Result<ModeState, String> {
        Ok(self
            .emulator
            .lock()
            .map_err(|_| "emulator poisoned")?
            .mode_state())
    }

    /// Begin or extend a text selection using viewport cell coordinates.
    pub fn update_selection(
        &self,
        anchor: (usize, usize),
        head: (usize, usize),
    ) -> Result<(), String> {
        self.emulator
            .lock()
            .map_err(|_| "emulator poisoned")?
            .update_selection_viewport(anchor, head);
        Ok(())
    }

    /// Clear any active selection.
    pub fn clear_selection(&self) -> Result<(), String> {
        self.emulator
            .lock()
            .map_err(|_| "emulator poisoned")?
            .clear_selection();
        Ok(())
    }

    /// The currently selected text, if any.
    pub fn selection_text(&self) -> Result<Option<String>, String> {
        Ok(self
            .emulator
            .lock()
            .map_err(|_| "emulator poisoned")?
            .selection_text())
    }

    /// Shell-integration metadata (cwd, title, prompt/command state). Updated
    /// as OSC 7 / 133 / 0-2 sequences arrive from the shell.
    pub fn metadata(&self) -> Result<SessionMetadata, String> {
        Ok(self
            .emulator
            .lock()
            .map_err(|_| "emulator poisoned")?
            .metadata()
            .clone())
    }

    /// The shell's current working directory (OSC 7), if known.
    pub fn cwd(&self) -> Option<String> {
        self.metadata().ok().and_then(|m| m.cwd)
    }

    /// A snapshot for the AI live-context reader: the working directory, the
    /// process title, and the last `max_lines` non-empty-trimmed visible rows.
    pub fn ai_context(&self, max_lines: usize) -> Result<TerminalContext, String> {
        let guard = self.emulator.lock().map_err(|_| "emulator poisoned")?;
        let meta = guard.metadata().clone();
        let text = guard.render().to_text();
        drop(guard);
        let all: Vec<String> = text.lines().map(str::to_string).collect();
        let start = all.len().saturating_sub(max_lines);
        Ok(TerminalContext {
            cwd: meta.cwd,
            title: meta.title,
            lines: all[start..].to_vec(),
        })
    }

    /// Take an immutable snapshot of the visible grid for rendering.
    pub fn render(&self) -> Result<RenderableScreen, String> {
        Ok(self
            .emulator
            .lock()
            .map_err(|_| "emulator poisoned")?
            .render())
    }

    /// Run `f` with a lock on the emulator (advanced access).
    pub fn with_emulator<R>(
        &self,
        f: impl FnOnce(&mut TerminalEmulator) -> R,
    ) -> Result<R, String> {
        let mut guard = self.emulator.lock().map_err(|_| "emulator poisoned")?;
        Ok(f(&mut guard))
    }

    /// Non-blocking drain of pending terminal events.
    pub fn drain_events(&self) -> Vec<TerminalEvent> {
        self.events.try_iter().collect()
    }

    /// Block until the next terminal event (used by the headless smoke test).
    pub fn recv_event_timeout(&self, timeout: std::time::Duration) -> Option<TerminalEvent> {
        self.events.recv_timeout(timeout).ok()
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        // Kill the child so the reader hits EOF and the thread unwinds.
        let _ = self.killer.kill();
        if let Some(handle) = self.reader_thread.take() {
            let _ = handle.join();
        }
    }
}

/// The interior-mutable read/selection/scroll surface the GPUI renderer reaches
/// through [`crate::SessionHandle::with`]. Implemented by the local
/// [`TerminalSession`] and the transport-backed [`RemoteSession`] alike so the
/// renderer never needs to know which one backs a tab (T07-001).
pub trait SessionAccess {
    fn render(&self) -> Result<RenderableScreen, String>;
    fn cwd(&self) -> Option<String>;
    fn metadata(&self) -> Result<SessionMetadata, String>;
    fn mode_state(&self) -> Result<ModeState, String>;
    fn selection_text(&self) -> Result<Option<String>, String>;
    fn scroll(&self, scroll: Scroll) -> Result<(), String>;
    fn clear_selection(&self) -> Result<(), String>;
    fn update_selection(&self, anchor: (usize, usize), head: (usize, usize)) -> Result<(), String>;
    fn ai_context(&self, max_lines: usize) -> Result<TerminalContext, String>;
}

impl SessionAccess for TerminalSession {
    fn render(&self) -> Result<RenderableScreen, String> {
        TerminalSession::render(self)
    }
    fn cwd(&self) -> Option<String> {
        TerminalSession::cwd(self)
    }
    fn metadata(&self) -> Result<SessionMetadata, String> {
        TerminalSession::metadata(self)
    }
    fn mode_state(&self) -> Result<ModeState, String> {
        TerminalSession::mode_state(self)
    }
    fn selection_text(&self) -> Result<Option<String>, String> {
        TerminalSession::selection_text(self)
    }
    fn scroll(&self, scroll: Scroll) -> Result<(), String> {
        TerminalSession::scroll(self, scroll)
    }
    fn clear_selection(&self) -> Result<(), String> {
        TerminalSession::clear_selection(self)
    }
    fn update_selection(&self, anchor: (usize, usize), head: (usize, usize)) -> Result<(), String> {
        TerminalSession::update_selection(self, anchor, head)
    }
    fn ai_context(&self, max_lines: usize) -> Result<TerminalContext, String> {
        TerminalSession::ai_context(self, max_lines)
    }
}

/// Sink for user-input bytes produced by a [`RemoteSession`] (the SSH PTY write
/// path). Called on the GPUI thread; implementations forward to the transport.
pub type RemoteWriter = Arc<dyn Fn(Vec<u8>) + Send + Sync>;
/// Sink for `(cols, rows)` resize requests from a [`RemoteSession`].
pub type RemoteResizer = Arc<dyn Fn(u16, u16) + Send + Sync>;

/// A terminal session whose bytes come from an external transport (SSH) rather
/// than a local PTY. The [`TerminalEmulator`] is identical to the local case —
/// only the "wire" differs (T07-001). Output is pushed in via [`RemoteFeed`];
/// user input / resizes go out through the [`RemoteWriter`] / [`RemoteResizer`].
pub struct RemoteSession {
    emulator: Arc<Mutex<TerminalEmulator>>,
    events: Receiver<TerminalEvent>,
    dimensions: TermDimensions,
    writer: RemoteWriter,
    resizer: RemoteResizer,
}

/// Cheap, `Clone`able handle the transport reader uses to push remote output
/// into a [`RemoteSession`]'s emulator and to signal that the connection
/// dropped.
#[derive(Clone)]
pub struct RemoteFeed {
    emulator: Arc<Mutex<TerminalEmulator>>,
    events: Sender<TerminalEvent>,
    writer: RemoteWriter,
}

impl RemoteFeed {
    /// Feed a chunk of remote output through the parser. Any reply bytes the
    /// emulator queues (DA/DSR answers) are sent straight back over the
    /// transport.
    pub fn feed(&self, bytes: &[u8]) {
        let Ok(mut guard) = self.emulator.lock() else {
            return;
        };
        let extra = guard.feed(bytes);
        let replies = guard.take_pty_output();
        drop(guard);
        for ev in extra {
            let _ = self.events.send(ev);
        }
        let _ = self.events.send(TerminalEvent::Wakeup);
        if !replies.is_empty() {
            (self.writer)(replies);
        }
    }

    /// Mark the transport as gone — surfaces as a shell-exit in the UI.
    pub fn mark_disconnected(&self) {
        let _ = self.events.send(TerminalEvent::Exit);
    }
}

impl RemoteSession {
    /// Build a remote session + its [`RemoteFeed`]. No I/O happens here — the
    /// caller wires `feed` to the transport reader.
    pub fn new(
        colors: TerminalColors,
        dimensions: TermDimensions,
        writer: RemoteWriter,
        resizer: RemoteResizer,
    ) -> (Self, RemoteFeed) {
        let (tx, rx): (Sender<TerminalEvent>, Receiver<TerminalEvent>) = channel();
        let emulator = Arc::new(Mutex::new(TerminalEmulator::new(
            colors,
            dimensions,
            tx.clone(),
        )));
        let feed = RemoteFeed {
            emulator: Arc::clone(&emulator),
            events: tx,
            writer: Arc::clone(&writer),
        };
        (
            Self {
                emulator,
                events: rx,
                dimensions,
                writer,
                resizer,
            },
            feed,
        )
    }

    pub fn dimensions(&self) -> TermDimensions {
        self.dimensions
    }

    pub fn write(&self, bytes: &[u8]) -> Result<(), String> {
        (self.writer)(bytes.to_vec());
        Ok(())
    }

    pub fn resize(&mut self, dimensions: TermDimensions) -> Result<(), String> {
        self.dimensions = dimensions;
        (self.resizer)(dimensions.columns as u16, dimensions.screen_lines as u16);
        self.emulator
            .lock()
            .map_err(|_| "emulator poisoned")?
            .resize(dimensions);
        Ok(())
    }

    pub fn set_colors(&self, colors: TerminalColors) -> Result<(), String> {
        self.emulator
            .lock()
            .map_err(|_| "emulator poisoned")?
            .set_colors(colors);
        Ok(())
    }

    pub fn drain_events(&self) -> Vec<TerminalEvent> {
        self.events.try_iter().collect()
    }

    fn with_emulator<R>(&self, f: impl FnOnce(&mut TerminalEmulator) -> R) -> Result<R, String> {
        let mut guard = self.emulator.lock().map_err(|_| "emulator poisoned")?;
        Ok(f(&mut guard))
    }
}

impl SessionAccess for RemoteSession {
    fn render(&self) -> Result<RenderableScreen, String> {
        self.with_emulator(|e| e.render())
    }
    fn cwd(&self) -> Option<String> {
        self.metadata().ok().and_then(|m| m.cwd)
    }
    fn metadata(&self) -> Result<SessionMetadata, String> {
        self.with_emulator(|e| e.metadata().clone())
    }
    fn mode_state(&self) -> Result<ModeState, String> {
        self.with_emulator(|e| e.mode_state())
    }
    fn selection_text(&self) -> Result<Option<String>, String> {
        self.with_emulator(|e| e.selection_text())
    }
    fn scroll(&self, scroll: Scroll) -> Result<(), String> {
        self.with_emulator(|e| e.scroll(scroll))
    }
    fn clear_selection(&self) -> Result<(), String> {
        self.with_emulator(|e| e.clear_selection())
    }
    fn update_selection(&self, anchor: (usize, usize), head: (usize, usize)) -> Result<(), String> {
        self.with_emulator(|e| e.update_selection_viewport(anchor, head))
    }
    fn ai_context(&self, max_lines: usize) -> Result<TerminalContext, String> {
        let guard = self.emulator.lock().map_err(|_| "emulator poisoned")?;
        let meta = guard.metadata().clone();
        let text = guard.render().to_text();
        drop(guard);
        let all: Vec<String> = text.lines().map(str::to_string).collect();
        let start = all.len().saturating_sub(max_lines);
        Ok(TerminalContext {
            cwd: meta.cwd,
            title: meta.title,
            lines: all[start..].to_vec(),
        })
    }
}

/// The user's login shell, falling back to `/bin/zsh` (matches the reference).
fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn dark_colors() -> TerminalColors {
        TerminalColors::from_theme(&labonair_theme::Theme::dark())
    }

    /// Pump events until `pred` sees the rendered screen text, or we time out.
    fn wait_for(
        session: &TerminalSession,
        timeout: Duration,
        pred: impl Fn(&str) -> bool,
    ) -> String {
        let start = Instant::now();
        loop {
            let _ = session.drain_events();
            let text = session.render().unwrap().to_text();
            if pred(&text) {
                return text;
            }
            if start.elapsed() > timeout {
                return text;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn spawns_a_shell_and_runs_a_command() {
        let mut opts = SessionOptions {
            shell: Some("/bin/sh".to_string()),
            ..SessionOptions::default()
        };
        opts.env.push(("PS1".to_string(), "$ ".to_string()));
        let session =
            TerminalSession::spawn(dark_colors(), TermDimensions::new(80, 24), opts).unwrap();
        assert!(session.shell_pid().is_some());

        session.write(b"printf 'PARITYCHECK_OK\\n'\n").unwrap();
        let text = wait_for(&session, Duration::from_secs(5), |t| {
            t.contains("PARITYCHECK_OK")
        });
        assert!(text.contains("PARITYCHECK_OK"), "screen was:\n{text}");
    }

    #[test]
    fn ansi_color_output_from_the_shell_is_parsed() {
        let session = TerminalSession::spawn(
            dark_colors(),
            TermDimensions::new(80, 24),
            SessionOptions {
                shell: Some("/bin/sh".to_string()),
                ..SessionOptions::default()
            },
        )
        .unwrap();
        session
            .write(b"printf '\\033[32mGREEN\\033[0m\\n'\n")
            .unwrap();
        // Two 'G's land on screen: the echoed command line (default fg) and the
        // colored program output. Assert at least one carries the theme green.
        wait_for(&session, Duration::from_secs(5), |t| {
            t.matches("GREEN").count() >= 2
        });
        let expected = dark_colors().normal[2];
        let cells = session.render().unwrap().cells;
        assert!(
            cells.iter().any(|c| c.c == 'G' && c.fg == expected),
            "no green-fg 'G' cell; fgs: {:?}",
            cells
                .iter()
                .filter(|c| c.c == 'G')
                .map(|c| c.fg)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn resize_propagates_to_the_shell() {
        let mut session = TerminalSession::spawn(
            dark_colors(),
            TermDimensions::new(80, 24),
            SessionOptions {
                shell: Some("/bin/sh".to_string()),
                ..SessionOptions::default()
            },
        )
        .unwrap();
        session.resize(TermDimensions::new(120, 40)).unwrap();
        assert_eq!(session.dimensions().columns, 120);
        assert_eq!(session.render().unwrap().columns, 120);

        session.write(b"stty size\n").unwrap();
        let text = wait_for(&session, Duration::from_secs(5), |t| t.contains("40 120"));
        assert!(text.contains("40 120"), "stty size reported:\n{text}");
    }

    #[test]
    fn bash_shell_integration_tracks_cwd() {
        let session = TerminalSession::spawn(
            dark_colors(),
            TermDimensions::new(80, 24),
            SessionOptions {
                shell: Some("/bin/bash".to_string()),
                ..SessionOptions::default()
            },
        )
        .unwrap();

        // Initial cwd is seeded from the process cwd before any prompt.
        assert!(session.cwd().is_some());

        session.write(b"cd /tmp\n").unwrap();
        let start = Instant::now();
        let mut tracked = false;
        while start.elapsed() < Duration::from_secs(10) {
            let _ = session.drain_events();
            if let Some(cwd) = session.cwd() {
                if cwd == "/tmp" || cwd == "/private/tmp" {
                    tracked = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(30));
        }
        assert!(
            tracked,
            "OSC 7 cwd tracking never reported /tmp; cwd = {:?}",
            session.cwd()
        );

        // The AI context reader sees the same cwd plus buffer lines.
        let ctx = session.ai_context(50).unwrap();
        assert!(matches!(ctx.cwd.as_deref(), Some("/tmp" | "/private/tmp")));
    }

    #[test]
    fn child_exit_produces_an_event() {
        let session = TerminalSession::spawn(
            dark_colors(),
            TermDimensions::new(80, 24),
            SessionOptions {
                shell: Some("/bin/sh".to_string()),
                ..SessionOptions::default()
            },
        )
        .unwrap();
        session.write(b"exit 0\n").unwrap();
        let start = Instant::now();
        let mut saw_exit = false;
        while start.elapsed() < Duration::from_secs(5) {
            if session
                .drain_events()
                .iter()
                .any(|e| matches!(e, TerminalEvent::Exit | TerminalEvent::ChildExit(_)))
            {
                saw_exit = true;
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert!(saw_exit, "no exit event received");
    }
}
