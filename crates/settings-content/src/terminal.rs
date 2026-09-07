//! `terminal` area.

use crate::MergeFrom;
use serde::{Deserialize, Serialize};

/// Terminal cursor shape.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    #[default]
    Block,
    Underline,
    Bar,
}

impl MergeFrom for CursorStyle {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct TerminalContent {
    pub terminal_shell: Option<String>,
    pub terminal_font_family: Option<String>,
    pub terminal_font_size: Option<u32>,
    pub terminal_scrollback: Option<u32>,
    /// Rows of scrollback persisted per pane on quit and replayed on the next
    /// launch. `0` = persist everything the buffer holds.
    pub session_scrollback_lines: Option<u32>,
    /// Per-file ceiling for a persisted scrollback, in MB.
    pub scrollback_max_size_mb: Option<u32>,
    /// Days a persisted scrollback file is kept before cleanup deletes it.
    /// `0` = keep until the pane/session goes away.
    pub scrollback_retention_days: Option<u32>,
    pub terminal_cursor_style: Option<CursorStyle>,
    pub terminal_cursor_blink: Option<bool>,
    pub terminal_copy_on_select: Option<bool>,
    pub terminal_right_click_pastes: Option<bool>,
    pub terminal_bell: Option<bool>,
    /// Terminal background opacity in percent (100 = fully opaque).
    pub terminal_opacity: Option<u32>,
}

impl TerminalContent {
    pub fn defaults() -> Self {
        let mono = "\"Lilex\", SFMono-Regular, Menlo, monospace".to_string();
        Self {
            terminal_shell: Some(String::new()),
            terminal_font_family: Some(mono),
            terminal_font_size: Some(15),
            terminal_scrollback: Some(5_000),
            session_scrollback_lines: Some(1_000),
            scrollback_max_size_mb: Some(10),
            scrollback_retention_days: Some(0),
            terminal_cursor_style: Some(CursorStyle::Bar),
            terminal_cursor_blink: Some(true),
            terminal_copy_on_select: Some(false),
            terminal_right_click_pastes: Some(false),
            terminal_bell: Some(false),
            terminal_opacity: Some(100),
        }
    }
}
