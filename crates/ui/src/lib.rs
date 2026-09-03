//! Labonair UI components and theme provider.
//!
//! Populated by later phases (T04+). T02-002 adds the runtime theme provider.

pub mod app_shell;
pub mod assets;
pub mod background;
pub mod bar_items;
pub mod cwd_breadcrumb;
pub mod diff;
pub mod editor;
pub mod live_bridge;
pub mod markdown;
pub mod menu;
pub mod pane;
pub mod preview;
pub mod session;
pub mod sftp;
pub mod sidebar_slot;
pub mod syntax_theme;
pub mod tabs;
pub mod terminal;
pub mod theme;
pub mod transfers;
pub mod updater;
pub mod window_state;
pub mod workspace;

pub use app_shell::{AppShell, SidebarPanel};
pub use assets::Assets;
pub use background::{
    background_store, init as init_background, BackgroundFit, BackgroundStore, BackgroundTarget,
    GlobalBackground, LayerScope,
};
pub use diff::{DiffLayout, DiffView};
pub use editor::{EditorEvent, EditorView};
pub use labonair_command_palette::{
    command_for_shortcut, effective_binding, find_conflict, resolve_conflict, shortcut,
    shortcut_from_slug, shortcut_slug, shortcuts, CommandId, CommandPalette, Conflict, KeybindMap,
    PaletteEvent, ShortcutId,
};
pub use labonair_hosts_ui::{HostManagerEvent, HostManagerView, HostStatus};
pub use labonair_notifications::{
    init as init_notifications, notification_center, notify_err, GlobalNotificationCenter,
    Notification, NotificationAction, NotificationCenter, Severity,
};
pub use labonair_panel_ai::{
    init as init_ai_chat, AgentAccessEntry, AgentAccessStore, AiChatStore, AiChatView, Attachment,
    AttachmentKind,
};
pub use labonair_panel_explorer::{BookmarkEvent, BookmarksView, DraggedPaths, ExplorerView};
pub use labonair_panel_git_graph::GitGraphView;
pub use labonair_panel_scm::GitPanelView;
pub use labonair_panel_snippets::{
    extract_snippet_variables, parse_tags, serialize_tags, substitute_snippet_variables,
    SnippetVariable, SnippetsView,
};
pub use labonair_settings_ui::{
    FieldDef, FieldKind, GlobalPreferences, PreferencesStore, SettingsView,
    CATEGORIES as SETTINGS_CATEGORIES, FIELDS,
};
pub use labonair_theme::{ThemeFile, ThemeFileVariant};
pub use labonair_ui_kit::{
    button, field_input, file_icon, folder_icon, text_field, ButtonSize, ButtonVariant, IconName,
};
pub use menu::{apply_keybinds, init as init_menus};
pub use pane::{CloseOutcome, PaneId, PaneNode, SplitAxis, WorkspaceLayout};
pub use preview::{is_previewable, PreviewView};
pub use session::{
    clear_snapshot, load_snapshot, save_snapshot, RestoreResult, SessionSnapshot, TabSnapshot,
};
pub use sftp::{SftpEvent, SftpView};
pub use syntax_theme::EditorPalette;
pub use tabs::{Tab, TabData, TabKind, TabStore};
pub use terminal::TerminalView;
pub use theme::{
    active_theme, init as init_theme, init_fonts, theme_store, EditorThemeId, FontOverrides,
    GlobalTheme, ThemeMode, ThemePreference, ThemeStore,
};
pub use transfers::{TransferBusEvent, TransfersEvent, TransfersView};
pub use updater::{UpdaterStatus, UpdaterView};
pub use workspace::Workspace;
