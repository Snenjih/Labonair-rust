//! Icon system — Zed-parity (`docs/architecture.md` §8.19).
//!
//! Two strictly separated concerns, mirroring Zed:
//!
//! 1. **UI / chrome icons** — the [`IconName`] enum below. One variant per SVG
//!    in `crates/shell/assets/icons/*.svg`, a verbatim vendored copy of Zed's
//!    Lucide-derived set (`zed-refrence/zed/assets/icons`, ISC) plus a small
//!    `// + Labonair addition` set for glyphs Zed has no equivalent for
//!    (dock-panel toggles, `house`, `shield`, …). Names follow Zed's
//!    `strum(snake_case)` stems; [`IconName`] also carries back-compat aliases
//!    (assoc. consts) for the port's earlier semantic names.
//! 2. **File / folder icons** — a swappable *icon theme*
//!    ([`labonair_theme::icon_theme::IconThemeContent`]). The free functions
//!    [`file_icon_path`] / [`folder_icon_path`] / [`chevron_icon_path`] /
//!    [`icon_for_path`] resolve a path against the active theme and return an
//!    asset path string; render it with [`svg_path`].
//!
//! [Lucide]: https://lucide.dev

use gpui::{px, svg, Hsla, SharedString, Styled, Svg};
use labonair_theme::icon_theme::IconThemeContent;

macro_rules! icon_enum {
    ($($variant:ident => $file:literal),* $(,)?) => {
        /// A bundled UI icon. Every variant maps to `icons/<file>.svg` in the
        /// asset bundle (`crates/shell/assets/icons/`, served by
        /// `labonair_shell::Assets`).
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum IconName {
            $($variant),*
        }

        impl IconName {
            /// The asset path passed to `gpui::svg().path(..)`.
            pub fn path(self) -> &'static str {
                match self {
                    $(IconName::$variant => concat!("icons/", $file, ".svg")),*
                }
            }

            /// Every variant, in declaration order. Consumed by the asset
            /// round-trip test in `labonair-shell` (the SVG bundle lives there).
            pub const ALL: &'static [IconName] = &[$(IconName::$variant),*];

            /// Resolve an icon *glyph id* (the snake_case SVG stem, e.g.
            /// `"file_code"`) to its [`IconName`]. `None` for an unknown id.
            pub fn from_glyph_id(id: &str) -> Option<IconName> {
                match id {
                    $($file => Some(IconName::$variant),)*
                    _ => None,
                }
            }
        }
    };
}

icon_enum! {
AcpRegistry => "acp_registry",
AiAnthropic => "ai_anthropic",
AiAnthropicCompat => "ai_anthropic_compat",
AiBedrock => "ai_bedrock",
AiClaude => "ai_claude",
AiDeepSeek => "ai_deep_seek",
AiEdit => "ai_edit",
AiGemini => "ai_gemini",
AiGoogle => "ai_google",
AiLlamaCpp => "ai_llama_cpp",
AiLmStudio => "ai_lm_studio",
AiMistral => "ai_mistral",
AiOllama => "ai_ollama",
AiOpenAi => "ai_open_ai",
AiOpenAiCompat => "ai_open_ai_compat",
AiOpenAiGptSub => "ai_open_ai_gpt_sub",
AiOpenCode => "ai_open_code",
AiOpenRouter => "ai_open_router",
AiVercel => "ai_vercel",
AiXAi => "ai_x_ai",
AiZed => "ai_zed",
Archive => "archive",
ArrowCircle => "arrow_circle",
ArrowDown => "arrow_down",
ArrowDown10 => "arrow_down10",
ArrowDownRight => "arrow_down_right",
ArrowDownUp => "arrow_down_up", // + Labonair addition
ArrowLeft => "arrow_left",
ArrowRight => "arrow_right",
ArrowRightLeft => "arrow_right_left",
ArrowUp => "arrow_up",
ArrowUpRight => "arrow_up_right",
AtSign => "at_sign",
Attach => "attach",
AudioOff => "audio_off",
AudioOn => "audio_on",
Backspace => "backspace",
Bell => "bell",
BellDot => "bell_dot",
BellOff => "bell_off",
BellRing => "bell_ring",
Binary => "binary",
Bitbucket => "bitbucket",
Blocks => "blocks",
BoltFilled => "bolt_filled",
BoltOutlined => "bolt_outlined",
Book => "book",
BookCopy => "book_copy",
Bookmark => "bookmark",
Box => "box",
BoxOpen => "box_open",
CaseSensitive => "case_sensitive",
Chat => "chat",
Check => "check",
CheckDouble => "check_double",
ChevronDown => "chevron_down",
ChevronDownUp => "chevron_down_up",
ChevronLeft => "chevron_left",
ChevronRight => "chevron_right",
ChevronUp => "chevron_up",
ChevronUpDown => "chevron_up_down",
Circle => "circle",
CircleCheck => "circle_check", // + Labonair addition
CircleHelp => "circle_help",
Clock => "clock",
Close => "close",
CloudDownload => "cloud_download",
Code => "code",
Codeberg => "codeberg",
Command => "command",
Compact => "compact",
Control => "control",
Copilot => "copilot",
CopilotDisabled => "copilot_disabled",
CopilotError => "copilot_error",
CopilotInit => "copilot_init",
Copy => "copy",
CountdownTimer => "countdown_timer",
Crosshair => "crosshair",
CursorIBeam => "cursor_i_beam",
Dash => "dash",
DatabaseZap => "database_zap",
Debug => "debug",
DebugBreakpoint => "debug_breakpoint",
DebugContinue => "debug_continue",
DebugContinueThread => "debug_continue_thread",
DebugDetach => "debug_detach",
DebugDisabledBreakpoint => "debug_disabled_breakpoint",
DebugDisabledLogBreakpoint => "debug_disabled_log_breakpoint",
DebugIgnoreBreakpoints => "debug_ignore_breakpoints",
DebugLogBreakpoint => "debug_log_breakpoint",
DebugPause => "debug_pause",
DebugStepInto => "debug_step_into",
DebugStepOut => "debug_step_out",
DebugStepOver => "debug_step_over",
Diff => "diff",
DiffSplit => "diff_split",
DiffSplitAuto => "diff_split_auto",
DiffUnified => "diff_unified",
Disconnected => "disconnected",
Download => "download",
EditorAtom => "editor_atom",
EditorCursor => "editor_cursor",
EditorEmacs => "editor_emacs",
EditorJetBrains => "editor_jet_brains",
EditorSublime => "editor_sublime",
EditorVsCode => "editor_vs_code",
Ellipsis => "ellipsis",
Envelope => "envelope",
Eraser => "eraser",
Escape => "escape",
Exit => "exit",
ExpandDown => "expand_down",
ExpandUp => "expand_up",
ExpandVertical => "expand_vertical",
Eye => "eye",
EyeOff => "eye_off",
FastForward => "fast_forward",
FastForwardOff => "fast_forward_off",
File => "file",
FileCode => "file_code",
FileCodeOff => "file_code_off",
FileDiff => "file_diff",
FileDoc => "file_doc",
FileGeneric => "file_generic",
FileGit => "file_git",
FileIgnored => "file_ignored",
FileLock => "file_lock",
FileMarkdown => "file_markdown",
FileMultiple => "file_multiple",
FileRust => "file_rust",
FileTextFilled => "file_text_filled",
FileTextOutlined => "file_text_outlined",
FileToml => "file_toml",
FileTree => "file_tree",
Filter => "filter",
FilterFunnel => "filter_funnel",
Flame => "flame",
FoldVertical => "fold_vertical",
Folder => "folder",
FolderAdd => "folder_add",
FolderInclude => "folder_include",
FolderOpen => "folder_open",
FolderSearch => "folder_search",
FolderShare => "folder_share",
FolderShared => "folder_shared",
Font => "font",
FontSize => "font_size",
FontWeight => "font_weight",
Forgejo => "forgejo",
ForwardArrow => "forward_arrow",
ForwardArrowUp => "forward_arrow_up",
GenericClose => "generic_close",
GenericMaximize => "generic_maximize",
GenericMinimize => "generic_minimize",
GenericRestore => "generic_restore",
Gerrit => "gerrit",
GitBranch => "git_branch",
GitBranchPlus => "git_branch_plus",
GitCommit => "git_commit",
GitGraph => "git_graph",
GitMergeConflict => "git_merge_conflict",
GitWorktree => "git_worktree",
Gitea => "gitea",
Github => "github",
Gitlab => "gitlab",
Hash => "hash",
HistoryRerun => "history_rerun",
Home => "house", // + Labonair addition
Image => "image",
Inception => "inception",
Indicator => "indicator",
Info => "info",
Json => "json",
Keyboard => "keyboard",
LineHeight => "line_height",
Link => "link",
Linux => "linux",
ListCollapse => "list_collapse",
ListTodo => "list_todo",
ListTree => "list_tree",
ListX => "list_x",
LoadCircle => "load_circle",
LocationEdit => "location_edit",
Lock => "lock",
LockOff => "lock_off",
MagnifyingGlass => "magnifying_glass",
Maximize => "maximize",
MaximizeAlt => "maximize_alt",
Menu => "menu",
Mic => "mic",
MicMute => "mic_mute",
Minimize => "minimize",
Notepad => "notepad",
OnCall => "on_call",
Option => "option",
PageDown => "page_down",
PageUp => "page_up",
Palette => "palette", // + Labonair addition
PanelBottom => "panel_bottom", // + Labonair addition
PanelLeft => "panel_left", // + Labonair addition
PanelTop => "panel_top", // + Labonair addition
Paperclip => "paperclip",
Pencil => "pencil",
PencilUnavailable => "pencil_unavailable",
Person => "person",
Pin => "pin",
PlayFilled => "play_filled",
PlayOutlined => "play_outlined",
Plus => "plus",
Power => "power",
Public => "public",
PullRequest => "pull_request",
QueueMessage => "queue_message",
Quote => "quote",
Reader => "reader",
RefreshTitle => "refresh_title",
Regex => "regex",
ReplNeutral => "repl_neutral",
Replace => "replace",
ReplaceAll => "replace_all",
ReplaceNext => "replace_next",
ReplyArrowRight => "reply_arrow_right",
Rerun => "rerun",
Return => "return",
RotateCcw => "rotate_ccw",
RotateCw => "rotate_cw",
Scissors => "scissors",
Screen => "screen",
SelectAll => "select_all",
Send => "send",
Server => "server",
Settings => "settings",
Share => "share",
Shield => "shield", // + Labonair addition
Shift => "shift",
SignalHigh => "signal_high",
SignalLow => "signal_low",
SignalMedium => "signal_medium",
Slash => "slash",
Sourcehut => "sourcehut",
Space => "space",
Sparkle => "sparkle",
Split => "split",
SplitAlt => "split_alt",
Square => "square", // + Labonair addition
SquareDot => "square_dot",
SquareMinus => "square_minus",
SquarePlus => "square_plus",
Star => "star",
StarFilled => "star_filled",
Stop => "stop",
Tab => "tab",
Table => "table",
Terminal => "terminal",
TerminalAlt => "terminal_alt",
TextSnippet => "text_snippet",
TextUnwrap => "text_unwrap",
TextWrap => "text_wrap",
ThinkingMode => "thinking_mode",
ThinkingModeOff => "thinking_mode_off",
ThisWindow => "this_window",
Thread => "thread",
ThreadFromSummary => "thread_from_summary",
ThreadsSidebarLeftClosed => "threads_sidebar_left_closed",
ThreadsSidebarLeftOpen => "threads_sidebar_left_open",
ThreadsSidebarRightClosed => "threads_sidebar_right_closed",
ThreadsSidebarRightOpen => "threads_sidebar_right_open",
ThumbsDown => "thumbs_down",
ThumbsUp => "thumbs_up",
TodoComplete => "todo_complete",
TodoPending => "todo_pending",
TodoProgress => "todo_progress",
ToolCopy => "tool_copy",
ToolDeleteFile => "tool_delete_file",
ToolDiagnostics => "tool_diagnostics",
ToolHammer => "tool_hammer",
ToolNotification => "tool_notification",
ToolPencil => "tool_pencil",
ToolSearch => "tool_search",
ToolTerminal => "tool_terminal",
ToolThink => "tool_think",
ToolWeb => "tool_web",
Trash => "trash",
Triangle => "triangle",
TriangleRight => "triangle_right",
Undo => "undo",
Unpin => "unpin",
UserArrowUp => "user_arrow_up",
UserCheck => "user_check",
UserGroup => "user_group",
UserRoundPen => "user_round_pen",
Warning => "warning",
WholeWord => "whole_word",
XCircle => "x_circle",
XCircleFilled => "x_circle_filled",
ZedAgent => "zed_agent",
ZedAgentTwo => "zed_agent_two",
ZedAssistant => "zed_assistant",
ZedPredict => "zed_predict",
ZedPredictDisabled => "zed_predict_disabled",
ZedPredictDown => "zed_predict_down",
ZedPredictError => "zed_predict_error",
ZedPredictUp => "zed_predict_up",
ZedSrcCustom => "zed_src_custom",
ZedSrcExtension => "zed_src_extension",
}

/// Back-compat aliases: the port's earlier semantic names, mapped onto the
/// Zed-named variant that now carries the glyph. Call sites keep compiling
/// unchanged (all uses are expression position — there are no `match` arms on
/// these outside this crate).
#[allow(non_upper_case_globals)]
impl IconName {
    pub const Search: Self = Self::MagnifyingGlass;
    pub const X: Self = Self::Close;
    pub const Minus: Self = Self::Dash;
    pub const Sparkles: Self = Self::Sparkle;
    pub const Zap: Self = Self::BoltOutlined;
    pub const Refresh: Self = Self::RotateCw;
    pub const MessageSquare: Self = Self::Chat;
    pub const CircleX: Self = Self::XCircle;
    pub const CornerDownRight: Self = Self::ReplyArrowRight;
    pub const GitCompare: Self = Self::Diff;
    pub const SquarePen: Self = Self::Pencil;
    pub const SquareCheck: Self = Self::Check;
    pub const Globe: Self = Self::Public;
    pub const FileJson: Self = Self::Json;
    pub const FileText: Self = Self::Reader;
    pub const FileTerminal: Self = Self::TerminalAlt;
    pub const Database: Self = Self::DatabaseZap;
    pub const Package: Self = Self::Box;
    pub const FolderTree: Self = Self::FileTree;
    pub const TriangleAlert: Self = Self::Warning;
    pub const Type: Self = Self::Font;
}

impl IconName {
    /// A `size-4` (16px) `svg()` element tinted `color`. Callers override
    /// `.size(..)` for other scales.
    pub fn svg(self, color: Hsla) -> Svg {
        svg()
            .path(self.path())
            .size(px(16.0))
            .flex_none()
            .text_color(color)
    }
}

/// A `size-4` (16px) `svg()` element for a raw asset path (an icon-theme
/// file/folder glyph), tinted `color`.
pub fn svg_path(path: impl Into<SharedString>, color: Hsla) -> Svg {
    svg()
        .path(path)
        .size(px(16.0))
        .flex_none()
        .text_color(color)
}

/// The file-icon asset path for `name` under `theme`.
pub fn file_icon_path(theme: &IconThemeContent, name: &str) -> SharedString {
    theme.file_icon_path(name).to_owned().into()
}

/// The folder-icon asset path for a directory named `name` under `theme`.
pub fn folder_icon_path(theme: &IconThemeContent, name: &str, expanded: bool) -> SharedString {
    theme.directory_icon_path(name, expanded).to_owned().into()
}

/// The disclosure-chevron asset path under `theme`.
pub fn chevron_icon_path(theme: &IconThemeContent, expanded: bool) -> SharedString {
    theme.chevron_icon_path(expanded).to_owned().into()
}

/// Resolve a filesystem entry to its icon-theme asset path — folder glyph for a
/// directory, file glyph otherwise.
pub fn icon_for_path(
    theme: &IconThemeContent,
    name: &str,
    is_dir: bool,
    expanded: bool,
) -> SharedString {
    if is_dir {
        folder_icon_path(theme, name, expanded)
    } else {
        file_icon_path(theme, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_icon_path_is_well_formed_and_unique() {
        let mut seen = HashSet::new();
        for icon in IconName::ALL {
            let path = icon.path();
            assert!(
                path.starts_with("icons/") && path.ends_with(".svg"),
                "malformed icon path for {icon:?}: {path}"
            );
            assert!(seen.insert(path), "duplicate icon path: {path}");
        }
    }

    #[test]
    fn aliases_resolve_to_real_variants() {
        assert_eq!(IconName::Search, IconName::MagnifyingGlass);
        assert_eq!(IconName::X, IconName::Close);
        assert_eq!(IconName::Refresh, IconName::RotateCw);
        assert_eq!(IconName::Globe, IconName::Public);
    }

    #[test]
    fn from_glyph_id_round_trips() {
        assert_eq!(
            IconName::from_glyph_id("file_code"),
            Some(IconName::FileCode)
        );
        assert_eq!(IconName::from_glyph_id("house"), Some(IconName::Home));
        assert_eq!(IconName::from_glyph_id("not-a-real-glyph"), None);
    }
}
