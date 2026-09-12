//! Versioned, editor-owned view/session persistence.
//!
//! Workspace owns the outer tab/session envelope. This module owns the data
//! inside one editor tab: view state, selections, folds, search, split
//! metadata, and an explicitly recoverable unsaved buffer. The codec is pure
//! so filesystem adapters can load and save bytes off the GPUI thread.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{EditorSplitTree, FoldRange, SearchQuery, SelectionSet};

pub const EDITOR_SESSION_SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EditorSessionId(String);

impl EditorSessionId {
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        (!value.is_empty() && value.len() <= 256).then_some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for EditorSessionId {
    fn default() -> Self {
        Self("editor".into())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorViewSession {
    pub group_id: u64,
    #[serde(default)]
    pub scroll_top: usize,
    #[serde(default)]
    pub scroll_left: usize,
    #[serde(default)]
    pub folds: Vec<FoldRange>,
    #[serde(default)]
    pub selections: SelectionSet,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsavedBufferRecovery {
    pub path: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorSessionSnapshot {
    pub schema_version: u32,
    pub session_id: EditorSessionId,
    pub file_path: Option<String>,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub split_tree: EditorSplitTree,
    #[serde(default)]
    pub active_group: u64,
    #[serde(default)]
    pub views: Vec<EditorViewSession>,
    #[serde(default)]
    pub search: Option<SearchQuery>,
    #[serde(default)]
    pub recovery: Option<UnsavedBufferRecovery>,
}

impl EditorSessionSnapshot {
    pub fn new(id: EditorSessionId, file_path: Option<String>) -> Self {
        let split_tree = EditorSplitTree::new(crate::EditorGroupId::new(1));
        Self {
            schema_version: EDITOR_SESSION_SCHEMA_VERSION,
            session_id: id,
            file_path,
            language: String::new(),
            active_group: 1,
            views: vec![EditorViewSession {
                group_id: 1,
                scroll_top: 0,
                scroll_left: 0,
                folds: Vec::new(),
                selections: SelectionSet::default(),
            }],
            split_tree,
            search: None,
            recovery: None,
        }
    }

    pub fn normalized(mut self) -> Result<Self, PersistenceError> {
        if self.schema_version != EDITOR_SESSION_SCHEMA_VERSION {
            return Err(PersistenceError::UnsupportedVersion(self.schema_version));
        }
        if self.session_id.as_str().is_empty() {
            return Err(PersistenceError::Invalid("empty session id".into()));
        }
        if self.split_tree.group_count() == 0
            || !self
                .split_tree
                .contains(crate::EditorGroupId::new(self.active_group))
        {
            self.split_tree = EditorSplitTree::new(crate::EditorGroupId::new(1));
            self.active_group = 1;
        }
        self.split_tree
            .validate()
            .map_err(|error| PersistenceError::Invalid((*error).into()))?;
        let groups = self.split_tree.groups();
        self.views
            .retain(|view| groups.contains(&crate::EditorGroupId::new(view.group_id)));
        if self.views.is_empty() {
            self.views.push(EditorViewSession {
                group_id: self.active_group,
                scroll_top: 0,
                scroll_left: 0,
                folds: Vec::new(),
                selections: SelectionSet::default(),
            });
        }
        for view in &mut self.views {
            view.folds.retain(|fold| fold.start_line < fold.end_line);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistenceError {
    Invalid(String),
    UnsupportedVersion(u32),
    Corrupt(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => write!(f, "invalid editor session: {message}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported editor session schema version {version}")
            }
            Self::Corrupt(message) => write!(f, "corrupt editor session: {message}"),
        }
    }
}

impl std::error::Error for PersistenceError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorPersistenceEvent {
    Loaded,
    Migrated { from_version: u32 },
    Corrupt { reason: String },
    Saved,
}

pub trait EditorSessionStore {
    fn load(&self, id: &EditorSessionId)
        -> Result<Option<EditorSessionSnapshot>, PersistenceError>;
    fn save(
        &mut self,
        snapshot: &EditorSessionSnapshot,
    ) -> Result<EditorPersistenceEvent, PersistenceError>;
}

pub fn decode_session(
    bytes: &[u8],
) -> Result<(EditorSessionSnapshot, EditorPersistenceEvent), PersistenceError> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| PersistenceError::Corrupt(error.to_string()))?;
    let version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1) as u32;
    if version == 1 {
        let legacy: LegacySession = serde_json::from_value(value)
            .map_err(|error| PersistenceError::Corrupt(error.to_string()))?;
        let id = EditorSessionId::new(legacy.session_id.unwrap_or_else(|| "editor".into()))
            .ok_or_else(|| PersistenceError::Invalid("empty session id".into()))?;
        return Ok((
            EditorSessionSnapshot::new(id, legacy.file_path),
            EditorPersistenceEvent::Migrated { from_version: 1 },
        ));
    }
    let snapshot: EditorSessionSnapshot = serde_json::from_value(value)
        .map_err(|error| PersistenceError::Corrupt(error.to_string()))?;
    Ok((snapshot.normalized()?, EditorPersistenceEvent::Loaded))
}

pub fn encode_session(snapshot: &EditorSessionSnapshot) -> Result<Vec<u8>, PersistenceError> {
    let snapshot = snapshot.clone().normalized()?;
    serde_json::to_vec_pretty(&snapshot)
        .map_err(|error| PersistenceError::Invalid(error.to_string()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacySession {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    file_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_session_round_trips_view_state() {
        let id = EditorSessionId::new("tab-1").expect("valid id");
        let mut snapshot = EditorSessionSnapshot::new(id, Some("/tmp/main.rs".into()));
        snapshot.views[0].scroll_top = 42;
        snapshot.views[0].folds = vec![FoldRange::new(2, 8).expect("fold")];
        let encoded = encode_session(&snapshot).expect("encode");
        let (decoded, event) = decode_session(&encoded).expect("decode");
        assert_eq!(decoded, snapshot);
        assert_eq!(event, EditorPersistenceEvent::Loaded);
    }

    #[test]
    fn v1_path_only_records_migrate_to_one_leaf() {
        let (snapshot, event) =
            decode_session(br#"{"version":1,"filePath":"/tmp/a.rs"}"#).expect("legacy record");
        assert_eq!(snapshot.file_path.as_deref(), Some("/tmp/a.rs"));
        assert_eq!(snapshot.split_tree.group_count(), 1);
        assert_eq!(event, EditorPersistenceEvent::Migrated { from_version: 1 });
    }

    #[test]
    fn corrupt_records_are_rejected_without_a_fallback_write() {
        let error = decode_session(b"not json").expect_err("corruption must be visible");
        assert!(matches!(error, PersistenceError::Corrupt(_)));
    }

    #[test]
    fn unsupported_versions_are_rejected() {
        let json = format!(
            r#"{{"schemaVersion":{},"sessionId":"x"}}"#,
            EDITOR_SESSION_SCHEMA_VERSION + 1
        );
        assert!(matches!(
            decode_session(json.as_bytes()),
            Err(PersistenceError::UnsupportedVersion(_))
        ));
    }
}
