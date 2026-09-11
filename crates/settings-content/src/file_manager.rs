//! `file_manager` area — Explorer / SFTP browser behaviour.

use crate::MergeFrom;
use serde::{Deserialize, Serialize};

/// One optional metadata column the SFTP browser can show next to the file
/// name. `Name` is always pinned first and is not part of this list.
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum SftpColumn {
    Size,
    Modified,
    Created,
    Permissions,
    Type,
}

impl MergeFrom for SftpColumn {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

impl SftpColumn {
    /// Every column in the canonical default order — the order a fresh install
    /// (or a `sftpColumns` value that omits some columns) falls back to.
    pub const ALL: [SftpColumn; 5] = [
        SftpColumn::Size,
        SftpColumn::Modified,
        SftpColumn::Created,
        SftpColumn::Permissions,
        SftpColumn::Type,
    ];

    /// Uppercase header label shown in the column head row.
    pub fn header(self) -> &'static str {
        match self {
            SftpColumn::Size => "SIZE",
            SftpColumn::Modified => "MODIFIED",
            SftpColumn::Created => "CREATED",
            SftpColumn::Permissions => "PERMS",
            SftpColumn::Type => "TYPE",
        }
    }

    /// Human label for the Settings ordered-column editor.
    pub fn label(self) -> &'static str {
        match self {
            SftpColumn::Size => "Size",
            SftpColumn::Modified => "Last modified",
            SftpColumn::Created => "Created",
            SftpColumn::Permissions => "Permissions",
            SftpColumn::Type => "Type",
        }
    }

    /// Serde token (matches `rename_all = "kebab-case"`).
    pub fn token(self) -> &'static str {
        match self {
            SftpColumn::Size => "size",
            SftpColumn::Modified => "modified",
            SftpColumn::Created => "created",
            SftpColumn::Permissions => "permissions",
            SftpColumn::Type => "type",
        }
    }

    pub fn from_token(tok: &str) -> Option<Self> {
        SftpColumn::ALL.into_iter().find(|c| c.token() == tok)
    }
}

/// Default visible SFTP columns, in order — matches the historical
/// `SIZE MODIFIED · PERMS` layout of the reference browser.
pub fn default_sftp_columns() -> Vec<SftpColumn> {
    vec![
        SftpColumn::Size,
        SftpColumn::Modified,
        SftpColumn::Permissions,
    ]
}

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

    // ── SFTP dual-pane browser ─────────────────────────────────────────────
    /// Start each SFTP pane with dotfiles visible.
    pub sftp_show_hidden_files: Option<bool>,
    /// Show a `..` row at the top of every non-root directory to walk up.
    pub sftp_show_up_folder: Option<bool>,
    /// Alternating row background ("zebra") in the SFTP file list.
    pub sftp_zebra_striping: Option<bool>,
    /// Show relative modified/created times (`13d ago`) instead of absolute
    /// timestamps in the SFTP file list.
    pub sftp_relative_times: Option<bool>,
    /// Which metadata columns the SFTP browser shows, in display order. An
    /// empty list hides every optional column (name only). Unknown tokens are
    /// ignored; duplicates are de-duplicated on read.
    pub sftp_columns: Option<Vec<SftpColumn>>,
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
            sftp_show_hidden_files: Some(false),
            sftp_show_up_folder: Some(true),
            sftp_zebra_striping: Some(true),
            sftp_relative_times: Some(true),
            sftp_columns: Some(default_sftp_columns()),
        }
    }
}
