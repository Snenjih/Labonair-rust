//! `file_manager` area — Explorer / SFTP browser behaviour.

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, crate::MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct FileManagerContent {
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
}

impl FileManagerContent {
    pub fn defaults() -> Self {
        Self {
            explorer_show_hidden_by_default: Some(false),
            explorer_indent_guides: Some(true),
            explorer_sticky_ancestors: Some(true),
            explorer_auto_reveal_active_file: Some(false),
            explorer_fold_single_child_dirs: Some(false),
            explorer_git_decorations: Some(true),
            scm_file_tree: Some(false),
        }
    }
}
