//! [`ToolHost`] — the filesystem + shell backend the AI tools run against
//! (T11-004). The default [`NativeHost`] delegates to `labonair-backend`'s
//! in-process FS helpers and runs shell commands via `std::process::Command`
//! with a timeout, off any UI thread.

use std::collections::BTreeMap;
use std::io::Read;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Outcome of reading a file.
pub enum FileRead {
    Text { content: String, size: u64 },
    Binary { size: u64 },
    TooLarge { size: u64, limit: u64 },
}

/// One directory entry.
pub struct DirEntry {
    pub name: String,
    /// `"file"`, `"dir"` or `"symlink"`.
    pub kind: String,
}

/// A single content match from [`ToolHost::grep`].
pub struct GrepHit {
    pub path: String,
    pub rel: String,
    pub line: u64,
    pub text: String,
}

/// stdout/stderr/exit of a one-shot shell command.
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

/// The system surface the FS/search/shell tools use. Injectable so tests can
/// run against a scratch directory without a real backend.
pub trait ToolHost: Send + Sync {
    fn read_file(&self, path: &str) -> Result<FileRead, String>;
    fn write_file(&self, path: &str, content: &str) -> Result<(), String>;
    fn create_dir(&self, path: &str) -> Result<(), String>;
    fn list_dir(&self, path: &str) -> Result<Vec<DirEntry>, String>;
    fn grep(
        &self,
        pattern: &str,
        root: &str,
        globs: &[String],
        case_insensitive: bool,
        max_results: usize,
    ) -> Result<(Vec<GrepHit>, bool), String>;
    fn glob(
        &self,
        pattern: &str,
        root: &str,
        max_results: usize,
    ) -> Result<(Vec<String>, bool), String>;
    /// Run `command` in a shell (`sh -c`) with `cwd` and a wall-clock timeout.
    fn run_shell(&self, command: &str, cwd: Option<&str>, timeout: Duration) -> ShellOutput;
}

/// Largest file the read tool will return (matches the reference `AI_READ_CAP`).
pub const AI_READ_CAP: usize = 200 * 1024;

/// Production [`ToolHost`] backed by `labonair-backend`.
#[derive(Default)]
pub struct NativeHost;

impl ToolHost for NativeHost {
    fn read_file(&self, path: &str) -> Result<FileRead, String> {
        use labonair_backend::modules::fs::file::{load_editor_file_sync, EditorLoad};
        match load_editor_file_sync(path, Some(AI_READ_CAP as u64 * 4))? {
            EditorLoad::Text { content, .. } => {
                let size = content.len() as u64;
                Ok(FileRead::Text { content, size })
            }
            EditorLoad::Binary => Ok(FileRead::Binary { size: 0 }),
            EditorLoad::TooLarge { size, limit } => Ok(FileRead::TooLarge { size, limit }),
        }
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), String> {
        labonair_backend::modules::fs::file::save_editor_file_sync(path, content).map(|_| ())
    }

    fn create_dir(&self, path: &str) -> Result<(), String> {
        labonair_backend::modules::fs::mutate::create_dir_sync(path)
    }

    fn list_dir(&self, path: &str) -> Result<Vec<DirEntry>, String> {
        use labonair_backend::modules::fs::tree::{list_dir_entries_sync, EntryKind};
        Ok(list_dir_entries_sync(path, false)?
            .into_iter()
            .map(|e| DirEntry {
                name: e.name,
                kind: match e.kind {
                    EntryKind::File => "file",
                    EntryKind::Dir => "dir",
                    EntryKind::Symlink => "symlink",
                }
                .to_string(),
            })
            .collect())
    }

    fn grep(
        &self,
        pattern: &str,
        root: &str,
        globs: &[String],
        case_insensitive: bool,
        max_results: usize,
    ) -> Result<(Vec<GrepHit>, bool), String> {
        let g = if globs.is_empty() {
            None
        } else {
            Some(globs.to_vec())
        };
        let r = labonair_backend::modules::fs::grep::fs_grep(
            pattern.to_string(),
            root.to_string(),
            g,
            Some(case_insensitive),
            Some(max_results),
        )?;
        Ok((
            r.hits
                .into_iter()
                .map(|h| GrepHit {
                    path: h.path,
                    rel: h.rel,
                    line: h.line,
                    text: h.text,
                })
                .collect(),
            r.truncated,
        ))
    }

    fn glob(
        &self,
        pattern: &str,
        root: &str,
        max_results: usize,
    ) -> Result<(Vec<String>, bool), String> {
        let r = labonair_backend::modules::fs::grep::fs_glob(
            pattern.to_string(),
            root.to_string(),
            Some(max_results),
        )?;
        Ok((r.hits.into_iter().map(|h| h.path).collect(), r.truncated))
    }

    fn run_shell(&self, command: &str, cwd: Option<&str>, timeout: Duration) -> ShellOutput {
        run_shell_blocking(command, cwd, timeout)
    }
}

/// Shared shell runner: `sh -c <command>`, kill on timeout, capture streams.
pub fn run_shell_blocking(command: &str, cwd: Option<&str>, timeout: Duration) -> ShellOutput {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(command);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return ShellOutput {
                stdout: String::new(),
                stderr: format!("failed to spawn shell: {e}"),
                exit_code: -1,
                timed_out: false,
            }
        }
    };

    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break None,
        }
    };

    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut o) = child.stdout.take() {
        let _ = o.read_to_string(&mut stdout);
    }
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut stderr);
    }
    let exit_code = status
        .and_then(|s| s.code())
        .unwrap_or(if timed_out { 124 } else { -1 });
    ShellOutput {
        stdout,
        stderr,
        exit_code,
        timed_out,
    }
}

/// In-memory [`ToolHost`] for tests: a virtual FS rooted at a real temp dir is
/// overkill, so this one just proxies to `std::fs` under a scratch root and
/// runs real `sh -c` (needed by the shell-tool tests). Grep/glob are naive
/// recursive walks — enough for the unit tests.
pub struct ScratchHost {
    /// Records every path passed to `write_file` — handy for assertions.
    pub writes: Mutex<BTreeMap<String, String>>,
}

impl Default for ScratchHost {
    fn default() -> Self {
        Self {
            writes: Mutex::new(BTreeMap::new()),
        }
    }
}

impl ToolHost for ScratchHost {
    fn read_file(&self, path: &str) -> Result<FileRead, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let size = bytes.len() as u64;
        if bytes.iter().take(8000).any(|b| *b == 0) {
            return Ok(FileRead::Binary { size });
        }
        match String::from_utf8(bytes) {
            Ok(content) => Ok(FileRead::Text { content, size }),
            Err(_) => Ok(FileRead::Binary { size }),
        }
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), String> {
        if let Some(p) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(p);
        }
        std::fs::write(path, content).map_err(|e| e.to_string())?;
        self.writes
            .lock()
            .unwrap()
            .insert(path.to_string(), content.to_string());
        Ok(())
    }

    fn create_dir(&self, path: &str) -> Result<(), String> {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())
    }

    fn list_dir(&self, path: &str) -> Result<Vec<DirEntry>, String> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            let e = e.map_err(|e| e.to_string())?;
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let ft = e.file_type().map_err(|e| e.to_string())?;
            out.push(DirEntry {
                name,
                kind: if ft.is_dir() {
                    "dir"
                } else if ft.is_symlink() {
                    "symlink"
                } else {
                    "file"
                }
                .to_string(),
            });
        }
        Ok(out)
    }

    fn grep(
        &self,
        pattern: &str,
        root: &str,
        _globs: &[String],
        case_insensitive: bool,
        max_results: usize,
    ) -> Result<(Vec<GrepHit>, bool), String> {
        let needle = if case_insensitive {
            pattern.to_lowercase()
        } else {
            pattern.to_string()
        };
        let mut hits = Vec::new();
        let mut truncated = false;
        walk(std::path::Path::new(root), &mut |p| {
            if hits.len() >= max_results {
                truncated = true;
                return;
            }
            let Ok(text) = std::fs::read_to_string(p) else {
                return;
            };
            for (i, line) in text.lines().enumerate() {
                let hay = if case_insensitive {
                    line.to_lowercase()
                } else {
                    line.to_string()
                };
                if hay.contains(&needle) {
                    if hits.len() >= max_results {
                        truncated = true;
                        break;
                    }
                    hits.push(GrepHit {
                        path: p.to_string_lossy().to_string(),
                        rel: p
                            .strip_prefix(root)
                            .unwrap_or(p)
                            .to_string_lossy()
                            .to_string(),
                        line: i as u64 + 1,
                        text: line.to_string(),
                    });
                }
            }
        });
        Ok((hits, truncated))
    }

    fn glob(
        &self,
        pattern: &str,
        root: &str,
        max_results: usize,
    ) -> Result<(Vec<String>, bool), String> {
        // Only supports a trailing "*.<ext>" / "**/*.<ext>" suffix match — the
        // unit tests don't need more.
        let ext = pattern.rsplit('.').next().unwrap_or("");
        let mut out = Vec::new();
        let mut truncated = false;
        walk(std::path::Path::new(root), &mut |p| {
            if out.len() >= max_results {
                truncated = true;
                return;
            }
            if p.extension().map(|e| e == ext).unwrap_or(false) {
                out.push(p.to_string_lossy().to_string());
            }
        });
        Ok((out, truncated))
    }

    fn run_shell(&self, command: &str, cwd: Option<&str>, timeout: Duration) -> ShellOutput {
        run_shell_blocking(command, cwd, timeout)
    }
}

fn walk(dir: &std::path::Path, f: &mut dyn FnMut(&std::path::Path)) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if p.is_dir() {
            walk(&p, f);
        } else {
            f(&p);
        }
    }
}
