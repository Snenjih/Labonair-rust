# T01-001: Cargo Workspace & Projektstruktur erstellen

## Status
✅ Done

## Phase
0 — Projekt-Setup & Grundgerüst

## Abhängigkeiten
Keine

## Ziel
Ein sauberes Cargo-Workspace erstellen, das als Grundgerüst für die gesamte Labonair-rust App dient. Das Projekt muss `cargo run` bestehen und ein leeres GPUI-Fenster auf macOS anzeigen.

## Anweisungen

### 1. Cargo Workspace erstellen

Erstelle eine `Cargo Workspace`-Struktur im Stammverzeichnis von `Labonair-rust`:

```
Labonair-rust/
├── Cargo.toml              ← Workspace Root
├── crates/
│   ├── app/                ← Hauptbinary (GPUI App Entry)
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── ui/                 ← UI-Komponenten & Theme
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── terminal/           ← Terminal-Engine (alacritty_terminal)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── editor/             ← Editor-Integration (TreeSitter)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── backend/            ← Rust-Backend-Logik (SSH, SFTP, Git, etc.)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── ai/                 ← AI-Provider-Integration
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── theme/              ← Theme-System & Design-Tokens
│       ├── Cargo.toml
│       └── src/lib.rs
├── tasks/                  ← Roadmap & Tasks (existiert bereits)
├── reference-src/          ← Eingefrorene Referenz-Kopie des Original-Webapps (existiert bereits, read-only)
└── README.md
```

### 2. Workspace Root Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "crates/app",
    "crates/ui",
    "crates/terminal",
    "crates/editor",
    "crates/backend",
    "crates/ai",
    "crates/theme",
]

[workspace.dependencies]
# GPUI (Apache-2.0, via gpui-component fork)
gpui = { version = "0.2", features = ["gpui-component"] }

# Terminal
alacritty_terminal = "0.24"
alacritty_config_derive = "0.24"
vte = "0.13"

# Async
tokio = { version = "1", features = ["full"] }

# SSH — Versionen 1:1 aus reference-src/src-tauri/Cargo.toml übernehmen
russh = { version = "0.62.2", default-features = false, features = ["ring", "flate2", "rsa"] }
russh-sftp = "2.3.0"

# Git — KEIN git2/libgit2. Das Original shellt zum `git`-CLI aus
# (lokal UND remote-over-SSH via GitExecutor). Parität = gleiche Strategie beibehalten.

# SQLite
rusqlite = { version = "0.40", features = ["bundled"] }

# Keyring
keyring = "3"

# HTTP (für AI-Provider APIs)
reqwest = { version = "0.12", features = ["json", "stream"] }

# JSON
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# PTY
portable-pty = "0.9"

# Fonts
fontdb = "0.17"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Error handling
anyhow = "1"
thiserror = "2"

# Utilities
dirs = "6"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
```

### 3. crates/app Cargo.toml

```toml
[package]
name = "labonair"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "labonair"
path = "src/main.rs"

[dependencies]
gpui = { workspace = true }
labonair-ui = { path = "../ui" }
labonair-terminal = { path = "../terminal" }
labonair-editor = { path = "../editor" }
labonair-backend = { path = "../backend" }
labonair-ai = { path = "../ai" }
labonair-theme = { path = "../theme" }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
```

### 4. crates/app/src/main.rs — GPUI App Entry

```rust
use gpui::{div, prelude::*, Application, WindowOptions, WindowBounds, Bounds, Point, Size, Pixels};

fn main() {
    Application::new().run(|cx: &mut App| {
        // Window erstellen
        let window_bounds = WindowBounds::Fixed(Bounds {
            origin: Point::new(Pixels(100.0), Pixels(100.0)),
            size: Size::new(Pixels(1200.0), Pixels(800.0)),
        });

        cx.open_window(
            WindowOptions {
                window_bounds: Some(window_bounds),
                titlebar: None, // macOS nativer Titlebar
                ..Default::default()
            },
            |cx| {
                // Leeres Fenster mit Hintergrundfarbe
                Ok(div()
                    .size_full()
                    .bg(gpui::rgb(0x1a1b26)) // Nord Night Hintergrund
                    .child(
                        div()
                            .p_4()
                            .text_color(gpui::rgb(0xc0caf5))
                            .child("Labonair-rust — Bereit für Entwicklung"),
                    ))
            },
        )
        .unwrap();
    });
}
```

### 5. Placeholder Crates

Erstelle für jedes Crate eine minimale `lib.rs`:

```rust
// crates/ui/src/lib.rs
pub mod components;
pub mod theme_provider;
```

```rust
// crates/terminal/src/lib.rs
pub mod pty_bridge;
pub mod terminal_element;
pub mod terminal_session;
```

```rust
// crates/editor/src/lib.rs
pub mod editor_element;
pub mod language_resolver;
```

```rust
// crates/backend/src/lib.rs
pub mod ssh;
pub mod sftp;
pub mod git;
pub mod fs;
pub mod pty;
pub mod hosts;
pub mod credentials;
pub mod snippets;
pub mod secrets;
```

```rust
// crates/ai/src/lib.rs
pub mod providers;
pub mod agent;
pub mod tools;
pub mod sessions;
pub mod keyring;
```

```rust
// crates/theme/src/lib.rs
pub mod tokens;
pub mod labonair_theme;
pub mod theme_store;
```

## Akzeptanzkriterien

- [ ] `cargo build` kompiliert ohne Fehler
- [ ] `cargo run` startet ein GPUI-Fenster (1200x800) mit dunklem Hintergrund
- [ ] Alle 7 Crates sind im Workspace registriert
- [ ] `cargo clippy` zeigt keine Warnings
- [ ] `reference-src/` liegt unangetastet im Repo (frozen, read-only — nicht anfassen)

## Notizen

- **gpui Version**: Verwende die offizielle `gpui` crate von crates.io (Apache-2.0). Die `gpui-component` crate von Longbridge wird separat hinzugefügt, wenn UI-Komponenten benötigt werden.
- **License**: gpui ist Apache-2.0, aber ACHTUNG: Die Abhängigkeit `ztracing` ist GPL. Für den Anfang ist das unkritisch (nur Link-Time, kein Code-Aufruf). Rechtliche Prüfung vor Release einplanen.
- **macOS-spezifisch**: GPUI nutzt Metal auf macOS. Für Linux-Vulkan-Unterstützung muss später `gpui_platform` konfiguriert werden.

## Warnungen

- ⚠️ **gpui ist pre-1.0** — API-Brüche zwischen Versionen sind möglich. Pin eine spezifische Version.
- ⚠️ **Dokumentation ist dünn** — Die offizielle Empfehlung ist "lese den Zed-Code". Für GPUI-Internas den Zed-Monorepo unter `crates/gpui/` konsultieren.
- ⚠️ **Kein WebView** — GPUI kann keine Web-Inhalte rendern. Falls jemals Markdown-Vorschau mit Web-CSS benötigt wird, geht das nicht.

## Weiterführende Tasks

- [T01-002: Backend-Logik extrahieren](./T01-002-extract-backend-logic.md)
- [T01-003: Referenz-Kopie verifizieren & Projekt-Doku](./T01-003-reference-symlink.md)
