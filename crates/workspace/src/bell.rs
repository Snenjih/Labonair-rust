//! Audible terminal bell (T06-005).
//!
//! Port of `reference-src/src/modules/terminal/lib/rendererPool.ts::playBell`:
//! when the `terminalBell` preference is on and the emulator parses a `BEL`
//! (`\a`, 0x07), play a short tone. The web app synthesises an 800 Hz / 150 ms
//! sine via `AudioContext`; GPUI has no such primitive, so on macOS we play the
//! standard system alert sound (dependency-free, fully detached process).
//! Linux audio output is a later platform concern (see the task notes).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use labonair_backend::modules::settings::preferences::Preferences;

/// Minimum gap between two audible bells. A pragmatic debounce so a program
/// spamming `BEL` does not machine-gun the speakers.
const MIN_INTERVAL_MS: u64 = 120;

static LAST_RING_MS: AtomicU64 = AtomicU64::new(0);

/// Whether a received `BEL` should produce an audible tone.
pub fn should_ring(prefs: &Preferences) -> bool {
    prefs.terminal_bell
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Play the terminal bell once, if `prefs` enables it and we are past the
/// debounce window from the previous ring.
pub fn ring(prefs: &Preferences) {
    if !should_ring(prefs) {
        return;
    }
    let now = now_ms();
    let last = LAST_RING_MS.load(Ordering::Relaxed);
    if now.saturating_sub(last) < MIN_INTERVAL_MS {
        return;
    }
    LAST_RING_MS.store(now, Ordering::Relaxed);
    play();
}

#[cfg(target_os = "macos")]
fn play() {
    use std::process::{Command, Stdio};
    let _ = Command::new("afplay")
        .arg("/System/Library/Sounds/Tink.aiff")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

#[cfg(not(target_os = "macos"))]
fn play() {
    // Linux audio output: deferred (see task T06-005 notes).
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_follows_preference() {
        let mut p = Preferences::default();
        assert!(!should_ring(&p), "off by default");
        p.terminal_bell = true;
        assert!(should_ring(&p));
    }

    #[test]
    fn ring_is_a_no_op_when_disabled() {
        // Must not panic / spawn anything when the pref is off.
        ring(&Preferences::default());
    }
}
