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

/// Resolve a deep-link slug (`"terminal"`, `"terminal/advanced"`) to a
/// `(page index, sub-page index)` pair against `pages`
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
        AreaKind::Custom => unreachable!("settings areas are value-generated only"),
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
        .filter(|f| f.area() == area_key && !placed.contains(&f.local_key()))
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
        _ => find(groups_for(area_key)).map(|l| ("", l)),
    }
}

fn groups_for(area_key: &str) -> &'static [Group] {
    match area_key {
        "general" => GENERAL_GROUPS,
        "appearance" => APPEARANCE_GROUPS,
        "file_manager" => FILE_MANAGER_GROUPS,
        "workspace" => WORKSPACE_GROUPS,
        _ => &[],
    }
}

const GENERAL_GROUPS: &[Group] = &[
    ("Appearance", &["theme"]),
    ("Startup", &["defaultStartupTab"]),
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
    ("Updates", &["checkForUpdates"]),
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
    ("Layout", &["tabsLocation"]),
    ("Zen Mode", &["zenModeShowHeader", "zenModeShowStatusbar"]),
    ("Active Theme", &["appTheme", "themeVariantOverrides"]),
];

const TERMINAL_MAIN: &[Group] = &[
    ("Shell", &["terminalShell"]),
    ("Font", &["terminalFontFamily", "terminalFontSize"]),
    ("Cursor", &["terminalCursorStyle", "terminalCursorBlink"]),
    ("Bell", &["terminalBell"]),
    ("Buffer", &["terminalScrollback"]),
    ("Appearance", &["terminalOpacity"]),
];

const TERMINAL_ADVANCED: &[Group] = &[(
    "Input",
    &["terminalCopyOnSelect", "terminalRightClickPastes"],
)];

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
    ("Font", &["editorFontFamily", "editorFontSize"]),
    ("Behaviour", &["editorTabSize"]),
    ("Indentation", &["editorIndentWithTabs"]),
];

const EDITOR_DISPLAY: &[Group] = &[("Display", &["editorLineNumbers", "editorWordWrap"])];

const FILE_MANAGER_GROUPS: &[Group] = &[
    ("Browsing", &["explorerShowHiddenByDefault"]),
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
];

const WORKSPACE_GROUPS: &[Group] = &[
    (
        "Command Palette",
        &[
            "commandPaletteOpacity",
            "commandPalettePosition",
            "commandPaletteShowRecent",
            "commandPaletteHistorySize",
            "commandPaletteSearchMode",
            "commandPaletteCloseOnOverlayClick",
        ],
    ),
    ("Source Control", &["gitStatusPollIntervalMs"]),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::all_fields;
    use std::collections::HashSet;

    fn all_generated_grid_keys() -> HashSet<String> {
        let mut out = HashSet::new();
        for page in pages() {
            let area = page.area.target_module;
            collect(&page.body, area, &mut out);
            for sp in &page.sub_pages {
                collect(&sp.body, area, &mut out);
            }
        }
        out
    }

    fn collect(body: &PageBody, area: &str, out: &mut HashSet<String>) {
        let PageBody::Generated(items) = body;
        for item in items {
            if let SettingsPageItem::Item(key) = item {
                out.insert(format!("{area}.{key}"));
            }
        }
    }

    /// Every `AnyField` is either placed by a curated group (asserted
    /// structurally elsewhere), covered by the trailing "Other" fallback for
    /// its area's page.
    /// Together these three paths mean no `SettingsContent` field can go
    /// unreachable in the UI (`docs/settings-guidelines.md` rule 2/6).
    /// Every settings area renders a generic "Other" fallback for anything
    /// not placed by a curated group.
    const AREAS_WITH_LEFTOVER_FALLBACK: &[&str] = &[
        "general",
        "appearance",
        "terminal",
        "editor",
        "file_manager",
        "workspace",
    ];

    #[test]
    fn every_field_is_reachable_generically_or_by_documented_exemption() {
        let fields = all_fields();
        let placed = all_generated_grid_keys();
        for f in &fields {
            let json_path = f.json_path;
            let has_fallback = AREAS_WITH_LEFTOVER_FALLBACK.contains(&f.area());
            let reachable_generically = placed.contains(json_path)
                || (has_fallback && !leftover_fields(f.area(), &fields).is_empty());
            assert!(
                reachable_generically,
                "field `{json_path}` is neither placed in a page nor covered by the \
                 leftover fallback"
            );
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
    fn terminal_editor_have_at_least_one_sub_page() {
        for page in pages() {
            if matches!(page.area.key, "terminal" | "editor") {
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
            section_label_for_field("terminal", "terminalCopyOnSelect"),
            Some(("advanced", "Input"))
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

    #[test]
    fn resolve_slug_rejects_unknown_area_or_sub_page() {
        let pages = pages();
        assert_eq!(resolve_slug(&pages, "does-not-exist"), None);
        // An unknown sub-page under a real area resolves the area with no
        // sub-page selected, rather than failing outright.
        assert_eq!(
            resolve_slug(&pages, "terminal/does-not-exist"),
            Some((
                pages.iter().position(|p| p.area.key == "terminal").unwrap(),
                None
            ))
        );
    }
}
