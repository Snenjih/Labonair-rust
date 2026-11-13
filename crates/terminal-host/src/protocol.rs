//! Wire protocol between the app and the detached session host (ADR 0009).
//!
//! A connection is a stream of length-prefixed frames, all integers
//! little-endian:
//!
//! * **data frame** (hot path, no serialization):
//!   `kind:u8 | session:u64 | len:u32 | bytes` — [`DataKind::Output`],
//!   [`DataKind::Input`], [`DataKind::Snapshot`], [`DataKind::Resync`].
//! * **control frame**: `0x80 | len:u32 | JSON` — one [`Message`].
//!
//! The protocol is additive: new control messages are new [`Message`]
//! variants; a peer that does not know one decodes it as [`Message::Unknown`]
//! and ignores it. New frame *kinds* require a `proto_version` bump.

use std::io::{self, Read};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt};

/// Protocol version this build speaks.
pub const PROTO_VERSION: u32 = 1;
/// Oldest peer version this build still understands.
pub const MIN_COMPAT: u32 = 1;
/// Largest accepted frame payload. Bigger frames are a protocol error.
pub const MAX_FRAME_LEN: usize = 32 * 1024 * 1024;

/// Wire id of a hosted session (unique within one daemon lifetime).
pub type SessionId = u64;

const CONTROL_TAG: u8 = 0x80;
const DATA_HEADER_LEN: usize = 1 + 8 + 4;
const CONTROL_HEADER_LEN: usize = 1 + 4;

/// Kind of a raw-bytes frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DataKind {
    /// Daemon -> client: live PTY output.
    Output = 0x01,
    /// Client -> daemon: user input for the PTY.
    Input = 0x02,
    /// Daemon -> client: emulator state as escape sequences, sent right after
    /// [`Message::Attached`]. Feed into a fresh emulator of the attach size.
    Snapshot = 0x03,
    /// Daemon -> client: the client fell behind and live output was dropped;
    /// reset the local emulator, feed this snapshot, then continue with live
    /// [`DataKind::Output`].
    Resync = 0x04,
}

impl DataKind {
    fn from_byte(byte: u8) -> Option<Self> {
        Some(match byte {
            0x01 => Self::Output,
            0x02 => Self::Input,
            0x03 => Self::Snapshot,
            0x04 => Self::Resync,
            _ => return None,
        })
    }
}

/// One frame on the wire.
// Control messages are rare; boxing would only make the client API clumsy.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Data {
        kind: DataKind,
        session: SessionId,
        data: Vec<u8>,
    },
    Control(Message),
}

/// Why a frame could not be encoded or decoded.
#[derive(Debug)]
pub enum ProtocolError {
    UnknownFrameType(u8),
    /// A frame announced a payload larger than [`MAX_FRAME_LEN`].
    Oversize(usize),
    /// The control payload was not valid JSON for [`Message`].
    Json(serde_json::Error),
    Io(io::Error),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownFrameType(t) => write!(f, "unknown frame type 0x{t:02x}"),
            Self::Oversize(n) => write!(f, "frame of {n} bytes exceeds {MAX_FRAME_LEN}"),
            Self::Json(e) => write!(f, "invalid control frame: {e}"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<io::Error> for ProtocolError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Append a data frame (`header + bytes`) to `out` without an intermediate
/// copy of `data`.
pub fn encode_data(kind: DataKind, session: SessionId, data: &[u8], out: &mut Vec<u8>) {
    out.reserve(DATA_HEADER_LEN + data.len());
    out.push(kind as u8);
    out.extend_from_slice(&session.to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
}

/// Build a data frame as one buffer.
pub fn data_frame(kind: DataKind, session: SessionId, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(DATA_HEADER_LEN + data.len());
    encode_data(kind, session, data, &mut out);
    out
}

/// Append a control frame to `out`.
pub fn encode_control(message: &Message, out: &mut Vec<u8>) -> Result<(), ProtocolError> {
    let json = serde_json::to_vec(message).map_err(ProtocolError::Json)?;
    if json.len() > MAX_FRAME_LEN {
        return Err(ProtocolError::Oversize(json.len()));
    }
    out.reserve(CONTROL_HEADER_LEN + json.len());
    out.push(CONTROL_TAG);
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&json);
    Ok(())
}

/// Build a control frame as one buffer.
pub fn control_frame(message: &Message) -> Result<Vec<u8>, ProtocolError> {
    let mut out = Vec::new();
    encode_control(message, &mut out)?;
    Ok(out)
}

impl Frame {
    /// Append this frame to `out`.
    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtocolError> {
        match self {
            Frame::Data {
                kind,
                session,
                data,
            } => {
                if data.len() > MAX_FRAME_LEN {
                    return Err(ProtocolError::Oversize(data.len()));
                }
                encode_data(*kind, *session, data, out);
                Ok(())
            }
            Frame::Control(message) => encode_control(message, out),
        }
    }

    /// Decode one frame from the front of `buf`. `Ok(None)` means the buffer
    /// holds only a partial frame (read more); otherwise the frame and the
    /// number of bytes it consumed. An oversize length is rejected from the
    /// header alone, before the payload is buffered.
    pub fn decode(buf: &[u8]) -> Result<Option<(Frame, usize)>, ProtocolError> {
        let Some(&tag) = buf.first() else {
            return Ok(None);
        };
        if tag == CONTROL_TAG {
            if buf.len() < CONTROL_HEADER_LEN {
                return Ok(None);
            }
            let len = u32::from_le_bytes(buf[1..5].try_into().unwrap()) as usize;
            check_len(len)?;
            let total = CONTROL_HEADER_LEN + len;
            if buf.len() < total {
                return Ok(None);
            }
            let message = parse_control(&buf[CONTROL_HEADER_LEN..total])?;
            return Ok(Some((Frame::Control(message), total)));
        }
        let kind = DataKind::from_byte(tag).ok_or(ProtocolError::UnknownFrameType(tag))?;
        if buf.len() < DATA_HEADER_LEN {
            return Ok(None);
        }
        let session = u64::from_le_bytes(buf[1..9].try_into().unwrap());
        let len = u32::from_le_bytes(buf[9..13].try_into().unwrap()) as usize;
        check_len(len)?;
        let total = DATA_HEADER_LEN + len;
        if buf.len() < total {
            return Ok(None);
        }
        let data = buf[DATA_HEADER_LEN..total].to_vec();
        Ok(Some((
            Frame::Data {
                kind,
                session,
                data,
            },
            total,
        )))
    }
}

fn check_len(len: usize) -> Result<(), ProtocolError> {
    if len > MAX_FRAME_LEN {
        Err(ProtocolError::Oversize(len))
    } else {
        Ok(())
    }
}

fn parse_control(json: &[u8]) -> Result<Message, ProtocolError> {
    serde_json::from_slice(json).map_err(ProtocolError::Json)
}

/// Read one frame from an async stream. `Ok(None)` on a clean EOF at a frame
/// boundary.
pub async fn read_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Option<Frame>, ProtocolError> {
    let mut tag = [0u8; 1];
    if reader.read(&mut tag).await? == 0 {
        return Ok(None);
    }
    if tag[0] == CONTROL_TAG {
        let mut len = [0u8; 4];
        reader.read_exact(&mut len).await?;
        let len = u32::from_le_bytes(len) as usize;
        check_len(len)?;
        let mut json = vec![0u8; len];
        reader.read_exact(&mut json).await?;
        return Ok(Some(Frame::Control(parse_control(&json)?)));
    }
    let kind = DataKind::from_byte(tag[0]).ok_or(ProtocolError::UnknownFrameType(tag[0]))?;
    let mut header = [0u8; 12];
    reader.read_exact(&mut header).await?;
    let session = u64::from_le_bytes(header[..8].try_into().unwrap());
    let len = u32::from_le_bytes(header[8..].try_into().unwrap()) as usize;
    check_len(len)?;
    let mut data = vec![0u8; len];
    reader.read_exact(&mut data).await?;
    Ok(Some(Frame::Data {
        kind,
        session,
        data,
    }))
}

/// Blocking twin of [`read_frame`] for non-async clients.
pub fn read_frame_blocking<R: Read>(reader: &mut R) -> Result<Option<Frame>, ProtocolError> {
    let mut tag = [0u8; 1];
    if reader.read(&mut tag)? == 0 {
        return Ok(None);
    }
    if tag[0] == CONTROL_TAG {
        let mut len = [0u8; 4];
        reader.read_exact(&mut len)?;
        let len = u32::from_le_bytes(len) as usize;
        check_len(len)?;
        let mut json = vec![0u8; len];
        reader.read_exact(&mut json)?;
        return Ok(Some(Frame::Control(parse_control(&json)?)));
    }
    let kind = DataKind::from_byte(tag[0]).ok_or(ProtocolError::UnknownFrameType(tag[0]))?;
    let mut header = [0u8; 12];
    reader.read_exact(&mut header)?;
    let session = u64::from_le_bytes(header[..8].try_into().unwrap());
    let len = u32::from_le_bytes(header[8..].try_into().unwrap()) as usize;
    check_len(len)?;
    let mut data = vec![0u8; len];
    reader.read_exact(&mut data)?;
    Ok(Some(Frame::Data {
        kind,
        session,
        data,
    }))
}

// ── Control messages ─────────────────────────────────────────────────────────

/// A control message. Direction is by convention (see each variant); one enum
/// keeps additions (SSH/SFTP/transfer RPC) a matter of adding variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)] // rare control frames, see `Frame`
pub enum Message {
    // client -> daemon
    /// First frame of every connection.
    Hello(Hello),
    /// Spawn a session. Answered by [`Message::Created`] or [`Message::Error`]
    /// carrying the same `req_id`. The creating client is NOT attached yet.
    CreateSession {
        req_id: u64,
        /// Client-chosen id; the daemon allocates one when `None`.
        #[serde(default)]
        id: Option<SessionId>,
        options: CreateOptions,
        /// Palette for the shadow emulator; xterm defaults when `None`.
        #[serde(default)]
        colors: Option<WireColors>,
    },
    /// Attach (stealing it from another client if needed). The daemon resizes
    /// to `cols`x`rows`, then answers [`Message::Attached`], a
    /// [`DataKind::Snapshot`] frame, and continues with live output.
    Attach {
        session: SessionId,
        cols: u16,
        rows: u16,
    },
    /// Stop receiving output; the session keeps running.
    Detach {
        session: SessionId,
    },
    Resize {
        session: SessionId,
        cols: u16,
        rows: u16,
    },
    /// Terminate the session and forget it (closing a tab).
    Kill {
        session: SessionId,
    },
    ListSessions,
    /// Acknowledge an exited session so the daemon forgets it.
    Ack {
        session: SessionId,
    },
    /// Kill every session and stop the daemon ("Quit and stop all sessions").
    Shutdown,

    // daemon -> client
    HelloAck(HelloAck),
    Created {
        req_id: u64,
        session: SessionId,
    },
    Attached {
        session: SessionId,
        cols: u16,
        rows: u16,
    },
    Sessions {
        sessions: Vec<SessionInfo>,
    },
    Event {
        session: SessionId,
        event: SessionEvent,
    },
    /// The child exited; the session stays as `Exited` until [`Message::Ack`].
    SessionExited {
        session: SessionId,
        exit_code: Option<i32>,
    },
    /// The daemon stopped sending output for this session to this client.
    Detached {
        session: SessionId,
        reason: DetachReason,
    },
    Error {
        #[serde(default)]
        req_id: Option<u64>,
        #[serde(default)]
        session: Option<SessionId>,
        message: String,
    },

    // SSH / SFTP / transfers (client -> daemon)
    /// Request/response call into the SSH, SFTP or transfer services. Answered
    /// by [`Message::RpcOk`] or [`Message::RpcErr`] with the same `req_id`.
    /// Calls may be answered out of order (a connect can take minutes while
    /// it waits for a host-key decision).
    Rpc {
        req_id: u64,
        call: RpcCall,
    },

    // SSH / SFTP / transfers (daemon -> client)
    RpcOk {
        req_id: u64,
        value: RpcValue,
    },
    RpcErr {
        req_id: u64,
        error: WireError,
    },
    /// A service event, in the shape of the in-process event bus
    /// (`labonair_events::RawEvent`): SSH connection events
    /// (`ssh_connect_log`, `known_hosts_warning`, `auth_required`,
    /// `passphrase_required`, `session_established`, `ssh_connection_lost`,
    /// `sftp_ready`) and transfer events (`transfer_progress`,
    /// `transfer_step`, `file_conflict`, `file_error`). Only sent to
    /// connections that called [`RpcCall::SubscribeEvents`]. The app side
    /// re-emits them on a local bus, so `SshEventSourceAdapter` and
    /// `TransferEventSourceAdapter` work unchanged.
    BusEvent {
        name: String,
        payload: serde_json::Value,
    },
    /// The SSH connection behind a hosted SSH session dropped (TCP loss,
    /// sleep, server hang-up, or the remote shell ended). `reason` is the
    /// humanized text. The session stays listed as `Exited` until
    /// [`Message::Ack`]; the daemon never reconnects on its own.
    ConnectionLost {
        session: SessionId,
        reason: String,
    },

    /// A message from a newer peer that this build does not know. Ignore it.
    #[serde(other)]
    Unknown,
}

/// Version handshake sent by the client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hello {
    pub proto_version: u32,
    pub min_compat: u32,
    pub build_id: String,
    /// Default daemon-side scrollback (lines) for sessions created without an
    /// explicit `scrollback`; also bounds snapshots.
    #[serde(default)]
    pub scrollback_lines: Option<u32>,
}

impl Hello {
    /// The handshake for this build.
    pub fn current(build_id: impl Into<String>, scrollback_lines: Option<u32>) -> Self {
        Self {
            proto_version: PROTO_VERSION,
            min_compat: MIN_COMPAT,
            build_id: build_id.into(),
            scrollback_lines,
        }
    }
}

/// Daemon answer to [`Hello`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HelloAck {
    pub proto_version: u32,
    pub min_compat: u32,
    pub build_id: String,
    pub pid: u32,
    /// Both sides accept each other's version. When `false` the daemon only
    /// serves `ListSessions`, `Kill` and `Shutdown` so the app can offer
    /// "restart host (N sessions end)".
    pub compatible: bool,
    pub session_count: u32,
}

/// Everything needed to spawn a session. The environment is exactly `env`:
/// the daemon never passes its own on.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CreateOptions {
    pub shell: Option<String>,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Vec<(String, String)>,
    pub startup_command: Option<String>,
    pub cols: u16,
    pub rows: u16,
    /// Opaque tag the app uses to match sessions to panes on reconcile.
    pub pane_tag: Option<String>,
    pub label: Option<String>,
    /// Daemon-side scrollback lines for this session.
    pub scrollback: Option<u32>,
    /// Block-terminal mode (`LABONAIR_BLOCKS=1`).
    pub blocks: bool,
    /// Key (36-char uuid, usually the pane's session id) under which the
    /// daemon persists scrollback on SIGTERM. Derived from the wire id when
    /// `None`.
    pub persist_key: Option<String>,
    /// Previously persisted scrollback (ANSI text) the daemon feeds into the
    /// shadow emulator before the shell starts, so a restored pane keeps its
    /// history above the new prompt.
    pub replay_scrollback: Option<String>,
}

/// Serializable mirror of the engine palette (`TerminalColors`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireColors {
    pub background: [u8; 3],
    pub foreground: [u8; 3],
    pub bright_foreground: [u8; 3],
    pub dim_foreground: [u8; 3],
    pub cursor: [u8; 3],
    pub cursor_text: [u8; 3],
    pub selection: [u8; 3],
    pub selection_alpha: f32,
    pub normal: [[u8; 3]; 8],
    pub bright: [[u8; 3]; 8],
    pub dim: [[u8; 3]; 8],
}

fn rgb(c: [u8; 3]) -> labonair_terminal_core::Rgb {
    labonair_terminal_core::Rgb {
        r: c[0],
        g: c[1],
        b: c[2],
    }
}

fn arr(c: labonair_terminal_core::Rgb) -> [u8; 3] {
    [c.r, c.g, c.b]
}

impl From<&labonair_terminal_core::TerminalColors> for WireColors {
    fn from(c: &labonair_terminal_core::TerminalColors) -> Self {
        Self {
            background: arr(c.background),
            foreground: arr(c.foreground),
            bright_foreground: arr(c.bright_foreground),
            dim_foreground: arr(c.dim_foreground),
            cursor: arr(c.cursor),
            cursor_text: arr(c.cursor_text),
            selection: arr(c.selection),
            selection_alpha: c.selection_alpha,
            normal: c.normal.map(arr),
            bright: c.bright.map(arr),
            dim: c.dim.map(arr),
        }
    }
}

impl From<&WireColors> for labonair_terminal_core::TerminalColors {
    fn from(c: &WireColors) -> Self {
        Self {
            background: rgb(c.background),
            foreground: rgb(c.foreground),
            bright_foreground: rgb(c.bright_foreground),
            dim_foreground: rgb(c.dim_foreground),
            cursor: rgb(c.cursor),
            cursor_text: rgb(c.cursor_text),
            selection: rgb(c.selection),
            selection_alpha: c.selection_alpha,
            normal: c.normal.map(rgb),
            bright: c.bright.map(rgb),
            dim: c.dim.map(rgb),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Running,
    Exited,
}

/// One row of [`Message::Sessions`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub pane_tag: Option<String>,
    pub label: Option<String>,
    pub cwd: Option<String>,
    pub title: Option<String>,
    pub status: SessionStatus,
    pub exit_code: Option<i32>,
    /// A client is currently attached.
    pub attached: bool,
    pub cols: u16,
    pub rows: u16,
    /// Name of the foreground job when it is not the shell (resume hook).
    #[serde(default)]
    pub foreground: Option<String>,
    /// Key under which scrollback is persisted on SIGTERM.
    pub persist_key: String,
}

/// Semantic session events mirrored to the attached client (the client's own
/// emulator derives the same from the byte stream; these are for convenience).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum SessionEvent {
    Title { title: Option<String> },
    Cwd { cwd: String },
    PromptStart,
    PromptEnd,
    CommandStart { command: Option<String> },
    CommandFinished { exit_code: Option<i32> },
    Bell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DetachReason {
    /// Another client attached the session.
    Stolen,
    /// The daemon is shutting down.
    Shutdown,
    /// Reserved: the client could not keep up even after resync.
    Overflow,
}

// ── SSH / SFTP / transfers ───────────────────────────────────────────────────
//
// The daemon runs the concrete SSH stack; these types let a thin app-side
// proxy implement every method of `SshConnectionService`, `SshPtyService`,
// `SshRemoteCommandService`, `SshRemoteFileService`, `SshTunnelService`,
// `SftpSessionService`, `SftpBrowserService` and `TransferService` 1:1.

/// A secret the daemon keeps in RAM only (the app resolved it from its own
/// store). Never serialized to disk, never logged: `Debug` redacts the value.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretEntry {
    /// Secret-store namespace, e.g. `labonair-app` (host password) or
    /// `labonair-cred` (credential secret).
    pub service: String,
    /// Account within the namespace: the host id or credential id.
    pub account: String,
    pub value: String,
}

impl std::fmt::Debug for SecretEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretEntry")
            .field("service", &self.service)
            .field("account", &self.account)
            .field("value", &"<redacted>")
            .finish()
    }
}

/// The columns of a `hosts` row the SSH stack reads.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HostRecord {
    pub id: String,
    pub name: String,
    pub host_address: String,
    pub port: i64,
    pub username: String,
    pub auth_method: String,
    pub private_key_path: Option<String>,
    pub keep_alive_interval: Option<i64>,
    pub keep_alive_tries: Option<i64>,
    pub default_path_ssh: Option<String>,
    pub credential_id: Option<String>,
    pub jump_host_id: Option<String>,
    /// JSON array of tunnel configs, as stored in the `tunnels` column.
    pub tunnels: Option<String>,
}

/// The columns of a `credentials` row the SSH stack reads.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CredentialRecord {
    pub id: String,
    pub name: String,
    pub cred_type: String,
    pub key_path: Option<String>,
    pub has_secret: bool,
}

/// Everything the daemon needs to resolve a host without the app's database
/// or secret store: the host row, its jump host, the referenced credentials
/// and the resolved secrets. Kept in RAM only (needed for keepalive and
/// reconnects by the transport).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SshHostSnapshot {
    pub hosts: Vec<HostRecord>,
    pub credentials: Vec<CredentialRecord>,
    pub secrets: Vec<SecretEntry>,
}

struct Redacted<'a>(&'a Option<String>);

impl std::fmt::Debug for Redacted<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(_) => f.write_str("Some(<redacted>)"),
            None => f.write_str("None"),
        }
    }
}

/// [`labonair_ssh::SshConnectRequest`] plus the metadata of the hosted
/// session that carries the PTY.
#[derive(Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SshConnectParams {
    /// The app's SSH session id (tab id).
    pub session_id: String,
    pub host_id: String,
    pub password: Option<String>,
    pub passphrase: Option<String>,
    pub initial_cols: Option<u32>,
    pub initial_rows: Option<u32>,
    pub blocks: bool,
    pub connect_timeout_secs: Option<u64>,
    /// `None` = keepalive on with the daemon default (25 s, 3 misses).
    pub keepalive_interval_secs: Option<u64>,
    pub keepalive_max_failures: Option<u32>,
    /// Host rows and secrets for this connect; see [`SshHostSnapshot`].
    pub host: SshHostSnapshot,
    /// Wire id of the hosted session; allocated by the daemon when `None`.
    pub id: Option<SessionId>,
    pub pane_tag: Option<String>,
    pub label: Option<String>,
    /// Daemon-side scrollback lines for the shadow emulator.
    pub scrollback: Option<u32>,
    pub persist_key: Option<String>,
    /// Palette for the shadow emulator; xterm defaults when `None`.
    pub colors: Option<WireColors>,
}

impl std::fmt::Debug for SshConnectParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshConnectParams")
            .field("session_id", &self.session_id)
            .field("host_id", &self.host_id)
            .field("password", &Redacted(&self.password))
            .field("passphrase", &Redacted(&self.passphrase))
            .field("initial_cols", &self.initial_cols)
            .field("initial_rows", &self.initial_rows)
            .field("blocks", &self.blocks)
            .field("connect_timeout_secs", &self.connect_timeout_secs)
            .field("keepalive_interval_secs", &self.keepalive_interval_secs)
            .field("keepalive_max_failures", &self.keepalive_max_failures)
            .field("host", &self.host)
            .field("id", &self.id)
            .field("pane_tag", &self.pane_tag)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

/// Mirror of `labonair_transfers::TransferResolution`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireResolution {
    Overwrite,
    Skip,
    SkipAll,
    Rename { name: String },
    Abort,
}

impl From<labonair_transfers::TransferResolution> for WireResolution {
    fn from(resolution: labonair_transfers::TransferResolution) -> Self {
        use labonair_transfers::TransferResolution as R;
        match resolution {
            R::Overwrite => Self::Overwrite,
            R::Skip => Self::Skip,
            R::SkipAll => Self::SkipAll,
            R::Rename(name) => Self::Rename { name },
            R::Abort => Self::Abort,
        }
    }
}

impl From<WireResolution> for labonair_transfers::TransferResolution {
    fn from(resolution: WireResolution) -> Self {
        use labonair_transfers::TransferResolution as R;
        match resolution {
            WireResolution::Overwrite => R::Overwrite,
            WireResolution::Skip => R::Skip,
            WireResolution::SkipAll => R::SkipAll,
            WireResolution::Rename { name } => R::Rename(name),
            WireResolution::Abort => R::Abort,
        }
    }
}

/// Transfer worker settings (`labonair_transfers::TransferSettings`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferConfig {
    pub max_concurrent: usize,
    pub chunk_size: usize,
    /// `ask` | `overwrite` | `skip`. While no UI client is connected the
    /// daemon treats `overwrite` as `ask`: it never overwrites unattended.
    pub default_conflict_resolution: String,
    /// `ask` | `skip` | `abort`.
    pub on_folder_file_error: String,
}

/// One transfer job with its log and pending prompt, as the daemon's
/// `TransferRegistry` knows it (`labonair_transfers::TransferSnapshot`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireTransferSnapshot {
    pub job: labonair_transfers::TransferJob,
    /// `(timestamp_ms, message)`.
    pub steps: Vec<(i64, String)>,
    /// `(source_path, destination_path)` of a pending conflict.
    pub conflict: Option<(String, String)>,
    /// `(path, error)` of a pending per-file error.
    pub file_error: Option<(String, String)>,
}

/// A hosted SSH session as listed by [`RpcCall::ListSshSessions`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SshSessionInfo {
    /// Wire id for `Attach`/`Detach`/`Kill`/`Ack`/`Resize` and data frames.
    pub session: SessionId,
    pub ssh_session_id: String,
    pub host_id: String,
    /// `false` while connecting or after the connection was lost.
    pub connected: bool,
    pub lost_reason: Option<String>,
    pub pane_tag: Option<String>,
    pub label: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

/// A request/response call; see [`Message::Rpc`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "call", rename_all = "snake_case")]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum RpcCall {
    /// Start receiving [`Message::BusEvent`]s. The daemon first replays its
    /// current state as events (per live SSH session `session_established`
    /// or `ssh_connection_lost`; per transfer job `transfer_progress`,
    /// `transfer_step`s and a pending `file_conflict` / `file_error`, oldest
    /// job first), then answers `RpcOk(Unit)`, then streams live events.
    /// Completed jobs are dropped from the daemon after they were replayed.
    /// Without a subscribed connection host-key prompts fail closed.
    SubscribeEvents,
    UnsubscribeEvents,

    // SshConnectionService
    /// `connect`: creates a hosted SSH session. The reply carries its wire
    /// id; `Attach` it to receive the snapshot and live output (PTY bytes
    /// never travel in the reply). Output arriving before the attach is part
    /// of the snapshot.
    SshConnect(SshConnectParams),
    SshDisconnect {
        session_id: String,
    },
    /// Answer to a `known_hosts_warning` event.
    SshTrustHost {
        session_id: String,
        accepted: bool,
    },

    // SshPtyService (the hot path is `DataKind::Input` / `Message::Resize` on
    // the wire id; these are the by-ssh-id equivalents)
    SshPtyWrite {
        session_id: String,
        data: String,
    },
    SshPtyResize {
        session_id: String,
        cols: u32,
        rows: u32,
    },

    // SshRemoteCommandService
    SshChown {
        session_id: String,
        path: String,
        owner: String,
        group: String,
    },
    SshCalculateSize {
        session_id: String,
        path: String,
    },

    // SshRemoteFileService (local paths are on the daemon's machine, i.e.
    // the app's: same user, same filesystem)
    SshPrepareRemoteEdit {
        session_id: String,
        remote_path: String,
        max_bytes: Option<u64>,
    },
    SshSaveRemoteEdit {
        session_id: String,
        remote_path: String,
        local_temp_path: String,
    },
    SshCleanupRemoteEditTemp {
        local_temp_path: String,
    },

    // SshTunnelService
    SshTunnelStart {
        host_id: String,
        host: SshHostSnapshot,
    },
    SshTunnelStop {
        host_id: String,
    },
    SshTunnelsActive,

    // SftpSessionService / SftpBrowserService (`session` is the handle string)
    SftpOpen {
        ssh_session: String,
    },
    SftpClose {
        session: String,
    },
    SftpReadDir {
        session: String,
        path: String,
    },
    SftpRename {
        session: String,
        old_path: String,
        new_path: String,
    },
    SftpDelete {
        session: String,
        paths: Vec<String>,
    },
    SftpMkdir {
        session: String,
        path: String,
        recursive: bool,
    },
    SftpCreateFile {
        session: String,
        path: String,
    },
    SftpChmod {
        session: String,
        path: String,
        mode: u32,
    },

    // TransferService
    /// Local paths must be absolute (or `~`-prefixed); relative and empty
    /// paths are rejected.
    TransferEnqueue {
        session_id: String,
        src_path: String,
        dest_path: String,
        direction: labonair_transfers::TransferDirection,
    },
    TransferCancel {
        job_id: String,
    },
    TransferResolve {
        job_id: String,
        resolution: WireResolution,
    },
    TransferConfigure(TransferConfig),
    ListTransfers,
    ListSshSessions,

    /// A call from a newer peer; answered with an error.
    #[serde(other)]
    Unknown,
}

/// Successful result of an [`RpcCall`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
#[non_exhaustive]
pub enum RpcValue {
    Unit,
    Text(String),
    /// Reply to `SshConnect`: wire id of the hosted session.
    SshConnected {
        session: SessionId,
    },
    /// Reply to `SftpOpen`: the handle string.
    SftpHandle(String),
    Entries(Vec<labonair_sftp::RemoteEntry>),
    /// Reply to `TransferEnqueue`.
    JobId(String),
    Tunnels(Vec<labonair_ssh::ActiveTunnel>),
    Transfers(Vec<WireTransferSnapshot>),
    SshSessions(Vec<SshSessionInfo>),
}

/// A failed [`RpcCall`]. `code` is the `LabonairError` variant name when the
/// service returned a typed error (`AuthFailed`, `NetworkError`,
/// `HostKeyMismatch`, `IoError`, `Internal`, `NotConnected`, `NotFound`,
/// `PermissionDenied`, `InvalidInput`, `Timeout`, `Conflict`), `None` for the
/// services that return a plain string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireError {
    #[serde(default)]
    pub code: Option<String>,
    pub message: String,
}

impl WireError {
    pub fn plain(message: impl Into<String>) -> Self {
        Self {
            code: None,
            message: message.into(),
        }
    }

    pub fn from_labonair(error: &labonair_errors::LabonairError) -> Self {
        let value = serde_json::to_value(error).unwrap_or_default();
        Self {
            code: value
                .get("code")
                .and_then(|c| c.as_str())
                .map(str::to_string),
            message: value
                .get("message")
                .and_then(|m| m.as_str())
                .map_or_else(|| error.to_string(), str::to_string),
        }
    }

    /// Rebuild the typed error. Unknown or missing codes become `Internal`.
    pub fn into_labonair(self) -> labonair_errors::LabonairError {
        use labonair_errors::LabonairError as E;
        let m = self.message;
        match self.code.as_deref() {
            Some("AuthFailed") => E::AuthFailed(m),
            Some("NetworkError") => E::NetworkError(m),
            Some("HostKeyMismatch") => E::HostKeyMismatch(m),
            Some("IoError") => E::IoError(m),
            Some("NotConnected") => E::NotConnected(m),
            Some("NotFound") => E::NotFound(m),
            Some("PermissionDenied") => E::PermissionDenied(m),
            Some("InvalidInput") => E::InvalidInput(m),
            Some("Timeout") => E::Timeout(m),
            Some("Conflict") => E::Conflict(m),
            _ => E::Internal(m),
        }
    }
}

impl From<String> for WireError {
    fn from(message: String) -> Self {
        Self::plain(message)
    }
}

impl From<labonair_errors::LabonairError> for WireError {
    fn from(error: labonair_errors::LabonairError) -> Self {
        Self::from_labonair(&error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_messages() -> Vec<Message> {
        vec![
            Message::Hello(Hello::current("test", Some(5000))),
            Message::CreateSession {
                req_id: 7,
                id: None,
                options: CreateOptions {
                    shell: Some("/bin/sh".into()),
                    env: vec![("A".into(), "b".into())],
                    cols: 80,
                    rows: 24,
                    ..Default::default()
                },
                colors: Some((&labonair_terminal_core::TerminalColors::xterm_default()).into()),
            },
            Message::Attach {
                session: 3,
                cols: 100,
                rows: 30,
            },
            Message::Sessions {
                sessions: vec![SessionInfo {
                    id: 1,
                    pane_tag: Some("p".into()),
                    label: None,
                    cwd: Some("/tmp".into()),
                    title: None,
                    status: SessionStatus::Exited,
                    exit_code: Some(3),
                    attached: false,
                    cols: 80,
                    rows: 24,
                    foreground: None,
                    persist_key: "k".into(),
                }],
            },
            Message::Event {
                session: 1,
                event: SessionEvent::CommandFinished { exit_code: Some(0) },
            },
            Message::Detached {
                session: 1,
                reason: DetachReason::Stolen,
            },
            Message::Shutdown,
            Message::Rpc {
                req_id: 9,
                call: RpcCall::SshConnect(connect_params()),
            },
            Message::Rpc {
                req_id: 10,
                call: RpcCall::TransferResolve {
                    job_id: "j".into(),
                    resolution: WireResolution::Rename { name: "b".into() },
                },
            },
            Message::Rpc {
                req_id: 11,
                call: RpcCall::SftpMkdir {
                    session: "s".into(),
                    path: "/a/b".into(),
                    recursive: true,
                },
            },
            Message::RpcOk {
                req_id: 9,
                value: RpcValue::SshConnected { session: 4 },
            },
            Message::RpcOk {
                req_id: 12,
                value: RpcValue::Entries(vec![labonair_sftp::RemoteEntry {
                    name: "f".into(),
                    path: "/f".into(),
                    size: 1,
                    modified_at: 2,
                    is_dir: false,
                    is_symlink: false,
                    symlink_target: None,
                    permissions: "rw-r--r--".into(),
                }]),
            },
            Message::RpcOk {
                req_id: 13,
                value: RpcValue::Transfers(vec![WireTransferSnapshot {
                    job: labonair_transfers::TransferJob {
                        id: "j".into(),
                        session_id: "s".into(),
                        src_path: "/a".into(),
                        dest_path: "/b".into(),
                        direction: labonair_transfers::TransferDirection::Upload,
                        status: labonair_transfers::TransferStatus::Failed("x".into()),
                        bytes_total: 10,
                        bytes_transferred: 5,
                        speed_bps: 1.5,
                        skipped_count: 0,
                    },
                    steps: vec![(1, "step".into())],
                    conflict: Some(("/a".into(), "/b".into())),
                    file_error: None,
                }]),
            },
            Message::RpcErr {
                req_id: 9,
                error: WireError::from_labonair(&labonair_errors::LabonairError::AuthFailed(
                    "no".into(),
                )),
            },
            Message::BusEvent {
                name: "file_conflict".into(),
                payload: serde_json::json!({ "job_id": "j" }),
            },
            Message::ConnectionLost {
                session: 4,
                reason: "Connection timed out".into(),
            },
        ]
    }

    fn connect_params() -> SshConnectParams {
        SshConnectParams {
            session_id: "tab-1".into(),
            host_id: "host-1".into(),
            password: Some("hunter2-password".into()),
            passphrase: Some("hunter2-passphrase".into()),
            host: SshHostSnapshot {
                hosts: vec![HostRecord {
                    id: "host-1".into(),
                    host_address: "example.org".into(),
                    port: 22,
                    username: "me".into(),
                    auth_method: "password".into(),
                    ..Default::default()
                }],
                credentials: vec![CredentialRecord {
                    id: "cred-1".into(),
                    ..Default::default()
                }],
                secrets: vec![SecretEntry {
                    service: "labonair-app".into(),
                    account: "host-1".into(),
                    value: "hunter2-secret".into(),
                }],
            },
            ..Default::default()
        }
    }

    fn all_frames() -> Vec<Frame> {
        let mut frames: Vec<Frame> = sample_messages().into_iter().map(Frame::Control).collect();
        for kind in [
            DataKind::Output,
            DataKind::Input,
            DataKind::Snapshot,
            DataKind::Resync,
        ] {
            frames.push(Frame::Data {
                kind,
                session: u64::MAX - 1,
                data: (0..=255u8).collect(),
            });
        }
        frames.push(Frame::Data {
            kind: DataKind::Output,
            session: 0,
            data: Vec::new(),
        });
        frames
    }

    #[test]
    fn round_trip_every_frame() {
        for frame in all_frames() {
            let mut buf = Vec::new();
            frame.encode(&mut buf).unwrap();
            let (decoded, used) = Frame::decode(&buf).unwrap().unwrap();
            assert_eq!(used, buf.len());
            assert_eq!(decoded, frame);
        }
    }

    #[test]
    fn data_frame_layout_is_raw() {
        let buf = data_frame(DataKind::Output, 0x0102, b"hi");
        assert_eq!(buf[0], 0x01);
        assert_eq!(&buf[1..9], &0x0102u64.to_le_bytes());
        assert_eq!(&buf[9..13], &2u32.to_le_bytes());
        assert_eq!(&buf[13..], b"hi");
    }

    #[test]
    fn partial_reads_at_every_split_point() {
        let mut stream = Vec::new();
        for frame in all_frames() {
            frame.encode(&mut stream).unwrap();
        }
        // Feed one byte at a time, as the worst-case partial read.
        let mut pending = Vec::new();
        let mut decoded = Vec::new();
        for byte in stream {
            pending.push(byte);
            while let Some((frame, used)) = Frame::decode(&pending).unwrap() {
                pending.drain(..used);
                decoded.push(frame);
            }
        }
        assert!(pending.is_empty());
        assert_eq!(decoded, all_frames());
    }

    #[test]
    fn oversize_is_rejected_from_the_header() {
        let mut buf = vec![DataKind::Output as u8];
        buf.extend_from_slice(&1u64.to_le_bytes());
        buf.extend_from_slice(&(MAX_FRAME_LEN as u32 + 1).to_le_bytes());
        assert!(matches!(
            Frame::decode(&buf),
            Err(ProtocolError::Oversize(_))
        ));
        let mut control = vec![CONTROL_TAG];
        control.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            Frame::decode(&control),
            Err(ProtocolError::Oversize(_))
        ));
    }

    #[test]
    fn unknown_type_and_garbage_json_are_errors_not_panics() {
        assert!(matches!(
            Frame::decode(&[0x7f, 0, 0, 0, 0]),
            Err(ProtocolError::UnknownFrameType(0x7f))
        ));
        let mut buf = vec![CONTROL_TAG];
        buf.extend_from_slice(&3u32.to_le_bytes());
        buf.extend_from_slice(b"{{{");
        assert!(matches!(Frame::decode(&buf), Err(ProtocolError::Json(_))));
    }

    #[test]
    fn unknown_control_variant_decodes_as_unknown() {
        let json = br#"{"type":"ssh_connect","host":"x"}"#;
        let mut buf = vec![CONTROL_TAG];
        buf.extend_from_slice(&(json.len() as u32).to_le_bytes());
        buf.extend_from_slice(json);
        let (frame, _) = Frame::decode(&buf).unwrap().unwrap();
        assert_eq!(frame, Frame::Control(Message::Unknown));
    }

    #[test]
    fn arbitrary_bytes_never_panic() {
        // Deterministic xorshift "fuzz": decode must return, never panic.
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        for _ in 0..2000 {
            let mut buf = Vec::new();
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let len = (state % 64) as usize;
            let mut s = state;
            for _ in 0..len {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                buf.push((s >> 56) as u8);
            }
            let _ = Frame::decode(&buf);
        }
    }

    #[test]
    fn secrets_never_show_up_in_debug_output() {
        let message = Message::Rpc {
            req_id: 1,
            call: RpcCall::SshConnect(connect_params()),
        };
        let tunnel = Message::Rpc {
            req_id: 2,
            call: RpcCall::SshTunnelStart {
                host_id: "host-1".into(),
                host: connect_params().host,
            },
        };
        for text in [format!("{message:?}"), format!("{tunnel:?}")] {
            assert!(!text.contains("hunter2"), "secret leaked: {text}");
            assert!(text.contains("<redacted>"));
            assert!(text.contains("host-1"), "non-secrets stay readable");
        }
        let pretty = format!("{:#?}", connect_params());
        assert!(!pretty.contains("hunter2"));
    }

    #[test]
    fn unknown_calls_decode_as_unknown() {
        let json = br#"{"type":"rpc","req_id":5,"call":{"call":"from_the_future","x":1}}"#;
        let mut buf = vec![CONTROL_TAG];
        buf.extend_from_slice(&(json.len() as u32).to_le_bytes());
        buf.extend_from_slice(json);
        let (frame, _) = Frame::decode(&buf).unwrap().unwrap();
        assert_eq!(
            frame,
            Frame::Control(Message::Rpc {
                req_id: 5,
                call: RpcCall::Unknown
            })
        );
    }

    #[test]
    fn typed_errors_survive_the_wire() {
        use labonair_errors::LabonairError as E;
        for error in [
            E::AuthFailed("a".into()),
            E::NetworkError("b".into()),
            E::HostKeyMismatch("c".into()),
            E::IoError("d".into()),
            E::NotConnected("e".into()),
            E::NotFound("f".into()),
            E::PermissionDenied("g".into()),
            E::InvalidInput("h".into()),
            E::Timeout("i".into()),
            E::Conflict("j".into()),
            E::Internal("k".into()),
        ] {
            let wire = WireError::from_labonair(&error);
            assert!(wire.code.is_some());
            assert_eq!(wire.into_labonair(), error);
        }
        assert_eq!(
            WireError::plain("x").into_labonair(),
            E::Internal("x".into())
        );
    }

    #[test]
    fn resolutions_round_trip() {
        use labonair_transfers::TransferResolution as R;
        for resolution in [
            R::Overwrite,
            R::Skip,
            R::SkipAll,
            R::Rename("n".into()),
            R::Abort,
        ] {
            assert_eq!(
                R::from(WireResolution::from(resolution.clone())),
                resolution
            );
        }
    }

    #[test]
    fn wire_colors_round_trip() {
        let colors = labonair_terminal_core::TerminalColors::xterm_default();
        let wire = WireColors::from(&colors);
        assert_eq!(labonair_terminal_core::TerminalColors::from(&wire), colors);
    }
}
