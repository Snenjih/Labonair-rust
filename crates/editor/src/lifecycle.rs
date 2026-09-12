//! UI-free file lifecycle contract for the editor capability.
//!
//! The editor owns decisions about dirty buffers, external changes, reloads,
//! and safe saves. Storage adapters only translate operating-system results
//! into these typed values; they do not decide whether a user's buffer may be
//! replaced or overwritten.

use std::fmt;
use std::path::{Path, PathBuf};

/// The line ending used when serializing a text file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineEnding {
    Lf,
    CrLf,
    Cr,
    /// The source contained more than one line-ending style. The editor uses
    /// LF internally and writes the dominant style until a future exact
    /// per-line EOL map is introduced.
    Mixed,
}

impl LineEnding {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::CrLf => "CRLF",
            Self::Cr => "CR",
            Self::Mixed => "Mixed",
        }
    }

    pub const fn sequence(self) -> &'static str {
        match self {
            Self::Lf | Self::Mixed => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }
}

/// Encoding support advertised by the editor boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Encoding {
    Utf8,
}

impl Encoding {
    pub const fn as_str(self) -> &'static str {
        "UTF-8"
    }
}

/// Byte-order mark metadata. Only the UTF-8 BOM is accepted today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bom {
    None,
    Utf8,
}

/// Capabilities discovered for the current file identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileCapabilities {
    pub can_read: bool,
    pub can_edit: bool,
    pub can_save: bool,
}

impl FileCapabilities {
    pub const fn editable() -> Self {
        Self {
            can_read: true,
            can_edit: true,
            can_save: true,
        }
    }

    pub const fn read_only() -> Self {
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

/// Content identity used to protect saves from external changes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    pub path: PathBuf,
    pub size: u64,
    pub modified_ms: u64,
    pub content_hash: u64,
}

impl FileIdentity {
    pub fn new(path: impl Into<PathBuf>, size: u64, modified_ms: u64, content_hash: u64) -> Self {
        Self {
            path: path.into(),
            size,
            modified_ms,
            content_hash,
        }
    }

    pub fn from_bytes(path: impl Into<PathBuf>, modified_ms: u64, bytes: &[u8]) -> Self {
        Self::new(path, bytes.len() as u64, modified_ms, content_hash(bytes))
    }
}

/// Immutable text and disk metadata accepted by the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSnapshot {
    pub identity: FileIdentity,
    /// Canonical UTF-8 text with LF line separators and no BOM.
    pub text: String,
    pub line_ending: LineEnding,
    pub encoding: Encoding,
    pub bom: Bom,
    pub capabilities: FileCapabilities,
}

/// File metadata that may be persisted with a session without serializing
/// document contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSessionMetadata {
    pub identity: FileIdentity,
    pub line_ending: LineEnding,
    pub encoding: Encoding,
    pub bom: Bom,
}

impl FileSnapshot {
    pub fn new(
        identity: FileIdentity,
        text: String,
        line_ending: LineEnding,
        encoding: Encoding,
        bom: Bom,
        capabilities: FileCapabilities,
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

/// The explicit lifecycle state of an open file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    New,
    Loading,
    Clean,
    Dirty,
    ReloadNeeded,
    Conflict,
    ReadOnly,
    Missing,
    Binary,
    TooLarge,
    Error(String),
}

impl FileState {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::New => "New",
            Self::Loading => "Loading",
            Self::Clean => "Clean",
            Self::Dirty => "Dirty",
            Self::ReloadNeeded => "ReloadNeeded",
            Self::Conflict => "Conflict",
            Self::ReadOnly => "ReadOnly",
            Self::Missing => "Missing",
            Self::Binary => "Binary",
            Self::TooLarge => "TooLarge",
            Self::Error(_) => "Error",
        }
    }
}

/// User intent required for a save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveIntent {
    /// Normal save. Refuses to overwrite a changed disk identity.
    Normal,
    /// Explicit user choice to overwrite the current external identity.
    OverwriteExternal,
}

/// User decision required when a disk change meets a dirty buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadDecision {
    Automatic,
    DiscardBuffer,
    KeepBuffer,
    Cancel,
}

/// Typed lifecycle notifications emitted by the editor owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileLifecycleEvent {
    DirtyChanged { dirty: bool },
    ReloadNeeded { identity: FileIdentity },
    ConflictDetected { identity: FileIdentity },
    Saved { identity: FileIdentity },
    Reloaded { identity: FileIdentity },
    ReadOnlyChanged { can_save: bool },
}

/// Errors returned when a lifecycle operation cannot be performed safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileLifecycleError {
    InvalidTransition {
        from: String,
        to: String,
    },
    ReadOnly,
    Missing,
    UnsupportedState(String),
    ExternalChange {
        expected: FileIdentity,
        actual: FileIdentity,
    },
    ReloadRequiresDecision,
    InvalidDecision,
}

impl fmt::Display for FileLifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid file lifecycle transition {from} -> {to}")
            }
            Self::ReadOnly => write!(f, "file is read-only"),
            Self::Missing => write!(f, "file is missing"),
            Self::UnsupportedState(state) => {
                write!(f, "file state does not support this operation: {state}")
            }
            Self::ExternalChange { .. } => write!(f, "file changed on disk since it was loaded"),
            Self::ReloadRequiresDecision => write!(f, "reloading would replace unsaved changes"),
            Self::InvalidDecision => {
                write!(f, "reload decision is not valid for the current file state")
            }
        }
    }
}

impl std::error::Error for FileLifecycleError {}

/// Editor-owned lifecycle state. It stores the last accepted disk snapshot,
/// never filesystem handles or UI state.
#[derive(Debug, Clone)]
pub struct FileLifecycle {
    state: FileState,
    accepted: Option<FileSnapshot>,
    observed: Option<FileIdentity>,
}

impl Default for FileLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLifecycle {
    pub fn new() -> Self {
        Self {
            state: FileState::New,
            accepted: None,
            observed: None,
        }
    }

    pub fn state(&self) -> &FileState {
        &self.state
    }

    pub fn accepted_snapshot(&self) -> Option<&FileSnapshot> {
        self.accepted.as_ref()
    }

    pub fn session_metadata(&self) -> Option<FileSessionMetadata> {
        self.accepted.as_ref().map(|snapshot| FileSessionMetadata {
            identity: snapshot.identity.clone(),
            line_ending: snapshot.line_ending,
            encoding: snapshot.encoding,
            bom: snapshot.bom,
        })
    }

    pub fn path(&self) -> Option<&Path> {
        self.accepted
            .as_ref()
            .map(|snapshot| snapshot.identity.path.as_path())
    }

    pub fn is_dirty(&self, current_text: &str) -> bool {
        self.accepted
            .as_ref()
            .is_some_and(|snapshot| snapshot.text != current_text)
    }

    pub fn can_edit(&self) -> bool {
        match &self.state {
            FileState::ReadOnly
            | FileState::Missing
            | FileState::Binary
            | FileState::TooLarge
            | FileState::Loading
            | FileState::Error(_) => false,
            _ => self
                .accepted
                .as_ref()
                .is_none_or(|snapshot| snapshot.capabilities.can_edit),
        }
    }

    pub fn can_save(&self) -> bool {
        matches!(
            self.state,
            FileState::Clean | FileState::Dirty | FileState::Conflict
        ) && self
            .accepted
            .as_ref()
            .is_some_and(|snapshot| snapshot.capabilities.can_save)
    }

    pub fn begin_loading(&mut self) -> Result<(), FileLifecycleError> {
        self.transition_to(FileState::Loading)
    }

    /// Validate and apply a state transition without silently accepting an
    /// impossible lifecycle edge.
    pub fn transition_to(&mut self, next: FileState) -> Result<(), FileLifecycleError> {
        if self.state.name() == next.name() || can_transition(&self.state, &next) {
            self.state = next;
            Ok(())
        } else {
            Err(FileLifecycleError::InvalidTransition {
                from: self.state.name().to_string(),
                to: next.name().to_string(),
            })
        }
    }

    pub fn accept_loaded(
        &mut self,
        snapshot: FileSnapshot,
    ) -> Result<FileLifecycleEvent, FileLifecycleError> {
        if !matches!(
            self.state,
            FileState::New | FileState::Loading | FileState::ReloadNeeded | FileState::Conflict
        ) {
            return Err(FileLifecycleError::UnsupportedState(
                self.state.name().to_string(),
            ));
        }
        let read_only = !snapshot.capabilities.can_edit;
        self.transition_to(if read_only {
            FileState::ReadOnly
        } else {
            FileState::Clean
        })?;
        let identity = snapshot.identity.clone();
        self.accepted = Some(snapshot);
        self.observed = None;
        Ok(FileLifecycleEvent::Reloaded { identity })
    }

    pub fn accept_uneditable(
        &mut self,
        state: FileState,
        identity: FileIdentity,
        capabilities: FileCapabilities,
    ) -> Result<(), FileLifecycleError> {
        if !matches!(
            state,
            FileState::Binary | FileState::TooLarge | FileState::Error(_)
        ) {
            return Err(FileLifecycleError::InvalidDecision);
        }
        self.transition_to(state)?;
        self.accepted = Some(FileSnapshot::new(
            identity,
            String::new(),
            LineEnding::Lf,
            Encoding::Utf8,
            Bom::None,
            capabilities,
        ));
        self.observed = None;
        Ok(())
    }

    pub fn fail(&mut self, state: FileState) -> Result<(), FileLifecycleError> {
        if !matches!(state, FileState::Missing | FileState::Error(_)) {
            return Err(FileLifecycleError::InvalidDecision);
        }
        self.transition_to(state)
    }

    pub fn mark_dirty(&mut self) -> Result<FileLifecycleEvent, FileLifecycleError> {
        if matches!(self.state, FileState::ReadOnly) {
            return Err(FileLifecycleError::ReadOnly);
        }
        if matches!(
            self.state,
            FileState::Missing | FileState::Binary | FileState::TooLarge
        ) {
            return Err(FileLifecycleError::UnsupportedState(
                self.state.name().to_string(),
            ));
        }
        if !matches!(self.state, FileState::Dirty) {
            self.transition_to(FileState::Dirty)?;
        }
        Ok(FileLifecycleEvent::DirtyChanged { dirty: true })
    }

    /// Reconcile the lifecycle state with the current buffer after undo/redo.
    /// External conflicts remain conflicts even when the buffer happens to
    /// equal the old disk baseline.
    pub fn sync_buffer(
        &mut self,
        current_text: &str,
    ) -> Result<Option<FileLifecycleEvent>, FileLifecycleError> {
        if matches!(self.state, FileState::Conflict | FileState::ReloadNeeded) {
            return Ok(None);
        }
        let dirty = self.is_dirty(current_text);
        if dirty {
            if matches!(self.state, FileState::Clean) {
                return self.mark_dirty().map(Some);
            }
        } else if matches!(self.state, FileState::Dirty) {
            let next = if self
                .accepted
                .as_ref()
                .is_some_and(|snapshot| !snapshot.capabilities.can_edit)
            {
                FileState::ReadOnly
            } else {
                FileState::Clean
            };
            self.transition_to(next)?;
            return Ok(Some(FileLifecycleEvent::DirtyChanged { dirty: false }));
        }
        Ok(None)
    }

    pub fn observe_external(
        &mut self,
        identity: Option<FileIdentity>,
    ) -> Result<Option<FileLifecycleEvent>, FileLifecycleError> {
        let Some(identity) = identity else {
            self.transition_to(FileState::Missing)?;
            self.observed = None;
            return Ok(None);
        };
        let Some(accepted) = self.accepted.as_ref() else {
            return Ok(None);
        };
        if accepted.identity == identity {
            return Ok(None);
        }
        self.observed = Some(identity.clone());
        if matches!(self.state, FileState::Dirty | FileState::Conflict) {
            self.transition_to(FileState::Conflict)?;
            Ok(Some(FileLifecycleEvent::ConflictDetected { identity }))
        } else {
            self.transition_to(FileState::ReloadNeeded)?;
            Ok(Some(FileLifecycleEvent::ReloadNeeded { identity }))
        }
    }

    pub fn keep_buffer(&mut self) -> Result<FileLifecycleEvent, FileLifecycleError> {
        if !matches!(self.state, FileState::Conflict) {
            return Err(FileLifecycleError::InvalidDecision);
        }
        self.transition_to(FileState::Dirty)?;
        Ok(FileLifecycleEvent::DirtyChanged { dirty: true })
    }

    pub fn prepare_save(
        &self,
        current_identity: &FileIdentity,
        intent: SaveIntent,
    ) -> Result<(), FileLifecycleError> {
        let Some(accepted) = self.accepted.as_ref() else {
            return Err(FileLifecycleError::Missing);
        };
        if !accepted.capabilities.can_save {
            return Err(FileLifecycleError::ReadOnly);
        }
        if matches!(
            self.state,
            FileState::Missing
                | FileState::Binary
                | FileState::TooLarge
                | FileState::Loading
                | FileState::Error(_)
                | FileState::ReadOnly
                | FileState::New
                | FileState::ReloadNeeded
        ) {
            return Err(FileLifecycleError::UnsupportedState(
                self.state.name().to_string(),
            ));
        }
        if intent == SaveIntent::Normal && accepted.identity != *current_identity {
            return Err(FileLifecycleError::ExternalChange {
                expected: accepted.identity.clone(),
                actual: current_identity.clone(),
            });
        }
        Ok(())
    }

    pub fn accept_saved(
        &mut self,
        snapshot: FileSnapshot,
    ) -> Result<FileLifecycleEvent, FileLifecycleError> {
        if !matches!(
            self.state,
            FileState::Dirty | FileState::Conflict | FileState::Clean | FileState::ReadOnly
        ) {
            return Err(FileLifecycleError::UnsupportedState(
                self.state.name().to_string(),
            ));
        }
        let read_only = !snapshot.capabilities.can_edit;
        self.transition_to(if read_only {
            FileState::ReadOnly
        } else {
            FileState::Clean
        })?;
        let identity = snapshot.identity.clone();
        self.accepted = Some(snapshot);
        self.observed = None;
        Ok(FileLifecycleEvent::Saved { identity })
    }

    pub fn reload_decision(
        &mut self,
        snapshot: FileSnapshot,
        decision: ReloadDecision,
    ) -> Result<FileLifecycleEvent, FileLifecycleError> {
        match (&self.state, decision) {
            (FileState::ReloadNeeded, ReloadDecision::Automatic)
            | (FileState::ReloadNeeded, ReloadDecision::DiscardBuffer)
            | (FileState::Conflict, ReloadDecision::DiscardBuffer) => self.accept_loaded(snapshot),
            (FileState::Conflict, ReloadDecision::KeepBuffer) => self.keep_buffer(),
            (_, ReloadDecision::Cancel) => Err(FileLifecycleError::InvalidDecision),
            _ => Err(FileLifecycleError::ReloadRequiresDecision),
        }
    }

    pub fn observed_identity(&self) -> Option<&FileIdentity> {
        self.observed.as_ref()
    }
}

fn can_transition(from: &FileState, to: &FileState) -> bool {
    use FileState::*;
    match from {
        New => matches!(to, Loading | Dirty | Missing | Error(_)),
        Loading => matches!(
            to,
            Clean | ReadOnly | Missing | Binary | TooLarge | Error(_)
        ),
        Clean => matches!(
            to,
            Dirty | ReloadNeeded | ReadOnly | Missing | Loading | Error(_)
        ),
        Dirty => matches!(
            to,
            Clean | ReloadNeeded | Conflict | ReadOnly | Missing | Loading | Error(_)
        ),
        ReloadNeeded => matches!(
            to,
            Loading | Clean | ReadOnly | Dirty | Conflict | Missing | Error(_)
        ),
        Conflict => matches!(to, Loading | Clean | ReadOnly | Dirty | Missing | Error(_)),
        ReadOnly => matches!(to, Loading | Clean | ReloadNeeded | Missing | Error(_)),
        Missing => matches!(to, New | Loading | Clean | ReadOnly | Error(_)),
        Binary | TooLarge => matches!(to, Loading | Missing | Error(_)),
        Error(_) => matches!(to, New | Loading | Missing),
    }
}

/// Detect the dominant line-ending style in raw UTF-8 text.
pub fn detect_line_ending(text: &str) -> LineEnding {
    let bytes = text.as_bytes();
    let mut lf = 0usize;
    let mut crlf = 0usize;
    let mut cr = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => {
                crlf += 1;
                i += 2;
            }
            b'\r' => {
                cr += 1;
                i += 1;
            }
            b'\n' => {
                lf += 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    let styles = [lf > 0, crlf > 0, cr > 0]
        .into_iter()
        .filter(|value| *value)
        .count();
    if styles > 1 {
        return LineEnding::Mixed;
    }
    if crlf > 0 {
        LineEnding::CrLf
    } else if cr > 0 {
        LineEnding::Cr
    } else {
        LineEnding::Lf
    }
}

/// Normalize accepted text to the editor's LF-only internal representation.
pub fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Serialize canonical LF text using the selected line-ending style.
pub fn apply_line_ending(text: &str, ending: LineEnding) -> String {
    match ending {
        LineEnding::Lf | LineEnding::Mixed => text.to_string(),
        LineEnding::CrLf => text.replace('\n', "\r\n"),
        LineEnding::Cr => text.replace('\n', "\r"),
    }
}

/// Deterministic, non-cryptographic identity hash for conflict detection.
pub fn content_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(path: &str, hash: u64) -> FileIdentity {
        FileIdentity::new(path, 1, 1, hash)
    }

    fn snapshot(path: &str, hash: u64) -> FileSnapshot {
        FileSnapshot::new(
            identity(path, hash),
            "one\ntwo".into(),
            LineEnding::Lf,
            Encoding::Utf8,
            Bom::None,
            FileCapabilities::editable(),
        )
    }

    #[test]
    fn detects_and_normalizes_all_line_endings() {
        assert_eq!(detect_line_ending("a\nb"), LineEnding::Lf);
        assert_eq!(detect_line_ending("a\r\nb"), LineEnding::CrLf);
        assert_eq!(detect_line_ending("a\rb"), LineEnding::Cr);
        assert_eq!(detect_line_ending("a\r\nb\nc"), LineEnding::Mixed);
        assert_eq!(normalize_line_endings("a\r\nb\rc\n"), "a\nb\nc\n");
        assert_eq!(apply_line_ending("a\nb", LineEnding::CrLf), "a\r\nb");
    }

    #[test]
    fn transition_table_rejects_impossible_edges() {
        let mut lifecycle = FileLifecycle::new();
        assert!(lifecycle.transition_to(FileState::Dirty).is_ok());
        assert!(lifecycle.transition_to(FileState::Binary).is_err());
    }

    #[test]
    fn dirty_external_change_requires_explicit_reload_decision() {
        let mut lifecycle = FileLifecycle::new();
        lifecycle.begin_loading().unwrap();
        lifecycle.accept_loaded(snapshot("a.txt", 1)).unwrap();
        lifecycle.mark_dirty().unwrap();
        let event = lifecycle
            .observe_external(Some(identity("a.txt", 2)))
            .unwrap();
        assert!(matches!(
            event,
            Some(FileLifecycleEvent::ConflictDetected { .. })
        ));
        assert_eq!(lifecycle.state(), &FileState::Conflict);
        assert!(lifecycle
            .reload_decision(snapshot("a.txt", 2), ReloadDecision::Automatic)
            .is_err());
        lifecycle.keep_buffer().unwrap();
        assert_eq!(lifecycle.state(), &FileState::Dirty);
    }

    #[test]
    fn normal_save_refuses_changed_identity() {
        let mut lifecycle = FileLifecycle::new();
        lifecycle.begin_loading().unwrap();
        lifecycle.accept_loaded(snapshot("a.txt", 1)).unwrap();
        lifecycle.mark_dirty().unwrap();
        let error = lifecycle
            .prepare_save(&identity("a.txt", 2), SaveIntent::Normal)
            .unwrap_err();
        assert!(matches!(error, FileLifecycleError::ExternalChange { .. }));
        lifecycle
            .prepare_save(&identity("a.txt", 2), SaveIntent::OverwriteExternal)
            .unwrap();
    }

    #[test]
    fn terminal_error_state_is_never_saveable_even_with_write_permissions() {
        let mut lifecycle = FileLifecycle::new();
        lifecycle.begin_loading().unwrap();
        lifecycle
            .accept_uneditable(
                FileState::Error("unsupported encoding".into()),
                identity("a.txt", 1),
                FileCapabilities::editable(),
            )
            .unwrap();
        assert!(!lifecycle.can_save());
        assert!(matches!(
            lifecycle.prepare_save(&identity("a.txt", 1), SaveIntent::Normal),
            Err(FileLifecycleError::UnsupportedState(state)) if state == "Error"
        ));
    }
}
