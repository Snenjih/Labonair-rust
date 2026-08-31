//! Shell-integration bootstrap (T03-004).
//!
//! Configures a [`CommandBuilder`] so the spawned login shell loads Labonair's
//! integration rc-files. Those scripts emit:
//!
//! * **OSC 7** on every prompt — the current working directory as a
//!   `file://host/path` URI (percent-encoded);
//! * **OSC 133 A/B/C/D** — prompt-start / prompt-end / pre-exec / command-done
//!   (with exit code), so the engine can track command boundaries without
//!   re-parsing the prompt;
//! * **OSC 0** reset at each prompt — clears a title a foreground TUI set, so
//!   the tab falls back to a cwd-derived title.
//!
//! Ported from `reference-src/src-tauri/src/modules/pty/shell_init.rs`, stripped
//! of the Tauri-specific `build_command` wrapper (the [`crate::session`] layer
//! owns shell selection, cwd and extra env). The rc-file contents live as real
//! files under `scripts/` and are inlined with [`include_str!`].

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use portable_pty::CommandBuilder;

pub(crate) const ZSHENV: &str = include_str!("scripts/zshenv.zsh");
pub(crate) const ZPROFILE: &str = include_str!("scripts/zprofile.zsh");
pub(crate) const ZLOGIN: &str = include_str!("scripts/zlogin.zsh");
pub(crate) const ZSHRC: &str = include_str!("scripts/zshrc.zsh");
pub(crate) const BASHRC: &str = include_str!("scripts/bashrc.bash");

/// The shell family we can install integration for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Zsh,
    Bash,
    /// Anything else — spawned without integration.
    Other,
}

impl Shell {
    /// Classify a shell by its executable path (`/bin/zsh` → [`Shell::Zsh`]).
    pub fn from_path(path: &str) -> Shell {
        match path.rsplit('/').next().unwrap_or(path) {
            "zsh" => Shell::Zsh,
            "bash" => Shell::Bash,
            _ => Shell::Other,
        }
    }
}

/// Set the environment, ZDOTDIR / rc-file and login args on `cmd` so `shell`
/// starts with Labonair shell integration active. Returns the detected shell
/// family. Never fails: if the integration directory can't be written the
/// shell still spawns, just without the OSC emitters.
///
/// `blocks` bakes in `LABONAIR_BLOCKS=1` (block-terminal mode — the prompt is
/// replaced by reserved blank rows and the pre-exec marker carries the literal
/// command text). It is fixed for the shell's lifetime.
pub fn configure(cmd: &mut CommandBuilder, shell: &str, blocks: bool) -> Shell {
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "Labonair");
    cmd.env("LABONAIR_TERMINAL", "1");
    if blocks {
        cmd.env("LABONAIR_BLOCKS", "1");
    }

    let kind = Shell::from_path(shell);
    match kind {
        Shell::Zsh => {
            match prepare_zdotdir() {
                Ok(zdotdir) => {
                    if let Ok(user_zd) = std::env::var("ZDOTDIR") {
                        cmd.env("LABONAIR_USER_ZDOTDIR", user_zd);
                    }
                    cmd.env("ZDOTDIR", zdotdir);
                }
                Err(e) => {
                    // Non-fatal: spawn without integration.
                    eprintln!("labonair: zsh shell integration disabled: {e}");
                }
            }
            // Login shell so /etc/zprofile runs (path_helper on macOS).
            cmd.arg("-l");
        }
        Shell::Bash => {
            match prepare_bash_rcfile() {
                Ok(rc) => {
                    cmd.arg("--rcfile");
                    cmd.arg(rc);
                }
                Err(e) => {
                    eprintln!("labonair: bash shell integration disabled: {e}");
                }
            }
            // NOT -l: bash ignores --rcfile for login shells; the rcfile
            // sources /etc/profile itself.
            cmd.arg("-i");
        }
        Shell::Other => {}
    }
    kind
}

fn integration_root() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME not set".to_string())?;
    let root = home
        .join(".cache")
        .join("labonair")
        .join("shell-integration");
    fs::create_dir_all(&root).map_err(|e| format!("create {}: {e}", root.display()))?;
    Ok(root)
}

fn prepare_zdotdir() -> Result<PathBuf, String> {
    let dir = integration_root()?.join("zsh");
    fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    write_if_changed(&dir.join(".zshenv"), ZSHENV)?;
    write_if_changed(&dir.join(".zprofile"), ZPROFILE)?;
    write_if_changed(&dir.join(".zshrc"), ZSHRC)?;
    write_if_changed(&dir.join(".zlogin"), ZLOGIN)?;
    Ok(dir)
}

fn prepare_bash_rcfile() -> Result<PathBuf, String> {
    let dir = integration_root()?.join("bash");
    fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let rc = dir.join("bashrc");
    write_if_changed(&rc, BASHRC)?;
    Ok(rc)
}

/// Atomically write `content` to `path` unless it is already identical (tmp +
/// rename, so a concurrent shell startup never sources a half-written file).
fn write_if_changed(path: &Path, content: &str) -> Result<(), String> {
    if let Ok(existing) = fs::read_to_string(path) {
        if existing == content {
            return Ok(());
        }
    }
    let mut tmp: OsString = path.as_os_str().to_owned();
    tmp.push(".__labonair_tmp__");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, content).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("rename {} -> {}: {e}", tmp.display(), path.display())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_shells_by_path() {
        assert_eq!(Shell::from_path("/bin/zsh"), Shell::Zsh);
        assert_eq!(Shell::from_path("/usr/local/bin/bash"), Shell::Bash);
        assert_eq!(Shell::from_path("/bin/sh"), Shell::Other);
    }

    #[test]
    fn configure_writes_zsh_integration_files() {
        let mut cmd = CommandBuilder::new("/bin/zsh");
        let kind = configure(&mut cmd, "/bin/zsh", false);
        assert_eq!(kind, Shell::Zsh);
        // The rc-files must exist and carry the OSC emitters.
        let dir = integration_root().unwrap().join("zsh");
        let zshrc = fs::read_to_string(dir.join(".zshrc")).unwrap();
        assert!(zshrc.contains("133;A"));
        assert!(zshrc.contains("]7;file://"));
    }

    #[test]
    fn other_shells_get_env_but_no_args() {
        let mut cmd = CommandBuilder::new("/bin/sh");
        assert_eq!(configure(&mut cmd, "/bin/sh", false), Shell::Other);
    }
}
