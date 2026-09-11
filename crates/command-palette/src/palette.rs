//! Command registry + the [`CommandPalette`] modal overlay view.
//!
//! * **Data** — a UI adapter for the injected, UI-free command registry
//!   descriptors (id / title / section / contexts / default binding) plus
//!   pure filtering / search helpers.
//! * **View** — [`CommandPalette`], a modal overlay opened with `Cmd+P`. Type
//!   to filter, arrow keys to move, `Enter` to run, `Esc` to close. Commands
//!   that need an argument ("Switch Tab\u{2026}") push a follow-up page.
//!
//! Execution: the palette does not own the app state — on `Enter` it emits
//! [`PaletteEvent`], which the composition root forwards to the owner action
//! registry. Pref/theme-derived scalars are read
//! straight from the layered `labonair-settings` slices (see the "Settings
//! reads" section below); [`PaletteWorkspace`] / [`UiTheme`] remain the
//! generic host contracts for everything else.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    Hsla, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};

use labonair_settings::content::workspace::PaletteSearchMode as ContentSearchMode;
use labonair_settings::{
    EditorSettings, Settings as _, TerminalSettings, ThemeSettings, WorkspaceSettings,
};
use labonair_theme::{EditorThemeId, ThemePreference};
use labonair_ui_kit::{kbd, keybinding_hint, IconName, Palette, UiTheme};

use crate::fuzzy::{match_score, SearchMode};
use crate::KeybindDisplay;
use labonair_command_palette_core::{
    toggle_pref_key, CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandSubmenu,
    PaletteAction, SubmenuAction, SubmenuItem, SubmenuRegistry,
};

// ─────────────────────────────────────────────────────────────────────────────
// Settings reads (T-block3: the palette reads its slice of the layered
// `SettingsStore` directly — the old `PalettePrefs` host-contract trait is gone)
// ─────────────────────────────────────────────────────────────────────────────

fn palette_search_mode_setting(cx: &App) -> SearchMode {
    match WorkspaceSettings::try_get(cx).map(|s| s.command_palette_search_mode()) {
        Some(ContentSearchMode::StartsWith) => SearchMode::StartsWith,
        Some(ContentSearchMode::Fuzzy) => SearchMode::Fuzzy,
        _ => SearchMode::Contains,
    }
}

fn set_palette_search_mode_setting(mode: SearchMode, cx: &mut App) {
    let mapped = match mode {
        SearchMode::Contains => ContentSearchMode::Contains,
        SearchMode::StartsWith => ContentSearchMode::StartsWith,
        SearchMode::Fuzzy => ContentSearchMode::Fuzzy,
    };
    if let Some(store) = cx.try_global::<labonair_settings::SettingsStore>() {
        // `SettingsStore` is a global; mutate through `global_mut`.
        let _ = store;
        let _ = cx
            .global_mut::<labonair_settings::SettingsStore>()
            .update_user_settings(move |c| c.workspace.command_palette_search_mode = Some(mapped));
    }
}

fn palette_history_size(cx: &App) -> u32 {
    WorkspaceSettings::try_get(cx)
        .map(|s| s.command_palette_history_size())
        .unwrap_or(5)
}

fn palette_opacity(cx: &App) -> u32 {
    WorkspaceSettings::try_get(cx)
        .map(|s| s.command_palette_opacity())
        .unwrap_or(95)
}

fn palette_position(cx: &App) -> String {
    WorkspaceSettings::try_get(cx)
        .map(|s| s.command_palette_position().to_string())
        .unwrap_or_else(|| "top".to_string())
}

fn palette_show_recent(cx: &App) -> bool {
    WorkspaceSettings::try_get(cx)
        .map(|s| s.command_palette_show_recent())
        .unwrap_or(true)
}

fn palette_close_on_overlay_click(cx: &App) -> bool {
    WorkspaceSettings::try_get(cx)
        .map(|s| s.command_palette_close_on_overlay_click())
        .unwrap_or(true)
}

fn palette_keybind_display(cx: &App) -> KeybindDisplay {
    cx.try_global::<KeybindDisplay>()
        .cloned()
        .unwrap_or_default()
}

/// Current value of the boolean setting a `Toggle: …` row flips.
fn palette_toggle_state(key: &str, cx: &App) -> bool {
    match key {
        "zenModeShowHeader" => ThemeSettings::try_get(cx)
            .map(|s| s.zen_mode_show_header())
            .unwrap_or(true),
        "zenModeShowStatusbar" => ThemeSettings::try_get(cx)
            .map(|s| s.zen_mode_show_statusbar())
            .unwrap_or(true),
        "editorWordWrap" => EditorSettings::try_get(cx)
            .map(|s| s.word_wrap())
            .unwrap_or(false),
        "editorLineNumbers" => EditorSettings::try_get(cx)
            .map(|s| s.line_numbers())
            .unwrap_or(true),
        "terminalCursorBlink" => TerminalSettings::try_get(cx)
            .map(|s| s.cursor_blink())
            .unwrap_or(true),
        "vimMode" => EditorSettings::try_get(cx)
            .map(|s| s.vim_mode())
            .unwrap_or(false),
        _ => false,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Host contracts (decoupling — the palette crate never names `crates/ui`)
// ─────────────────────────────────────────────────────────────────────────────

/// The tab-kind surface [`context_of`] maps to a [`CommandContext`]. The
/// palette crate owns this enum so it never has to name `crates/ui`'s
/// `TabKind`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaletteTabKind {
    Workspace,
    Editor,
    Sftp,
    Home,
    /// Any other tab kind — maps to no context.
    Other,
}

/// One open tab, as the host workspace exposes it for the `Switch Tab\u{2026}`
/// sub-page.
#[derive(Clone, Debug)]
pub struct PaletteTabRow {
    pub id: u64,
    pub label: String,
    pub kind_title: String,
    pub is_ssh: bool,
}

/// The workspace surface the palette view reads.
pub trait PaletteWorkspace {
    /// The active tab's [`CommandContext`], if it maps to one.
    fn palette_active_context(&self, cx: &App) -> Option<CommandContext>;
    /// Every open tab, for the `Switch Tab\u{2026}` sub-page.
    fn palette_tab_rows(&self, cx: &App) -> Vec<PaletteTabRow>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Command registry adapter
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Sub-pages (port of the reference `CommandPage` registry — 11 named pages)
// ─────────────────────────────────────────────────────────────────────────────

/// Every palette page. `Root` is the command list; the rest are the
/// context-drill sub-pages the reference registers in `useCommandRegistry`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Page {
    Root,
    Tabs,
    ColorMode,
    EditorTheme,
    Themes,
    IconThemes,
    Hosts,
    Snippets,
    Outline,
    GitBranches,
    StatusBarHidden,
}

impl Page {
    /// The search-input placeholder for this page.
    pub fn placeholder(self) -> &'static str {
        match self {
            Page::Root => "Search commands\u{2026}",
            Page::Tabs => "Search open tabs\u{2026}",
            Page::ColorMode => "Search color modes\u{2026}",
            Page::EditorTheme => "Search editor themes\u{2026}",
            Page::Themes => "Search themes\u{2026}",
            Page::IconThemes => "Search icon themes\u{2026}",
            Page::Hosts => "Search hosts\u{2026}",
            Page::Snippets => "Search snippets\u{2026}",
            Page::Outline => "Search symbols\u{2026}",
            Page::GitBranches => "Search branches\u{2026}",
            Page::StatusBarHidden => "Search hidden items\u{2026}",
        }
    }

    /// The breadcrumb label for this page.
    pub fn label(self) -> &'static str {
        match self {
            Page::Root => "Commands",
            Page::Tabs => "Open Tabs",
            Page::ColorMode => "Color Mode",
            Page::EditorTheme => "Editor Theme",
            Page::Themes => "App Theme",
            Page::IconThemes => "Icon Themes",
            Page::Hosts => "Hosts",
            Page::Snippets => "Snippets",
            Page::Outline => "Symbols",
            Page::GitBranches => "Branches",
            Page::StatusBarHidden => "Hidden Status Bar Items",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Command {
    pub id: CommandId,
    pub title: String,
    pub section: String,
    pub aliases: Vec<String>,
    /// Empty = always available; otherwise only when the active context is
    /// listed (reference `filterByContext`).
    pub contexts: Vec<CommandContext>,
    /// First owner-contributed default binding, used before the shell has
    /// published a runtime snapshot (for example in isolated palette tests).
    pub default_binding: Option<String>,
    /// Leading icon (reference renders a Hugeicons glyph on every row).
    pub icon: IconName,
    /// If set, picking this row navigates to a sub-page instead of running.
    pub sub_page: Option<Page>,
}

impl Command {
    /// Adapt the UI-free descriptor into the palette's presentation model.
    pub fn from_descriptor(descriptor: CommandDescriptor) -> Self {
        Self {
            id: descriptor.id,
            title: descriptor.title,
            section: descriptor.section,
            aliases: descriptor.aliases,
            contexts: descriptor.contexts,
            default_binding: descriptor
                .default_bindings
                .first()
                .map(|binding| binding.keystrokes.clone()),
            icon: icon_for(descriptor.icon),
            sub_page: descriptor.submenu.map(page_for),
        }
    }
}

fn command_key_tokens(command: &Command, display: &KeybindDisplay) -> Vec<String> {
    display.keys_for(command.id, command.default_binding.as_deref())
}

fn icon_for(icon: CommandIcon) -> IconName {
    match icon {
        CommandIcon::Terminal => IconName::Terminal,
        CommandIcon::File => IconName::File,
        CommandIcon::Copy => IconName::Copy,
        CommandIcon::Close => IconName::X,
        CommandIcon::ChevronRight => IconName::ChevronRight,
        CommandIcon::ChevronDown => IconName::ChevronDown,
        CommandIcon::Trash => IconName::Trash,
        CommandIcon::Server => IconName::Server,
        CommandIcon::Folder => IconName::Folder,
        CommandIcon::Search => IconName::Search,
        CommandIcon::PanelLeft => IconName::PanelLeft,
        CommandIcon::Square => IconName::Square,
        CommandIcon::Sparkles => IconName::Sparkles,
        CommandIcon::Plus => IconName::Plus,
        CommandIcon::Minus => IconName::Minus,
        CommandIcon::Refresh => IconName::Refresh,
        CommandIcon::Command => IconName::Command,
        CommandIcon::GitBranch => IconName::GitBranch,
        CommandIcon::Edit => IconName::SquarePen,
        CommandIcon::FileCode => IconName::FileCode,
        CommandIcon::Eye => IconName::Eye,
        CommandIcon::Check => IconName::SquareCheck,
        CommandIcon::PanelTop => IconName::PanelTop,
        CommandIcon::PanelBottom => IconName::PanelBottom,
        CommandIcon::Download => IconName::Download,
        CommandIcon::Palette => IconName::Palette,
    }
}

fn page_for(page: CommandSubmenu) -> Page {
    match page {
        CommandSubmenu::Tabs => Page::Tabs,
        CommandSubmenu::RecentHosts => Page::Hosts,
        CommandSubmenu::ColorMode => Page::ColorMode,
        CommandSubmenu::EditorTheme => Page::EditorTheme,
        CommandSubmenu::Themes => Page::Themes,
        CommandSubmenu::IconThemes => Page::IconThemes,
        CommandSubmenu::Hosts => Page::Hosts,
        CommandSubmenu::Snippets => Page::Snippets,
        CommandSubmenu::Outline => Page::Outline,
        CommandSubmenu::GitBranches => Page::GitBranches,
        CommandSubmenu::StatusBarHidden => Page::StatusBarHidden,
    }
}

/// Commands are supplied by the shell's composition registry. The palette owns
/// presentation and filtering, but never defines the product command list.
fn available(commands: &[Command], ctx: Option<CommandContext>) -> Vec<&Command> {
    commands
        .iter()
        .filter(|command| match ctx {
            None => command.contexts.is_empty(),
            Some(active) => command.contexts.is_empty() || command.contexts.contains(&active),
        })
        .collect()
}

/// Search over title + section, restricted to what's available in the active context.
fn search_mode<'a>(
    commands: &'a [Command],
    query: &str,
    ctx: Option<CommandContext>,
    mode: SearchMode,
) -> Vec<&'a Command> {
    let mut scored: Vec<(i64, usize, &Command)> = available(commands, ctx)
        .into_iter()
        .enumerate()
        .filter_map(|(index, command)| {
            let mut haystack = String::with_capacity(command.title.len() + command.section.len() + 1);
            haystack.push_str(&command.title);
            haystack.push(' ');
            haystack.push_str(&command.section);
            for alias in &command.aliases {
                haystack.push(' ');
                haystack.push_str(alias);
            }
            match_score(mode, &haystack, query).map(|score| (score, index, command))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, _, command)| command).collect()
}

fn command(commands: &[Command], id: CommandId) -> Option<&Command> {
    commands.iter().find(|command| command.id == id)
}

/// Map the active palette tab to the registry context used for filtering.
pub fn context_of(kind: PaletteTabKind, is_ssh: bool) -> Option<CommandContext> {
    Some(match kind {
        PaletteTabKind::Workspace if is_ssh => CommandContext::SshTerminal,
        PaletteTabKind::Workspace => CommandContext::Terminal,
        PaletteTabKind::Editor => CommandContext::Editor,
        PaletteTabKind::Sftp => CommandContext::Sftp,
        PaletteTabKind::Home => CommandContext::Home,
        PaletteTabKind::Other => return None,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// View
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted when the user picks something in the palette. The host shell
/// handles it.
#[derive(Clone, Debug)]
pub enum PaletteEvent {
    Run(CommandId),
    /// A dynamic row or transient preview action. The palette does not
    /// interpret this value; the composition root forwards it to the
    /// owner-registered action registry.
    Action(PaletteAction),
}

/// A dynamic choice rendered on a sub-page (tab, host, session, branch…).
/// Live application state the palette needs for its dynamic sub-pages and
/// `rightLabel` states. The host builds this once when the palette opens and
/// hands it over via [`CommandPalette::set_data`]. Volatile domain lists are
/// immutable snapshots in [`SubmenuRegistry`]; the palette does not read
/// feature entities or assemble feature-specific rows.
///
/// Slimmed in T17-007: the pref/theme-derived scalars (`color_mode`,
/// `editor_theme`, `font_size`, the toggle bools) are read straight from the
/// layered `SettingsStore` (see this module's "Settings reads" section).
/// What remains is the genuinely panel-/workspace-/settings-sourced
/// choice lists that the palette crate cannot pull itself without a crate
/// cycle (`labonair-panel-* → labonair-command-palette` back-edge,
/// `labonair-settings-ui` dependency).
#[derive(Clone, Debug, Default)]
pub struct PaletteData {
    /// Snapshot of the composition-root command registry. The palette never
    /// constructs this list itself.
    pub commands: Vec<Command>,
    /// Runtime-backed submenu snapshots supplied by the owning capabilities.
    /// The palette never rebuilds feature-specific lists itself.
    pub submenus: SubmenuRegistry,
}

/// Persisted "recently used" command ids (mirrors the reference
/// `labonair-palette-recent` localStorage list). Stored as debug-formatted
/// Stable action names in `command-palette-recent.json` in the config dir.
mod recent {
    use super::CommandId;

    fn path() -> std::path::PathBuf {
        labonair_filesystem::paths::config_dir().join("command-palette-recent.json")
    }

    pub fn load() -> Vec<CommandId> {
        let Ok(raw) = std::fs::read_to_string(path()) else {
            return Vec::new();
        };
        let ids: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        ids.iter().filter_map(|s| from_slug(s)).collect()
    }

    pub fn save(ids: &[CommandId]) {
        let slugs: Vec<&str> = ids.iter().map(|id| id.action_name()).collect();
        if let Ok(json) = serde_json::to_string(&slugs) {
            let _ = std::fs::write(path(), json);
        }
    }

    fn from_slug(s: &str) -> Option<CommandId> {
        CommandId::from_action_name(s)
    }
}

#[derive(Clone)]
enum RowKey {
    Command(CommandId),
    Navigate(Page),
    Tab(u64),
    SetColorMode(ThemePreference),
    SetEditorTheme(EditorThemeId),
    ConnectHost {
        host_id: String,
        sftp: bool,
    },
    /// Activate a JSON app theme by id (`"default"` = built-in).
    SetAppTheme(String),
    /// Activate a registered icon theme by id.
    SetIconTheme(String),
    /// Run a saved snippet by id with its default execution mode.
    RunSnippet(String),
    /// Check out a git branch by name.
    SwitchBranch(String),
    /// Jump the active editor's caret to a 0-based line (Go to Symbol).
    GoToLine(usize),
    /// Un-hide a status-bar item by id.
    ShowStatusBarItem(String),
    /// Non-actionable (empty-state placeholder line).
    Noop,
}

/// A `Shift+Enter` alternate action for a row (e.g. "Open SFTP" on a host
/// row whose primary action is "Open SSH").
#[derive(Clone)]
struct SecondaryAction {
    label: String,
    key: RowKey,
}

struct PaletteRow {
    key: RowKey,
    /// Optional `Shift+Enter` action; a footer hint is shown when present.
    secondary: Option<SecondaryAction>,
    icon: Option<IconName>,
    title: String,
    subtitle: Option<String>,
    section: String,
    keys: Vec<String>,
    right_label: Option<String>,
    has_sub: bool,
}

fn row_key_for_action(action: &SubmenuAction) -> RowKey {
    match action {
        SubmenuAction::RunCommand(id) => RowKey::Command(*id),
        SubmenuAction::SwitchToTab(id) => RowKey::Tab(*id),
        SubmenuAction::ConnectHost { host_id, sftp } => RowKey::ConnectHost {
            host_id: host_id.clone(),
            sftp: *sftp,
        },
        SubmenuAction::SetColorMode(mode) => match mode.as_str() {
            "dark" => RowKey::SetColorMode(ThemePreference::Dark),
            "light" => RowKey::SetColorMode(ThemePreference::Light),
            "system" => RowKey::SetColorMode(ThemePreference::System),
            _ => RowKey::Noop,
        },
        SubmenuAction::SetEditorTheme(id) => EditorThemeId::from_slug(id)
            .map(RowKey::SetEditorTheme)
            .unwrap_or(RowKey::Noop),
        SubmenuAction::SetAppTheme(id) => RowKey::SetAppTheme(id.clone()),
        SubmenuAction::SetIconTheme(id) => RowKey::SetIconTheme(id.clone()),
        SubmenuAction::RunSnippet(id) => RowKey::RunSnippet(id.clone()),
        SubmenuAction::SwitchBranch(name) => RowKey::SwitchBranch(name.clone()),
        SubmenuAction::GoToLine(line) => RowKey::GoToLine(*line),
        SubmenuAction::ShowStatusBarItem(id) => RowKey::ShowStatusBarItem(id.clone()),
    }
}

/// The backdrop fill for the palette overlay. The reference paints every
/// `DialogOverlay` with `bg-black/30`, theme-independent.
fn modal_scrim() -> Hsla {
    gpui::black().opacity(0.30)
}

/// The Cmd+P command palette overlay.
pub struct CommandPalette<W, Th> {
    theme: Entity<Th>,
    workspace: Entity<W>,
    open: bool,
    /// Navigation stack — `[Root]` at rest, pushed on drill-in.
    pages: Vec<Page>,
    query: String,
    selected: usize,
    recent: Vec<CommandId>,
    data: PaletteData,
    focus: FocusHandle,
}

impl<W, Th> EventEmitter<PaletteEvent> for CommandPalette<W, Th>
where
    W: 'static,
    Th: 'static,
{
}

/// Emitted so a hosting [`ModalLayer`](labonair_workspace::modal_layer::ModalLayer)
/// can drop the palette when it closes itself (Esc / overlay click / a pick).
impl<W, Th> EventEmitter<DismissEvent> for CommandPalette<W, Th>
where
    W: 'static,
    Th: 'static,
{
}

impl<W, Th> CommandPalette<W, Th>
where
    W: PaletteWorkspace + 'static,
    Th: UiTheme + 'static,
{
    pub fn new(theme: Entity<Th>, workspace: Entity<W>, cx: &mut Context<Self>) -> Self {
        Self {
            theme,
            workspace,
            open: false,
            pages: vec![Page::Root],
            query: String::new(),
            selected: 0,
            recent: recent::load(),
            data: PaletteData::default(),
            focus: cx.focus_handle(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Refresh the live-state snapshot (called from the host's `render`).
    pub fn set_data(&mut self, data: PaletteData) {
        self.data = data;
    }

    fn page(&self) -> Page {
        *self.pages.last().unwrap_or(&Page::Root)
    }

    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.close(cx);
        } else {
            self.open(window, cx);
        }
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.pages = vec![Page::Root];
        self.query.clear();
        self.selected = 0;
        window.focus(&self.focus);
        cx.notify();
    }

    /// Open the palette navigated straight to `page` (used by `Cmd+Shift+N` →
    /// the Hosts page).
    pub fn open_to_page(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        self.open(window, cx);
        if page != Page::Root {
            self.pages.push(page);
            self.sync_theme_preview(cx);
        }
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        let was_open = self.open;
        self.open = false;
        self.query.clear();
        self.pages = vec![Page::Root];
        self.selected = 0;
        cx.emit(PaletteEvent::Action(PaletteAction::PreviewAppTheme(None)));
        cx.emit(PaletteEvent::Action(PaletteAction::PreviewIconTheme(None)));
        if was_open {
            cx.emit(DismissEvent);
        }
        cx.notify();
    }

    fn navigate(&mut self, page: Page, cx: &mut Context<Self>) {
        self.pages.push(page);
        self.query.clear();
        self.selected = 0;
        self.sync_theme_preview(cx);
        cx.notify();
    }

    fn go_back(&mut self, cx: &mut Context<Self>) {
        if self.pages.len() > 1 {
            self.pages.pop();
            self.query.clear();
            self.selected = 0;
            self.sync_theme_preview(cx);
            cx.notify();
        }
    }

    fn go_back_to(&mut self, index: usize, cx: &mut Context<Self>) {
        if index + 1 < self.pages.len() {
            self.pages.truncate(index + 1);
            self.query.clear();
            self.selected = 0;
            self.sync_theme_preview(cx);
            cx.notify();
        }
    }

    /// Emit live previews for the highlighted row on either theme page, or
    /// revert both previews on any other page. Called on every selection /
    /// navigation change while the palette is open.
    fn sync_theme_preview(&mut self, cx: &mut Context<Self>) {
        if matches!(self.page(), Page::Themes) {
            let rows = self.rows(cx);
            if let Some(RowKey::SetAppTheme(id)) = rows.get(self.selected).map(|r| r.key.clone()) {
                cx.emit(PaletteEvent::Action(PaletteAction::PreviewAppTheme(Some(
                    id,
                ))));
                cx.emit(PaletteEvent::Action(PaletteAction::PreviewIconTheme(None)));
                return;
            }
        }
        if matches!(self.page(), Page::IconThemes) {
            let rows = self.rows(cx);
            if let Some(RowKey::SetIconTheme(id)) = rows.get(self.selected).map(|r| r.key.clone()) {
                cx.emit(PaletteEvent::Action(PaletteAction::PreviewAppTheme(None)));
                cx.emit(PaletteEvent::Action(PaletteAction::PreviewIconTheme(Some(
                    id,
                ))));
                return;
            }
        }
        cx.emit(PaletteEvent::Action(PaletteAction::PreviewAppTheme(None)));
        cx.emit(PaletteEvent::Action(PaletteAction::PreviewIconTheme(None)));
    }

    fn active_context(&self, cx: &App) -> Option<CommandContext> {
        self.workspace.read(cx).palette_active_context(cx)
    }

    fn search_mode(&self, cx: &App) -> SearchMode {
        palette_search_mode_setting(cx)
    }

    fn push_recent(&mut self, id: CommandId, cx: &App) {
        let max = palette_history_size(cx).max(1) as usize;
        self.recent.retain(|&r| r != id);
        self.recent.insert(0, id);
        self.recent.truncate(max);
        recent::save(&self.recent);
    }

    /// Rows for a registry-owned dynamic submenu, filtered by the current
    /// query. The palette only maps typed actions to presentation events.
    fn submenu_rows(
        &self,
        submenu: CommandSubmenu,
        section: &str,
        icon: IconName,
        mode: SearchMode,
        empty_hint: &str,
    ) -> Vec<PaletteRow> {
        let items = self
            .data
            .submenus
            .get(submenu)
            .map(|snapshot| snapshot.items.as_slice())
            .unwrap_or(&[]);
        if items.is_empty() {
            return vec![PaletteRow {
                key: RowKey::Noop,
                secondary: None,
                icon: None,
                title: empty_hint.to_string(),
                subtitle: None,
                section: section.to_string(),
                keys: vec![],
                right_label: None,
                has_sub: false,
            }];
        }
        let mut scored: Vec<(i64, usize, &SubmenuItem)> = items
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let hay = format!("{} {}", c.title, c.subtitle.as_deref().unwrap_or(""));
                match_score(mode, &hay, &self.query).map(|s| (s, i, c))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        scored
            .into_iter()
            .map(|(_, _, c)| PaletteRow {
                key: row_key_for_action(&c.action),
                secondary: c.secondary.as_ref().map(|secondary| SecondaryAction {
                    label: secondary.label.clone(),
                    key: row_key_for_action(&secondary.action),
                }),
                icon: Some(icon),
                title: c.title.clone(),
                subtitle: c.subtitle.clone(),
                section: section.to_string(),
                keys: vec![],
                right_label: c.active.then(|| "active".to_string()),
                has_sub: false,
            })
            .collect()
    }

    fn rows(&self, cx: &App) -> Vec<PaletteRow> {
        let mode = self.search_mode(cx);
        let display = palette_keybind_display(cx);
        match self.page() {
            Page::Root => {
                let ctx = self.active_context(cx);
                let tab_count = self.workspace.read(cx).palette_tab_rows(cx).len();
                let mut root: Vec<PaletteRow> =
                    search_mode(&self.data.commands, &self.query, ctx, mode)
                        .into_iter()
                        .map(|c| {
                            let right_label = toggle_pref_key(c.id).map(|k| {
                                if palette_toggle_state(k, cx) {
                                    "ON".to_string()
                                } else {
                                    "OFF".to_string()
                                }
                            });
                            let subtitle = match c.id {
                                CommandId::SwitchTab => Some(format!("{tab_count} open")),
                                _ => None,
                            };
                            PaletteRow {
                                key: c
                                    .sub_page
                                    .map(RowKey::Navigate)
                                    .unwrap_or(RowKey::Command(c.id)),
                                secondary: None,
                                icon: Some(c.icon),
                                title: c.title.to_string(),
                                subtitle,
                                section: c.section.to_string(),
                                keys: command_key_tokens(c, &display),
                                right_label,
                                has_sub: c.sub_page.is_some(),
                            }
                        })
                        .collect();
                if self
                    .data
                    .submenus
                    .get(CommandSubmenu::RecentHosts)
                    .is_some_and(|snapshot| !snapshot.items.is_empty())
                {
                    root.extend(self.submenu_rows(
                        CommandSubmenu::RecentHosts,
                        "Hosts",
                        IconName::Server,
                        mode,
                        "No hosts configured yet",
                    ));
                }
                root
            }
            Page::Tabs => self.submenu_rows(
                CommandSubmenu::Tabs,
                "Open Tabs",
                IconName::Terminal,
                mode,
                "No tabs open",
            ),
            Page::ColorMode => self.submenu_rows(
                CommandSubmenu::ColorMode,
                "Color Mode",
                IconName::Refresh,
                mode,
                "No color modes available",
            ),
            Page::EditorTheme => self.submenu_rows(
                CommandSubmenu::EditorTheme,
                "Editor Themes",
                IconName::Sparkles,
                mode,
                "No editor themes installed yet",
            ),
            Page::Themes => self.submenu_rows(
                CommandSubmenu::Themes,
                "App Themes",
                IconName::Sparkles,
                mode,
                "No themes installed yet",
            ),
            Page::IconThemes => self.submenu_rows(
                CommandSubmenu::IconThemes,
                "Icon Themes",
                IconName::Palette,
                mode,
                "No icon themes installed yet",
            ),
            Page::Hosts => self.submenu_rows(
                CommandSubmenu::Hosts,
                "Hosts",
                IconName::Server,
                mode,
                "No hosts configured yet",
            ),
            Page::Snippets => self.submenu_rows(
                CommandSubmenu::Snippets,
                "Snippets",
                IconName::Command,
                mode,
                "No snippets saved yet",
            ),
            Page::Outline => self.submenu_rows(
                CommandSubmenu::Outline,
                "Symbols",
                IconName::FileCode,
                mode,
                "No symbols found",
            ),
            Page::GitBranches => self.submenu_rows(
                CommandSubmenu::GitBranches,
                "Branches",
                IconName::GitBranch,
                mode,
                "No repository detected",
            ),
            Page::StatusBarHidden => self.submenu_rows(
                CommandSubmenu::StatusBarHidden,
                "Hidden Status Bar Items",
                IconName::Eye,
                mode,
                "No hidden status-bar items",
            ),
        }
    }

    fn run_selected(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.rows(cx);
        let Some(key) = rows.get(self.selected).map(|r| r.key.clone()) else {
            return;
        };
        self.dispatch(key, cx);
    }

    /// `Shift+Enter` — run the selected row's secondary action, if it has one.
    fn run_secondary(&mut self, cx: &mut Context<Self>) {
        let rows = self.rows(cx);
        let Some(sec) = rows.get(self.selected).and_then(|r| r.secondary.clone()) else {
            return;
        };
        self.dispatch(sec.key, cx);
    }

    fn dispatch(&mut self, key: RowKey, cx: &mut Context<Self>) {
        match key {
            RowKey::Noop => {}
            RowKey::Navigate(page) => self.navigate(page, cx),
            RowKey::Command(id) => {
                self.push_recent(id, cx);
                self.close(cx);
                cx.emit(PaletteEvent::Run(id));
            }
            RowKey::Tab(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::Action(PaletteAction::Submenu(
                    SubmenuAction::SwitchToTab(id),
                )));
            }
            RowKey::SetColorMode(p) => {
                self.close(cx);
                cx.emit(PaletteEvent::Action(PaletteAction::Submenu(
                    SubmenuAction::SetColorMode(
                        match p {
                            ThemePreference::System => "system",
                            ThemePreference::Light => "light",
                            ThemePreference::Dark => "dark",
                        }
                        .to_string(),
                    ),
                )));
            }
            RowKey::SetEditorTheme(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::Action(PaletteAction::Submenu(
                    SubmenuAction::SetEditorTheme(id.slug().to_string()),
                )));
            }
            RowKey::ConnectHost { host_id, sftp } => {
                self.close(cx);
                cx.emit(PaletteEvent::Action(PaletteAction::Submenu(
                    SubmenuAction::ConnectHost { host_id, sftp },
                )));
            }
            RowKey::SetAppTheme(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::Action(PaletteAction::Submenu(
                    SubmenuAction::SetAppTheme(id),
                )));
            }
            RowKey::SetIconTheme(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::Action(PaletteAction::Submenu(
                    SubmenuAction::SetIconTheme(id),
                )));
            }
            RowKey::RunSnippet(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::Action(PaletteAction::Submenu(
                    SubmenuAction::RunSnippet(id),
                )));
            }
            RowKey::SwitchBranch(name) => {
                self.close(cx);
                cx.emit(PaletteEvent::Action(PaletteAction::Submenu(
                    SubmenuAction::SwitchBranch(name),
                )));
            }
            RowKey::GoToLine(line) => {
                self.close(cx);
                cx.emit(PaletteEvent::Action(PaletteAction::Submenu(
                    SubmenuAction::GoToLine(line),
                )));
            }
            RowKey::ShowStatusBarItem(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::Action(PaletteAction::Submenu(
                    SubmenuAction::ShowStatusBarItem(id),
                )));
            }
        }
    }

    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        let ks = &ev.keystroke;
        let len = self.rows(cx).len();
        match ks.key.as_str() {
            "escape" => {
                if !self.query.is_empty() {
                    self.query.clear();
                    self.selected = 0;
                    cx.notify();
                } else if self.pages.len() > 1 {
                    self.go_back(cx);
                } else {
                    self.close(cx);
                }
            }
            "enter" => {
                if ks.modifiers.shift {
                    self.run_secondary(cx);
                } else {
                    self.run_selected(window, cx);
                }
            }
            "down" => {
                if len > 0 {
                    self.selected = (self.selected + 1) % len;
                    cx.notify();
                }
            }
            "up" => {
                if len > 0 {
                    self.selected = (self.selected + len - 1) % len;
                    cx.notify();
                }
            }
            "backspace" => {
                if self.query.is_empty() && self.pages.len() > 1 {
                    self.go_back(cx);
                } else {
                    self.query.pop();
                    self.selected = 0;
                    cx.notify();
                }
            }
            key => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                let ch = ks
                    .key_char
                    .clone()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                    .or_else(|| (key.chars().count() == 1).then(|| key.to_string()));
                if let Some(ch) = ch {
                    self.query.push_str(&ch);
                    self.selected = 0;
                    cx.notify();
                }
            }
        }
        if self.open {
            self.sync_theme_preview(cx);
        }
        cx.stop_propagation();
    }

    fn cycle_search_mode(&mut self, cx: &mut Context<Self>) {
        let next = self.search_mode(cx).next();
        set_palette_search_mode_setting(next, cx);
        cx.notify();
    }
}

/// Human label for an editor-theme slug (e.g. `github-dark` → "Github Dark").
#[cfg(test)]
fn editor_theme_label(id: EditorThemeId) -> String {
    id.slug()
        .split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl<W, Th> Focusable for CommandPalette<W, Th>
where
    W: PaletteWorkspace + 'static,
    Th: UiTheme + 'static,
{
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl<W, Th> Render for CommandPalette<W, Th>
where
    W: PaletteWorkspace + 'static,
    Th: UiTheme + 'static,
{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }

        let opacity = (palette_opacity(cx) as f32 / 100.0).clamp(0.35, 1.0);
        let position = palette_position(cx);
        let show_recent = palette_show_recent(cx);
        let close_on_overlay = palette_close_on_overlay_click(cx);
        let mode = self.search_mode(cx);

        let t = self.theme.read(cx);
        let (fg, muted, border, success, primary) = (
            t.foreground(),
            t.muted_foreground(),
            t.border(),
            t.status_success(),
            t.primary(),
        );
        // T20-001: the shared token snapshot, so the `Kbd` chips below are
        // built from the same `Palette` every other primitive uses.
        let c = Palette::from_theme(t);
        let mut card = t.card();
        card.a *= opacity;
        let sel_fill = t.selected_fill();
        let hover_fill = t.accent();
        let chip_bg = t.muted();

        let page = self.page();
        let (input_text, input_color) = if self.query.is_empty() {
            (page.placeholder().to_string(), muted)
        } else {
            (self.query.clone(), fg)
        };

        // Rows, with an optional "Recently Used" group prepended on Root.
        let mut rows = Vec::new();
        if page == Page::Root && self.query.is_empty() && show_recent && !self.recent.is_empty() {
            let ctx = self.active_context(cx);
            let display = palette_keybind_display(cx);
            let avail: std::collections::HashSet<CommandId> = available(&self.data.commands, ctx)
                .into_iter()
                .map(|c| c.id)
                .collect();
            for id in self.recent.iter().copied().filter(|id| avail.contains(id)) {
                let Some(c) = command(&self.data.commands, id) else {
                    continue;
                };
                rows.push(PaletteRow {
                    key: c
                        .sub_page
                        .map(RowKey::Navigate)
                        .unwrap_or(RowKey::Command(id)),
                    secondary: None,
                    icon: Some(c.icon),
                    title: c.title.to_string(),
                    subtitle: None,
                    section: "Recently Used".to_string(),
                    keys: command_key_tokens(c, &display),
                    right_label: None,
                    has_sub: c.sub_page.is_some(),
                });
            }
        }
        rows.extend(self.rows(cx));
        let result_count = rows
            .iter()
            .filter(|r| !matches!(r.key, RowKey::Noop))
            .count();
        let selected = self.selected.min(rows.len().saturating_sub(1));

        // ── list ─────────────────────────────────────────────────────────────
        let mut list = div()
            .id("palette-list")
            .flex()
            .flex_col()
            .p(px(8.0))
            .max_h(px(384.0))
            .overflow_y_scroll();
        if result_count == 0 {
            list = list.child(
                div()
                    .py(px(40.0))
                    .flex()
                    .justify_center()
                    .text_size(px(13.0))
                    .text_color(muted)
                    .child("No results found."),
            );
        }
        let mut last_section: Option<String> = None;
        for (i, row) in rows.iter().enumerate() {
            if last_section.as_deref() != Some(row.section.as_str()) {
                list = list.child(
                    div()
                        .px(px(12.0))
                        .py(px(8.0))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(muted.opacity(0.7))
                        .child(SharedString::from(row.section.to_uppercase())),
                );
                last_section = Some(row.section.clone());
            }
            let is_sel = i == selected;
            let actionable = !matches!(row.key, RowKey::Noop);
            // T20-003: not a `ListItem` — non-actionable (`Noop`) rows must
            // render with no hover/cursor-pointer at full opacity, but
            // `ListItem` always adds hover+pointer unless `.disabled()`
            // (which also drops opacity to 0.5). The border-left selection
            // indicator, icon-chip background square, and selection-
            // dependent horizontal padding are also outside `ListItem`'s
            // shape. Documented exception.
            let mut r = div()
                .id(("palette-row", i))
                .flex()
                .items_center()
                .gap(px(12.0))
                .mx(px(2.0))
                .my(px(2.0))
                .px(px(if is_sel { 10.0 } else { 12.0 }))
                .py(px(8.0))
                .min_h(px(40.0))
                .rounded(px(8.0))
                .border_l_2()
                .border_color(if is_sel {
                    primary
                } else {
                    gpui::transparent_black()
                })
                .text_color(fg)
                .when(is_sel, |d| d.bg(sel_fill))
                .when(actionable && !is_sel, |d| d.hover(|s| s.bg(hover_fill)));

            if let Some(icon) = row.icon {
                r = r.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(28.0))
                        .rounded(px(6.0))
                        .bg(chip_bg)
                        .flex_none()
                        .child(icon.svg(muted)),
                );
            }

            let mut text_col = div().flex().flex_col().flex_1().min_w_0().child(
                div()
                    .text_size(px(13.0))
                    .truncate()
                    .child(SharedString::from(row.title.clone())),
            );
            if let Some(sub) = &row.subtitle {
                text_col = text_col.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(muted)
                        .truncate()
                        .child(SharedString::from(sub.clone())),
                );
            }
            r = r.child(text_col);

            let mut right = div().flex().items_center().gap(px(6.0)).flex_none();
            if let Some(label) = &row.right_label {
                let on = label == "ON" || label == "active";
                right = right.child(
                    div()
                        .text_size(px(10.0))
                        .text_color(if on { success } else { muted })
                        .child(SharedString::from(label.to_uppercase())),
                );
            }
            for k in &row.keys {
                right = right.child(kbd(k.clone(), c));
            }
            if row.has_sub {
                right = right.child(IconName::ChevronRight.svg(muted).size(px(12.0)));
            }
            r = r.child(right);

            if actionable {
                r = r.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.selected = i;
                    this.run_selected(window, cx);
                }));
            }
            list = list.child(r);
        }

        // ── header (breadcrumb or search icon + input) ───────────────────────
        let mut header = div()
            .h(px(56.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(16.0))
            .border_b_1()
            .border_color(border);
        if self.pages.len() > 1 {
            let mut crumbs = div().flex().items_center().gap(px(4.0)).flex_none();
            let last = self.pages.len() - 1;
            for (idx, pg) in self.pages.clone().into_iter().enumerate() {
                if idx > 0 {
                    crumbs = crumbs.child(
                        div()
                            .text_size(px(10.0))
                            .text_color(muted)
                            .child("\u{203a}"),
                    );
                }
                let is_current = idx == last;
                // T20-003: a breadcrumb pill that is a plain bg-fill on the
                // current page and only hoverable/clickable on prior pages —
                // no `ui-kit` primitive is a plain, non-square, conditionally
                // interactive pill; documented exception (same shape as
                // `hosts.rs`'s `render_group_chips`).
                crumbs = crumbs.child(
                    div()
                        .id(("crumb", idx))
                        .px(px(8.0))
                        .py(px(2.0))
                        .rounded(px(6.0))
                        .text_size(px(11.0))
                        .text_color(if is_current { fg } else { muted })
                        .when(is_current, |d| d.bg(sel_fill))
                        .when(!is_current, |d| {
                            d.hover(|s| s.bg(hover_fill)).on_click(cx.listener(
                                move |this, _: &ClickEvent, _w, cx| this.go_back_to(idx, cx),
                            ))
                        })
                        .child(SharedString::from(pg.label())),
                );
            }
            header = header.child(crumbs);
        } else {
            header = header.child(IconName::Search.svg(muted));
        }
        header = header.child(
            div()
                .flex_1()
                .text_size(px(15.0))
                .text_color(input_color)
                .child(SharedString::from(input_text)),
        );

        // ── footer ───────────────────────────────────────────────────────────
        let mut hints = div().flex().items_center().gap(px(12.0)).ml_auto();
        hints = hints
            .child(keybinding_hint("navigate", ["\u{2191}\u{2193}"], c))
            .child(keybinding_hint("select", ["\u{21b5}"], c));
        if let Some(sec) = rows.get(selected).and_then(|r| r.secondary.as_ref()) {
            hints = hints.child(keybinding_hint(
                SharedString::from(sec.label.clone()),
                ["\u{21e7}\u{21b5}"],
                c,
            ));
        }
        if self.pages.len() > 1 {
            hints = hints.child(keybinding_hint("back", ["\u{232b}"], c));
        }
        hints = hints.child(keybinding_hint("close", ["Esc"], c));
        let footer = div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(16.0))
            .py(px(8.0))
            .border_t_1()
            .border_color(border)
            .child(
                // T20-003: a subtle bordered footer mode-indicator chip —
                // `button()`'s pill (`rounded.xl4`) and fixed size scale
                // would visibly change this small rectangular badge's shape;
                // documented exception.
                div()
                    .id("palette-search-mode")
                    .px(px(6.0))
                    .py(px(2.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(border)
                    .text_size(px(10.0))
                    .text_color(muted)
                    .hover(|s| s.text_color(fg))
                    .child(SharedString::from(mode.label()))
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| this.cycle_search_mode(cx)),
                    ),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(muted)
                    .child(SharedString::from(format!(
                        "{result_count} {}",
                        if result_count == 1 {
                            "result"
                        } else {
                            "results"
                        }
                    ))),
            )
            .child(hints);

        let top = match position.as_str() {
            "high" => px(48.0),
            "center" => px(160.0),
            _ => px(96.0),
        };

        div()
            .id("palette-overlay")
            .absolute()
            .inset_0()
            .flex()
            .justify_center()
            .items_start()
            .pt(top)
            .bg(modal_scrim())
            .track_focus(&self.focus)
            .key_context("CommandPalette")
            .on_key_down(cx.listener(Self::on_key))
            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                if close_on_overlay {
                    this.close(cx);
                }
            }))
            .child(
                div()
                    .id("palette-card")
                    .occlude()
                    .w(px(640.0))
                    .max_h(px(560.0))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(px(16.0))
                    .bg(card)
                    .border_1()
                    .border_color(border.opacity(0.6))
                    .shadow_lg()
                    .child(header)
                    .child(div().flex().flex_col().min_h_0().child(list))
                    .child(footer),
            )
            .into_any_element()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn palette_commands() -> Vec<Command> {
        vec![
            Command::from_descriptor(CommandDescriptor::new(
                CommandId::NewTerminalTab,
                "New Terminal Tab",
                "Layout",
            )),
            Command::from_descriptor(
                CommandDescriptor::new(CommandId::SplitRight, "Split Pane Right", "Layout")
                    .with_contexts(&[CommandContext::Terminal]),
            ),
            Command::from_descriptor(
                CommandDescriptor::new(CommandId::GoToSymbol, "Go to Symbol", "Editor")
                    .with_contexts(&[CommandContext::Editor]),
            ),
        ]
    }

    #[test]
    fn registry_snapshot_drives_filtering_and_search() {
        let commands = palette_commands();
        let home = available(&commands, None);
        assert!(home.iter().any(|c| c.id == CommandId::NewTerminalTab));
        assert!(!home.iter().any(|c| c.id == CommandId::SplitRight));

        let term = available(&commands, Some(CommandContext::Terminal));
        assert!(term.iter().any(|c| c.id == CommandId::SplitRight));
        let editor = available(&commands, Some(CommandContext::Editor));
        assert!(editor.iter().any(|c| c.id == CommandId::GoToSymbol));

        let hits = search_mode(
            &commands,
            "split pane",
            Some(CommandContext::Terminal),
            SearchMode::Contains,
        );
        assert!(hits.iter().any(|c| c.id == CommandId::SplitRight));
        assert!(search_mode(&commands, "zzzznope", None, SearchMode::Contains).is_empty());
    }

    #[test]
    fn descriptor_metadata_is_adapted_for_the_view() {
        let commands = palette_commands();
        let split = command(&commands, CommandId::SplitRight).expect("test command");
        assert_eq!(split.title, "Split Pane Right");
        assert_eq!(split.contexts, vec![CommandContext::Terminal]);
    }

    #[test]
    fn context_of_maps_tab_kinds() {
        assert_eq!(
            context_of(PaletteTabKind::Workspace, false),
            Some(CommandContext::Terminal)
        );
        assert_eq!(
            context_of(PaletteTabKind::Workspace, true),
            Some(CommandContext::SshTerminal)
        );
        assert_eq!(
            context_of(PaletteTabKind::Editor, false),
            Some(CommandContext::Editor)
        );
        assert_eq!(context_of(PaletteTabKind::Other, false), None);
    }

    #[test]
    fn search_mode_ranks_results() {
        let commands = palette_commands();
        let hits = search_mode(
            &commands,
            "split",
            Some(CommandContext::Terminal),
            SearchMode::Fuzzy,
        );
        assert!(hits.iter().any(|c| c.id == CommandId::SplitRight));
        // Fuzzy still filters nonsense out.
        assert!(search_mode(&commands, "zzzznope", None, SearchMode::Fuzzy).is_empty());
    }

    #[test]
    fn every_command_has_an_icon_and_nav_targets_resolve() {
        for c in palette_commands() {
            // `icon` is non-optional by type; assert `sub_page` rows are
            // navigators, not runnable no-ops.
            let _ = c.icon;
            if let Some(pg) = c.sub_page {
                assert_ne!(pg, Page::Root, "{:?} navigates to Root", c.id);
                assert!(!pg.placeholder().is_empty());
                assert!(!pg.label().is_empty());
            }
        }
    }

    #[test]
    fn toggle_commands_map_to_pref_keys() {
        assert_eq!(toggle_pref_key(CommandId::ToggleVimMode), Some("vimMode"));
        assert_eq!(
            toggle_pref_key(CommandId::ToggleEditorWordWrap),
            Some("editorWordWrap")
        );
        assert_eq!(toggle_pref_key(CommandId::NewTerminalTab), None);
    }

    #[test]
    fn editor_theme_label_is_titlecased() {
        assert_eq!(editor_theme_label(EditorThemeId::GithubDark), "Github Dark");
        assert_eq!(editor_theme_label(EditorThemeId::Nord), "Nord");
    }

    #[test]
    fn all_pages_have_distinct_labels() {
        let pages = [
            Page::Root,
            Page::Tabs,
            Page::ColorMode,
            Page::EditorTheme,
            Page::Themes,
            Page::IconThemes,
            Page::Hosts,
            Page::Snippets,
            Page::Outline,
            Page::GitBranches,
            Page::StatusBarHidden,
        ];
        let labels: std::collections::HashSet<_> = pages.iter().map(|p| p.label()).collect();
        assert_eq!(labels.len(), pages.len());
    }
}
