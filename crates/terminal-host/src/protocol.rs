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
        ]
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
    fn wire_colors_round_trip() {
        let colors = labonair_terminal_core::TerminalColors::xterm_default();
        let wire = WireColors::from(&colors);
        assert_eq!(labonair_terminal_core::TerminalColors::from(&wire), colors);
    }
}
