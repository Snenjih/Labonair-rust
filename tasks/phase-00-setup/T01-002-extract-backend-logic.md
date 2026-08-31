# T01-002: Backend-Logik aus reference-src extrahieren

## Status
⏳ Pending

## Phase
0 — Projekt-Setup & Grundgerüst

## Abhängigkeiten
T01-001 (Cargo Workspace)

## Ziel
Die gesamte Rust-Backend-Logik aus `reference-src/src-tauri/src/modules/` in das neue `crates/backend/` extrahieren und von Tauri-spezifischen Abhängigkeiten befreien.

> **Nicht in dieser Task:** `dock_menu.rs` / `menu_sync.rs` (native Menüs → T04-005) und `errors.rs`
> (geht im neuen `crates/backend/src/error.rs` auf).

## Anweisungen

### 1. Rust-Module kopieren

Kopiere die folgenden Module aus `reference-src/src-tauri/src/modules/` nach `crates/backend/src/`:

| Modul | Quelle | Ziel | Dateien |
|---|---|---|---|
| **ssh** | `src-tauri/src/modules/ssh/` | `crates/backend/src/ssh/` | `mod.rs`, `client.rs`, `pty.rs`, `sftp.rs`, `tunnels.rs`, `config_parser.rs`, `shell.rs`, `shell_integration.rs`, `exec.rs` |
| **sftp** | `src-tauri/src/modules/sftp/` | `crates/backend/src/sftp/` | `mod.rs`, `worker.rs`, `commands.rs`, `connection.rs`, `net_error.rs` |
| **git** | `src-tauri/src/modules/git/` | `crates/backend/src/git/` | `mod.rs`, `executor.rs` |
| **fs** | `src-tauri/src/modules/fs/` | `crates/backend/src/fs/` | `mod.rs`, `tree.rs`, `file.rs`, `mutate.rs`, `grep.rs`, `search.rs`, `watcher.rs`, `paths.rs` |
| **pty** | `src-tauri/src/modules/pty/` | `crates/backend/src/pty/` | `mod.rs`, `session.rs`, `shell_init.rs` |
| **hosts** | `src-tauri/src/modules/hosts/` | `crates/backend/src/hosts/` | `mod.rs`, `db.rs` |
| **credentials** | `src-tauri/src/modules/credentials/` | `crates/backend/src/credentials/` | `mod.rs` |
| **snippets** | `src-tauri/src/modules/snippets/` | `crates/backend/src/snippets/` | `mod.rs`, `db.rs`, `exec.rs` |
| **secrets** | `src-tauri/src/modules/secrets.rs` | `crates/backend/src/secrets.rs` | 1 Datei |
| **shell** | `src-tauri/src/modules/shell/` | `crates/backend/src/shell/` | `mod.rs`, `session.rs`, `background.rs`, `ringbuffer.rs` |
| **themes** | `src-tauri/src/modules/themes/` | `crates/backend/src/themes/` | `mod.rs` |
| **backgrounds** | `src-tauri/src/modules/backgrounds/` | `crates/backend/src/backgrounds/` | `mod.rs` |
| **fonts** | `src-tauri/src/modules/fonts/` | `crates/backend/src/fonts/` | `mod.rs` |
| **scrollback** | `src-tauri/src/modules/scrollback/` | `crates/backend/src/scrollback/` | `mod.rs` |
| **terminal_exec** | `src-tauri/src/modules/terminal_exec/` | `crates/backend/src/terminal_exec/` | `mod.rs` |
| **settings** | `src-tauri/src/modules/settings/` | `crates/backend/src/settings/` | `mod.rs` |
| **mcp** | `src-tauri/src/modules/mcp/` | `crates/backend/src/mcp/` | `mod.rs`, `server.rs`, `osc133.rs` |

### 2. Tauri-Abhängigkeiten entfernen

In allen kopierten Dateien müssen Tauri-spezifische Imports/Typen entfernt/ersetzt werden:

**Zu entfernende Imports:**
```rust
// ENTRERNEN:
use tauri::{AppHandle, State, Manager, command, Window, Emitter};
use tauri::ipc::InvokeBody;

// ERSETZEN durch:
use crate::AppState; // Eigener State-Typ
```

**Zu entfernende Annotationen:**
```rust
// ENTREMNEN:
#[tauri::command]
pub async fn my_command(
    state: State<'_, AppState>,
    param: String,
) -> Result<String, String> {

// ERSETZEN durch:
pub async fn my_command(
    state: &AppState,
    param: String,
) -> Result<String, AppError> {
```

**Fehlerbehandlung anpassen:**
```rust
// HEUTE (Tauri):
.map_err(|e| e.to_string())?;  // Tauri kommandos liefern String-Fehler

// ZIEL:
.map_err(AppError::from)?;  // Eigener Error-Typ
```

### 3. AppState definieren

Erstelle `crates/backend/src/state.rs`:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use rusqlite::Connection;

pub struct AppState {
    pub db: Arc<RwLock<Connection>>,
    pub ssh_sessions: Arc<RwLock<HashMap<String, SshSession>>>,
    pub sftp_worker: Arc<SftpWorker>,
    pub settings: Arc<RwLock<AppSettings>>,
    // ... weitere Zustände aus Tauri's managed state
}

impl AppState {
    pub async fn new(data_dir: &Path) -> Result<Self, AppError> {
        // SQLite initialisieren
        // SSH-State initialisieren
        // SFTP-Worker starten
        // Settings laden
    }
}
```

### 4. Fehler-Typ erstellen

```rust
// crates/backend/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("SSH error: {0}")]
    Ssh(#[from] russh::Error),
    
    #[error("SFTP error: {0}")]
    Sftp(#[from] russh_sftp::Error),
    
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Git error: {0}")]
    Git(String), // git-CLI-Fehler (stderr/exit-code) — KEIN git2/libgit2
    
    #[error("Keyring error: {0}")]
    Keyring(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

// Für GPUI-Kompatibilität:
impl From<AppError> for String {
    fn from(e: AppError) -> String {
        e.to_string()
    }
}
```

### 5. Backend Cargo.toml

```toml
[package]
name = "labonair-backend"
version = "0.1.0"
edition = "2021"

[dependencies]
# SSH
russh = { workspace = true }
russh-sftp = { workspace = true }

# Git — KEIN git2. Git läuft über das `git`-CLI (std::process / tokio::process),
# lokal und remote-over-SSH (GitExecutor). So macht es auch das Original.

# SQLite
rusqlite = { workspace = true }

# Keyring
keyring = { workspace = true }

# HTTP
reqwest = { workspace = true }

# PTY
portable-pty = { workspace = true }

# Fonts
fontdb = { workspace = true }

# Async
tokio = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Errors
anyhow = { workspace = true }
thiserror = { workspace = true }

# Logging
tracing = { workspace = true }

# Utilities
dirs = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }

# Encryption (für Secrets)
aes-gcm = "0.10"
rand = "0.8"
```

### 6. Backend lib.rs aktualisieren

```rust
pub mod error;
pub mod state;
pub mod ssh;
pub mod sftp;
pub mod git;
pub mod fs;
pub mod pty;
pub mod hosts;
pub mod credentials;
pub mod snippets;
pub mod secrets;
pub mod shell;
pub mod themes;
pub mod backgrounds;
pub mod fonts;
pub mod scrollback;
pub mod terminal_exec;
pub mod settings;
pub mod mcp;

pub use error::{AppError, AppResult};
pub use state::AppState;
```

## Akzeptanzkriterien

- [ ] Alle 16 Module sind in `crates/backend/src/` vorhanden
- [ ] Keine `tauri::` Imports mehr in den extrahierten Dateien
- [ ] `cargo check -p labonair-backend` kompiliert (evtl. mit `todo!()` Platzhaltern)
- [ ] `AppState` kann mit einem Datenbankpfad initialisiert werden
- [ ] `AppError` implementiert `From` für alle relevanten Error-Typen

## Notizen

- **Nicht alles muss sofort kompilieren** — Einige Module haben tiefe Tauri-Kopplung (z.B. `mcp/server.rs` nutzt `AppHandle` für Window-Zugriff). Diese können vorerst mit `todo!()` markiert werden.
- **SSH-State** ist heute in Tauri's `manage()` registriert. In der neuen App wird er als `Arc<RwLock<>>` im `AppState` gehalten.
- **SFTP-Worker** nutzt `tokio::sync::mpsc` — das funktioniert auch ohne Tauri.
- **SQLite-Pfad** war heute `app_local_data_dir`. In der neuen App: `dirs::data_local_dir()` + "labonair/".

## Warnungen

- ⚠️ **`#[tauri::command]` Annotationen** — Diese sind Tauri-spezifisch und müssen alle entfernt werden. Es gibt ~150 davon.
- ⚠️ **`State<'_, T>` Typ** — Tauri's Dependency-Injection. Muss durch direkte Referenzen auf `AppState` ersetzt werden.
- ⚠️ **Event-Emission** — Tauri's `window.emit()` muss durch ein eigenes Event-System ersetzt werden (z.B. `tokio::sync::broadcast`).
- ⚠️ **File-Watcher** — Das `notify`-Crate funktioniert auch ohne Tauri, aber der `WatcherState` muss neu initialisiert werden.

## Weiterführende Tasks

- [T01-003: Referenz-Kopie verifizieren & Projekt-Doku](./T01-003-reference-symlink.md)
- [T01-004: Event-System definieren](./T01-004-event-system.md)
