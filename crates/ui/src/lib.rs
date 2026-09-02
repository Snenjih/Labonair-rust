//! Labonair UI components and theme provider.
//!
//! Populated by later phases (T04+). T02-002 adds the runtime theme provider.

pub mod agent_access;
pub mod ai_chat;
pub mod app_shell;
pub mod assets;
pub mod background;
pub mod bar_items;
pub mod bell;
pub mod bookmarks;
pub mod command_palette;
pub mod components;
pub mod cwd_breadcrumb;
pub mod diff;
pub mod editor;
pub mod explorer;
pub mod git;
pub mod git_graph;
pub mod hosts;
pub mod markdown;
pub mod menu;
pub mod notifications;
pub mod pane;
pub mod preview;
pub mod session;
pub mod settings;
pub mod sftp;
pub mod sidebar_slot;
pub mod snippets;
pub mod ssh_connection;
pub mod syntax_theme;
pub mod tabs;
pub mod terminal;
pub mod theme;
pub mod transfers;
pub mod updater;
pub mod window_state;
pub mod workspace;

pub use agent_access::{AgentAccessEntry, AgentAccessStore};
pub use ai_chat::{init as init_ai_chat, AiChatStore, AiChatView, Attachment, AttachmentKind};
pub use app_shell::{AppShell, SidebarPanel};
pub use assets::Assets;
pub use background::{
    background_store, init as init_background, BackgroundFit, BackgroundStore, BackgroundTarget,
    GlobalBackground, LayerScope,
};
pub use bookmarks::{BookmarkEvent, BookmarksView};
pub use command_palette::{
    command_for_shortcut, effective_binding, find_conflict, resolve_conflict, shortcut,
    shortcut_from_slug, shortcut_slug, shortcuts, CommandId, CommandPalette, Conflict, KeybindMap,
    PaletteEvent, ShortcutId,
};
pub use components::{
    button, field_input, file_icon, folder_icon, text_field, ButtonSize, ButtonVariant, IconName,
};
pub use diff::{DiffLayout, DiffView};
pub use editor::{EditorEvent, EditorView};
pub use explorer::{DraggedPaths, ExplorerView};
pub use git::GitPanelView;
pub use git_graph::GitGraphView;
pub use hosts::{HostManagerEvent, HostManagerView, HostStatus};
pub use labonair_theme::{ThemeFile, ThemeFileVariant};
pub use menu::{apply_keybinds, init as init_menus};
pub use notifications::{
    init as init_notifications, notification_center, notify_err, GlobalNotificationCenter,
    Notification, NotificationAction, NotificationCenter, Severity,
};
pub use pane::{CloseOutcome, PaneId, PaneNode, SplitAxis, WorkspaceLayout};
pub use preview::{is_previewable, PreviewView};
pub use session::{
    clear_snapshot, load_snapshot, save_snapshot, RestoreResult, SessionSnapshot, TabSnapshot,
};
pub use settings::{
    FieldDef, FieldKind, GlobalPreferences, PreferencesStore, SettingsView,
    CATEGORIES as SETTINGS_CATEGORIES, FIELDS,
};
pub use sftp::{SftpEvent, SftpView};
pub use snippets::{
    extract_snippet_variables, parse_tags, serialize_tags, substitute_snippet_variables,
    SnippetVariable, SnippetsView,
};
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
