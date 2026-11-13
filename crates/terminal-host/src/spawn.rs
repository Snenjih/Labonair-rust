//! Finding, installing and starting the `labonair-host` daemon, and the
//! process-wide [`shared_client`] (ADR 0009, "OS notes").
//!
//! The app never spawns the daemon through launchd (TCC attribution would
//! move to launchd): it copies the helper binary to a versioned path inside
//! the data dir -- so neither the `.app` nor a mounted DMG stays pinned by a
//! long-lived daemon -- and starts that copy detached (own session, stdio to
//! `/dev/null`; the daemon logs to `<data_dir>/host/host.log` itself) with the
//! app's normal environment.

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::client::HostClient;
use crate::protocol::Hello;
use crate::server::{ensure_private_dir, HostPaths};

const HELPER_NAME: &str = "labonair-host";
/// Overrides the helper location (tests, unusual layouts).
pub const HELPER_ENV: &str = "LABONAIR_HOST_BIN";
const START_TIMEOUT: Duration = Duration::from_secs(4);
const START_POLL: Duration = Duration::from_millis(25);

/// Where the helper may live: the `LABONAIR_HOST_BIN` override, next to the
/// running executable (`target/{debug,release}`, `Contents/MacOS`), or in the
/// bundle's `Contents/Resources`.
fn helper_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(path) = std::env::var_os(HELPER_ENV) {
        out.push(PathBuf::from(path));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        out.push(dir.join(HELPER_NAME));
        out.push(dir.join("../Resources").join(HELPER_NAME));
    }
    out
}

fn locate_helper() -> Result<PathBuf, String> {
    let candidates = helper_candidates();
    candidates
        .iter()
        .find(|path| path.is_file())
        .cloned()
        .ok_or_else(|| {
            format!(
                "the `{HELPER_NAME}` helper was not found (looked in {}). Build it with \
                 `cargo build -p labonair-terminal-host` (dev) or bundle it next to the app \
                 executable.",
                candidates
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Copy `source` to `<host dir>/bin/labonair-host-<content hash>` (skipped when
/// that file already exists with the same size) and drop older copies.
fn install_helper(source: &Path, paths: &HostPaths) -> io::Result<PathBuf> {
    let bytes = fs::read(source)?;
    let bin_dir = paths.dir.join("bin");
    ensure_private_dir(&bin_dir)?;
    let name = format!("{HELPER_NAME}-{:016x}", fnv1a(&bytes));
    let dest = bin_dir.join(&name);
    let up_to_date = fs::metadata(&dest).is_ok_and(|m| m.len() == bytes.len() as u64);
    if !up_to_date {
        let tmp = bin_dir.join(format!(".{name}.tmp"));
        fs::write(&tmp, &bytes)?;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
        fs::rename(&tmp, &dest)?;
    }
    // Older versions: a running daemon keeps its (unlinked) inode.
    if let Ok(entries) = fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            let file = entry.file_name();
            let file = file.to_string_lossy();
            if file.starts_with(HELPER_NAME) && file != name {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(dest)
}

fn daemon_reachable(paths: &HostPaths) -> bool {
    std::os::unix::net::UnixStream::connect(&paths.socket).is_ok()
}

/// Make sure a daemon is listening for `data_dir`, starting one if needed.
pub fn ensure_daemon(data_dir: &Path) -> Result<(), String> {
    let paths = HostPaths::new(data_dir);
    if daemon_reachable(&paths) {
        return Ok(());
    }
    ensure_private_dir(&paths.dir)
        .map_err(|e| format!("cannot prepare {}: {e}", paths.dir.display()))?;
    let source = locate_helper()?;
    let helper = install_helper(&source, &paths)
        .map_err(|e| format!("cannot install the session host helper: {e}"))?;
    let mut command = Command::new(&helper);
    command
        .arg("--data-dir")
        .arg(data_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: `setsid` is async-signal-safe and touches no shared state; it
    // detaches the daemon from the app's session so it survives the app.
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("cannot start {}: {e}", helper.display()))?;
    let started = Instant::now();
    loop {
        if daemon_reachable(&paths) {
            // Reap the child when the daemon eventually exits (no zombie).
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            return Ok(());
        }
        if let Ok(Some(status)) = child.try_wait() {
            // Exit 0 = another instance won the lock race; check once more.
            if status.success() && daemon_reachable(&paths) {
                return Ok(());
            }
            return Err(format!(
                "the session host exited immediately ({status}); see {}",
                paths.log.display()
            ));
        }
        if started.elapsed() > START_TIMEOUT {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            return Err(format!(
                "the session host did not start in time; see {}",
                paths.log.display()
            ));
        }
        std::thread::sleep(START_POLL);
    }
}

static SHARED: Mutex<Option<HostClient>> = Mutex::new(None);

/// The process-wide client, connected. With `start` a missing daemon is
/// started first; without it only an already running daemon is used. Calls are
/// serialized, so concurrent first callers start the daemon once.
pub fn shared_client(
    data_dir: &Path,
    scrollback_lines: u32,
    start: bool,
) -> Result<HostClient, String> {
    let mut slot = SHARED.lock().unwrap_or_else(|p| p.into_inner());
    let client = match slot.as_ref() {
        Some(client) if client.data_dir() == data_dir => client.clone(),
        _ => {
            let hello = Hello::current(env!("CARGO_PKG_VERSION"), Some(scrollback_lines));
            let client = HostClient::new(data_dir.to_path_buf(), hello);
            *slot = Some(client.clone());
            client
        }
    };
    if !client.is_connected() {
        if start {
            ensure_daemon(data_dir)?;
        }
        client
            .reconnect()
            .map_err(|e| format!("cannot reach the background session host: {e}"))?;
    }
    Ok(client)
}
