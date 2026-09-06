//! UI-free command identity, descriptors, and registry.
//!
//! The registry is the product-level extension point for commands. Feature
//! modules publish descriptors here and keep their execution behaviour in the
//! composition root. This crate deliberately has no GPUI dependency: the
//! palette view adapts descriptors to icons and visual sub-pages separately.

use std::fmt;

use labonair_keymap::ShortcutId;

/// The surface that is active in the focused tab.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum CommandContext {
    Terminal,
    Editor,
    Sftp,
    Home,
    SshTerminal,
}

/// Stable identity of a built-in command.
///
/// The enum keeps existing persisted action names and dispatch sites stable.
/// New feature-owned commands should first use a dedicated variant here; a
/// later registry revision can introduce an owned external identity without
/// coupling it to the palette view.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum CommandId {
    NewTerminalTab,
    NewEditorTab,
    DuplicateTab,
    CloseOtherTabs,
    SplitRight,
    SplitDown,
    ClosePane,
    CloseTab,
    NextTab,
    PrevTab,
    SwitchTab,
    Find,
    ToggleSidebar,
    ToggleFullScreen,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    OpenSnippetsPanel,
    OpenGitGraph,
    FocusSourceControl,
    OpenHostSettings,
    ClearTerminal,
    OpenShortcuts,
    OpenSettings,
    OpenProject,
    OpenProjectSettings,
    OpenSettingsJson,
    CheckForUpdates,
    FormatDocument,
    ToggleZenModeHeader,
    ToggleZenModeStatusbar,
    ToggleZenMode,
    AdjustFontSize,
    ConnectSsh,
    OpenSftp,
    ChangeAppTheme,
    ChangeIconTheme,
    ChangeColorMode,
    ChangeEditorTheme,
    RunSnippet,
    GitSwitchBranch,
    GoToSymbol,
    ShowStatusBarItem,
    ToggleEditorWordWrap,
    ToggleLineNumbers,
    ToggleFormatOnSave,
    ToggleCursorBlink,
    TogglePaneHeader,
    TogglePaneFooter,
    ToggleVimMode,
    OpenCommandPalette,
    NewPreviewTab,
    Save,
    NewSshTab,
    NewSftpTab,
    NewSshConnection,
    NewQuickSsh,
    FocusNextPane,
    SelectTab1,
    SelectTab2,
    SelectTab3,
    SelectTab4,
    SelectTab5,
    SelectTab6,
    SelectTab7,
    SelectTab8,
    SelectTab9,
    DebugCyclePanelDock,
    DebugToggleDockZoom,
    OpenKeymapJson,
    OpenComponentGallery,
}

/// A visual hint that the GPUI adapter maps to its icon set.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum CommandIcon {
    Terminal,
    File,
    Copy,
    Close,
    ChevronRight,
    ChevronDown,
    Trash,
    Server,
    Folder,
    Search,
    PanelLeft,
    Square,
    Sparkles,
    Plus,
    Minus,
    Refresh,
    Command,
    GitBranch,
    Edit,
    FileCode,
    Eye,
    Check,
    PanelTop,
    PanelBottom,
    Download,
    Palette,
}

/// A palette follow-up page requested by a command descriptor.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum CommandSubmenu {
    Tabs,
    Zoom,
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

/// Stable identity and presentation metadata for a dynamic submenu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmenuDescriptor {
    pub id: String,
    pub title: String,
}

impl SubmenuDescriptor {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
        }
    }
}

/// One immutable row returned by a dynamic submenu provider. The action type
/// belongs to the provider, so selecting a host/theme/tab cannot be reduced
/// to an untyped string inside the palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmenuItem<Action> {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub action: Action,
}

/// Module-owned source for a dynamic submenu. The provider controls loading,
/// snapshots, and typed selection payloads; the palette only renders and
/// navigates the returned rows.
pub trait SubmenuProvider {
    type Action: Clone;

    fn descriptor(&self) -> SubmenuDescriptor;
    fn snapshot(&self) -> Vec<SubmenuItem<Self::Action>>;
}

/// Metadata published by an owning module for one command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandDescriptor {
    pub id: CommandId,
    pub title: String,
    pub section: String,
    pub aliases: Vec<String>,
    pub contexts: Vec<CommandContext>,
    pub shortcut: Option<ShortcutId>,
    pub icon: CommandIcon,
    pub submenu: Option<CommandSubmenu>,
}

impl CommandDescriptor {
    pub fn new(id: CommandId, title: impl Into<String>, section: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            section: section.into(),
            aliases: Vec::new(),
            contexts: Vec::new(),
            shortcut: None,
            icon: CommandIcon::Command,
            submenu: None,
        }
    }

    pub fn with_contexts(mut self, contexts: &[CommandContext]) -> Self {
        self.contexts = contexts.to_vec();
        self
    }

    pub fn with_aliases(mut self, aliases: &[&str]) -> Self {
        self.aliases = aliases.iter().map(|alias| (*alias).to_string()).collect();
        self
    }

    pub fn with_shortcut(mut self, shortcut: ShortcutId) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    pub fn with_icon(mut self, icon: CommandIcon) -> Self {
        self.icon = icon;
        self
    }

    pub fn with_submenu(mut self, submenu: CommandSubmenu) -> Self {
        self.submenu = Some(submenu);
        self
    }

    pub fn is_available_in(&self, context: Option<CommandContext>) -> bool {
        match context {
            None => self.contexts.is_empty(),
            Some(active) => self.contexts.is_empty() || self.contexts.contains(&active),
        }
    }
}

/// A module can contribute one or more command descriptors without knowing
/// anything about the palette view or shell dispatch implementation.
pub trait CommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor>;
}

/// Duplicate registration is a programmer/configuration error and is reported
/// explicitly so composition roots cannot silently shadow a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandRegistryError {
    DuplicateId(CommandId),
}

impl fmt::Display for CommandRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "command {id:?} was registered more than once"),
        }
    }
}

impl std::error::Error for CommandRegistryError {}

/// The one source of command metadata consumed by palette, keymap, and menus.
#[derive(Clone, Debug, Default)]
pub struct CommandRegistry {
    commands: Vec<CommandDescriptor>,
}

impl CommandRegistry {
    pub fn register(&mut self, descriptor: CommandDescriptor) -> Result<(), CommandRegistryError> {
        if self
            .commands
            .iter()
            .any(|command| command.id == descriptor.id)
        {
            return Err(CommandRegistryError::DuplicateId(descriptor.id));
        }
        self.commands.push(descriptor);
        Ok(())
    }

    pub fn register_provider<P: CommandProvider>(
        &mut self,
        provider: &P,
    ) -> Result<(), CommandRegistryError> {
        let additions = provider.commands();
        if let Some(duplicate) = additions.iter().find_map(|item| {
            (self.commands.iter().any(|prior| prior.id == item.id)
                || additions
                    .iter()
                    .filter(|candidate| candidate.id == item.id)
                    .count()
                    > 1)
            .then_some(item.id)
        }) {
            return Err(CommandRegistryError::DuplicateId(duplicate));
        }
        self.commands.extend(additions);
        Ok(())
    }

    pub fn snapshot(&self) -> Vec<CommandDescriptor> {
        self.commands.clone()
    }

    pub fn iter(&self) -> impl Iterator<Item = &CommandDescriptor> {
        self.commands.iter()
    }

    pub fn command(&self, id: CommandId) -> Option<&CommandDescriptor> {
        self.commands.iter().find(|command| command.id == id)
    }

    pub fn available(&self, context: Option<CommandContext>) -> Vec<&CommandDescriptor> {
        self.commands
            .iter()
            .filter(|command| command.is_available_in(context))
            .collect()
    }

    pub fn command_for_shortcut(&self, shortcut: ShortcutId) -> Option<CommandId> {
        self.commands
            .iter()
            .find(|command| command.shortcut == Some(shortcut))
            .map(|command| command.id)
    }
}

#[rustfmt::skip]
const ACTION_NAMES: &[(CommandId, &str)] = &[
    (CommandId::NewTerminalTab, "tab::NewTerminal"),
    (CommandId::NewEditorTab, "tab::NewEditor"),
    (CommandId::NewPreviewTab, "tab::NewPreview"),
    (CommandId::NewSshTab, "tab::NewSsh"),
    (CommandId::NewSftpTab, "tab::NewSftp"),
    (CommandId::DuplicateTab, "tab::Duplicate"),
    (CommandId::CloseOtherTabs, "tab::CloseOthers"),
    (CommandId::CloseTab, "tab::Close"),
    (CommandId::NextTab, "tab::Next"),
    (CommandId::PrevTab, "tab::Prev"),
    (CommandId::SwitchTab, "tab::Switch"),
    (CommandId::SelectTab1, "tab::Select1"),
    (CommandId::SelectTab2, "tab::Select2"),
    (CommandId::SelectTab3, "tab::Select3"),
    (CommandId::SelectTab4, "tab::Select4"),
    (CommandId::SelectTab5, "tab::Select5"),
    (CommandId::SelectTab6, "tab::Select6"),
    (CommandId::SelectTab7, "tab::Select7"),
    (CommandId::SelectTab8, "tab::Select8"),
    (CommandId::SelectTab9, "tab::Select9"),
    (CommandId::Save, "tab::Save"),
    (CommandId::SplitRight, "pane::SplitRight"),
    (CommandId::SplitDown, "pane::SplitDown"),
    (CommandId::ClosePane, "pane::Close"),
    (CommandId::FocusNextPane, "pane::FocusNext"),
    (CommandId::ClearTerminal, "terminal::Clear"),
    (CommandId::Find, "search::Toggle"),
    (CommandId::ToggleSidebar, "sidebar::Toggle"),
    (CommandId::ToggleFullScreen, "view::ToggleFullScreen"),
    (CommandId::ZoomIn, "view::ZoomIn"),
    (CommandId::ZoomOut, "view::ZoomOut"),
    (CommandId::ZoomReset, "view::ZoomReset"),
    (CommandId::AdjustFontSize, "view::AdjustFontSize"),
    (CommandId::ChangeAppTheme, "view::ChangeAppTheme"),
    (CommandId::ChangeIconTheme, "view::ChangeIconTheme"),
    (CommandId::ChangeColorMode, "view::ChangeColorMode"),
    (CommandId::ChangeEditorTheme, "view::ChangeEditorTheme"),
    (CommandId::ToggleZenMode, "view::ToggleZenMode"),
    (CommandId::ToggleZenModeHeader, "view::ToggleZenModeHeader"),
    (CommandId::ToggleZenModeStatusbar, "view::ToggleZenModeStatusbar"),
    (CommandId::ShowStatusBarItem, "view::ShowStatusBarItem"),
    (CommandId::ToggleEditorWordWrap, "editor::ToggleWordWrap"),
    (CommandId::ToggleLineNumbers, "editor::ToggleLineNumbers"),
    (CommandId::ToggleFormatOnSave, "editor::ToggleFormatOnSave"),
    (CommandId::FormatDocument, "editor::FormatDocument"),
    (CommandId::GoToSymbol, "editor::GoToSymbol"),
    (CommandId::ToggleVimMode, "editor::ToggleVimMode"),
    (CommandId::ToggleCursorBlink, "terminal::ToggleCursorBlink"),
    (CommandId::TogglePaneHeader, "terminal::TogglePaneHeader"),
    (CommandId::TogglePaneFooter, "terminal::TogglePaneFooter"),
    (CommandId::RunSnippet, "snippets::Run"),
    (CommandId::OpenSnippetsPanel, "snippets::OpenPanel"),
    (CommandId::OpenGitGraph, "git::OpenGraph"),
    (CommandId::FocusSourceControl, "git::FocusSourceControl"),
    (CommandId::GitSwitchBranch, "git::SwitchBranch"),
    (CommandId::OpenHostSettings, "connections::OpenHostSettings"),
    (CommandId::NewSshConnection, "connections::NewSshConnection"),
    (CommandId::NewQuickSsh, "connections::NewQuickSsh"),
    (CommandId::ConnectSsh, "connections::Connect"),
    (CommandId::OpenSftp, "connections::OpenSftp"),
    (CommandId::OpenCommandPalette, "command_palette::Toggle"),
    (CommandId::OpenShortcuts, "settings::OpenShortcuts"),
    (CommandId::OpenSettings, "settings::Open"),
    (CommandId::OpenProject, "workspace::OpenProject"),
    (CommandId::OpenProjectSettings, "settings::OpenProjectJson"),
    (CommandId::OpenSettingsJson, "settings::OpenUserJson"),
    (CommandId::OpenKeymapJson, "zed::OpenKeymap"),
    (CommandId::CheckForUpdates, "app::CheckForUpdates"),
    (CommandId::DebugCyclePanelDock, "debug::CyclePanelDock"),
    (CommandId::DebugToggleDockZoom, "debug::ToggleDockZoom"),
    (CommandId::OpenComponentGallery, "debug::OpenComponentGallery"),
];

impl CommandId {
    pub fn action_name(self) -> &'static str {
        ACTION_NAMES
            .iter()
            .find(|(id, _)| *id == self)
            .map(|(_, name)| *name)
            .unwrap_or_else(|| panic!("CommandId::{self:?} has no action name"))
    }

    pub fn from_action_name(name: &str) -> Option<Self> {
        ACTION_NAMES
            .iter()
            .find(|(_, action)| *action == name)
            .map(|(id, _)| *id)
    }
}

pub fn known_action_names() -> std::collections::BTreeSet<&'static str> {
    ACTION_NAMES.iter().map(|(_, name)| *name).collect()
}

pub fn toggle_pref_key(id: CommandId) -> Option<&'static str> {
    Some(match id {
        CommandId::ToggleZenModeHeader => "zenModeShowHeader",
        CommandId::ToggleZenModeStatusbar => "zenModeShowStatusbar",
        CommandId::ToggleEditorWordWrap => "editorWordWrap",
        CommandId::ToggleLineNumbers => "editorLineNumbers",
        CommandId::ToggleFormatOnSave => "editorFormatOnSave",
        CommandId::ToggleCursorBlink => "terminalCursorBlink",
        CommandId::TogglePaneHeader => "terminalShowPaneHeader",
        CommandId::TogglePaneFooter => "terminalShowPaneFooter",
        CommandId::ToggleVimMode => "vimMode",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_rejects_duplicate_ids() {
        let mut registry = CommandRegistry::default();
        let descriptor = CommandDescriptor::new(CommandId::Find, "Find", "Search");
        registry
            .register(descriptor.clone())
            .expect("first registration");
        assert_eq!(
            registry.register(descriptor),
            Err(CommandRegistryError::DuplicateId(CommandId::Find))
        );
    }

    #[test]
    fn registry_filters_contexts_and_preserves_order() {
        let mut registry = CommandRegistry::default();
        registry
            .register(CommandDescriptor::new(CommandId::Find, "Find", "Search"))
            .expect("register find");
        registry
            .register(
                CommandDescriptor::new(CommandId::FormatDocument, "Format", "Editor")
                    .with_contexts(&[CommandContext::Editor]),
            )
            .expect("register format");

        let commands = registry.available(Some(CommandContext::Editor));
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].id, CommandId::Find);
        assert_eq!(registry.available(None).len(), 1);
    }

    struct TestCommandProvider;

    impl CommandProvider for TestCommandProvider {
        fn commands(&self) -> Vec<CommandDescriptor> {
            vec![CommandDescriptor::new(
                CommandId::OpenGitGraph,
                "Open Git Graph",
                "Source Control",
            )]
        }
    }

    #[test]
    fn feature_provider_contributes_without_palette_knowledge() {
        let mut registry = CommandRegistry::default();
        registry
            .register_provider(&TestCommandProvider)
            .expect("provider registration");
        assert_eq!(registry.snapshot()[0].id, CommandId::OpenGitGraph);
    }

    #[test]
    fn action_names_round_trip() {
        for (id, action) in ACTION_NAMES {
            assert_eq!(CommandId::from_action_name(action), Some(*id));
            assert!(!action.is_empty());
        }
    }

    #[derive(Clone)]
    struct TestSubmenu;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum TestAction {
        Select,
    }

    impl SubmenuProvider for TestSubmenu {
        type Action = TestAction;

        fn descriptor(&self) -> SubmenuDescriptor {
            SubmenuDescriptor::new("test.items", "Test Items")
        }

        fn snapshot(&self) -> Vec<SubmenuItem<Self::Action>> {
            vec![SubmenuItem {
                id: "one".to_string(),
                title: "One".to_string(),
                subtitle: None,
                action: TestAction::Select,
            }]
        }
    }

    #[test]
    fn submenu_provider_returns_typed_immutable_rows() {
        let provider = TestSubmenu;
        assert_eq!(provider.descriptor().id, "test.items");
        assert_eq!(provider.snapshot()[0].action, TestAction::Select);
    }
}
