use std::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use encoding_rs::{Encoding, SHIFT_JIS, UTF_16BE, UTF_16LE, UTF_8, WINDOWS_1252};
use serde::Serialize;

const MAX_READ_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
const BINARY_SNIFF_BYTES: usize = 8 * 1024;

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ReadResult {
    Text {
        content: String,
        size: u64,
    },
    Binary {
        size: u64,
    },
    /// File exceeds MAX_READ_BYTES. UI decides whether to offer "open anyway".
    TooLarge {
        size: u64,
        limit: u64,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StatKind {
    File,
    Dir,
    Symlink,
}

#[derive(Serialize)]
pub struct FileStat {
    pub size: u64,
    pub mtime: u64,
    pub kind: StatKind,
}

/// Resolves symlinks and returns the canonical absolute path. Used by the AI
/// tool layer (`src/modules/ai/lib/security.ts`'s `checkReadableResolved`/
/// `checkWritableResolved`) to re-check the deny-list against a symlink's
/// actual target, not just the literal path the model asked for.
pub async fn fs_realpath(path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let p = super::paths::expand_home(&path)?;
        std::fs::canonicalize(&p)
            .map(|c| c.to_string_lossy().to_string())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn fs_read_file(path: String, max_bytes: Option<u64>) -> Result<ReadResult, String> {
    let limit = max_bytes.unwrap_or(MAX_READ_BYTES);
    tokio::task::spawn_blocking(move || {
        let p = super::paths::expand_home(&path)?;
        let meta = std::fs::metadata(&p).map_err(|e| {
            log::debug!("fs_read_file stat({}) failed: {e}", p.display());
            e.to_string()
        })?;

        let size = meta.len();
        if size > limit {
            return Ok(ReadResult::TooLarge { size, limit });
        }

        let bytes = std::fs::read(&p).map_err(|e| {
            log::debug!("fs_read_file read({}) failed: {e}", p.display());
            e.to_string()
        })?;

        // Null-byte sniff on the first chunk. Not perfect (misses UTF-16 BOM
        // cases) but catches the common "this is a PNG" mistake cheaply.
        let sniff_len = bytes.len().min(BINARY_SNIFF_BYTES);
        if bytes[..sniff_len].contains(&0) {
            return Ok(ReadResult::Binary { size });
        }

        // Strict UTF-8: fs_write_file writes the returned content back to disk
        // verbatim, so a lossy decode here would silently replace invalid byte
        // sequences with U+FFFD on save, permanently corrupting any file that
        // isn't valid UTF-8. Files that fail this check are reported as
        // Binary — refusing to open them as text is safer than corrupting them.
        match String::from_utf8(bytes) {
            Ok(content) => Ok(ReadResult::Text { content, size }),
            Err(_) => Ok(ReadResult::Binary { size }),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Atomic write: stage into a sibling temp file, then rename over the target.
/// Prevents partial writes from leaving a half-saved file on crash/power loss.
pub async fn fs_write_file(path: String, content: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let target = super::paths::expand_home(&path)?;
        let parent = target
            .parent()
            .ok_or_else(|| "path has no parent".to_string())?;
        let file_name = target
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "path has no file name".to_string())?;

        let tmp = parent.join(format!(".{file_name}.labonair.tmp"));

        {
            let mut f = std::fs::File::create(&tmp).map_err(|e| {
                log::debug!("fs_write_file create({}) failed: {e}", tmp.display());
                e.to_string()
            })?;
            f.write_all(content.as_bytes()).map_err(|e| {
                log::debug!("fs_write_file write({}) failed: {e}", tmp.display());
                e.to_string()
            })?;
            f.sync_all().map_err(|e| e.to_string())?;
        }

        std::fs::rename(&tmp, &target).map_err(|e| {
            log::warn!(
                "fs_write_file rename({} -> {}) failed: {e}",
                tmp.display(),
                target.display()
            );
            // Best-effort cleanup of the staged temp.
            let _ = std::fs::remove_file(&tmp);
            e.to_string()
        })?;

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn fs_stat(path: String) -> Result<FileStat, String> {
    tokio::task::spawn_blocking(move || {
        let p = super::paths::expand_home(&path)?;
        let meta = std::fs::metadata(&p).map_err(|e| e.to_string())?;
        let kind = if meta.is_dir() {
            StatKind::Dir
        } else if meta.file_type().is_symlink() {
            StatKind::Symlink
        } else {
            StatKind::File
        };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Ok(FileStat {
            size: meta.len(),
            mtime,
            kind,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

// --- Blocking, in-process variants for the code editor (T06-001). ---
// Same semantics as `fs_read_file` / `fs_write_file`, minus the async wrapper;
// the GPUI editor view runs these on `cx.background_executor().spawn`. They also
// surface the mtime so the editor can detect external changes.

/// Storage-side identity. The editor capability converts this value into its
/// own domain identity at the Workspace adapter boundary; the foundation must
/// not depend upward on `labonair-editor`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StorageFileIdentity {
    pub path: PathBuf,
    pub size: u64,
    pub modified_ms: u64,
    pub content_hash: u64,
}

impl StorageFileIdentity {
    fn new(path: impl Into<PathBuf>, size: u64, modified_ms: u64, content_hash: u64) -> Self {
        Self {
            path: path.into(),
            size,
            modified_ms,
            content_hash,
        }
    }

    pub fn from_bytes(path: impl Into<PathBuf>, modified_ms: u64, bytes: &[u8]) -> Self {
        Self::new(
            path,
            bytes.len() as u64,
            modified_ms,
            storage_content_hash(bytes),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StorageFileCapabilities {
    pub can_read: bool,
    pub can_edit: bool,
    pub can_save: bool,
}

impl StorageFileCapabilities {
    const fn editable() -> Self {
        Self {
            can_read: true,
            can_edit: true,
            can_save: true,
        }
    }

    const fn read_only() -> Self {
        Self {
            can_read: true,
            can_edit: false,
            can_save: false,
        }
    }

    pub const fn unavailable() -> Self {
        Self {
            can_read: false,
            can_edit: false,
            can_save: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageLineEnding {
    Lf,
    CrLf,
    Cr,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageBom {
    None,
    Utf8,
    Utf16Le,
    Utf16Be,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageEncoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Windows1252,
    ShiftJis,
}

impl StorageBom {
    const fn as_encoding(self) -> Option<StorageEncoding> {
        match self {
            Self::None => None,
            Self::Utf8 => Some(StorageEncoding::Utf8),
            Self::Utf16Le => Some(StorageEncoding::Utf16Le),
            Self::Utf16Be => Some(StorageEncoding::Utf16Be),
        }
    }
}

impl StorageEncoding {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Windows1252 => "Windows-1252",
            Self::ShiftJis => "Shift_JIS",
        }
    }

    const fn is_utf16(self) -> bool {
        matches!(self, Self::Utf16Le | Self::Utf16Be)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageFileSnapshot {
    pub identity: StorageFileIdentity,
    pub text: String,
    pub line_ending: StorageLineEnding,
    pub encoding: StorageEncoding,
    pub bom: StorageBom,
    pub capabilities: StorageFileCapabilities,
}

impl StorageFileSnapshot {
    fn new(
        identity: StorageFileIdentity,
        text: String,
        line_ending: StorageLineEnding,
        encoding: StorageEncoding,
        bom: StorageBom,
        capabilities: StorageFileCapabilities,
    ) -> Self {
        Self {
            identity,
            text,
            line_ending,
            encoding,
            bom,
            capabilities,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageSaveIntent {
    Normal,
    OverwriteExternal,
}

/// Outcome of loading a file for the editor.
pub enum EditorLifecycleLoad {
    /// Decoded text plus the complete accepted disk snapshot.
    Text { snapshot: StorageFileSnapshot },
    /// Binary — the editor refuses to open it as text.
    Binary {
        identity: StorageFileIdentity,
        capabilities: StorageFileCapabilities,
    },
    /// Larger than `limit` bytes.
    TooLarge {
        identity: StorageFileIdentity,
        size: u64,
        limit: u64,
        capabilities: StorageFileCapabilities,
    },
    /// The detected file encoding is not supported by the editor.
    UnsupportedEncoding {
        identity: StorageFileIdentity,
        encoding: String,
        capabilities: StorageFileCapabilities,
    },
    /// Explicit UTF-8 decoding failed without exposing replacement text.
    InvalidUtf8 {
        identity: StorageFileIdentity,
        capabilities: StorageFileCapabilities,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorFileStat {
    pub identity: StorageFileIdentity,
    pub capabilities: StorageFileCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorFileError {
    Missing {
        path: PathBuf,
    },
    Io {
        path: PathBuf,
        message: String,
    },
    InvalidUtf8 {
        path: PathBuf,
    },
    UnsupportedEncoding {
        path: PathBuf,
        encoding: String,
    },
    InvalidEncodingData {
        path: PathBuf,
        encoding: String,
    },
    UnrepresentableText {
        path: PathBuf,
        encoding: String,
    },
    Binary {
        path: PathBuf,
    },
    TooLarge {
        path: PathBuf,
        size: u64,
        limit: u64,
    },
    ExternalChange {
        expected: StorageFileIdentity,
        actual: StorageFileIdentity,
    },
    ReadOnly {
        path: PathBuf,
    },
}

impl fmt::Display for EditorFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { path } => write!(f, "file not found: {}", path.display()),
            Self::Io { path, message } => write!(f, "{}: {message}", path.display()),
            Self::InvalidUtf8 { path } => write!(f, "invalid UTF-8: {}", path.display()),
            Self::UnsupportedEncoding { path, encoding } => {
                write!(f, "unsupported {encoding} encoding: {}", path.display())
            }
            Self::InvalidEncodingData { path, encoding } => {
                write!(f, "invalid {encoding} data: {}", path.display())
            }
            Self::UnrepresentableText { path, encoding } => {
                write!(
                    f,
                    "text cannot be represented as {encoding}: {}",
                    path.display()
                )
            }
            Self::Binary { path } => write!(f, "binary file: {}", path.display()),
            Self::TooLarge { path, size, limit } => write!(
                f,
                "file {} is {size} bytes, exceeding the {limit}-byte editor limit",
                path.display()
            ),
            Self::ExternalChange { .. } => write!(f, "file changed on disk before save"),
            Self::ReadOnly { path } => write!(f, "file is read-only: {}", path.display()),
        }
    }
}

impl std::error::Error for EditorFileError {}

fn mtime_ms(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn storage_content_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn storage_detect_line_ending(text: &str) -> StorageLineEnding {
    let bytes = text.as_bytes();
    let mut lf = 0usize;
    let mut crlf = 0usize;
    let mut cr = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                crlf += 1;
                index += 2;
            }
            b'\r' => {
                cr += 1;
                index += 1;
            }
            b'\n' => {
                lf += 1;
                index += 1;
            }
            _ => index += 1,
        }
    }
    let styles = [lf > 0, crlf > 0, cr > 0]
        .into_iter()
        .filter(|present| *present)
        .count();
    if styles > 1 {
        StorageLineEnding::Mixed
    } else if crlf > 0 {
        StorageLineEnding::CrLf
    } else if cr > 0 {
        StorageLineEnding::Cr
    } else {
        StorageLineEnding::Lf
    }
}

fn storage_normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn storage_apply_line_ending(text: &str, ending: StorageLineEnding) -> String {
    match ending {
        StorageLineEnding::Lf | StorageLineEnding::Mixed => text.to_string(),
        StorageLineEnding::CrLf => text.replace('\n', "\r\n"),
        StorageLineEnding::Cr => text.replace('\n', "\r"),
    }
}

fn capabilities(meta: &std::fs::Metadata) -> StorageFileCapabilities {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o222 != 0 {
            StorageFileCapabilities::editable()
        } else {
            StorageFileCapabilities::read_only()
        }
    }
    #[cfg(not(unix))]
    {
        if meta.permissions().readonly() {
            StorageFileCapabilities::read_only()
        } else {
            StorageFileCapabilities::editable()
        }
    }
}

fn stat_for_bytes(
    path: &std::path::Path,
    meta: &std::fs::Metadata,
    bytes: &[u8],
) -> EditorFileStat {
    EditorFileStat {
        identity: StorageFileIdentity::new(
            path.to_path_buf(),
            meta.len(),
            mtime_ms(meta),
            storage_content_hash(bytes),
        ),
        capabilities: capabilities(meta),
    }
}

fn unsupported_encoding(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xff, 0xfe, 0, 0]) || bytes.starts_with(&[0, 0, 0xfe, 0xff]) {
        Some("UTF-32")
    } else {
        None
    }
}

fn detected_bom(bytes: &[u8]) -> (StorageBom, usize) {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        (StorageBom::Utf8, 3)
    } else if bytes.starts_with(&[0xff, 0xfe]) {
        (StorageBom::Utf16Le, 2)
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        (StorageBom::Utf16Be, 2)
    } else {
        (StorageBom::None, 0)
    }
}

fn encoding_for_bytes(
    bytes: &[u8],
    forced: Option<StorageEncoding>,
) -> (StorageEncoding, StorageBom, usize) {
    let (detected, prefix_len) = detected_bom(bytes);
    let payload = &bytes[prefix_len..];
    let inferred = detected.as_encoding().unwrap_or_else(|| {
        if std::str::from_utf8(payload).is_ok() {
            StorageEncoding::Utf8
        } else {
            StorageEncoding::Windows1252
        }
    });
    let encoding = forced.unwrap_or(inferred);
    let bom = if detected.as_encoding() == Some(encoding) {
        detected
    } else {
        StorageBom::None
    };
    (encoding, bom, prefix_len)
}

fn codec(encoding: StorageEncoding) -> &'static Encoding {
    match encoding {
        StorageEncoding::Utf8 => UTF_8,
        StorageEncoding::Utf16Le => UTF_16LE,
        StorageEncoding::Utf16Be => UTF_16BE,
        StorageEncoding::Windows1252 => WINDOWS_1252,
        StorageEncoding::ShiftJis => SHIFT_JIS,
    }
}

fn decode_text(
    bytes: &[u8],
    encoding: StorageEncoding,
    path: &std::path::Path,
) -> Result<String, EditorFileError> {
    let (text, _, had_errors) = codec(encoding).decode(bytes);
    if had_errors {
        return if encoding == StorageEncoding::Utf8 {
            Err(EditorFileError::InvalidUtf8 {
                path: path.to_path_buf(),
            })
        } else {
            Err(EditorFileError::InvalidEncodingData {
                path: path.to_path_buf(),
                encoding: encoding.as_str().to_string(),
            })
        };
    }
    Ok(text.into_owned())
}

fn encode_text(
    text: &str,
    encoding: StorageEncoding,
    path: &std::path::Path,
) -> Result<Vec<u8>, EditorFileError> {
    if encoding.is_utf16() {
        let mut bytes = Vec::with_capacity(text.len() * 2);
        for unit in text.encode_utf16() {
            let pair = match encoding {
                StorageEncoding::Utf16Le => unit.to_le_bytes(),
                StorageEncoding::Utf16Be => unit.to_be_bytes(),
                _ => {
                    return Err(EditorFileError::InvalidEncodingData {
                        path: path.to_path_buf(),
                        encoding: encoding.as_str().to_string(),
                    })
                }
            };
            bytes.extend_from_slice(&pair);
        }
        return Ok(bytes);
    }

    let (bytes, _, had_errors) = codec(encoding).encode(text);
    if had_errors {
        return Err(EditorFileError::UnrepresentableText {
            path: path.to_path_buf(),
            encoding: encoding.as_str().to_string(),
        });
    }
    Ok(bytes.into_owned())
}

fn snapshot_from_bytes(
    path: &std::path::Path,
    meta: &std::fs::Metadata,
    bytes: &[u8],
    forced_encoding: Option<StorageEncoding>,
) -> Result<StorageFileSnapshot, EditorFileError> {
    let stat = stat_for_bytes(path, meta, bytes);
    if let Some(encoding) = unsupported_encoding(bytes) {
        return Err(EditorFileError::UnsupportedEncoding {
            path: path.to_path_buf(),
            encoding: encoding.to_string(),
        });
    }
    let (encoding, bom, prefix_len) = encoding_for_bytes(bytes, forced_encoding);
    let text = decode_text(&bytes[prefix_len..], encoding, path)?;
    Ok(StorageFileSnapshot::new(
        stat.identity,
        storage_normalize_line_endings(&text),
        storage_detect_line_ending(&text),
        encoding,
        bom,
        stat.capabilities,
    ))
}

/// Read a file as editor text. `max_bytes` defaults to [`MAX_READ_BYTES`].
pub fn load_editor_file_lifecycle_sync(
    path: &str,
    max_bytes: Option<u64>,
) -> Result<EditorLifecycleLoad, EditorFileError> {
    load_editor_file_lifecycle_with_encoding_sync(path, max_bytes, None)
}

/// Read editor text using a specific encoding when requested. A missing
/// override uses BOM detection, then strict UTF-8, with Windows-1252 as the
/// fallback for valid non-UTF-8 text.
pub fn load_editor_file_lifecycle_with_encoding_sync(
    path: &str,
    max_bytes: Option<u64>,
    forced_encoding: Option<StorageEncoding>,
) -> Result<EditorLifecycleLoad, EditorFileError> {
    let limit = max_bytes.unwrap_or(MAX_READ_BYTES);
    let p = super::paths::expand_home(path).map_err(|message| EditorFileError::Io {
        path: PathBuf::from(path),
        message,
    })?;
    let meta = std::fs::metadata(&p).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            EditorFileError::Missing { path: p.clone() }
        } else {
            EditorFileError::Io {
                path: p.clone(),
                message: error.to_string(),
            }
        }
    })?;
    let bytes = std::fs::read(&p).map_err(|error| EditorFileError::Io {
        path: p.clone(),
        message: error.to_string(),
    })?;
    let stat = stat_for_bytes(&p, &meta, &bytes);
    if meta.len() > limit {
        return Ok(EditorLifecycleLoad::TooLarge {
            identity: stat.identity,
            size: meta.len(),
            limit,
            capabilities: stat.capabilities,
        });
    }
    if let Some(encoding) = unsupported_encoding(&bytes) {
        return Ok(EditorLifecycleLoad::UnsupportedEncoding {
            identity: stat.identity,
            encoding: encoding.to_string(),
            capabilities: stat.capabilities,
        });
    }
    let (encoding, _, _) = encoding_for_bytes(&bytes, forced_encoding);
    let sniff_len = bytes.len().min(BINARY_SNIFF_BYTES);
    if !encoding.is_utf16() && bytes[..sniff_len].contains(&0) {
        return Ok(EditorLifecycleLoad::Binary {
            identity: stat.identity,
            capabilities: stat.capabilities,
        });
    }
    match snapshot_from_bytes(&p, &meta, &bytes, forced_encoding) {
        Ok(snapshot) => Ok(EditorLifecycleLoad::Text { snapshot }),
        Err(EditorFileError::InvalidUtf8 { .. }) => Ok(EditorLifecycleLoad::InvalidUtf8 {
            identity: stat.identity,
            capabilities: stat.capabilities,
        }),
        Err(error) => Err(error),
    }
}

/// Read the current identity and permissions for an open editor file.
pub fn stat_editor_file_sync(path: &str) -> Result<EditorFileStat, EditorFileError> {
    let p = super::paths::expand_home(path).map_err(|message| EditorFileError::Io {
        path: PathBuf::from(path),
        message,
    })?;
    let meta = std::fs::metadata(&p).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            EditorFileError::Missing { path: p.clone() }
        } else {
            EditorFileError::Io {
                path: p.clone(),
                message: error.to_string(),
            }
        }
    })?;
    let bytes = std::fs::read(&p).map_err(|error| EditorFileError::Io {
        path: p.clone(),
        message: error.to_string(),
    })?;
    Ok(stat_for_bytes(&p, &meta, &bytes))
}

/// Atomic write with encoding/EOL/BOM preservation and a pre-rename identity check.
pub fn save_editor_file_lifecycle_sync(
    path: &str,
    content: &str,
    encoding: StorageEncoding,
    line_ending: StorageLineEnding,
    bom: StorageBom,
    expected: &StorageFileIdentity,
    intent: StorageSaveIntent,
) -> Result<StorageFileSnapshot, EditorFileError> {
    let target = super::paths::expand_home(path).map_err(|message| EditorFileError::Io {
        path: PathBuf::from(path),
        message,
    })?;
    let parent = target.parent().ok_or_else(|| EditorFileError::Io {
        path: target.clone(),
        message: "path has no parent".into(),
    })?;
    let file_name =
        target
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| EditorFileError::Io {
                path: target.clone(),
                message: "path has no file name".into(),
            })?;
    let current = stat_editor_file_sync(path)?;
    if !current.capabilities.can_save {
        return Err(EditorFileError::ReadOnly {
            path: target.clone(),
        });
    }
    if intent == StorageSaveIntent::Normal && current.identity != *expected {
        return Err(EditorFileError::ExternalChange {
            expected: expected.clone(),
            actual: current.identity,
        });
    }
    let tmp = parent.join(format!(".{file_name}.labonair.tmp"));
    let original_permissions = std::fs::metadata(&target)
        .map_err(|e| EditorFileError::Io {
            path: target.clone(),
            message: e.to_string(),
        })?
        .permissions();
    if bom
        .as_encoding()
        .is_some_and(|bom_encoding| bom_encoding != encoding)
    {
        return Err(EditorFileError::InvalidEncodingData {
            path: target,
            encoding: encoding.as_str().to_string(),
        });
    }
    let formatted = storage_apply_line_ending(content, line_ending);
    let encoded = encode_text(&formatted, encoding, &target)?;
    let mut serialized = Vec::with_capacity(encoded.len() + 4);
    match bom {
        StorageBom::None => {}
        StorageBom::Utf8 => serialized.extend_from_slice(&[0xef, 0xbb, 0xbf]),
        StorageBom::Utf16Le => serialized.extend_from_slice(&[0xff, 0xfe]),
        StorageBom::Utf16Be => serialized.extend_from_slice(&[0xfe, 0xff]),
    }
    serialized.extend_from_slice(&encoded);
    {
        let mut f = std::fs::File::create(&tmp).map_err(|e| EditorFileError::Io {
            path: tmp.clone(),
            message: e.to_string(),
        })?;
        f.write_all(&serialized).map_err(|e| EditorFileError::Io {
            path: tmp.clone(),
            message: e.to_string(),
        })?;
        f.set_permissions(original_permissions.clone())
            .map_err(|e| EditorFileError::Io {
                path: tmp.clone(),
                message: e.to_string(),
            })?;
        f.sync_all().map_err(|e| EditorFileError::Io {
            path: tmp.clone(),
            message: e.to_string(),
        })?;
    }
    if intent == StorageSaveIntent::Normal {
        let latest = stat_editor_file_sync(path)?;
        if latest.identity != *expected {
            let _ = std::fs::remove_file(&tmp);
            return Err(EditorFileError::ExternalChange {
                expected: expected.clone(),
                actual: latest.identity,
            });
        }
    }
    std::fs::rename(&tmp, &target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        EditorFileError::Io {
            path: target.clone(),
            message: e.to_string(),
        }
    })?;
    let meta = std::fs::metadata(&target).map_err(|e| EditorFileError::Io {
        path: target.clone(),
        message: e.to_string(),
    })?;
    let mut snapshot = snapshot_from_bytes(&target, &meta, &serialized, Some(encoding))?;
    snapshot.line_ending = match line_ending {
        StorageLineEnding::Mixed => StorageLineEnding::Lf,
        selected => selected,
    };
    Ok(snapshot)
}

// Compatibility boundary for the AI file tools. The editor/workspace uses
// the typed lifecycle functions above; this adapter remains only until the AI
// capability consumes the same FileSnapshot contract.
#[derive(Debug)]
pub enum EditorLoad {
    Text { content: String, mtime: u64 },
    Binary,
    TooLarge { size: u64, limit: u64 },
}

pub fn load_editor_file_sync(path: &str, max_bytes: Option<u64>) -> Result<EditorLoad, String> {
    match load_editor_file_lifecycle_sync(path, max_bytes).map_err(|error| error.to_string())? {
        EditorLifecycleLoad::Text { snapshot } => Ok(EditorLoad::Text {
            content: snapshot.text,
            mtime: snapshot.identity.modified_ms,
        }),
        EditorLifecycleLoad::Binary { .. } | EditorLifecycleLoad::InvalidUtf8 { .. } => {
            Ok(EditorLoad::Binary)
        }
        EditorLifecycleLoad::TooLarge { size, limit, .. } => {
            Ok(EditorLoad::TooLarge { size, limit })
        }
        EditorLifecycleLoad::UnsupportedEncoding { .. } => Ok(EditorLoad::Binary),
    }
}

pub fn save_editor_file_sync(path: &str, content: &str) -> Result<u64, String> {
    let snapshot =
        match load_editor_file_lifecycle_sync(path, None).map_err(|error| error.to_string())? {
            EditorLifecycleLoad::Text { snapshot } => snapshot,
            EditorLifecycleLoad::TooLarge { size, limit, .. } => {
                return Err(format!(
                    "file is {size} bytes, exceeding the {limit}-byte limit"
                ))
            }
            EditorLifecycleLoad::Binary { .. }
            | EditorLifecycleLoad::InvalidUtf8 { .. }
            | EditorLifecycleLoad::UnsupportedEncoding { .. } => {
                return Err("file encoding is not supported by the editor reader".into())
            }
        };
    save_editor_file_lifecycle_sync(
        path,
        content,
        snapshot.encoding,
        snapshot.line_ending,
        snapshot.bom,
        &snapshot.identity,
        StorageSaveIntent::Normal,
    )
    .map(|saved| saved.identity.modified_ms)
    .map_err(|error| error.to_string())
}

pub async fn fs_file_exists(path: String) -> bool {
    tokio::task::spawn_blocking(move || {
        let p = if path == "~" {
            dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(&path))
        } else if let Some(stripped) = path.strip_prefix("~/") {
            dirs::home_dir()
                .map(|mut h| {
                    h.push(stripped);
                    h
                })
                .unwrap_or_else(|| std::path::PathBuf::from(&path))
        } else {
            std::path::PathBuf::from(&path)
        };
        p.exists()
    })
    .await
    .unwrap_or(false)
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "labonair-{label}-{}-{}.txt",
            std::process::id(),
            NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn save_keeps_selected_line_ending_for_single_line_files() {
        let path = temp_path("single-line-eol");
        std::fs::write(&path, b"plain text").expect("create test input");
        let path_text = path.to_string_lossy().to_string();
        let loaded = load_editor_file_lifecycle_sync(&path_text, None).expect("load test input");
        let EditorLifecycleLoad::Text { snapshot } = loaded else {
            panic!("test input should load as text");
        };

        let saved = save_editor_file_lifecycle_sync(
            &path_text,
            "plain text",
            StorageEncoding::Utf8,
            StorageLineEnding::CrLf,
            StorageBom::None,
            &snapshot.identity,
            StorageSaveIntent::Normal,
        )
        .expect("save test input");

        assert_eq!(saved.line_ending, StorageLineEnding::CrLf);
        assert_eq!(
            std::fs::read(&path).expect("read saved input"),
            b"plain text"
        );
        std::fs::remove_file(path).expect("remove test input");
    }

    #[test]
    fn explicit_overwrite_replaces_external_file_changes() {
        let path = temp_path("overwrite-external");
        std::fs::write(&path, b"original").expect("create overwrite test input");
        let path_text = path.to_string_lossy().to_string();
        let loaded = load_editor_file_lifecycle_sync(&path_text, None).expect("load test input");
        let EditorLifecycleLoad::Text { snapshot } = loaded else {
            panic!("test input should load as text");
        };
        std::fs::write(&path, b"external edit").expect("write external change");

        let saved = save_editor_file_lifecycle_sync(
            &path_text,
            "local edit",
            StorageEncoding::Utf8,
            StorageLineEnding::Lf,
            StorageBom::None,
            &snapshot.identity,
            StorageSaveIntent::OverwriteExternal,
        )
        .expect("explicit overwrite should replace the changed file");

        assert_eq!(saved.text, "local edit");
        assert_eq!(
            std::fs::read(&path).expect("read overwritten file"),
            b"local edit"
        );
        std::fs::remove_file(path).expect("remove test input");
    }

    #[test]
    fn loads_utf16_bom_and_detects_its_line_endings() {
        let path = temp_path("utf16-load");
        let mut bytes = vec![0xff, 0xfe];
        for unit in "alpha\r\nbeta".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        std::fs::write(&path, bytes).expect("create UTF-16 test input");

        let loaded = load_editor_file_lifecycle_sync(&path.to_string_lossy(), None)
            .expect("load UTF-16 test input");
        let EditorLifecycleLoad::Text { snapshot } = loaded else {
            panic!("UTF-16 text should load");
        };
        assert_eq!(snapshot.text, "alpha\nbeta");
        assert_eq!(snapshot.encoding, StorageEncoding::Utf16Le);
        assert_eq!(snapshot.bom, StorageBom::Utf16Le);
        assert_eq!(snapshot.line_ending, StorageLineEnding::CrLf);

        std::fs::remove_file(path).expect("remove UTF-16 test input");
    }

    #[test]
    fn auto_loads_windows_1252_and_explicit_reload_uses_selected_encoding() {
        let path = temp_path("legacy-encoding");
        std::fs::write(&path, b"caf\xe9").expect("create Windows-1252 test input");
        let path_text = path.to_string_lossy().to_string();

        let loaded = load_editor_file_lifecycle_sync(&path_text, None).expect("auto-load legacy");
        let EditorLifecycleLoad::Text { snapshot } = loaded else {
            panic!("Windows-1252 text should load");
        };
        assert_eq!(snapshot.text, "café");
        assert_eq!(snapshot.encoding, StorageEncoding::Windows1252);

        let reloaded = load_editor_file_lifecycle_with_encoding_sync(
            &path_text,
            None,
            Some(StorageEncoding::Utf8),
        )
        .expect("explicit UTF-8 load result");
        assert!(matches!(reloaded, EditorLifecycleLoad::InvalidUtf8 { .. }));

        std::fs::remove_file(path).expect("remove Windows-1252 test input");
    }

    #[test]
    fn forced_shift_jis_load_decodes_multibyte_text() {
        let path = temp_path("shift-jis");
        let (encoded, _, had_errors) = SHIFT_JIS.encode("日本語");
        assert!(!had_errors);
        std::fs::write(&path, encoded.as_ref()).expect("create Shift_JIS test input");
        let loaded = load_editor_file_lifecycle_with_encoding_sync(
            &path.to_string_lossy(),
            None,
            Some(StorageEncoding::ShiftJis),
        )
        .expect("load Shift_JIS input");
        let EditorLifecycleLoad::Text { snapshot } = loaded else {
            panic!("Shift_JIS text should load");
        };
        assert_eq!(snapshot.text, "日本語");
        assert_eq!(snapshot.encoding, StorageEncoding::ShiftJis);

        std::fs::remove_file(path).expect("remove Shift_JIS test input");
    }

    #[test]
    fn saves_utf16_with_bom_and_rejects_unrepresentable_text() {
        let path = temp_path("utf16-save");
        std::fs::write(&path, b"a\nb").expect("create save test input");
        let path_text = path.to_string_lossy().to_string();
        let loaded = load_editor_file_lifecycle_sync(&path_text, None).expect("load save input");
        let EditorLifecycleLoad::Text { snapshot } = loaded else {
            panic!("save test input should load as text");
        };

        let saved = save_editor_file_lifecycle_sync(
            &path_text,
            "a\nb",
            StorageEncoding::Utf16Le,
            StorageLineEnding::CrLf,
            StorageBom::Utf16Le,
            &snapshot.identity,
            StorageSaveIntent::Normal,
        )
        .expect("save as UTF-16 LE");
        assert_eq!(saved.encoding, StorageEncoding::Utf16Le);
        assert_eq!(saved.line_ending, StorageLineEnding::CrLf);
        assert_eq!(saved.bom, StorageBom::Utf16Le);
        assert_eq!(
            std::fs::read(&path).expect("read UTF-16 output"),
            [
                vec![0xff, 0xfe],
                "a\r\nb".encode_utf16().flat_map(u16::to_le_bytes).collect()
            ]
            .concat()
        );
        assert!(matches!(
            encode_text("終", StorageEncoding::Windows1252, &path),
            Err(EditorFileError::UnrepresentableText { .. })
        ));

        std::fs::remove_file(path).expect("remove save test input");
    }
}
