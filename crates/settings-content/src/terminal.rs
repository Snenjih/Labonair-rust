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
    pub terminal_default_path: Option<String>,
    pub new_tab_inherits_cwd: Option<bool>,
    pub confirm_close_terminal_tab: Option<bool>,
    pub terminal_font_family: Option<String>,
    pub terminal_font_size: Option<u32>,
    /// `"normal"` | `"medium"` | `"bold"`.
    pub terminal_font_weight: Option<String>,
    pub terminal_letter_spacing: Option<f32>,
    pub terminal_line_height: Option<f32>,
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
    pub terminal_cursor_blink_interval: Option<u32>,
    pub terminal_copy_on_select: Option<bool>,
    pub terminal_right_click_pastes: Option<bool>,
    pub terminal_word_separator: Option<String>,
    pub terminal_scroll_sensitivity: Option<u32>,
    /// `"none"` | `"alt"` | `"ctrl"` | `"shift"`.
    pub terminal_fast_scroll_modifier: Option<String>,
    pub terminal_show_pane_header: Option<bool>,
    pub terminal_show_pane_footer: Option<bool>,
    pub terminal_use_webgl: Option<bool>,
    pub terminal_composer_enabled: Option<bool>,
    pub terminal_composer_history_popup: Option<bool>,
    pub terminal_composer_argument_completion: Option<bool>,
    pub terminal_blocks_enabled: Option<bool>,
    pub terminal_blocks_auto_collapse_on_alt_screen: Option<bool>,
    pub terminal_bell: Option<bool>,
    /// Terminal background opacity in percent (100 = fully opaque).
    pub terminal_opacity: Option<u32>,
}

impl TerminalContent {
    pub fn defaults() -> Self {
        let mono = "\"JetBrains Mono\", SFMono-Regular, Menlo, monospace".to_string();
        Self {
            terminal_shell: Some(String::new()),
            terminal_default_path: Some(String::new()),
            new_tab_inherits_cwd: Some(true),
            confirm_close_terminal_tab: Some(false),
            terminal_font_family: Some(mono),
            terminal_font_size: Some(14),
            terminal_font_weight: Some("normal".to_string()),
            terminal_letter_spacing: Some(0.0),
            terminal_line_height: Some(1.05),
            terminal_scrollback: Some(5_000),
            session_scrollback_lines: Some(1_000),
            scrollback_max_size_mb: Some(10),
            scrollback_retention_days: Some(0),
            terminal_cursor_style: Some(CursorStyle::Bar),
            terminal_cursor_blink: Some(true),
            terminal_cursor_blink_interval: Some(1000),
            terminal_copy_on_select: Some(false),
            terminal_right_click_pastes: Some(false),
            terminal_word_separator: Some(" ()[]{}',\"`".to_string()),
            terminal_scroll_sensitivity: Some(1),
            terminal_fast_scroll_modifier: Some("alt".to_string()),
            terminal_show_pane_header: Some(false),
            terminal_show_pane_footer: Some(false),
            terminal_use_webgl: Some(true),
            terminal_composer_enabled: Some(false),
            terminal_composer_history_popup: Some(false),
            terminal_composer_argument_completion: Some(true),
            terminal_blocks_enabled: Some(false),
            terminal_blocks_auto_collapse_on_alt_screen: Some(true),
            terminal_bell: Some(false),
            terminal_opacity: Some(100),
        }
    }
}
