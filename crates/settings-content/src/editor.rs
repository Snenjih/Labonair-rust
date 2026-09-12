//! `editor` area. Folds together the `Preferences` "Editor" field group and
//! the previously-separate `"editor"` settings-file key
//! (The old standalone editor wire object is read only by the Settings
//! migrator; `hlsearch`/`incsearch`/`smartcase` are retained during conversion)
//! into one typed area, per T19-001's instructions.

use serde::{Deserialize, Serialize};

/// Editor caret shape (independent of the terminal's `CursorStyle` — the two
/// areas stay decoupled per-module).
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum EditorCursorStyle {
    Block,
    Underline,
    #[default]
    Bar,
}

impl crate::MergeFrom for EditorCursorStyle {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, crate::MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct EditorContent {
    pub editor_font_family: Option<String>,
    pub editor_font_size: Option<u32>,
    pub editor_line_height: Option<f32>,
    pub editor_tab_size: Option<u32>,
    pub editor_word_wrap: Option<bool>,
    pub editor_line_numbers: Option<bool>,
    pub editor_relative_line_numbers: Option<bool>,
    pub editor_indent_with_tabs: Option<bool>,
    pub editor_indentation_guides: Option<bool>,
    pub editor_whitespace: Option<String>,
    pub editor_minimap: Option<bool>,
    pub editor_bracket_matching: Option<bool>,
    pub editor_rulers: Option<String>,
    pub editor_scroll_beyond_last_line: Option<bool>,
    pub editor_sticky_context: Option<bool>,
    pub editor_diagnostics: Option<bool>,
    pub editor_semantic_tokens: Option<bool>,
    pub editor_git_gutter: Option<bool>,
    pub editor_git_word_diff: Option<bool>,
    pub editor_completion: Option<bool>,
    pub editor_hover: Option<bool>,
    pub editor_format_on_save: Option<bool>,
    pub editor_auto_save: Option<bool>,
    pub editor_auto_save_delay: Option<u32>,
    pub editor_trim_trailing_whitespace: Option<bool>,
    pub editor_insert_final_newline: Option<bool>,
    pub editor_show_cursor_position: Option<bool>,
    pub editor_show_selection_stats: Option<bool>,
    pub editor_show_outline: Option<bool>,
    pub editor_autocomplete_debounce_ms: Option<u32>,
    pub editor_max_file_size_mb: Option<u32>,
    #[serde(rename = "vimMode")]
    pub editor_vim_mode: Option<bool>,
    /// Syntax colour scheme slug.
    pub editor_theme: Option<String>,
    /// Vim `hlsearch` (folded in from the legacy `"editor"` key).
    pub vim_hlsearch: Option<bool>,
    /// Vim `incsearch`.
    pub vim_incsearch: Option<bool>,
    /// Vim `smartcase`.
    pub vim_smartcase: Option<bool>,
    /// Tint the line the caret is on.
    pub editor_highlight_current_line: Option<bool>,
    /// Blink the caret while the editor is focused.
    pub editor_cursor_blink: Option<bool>,
    /// Caret blink half-period, in milliseconds.
    pub editor_cursor_blink_interval_ms: Option<u32>,
    /// Caret shape.
    pub editor_cursor_style: Option<EditorCursorStyle>,
}

impl EditorContent {
    pub fn defaults() -> Self {
        let mono = "\"Lilex\", SFMono-Regular, Menlo, monospace".to_string();
        Self {
            editor_font_family: Some(mono),
            editor_font_size: Some(15),
            editor_line_height: Some(1.618),
            editor_tab_size: Some(2),
            editor_word_wrap: Some(false),
            editor_line_numbers: Some(true),
            editor_relative_line_numbers: Some(false),
            editor_indent_with_tabs: Some(false),
            editor_indentation_guides: Some(true),
            editor_whitespace: Some("none".to_string()),
            editor_minimap: Some(false),
            editor_bracket_matching: Some(true),
            editor_rulers: Some(String::new()),
            editor_scroll_beyond_last_line: Some(true),
            editor_sticky_context: Some(false),
            editor_diagnostics: Some(true),
            editor_semantic_tokens: Some(true),
            editor_git_gutter: Some(true),
            editor_git_word_diff: Some(true),
            editor_completion: Some(true),
            editor_hover: Some(true),
            editor_format_on_save: Some(false),
            editor_auto_save: Some(false),
            editor_auto_save_delay: Some(1_000),
            editor_trim_trailing_whitespace: Some(false),
            editor_insert_final_newline: Some(false),
            editor_show_cursor_position: Some(true),
            editor_show_selection_stats: Some(false),
            editor_show_outline: Some(false),
            editor_autocomplete_debounce_ms: Some(350),
            editor_max_file_size_mb: Some(10),
            editor_vim_mode: Some(false),
            editor_theme: Some("atomone".to_string()),
            vim_hlsearch: Some(true),
            vim_incsearch: Some(true),
            vim_smartcase: Some(true),
            editor_highlight_current_line: Some(true),
            editor_cursor_blink: Some(true),
            editor_cursor_blink_interval_ms: Some(530),
            editor_cursor_style: Some(EditorCursorStyle::Bar),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_word_diff_defaults_to_enabled() {
        assert_eq!(EditorContent::defaults().editor_git_word_diff, Some(true));
    }

    #[test]
    fn git_word_diff_uses_the_canonical_camel_case_key() {
        let value = serde_json::to_value(EditorContent::defaults()).expect("editor defaults");
        assert_eq!(value["editorGitWordDiff"], true);
    }
}
