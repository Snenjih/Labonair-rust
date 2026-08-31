//! Headless smoke test for the PTY → parser → cells chain (T03-001).
//!
//! Spawns a shell, runs a command, and prints the resulting terminal grid to
//! stdout — no GUI. Run with: `cargo run -p labonair-terminal --example headless_dump`

use std::time::{Duration, Instant};

use labonair_terminal::{SessionOptions, TerminalColors, TerminalSession};
use labonair_terminal::{TermDimensions, TerminalEvent};

fn main() {
    let colors = TerminalColors::from_theme(&labonair_theme::Theme::dark());
    let session = TerminalSession::spawn(
        colors,
        TermDimensions::new(80, 24),
        SessionOptions::default(),
    )
    .expect("spawn shell");

    println!("shell pid: {:?}", session.shell_pid());
    session
        .write(b"printf '\\033[32mGREETING\\033[0m from alacritty_terminal\\n'\n")
        .expect("write");

    // The marker appears twice once the shell runs it: the echoed input line
    // and the program's own output.
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        for ev in session.drain_events() {
            if let TerminalEvent::Title(t) = ev {
                println!("[title] {t}");
            }
        }
        if session
            .render()
            .unwrap()
            .to_text()
            .matches("GREETING")
            .count()
            >= 2
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(30));
    }

    let screen = session.render().unwrap();
    println!(
        "--- terminal grid ({}x{}) ---",
        screen.columns, screen.screen_lines
    );
    println!("{}", screen.to_text());
    println!("--- end ---");
}
