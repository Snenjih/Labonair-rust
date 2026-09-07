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
    pub editor_tab_size: Option<u32>,
    pub editor_word_wrap: Option<bool>,
    pub editor_line_numbers: Option<bool>,
    pub editor_relative_line_numbers: Option<bool>,
    pub editor_indent_with_tabs: Option<bool>,
    pub editor_format_on_save: Option<bool>,
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
            editor_tab_size: Some(2),
            editor_word_wrap: Some(false),
            editor_line_numbers: Some(true),
            editor_relative_line_numbers: Some(false),
            editor_indent_with_tabs: Some(false),
            editor_format_on_save: Some(false),
            editor_vim_mode: Some(false),
            editor_theme: Some("atomone".to_string()),
            vim_hlsearch: Some(true),
            vim_incsearch: Some(true),
            vim_smartcase: Some(true),
        }
    }
}
