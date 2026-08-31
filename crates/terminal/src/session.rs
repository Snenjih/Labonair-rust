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

use crate::engine::{ModeState, RenderableScreen, TermDimensions, TerminalEmulator, TerminalEvent};
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
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("TERM_PROGRAM", "Labonair");
        cmd.env("LABONAIR_TERMINAL", "1");
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
        let emulator = Arc::new(Mutex::new(TerminalEmulator::new(
            colors,
            dimensions,
            event_tx.clone(),
        )));

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
