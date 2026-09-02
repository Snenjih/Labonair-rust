//! Release smoke test (T15-004).
//!
//! Exercises the packaging-critical core paths without a display server, so it
//! runs in CI and as the last step of `scripts/smoke-test.sh` against the same
//! code that goes into the `.app` bundle:
//!
//! * the backend state (SQLite schema init under a fresh data dir),
//! * a real local PTY shell — spawn, write a command, read its output back
//!   (covers the embedded fonts/grammars-are-linked assumption indirectly:
//!   a broken build wouldn't link),
//! * the auto-update manifest decision layer.

use std::time::{Duration, Instant};

use labonair_backend::{UpdateManifest, CURRENT_VERSION};
use labonair_terminal::{SessionOptions, TermDimensions, TerminalColors, TerminalSession};

#[test]
fn backend_state_initializes_in_a_fresh_data_dir() {
    let tmp = std::env::temp_dir().join(format!("labonair-smoke-be-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let backend = labonair_backend::App::new(&tmp).expect("backend init");
    // The SQLite file is created by App::new.
    assert!(tmp.join("labonair.db").exists(), "hosts db not created");
    drop(backend);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn local_terminal_spawns_and_runs_a_command() {
    let colors = TerminalColors::from_theme(&labonair_theme::Theme::dark());
    let mut opts = SessionOptions {
        shell: Some("/bin/sh".to_string()),
        ..SessionOptions::default()
    };
    opts.env.push(("PS1".to_string(), "$ ".to_string()));
    let session = TerminalSession::spawn(colors, TermDimensions::new(80, 24), opts)
        .expect("spawn local shell");
    assert!(session.shell_pid().is_some());

    session
        .write(b"printf 'LABONAIR_SMOKE_OK\\n'\n")
        .expect("write to pty");

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let _ = session.drain_events();
        if session
            .render()
            .expect("render")
            .to_text()
            .contains("LABONAIR_SMOKE_OK")
        {
            break;
        }
        assert!(Instant::now() < deadline, "shell produced no smoke output");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn update_manifest_check_against_the_bundled_version() {
    let manifest = r#"{"version":"9999.0.0","notes":"smoke","platforms":{
        "darwin-aarch64":{"url":"https://example.com/x","signature":"s"},
        "darwin-x86_64":{"url":"https://example.com/x","signature":"s"},
        "linux-x86_64":{"url":"https://example.com/x","signature":"s"},
        "linux-aarch64":{"url":"https://example.com/x","signature":"s"}
    }}"#;
    let m = UpdateManifest::parse(manifest).expect("parse manifest");
    assert!(
        m.available().is_some(),
        "a 9999.0.0 manifest should offer an update over {CURRENT_VERSION}"
    );
    // The currently-shipped version never offers an update to itself.
    let same = format!(r#"{{"version":"{CURRENT_VERSION}","platforms":{{}}}}"#);
    assert!(UpdateManifest::parse(&same).unwrap().available().is_none());
}
