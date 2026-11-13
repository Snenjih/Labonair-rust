//! `terminal` area.

use crate::MergeFrom;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

/// Where new terminal/SSH/SFTP sessions live. Fixed per session at creation;
/// changing the setting only affects sessions opened afterwards.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum SessionHostMode {
    /// Sessions end with the app (in-process).
    #[default]
    App,
    /// Sessions run in the background `labonair-host` daemon and survive quit.
    Detached,
}

impl MergeFrom for SessionHostMode {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// Whether a restored terminal re-runs the command that was in the foreground
/// when the session was saved (e.g. `claude --continue`).
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum ResumeOnRestore {
    Off,
    /// Offer a one-click banner in the pane.
    #[default]
    Ask,
    /// Re-run the command without asking.
    Auto,
}

impl MergeFrom for ResumeOnRestore {
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
    /// macOS: treat Option as Meta, so Option+key sends `ESC` + the base key
    /// (e.g. Option+B = `ESC b`) instead of the composed character (`∫`).
    pub terminal_option_as_meta: Option<bool>,
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
    /// Extra environment variables applied to every new terminal/SSH shell,
    /// as `KEY=value` pairs separated by commas (e.g. `"EDITOR=hx,FOO=bar"`).
    /// Malformed pairs (no `=`) are ignored.
    pub terminal_environment_variables: Option<String>,
    /// Extra arguments appended to `terminalShell`, whitespace-separated
    /// (e.g. `"-l --foo"`). No quoting support — use a wrapper script for
    /// arguments that contain spaces.
    pub terminal_shell_args: Option<String>,
    /// Where new sessions live: `app` (end with the app) or `detached`
    /// (background host that survives quit). Unix only.
    pub session_host: Option<SessionHostMode>,
    /// Re-run the foreground command of a restored terminal: `off`, `ask`
    /// (banner) or `auto`.
    pub resume_on_restore: Option<ResumeOnRestore>,
    /// User overrides for the resume table: foreground process name -> command
    /// to re-run (e.g. `"claude": "claude --continue"`). Built-in defaults live
    /// in code; an entry here replaces the default for that process.
    pub resume_commands: Option<BTreeMap<String, String>>,
    /// Scrollback rows the detached session host keeps per session.
    pub detached_scrollback_lines: Option<u32>,
}

/// Parse [`TerminalContent::terminal_environment_variables`]'s `KEY=value,
/// KEY2=value2` syntax. Entries without an `=` are dropped; a later
/// duplicate key overrides an earlier one.
pub fn parse_environment_variables(raw: &str) -> Vec<(String, String)> {
    let mut vars = Vec::new();
    for pair in raw.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        if let Some((key, value)) = pair.split_once('=') {
            let key = key.trim();
            if !key.is_empty() {
                vars.push((key.to_string(), value.trim().to_string()));
            }
        }
    }
    vars
}

/// Parse [`TerminalContent::terminal_shell_args`]'s whitespace-separated
/// argument list.
pub fn parse_shell_args(raw: &str) -> Vec<String> {
    raw.split_whitespace().map(str::to_string).collect()
}

/// The default word-separator set for double-click selection (mirrors the
/// reference client's `terminalWordSeparator` default).
pub const DEFAULT_WORD_SEPARATOR: &str = " ()[]{}',\"`";

impl TerminalContent {
    pub fn defaults() -> Self {
        let mono = "\"JetBrains Mono\", SFMono-Regular, Menlo, monospace".to_string();
        Self {
            terminal_shell: Some(String::new()),
            terminal_font_family: Some(mono),
            terminal_font_size: Some(14),
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
            terminal_option_as_meta: Some(false),
            terminal_word_separator: Some(DEFAULT_WORD_SEPARATOR.to_string()),
            terminal_scroll_sensitivity: Some(1),
            terminal_fast_scroll_modifier: Some(FastScrollModifier::Alt),
            terminal_bell: Some(false),
            confirm_close_terminal_tab: Some(false),
            terminal_opacity: Some(100),
            terminal_environment_variables: Some(String::new()),
            terminal_shell_args: Some(String::new()),
            session_host: Some(SessionHostMode::App),
            resume_on_restore: Some(ResumeOnRestore::Ask),
            resume_commands: Some(BTreeMap::new()),
            detached_scrollback_lines: Some(5_000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_host_settings_default_and_parse() {
        let d = TerminalContent::defaults();
        assert_eq!(d.session_host, Some(SessionHostMode::App));
        assert_eq!(d.resume_on_restore, Some(ResumeOnRestore::Ask));
        assert_eq!(d.detached_scrollback_lines, Some(5_000));

        let parsed: TerminalContent = serde_json::from_str(
            r#"{"sessionHost":"detached","resumeOnRestore":"auto",
                "resumeCommands":{"claude":"claude -c"},"detachedScrollbackLines":100}"#,
        )
        .unwrap();
        assert_eq!(parsed.session_host, Some(SessionHostMode::Detached));
        assert_eq!(parsed.resume_on_restore, Some(ResumeOnRestore::Auto));
        assert_eq!(parsed.resume_commands.unwrap()["claude"], "claude -c");
        assert_eq!(parsed.detached_scrollback_lines, Some(100));
    }
}
