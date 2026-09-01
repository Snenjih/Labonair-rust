//! Labonair UI components and theme provider.
//!
//! Populated by later phases (T04+). T02-002 adds the runtime theme provider.

pub mod app_shell;
pub mod background;
pub mod diff;
pub mod editor;
pub mod explorer;
pub mod git;
pub mod git_graph;
pub mod hosts;
pub mod menu;
pub mod notifications;
pub mod pane;
pub mod sftp;
pub mod syntax_theme;
pub mod tabs;
pub mod terminal;
pub mod theme;
pub mod transfers;
pub mod window_state;
pub mod workspace;

pub use app_shell::{AppShell, SidebarPanel};
pub use background::{
    background_store, init as init_background, BackgroundFit, BackgroundStore, BackgroundTarget,
    GlobalBackground, LayerScope,
};
pub use diff::{DiffLayout, DiffView};
pub use editor::{EditorEvent, EditorView};
pub use explorer::{DraggedPaths, ExplorerView};
pub use git::GitPanelView;
pub use git_graph::GitGraphView;
pub use hosts::{HostManagerEvent, HostManagerView, HostStatus};
pub use labonair_theme::{ThemeFile, ThemeFileVariant};
pub use menu::init as init_menus;
pub use notifications::{
    init as init_notifications, notification_center, notify_err, GlobalNotificationCenter,
    Notification, NotificationAction, NotificationCenter, Severity,
};
pub use pane::{CloseOutcome, PaneId, PaneNode, SplitAxis, WorkspaceLayout};
pub use sftp::{SftpEvent, SftpView};
pub use syntax_theme::EditorPalette;
pub use tabs::{Tab, TabData, TabKind, TabStore};
pub use terminal::TerminalView;
pub use theme::{
    active_theme, init as init_theme, init_fonts, theme_store, EditorThemeId, GlobalTheme,
    ThemeMode, ThemePreference, ThemeStore,
};
pub use transfers::{TransferBusEvent, TransfersEvent, TransfersView};
pub use workspace::Workspace;
