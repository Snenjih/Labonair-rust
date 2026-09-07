//! Persisted terminal scrollback owned by the terminal capability.
//!
//! Local terminal sessions save their plain-text ANSI history as compressed
//! files under the application data directory. Workspace/session orchestration
//! supplies the stable pane id and retention policy; this module owns the file
//! format, size limits, and cleanup rules.

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_UNCOMPRESSED_BYTES: usize = 10 * 1024 * 1024;
const HARD_MAX_UNCOMPRESSED_BYTES: usize = 100 * 1024 * 1024;
const OVERFLOW_NOTICE: &str =
    "\r\n\x1b[0m\x1b[2m[labonair: earlier scrollback was truncated to fit the size limit]\x1b[0m\r\n";

fn scrollback_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("scrollback")
}

fn valid_session_id(session_id: &str) -> bool {
    session_id.len() == 36
        && session_id
            .chars()
            .all(|character| character.is_ascii_hexdigit() || character == '-')
}

fn scrollback_path_in(dir: &Path, session_id: &str) -> Result<PathBuf, String> {
    if !valid_session_id(session_id) {
        return Err("invalid session_id".to_string());
    }
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    Ok(dir.join(format!("{session_id}.ansi.gz")))
}

fn truncate_scrollback(ansi: &str, max_bytes: usize) -> String {
    if ansi.len() <= max_bytes {
        return ansi.to_string();
    }
    if OVERFLOW_NOTICE.len() >= max_bytes {
        return String::new();
    }
    let budget = max_bytes - OVERFLOW_NOTICE.len();
    let mut cut_start = ansi.len() - budget;
    while cut_start < ansi.len() && !ansi.is_char_boundary(cut_start) {
        cut_start += 1;
    }
    let start = ansi[cut_start..]
        .find('\n')
        .map_or(cut_start, |offset| cut_start + offset + 1);
    format!("{OVERFLOW_NOTICE}{}", &ansi[start..])
}

fn size_limit(max_bytes: Option<usize>) -> usize {
    max_bytes
        .unwrap_or(MAX_UNCOMPRESSED_BYTES)
        .min(HARD_MAX_UNCOMPRESSED_BYTES)
}

/// Save one local session's ANSI scrollback using an atomic compressed write.
pub fn save(
    data_dir: &Path,
    session_id: &str,
    ansi: &str,
    max_bytes: Option<usize>,
) -> Result<(), String> {
    save_in(&scrollback_dir(data_dir), session_id, ansi, max_bytes)
}

fn save_in(
    dir: &Path,
    session_id: &str,
    ansi: &str,
    max_bytes: Option<usize>,
) -> Result<(), String> {
    if ansi.trim().is_empty() {
        return Ok(());
    }
    let ansi = truncate_scrollback(ansi, size_limit(max_bytes));
    let path = match scrollback_path_in(dir, session_id) {
        Ok(path) => path,
        Err(_) => return Ok(()),
    };
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder
        .write_all(ansi.as_bytes())
        .map_err(|error| error.to_string())?;
    let compressed = encoder.finish().map_err(|error| error.to_string())?;
    let temporary = path.with_extension("ansi.gz.tmp");
    std::fs::write(&temporary, compressed).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, &path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        error.to_string()
    })
}

/// Load one session's persisted scrollback, dropping corrupt or oversized data.
pub fn load(data_dir: &Path, session_id: &str, max_bytes: Option<usize>) -> Option<String> {
    load_in(&scrollback_dir(data_dir), session_id, max_bytes)
}

fn load_in(dir: &Path, session_id: &str, max_bytes: Option<usize>) -> Option<String> {
    let path = scrollback_path_in(dir, session_id).ok()?;
    if !path.exists() {
        return None;
    }
    let compressed = std::fs::read(&path).ok()?;
    let mut decoder = GzDecoder::new(&compressed[..]);
    let mut ansi = String::new();
    match decoder.read_to_string(&mut ansi) {
        Ok(_) if ansi.len() <= size_limit(max_bytes) => Some(ansi),
        _ => {
            let _ = std::fs::remove_file(path);
            None
        }
    }
}

/// Delete one session's persisted scrollback.
pub fn delete(data_dir: &Path, session_id: &str) {
    if let Ok(path) = scrollback_path_in(&scrollback_dir(data_dir), session_id) {
        let _ = std::fs::remove_file(path);
    }
}

/// Remove orphaned files, stale temporary files, and expired known sessions.
pub fn cleanup(data_dir: &Path, known_session_ids: &[String], max_age_secs: Option<u64>) {
    cleanup_in(&scrollback_dir(data_dir), known_session_ids, max_age_secs);
}

fn cleanup_in(dir: &Path, known_session_ids: &[String], max_age_secs: Option<u64>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let known: std::collections::HashSet<&str> =
        known_session_ids.iter().map(String::as_str).collect();
    let max_age = max_age_secs
        .filter(|seconds| *seconds > 0)
        .map(Duration::from_secs);
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".ansi.gz.tmp") {
            let _ = std::fs::remove_file(entry.path());
            continue;
        }
        let Some(stem) = name.strip_suffix(".ansi.gz") else {
            continue;
        };
        if !known.contains(stem) {
            let _ = std::fs::remove_file(entry.path());
            continue;
        }
        if let Some(max_age) = max_age {
            let age = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| now.duration_since(modified).ok());
            if age.is_some_and(|age| age > max_age) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "labonair-terminal-scrollback-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn session_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = temp_dir();
        let id = session_id();
        save_in(&dir, &id, "line one\r\nline two", None).unwrap();
        assert_eq!(
            load_in(&dir, &id, None).as_deref(),
            Some("line one\r\nline two")
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn save_truncates_oldest_content() {
        let dir = temp_dir();
        let id = session_id();
        let content = "old\n".repeat(1024) + "TAIL";
        let limit = OVERFLOW_NOTICE.len() + 16;
        save_in(&dir, &id, &content, Some(limit)).unwrap();
        let loaded = load_in(&dir, &id, Some(limit)).unwrap();
        assert!(loaded.starts_with(OVERFLOW_NOTICE));
        assert!(loaded.contains("TAIL"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn cleanup_removes_orphans_and_temporary_files() {
        let dir = temp_dir();
        let keep = session_id();
        let orphan = session_id();
        save_in(&dir, &keep, "keep", None).unwrap();
        save_in(&dir, &orphan, "drop", None).unwrap();
        std::fs::write(dir.join("stale.ansi.gz.tmp"), b"junk").unwrap();
        cleanup_in(&dir, std::slice::from_ref(&keep), None);
        assert!(load_in(&dir, &keep, None).is_some());
        assert!(load_in(&dir, &orphan, None).is_none());
        assert!(!dir.join("stale.ansi.gz.tmp").exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
