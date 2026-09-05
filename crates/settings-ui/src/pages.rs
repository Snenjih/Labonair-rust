//! Declarative settings pages (T19-004, replacing the old `SECTION_GROUPS`
//! table in `crates/settings-ui/src/sections.rs`): one [`SettingsPage`] per
//! [`labonair_settings_content::areas::AREAS`] entry, built from curated
//! section groupings (the old `SECTION_GROUPS` shape, ported almost
//! verbatim — its curation is still correct, only its lookup now targets
//! [`crate::schema::AnyField`] instead of the deleted `FieldDef`) plus
//! [`SubPage`] entries for the categories large enough to need one (Terminal,
//! Editor, AI — mandatory per the task's Notizen).
//!
//! This is the **one hand-maintained placement list** `docs/settings-
//! guidelines.md` rule 3 allows: it only ever references [`AnyField`]s by
//! their local key, never re-declares a control kind or range — a field not
//! listed here still renders, appended to a trailing "Other" section on its
//! area's page, so nothing in `schema.rs` can ever go unreachable by being
//! forgotten here (`tests::every_generated_field_is_placed_or_falls_through`).

use labonair_settings_content::areas::{AreaKind, AreaMeta, AREAS};

use crate::schema::AnyField;

/// One row in a generated page's body.
pub enum SettingsPageItem {
    /// A collapsible disclosure heading (`docs/settings-guidelines.md` rule 1).
    SectionHeader(&'static str),
    /// An [`AnyField`], referenced by its local (leaf) key.
    Item(&'static str),
}

/// What a page or sub-page renders below the standard chrome.
pub enum PageBody {
    /// Rendered mechanically from `items` + the trailing "Other" fallback.
    Generated(Vec<SettingsPageItem>),
    /// A hand-written `render_fn`, dispatched by `SettingsView` on
    /// `area.key` (`docs/settings-guidelines.md` rule 4) — still inside the
    /// standard header/search/badge chrome, only the body is custom.
    Custom,
}

/// A `SubPageLink` target (rule 1: "large categories may additionally have
/// sub-pages… for content too large for a single scrolling page").
pub struct SubPage {
    pub title: &'static str,
    /// Deep-link slug suffix, e.g. `"advanced"` under `terminal/advanced`.
    pub slug: &'static str,
    pub body: PageBody,
}

pub struct SettingsPage {
    pub area: &'static AreaMeta,
    pub body: PageBody,
    pub sub_pages: Vec<SubPage>,
}

type Group = (&'static str, &'static [&'static str]);

/// Build every top-level page, in `AREAS` order (rule 1: "top-level
/// categories, in a fixed order").
pub fn pages() -> Vec<SettingsPage> {
    AREAS.iter().map(build_page).collect()
}

/// Resolve a deep-link slug (`"terminal"`, `"terminal/advanced"`, `"hosts/
/// ssh-config"`) to a `(page index, sub-page index)` pair against `pages`
/// (rule 7). Pure so it's testable without constructing a `SettingsView`
/// (`SettingsView::navigate_to_slug` is a thin wrapper around this).
pub fn resolve_slug(pages: &[SettingsPage], slug: &str) -> Option<(usize, Option<usize>)> {
    let (area_slug, sub_slug) = match slug.split_once('/') {
        Some((a, s)) => (a, Some(s)),
        None => (slug, None),
    };
    let area_idx = AREAS.iter().position(|a| a.slug == area_slug)?;
    let sub_idx =
        sub_slug.and_then(|s| pages[area_idx].sub_pages.iter().position(|sp| sp.slug == s));
    Some((area_idx, sub_idx))
}

fn build_page(area: &'static AreaMeta) -> SettingsPage {
    match area.kind {
        AreaKind::Generated => match area.key {
            "terminal" => SettingsPage {
                area,
                body: PageBody::Generated(items_from_groups(TERMINAL_MAIN)),
                sub_pages: vec![SubPage {
                    title: "Advanced",
                    slug: "advanced",
                    body: PageBody::Generated(items_from_groups(TERMINAL_ADVANCED)),
                }],
            },
            "editor" => SettingsPage {
                area,
                body: PageBody::Generated(items_from_groups(EDITOR_MAIN)),
                sub_pages: vec![SubPage {
                    title: "Display",
                    slug: "display",
                    body: PageBody::Generated(items_from_groups(EDITOR_DISPLAY)),
                }],
            },
            _ => SettingsPage {
                area,
                body: PageBody::Generated(items_from_groups(groups_for(area.key))),
                sub_pages: Vec::new(),
            },
        },
        // Custom top-level categories (rule 4): the body is a hand-written
        // render_fn dispatched by `SettingsView` on `area.key`; "ai" also
        // gets a Custom sub-page for its provider/agent/directive lists
        // (Notizen: AI must have a sub-page too).
        AreaKind::Custom if area.key == "ai" => SettingsPage {
            area,
            body: PageBody::Custom,
            sub_pages: vec![SubPage {
                title: "Providers & Agents",
                slug: "providers",
                body: PageBody::Custom,
            }],
        },
        // Hosts (T19-010): the main page embeds `HostManagerView` verbatim
        // (list + edit form + jump-hosts + tunnels are already one
        // component there, per the task's own Notizen — "nicht neu
        // bauen"); the deep-link slugs `hosts/list`/`hosts/edit` from the
        // task's Anweisungen both resolve to that same main-page body.
        // `ssh-config`/`availability` get their own sub-pages since they
        // are separately deep-linkable per the Akzeptanzkriterien.
        AreaKind::Custom if area.key == "hosts" => SettingsPage {
            area,
            body: PageBody::Custom,
            sub_pages: vec![
                SubPage {
                    title: "SSH Config",
                    slug: "ssh-config",
                    body: PageBody::Custom,
                },
                SubPage {
                    title: "Availability",
                    slug: "availability",
                    body: PageBody::Custom,
                },
            ],
        },
        AreaKind::Custom => SettingsPage {
            area,
            body: PageBody::Custom,
            sub_pages: Vec::new(),
        },
    }
}

/// Expand curated `(section, &[local_key])` groups into
/// `SettingsPageItem`s, dropping empty groups.
fn items_from_groups(groups: &'static [Group]) -> Vec<SettingsPageItem> {
    let mut items = Vec::new();
    for (label, keys) in groups {
        items.push(SettingsPageItem::SectionHeader(label));
        for key in *keys {
            items.push(SettingsPageItem::Item(key));
        }
    }
    items
}

/// Every local key already placed by *any* group across *any* page (main +
/// sub-pages) for the given area — used to compute each page's trailing
/// "Other" fallback so a field is never listed twice and never dropped.
pub fn placed_keys_for_area(area_key: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    match area_key {
        "terminal" => {
            out.extend(TERMINAL_MAIN.iter().flat_map(|(_, k)| k.iter().copied()));
            out.extend(
                TERMINAL_ADVANCED
                    .iter()
                    .flat_map(|(_, k)| k.iter().copied()),
            );
        }
        "editor" => {
            out.extend(EDITOR_MAIN.iter().flat_map(|(_, k)| k.iter().copied()));
            out.extend(EDITOR_DISPLAY.iter().flat_map(|(_, k)| k.iter().copied()));
        }
        "ai" => out.extend(AI_GROUPS.iter().flat_map(|(_, k)| k.iter().copied())),
        "personalization" => {
            out.extend(
                PERSONALIZATION_GROUPS
                    .iter()
                    .flat_map(|(_, k)| k.iter().copied()),
            );
        }
        _ => out.extend(
            groups_for(area_key)
                .iter()
                .flat_map(|(_, k)| k.iter().copied()),
        ),
    }
    out
}

/// Fields for `area_key` not covered by any curated group — appended as a
/// trailing "Other" section by the renderer so nothing added to `schema.rs`
/// is ever silently dropped just because `pages.rs` forgot to place it.
pub fn leftover_fields<'a>(area_key: &str, fields: &'a [AnyField]) -> Vec<&'a AnyField> {
    let placed = placed_keys_for_area(area_key);
    fields
        .iter()
        .filter(|f| {
            f.area() == area_key
                && !placed.contains(&f.local_key())
                && !DEDICATED_PANE_EXEMPTIONS.contains(&f.json_path)
        })
        .collect()
}

/// Which sub-page (slug, `""` for the main page) and section a field's local
/// key is placed under by a curated group — used by the search jump (T19-007)
/// to open the right sub-page and un-collapse the right section before
/// scrolling. `None` means the field isn't placed by any curated group (it
/// falls through to the trailing "Other" section on the area's main page).
pub fn section_label_for_field(
    area_key: &str,
    local_key: &str,
) -> Option<(&'static str, &'static str)> {
    let find = |groups: &'static [Group]| -> Option<&'static str> {
        groups
            .iter()
            .find(|(_, keys)| keys.contains(&local_key))
            .map(|(label, _)| *label)
    };
    match area_key {
        "terminal" => find(TERMINAL_MAIN)
            .map(|l| ("", l))
            .or_else(|| find(TERMINAL_ADVANCED).map(|l| ("advanced", l))),
        "editor" => find(EDITOR_MAIN)
            .map(|l| ("", l))
            .or_else(|| find(EDITOR_DISPLAY).map(|l| ("display", l))),
        "ai" => find(AI_GROUPS).map(|l| ("", l)),
        "personalization" => find(PERSONALIZATION_GROUPS).map(|l| ("", l)),
        _ => find(groups_for(area_key)).map(|l| ("", l)),
    }
}

fn groups_for(area_key: &str) -> &'static [Group] {
    match area_key {
        "general" => GENERAL_GROUPS,
        "appearance" => APPEARANCE_GROUPS,
        "file_manager" => FILE_MANAGER_GROUPS,
        "connections" => CONNECTIONS_GROUPS,
        "workspace" => WORKSPACE_GROUPS,
        _ => &[],
    }
}

/// AI's field grid (used inside its Custom render_fn — AI is `AreaKind::
/// Custom` per `AREAS`, but its body still renders the generic grid for its
/// scalar preferences before the bespoke provider/agent/directive sections,
/// exactly as rule 4 allows: "may still read/write fields under
/// `target_module`").
pub const AI_GROUPS: &[Group] = &[
    (
        "Defaults",
        &[
            "defaultModelId",
            "autocompleteEnabled",
            "autocompleteProvider",
            "autocompleteModelId",
        ],
    ),
    ("General", &["aiEnabled", "aiWarnDestructiveCommands"]),
    (
        "Behaviour",
        &[
            "aiAutoOpenMiniOnSend",
            "aiNotifyOnHeadlessCommand",
            "aiMaxAgentSteps",
            "aiTemperature",
            "aiTerminalContextLines",
            "aiShellMaxTimeoutSecs",
            "aiShellMaxOutputKb",
        ],
    ),
    (
        "Local Providers",
        &[
            "lmstudioBaseURL",
            "lmstudioChatModelId",
            "openaiCompatibleBaseURL",
            "openaiCompatibleModelId",
            "mlxBaseURL",
            "mlxChatModelId",
            "ollamaBaseURL",
            "ollamaChatModelId",
        ],
    ),
    ("Agent Instructions", &["customInstructions"]),
];

const GENERAL_GROUPS: &[Group] = &[
    ("Appearance", &["theme"]),
    (
        "Startup",
        &["defaultStartupTab", "startupTerminalCount", "autostart"],
    ),
    (
        "Session Restore",
        &[
            "sessionRestore",
            "sessionScrollbackLines",
            "scrollbackMaxSizeMb",
            "scrollbackRetentionDays",
        ],
    ),
    ("Window", &["restoreWindowState"]),
    ("Security", &["credentialEncryption"]),
    ("Quit", &["confirmQuitWithSsh"]),
    ("Updates", &["checkForUpdates"]),
    ("Notifications", &["notifyOnErrors"]),
];

const APPEARANCE_GROUPS: &[Group] = &[
    (
        "Typography",
        &[
            "appFontFamily",
            "appFontSize",
            "appLineHeight",
            "bufferFontFamily",
            "bufferFontSize",
            "bufferLineHeight",
        ],
    ),
    (
        "Density & Motion",
        &["uiDensity", "cornerRadiusScale", "reduceMotion"],
    ),
    (
        "Layout",
        &[
            "appCornerRadius",
            "tabsLocation",
            "sidebarGroupByFolder",
            "sidebarGroupSingleTabs",
            "badgesAlwaysVisible",
            "titlebarsIconsPosition",
            "sidebarTabInfoLine",
        ],
    ),
    (
        "Background",
        &[
            "backgroundImage",
            "backgroundOpacity",
            "backgroundBlur",
            "backgroundTintColor",
            "backgroundTintOpacity",
        ],
    ),
    ("Zen Mode", &["zenModeShowHeader", "zenModeShowStatusbar"]),
    ("Active Theme", &["appTheme", "themeVariantOverrides"]),
];

const TERMINAL_MAIN: &[Group] = &[
    (
        "Shell",
        &[
            "terminalShell",
            "terminalDefaultPath",
            "newTabInheritsCwd",
            "confirmCloseTerminalTab",
        ],
    ),
    (
        "Font",
        &[
            "terminalFontFamily",
            "terminalFontSize",
            "terminalFontWeight",
            "terminalLineHeight",
            "terminalLetterSpacing",
        ],
    ),
    (
        "Cursor",
        &[
            "terminalCursorStyle",
            "terminalCursorBlink",
            "terminalCursorBlinkInterval",
        ],
    ),
    ("Bell", &["terminalBell"]),
    ("Buffer", &["terminalScrollback"]),
    ("Appearance", &["terminalOpacity", "terminalUseWebgl"]),
];

const TERMINAL_ADVANCED: &[Group] = &[
    (
        "Layout",
        &["terminalShowPaneHeader", "terminalShowPaneFooter"],
    ),
    (
        "Composer & Blocks",
        &[
            "terminalComposerEnabled",
            "terminalComposerHistoryPopup",
            "terminalComposerArgumentCompletion",
            "terminalBlocksEnabled",
            "terminalBlocksAutoCollapseOnAltScreen",
        ],
    ),
    (
        "Input",
        &[
            "terminalCopyOnSelect",
            "terminalRightClickPastes",
            "terminalWordSeparator",
        ],
    ),
    (
        "Scrolling",
        &["terminalScrollSensitivity", "terminalFastScrollModifier"],
    ),
];

const EDITOR_MAIN: &[Group] = &[
    (
        "Keybindings",
        &[
            "vimMode",
            "editorRelativeLineNumbers",
            "vimHlsearch",
            "vimIncsearch",
            "vimSmartcase",
        ],
    ),
    ("Theme", &["editorTheme"]),
    (
        "Font",
        &["editorFontFamily", "editorFontSize", "editorLineHeight"],
    ),
    (
        "Behaviour",
        &[
            "editorFormatOnSave",
            "editorAutoSave",
            "editorAutoSaveDelay",
            "editorTabSize",
        ],
    ),
    ("Indentation", &["editorIndentWithTabs"]),
    ("Files", &["editorMaxFileSizeMb"]),
];

const EDITOR_DISPLAY: &[Group] = &[
    (
        "Display",
        &[
            "editorLineNumbers",
            "editorWordWrap",
            "editorBracketMatching",
            "editorShowCursorPosition",
            "editorShowSelectionStats",
            "editorShowOutline",
            "editorIndentationGuides",
        ],
    ),
    (
        "On Save",
        &["editorTrimTrailingWhitespace", "editorInsertFinalNewline"],
    ),
    ("AI Completion", &["editorAutocompleteDebounceMs"]),
];

const FILE_MANAGER_GROUPS: &[Group] = &[
    (
        "Browsing",
        &[
            "sftpShowHiddenFiles",
            "sftpShowUpFolder",
            "explorerShowHiddenByDefault",
        ],
    ),
    (
        "Explorer tree",
        &[
            "explorerIndentGuides",
            "explorerStickyAncestors",
            "explorerAutoRevealActiveFile",
            "explorerFoldSingleChildDirs",
            "explorerGitDecorations",
        ],
    ),
    ("Source Control", &["scmFileTree"]),
    (
        "Columns",
        &[
            "sftpColumnSize",
            "sftpColumnModified",
            "sftpColumnPermissions",
            "sftpColumnType",
        ],
    ),
    (
        "Remote Editing",
        &[
            "sftpRemoteEditShowTransfers",
            "sftpMaxRemoteFileSizeMb",
            "sftpFontSize",
        ],
    ),
    (
        "Transfers",
        &[
            "sftpMaxConcurrentTransfers",
            "sftpDefaultConflictResolution",
            "sftpChunkSizeKb",
            "sftpOnFolderFileError",
        ],
    ),
];

const CONNECTIONS_GROUPS: &[Group] = &[
    ("Host Availability", &["hostPingInterval"]),
    (
        "SSH Terminal Sessions",
        &[
            "sshConnectTimeoutSecs",
            "sshAutoReconnect",
            "sshAutoReconnectDelay",
            "sshAutoReconnectMaxAttempts",
        ],
    ),
    (
        "Remote File Browsing",
        &[
            "explorerRemotePollInterval",
            "explorerAutoReconnect",
            "explorerIdleSessionTimeoutMin",
            "explorerMaxIdleSessions",
            "explorerMaxCachedRemoteScopes",
        ],
    ),
];

/// Hosts' `hosts/availability` sub-page field grid (T19-010) — the
/// connections-area polling knobs rendered as normal generated
/// `SettingField`s inside the Hosts custom pane's body (task Notizen:
/// "Custom-Body heißt nicht 'keine generierten Felder'"). These fields are
/// also reachable from the Connections area's own page
/// (`CONNECTIONS_GROUPS`) — shown in both places deliberately, so managing a
/// host's reachability doesn't require leaving Settings › Hosts.
pub const HOSTS_AVAILABILITY_GROUPS: &[Group] = &[(
    "Availability Polling",
    &[
        "hostPingInterval",
        "sshConnectTimeoutSecs",
        "sshAutoReconnect",
        "sshAutoReconnectDelay",
        "sshAutoReconnectMaxAttempts",
    ],
)];

/// Personalization's field grid (used inside its Custom render_fn, same
/// pattern as `AI_GROUPS` — see `render_personalization`'s call to
/// `SettingsView::render_generated_body`).
pub const PERSONALIZATION_GROUPS: &[Group] = &[(
    "Status Bar Buttons",
    &[
        "statusBarShowExplorerButton",
        "statusBarShowSnippetsButton",
        "statusBarShowSourceControlButton",
        "statusBarShowTabsButton",
        "statusBarShowCwdBreadcrumb",
        "statusBarShowPreviewUrl",
        "statusBarShowAiControls",
    ],
)];

const WORKSPACE_GROUPS: &[Group] = &[
    (
        "Bookmarks",
        &[
            "bookmarksEnabled",
            "bookmarksActionNewTerminal",
            "bookmarksActionCurrentTerminal",
            "bookmarksActionCurrentSftp",
            "bookmarksActionNewSftp",
            "bookmarksPrimaryClickBehavior",
            "bookmarksShowBadge",
        ],
    ),
    (
        "Command Palette",
        &[
            "commandPaletteBlur",
            "commandPaletteOpacity",
            "commandPalettePosition",
            "commandPaletteAnimation",
            "commandPaletteShowRecent",
            "commandPaletteHistorySize",
            "commandPaletteSearchMode",
            "commandPaletteCloseOnOverlayClick",
        ],
    ),
    ("Source Control", &["gitStatusPollIntervalMs"]),
    (
        "Layout",
        &[
            "sidebarPosition",
            "sidebarOpen",
            "sidebarActivePanel",
            "sidebarRightOpen",
            "sidebarRightActivePanel",
            "sidebarWidth",
            "sidebarRightWidth",
            "dockLayout",
        ],
    ),
];

/// Fields covered by dedicated, non-generic pane UI rather than the
/// [`AnyField`] grid — the settings-design-contract rule 3/4 allowance for
/// "a field that needs something outside this table needs a new
/// renderer-registry entry… not a one-off widget inlined into a page" is
/// satisfied here by widgets that already existed before T19-004 and have
/// real side effects the generic grid does not (MCP writes call the live
/// backend bridge; Hosts is explicitly deferred to T19-010 per this task's
/// own Notizen). Each entry is documented, not silently dropped — see
/// `tests::every_field_is_reachable_generically_or_by_documented_exemption`.
pub const DEDICATED_PANE_EXEMPTIONS: &[&str] = &[
    // `render_agent_bridge` (panes/generic.rs) owns these 5 — each write also
    // calls the live backend (`mcp_set_port`, …), which a generic
    // SettingsContent-only write would not do.
    "mcp.bridgeEnabled",
    "mcp.bridgePort",
    "mcp.maxCommandTimeoutSecs",
    "mcp.autoRevokeMinutes",
    "mcp.notifyOnActivity",
    // `render_personalization` (panes/personalization.rs) owns the status
    // bar layout editor + panel-toggle visibility grid directly (they are
    // `BTreeMap`s driven by drag/drop + toggle rows, not a scalar control).
    "personalization.statusBarItemPlacements",
    "personalization.panelToggleVisibility",
    // Hosts (T19-010): `render_hosts_pane`/`render_hosts_ssh_config` own
    // every `hosts.*` field directly via the embedded `HostManagerView`
    // (list/edit form/jump-hosts/tunnels/SSH-config import-export) rather
    // than a generic scalar grid — `entries` is a `Vec<HostEntry>`, not a
    // single control, and the rest (`defaultShell`/`keepalive`/
    // `sshConfigImport`/`layout`/`sort`/`cardScale`) are written by that
    // same component's own save flow, not a generic field row.
    "hosts.entries",
    "hosts.defaultShell",
    "hosts.keepalive",
    "hosts.sshConfigImport",
    "hosts.layout",
    "hosts.sort",
    "hosts.cardScale",
    // `render_shortcuts` (panes/shortcuts.rs) renders this one field
    // directly as its "reset to preset" control, not via a generic grid.
    "keymap.baseKeymap",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::all_fields;
    use std::collections::HashSet;

    fn all_generated_and_custom_grid_keys() -> HashSet<String> {
        let mut out = HashSet::new();
        for page in pages() {
            let area = page.area.target_module;
            collect(&page.body, area, &mut out);
            for sp in &page.sub_pages {
                collect(&sp.body, area, &mut out);
            }
        }
        // AI's / Personalization's Custom bodies also render their own
        // generic-grid groups (see panes dispatch).
        for (_, keys) in AI_GROUPS {
            for k in *keys {
                out.insert(format!("ai.{k}"));
            }
        }
        for (_, keys) in PERSONALIZATION_GROUPS {
            for k in *keys {
                out.insert(format!("personalization.{k}"));
            }
        }
        out
    }

    fn collect(body: &PageBody, area: &str, out: &mut HashSet<String>) {
        if let PageBody::Generated(items) = body {
            for item in items {
                if let SettingsPageItem::Item(key) = item {
                    out.insert(format!("{area}.{key}"));
                }
            }
        }
    }

    /// Every `AnyField` is either placed by a curated group (asserted
    /// structurally elsewhere), covered by the trailing "Other" fallback for
    /// its area's page, or explicitly exempted with a documented reason.
    /// Together these three paths mean no `SettingsContent` field can go
    /// unreachable in the UI (`docs/settings-guidelines.md` rule 2/6).
    /// Areas whose page renders a "Other" leftover fallback for anything not
    /// placed by a curated group — the true `AreaKind::Generated` pages, plus
    /// the Custom areas that fold a generic grid into their body (AI,
    /// Personalization). Areas *not* in this list (Themes, Hosts, Shortcuts,
    /// MCP) render no generic fallback at all — every one of their fields
    /// must be either placed or a documented exemption.
    const AREAS_WITH_LEFTOVER_FALLBACK: &[&str] = &[
        "general",
        "appearance",
        "terminal",
        "editor",
        "file_manager",
        "connections",
        "workspace",
        "ai",
        "personalization",
    ];

    #[test]
    fn every_field_is_reachable_generically_or_by_documented_exemption() {
        let fields = all_fields();
        let placed = all_generated_and_custom_grid_keys();
        for f in &fields {
            let json_path = f.json_path;
            let has_fallback = AREAS_WITH_LEFTOVER_FALLBACK.contains(&f.area());
            let reachable_generically = placed.contains(json_path)
                || (has_fallback && !leftover_fields(f.area(), &fields).is_empty());
            let exempted = DEDICATED_PANE_EXEMPTIONS.contains(&json_path);
            assert!(
                reachable_generically || exempted,
                "field `{json_path}` is neither placed in a page, covered by the \
                 leftover fallback, nor a documented DEDICATED_PANE_EXEMPTIONS entry"
            );
        }
    }

    /// No leftover-fallback field is *also* a documented exemption — an
    /// exemption always means "this key never reaches the generic grid at
    /// all", not "it happens to also fall through".
    #[test]
    fn exemptions_are_never_double_counted_in_the_leftover_fallback() {
        let fields = all_fields();
        let placed = all_generated_and_custom_grid_keys();
        for exempt in DEDICATED_PANE_EXEMPTIONS {
            let area = exempt.split('.').next().unwrap();
            let local = exempt.rsplit('.').next().unwrap();
            assert!(
                !placed.contains(*exempt),
                "`{exempt}` is both a DEDICATED_PANE_EXEMPTIONS entry and placed in a page"
            );
            if AREAS_WITH_LEFTOVER_FALLBACK.contains(&area) {
                let leftover = leftover_fields(area, &fields);
                assert!(
                    !leftover.iter().any(|f| f.local_key() == local),
                    "`{exempt}` is exempted but would also render via the generic leftover fallback"
                );
            }
        }
    }

    #[test]
    fn every_area_has_a_page() {
        let pages = pages();
        assert_eq!(pages.len(), AREAS.len());
        for (page, area) in pages.iter().zip(AREAS.iter()) {
            assert_eq!(page.area.key, area.key);
        }
    }

    #[test]
    fn terminal_editor_ai_have_at_least_one_sub_page() {
        for page in pages() {
            if matches!(page.area.key, "terminal" | "editor" | "ai") {
                assert!(
                    !page.sub_pages.is_empty(),
                    "{} must have at least one SubPageLink (task Notizen)",
                    page.area.key
                );
            }
        }
    }

    #[test]
    fn section_label_for_field_resolves_sub_page_and_section() {
        assert_eq!(
            section_label_for_field("terminal", "terminalCursorStyle"),
            Some(("", "Cursor"))
        );
        assert_eq!(
            section_label_for_field("terminal", "terminalScrollSensitivity"),
            Some(("advanced", "Scrolling"))
        );
        assert_eq!(section_label_for_field("terminal", "doesNotExist"), None);
    }

    #[test]
    fn every_slug_within_a_page_is_unique() {
        for page in pages() {
            let mut seen = HashSet::new();
            for sp in &page.sub_pages {
                assert!(
                    seen.insert(sp.slug),
                    "duplicate sub-page slug `{}`",
                    sp.slug
                );
            }
        }
    }

    // ── T19-010: Hosts category + its deep links ────────────────────────

    #[test]
    fn hosts_is_a_top_level_custom_category_peer_of_themes() {
        let pages = pages();
        let hosts = pages.iter().find(|p| p.area.key == "hosts").unwrap();
        let themes = pages.iter().find(|p| p.area.key == "themes").unwrap();
        assert!(matches!(hosts.body, PageBody::Custom));
        assert!(matches!(themes.body, PageBody::Custom));
        assert_eq!(hosts.area.slug, "hosts");
        // Peers: both are direct entries in `AREAS`, not nested under
        // another category.
        assert!(AREAS.iter().any(|a| a.key == "hosts"));
        assert!(AREAS.iter().any(|a| a.key == "themes"));
    }

    #[test]
    fn hosts_has_ssh_config_and_availability_sub_pages() {
        let pages = pages();
        let hosts = pages.iter().find(|p| p.area.key == "hosts").unwrap();
        let slugs: Vec<&str> = hosts.sub_pages.iter().map(|sp| sp.slug).collect();
        assert!(slugs.contains(&"ssh-config"));
        assert!(slugs.contains(&"availability"));
    }

    #[test]
    fn resolve_slug_lands_on_hosts_deep_links() {
        let pages = pages();
        let hosts_idx = pages.iter().position(|p| p.area.key == "hosts").unwrap();

        // `settings://hosts` — main page, no sub-page.
        assert_eq!(resolve_slug(&pages, "hosts"), Some((hosts_idx, None)));

        // `settings://hosts/ssh-config` — the SSH Config sub-page.
        let ssh_config_idx = pages[hosts_idx]
            .sub_pages
            .iter()
            .position(|sp| sp.slug == "ssh-config")
            .unwrap();
        assert_eq!(
            resolve_slug(&pages, "hosts/ssh-config"),
            Some((hosts_idx, Some(ssh_config_idx)))
        );
    }

    #[test]
    fn resolve_slug_rejects_unknown_area_or_sub_page() {
        let pages = pages();
        assert_eq!(resolve_slug(&pages, "does-not-exist"), None);
        // An unknown sub-page under a real area resolves the area with no
        // sub-page selected, rather than failing outright.
        assert_eq!(
            resolve_slug(&pages, "hosts/does-not-exist"),
            Some((
                pages.iter().position(|p| p.area.key == "hosts").unwrap(),
                None
            ))
        );
    }
}
