//! `AREAS` — the single place a new top-level settings category is
//! registered (`docs/settings-guidelines.md` rule 4). No UI here, only data;
//! `T19-004` (the disclosure-nav renderer) and `T19-007` (search) consume
//! this list.

/// Whether a category's page is mechanically generated from a
/// `SettingsContent` field's type, or is a hand-written custom pane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AreaKind {
    /// Rendered field-by-field from `target_module`'s struct.
    Generated,
    /// A bespoke render function for a genuine Settings value surface.
    Custom,
}

/// One top-level settings category.
#[derive(Clone, Copy, Debug)]
pub struct AreaMeta {
    /// Stable identifier for this category (used by deep links, `T19-007`
    /// search, and this module's own consistency test).
    pub key: &'static str,
    pub title: &'static str,
    /// Deep-link slug (`settings://<slug>`, rule 7).
    pub slug: &'static str,
    pub kind: AreaKind,
    /// Name of the `SettingsContent` field/submodule this category's data
    /// lives under (see the `settings_content::tests::every_area_hits_a_real_module`
    /// test — every `target_module` here must be one of `SettingsContent`'s
    /// actual field names).
    pub target_module: &'static str,
}

pub const AREAS: &[AreaMeta] = &[
    AreaMeta {
        key: "general",
        title: "General",
        slug: "general",
        kind: AreaKind::Generated,
        target_module: "general",
    },
    AreaMeta {
        key: "appearance",
        title: "Appearance",
        slug: "appearance",
        kind: AreaKind::Generated,
        target_module: "appearance",
    },
    AreaMeta {
        key: "terminal",
        title: "Terminal",
        slug: "terminal",
        kind: AreaKind::Generated,
        target_module: "terminal",
    },
    AreaMeta {
        key: "editor",
        title: "Editor",
        slug: "editor",
        kind: AreaKind::Generated,
        target_module: "editor",
    },
    AreaMeta {
        key: "file_manager",
        title: "File Manager",
        slug: "file-manager",
        kind: AreaKind::Generated,
        target_module: "file_manager",
    },
    AreaMeta {
        key: "workspace",
        title: "Workspace",
        slug: "workspace",
        kind: AreaKind::Generated,
        target_module: "workspace",
    },
];
