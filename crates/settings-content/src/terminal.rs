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

/// Terminal text weight (maps onto the theme typography's mono font weight).
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum TerminalFontWeight {
    #[default]
    Normal,
    Medium,
    Bold,
}

impl MergeFrom for TerminalFontWeight {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// Modifier that, while held, multiplies the mouse-wheel scroll step.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum FastScrollModifier {
    None,
    #[default]
    Alt,
    Ctrl,
    Shift,
}

impl MergeFrom for FastScrollModifier {
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
    /// Vertical line spacing as a multiple of the font size.
    pub terminal_line_height: Option<f32>,
    /// Terminal text weight.
    pub terminal_font_weight: Option<TerminalFontWeight>,
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
    /// Cursor blink half-period, in milliseconds.
    pub terminal_cursor_blink_interval: Option<u32>,
    pub terminal_copy_on_select: Option<bool>,
    pub terminal_right_click_pastes: Option<bool>,
    /// Characters that terminate a double-click word selection.
    pub terminal_word_separator: Option<String>,
    /// Multiplier applied to every mouse-wheel scroll step.
    pub terminal_scroll_sensitivity: Option<u32>,
    /// Modifier that multiplies the scroll step by 5 while held.
    pub terminal_fast_scroll_modifier: Option<FastScrollModifier>,
    pub terminal_bell: Option<bool>,
    /// Ask for confirmation before closing a terminal tab with a running shell.
    pub confirm_close_terminal_tab: Option<bool>,
    /// Terminal background opacity in percent (100 = fully opaque).
    pub terminal_opacity: Option<u32>,
}

/// The default word-separator set for double-click selection (mirrors the
/// reference client's `terminalWordSeparator` default).
pub const DEFAULT_WORD_SEPARATOR: &str = " ()[]{}',\"`";

impl TerminalContent {
    pub fn defaults() -> Self {
        let mono = "\"Lilex\", SFMono-Regular, Menlo, monospace".to_string();
        Self {
            terminal_shell: Some(String::new()),
            terminal_font_family: Some(mono),
            terminal_font_size: Some(15),
            terminal_line_height: Some(1.05),
            terminal_font_weight: Some(TerminalFontWeight::Normal),
            terminal_scrollback: Some(5_000),
            session_scrollback_lines: Some(1_000),
            scrollback_max_size_mb: Some(10),
            scrollback_retention_days: Some(0),
            terminal_cursor_style: Some(CursorStyle::Bar),
            terminal_cursor_blink: Some(true),
            terminal_cursor_blink_interval: Some(1_000),
            terminal_copy_on_select: Some(false),
            terminal_right_click_pastes: Some(false),
            terminal_word_separator: Some(DEFAULT_WORD_SEPARATOR.to_string()),
            terminal_scroll_sensitivity: Some(1),
            terminal_fast_scroll_modifier: Some(FastScrollModifier::Alt),
            terminal_bell: Some(false),
            confirm_close_terminal_tab: Some(false),
            terminal_opacity: Some(100),
        }
    }
}
