//! `editor` area. Folds together the `Preferences` "Editor" field group and
//! the previously-separate `"editor"` settings-file key
//! (`labonair-backend::modules::settings::editor::EditorPrefs` — `hlsearch`/
//! `incsearch`/`smartcase`) into one typed area, per T19-001's instructions.

use serde::{Deserialize, Serialize};

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
    pub editor_format_on_save: Option<bool>,
    pub editor_trim_trailing_whitespace: Option<bool>,
    pub editor_insert_final_newline: Option<bool>,
    pub editor_bracket_matching: Option<bool>,
    pub editor_show_cursor_position: Option<bool>,
    pub editor_show_selection_stats: Option<bool>,
    pub editor_show_outline: Option<bool>,
    pub editor_indentation_guides: Option<bool>,
    /// `"off"` | `"afterDelay"` | `"onFocusChange"`.
    pub editor_auto_save: Option<String>,
    pub editor_auto_save_delay: Option<u32>,
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
            editor_format_on_save: Some(false),
            editor_trim_trailing_whitespace: Some(false),
            editor_insert_final_newline: Some(false),
            editor_bracket_matching: Some(true),
            editor_show_cursor_position: Some(true),
            editor_show_selection_stats: Some(true),
            editor_show_outline: Some(false),
            editor_indentation_guides: Some(true),
            editor_auto_save: Some("off".to_string()),
            editor_auto_save_delay: Some(1000),
            editor_autocomplete_debounce_ms: Some(350),
            editor_max_file_size_mb: Some(10),
            editor_vim_mode: Some(false),
            editor_theme: Some("atomone".to_string()),
            vim_hlsearch: Some(true),
            vim_incsearch: Some(true),
            vim_smartcase: Some(true),
        }
    }
}
