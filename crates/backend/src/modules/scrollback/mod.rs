//! Terminal scrollback persistence (T14-002).
//!
//! On quit the plain-text scrollback of each restorable local terminal pane is
//! gzip-compressed to `<data_dir>/scrollback/<session-uuid>.ansi.gz`; on the
//! next launch [`crate::modules::scrollback::scrollback_load`] reads it back and
//! the session-restore layer replays it into the freshly spawned shell. Files
//! are keyed by a stable per-pane UUID that the session snapshot carries, so a
//! deleted pane / disabled session-restore leaves an orphan that
//! [`scrollback_cleanup`] removes.
//!
//! All entry points are synchronous small-file IO (a few KB gzip), called from
//! the same startup / shutdown paths as `session.json` — never on a UI hot
//! path.

use crate::modules::fs::paths::data_dir;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::io::{Read, Write};
use std::time::Duration;

const MAX_UNCOMPRESSED_BYTES: usize = 10 * 1024 * 1024; // 10 MB
                                                        // Absolute ceiling applied regardless of the configured `scrollbackMaxSizeMb`
                                                        // setting — protects against a misconfigured huge value writing unbounded
                                                        // scrollback files to disk.
const HARD_MAX_UNCOMPRESSED_BYTES: usize = 100 * 1024 * 1024;

// Visible marker prepended once `truncate_scrollback` has to cut content —
// mirrors the frontend's SCROLLBACK_OVERFLOW_NOTICE (session/scrollback.ts)
// so this defense-in-depth truncation reads the same way as the primary
// frontend-side truncation it backs up.
const OVERFLOW_NOTICE: &str =
    "\r\n\x1b[0m\x1b[2m[labonair: earlier scrollback was truncated to fit the size limit]\x1b[0m\r\n";

/// Truncates `ansi` from the front (oldest content first) once it exceeds
/// `max_bytes`, keeping the most recent output — this is a defense-in-depth
/// backstop for `scrollback_save`; the frontend (session/scrollback.ts)
/// already truncates before calling this command, but a future caller
/// shouldn't be able to write an oversized file by skipping that step. The
/// cut point is advanced to the next line boundary (and snapped to a valid
/// UTF-8 char boundary first) so a multi-byte character or ANSI escape
/// sequence never gets split mid-sequence.
fn truncate_scrollback(ansi: &str, max_bytes: usize) -> String {
    if ansi.len() <= max_bytes {
        return ansi.to_string();
    }
    if OVERFLOW_NOTICE.len() >= max_bytes {
        // Degenerate case: the configured budget is too small even for the
        // notice itself — nothing meaningful can be kept.
        return String::new();
    }
    let budget = max_bytes - OVERFLOW_NOTICE.len();
    let mut cut_start = ansi.len() - budget;
    while cut_start < ansi.len() && !ansi.is_char_boundary(cut_start) {
        cut_start += 1;
    }
    let start = match ansi[cut_start..].find('\n') {
        Some(offset) => cut_start + offset + 1,
        None => cut_start,
    };
    format!("{OVERFLOW_NOTICE}{}", &ansi[start..])
}

fn scrollback_dir() -> std::path::PathBuf {
    data_dir().join("scrollback")
}

fn valid_session_id(session_id: &str) -> bool {
    session_id.len() == 36
        && session_id
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-')
}

fn scrollback_path_in(
    dir: &std::path::Path,
    session_id: &str,
) -> Result<std::path::PathBuf, String> {
    if !valid_session_id(session_id) {
        return Err("invalid session_id".to_string());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    Ok(dir.join(format!("{session_id}.ansi.gz")))
}

fn scrollback_path(session_id: &str) -> Result<std::path::PathBuf, String> {
    scrollback_path_in(&scrollback_dir(), session_id)
}

pub fn scrollback_save(
    session_id: &str,
    ansi: &str,
    max_bytes: Option<usize>,
) -> Result<(), String> {
    save_in(&scrollback_dir(), session_id, ansi, max_bytes)
}

fn save_in(
    dir: &std::path::Path,
    session_id: &str,
    ansi: &str,
    max_bytes: Option<usize>,
) -> Result<(), String> {
    let max_bytes = max_bytes
        .unwrap_or(MAX_UNCOMPRESSED_BYTES)
        .min(HARD_MAX_UNCOMPRESSED_BYTES);
    if ansi.trim().is_empty() {
        return Ok(());
    }
    // Oversized content is truncated from the front (oldest content first),
    // keeping the most recent output plus a visible overflow notice — see
    // `truncate_scrollback`. This is a defense-in-depth backstop: the
    // frontend (session/scrollback.ts) already truncates before calling this
    // command, so in practice `ansi` should already fit `max_bytes` here.
    let ansi = truncate_scrollback(ansi, max_bytes);
    let path = match scrollback_path_in(dir, session_id) {
        Ok(p) => p,
        Err(_) => return Ok(()), // invalid id — silently skip
    };
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder
        .write_all(ansi.as_bytes())
        .map_err(|e| e.to_string())?;
    let compressed = encoder.finish().map_err(|e| e.to_string())?;
    // Atomic write: write to .tmp then rename.
    let tmp_path = path.with_extension("ansi.gz.tmp");
    std::fs::write(&tmp_path, &compressed).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_path, &path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        e.to_string()
    })
}

pub fn scrollback_load(session_id: &str, max_bytes: Option<usize>) -> Option<String> {
    load_in(&scrollback_dir(), session_id, max_bytes)
}

fn load_in(dir: &std::path::Path, session_id: &str, max_bytes: Option<usize>) -> Option<String> {
    let max_bytes = max_bytes
        .unwrap_or(MAX_UNCOMPRESSED_BYTES)
        .min(HARD_MAX_UNCOMPRESSED_BYTES);
    let path = scrollback_path_in(dir, session_id).ok()?;
    if !path.exists() {
        return None;
    }
    let compressed = std::fs::read(&path).ok()?;
    let mut decoder = GzDecoder::new(&compressed[..]);
    let mut ansi = String::new();
    match decoder.read_to_string(&mut ansi) {
        Ok(_) if ansi.len() <= max_bytes => Some(ansi),
        _ => {
            // Corrupt or over-budget — delete so it doesn't linger.
            let _ = std::fs::remove_file(&path);
            None
        }
    }
}

/// Delete the scrollback file for one session (called when its pane/tab is
/// closed). No-op for an unknown id or a missing file.
pub fn scrollback_delete(session_id: &str) {
    if let Ok(path) = scrollback_path(session_id) {
        let _ = std::fs::remove_file(path);
    }
}

pub fn scrollback_cleanup(known_session_ids: &[String], max_age_secs: Option<u64>) {
    cleanup_in(&scrollback_dir(), known_session_ids, max_age_secs);
}

fn cleanup_in(dir: &std::path::Path, known_session_ids: &[String], max_age_secs: Option<u64>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return, // directory doesn't exist yet — nothing to clean
    };
    let known: std::collections::HashSet<&str> =
        known_session_ids.iter().map(|s| s.as_str()).collect();
    let max_age = max_age_secs.filter(|&s| s > 0).map(Duration::from_secs);
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Delete stale temp files unconditionally.
        if name_str.ends_with(".ansi.gz.tmp") {
            let _ = std::fs::remove_file(entry.path());
            continue;
        }
        let Some(stem) = name_str.strip_suffix(".ansi.gz") else {
            continue;
        };
        if !known.contains(stem) {
            let _ = std::fs::remove_file(entry.path());
            continue;
        }
        // Known (active) session, but old enough to fall outside the
        // configured retention window — delete it too.
        if let Some(max_age) = max_age {
            let age = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|m| now.duration_since(m).ok());
            if age.is_some_and(|age| age > max_age) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("labonair-sb-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    #[test]
    fn save_then_load_round_trips_the_scrollback() {
        let dir = tmp_dir();
        let sid = id();
        let content = "line one\r\nline two\r\nlast line";
        save_in(&dir, &sid, content, None).unwrap();
        assert_eq!(load_in(&dir, &sid, None).as_deref(), Some(content));
    }

    #[test]
    fn empty_scrollback_is_not_persisted() {
        let dir = tmp_dir();
        let sid = id();
        save_in(&dir, &sid, "   \r\n  ", None).unwrap();
        assert_eq!(load_in(&dir, &sid, None), None);
    }

    #[test]
    fn save_enforces_the_size_ceiling() {
        let dir = tmp_dir();
        let sid = id();
        let big = "x".repeat(4096) + "\nTAIL_MARKER";
        let cap = OVERFLOW_NOTICE.len() + 64;
        save_in(&dir, &sid, &big, Some(cap)).unwrap();
        let back = load_in(&dir, &sid, Some(cap)).unwrap();
        assert!(back.len() <= cap);
        assert!(back.contains("TAIL_MARKER"));
        assert!(back.starts_with(OVERFLOW_NOTICE));
    }

    #[test]
    fn delete_removes_a_sessions_scrollback() {
        let dir = tmp_dir();
        let sid = id();
        save_in(&dir, &sid, "history", None).unwrap();
        let path = scrollback_path_in(&dir, &sid).unwrap();
        assert!(path.exists());
        // `scrollback_delete` targets the real data dir; exercise the same
        // remove_file path here against the temp copy.
        std::fs::remove_file(&path).unwrap();
        assert_eq!(load_in(&dir, &sid, None), None);
    }

    #[test]
    fn cleanup_removes_orphans_and_temp_files_but_keeps_known_sessions() {
        let dir = tmp_dir();
        let keep = id();
        let orphan = id();
        save_in(&dir, &keep, "keep me", None).unwrap();
        save_in(&dir, &orphan, "drop me", None).unwrap();
        std::fs::write(dir.join("stale.ansi.gz.tmp"), b"junk").unwrap();

        cleanup_in(&dir, std::slice::from_ref(&keep), None);

        assert!(scrollback_path_in(&dir, &keep).unwrap().exists());
        assert!(!scrollback_path_in(&dir, &orphan).unwrap().exists());
        assert!(!dir.join("stale.ansi.gz.tmp").exists());
    }

    #[test]
    fn cleanup_drops_known_sessions_past_the_retention_window() {
        let dir = tmp_dir();
        let fresh = id();
        let stale = id();
        save_in(&dir, &fresh, "recent", None).unwrap();
        save_in(&dir, &stale, "ancient", None).unwrap();
        // Back-date the stale file to two hours ago.
        let old = std::time::SystemTime::now() - Duration::from_secs(7200);
        std::fs::File::options()
            .write(true)
            .open(scrollback_path_in(&dir, &stale).unwrap())
            .unwrap()
            .set_modified(old)
            .unwrap();

        let known = [fresh.clone(), stale.clone()];
        cleanup_in(&dir, &known, Some(3600)); // 1h retention

        assert!(scrollback_path_in(&dir, &fresh).unwrap().exists());
        assert!(!scrollback_path_in(&dir, &stale).unwrap().exists());
    }

    #[test]
    fn truncate_scrollback_returns_input_unchanged_when_within_budget() {
        let ansi = "line one\nline two\n";
        assert_eq!(truncate_scrollback(ansi, 1024), ansi);
    }

    #[test]
    fn truncate_scrollback_keeps_most_recent_content_with_notice() {
        // OVERFLOW_NOTICE itself is 82 bytes, so the input needs to be large
        // enough that a budget which comfortably exceeds the notice can still
        // land short of the whole content — six 20-byte lines (120 bytes)
        // against a 110-byte max_bytes leaves room for exactly the last line.
        let lines: Vec<String> = ('a'..='f')
            .map(|c| c.to_string().repeat(19) + "\n")
            .collect();
        let ansi = lines.concat();
        let max_bytes = OVERFLOW_NOTICE.len() + 28;
        let result = truncate_scrollback(&ansi, max_bytes);
        assert!(
            result.len() <= max_bytes,
            "result must fit max_bytes, got {}",
            result.len()
        );
        assert!(result.starts_with(OVERFLOW_NOTICE));
        // The most recent line should survive the cut.
        assert!(result.ends_with(&lines[5]));
        // The oldest content should be gone.
        assert!(!result.contains(&lines[0]));
    }

    #[test]
    fn truncate_scrollback_resumes_from_next_line_boundary() {
        // Ten 20-byte filler lines followed by a distinct final line — the
        // byte-offset cut point lands mid-way through the last filler line,
        // so the partial line before the next '\n' must be dropped entirely
        // rather than emitting a broken fragment.
        let filler = "X".repeat(19) + "\n";
        let ansi = format!("{}{}", filler.repeat(10), "FINAL-LINE\n");
        let max_bytes = OVERFLOW_NOTICE.len() + 15;
        let result = truncate_scrollback(&ansi, max_bytes);
        assert!(!result.contains('X'));
        assert!(result.ends_with("FINAL-LINE\n"));
    }

    #[test]
    fn truncate_scrollback_handles_budget_smaller_than_notice() {
        let ansi = "some scrollback content that is too long to keep";
        let result = truncate_scrollback(ansi, 5);
        assert_eq!(result, "");
    }

    #[test]
    fn truncate_scrollback_never_splits_a_multibyte_char() {
        // Each "é" is 2 bytes in UTF-8 — a naive byte-index cut could land
        // inside one and panic (or corrupt output).
        let ansi = "é".repeat(50);
        let max_bytes = OVERFLOW_NOTICE.len() + 10;
        let result = truncate_scrollback(&ansi, max_bytes);
        // Must not panic above, and must still be valid UTF-8 (guaranteed by
        // the String type itself) with a sane length.
        assert!(result.len() <= max_bytes + OVERFLOW_NOTICE.len());
    }
}
