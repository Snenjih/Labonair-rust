# T01-004: Event-System & Logging definieren

## Status
⏳ Pending

## Phase
0 — Projekt-Setup & Grundgerüst

## Abhängigkeiten
T01-001 (Cargo Workspace)
T01-002 (Backend-Logik extrahieren)

## Ziel
Ein eigenes Event-System und Logging-Setup definieren, das die Tauri-Event-Emission (Rust → Frontend) ersetzt. Ohne Tauri gibt es keine `AppHandle::emit()` mehr — stattdessen brauchen wir ein kaufmännisches, bekanntes Broadcast-System, das die GPUI-UI-Schicht subscriben kann.

## Anweisungen

### 1. Event-System in crates/backend definieren

Erstelle `crates/backend/src/event.rs`:

```rust
use std::sync::Arc;
use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};

/// Alle Events, die vom Backend an die UI-Schicht gehen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    // Transfer-Events
    TransferProgress {
        transfer_id: String,
        file_name: String,
        transferred: u64,
        total: u64,
        direction: String,
    },
    TransferCompleted {
        transfer_id: String,
        success: bool,
        error: Option<String>,
    },
    FileConflict {
        transfer_id: String,
        file_name: String,
        conflict_type: String,
    },
    
    // SSH-Events
    SshSessionEstablished {
        session_id: String,
        default_path: Option<String>,
    },
    SshAuthRequired {
        session_id: String,
        prompt_message: String,
        is_2fa: bool,
    },
    SshPassphraseRequired {
        session_id: String,
    },
    SshKnownHostsWarning {
        session_id: String,
        fingerprint: String,
        host: String,
        is_mismatch: bool,
    },
    SshConnectionLost {
        session_id: String,
    },
    
    // FS-Events
    DirChanged {
        path: String,
    },
    
    // Menu-Events
    MenuActivated {
        action: String,
    },
    
    // MCP-Events
    McpOpenTabRequest {
        request_id: String,
        path: String,
        host_id: Option<String>,
    },
    McpCloseTabRequest {
        request_id: String,
    },
}

/// Broadcast-Kanal für App-Events.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { tx }
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }
    
    pub fn emit(&self, event: AppEvent) {
        let _ = self.tx.send(event);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
```

### 2. In AppState integrieren

Erweitere `crates/backend/src/state.rs`:

```rust
use crate::event::{AppEvent, EventBus};

pub struct AppState {
    pub db: Arc<RwLock<Connection>>,
    pub events: EventBus,              // NEU
    // ... rest
}
```

### 3. Backend nutzt EventBus statt Tauri emit

Beispiel für FTPS Transfer-Update:

```rust
// HEUTE (Tauri):
window.emit("transfer_progress", payload)?;

// NEU:
state.events.emit(AppEvent::TransferProgress {
    transfer_id: id,
    file_name: name,
    transferred: count,
    total: total_size,
    direction: direction.to_string(),
});
```

### 4. GPUI-UI subscribt auf Events

In `crates/app/src/main.rs` (oder einem UI-Modul):

```rust
// Event-Bus-Subscription in GPUI integrieren
let mut rx = state.events.subscribe();

// Hintergrund-Task für Event-Verarbeitung
cx.spawn(async move {
    while let Ok(event) = rx.recv().await {
        // Event an GPUI übergeben
        // z.B. via App::update, Entity-Notify, etc.
    }
}).detach();
```

### 5. Logging einrichten

In `crates/app/src/main.rs`:

```rust
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    let filter = EnvFilter::from_default_env()
        .add_directive("labonair=debug".parse().unwrap())
        .add_directive("labonair_backend=debug".parse().unwrap());
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(true)          // macOS Terminal: ANSI-Output
        .with_target(true)
        .init();
}
```

## Akzeptanzkriterien

- [ ] `AppEvent` enum existiert mit allen relevanten Varianten
- [ ] `EventBus` mit `subscribe()`/`emit()` funktioniert
- [ ] Mindestens ein Modul nutzt `EventBus` statt Tauri-Emission (z.B. SFTP-Worker)
- [ ] GPUI-Haupt-Crate subscribt auf den Event-Bus
- [ ] `tracing`-Logging ist in `main()` initialisiert
- [ ] `cargo run` zeigt kompilierungserfolgreiche Logging-Output

## Notizen

- **Tauri-Events** die ersetzt werden müssen:
  - `transfer_progress`
  - `file_conflict`
  - `file_error`
  - `session_established`
  - `auth_required`
  - `passphrase_required`
  - `known_hosts_warning`
  - `ssh_connection_lost`
  - `fs:dir-changed`
  - `mcp_*`
  - `menu:*`
- **SSH-PTY-Output** ist seit der russh-Migration **kein** Broadcast-Event mehr — es läuft über per-session `Channel<SshPtyEvent>`. Das bleibt in der neuen App gleich (tokio mpsc pro Session).

## Warnungen

- ⚠️ **Broadcast-Kanal-Drop**: Der `broadcast::channel` droppt Nachrichten wenn Senders zu langsam sind. Für kritische Events (SFTP-Progress) ggf. `tokio::sync::mpsc` mit Backpressure nutzen.
- ⚠️ **Keine serde-Pflicht**: `AppEvent` muss nicht serde-serializierbar sein, da kein IPC mehr existiert. Das Derive ist optional aber nützlich für Debugging/Tests.

## Weiterführende Tasks

- Phase 1: Theme-System
