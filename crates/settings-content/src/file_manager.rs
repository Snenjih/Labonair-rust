//! `file_manager` area — Explorer / SFTP browser behaviour.

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, crate::MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct FileManagerContent {
    pub sftp_show_hidden_files: Option<bool>,
    pub sftp_show_up_folder: Option<bool>,
    pub explorer_show_hidden_by_default: Option<bool>,
    /// Draw thin vertical indent guides at each Explorer tree depth.
    pub explorer_indent_guides: Option<bool>,
    /// Pin the current scroll position's ancestor directory rows above the
    /// Explorer list.
    pub explorer_sticky_ancestors: Option<bool>,
    /// Scroll the Explorer to (and mark) the file open in the active editor.
    pub explorer_auto_reveal_active_file: Option<bool>,
    /// Collapse single-child directory chains (`a/b/c`) into one compressed
    /// Explorer row.
    pub explorer_fold_single_child_dirs: Option<bool>,
    /// Show Git status decorations (tint + letter) on Explorer rows.
    pub explorer_git_decorations: Option<bool>,
    /// Source-Control panel change list: `true` = directory tree, `false` =
    /// flat status buckets (Zed-parity Phase 4).
    pub scm_file_tree: Option<bool>,
    pub sftp_column_size: Option<bool>,
    pub sftp_column_modified: Option<bool>,
    pub sftp_column_permissions: Option<bool>,
    pub sftp_column_type: Option<bool>,
    pub sftp_remote_edit_show_transfers: Option<bool>,
    pub sftp_max_remote_file_size_mb: Option<u32>,
    pub sftp_font_size: Option<u32>,
    pub sftp_max_concurrent_transfers: Option<u32>,
    /// `"ask"` | `"overwrite"` | `"skip"`.
    pub sftp_default_conflict_resolution: Option<String>,
    pub sftp_chunk_size_kb: Option<u32>,
    /// `"ask"` | `"skip"` | `"abort"`.
    pub sftp_on_folder_file_error: Option<String>,
}

impl FileManagerContent {
    pub fn defaults() -> Self {
        Self {
            sftp_show_hidden_files: Some(false),
            sftp_show_up_folder: Some(true),
            explorer_show_hidden_by_default: Some(false),
            explorer_indent_guides: Some(true),
            explorer_sticky_ancestors: Some(true),
            explorer_auto_reveal_active_file: Some(false),
            explorer_fold_single_child_dirs: Some(false),
            explorer_git_decorations: Some(true),
            scm_file_tree: Some(false),
            sftp_column_size: Some(true),
            sftp_column_modified: Some(true),
            sftp_column_permissions: Some(true),
            sftp_column_type: Some(false),
            sftp_remote_edit_show_transfers: Some(true),
            sftp_max_remote_file_size_mb: Some(5),
            sftp_font_size: Some(13),
            sftp_max_concurrent_transfers: Some(2),
            sftp_default_conflict_resolution: Some("ask".to_string()),
            sftp_chunk_size_kb: Some(64),
            sftp_on_folder_file_error: Some("ask".to_string()),
        }
    }
}
