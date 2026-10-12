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
    Hollow,
    #[default]
    Bar,
}

impl crate::MergeFrom for EditorCursorStyle {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// Cursor shape override for an active Vim mode. `Inherit` uses the editor's
/// normal cursor-style setting.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum VimCursorShape {
    #[default]
    Inherit,
    Block,
    Underline,
    Bar,
}

impl crate::MergeFrom for VimCursorShape {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// Pointer modifier used to add cursors or start a rectangular selection.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum MultiCursorModifier {
    #[default]
    Alt,
    Control,
    Platform,
}

impl crate::MergeFrom for MultiCursorModifier {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// Active editor input mode. The value is stored as `editorMode` so the modal
/// modes remain mutually exclusive in settings and the editor UI.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum EditorModalMode {
    #[default]
    Normal,
    Vim,
    Helix,
}

impl crate::MergeFrom for EditorModalMode {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// How a new editor search query is seeded from the current buffer.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SearchSeedBehavior {
    /// Prefer a selection, otherwise use the word under the cursor.
    #[default]
    Always,
    /// Seed only from a non-empty single-line selection.
    Selection,
    /// Never replace the previous search query from buffer text.
    Never,
}

impl crate::MergeFrom for SearchSeedBehavior {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// Visibility policy for editor-owned scrollbars.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EditorScrollbarVisibility {
    #[default]
    Auto,
    System,
    Always,
    Never,
}

impl crate::MergeFrom for EditorScrollbarVisibility {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// Maximum diagnostic severity shown in editor scrollbars.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EditorDiagnosticMaxSeverity {
    Error,
    Warning,
    Information,
    #[default]
    All,
}

impl crate::MergeFrom for EditorDiagnosticMaxSeverity {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// How editor LSP document colors are presented.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EditorDocumentColorMode {
    Off,
    #[default]
    Inlay,
    Inline,
}

impl crate::MergeFrom for EditorDocumentColorMode {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// Unit system used for raster image file sizes in the editor preview.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum ImageViewerUnit {
    /// Base-1024 units (KiB, MiB, …).
    #[default]
    Binary,
    /// Base-1000 units (kB, MB, …).
    Decimal,
}

impl crate::MergeFrom for ImageViewerUnit {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// How the editor minimap is shown. `Auto` is shown only when the document
/// extends beyond the visible viewport.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EditorMinimapVisibility {
    Auto,
    Always,
    #[default]
    Never,
}

impl crate::MergeFrom for EditorMinimapVisibility {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// The minimap setting also accepts the old boolean wire value: `true` maps
/// to `always`, and `false` maps to `never`. The Settings UI writes the enum
/// form, so the legacy input disappears naturally as users change the value.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum EditorMinimapSetting {
    Visibility(EditorMinimapVisibility),
    Legacy(bool),
}

impl Default for EditorMinimapSetting {
    fn default() -> Self {
        Self::Visibility(EditorMinimapVisibility::Never)
    }
}

impl crate::MergeFrom for EditorMinimapSetting {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

impl EditorMinimapSetting {
    pub fn visibility(self) -> EditorMinimapVisibility {
        match self {
            Self::Visibility(visibility) => visibility,
            Self::Legacy(true) => EditorMinimapVisibility::Always,
            Self::Legacy(false) => EditorMinimapVisibility::Never,
        }
    }
}

/// Editors in which the minimap appears.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EditorMinimapDisplayIn {
    #[default]
    ActiveEditor,
    All,
}

impl crate::MergeFrom for EditorMinimapDisplayIn {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// When the minimap viewport thumb is drawn.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EditorMinimapThumb {
    #[default]
    Always,
    Hover,
}

impl crate::MergeFrom for EditorMinimapThumb {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// Border treatment for the minimap viewport thumb.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EditorMinimapThumbBorder {
    All,
    None,
    #[default]
    LeftOpen,
}

impl crate::MergeFrom for EditorMinimapThumbBorder {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// Current-line highlight policy for the minimap.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EditorMinimapCurrentLineHighlight {
    #[default]
    Inherit,
    Always,
    Never,
}

impl crate::MergeFrom for EditorMinimapCurrentLineHighlight {
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
    pub editor_font_weight: Option<u32>,
    pub editor_line_height: Option<f32>,
    pub editor_tab_size: Option<u32>,
    pub editor_word_wrap: Option<bool>,
    pub editor_line_numbers: Option<bool>,
    pub editor_relative_line_numbers: Option<bool>,
    pub editor_indent_with_tabs: Option<bool>,
    pub editor_auto_close_brackets: Option<bool>,
    pub editor_auto_surround: Option<bool>,
    pub editor_drag_and_drop_selection: Option<bool>,
    /// Modifier used for additional cursors and rectangular selections.
    pub editor_multi_cursor_modifier: Option<MultiCursorModifier>,
    pub editor_mouse_wheel_zoom: Option<bool>,
    pub editor_scroll_sensitivity: Option<f32>,
    pub editor_fast_scroll_sensitivity: Option<f32>,
    pub editor_vertical_scroll_margin: Option<u32>,
    pub editor_indentation_guides: Option<bool>,
    pub editor_whitespace: Option<String>,
    pub editor_whitespace_space: Option<String>,
    pub editor_whitespace_tab: Option<String>,
    pub editor_minimap: Option<EditorMinimapSetting>,
    pub editor_minimap_display_in: Option<EditorMinimapDisplayIn>,
    pub editor_minimap_thumb: Option<EditorMinimapThumb>,
    pub editor_minimap_thumb_border: Option<EditorMinimapThumbBorder>,
    pub editor_minimap_current_line_highlight: Option<EditorMinimapCurrentLineHighlight>,
    pub image_viewer_unit: Option<ImageViewerUnit>,
    pub editor_minimap_max_width_columns: Option<u32>,
    pub editor_scrollbar_show: Option<EditorScrollbarVisibility>,
    pub editor_scrollbar_cursors: Option<bool>,
    pub editor_scrollbar_git_diff: Option<bool>,
    pub editor_scrollbar_search_results: Option<bool>,
    pub editor_scrollbar_selected_text: Option<bool>,
    pub editor_scrollbar_selected_symbol: Option<bool>,
    pub editor_scrollbar_diagnostics: Option<EditorDiagnosticMaxSeverity>,
    pub editor_scrollbar_horizontal: Option<bool>,
    pub editor_scrollbar_vertical: Option<bool>,
    pub editor_bracket_matching: Option<bool>,
    pub editor_rulers: Option<String>,
    pub editor_scroll_beyond_last_line: Option<bool>,
    pub editor_sticky_context: Option<bool>,
    pub editor_diagnostics: Option<bool>,
    pub editor_diagnostics_max_severity: Option<EditorDiagnosticMaxSeverity>,
    pub editor_lsp_document_colors: Option<EditorDocumentColorMode>,
    pub editor_error_lens: Option<bool>,
    pub editor_error_lens_delay_ms: Option<u32>,
    pub editor_error_lens_spacing: Option<u32>,
    pub editor_error_lens_minimum_column: Option<u32>,
    pub editor_semantic_tokens: Option<bool>,
    pub editor_code_lens: Option<bool>,
    pub editor_inlay_hints: Option<bool>,
    pub editor_inlay_type_hints: Option<bool>,
    pub editor_inlay_parameter_hints: Option<bool>,
    pub editor_inlay_other_hints: Option<bool>,
    pub editor_inlay_show_background: Option<bool>,
    pub editor_inlay_edit_debounce_ms: Option<u32>,
    pub editor_inlay_scroll_debounce_ms: Option<u32>,
    pub editor_git_gutter: Option<bool>,
    pub editor_git_word_diff: Option<bool>,
    pub editor_git_blame_status: Option<bool>,
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
    pub editor_selections_menu: Option<bool>,
    pub editor_toolbar_breadcrumbs: Option<bool>,
    pub editor_toolbar_quick_actions: Option<bool>,
    pub editor_toolbar_code_actions: Option<bool>,
    pub editor_autocomplete_debounce_ms: Option<u32>,
    pub editor_max_file_size_mb: Option<u32>,
    /// Defaults and behavior for the editor's search surfaces.
    pub search_case_sensitive: Option<bool>,
    pub search_smart_case: Option<bool>,
    pub search_whole_word: Option<bool>,
    pub search_regex: Option<bool>,
    pub search_wrap: Option<bool>,
    pub search_center_on_match: Option<bool>,
    pub search_include_ignored: Option<bool>,
    pub search_on_type: Option<bool>,
    pub search_seed_from_cursor: Option<SearchSeedBehavior>,
    #[serde(rename = "editorMode")]
    pub editor_modal_mode: Option<EditorModalMode>,
    /// Legacy Vim toggle read when `editorMode` is absent.
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
    /// Highlight other occurrences of the selected text in this buffer.
    pub editor_selection_highlight: Option<bool>,
    /// Round the corners of text-selection backgrounds.
    pub editor_rounded_selection: Option<bool>,
    /// Blink the caret while the editor is focused.
    pub editor_cursor_blink: Option<bool>,
    /// Caret blink half-period, in milliseconds.
    pub editor_cursor_blink_interval_ms: Option<u32>,
    /// Caret shape.
    pub editor_cursor_style: Option<EditorCursorStyle>,
    /// Vim Normal-mode caret shape.
    pub vim_cursor_shape_normal: Option<VimCursorShape>,
    /// Vim Insert-mode caret shape.
    pub vim_cursor_shape_insert: Option<VimCursorShape>,
    /// Vim Replace-mode caret shape.
    pub vim_cursor_shape_replace: Option<VimCursorShape>,
    /// Vim Visual-mode caret shape.
    pub vim_cursor_shape_visual: Option<VimCursorShape>,
}

impl EditorContent {
    pub fn defaults() -> Self {
        let mono = "\"JetBrains Mono\", SFMono-Regular, Menlo, monospace".to_string();
        Self {
            editor_font_family: Some(mono),
            editor_font_size: Some(13),
            editor_font_weight: Some(400),
            editor_line_height: Some(1.55),
            editor_tab_size: Some(2),
            editor_word_wrap: Some(false),
            editor_line_numbers: Some(true),
            editor_relative_line_numbers: Some(false),
            editor_indent_with_tabs: Some(false),
            editor_auto_close_brackets: Some(true),
            editor_auto_surround: Some(true),
            editor_drag_and_drop_selection: Some(true),
            editor_multi_cursor_modifier: Some(MultiCursorModifier::Alt),
            editor_mouse_wheel_zoom: Some(false),
            editor_scroll_sensitivity: Some(1.0),
            editor_fast_scroll_sensitivity: Some(4.0),
            editor_vertical_scroll_margin: Some(3),
            editor_indentation_guides: Some(true),
            editor_whitespace: Some("none".to_string()),
            editor_whitespace_space: Some("•".to_string()),
            editor_whitespace_tab: Some("→".to_string()),
            editor_minimap: Some(EditorMinimapSetting::default()),
            editor_minimap_display_in: Some(EditorMinimapDisplayIn::ActiveEditor),
            editor_minimap_thumb: Some(EditorMinimapThumb::Always),
            editor_minimap_thumb_border: Some(EditorMinimapThumbBorder::LeftOpen),
            editor_minimap_current_line_highlight: Some(EditorMinimapCurrentLineHighlight::Inherit),
            image_viewer_unit: Some(ImageViewerUnit::Binary),
            editor_minimap_max_width_columns: Some(80),
            editor_scrollbar_show: Some(EditorScrollbarVisibility::Auto),
            editor_scrollbar_cursors: Some(true),
            editor_scrollbar_git_diff: Some(true),
            editor_scrollbar_search_results: Some(true),
            editor_scrollbar_selected_text: Some(true),
            editor_scrollbar_selected_symbol: Some(true),
            editor_scrollbar_diagnostics: Some(EditorDiagnosticMaxSeverity::All),
            editor_scrollbar_horizontal: Some(true),
            editor_scrollbar_vertical: Some(true),
            editor_bracket_matching: Some(true),
            editor_rulers: Some(String::new()),
            editor_scroll_beyond_last_line: Some(true),
            editor_sticky_context: Some(false),
            editor_diagnostics: Some(true),
            editor_diagnostics_max_severity: Some(EditorDiagnosticMaxSeverity::All),
            editor_lsp_document_colors: Some(EditorDocumentColorMode::Inlay),
            editor_error_lens: Some(false),
            editor_error_lens_delay_ms: Some(150),
            editor_error_lens_spacing: Some(4),
            editor_error_lens_minimum_column: Some(0),
            editor_semantic_tokens: Some(true),
            editor_code_lens: Some(false),
            editor_inlay_hints: Some(false),
            editor_inlay_type_hints: Some(true),
            editor_inlay_parameter_hints: Some(true),
            editor_inlay_other_hints: Some(true),
            editor_inlay_show_background: Some(false),
            editor_inlay_edit_debounce_ms: Some(700),
            editor_inlay_scroll_debounce_ms: Some(50),
            editor_git_gutter: Some(true),
            editor_git_word_diff: Some(true),
            editor_git_blame_status: Some(false),
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
            editor_selections_menu: Some(true),
            editor_toolbar_breadcrumbs: Some(true),
            editor_toolbar_quick_actions: Some(true),
            editor_toolbar_code_actions: Some(false),
            editor_autocomplete_debounce_ms: Some(350),
            editor_max_file_size_mb: Some(10),
            search_case_sensitive: Some(false),
            search_smart_case: Some(false),
            search_whole_word: Some(false),
            search_regex: Some(false),
            search_wrap: Some(true),
            search_center_on_match: Some(false),
            search_include_ignored: Some(false),
            search_on_type: Some(true),
            search_seed_from_cursor: Some(SearchSeedBehavior::Always),
            editor_modal_mode: Some(EditorModalMode::Normal),
            editor_vim_mode: Some(false),
            editor_theme: Some("atomone".to_string()),
            vim_hlsearch: Some(true),
            vim_incsearch: Some(true),
            vim_smartcase: Some(true),
            editor_highlight_current_line: Some(true),
            editor_selection_highlight: Some(true),
            editor_rounded_selection: Some(true),
            editor_cursor_blink: Some(true),
            editor_cursor_blink_interval_ms: Some(530),
            editor_cursor_style: Some(EditorCursorStyle::Bar),
            vim_cursor_shape_normal: Some(VimCursorShape::Block),
            vim_cursor_shape_insert: Some(VimCursorShape::Inherit),
            vim_cursor_shape_replace: Some(VimCursorShape::Underline),
            vim_cursor_shape_visual: Some(VimCursorShape::Block),
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

    #[test]
    fn git_blame_status_defaults_off_and_uses_the_canonical_key() {
        let defaults = EditorContent::defaults();
        assert_eq!(defaults.editor_git_blame_status, Some(false));
        let value = serde_json::to_value(defaults).expect("editor defaults");
        assert_eq!(value["editorGitBlameStatus"], false);
        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "editorGitBlameStatus": true
        }))
        .expect("blame status setting");
        assert_eq!(parsed.editor_git_blame_status, Some(true));
    }

    #[test]
    fn search_defaults_parse_and_serialize_with_canonical_names() {
        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "searchCaseSensitive": true,
            "searchSmartCase": true,
            "searchWholeWord": true,
            "searchRegex": true,
            "searchWrap": false,
            "searchCenterOnMatch": true,
            "searchIncludeIgnored": true,
            "searchOnType": false,
            "searchSeedFromCursor": "never"
        }))
        .expect("search settings");
        assert_eq!(parsed.search_case_sensitive, Some(true));
        assert_eq!(parsed.search_smart_case, Some(true));
        assert_eq!(parsed.search_whole_word, Some(true));
        assert_eq!(parsed.search_regex, Some(true));
        assert_eq!(parsed.search_wrap, Some(false));
        assert_eq!(parsed.search_center_on_match, Some(true));
        assert_eq!(parsed.search_include_ignored, Some(true));
        assert_eq!(parsed.search_on_type, Some(false));
        assert_eq!(
            parsed.search_seed_from_cursor,
            Some(SearchSeedBehavior::Never)
        );

        let defaults = serde_json::to_value(EditorContent::defaults()).expect("editor defaults");
        assert_eq!(defaults["searchCaseSensitive"], false);
        assert_eq!(defaults["searchSmartCase"], false);
        assert_eq!(defaults["searchWholeWord"], false);
        assert_eq!(defaults["searchRegex"], false);
        assert_eq!(defaults["searchWrap"], true);
        assert_eq!(defaults["searchCenterOnMatch"], false);
        assert_eq!(defaults["searchIncludeIgnored"], false);
        assert_eq!(defaults["searchOnType"], true);
        assert_eq!(defaults["searchSeedFromCursor"], "always");
    }

    #[test]
    fn drag_and_drop_selection_setting_parses_and_serializes() {
        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "editorDragAndDropSelection": false
        }))
        .expect("parse editor drag-and-drop preference");
        assert_eq!(parsed.editor_drag_and_drop_selection, Some(false));

        let value = serde_json::to_value(parsed).expect("serialize editor preferences");
        assert_eq!(value["editorDragAndDropSelection"], false);
    }

    #[test]
    fn multi_cursor_modifier_defaults_to_alt_and_uses_typed_values() {
        let defaults = EditorContent::defaults();
        assert_eq!(
            defaults.editor_multi_cursor_modifier,
            Some(MultiCursorModifier::Alt)
        );

        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "editorMultiCursorModifier": "control"
        }))
        .expect("parse multi-cursor modifier");
        assert_eq!(
            parsed.editor_multi_cursor_modifier,
            Some(MultiCursorModifier::Control)
        );
        let value = serde_json::to_value(parsed).expect("serialize multi-cursor modifier");
        assert_eq!(value["editorMultiCursorModifier"], "control");
    }

    #[test]
    fn editor_cursor_style_parses_and_serializes_hollow() {
        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "editorCursorStyle": "hollow"
        }))
        .expect("parse hollow editor cursor style");
        assert_eq!(parsed.editor_cursor_style, Some(EditorCursorStyle::Hollow));
        let value = serde_json::to_value(parsed).expect("serialize hollow editor cursor style");
        assert_eq!(value["editorCursorStyle"], "hollow");
    }

    #[test]
    fn mouse_wheel_zoom_setting_parses_and_serializes() {
        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "editorMouseWheelZoom": true
        }))
        .expect("parse editor mouse-wheel zoom preference");
        assert_eq!(parsed.editor_mouse_wheel_zoom, Some(true));

        let value = serde_json::to_value(parsed).expect("serialize editor preferences");
        assert_eq!(value["editorMouseWheelZoom"], true);
    }

    #[test]
    fn scroll_sensitivity_settings_parse_and_serialize() {
        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "editorScrollSensitivity": 1.5,
            "editorFastScrollSensitivity": 5.0
        }))
        .expect("parse editor scroll preferences");
        assert_eq!(parsed.editor_scroll_sensitivity, Some(1.5));
        assert_eq!(parsed.editor_fast_scroll_sensitivity, Some(5.0));

        let value = serde_json::to_value(parsed).expect("serialize editor preferences");
        assert_eq!(value["editorScrollSensitivity"], 1.5);
        assert_eq!(value["editorFastScrollSensitivity"], 5.0);
    }

    #[test]
    fn vertical_scroll_margin_parses_and_serializes() {
        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "editorVerticalScrollMargin": 6
        }))
        .expect("parse vertical scroll margin");
        assert_eq!(parsed.editor_vertical_scroll_margin, Some(6));

        let value = serde_json::to_value(parsed).expect("serialize editor preferences");
        assert_eq!(value["editorVerticalScrollMargin"], 6);
    }

    #[test]
    fn bracket_input_settings_parse_and_serialize() {
        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "editorAutoCloseBrackets": false,
            "editorAutoSurround": true
        }))
        .expect("parse bracket input settings");
        assert_eq!(parsed.editor_auto_close_brackets, Some(false));
        assert_eq!(parsed.editor_auto_surround, Some(true));

        let value = serde_json::to_value(parsed).expect("serialize editor preferences");
        assert_eq!(value["editorAutoCloseBrackets"], false);
        assert_eq!(value["editorAutoSurround"], true);
    }

    #[test]
    fn scrollbar_preferences_parse_and_serialize_with_canonical_names() {
        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "editorScrollbarShow": "never",
            "editorScrollbarCursors": false,
            "editorScrollbarDiagnostics": "warning",
            "editorScrollbarHorizontal": false,
        }))
        .expect("scrollbar preferences");
        assert_eq!(
            parsed.editor_scrollbar_show,
            Some(EditorScrollbarVisibility::Never)
        );
        assert_eq!(parsed.editor_scrollbar_cursors, Some(false));
        assert_eq!(
            parsed.editor_scrollbar_diagnostics,
            Some(EditorDiagnosticMaxSeverity::Warning)
        );
        assert_eq!(parsed.editor_scrollbar_horizontal, Some(false));

        let value = serde_json::to_value(parsed).expect("serialize scrollbar preferences");
        assert_eq!(value["editorScrollbarShow"], "never");
        assert_eq!(value["editorScrollbarDiagnostics"], "warning");
    }

    #[test]
    fn minimap_mode_accepts_old_booleans_and_serializes_the_new_mode() {
        let legacy_enabled: EditorContent = serde_json::from_value(serde_json::json!({
            "editorMinimap": true
        }))
        .expect("legacy minimap setting");
        assert_eq!(
            legacy_enabled
                .editor_minimap
                .map(EditorMinimapSetting::visibility),
            Some(EditorMinimapVisibility::Always)
        );

        let disabled: EditorContent = serde_json::from_value(serde_json::json!({
            "editorMinimap": false
        }))
        .expect("legacy disabled minimap setting");
        assert_eq!(
            disabled
                .editor_minimap
                .map(EditorMinimapSetting::visibility),
            Some(EditorMinimapVisibility::Never)
        );

        let defaults = serde_json::to_value(EditorContent::defaults()).expect("editor defaults");
        assert_eq!(defaults["editorMinimap"], "never");
        assert_eq!(defaults["editorMinimapDisplayIn"], "active_editor");
        assert_eq!(defaults["editorMinimapMaxWidthColumns"], 80);
    }

    #[test]
    fn toolbar_preferences_have_documented_defaults_and_wire_names() {
        let defaults = serde_json::to_value(EditorContent::defaults()).expect("editor defaults");
        assert_eq!(defaults["editorToolbarBreadcrumbs"], true);
        assert_eq!(defaults["editorToolbarQuickActions"], true);
        assert_eq!(defaults["editorToolbarCodeActions"], false);

        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "editorToolbarBreadcrumbs": false,
            "editorToolbarQuickActions": false,
            "editorToolbarCodeActions": true,
        }))
        .expect("toolbar preferences");
        assert_eq!(parsed.editor_toolbar_breadcrumbs, Some(false));
        assert_eq!(parsed.editor_toolbar_quick_actions, Some(false));
        assert_eq!(parsed.editor_toolbar_code_actions, Some(true));
    }

    #[test]
    fn editor_mode_parses_and_serializes_all_modal_choices() {
        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "editorMode": "helix"
        }))
        .expect("editor mode preference");
        assert_eq!(parsed.editor_modal_mode, Some(EditorModalMode::Helix));
        let value = serde_json::to_value(parsed).expect("serialize editor mode preference");
        assert_eq!(value["editorMode"], "helix");

        let vim: EditorContent = serde_json::from_value(serde_json::json!({ "vimMode": true }))
            .expect("legacy Vim mode preference");
        assert_eq!(vim.editor_vim_mode, Some(true));
    }

    #[test]
    fn document_color_mode_uses_the_editor_wire_key_and_defaults_to_inlay() {
        let defaults = serde_json::to_value(EditorContent::defaults()).expect("editor defaults");
        assert_eq!(defaults["editorLspDocumentColors"], "inlay");

        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "editorLspDocumentColors": "inline"
        }))
        .expect("document color setting");
        assert_eq!(
            parsed.editor_lsp_document_colors,
            Some(EditorDocumentColorMode::Inline)
        );
    }

    #[test]
    fn image_viewer_unit_parses_and_defaults_to_binary() {
        let defaults = serde_json::to_value(EditorContent::defaults()).expect("editor defaults");
        assert_eq!(defaults["imageViewerUnit"], "binary");

        let parsed: EditorContent = serde_json::from_value(serde_json::json!({
            "imageViewerUnit": "decimal"
        }))
        .expect("image viewer unit");
        assert_eq!(parsed.image_viewer_unit, Some(ImageViewerUnit::Decimal));
        let serialized = serde_json::to_value(parsed).expect("serialize image viewer unit");
        assert_eq!(serialized["imageViewerUnit"], "decimal");
    }
}
