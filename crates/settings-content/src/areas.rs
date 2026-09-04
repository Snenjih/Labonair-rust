//! `AREAS` — the single place a new top-level settings category is
//! registered (`docs/settings-guidelines.md` rule 4). No UI here, only data;
//! `T19-004` (the disclosure-nav renderer) and `T19-007` (search) consume
//! this list.

/// Whether a category's page is mechanically generated from a
/// `SettingsContent` field's type (`T19-004`'s renderer registry), or is a
/// hand-written [`docs::settings_guidelines`] rule-4 custom pane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AreaKind {
    /// Rendered field-by-field from `target_module`'s struct.
    Generated,
    /// A bespoke `render_fn` (theme gallery, host manager, shortcut
    /// recorder, …). May still read/write fields under `target_module`.
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
        key: "connections",
        title: "Connections",
        slug: "connections",
        kind: AreaKind::Generated,
        target_module: "connections",
    },
    AreaMeta {
        key: "workspace",
        title: "Workspace",
        slug: "workspace",
        kind: AreaKind::Generated,
        target_module: "workspace",
    },
    // ── Custom top-level categories (guidelines rule 4) ────────────────
    AreaMeta {
        key: "themes",
        title: "Themes",
        slug: "themes",
        kind: AreaKind::Custom,
        // The active theme id + variant overrides live on `appearance`; the
        // theme gallery itself reads the on-disk theme index, not a
        // dedicated `SettingsContent` submodule.
        target_module: "appearance",
    },
    AreaMeta {
        key: "hosts",
        title: "Hosts",
        slug: "hosts",
        kind: AreaKind::Custom,
        target_module: "hosts",
    },
    AreaMeta {
        key: "shortcuts",
        title: "Shortcuts",
        slug: "shortcuts",
        kind: AreaKind::Custom,
        // Bindings live in keymap.json (T19-008); SettingsContent only
        // carries which base preset a reset seeds from.
        target_module: "keymap",
    },
    AreaMeta {
        key: "ai",
        title: "AI",
        slug: "ai",
        kind: AreaKind::Custom,
        target_module: "ai",
    },
    AreaMeta {
        key: "mcp",
        title: "MCP",
        slug: "mcp",
        kind: AreaKind::Custom,
        target_module: "mcp",
    },
    AreaMeta {
        key: "personalization",
        title: "Personalization",
        slug: "personalization",
        kind: AreaKind::Custom,
        target_module: "personalization",
    },
];
