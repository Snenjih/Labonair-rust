//! `SettingField`-equivalent registry (T19-004), replacing the old
//! hand-maintained `FIELDS: &[FieldDef]` table (T16-007's `fields.rs`, 131
//! entries chronically drifting against ~170 `Preferences` fields).
//!
//! Blueprint: `zed-refrence/zed/crates/settings_ui/src/settings_ui.rs`'s
//! `SettingField<T>` + `SettingFieldRenderer` registry, scoped down to what
//! Labonair needs (`docs/settings-guidelines.md` rule 3): every entry below is
//! a single declarative line naming a real `SettingsContent` field, a widget
//! kind (`FieldControl`), and copy. `get`/`set` are generated once by the
//! `field!` macro from the field's own Rust type via `serde_json`
//! (`Serialize`/`DeserializeOwned`, already derived on every `SettingsContent`
//! leaf type) — nothing here hand-parses a value, so a field can never drift
//! from its struct definition the way the old string-keyed `Preferences`
//! table could. Adding a new `bool` setting is exactly one `field!` line; no
//! bespoke widget code is written for a type this module already covers
//! (`docs/settings-guidelines.md` rule 3's "no bespoke hand-built toggle").
//!
//! `AnyField::get`/`set` are plain `fn` pointers (no captures), matching the
//! project's warning to keep pick/write as `fn` pointers rather than
//! closures — every `field!` invocation expands to a non-capturing closure
//! literal, which Rust coerces to a `fn` pointer automatically.

use labonair_settings_content::SettingsContent;
use serde_json::Value;

/// Extra copy/behaviour a rendered field carries beyond title+description
/// (`docs/settings-guidelines.md` rule 2: "title, description, and where
/// applicable unit, range, placeholder, requires_restart").
#[derive(Clone, Copy)]
pub struct SettingsFieldMetadata {
    pub title: &'static str,
    pub description: &'static str,
}

/// The renderer-registry key: which Rust-type-shaped widget a field gets
/// (`docs/settings-guidelines.md` rule 3's table). `Select`'s pairs are
/// `(serialized token, human label)` — labels are never the raw Rust variant
/// name (the project's warning).
#[derive(Clone, Copy)]
pub enum FieldControl {
    Switch,
    Int {
        min: i64,
        max: i64,
        step: i64,
    },
    /// `step`/`min`/`max` are expressed in hundredths so the descriptor stays
    /// a plain `i64` (mirrors the old `FieldKind::Float`).
    Float {
        min_centi: i64,
        max_centi: i64,
        step_centi: i64,
    },
    Select(&'static [(&'static str, &'static str)]),
    FontFamily,
    Text,
    /// The "anything else" fallback (rule 3): a raw JSON snippet editor. Used
    /// for containers (`Vec`, `BTreeMap`, nested structs) that have no
    /// scalar widget — this is what makes "no `SettingsContent` field is
    /// unreachable" true mechanically, without a bespoke widget per
    /// container type.
    Json,
}

/// One generated settings row: a stable deep-link path into the merged
/// `SettingsContent` tree, the widget it renders as, and its copy. `get`/
/// `set` are produced by the `field!` macro below from the field's own type —
/// see the module doc comment. Every member is individually `Copy` (a
/// `&'static str`, two `Copy` descriptor types, and two non-capturing `fn`
/// pointers), so the whole struct is too — cheap to copy out of the
/// `Vec<AnyField>` wherever a render closure needs to outlive the borrow.
#[derive(Clone, Copy)]
pub struct AnyField {
    /// Dot-separated path matching `SettingsStore::source_of`'s convention,
    /// e.g. `"terminal.terminalFontSize"` (rule 7: stable deep-link slugs).
    pub json_path: &'static str,
    pub control: FieldControl,
    pub meta: SettingsFieldMetadata,
    pub get: fn(&SettingsContent) -> Option<Value>,
    /// Returns `false` (rejecting the write) if `v` doesn't deserialize into
    /// the field's real type — the same "wrong-typed value is rejected, not
    /// stored" guarantee the old `PreferencesStore::set_value` had.
    pub set: fn(&mut SettingsContent, Value) -> bool,
}

impl AnyField {
    /// The area this field belongs to — the first `json_path` segment,
    /// matching a `labonair_settings_content::areas::AreaMeta::target_module`.
    pub fn area(&self) -> &'static str {
        self.json_path.split('.').next().unwrap_or("")
    }

    /// The local (leaf) key — the last `json_path` segment, matching the old
    /// `FIELDS`/`SECTION_GROUPS` table's bare camelCase keys, so the existing
    /// curated section groupings (`pages.rs`) can look fields up by it.
    pub fn local_key(&self) -> &'static str {
        self.json_path.rsplit('.').next().unwrap_or(self.json_path)
    }
}

macro_rules! meta {
    ($title:expr, $desc:expr) => {
        SettingsFieldMetadata {
            title: $title,
            description: $desc,
        }
    };
}

/// Generate one [`AnyField`]. `get`/`set` round-trip through `serde_json`
/// using the field's own `Serialize`/`Deserialize` impl (already derived on
/// every `SettingsContent` leaf type) — the widget/copy is the only thing a
/// new entry has to state; the marshalling can never drift from the struct.
macro_rules! field {
    ($area:ident.$field:ident, $json:literal, $control:expr, $title:expr, $desc:expr) => {
        AnyField {
            json_path: concat!(stringify!($area), ".", $json),
            control: $control,
            meta: meta!($title, $desc),
            get: |c| {
                c.$area
                    .$field
                    .clone()
                    .and_then(|v| serde_json::to_value(v).ok())
            },
            set: |c, v| match serde_json::from_value(v) {
                Ok(parsed) => {
                    c.$area.$field = Some(parsed);
                    true
                }
                Err(_) => false,
            },
        }
    };
}

/// Every `SettingsContent` leaf that has a settings-UI row (rule 2: "if it's
/// not in `SettingsContent`, it is not a setting" — the converse this table
/// enforces via `tests::every_leaf_field_has_exactly_one_settingfield` is "if
/// it's in `SettingsContent`, it has exactly one row here"). Order is
/// declaration order within each area; page layout (`pages.rs`) decides
/// on-screen placement, not this list.
pub fn all_fields() -> Vec<AnyField> {
    use FieldControl::{Float, FontFamily, Int, Json, Select, Switch, Text};
    vec![
        // ── general ─────────────────────────────────────────────────────
        field!(
            general.theme,
            "theme",
            Select(&[("system", "System"), ("light", "Light"), ("dark", "Dark")]),
            "Theme",
            "System, light, or dark appearance."
        ),
        field!(
            general.restore_window_state,
            "restoreWindowState",
            Switch,
            "Restore window",
            "Restore window size & position on launch."
        ),
        field!(
            general.default_startup_tab,
            "defaultStartupTab",
            Select(&[("terminal", "Terminal"), ("empty", "Empty")]),
            "Startup tab",
            "What opens on launch when there is no session to restore."
        ),
        field!(
            general.startup_terminal_count,
            "startupTerminalCount",
            Int { min: 1, max: 3, step: 1 },
            "Startup terminal count",
            "How many terminals open on launch."
        ),
        field!(
            general.autostart,
            "autostart",
            Switch,
            "Launch at login",
            "Start Labonair automatically when you log in."
        ),
        field!(
            general.credential_encryption,
            "credentialEncryption",
            Switch,
            "Encrypt stored credentials",
            "Encrypt saved credentials at rest with an OS-backed key."
        ),
        field!(
            general.notify_on_errors,
            "notifyOnErrors",
            Switch,
            "Notify on errors",
            "Show a toast when a background task fails."
        ),
        field!(
            general.confirm_quit_with_ssh,
            "confirmQuitWithSsh",
            Switch,
            "Confirm quit with SSH",
            "Ask before quitting with active SSH sessions."
        ),
        field!(
            general.check_for_updates,
            "checkForUpdates",
            Switch,
            "Check for updates",
            "Check for new versions automatically."
        ),
        field!(
            general.session_restore,
            "sessionRestore",
            Switch,
            "Session restore",
            "Reopen all tabs, SSH connections, SFTP paths and editor files on the next launch."
        ),
        // ── appearance ──────────────────────────────────────────────────
        field!(
            appearance.app_theme,
            "appTheme",
            Text,
            "Active theme id",
            "JSON theme file id (\"default\" = built-in); managed from the Themes page."
        ),
        field!(
            appearance.theme_variant_overrides,
            "themeVariantOverrides",
            Json,
            "Theme variant overrides",
            "Per-theme light/dark variant selection; managed from the Themes page."
        ),
        field!(
            appearance.app_font_size,
            "appFontSize",
            Int { min: 9, max: 24, step: 1 },
            "App font size",
            "Base UI font size in points."
        ),
        field!(
            appearance.app_line_height,
            "appLineHeight",
            Float { min_centi: 100, max_centi: 200, step_centi: 5 },
            "UI line height",
            "Line height multiplier for application text."
        ),
        field!(
            appearance.app_font_family,
            "appFontFamily",
            FontFamily,
            "UI font family",
            "Font used for all application UI text (empty = system default)."
        ),
        field!(
            appearance.reduce_motion,
            "reduceMotion",
            Switch,
            "Reduce motion",
            "Minimise animations and transitions."
        ),
        field!(
            appearance.app_corner_radius,
            "appCornerRadius",
            Int { min: 0, max: 20, step: 1 },
            "Corner radius (legacy)",
            "Legacy corner-radius base in px — superseded by \u{201c}Corner radius scale\u{201d}."
        ),
        field!(
            appearance.ui_density,
            "uiDensity",
            Select(&[
                ("compact", "Compact"),
                ("default", "Default"),
                ("comfortable", "Comfortable"),
            ]),
            "UI density",
            "Spacing and control sizes across the whole interface."
        ),
        field!(
            appearance.corner_radius_scale,
            "cornerRadiusScale",
            Float {
                min_centi: 50,
                max_centi: 200,
                step_centi: 5
            },
            "Corner radius scale",
            "Multiplier applied to the active theme's rounded corners (1.0 = unchanged)."
        ),
        field!(
            appearance.buffer_font_family,
            "bufferFontFamily",
            FontFamily,
            "Editor & terminal font",
            "Font used for editor and terminal text (empty = the theme's mono font)."
        ),
        field!(
            appearance.buffer_font_size,
            "bufferFontSize",
            Int { min: 9, max: 24, step: 1 },
            "Editor & terminal font size",
            "Text size for editor and terminal content, in points."
        ),
        field!(
            appearance.buffer_line_height,
            "bufferLineHeight",
            Float {
                min_centi: 100,
                max_centi: 200,
                step_centi: 5
            },
            "Editor & terminal line height",
            "Line-height multiplier for editor and terminal text."
        ),
        field!(
            appearance.background_image,
            "backgroundImage",
            Text,
            "Background image",
            "Filename of the background image (empty = none)."
        ),
        field!(
            appearance.background_opacity,
            "backgroundOpacity",
            Int { min: 0, max: 100, step: 5 },
            "Background opacity",
            "Opacity of the background image (%)."
        ),
        field!(
            appearance.background_blur,
            "backgroundBlur",
            Int { min: 0, max: 40, step: 1 },
            "Background blur",
            "Backdrop blur applied to the background image (px)."
        ),
        field!(
            appearance.background_tint_color,
            "backgroundTintColor",
            Text,
            "Background tint color",
            "Hex color overlaid on the background image."
        ),
        field!(
            appearance.background_tint_opacity,
            "backgroundTintOpacity",
            Int { min: 0, max: 100, step: 5 },
            "Background tint opacity",
            "Opacity of the background tint overlay (%)."
        ),
        field!(
            appearance.tabs_location,
            "tabsLocation",
            Select(&[("titlebar", "Titlebar"), ("sidebar", "Sidebar")]),
            "Tab bar location",
            "Where the tab strip lives."
        ),
        field!(
            appearance.sidebar_tab_info_line,
            "sidebarTabInfoLine",
            Json,
            "Sidebar tab info line",
            "Up to two info badges shown on each sidebar tab."
        ),
        field!(
            appearance.sidebar_group_by_folder,
            "sidebarGroupByFolder",
            Switch,
            "Group sidebar tabs by folder",
            "Group tabs that share a working directory."
        ),
        field!(
            appearance.sidebar_group_single_tabs,
            "sidebarGroupSingleTabs",
            Switch,
            "Group single tabs too",
            "Also show a group header for a lone tab."
        ),
        field!(
            appearance.badges_always_visible,
            "badgesAlwaysVisible",
            Switch,
            "Always show badges",
            "Keep count badges visible even at zero."
        ),
        field!(
            appearance.titlebars_icons_position,
            "titlebarsIconsPosition",
            Select(&[("auto", "Auto"), ("left", "Left"), ("right", "Right")]),
            "Titlebar icon position",
            "Legacy titlebar traffic-light / icon alignment."
        ),
        field!(
            appearance.zen_mode_show_header,
            "zenModeShowHeader",
            Switch,
            "Show header bar",
            "Show the window header bar (zen mode off)."
        ),
        field!(
            appearance.zen_mode_show_statusbar,
            "zenModeShowStatusbar",
            Switch,
            "Show status bar",
            "Show the bottom status bar (zen mode off)."
        ),
        // ── terminal ────────────────────────────────────────────────────
        field!(
            terminal.terminal_shell,
            "terminalShell",
            Text,
            "Shell",
            "Program to launch (empty = system default)."
        ),
        field!(
            terminal.terminal_default_path,
            "terminalDefaultPath",
            Text,
            "Default working directory",
            "Directory new terminals start in (empty = home)."
        ),
        field!(
            terminal.new_tab_inherits_cwd,
            "newTabInheritsCwd",
            Switch,
            "New tab inherits directory",
            "Open new terminal tabs in the current tab's directory."
        ),
        field!(
            terminal.confirm_close_terminal_tab,
            "confirmCloseTerminalTab",
            Switch,
            "Confirm before closing a terminal tab",
            "Ask for confirmation when closing a terminal tab."
        ),
        field!(
            terminal.terminal_font_family,
            "terminalFontFamily",
            FontFamily,
            "Font family",
            "Terminal typeface."
        ),
        field!(
            terminal.terminal_font_size,
            "terminalFontSize",
            Int { min: 8, max: 32, step: 1 },
            "Font size",
            "Terminal font size in points."
        ),
        field!(
            terminal.terminal_font_weight,
            "terminalFontWeight",
            Select(&[("normal", "Normal"), ("medium", "Medium"), ("bold", "Bold")]),
            "Font weight",
            "Weight of the terminal typeface."
        ),
        field!(
            terminal.terminal_letter_spacing,
            "terminalLetterSpacing",
            Float { min_centi: -200, max_centi: 1000, step_centi: 50 },
            "Letter spacing",
            "Extra horizontal spacing between glyphs, in pixels."
        ),
        field!(
            terminal.terminal_line_height,
            "terminalLineHeight",
            Float { min_centi: 80, max_centi: 200, step_centi: 5 },
            "Line height",
            "Terminal line height multiplier."
        ),
        field!(
            terminal.terminal_scrollback,
            "terminalScrollback",
            Int { min: 1000, max: 200_000, step: 1000 },
            "Scrollback lines",
            "Lines of history kept per terminal."
        ),
        field!(
            terminal.session_scrollback_lines,
            "sessionScrollbackLines",
            Int { min: 0, max: 100_000, step: 500 },
            "Persisted scrollback lines",
            "Rows of history saved per pane on quit and replayed on the next launch (0 = all)."
        ),
        field!(
            terminal.scrollback_max_size_mb,
            "scrollbackMaxSizeMb",
            Int { min: 1, max: 100, step: 1 },
            "Persisted scrollback size cap",
            "Per-file ceiling for a saved scrollback, in MB."
        ),
        field!(
            terminal.scrollback_retention_days,
            "scrollbackRetentionDays",
            Int { min: 0, max: 365, step: 1 },
            "Persisted scrollback retention",
            "Days a saved scrollback file is kept before cleanup removes it (0 = keep with the session)."
        ),
        field!(
            terminal.terminal_cursor_style,
            "terminalCursorStyle",
            Select(&[
                ("block", "Block"),
                ("underline", "Underline"),
                ("bar", "Bar")
            ]),
            "Cursor style",
            "Shape of the terminal cursor."
        ),
        field!(
            terminal.terminal_cursor_blink,
            "terminalCursorBlink",
            Switch,
            "Cursor blink",
            "Blink the terminal cursor."
        ),
        field!(
            terminal.terminal_cursor_blink_interval,
            "terminalCursorBlinkInterval",
            Int { min: 200, max: 2000, step: 50 },
            "Cursor blink interval",
            "How fast the terminal cursor blinks (ms)."
        ),
        field!(
            terminal.terminal_copy_on_select,
            "terminalCopyOnSelect",
            Switch,
            "Copy on select",
            "Copy selected text to the clipboard automatically."
        ),
        field!(
            terminal.terminal_right_click_pastes,
            "terminalRightClickPastes",
            Switch,
            "Right-click pastes",
            "Paste the clipboard on right-click instead of a context menu."
        ),
        field!(
            terminal.terminal_word_separator,
            "terminalWordSeparator",
            Text,
            "Word separators",
            "Characters that break a word for double-click selection."
        ),
        field!(
            terminal.terminal_scroll_sensitivity,
            "terminalScrollSensitivity",
            Int { min: 1, max: 10, step: 1 },
            "Scroll sensitivity",
            "Lines scrolled per wheel notch."
        ),
        field!(
            terminal.terminal_fast_scroll_modifier,
            "terminalFastScrollModifier",
            Select(&[
                ("none", "None"),
                ("alt", "Alt"),
                ("ctrl", "Ctrl"),
                ("shift", "Shift")
            ]),
            "Fast-scroll modifier",
            "Hold this key to scroll faster."
        ),
        field!(
            terminal.terminal_show_pane_header,
            "terminalShowPaneHeader",
            Switch,
            "Show pane headers",
            "Show a header strip above each terminal pane."
        ),
        field!(
            terminal.terminal_show_pane_footer,
            "terminalShowPaneFooter",
            Switch,
            "Show pane footer",
            "Show a footer strip below each terminal pane."
        ),
        field!(
            terminal.terminal_use_webgl,
            "terminalUseWebgl",
            Switch,
            "Use WebGL renderer",
            "Render the terminal via WebGL when the platform supports it."
        ),
        field!(
            terminal.terminal_composer_enabled,
            "terminalComposerEnabled",
            Switch,
            "Command composer",
            "Show the composer input above the terminal."
        ),
        field!(
            terminal.terminal_composer_history_popup,
            "terminalComposerHistoryPopup",
            Switch,
            "Composer history popup",
            "Show a history dropdown while composing."
        ),
        field!(
            terminal.terminal_composer_argument_completion,
            "terminalComposerArgumentCompletion",
            Switch,
            "Argument completion",
            "Suggest command arguments in the composer."
        ),
        field!(
            terminal.terminal_blocks_enabled,
            "terminalBlocksEnabled",
            Switch,
            "Block terminal",
            "Group command output into collapsible blocks."
        ),
        field!(
            terminal.terminal_blocks_auto_collapse_on_alt_screen,
            "terminalBlocksAutoCollapseOnAltScreen",
            Switch,
            "Auto-collapse blocks for full-screen apps",
            "Collapse blocks when an app takes the alternate screen."
        ),
        field!(
            terminal.terminal_bell,
            "terminalBell",
            Switch,
            "Audible bell",
            "Play a sound on the terminal bell."
        ),
        field!(
            terminal.terminal_opacity,
            "terminalOpacity",
            Int { min: 20, max: 100, step: 5 },
            "Background opacity",
            "Terminal background opacity in percent (100 = opaque)."
        ),
        // ── editor ──────────────────────────────────────────────────────
        field!(
            editor.editor_font_family,
            "editorFontFamily",
            FontFamily,
            "Font family",
            "Editor typeface."
        ),
        field!(
            editor.editor_font_size,
            "editorFontSize",
            Int { min: 8, max: 32, step: 1 },
            "Font size",
            "Editor font size in points."
        ),
        field!(
            editor.editor_line_height,
            "editorLineHeight",
            Float { min_centi: 100, max_centi: 300, step_centi: 5 },
            "Line height",
            "Editor line height multiplier."
        ),
        field!(
            editor.editor_tab_size,
            "editorTabSize",
            Int { min: 2, max: 8, step: 2 },
            "Tab size",
            "Spaces per indentation level."
        ),
        field!(
            editor.editor_word_wrap,
            "editorWordWrap",
            Switch,
            "Word wrap",
            "Wrap long lines to the viewport width."
        ),
        field!(
            editor.editor_line_numbers,
            "editorLineNumbers",
            Switch,
            "Line numbers",
            "Show the line-number gutter."
        ),
        field!(
            editor.editor_relative_line_numbers,
            "editorRelativeLineNumbers",
            Switch,
            "Relative line numbers",
            "Number lines relative to the cursor."
        ),
        field!(
            editor.editor_indent_with_tabs,
            "editorIndentWithTabs",
            Switch,
            "Indent with tabs",
            "Use tab characters instead of spaces."
        ),
        field!(
            editor.editor_format_on_save,
            "editorFormatOnSave",
            Switch,
            "Format on save",
            "Run the formatter when saving."
        ),
        field!(
            editor.editor_trim_trailing_whitespace,
            "editorTrimTrailingWhitespace",
            Switch,
            "Trim trailing whitespace",
            "Remove trailing spaces on save."
        ),
        field!(
            editor.editor_insert_final_newline,
            "editorInsertFinalNewline",
            Switch,
            "Insert final newline",
            "Ensure a trailing newline on save."
        ),
        field!(
            editor.editor_bracket_matching,
            "editorBracketMatching",
            Switch,
            "Bracket matching",
            "Highlight the matching bracket at the cursor."
        ),
        field!(
            editor.editor_show_cursor_position,
            "editorShowCursorPosition",
            Switch,
            "Cursor position",
            "Show line/column in the status bar."
        ),
        field!(
            editor.editor_show_selection_stats,
            "editorShowSelectionStats",
            Switch,
            "Selection stats",
            "Show selected character / line counts."
        ),
        field!(
            editor.editor_show_outline,
            "editorShowOutline",
            Switch,
            "Outline panel",
            "Show the document symbol outline."
        ),
        field!(
            editor.editor_indentation_guides,
            "editorIndentationGuides",
            Switch,
            "Indentation guides",
            "Draw vertical indentation guide lines."
        ),
        field!(
            editor.editor_auto_save,
            "editorAutoSave",
            Select(&[
                ("off", "Off"),
                ("afterDelay", "After delay"),
                ("onFocusChange", "On focus change")
            ]),
            "Auto save",
            "When to automatically save edited files."
        ),
        field!(
            editor.editor_auto_save_delay,
            "editorAutoSaveDelay",
            Int { min: 100, max: 60_000, step: 100 },
            "Auto save delay",
            "Idle time before an auto save when 'after delay' is selected (ms)."
        ),
        field!(
            editor.editor_autocomplete_debounce_ms,
            "editorAutocompleteDebounceMs",
            Int { min: 50, max: 2000, step: 50 },
            "Autocomplete debounce",
            "Idle time before requesting an AI completion (ms)."
        ),
        field!(
            editor.editor_max_file_size_mb,
            "editorMaxFileSizeMb",
            Int { min: 1, max: 100, step: 1 },
            "Max file size",
            "Files larger than this open read-only / unhighlighted (MB)."
        ),
        field!(
            editor.editor_vim_mode,
            "vimMode",
            Switch,
            "Vim mode",
            "Enable the modal Vim keybinding layer."
        ),
        field!(
            editor.editor_theme,
            "editorTheme",
            Select(&[
                ("auto", "Auto"),
                ("atomone", "Atom One"),
                ("aura", "Aura"),
                ("copilot", "Copilot"),
                ("github-dark", "GitHub Dark"),
                ("github-light", "GitHub Light"),
                ("nord", "Nord"),
                ("tokyo-night", "Tokyo Night"),
                ("xcode-dark", "Xcode Dark"),
                ("xcode-light", "Xcode Light")
            ]),
            "Syntax theme",
            "Editor colour scheme (auto follows the app theme)."
        ),
        field!(
            editor.vim_hlsearch,
            "vimHlsearch",
            Switch,
            "Vim: highlight search",
            "Highlight all matches of the last search pattern."
        ),
        field!(
            editor.vim_incsearch,
            "vimIncsearch",
            Switch,
            "Vim: incremental search",
            "Show search matches as you type."
        ),
        field!(
            editor.vim_smartcase,
            "vimSmartcase",
            Switch,
            "Vim: smart case",
            "Case-insensitive search unless the pattern has an uppercase letter."
        ),
        // ── file_manager ────────────────────────────────────────────────
        field!(
            file_manager.sftp_show_hidden_files,
            "sftpShowHiddenFiles",
            Switch,
            "Show hidden files",
            "Show dotfiles in the file browser by default."
        ),
        field!(
            file_manager.sftp_show_up_folder,
            "sftpShowUpFolder",
            Switch,
            "Show '..' up-folder entry",
            "Show an entry to go to the parent directory."
        ),
        field!(
            file_manager.explorer_show_hidden_by_default,
            "explorerShowHiddenByDefault",
            Switch,
            "Explorer: show hidden files by default",
            "Show dotfiles in the sidebar explorer."
        ),
        field!(
            file_manager.explorer_indent_guides,
            "explorerIndentGuides",
            Switch,
            "Explorer: indent guides",
            "Draw thin vertical guides at each tree depth."
        ),
        field!(
            file_manager.explorer_sticky_ancestors,
            "explorerStickyAncestors",
            Switch,
            "Explorer: sticky ancestors",
            "Pin the scrolled-past ancestor folders above the tree."
        ),
        field!(
            file_manager.explorer_auto_reveal_active_file,
            "explorerAutoRevealActiveFile",
            Switch,
            "Explorer: reveal active file",
            "Scroll the tree to the file open in the active editor."
        ),
        field!(
            file_manager.explorer_fold_single_child_dirs,
            "explorerFoldSingleChildDirs",
            Switch,
            "Explorer: fold single-child folders",
            "Collapse a chain of folders with one child into one row."
        ),
        field!(
            file_manager.explorer_git_decorations,
            "explorerGitDecorations",
            Switch,
            "Explorer: Git decorations",
            "Tint changed files and show their status letter in the tree."
        ),
        field!(
            file_manager.scm_file_tree,
            "scmFileTree",
            Switch,
            "Source Control: file tree",
            "Show the change list as a directory tree instead of flat status groups."
        ),
        field!(
            file_manager.sftp_column_size,
            "sftpColumnSize",
            Switch,
            "Show Size column",
            "Show the file size column."
        ),
        field!(
            file_manager.sftp_column_modified,
            "sftpColumnModified",
            Switch,
            "Show Modified column",
            "Show the modification time column."
        ),
        field!(
            file_manager.sftp_column_permissions,
            "sftpColumnPermissions",
            Switch,
            "Show Permissions column",
            "Show the permissions column."
        ),
        field!(
            file_manager.sftp_column_type,
            "sftpColumnType",
            Switch,
            "Show Type column",
            "Show the file type column."
        ),
        field!(
            file_manager.sftp_remote_edit_show_transfers,
            "sftpRemoteEditShowTransfers",
            Switch,
            "Show remote edit transfers",
            "Show a transfer indicator when editing remote files."
        ),
        field!(
            file_manager.sftp_max_remote_file_size_mb,
            "sftpMaxRemoteFileSizeMb",
            Int { min: 1, max: 100, step: 1 },
            "Max remote file size",
            "Refuse to open remote files larger than this for editing (MB)."
        ),
        field!(
            file_manager.sftp_font_size,
            "sftpFontSize",
            Int { min: 8, max: 32, step: 1 },
            "Font size",
            "File-browser font size in points."
        ),
        field!(
            file_manager.sftp_max_concurrent_transfers,
            "sftpMaxConcurrentTransfers",
            Int { min: 1, max: 16, step: 1 },
            "Max concurrent transfers",
            "Parallel SFTP transfers."
        ),
        field!(
            file_manager.sftp_default_conflict_resolution,
            "sftpDefaultConflictResolution",
            Select(&[
                ("ask", "Ask"),
                ("overwrite", "Overwrite"),
                ("skip", "Skip")
            ]),
            "On name conflict",
            "Default action when a transfer target already exists."
        ),
        field!(
            file_manager.sftp_chunk_size_kb,
            "sftpChunkSizeKb",
            Int { min: 16, max: 1024, step: 16 },
            "Transfer chunk size",
            "Block size used for SFTP transfers (KB)."
        ),
        field!(
            file_manager.sftp_on_folder_file_error,
            "sftpOnFolderFileError",
            Select(&[
                ("ask", "Ask"),
                ("skip", "Skip"),
                ("abort", "Abort")
            ]),
            "On file error in folder transfers",
            "What to do when one file in a recursive transfer fails."
        ),
        // ── connections ─────────────────────────────────────────────────
        field!(
            connections.host_ping_interval,
            "hostPingInterval",
            Int { min: 0, max: 600, step: 10 },
            "Host availability ping interval",
            "How often to ping saved hosts (s, 0 = never)."
        ),
        field!(
            connections.ssh_connect_timeout_secs,
            "sshConnectTimeoutSecs",
            Int { min: 3, max: 60, step: 1 },
            "SSH connect timeout",
            "Give up connecting after this long (s)."
        ),
        field!(
            connections.ssh_auto_reconnect,
            "sshAutoReconnect",
            Switch,
            "Auto-reconnect SSH sessions",
            "Reconnect dropped SSH terminal sessions automatically."
        ),
        field!(
            connections.ssh_auto_reconnect_delay,
            "sshAutoReconnectDelay",
            Int { min: 1, max: 30, step: 1 },
            "Reconnect delay",
            "Wait this long before an SSH reconnect attempt (s)."
        ),
        field!(
            connections.ssh_auto_reconnect_max_attempts,
            "sshAutoReconnectMaxAttempts",
            Int { min: 1, max: 10, step: 1 },
            "Max reconnect attempts",
            "Give up after this many SSH reconnect attempts."
        ),
        field!(
            connections.explorer_remote_poll_interval,
            "explorerRemotePollInterval",
            Int { min: 0, max: 60, step: 10 },
            "Explorer: remote refresh interval",
            "How often the remote explorer re-reads the directory (s, 0 = never)."
        ),
        field!(
            connections.explorer_auto_reconnect,
            "explorerAutoReconnect",
            Switch,
            "Explorer: auto-reconnect remote sessions",
            "Reconnect dropped remote explorer sessions."
        ),
        field!(
            connections.explorer_idle_session_timeout_min,
            "explorerIdleSessionTimeoutMin",
            Int { min: 1, max: 30, step: 1 },
            "Explorer: idle session timeout",
            "Close idle cached remote sessions after this long (min)."
        ),
        field!(
            connections.explorer_max_idle_sessions,
            "explorerMaxIdleSessions",
            Int { min: 1, max: 10, step: 1 },
            "Explorer: max cached remote sessions",
            "Upper bound on kept-alive idle remote sessions."
        ),
        field!(
            connections.explorer_max_cached_remote_scopes,
            "explorerMaxCachedRemoteScopes",
            Int { min: 1, max: 20, step: 1 },
            "Explorer: max cached remote folders",
            "Upper bound on cached remote directory listings."
        ),
        // ── workspace ───────────────────────────────────────────────────
        field!(
            workspace.command_palette_search_mode,
            "commandPaletteSearchMode",
            Select(&[
                ("contains", "Contains"),
                ("startsWith", "Starts with"),
                ("fuzzy", "Fuzzy")
            ]),
            "Search mode",
            "How the palette matches your query."
        ),
        field!(
            workspace.command_palette_show_recent,
            "commandPaletteShowRecent",
            Switch,
            "Show recent",
            "Surface recently-run commands first."
        ),
        field!(
            workspace.command_palette_blur,
            "commandPaletteBlur",
            Int { min: 0, max: 20, step: 1 },
            "Background blur",
            "Backdrop blur behind the palette (px)."
        ),
        field!(
            workspace.command_palette_opacity,
            "commandPaletteOpacity",
            Int { min: 35, max: 100, step: 1 },
            "Card opacity",
            "Palette card opacity (%)."
        ),
        field!(
            workspace.command_palette_position,
            "commandPalettePosition",
            Select(&[
                ("top", "Top"),
                ("high", "High"),
                ("center", "Center")
            ]),
            "Position",
            "Where the palette opens vertically."
        ),
        field!(
            workspace.command_palette_animation,
            "commandPaletteAnimation",
            Select(&[
                ("fast", "Fast"),
                ("normal", "Normal"),
                ("slow", "Slow"),
                ("none", "None")
            ]),
            "Animation speed",
            "Open/close animation speed."
        ),
        field!(
            workspace.command_palette_history_size,
            "commandPaletteHistorySize",
            Int { min: 0, max: 20, step: 1 },
            "Recent history size",
            "How many recently-run commands to remember."
        ),
        field!(
            workspace.command_palette_close_on_overlay_click,
            "commandPaletteCloseOnOverlayClick",
            Switch,
            "Close on click-away",
            "Dismiss the palette when clicking outside the card."
        ),
        field!(
            workspace.git_status_poll_interval_ms,
            "gitStatusPollIntervalMs",
            Int { min: 500, max: 30_000, step: 500 },
            "Status poll interval",
            "How often to refresh git status (ms)."
        ),
        field!(
            workspace.dock_layout,
            "dockLayout",
            Json,
            "Dock layout",
            "Internal persisted dock/panel layout snapshot."
        ),
        field!(
            workspace.sidebar_position,
            "sidebarPosition",
            Select(&[("left", "Left"), ("right", "Right")]),
            "Sidebar position",
            "Which edge the primary sidebar docks to."
        ),
        field!(
            workspace.sidebar_open,
            "sidebarOpen",
            Switch,
            "Sidebar open",
            "Whether the primary sidebar is open."
        ),
        field!(
            workspace.sidebar_active_panel,
            "sidebarActivePanel",
            Text,
            "Sidebar active panel",
            "Which panel is active in the primary sidebar."
        ),
        field!(
            workspace.sidebar_right_open,
            "sidebarRightOpen",
            Switch,
            "Right sidebar open",
            "Whether the secondary (right) sidebar is open."
        ),
        field!(
            workspace.sidebar_right_active_panel,
            "sidebarRightActivePanel",
            Text,
            "Right sidebar active panel",
            "Which panel is active in the secondary sidebar."
        ),
        field!(
            workspace.sidebar_width,
            "sidebarWidth",
            Int { min: 100, max: 500, step: 10 },
            "Sidebar width",
            "Width of the primary sidebar in pixels."
        ),
        field!(
            workspace.sidebar_right_width,
            "sidebarRightWidth",
            Int { min: 100, max: 500, step: 10 },
            "Right sidebar width",
            "Width of the secondary sidebar in pixels."
        ),
        // ── mcp ──────────────────────────────────────────────────────────
        field!(
            mcp.bridge_enabled,
            "bridgeEnabled",
            Switch,
            "Enable AI Agent Bridge",
            "Let an external agent CLI drive granted SSH / local tabs over MCP."
        ),
        field!(
            mcp.bridge_port,
            "bridgePort",
            Int { min: 1024, max: 65535, step: 1 },
            "Port",
            "Local port the Streamable-HTTP listener binds to."
        ),
        field!(
            mcp.max_command_timeout_secs,
            "maxCommandTimeoutSecs",
            Int { min: 5, max: 3600, step: 5 },
            "Max command timeout",
            "Upper bound on a single agent-run command before it returns still_running (s)."
        ),
        field!(
            mcp.auto_revoke_minutes,
            "autoRevokeMinutes",
            Int { min: 0, max: 1440, step: 5 },
            "Auto-revoke after",
            "Revoke a granted tab after this many minutes without agent activity (0 = off)."
        ),
        field!(
            mcp.notify_on_activity,
            "notifyOnActivity",
            Switch,
            "Notify on agent activity",
            "Show a toast for every command / keystroke an agent sends."
        ),
        // ── personalization ─────────────────────────────────────────────
        field!(
            personalization.status_bar_item_placements,
            "statusBarItemPlacements",
            Json,
            "Status bar item placements",
            "Which cluster each status-bar item lives in; managed from the Personalization page."
        ),
        field!(
            personalization.panel_toggle_visibility,
            "panelToggleVisibility",
            Json,
            "Panel toggle visibility",
            "Which panels are hidden; managed from the Personalization page."
        ),
        field!(
            personalization.status_bar_show_explorer_button,
            "statusBarShowExplorerButton",
            Switch,
            "Show Explorer button",
            "Show the Explorer toggle in the status bar."
        ),
        field!(
            personalization.status_bar_show_snippets_button,
            "statusBarShowSnippetsButton",
            Switch,
            "Show Snippets button",
            "Show the Snippets toggle in the status bar."
        ),
        field!(
            personalization.status_bar_show_source_control_button,
            "statusBarShowSourceControlButton",
            Switch,
            "Show Source Control button",
            "Show the Source Control toggle in the status bar."
        ),
        field!(
            personalization.status_bar_show_tabs_button,
            "statusBarShowTabsButton",
            Switch,
            "Show Tabs button",
            "Show the Tabs toggle in the status bar."
        ),
        field!(
            personalization.status_bar_show_cwd_breadcrumb,
            "statusBarShowCwdBreadcrumb",
            Switch,
            "Show directory breadcrumb",
            "Show the current working directory in the status bar."
        ),
        field!(
            personalization.status_bar_show_preview_url,
            "statusBarShowPreviewUrl",
            Switch,
            "Show preview URL",
            "Show a detected local dev-server URL in the status bar."
        ),
        field!(
            personalization.status_bar_show_ai_controls,
            "statusBarShowAiControls",
            Switch,
            "Show AI controls",
            "Show the AI agent/model controls in the status bar."
        ),
        // ── hosts ────────────────────────────────────────────────────────
        field!(
            hosts.entries,
            "entries",
            Json,
            "Saved hosts",
            "Non-secret host metadata; managed from the Hosts page (T19-010)."
        ),
        field!(
            hosts.default_shell,
            "defaultShell",
            Text,
            "Default remote shell",
            "Shell command used for new SSH sessions when a host doesn't specify one."
        ),
        field!(
            hosts.keepalive,
            "keepalive",
            Json,
            "SSH keepalive",
            "Keepalive interval / max-missed settings for SSH sessions."
        ),
        field!(
            hosts.ssh_config_import,
            "sshConfigImport",
            Json,
            "SSH config import",
            "Whether to import hosts from ~/.ssh/config on startup, and from where."
        ),
        field!(
            hosts.layout,
            "layout",
            Select(&[("grid", "Grid"), ("list", "List")]),
            "Host Manager layout",
            "Card grid or list layout for the Host Manager."
        ),
        field!(
            hosts.sort,
            "sort",
            Select(&[
                ("last_connected", "Last connected"),
                ("name", "Name"),
                ("manual", "Manual")
            ]),
            "Host Manager sort",
            "Default sort order for saved hosts."
        ),
        field!(
            hosts.card_scale,
            "cardScale",
            Int { min: 50, max: 200, step: 10 },
            "Host card scale",
            "Size of host cards in the Host Manager grid (%)."
        ),
        // ── keymap ───────────────────────────────────────────────────────
        field!(
            keymap.base_keymap,
            "baseKeymap",
            Select(&[
                ("native", "Native"),
                ("vsCode", "VS Code"),
                ("jetBrains", "JetBrains")
            ]),
            "Base keymap",
            "Preset a fresh install (or a reset) seeds shortcuts from."
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use labonair_settings_content::areas::AREAS;
    use std::collections::HashSet;

    /// Structural sanity: every `json_path` is `area.localKey` and every
    /// `area` segment is a real `SettingsContent` top-level field (reuses the
    /// same `FIELD_NAMES` contract `labonair-settings-content`'s own tests
    /// enforce).
    #[test]
    fn every_field_area_is_a_real_settings_content_module() {
        let areas: HashSet<&str> = AREAS.iter().map(|a| a.target_module).collect();
        for f in all_fields() {
            assert!(
                areas.contains(f.area()),
                "field `{}` has unknown area `{}`",
                f.json_path,
                f.area()
            );
        }
    }

    #[test]
    fn json_paths_are_unique() {
        let fields = all_fields();
        let mut seen = HashSet::new();
        for f in &fields {
            assert!(
                seen.insert(f.json_path),
                "duplicate json_path `{}`",
                f.json_path
            );
        }
    }

    #[test]
    fn get_set_round_trips_on_the_default_tree() {
        let mut content = SettingsContent::defaults();
        for f in all_fields() {
            let Some(before) = (f.get)(&content) else {
                continue;
            };
            assert!(
                (f.set)(&mut content, before.clone()),
                "field `{}` rejected its own default value",
                f.json_path
            );
            let after = (f.get)(&content);
            assert_eq!(
                after,
                Some(before),
                "field `{}` did not round-trip",
                f.json_path
            );
        }
    }

    #[test]
    fn set_rejects_wrong_typed_values() {
        let mut content = SettingsContent::defaults();
        let fields = all_fields();
        let font_size = fields
            .iter()
            .find(|f| f.json_path == "terminal.terminalFontSize")
            .unwrap();
        assert!(!(font_size.set)(&mut content, Value::String("huge".into())));
        assert_eq!(content.terminal.terminal_font_size, Some(15));
    }

    #[test]
    fn select_tokens_deserialize_into_the_real_field() {
        let mut content = SettingsContent::defaults();
        for f in all_fields() {
            if let FieldControl::Select(opts) = f.control {
                for (token, _label) in opts {
                    assert!(
                        (f.set)(&mut content, Value::String((*token).to_string())),
                        "field `{}` rejects its own option token `{}`",
                        f.json_path,
                        token
                    );
                }
            }
        }
    }
}
