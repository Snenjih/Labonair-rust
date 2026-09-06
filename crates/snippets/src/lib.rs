//! Saved command snippets and snippet groups.
//!
//! Persistence and domain data live here. Execution remains a separate
//! backend adapter until local/SSH process ownership is extracted into the
//! snippet capability as well.

pub mod store;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommandSnippet {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub target: String,
    pub host_id: Option<String>,
    pub default_exec_mode: String,
    pub working_dir: Option<String>,
    pub group_id: Option<String>,
    pub tags: Option<String>,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnippetGroup {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetReorderItem {
    pub id: String,
    pub sort_order: i64,
}
