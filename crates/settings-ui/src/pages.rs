//! Declarative Settings navigation and field placement.
//!
//! Visible pages are independent of `SettingsContent`'s persisted JSON groups.
//! Page rows use stable JSON paths, while unplaced fields fall back to the
//! first page that owns their persisted group. This lets the UI taxonomy evolve
//! without renaming stored settings or creating duplicate field editors.

use crate::schema::AnyField;
use crate::services::{SettingsSurface, SettingsSurfaceId};

/// One row in a generated page's body.
pub enum SettingsPageItem {
    /// A collapsible disclosure heading.
    SectionHeader(&'static str),
    /// An [`AnyField`], referenced by its stable JSON path.
    Item(&'static str),
    /// Navigate to the canonical feature-owned surface for this preference.
    OwnerSurface(SettingsSurfaceId),
}

/// What a page or sub-page renders below the standard chrome.
pub enum PageBody {
    /// Rendered from curated placement groups plus its owning fallback fields.
    Generated(Vec<SettingsPageItem>),
}

/// A `SubPageLink` target for content too large for one scrolling page.
pub struct SubPage {
    pub title: &'static str,
    /// Deep-link slug suffix, e.g. `"advanced"` under `terminal/advanced`.
    pub slug: &'static str,
    pub body: PageBody,
}

/// A visible Settings category. `fallback_areas` name persisted value groups;
/// they are deliberately separate from the page key and title.
pub struct SettingsPage {
    pub key: &'static str,
    pub title: &'static str,
    pub slug: &'static str,
    pub fallback_areas: &'static [&'static str],
    pub body: PageBody,
    pub sub_pages: Vec<SubPage>,
}

/// Resolved location for a field in the Settings navigation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldLocation {
    pub page_index: usize,
    pub sub_page_slug: Option<&'static str>,
    pub section: Option<&'static str>,
}

type Group = (&'static str, &'static [&'static str]);

macro_rules! paths {
    ($area:literal: $($key:literal),* $(,)?) => {
        &[$(concat!($area, ".", $key)),*]
    };
}

/// Build the Settings navigation in its visible product order.
pub fn pages(surfaces: &[SettingsSurface]) -> Vec<SettingsPage> {
    let mut pages = vec![
        page("general", "General", &["general"], GENERAL_GROUPS, vec![]),
        appearance_page(),
        page("editor", "Editor", &["editor"], EDITOR_GROUPS, vec![]),
        page(
            "languages-tools",
            "Languages & Tools",
            &[],
            LANGUAGE_GROUPS,
            vec![],
        ),
        page(
            "search-files",
            "Search & Files",
            &["file_manager"],
            FILE_GROUPS,
            vec![],
        ),
        page(
            "window-layout",
            "Window & Layout",
            &["workspace"],
            LAYOUT_GROUPS,
            vec![],
        ),
        SettingsPage {
            key: "terminal",
            title: "Terminal",
            slug: "terminal",
            fallback_areas: &["terminal"],
            body: PageBody::Generated(items_from_groups(TERMINAL_MAIN)),
            sub_pages: vec![SubPage {
                title: "Advanced",
                slug: "advanced",
                body: PageBody::Generated(items_from_groups(TERMINAL_ADVANCED)),
            }],
        },
        page(
            "version-control",
            "Version Control",
            &[],
            VERSION_CONTROL_GROUPS,
            vec![],
        ),
        page(
            "network",
            "Network",
            &["connections"],
            NETWORK_GROUPS,
            vec![],
        ),
    ];
    compose_owner_surfaces(&mut pages, surfaces);
    pages
}

fn page(
    key: &'static str,
    title: &'static str,
    fallback_areas: &'static [&'static str],
    groups: &'static [Group],
    sub_pages: Vec<SubPage>,
) -> SettingsPage {
    SettingsPage {
        key,
        title,
        slug: key,
        fallback_areas,
        body: PageBody::Generated(items_from_groups(groups)),
        sub_pages,
    }
}

fn appearance_page() -> SettingsPage {
    let mut items = vec![
        SettingsPageItem::SectionHeader("Theme"),
        SettingsPageItem::Item("general.theme"),
        SettingsPageItem::SectionHeader("Background"),
    ];
    items.extend(items_from_groups(&APPEARANCE_GROUPS[1..]));
    SettingsPage {
        key: "appearance",
        title: "Appearance",
        slug: "appearance",
        fallback_areas: &["appearance"],
        body: PageBody::Generated(items),
        sub_pages: Vec::new(),
    }
}

fn compose_owner_surfaces(pages: &mut Vec<SettingsPage>, surfaces: &[SettingsSurface]) {
    let mut contributed_pages: Vec<(usize, &'static str, SettingsPage)> = Vec::new();
    for surface in surfaces {
        if let Some(page) = pages.iter_mut().find(|page| page.slug == surface.page.slug) {
            insert_owner_surface(page, surface);
            continue;
        }

        if let Some((_, _, page)) = contributed_pages
            .iter_mut()
            .find(|(_, slug, _)| *slug == surface.page.slug)
        {
            insert_owner_surface(page, surface);
            continue;
        }

        let mut page = SettingsPage {
            key: surface.page.slug,
            title: surface.page.title,
            slug: surface.page.slug,
            fallback_areas: &[],
            body: PageBody::Generated(Vec::new()),
            sub_pages: Vec::new(),
        };
        insert_owner_surface(&mut page, surface);
        let insert_at = surface.page.insert_at.unwrap_or(pages.len());
        contributed_pages.push((insert_at, surface.page.slug, page));
    }

    contributed_pages.sort_by_key(|(insert_at, slug, _)| (*insert_at, *slug));
    for (inserted, (insert_at, _, page)) in contributed_pages.into_iter().enumerate() {
        let position = insert_at.saturating_add(inserted).min(pages.len());
        pages.insert(position, page);
    }
}

fn insert_owner_surface(page: &mut SettingsPage, surface: &SettingsSurface) {
    let PageBody::Generated(items) = &mut page.body;
    let Some(section_index) = items.iter().position(
        |item| matches!(item, SettingsPageItem::SectionHeader(section) if *section == surface.page.section),
    ) else {
        items.push(SettingsPageItem::SectionHeader(surface.page.section));
        items.push(SettingsPageItem::OwnerSurface(surface.id));
        return;
    };
    let insert_at = items[section_index + 1..]
        .iter()
        .position(|item| matches!(item, SettingsPageItem::SectionHeader(_)))
        .map(|offset| section_index + 1 + offset)
        .unwrap_or(items.len());
    items.insert(insert_at, SettingsPageItem::OwnerSurface(surface.id));
}

/// Resolve a deep-link slug (`"terminal"`, `"terminal/advanced"`) to a
/// `(page index, sub-page index)` pair.
pub fn resolve_slug(pages: &[SettingsPage], slug: &str) -> Option<(usize, Option<usize>)> {
    let (page_slug, sub_slug) = match slug.split_once('/') {
        Some((page, sub_page)) => (page, Some(sub_page)),
        None => (slug, None),
    };
    let page_index = pages.iter().position(|page| page.slug == page_slug)?;
    let sub_page_index = sub_slug.and_then(|slug| {
        pages[page_index]
            .sub_pages
            .iter()
            .position(|page| page.slug == slug)
    });
    Some((page_index, sub_page_index))
}

/// Resolve an explicit placement first, then the page that owns the field's
/// persisted value group. Search and deep links use this same route.
pub fn field_location(pages: &[SettingsPage], field: &AnyField) -> Option<FieldLocation> {
    if let Some(location) = explicit_field_location(pages, field.json_path) {
        return Some(location);
    }
    pages
        .iter()
        .position(|page| page.fallback_areas.contains(&field.area()))
        .map(|page_index| FieldLocation {
            page_index,
            sub_page_slug: None,
            section: None,
        })
}

/// Fields not explicitly placed in any page and owned by this page's fallback
/// groups. An explicit cross-page placement always wins, so a field is never
/// rendered twice merely because its stored group has a fallback page.
pub fn leftover_fields<'a>(
    page_index: usize,
    pages: &[SettingsPage],
    fields: &'a [AnyField],
) -> Vec<&'a AnyField> {
    fields
        .iter()
        .filter(|field| {
            explicit_field_location(pages, field.json_path).is_none()
                && field_location(pages, field).is_some_and(|location| {
                    location.page_index == page_index
                        && pages[page_index].fallback_areas.contains(&field.area())
                })
        })
        .collect()
}

/// Find the page, sub-page slug, and section of a field explicitly placed by
/// the page groups. Fallback fields are intentionally returned as `None` so
/// callers can keep them in the page's trailing “Other” section.
pub fn section_label_for_field(pages: &[SettingsPage], json_path: &str) -> Option<FieldLocation> {
    explicit_field_location(pages, json_path)
}

fn explicit_field_location(pages: &[SettingsPage], json_path: &str) -> Option<FieldLocation> {
    for (page_index, page) in pages.iter().enumerate() {
        let PageBody::Generated(items) = &page.body;
        if let Some(section) = section_for_path(items, json_path) {
            return Some(FieldLocation {
                page_index,
                sub_page_slug: None,
                section: Some(section),
            });
        }
        for sub_page in &page.sub_pages {
            let PageBody::Generated(items) = &sub_page.body;
            if let Some(section) = section_for_path(items, json_path) {
                return Some(FieldLocation {
                    page_index,
                    sub_page_slug: Some(sub_page.slug),
                    section: Some(section),
                });
            }
        }
    }
    None
}

fn section_for_path(items: &[SettingsPageItem], json_path: &str) -> Option<&'static str> {
    let mut section = None;
    for item in items {
        match item {
            SettingsPageItem::SectionHeader(label) => section = Some(*label),
            SettingsPageItem::Item(path) if *path == json_path => return section,
            SettingsPageItem::Item(_) | SettingsPageItem::OwnerSurface(_) => {}
        }
    }
    None
}

/// Expand curated `(section, &[json_path])` groups into page rows.
fn items_from_groups(groups: &'static [Group]) -> Vec<SettingsPageItem> {
    let mut items = Vec::new();
    for (label, paths) in groups {
        items.push(SettingsPageItem::SectionHeader(label));
        for path in *paths {
            items.push(SettingsPageItem::Item(path));
        }
    }
    items
}

const GENERAL_GROUPS: &[Group] = &[
    ("Startup", paths!("general": "defaultStartupTab")),
    ("Session Restore", paths!("general": "sessionRestore")),
    (
        "Command Palette",
        paths!("workspace":
            "commandPaletteOpacity",
            "commandPalettePosition",
            "commandPaletteShowRecent",
            "commandPaletteHistorySize",
            "commandPaletteSearchMode",
            "commandPaletteCloseOnOverlayClick"
        ),
    ),
    ("Updates", paths!("general": "checkForUpdates")),
    ("Quit", paths!("workspace": "confirmQuitWithActiveSessions")),
];

const APPEARANCE_GROUPS: &[Group] = &[
    ("Theme", paths!("general": "theme")),
    (
        "Typography",
        paths!("appearance":
            "appFontFamily",
            "appFontSize",
            "appLineHeight",
            "bufferFontFamily",
            "bufferFontSize",
            "bufferLineHeight"
        ),
    ),
    (
        "Density & Motion",
        paths!("appearance":
            "uiDensity",
            "cornerRadiusScale",
            "reduceMotion"
        ),
    ),
];

const EDITOR_GROUPS: &[Group] = &[
    (
        "Keybindings",
        paths!("editor":
            "vimMode",
            "editorRelativeLineNumbers",
            "vimHlsearch",
            "vimIncsearch",
            "vimSmartcase"
        ),
    ),
    ("Theme", paths!("editor": "editorTheme")),
    (
        "Font",
        paths!("editor":
            "editorFontFamily",
            "editorFontSize",
            "editorLineHeight"
        ),
    ),
    ("Behaviour", paths!("editor": "editorTabSize")),
    ("Indentation", paths!("editor": "editorIndentWithTabs")),
    (
        "Display",
        paths!("editor":
            "editorLineNumbers",
            "editorWordWrap",
            "editorWhitespace",
            "editorMinimap",
            "editorRulers",
            "editorScrollBeyondLastLine",
            "editorStickyContext",
            "editorHighlightCurrentLine"
        ),
    ),
    (
        "Caret",
        paths!("editor":
            "editorCursorStyle",
            "editorCursorBlink",
            "editorCursorBlinkIntervalMs"
        ),
    ),
];

const LANGUAGE_GROUPS: &[Group] = &[(
    "Language Services",
    paths!("editor":
        "editorDiagnostics",
        "editorSemanticTokens",
        "editorCompletion",
        "editorHover"
    ),
)];

const FILE_GROUPS: &[Group] = &[
    (
        "Browsing",
        paths!("file_manager": "explorerShowHiddenByDefault"),
    ),
    (
        "Explorer Tree",
        paths!("file_manager":
            "explorerIndentGuides",
            "explorerStickyAncestors",
            "explorerAutoRevealActiveFile",
            "explorerFoldSingleChildDirs"
        ),
    ),
    (
        "SFTP Browser",
        paths!("file_manager":
            "sftpShowHiddenFiles",
            "sftpShowUpFolder",
            "sftpZebraStriping",
            "sftpRelativeTimes",
            "sftpColumns"
        ),
    ),
];

const LAYOUT_GROUPS: &[Group] = &[
    ("Window", paths!("general": "restoreWindowState")),
    ("Tabs", paths!("appearance": "tabsLocation")),
    (
        "Zen Mode",
        paths!("appearance":
            "zenModeShowHeader",
            "zenModeShowStatusbar"
        ),
    ),
];

const TERMINAL_MAIN: &[Group] = &[
    ("Shell", paths!("terminal": "terminalShell")),
    (
        "Font",
        paths!("terminal":
            "terminalFontFamily",
            "terminalFontSize",
            "terminalLineHeight",
            "terminalFontWeight"
        ),
    ),
    (
        "Cursor",
        paths!("terminal":
            "terminalCursorStyle",
            "terminalCursorBlink",
            "terminalCursorBlinkInterval"
        ),
    ),
    (
        "Scrolling",
        paths!("terminal":
            "terminalScrollback",
            "terminalScrollSensitivity",
            "terminalFastScrollModifier",
            "sessionScrollbackLines",
            "scrollbackMaxSizeMb",
            "scrollbackRetentionDays"
        ),
    ),
    ("Bell", paths!("terminal": "terminalBell")),
    ("Appearance", paths!("terminal": "terminalOpacity")),
];

const TERMINAL_ADVANCED: &[Group] = &[
    (
        "Input",
        paths!("terminal":
            "terminalCopyOnSelect",
            "terminalRightClickPastes",
            "terminalWordSeparator"
        ),
    ),
    ("Tabs", paths!("terminal": "confirmCloseTerminalTab")),
    (
        "Environment",
        paths!("terminal":
            "terminalEnvironmentVariables",
            "terminalShellArgs"
        ),
    ),
];

const VERSION_CONTROL_GROUPS: &[Group] = &[
    (
        "Editor",
        paths!("editor":
            "editorGitGutter",
            "editorGitWordDiff"
        ),
    ),
    (
        "Explorer & Source Control",
        paths!("file_manager":
            "explorerGitDecorations",
            "scmFileTree"
        ),
    ),
];

const NETWORK_GROUPS: &[Group] = &[(
    "SSH Defaults",
    paths!("connections":
        "sshConnectTimeoutSecs",
        "sshKeepaliveIntervalSecs",
        "sshKeepaliveMaxFailures"
    ),
)];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::all_fields;

    fn pages() -> Vec<SettingsPage> {
        let surfaces = [
            SettingsSurface::new(
                SettingsSurfaceId::new("app-themes"),
                "App theme",
                "Choose a color theme and preview its variants.",
                "Choose theme…",
                crate::services::SettingsSurfacePage::section("appearance", "Appearance", "Theme"),
            ),
            SettingsSurface::new(
                SettingsSurfaceId::new("icon-themes"),
                "Icon theme",
                "Choose the icon set used throughout the app.",
                "Choose icons…",
                crate::services::SettingsSurfacePage::section("appearance", "Appearance", "Theme"),
            ),
            SettingsSurface::new(
                SettingsSurfaceId::new("backgrounds"),
                "Background image",
                "Choose a background image and adjust its appearance.",
                "Choose background…",
                crate::services::SettingsSurfacePage::section(
                    "appearance",
                    "Appearance",
                    "Background",
                ),
            ),
            SettingsSurface::new(
                SettingsSurfaceId::new("keymap"),
                "Keyboard shortcuts",
                "Browse and edit the shortcuts provided by Labonair.",
                "Open keymap editor…",
                crate::services::SettingsSurfacePage::new_page(
                    "keymap",
                    "Keymap",
                    "Keyboard shortcuts",
                    2,
                ),
            ),
        ];
        super::pages(&surfaces)
    }
    use std::collections::HashSet;

    fn all_placed_paths(pages: &[SettingsPage]) -> Vec<&'static str> {
        let mut paths = Vec::new();
        for page in pages {
            collect(&page.body, &mut paths);
            for sub_page in &page.sub_pages {
                collect(&sub_page.body, &mut paths);
            }
        }
        paths
    }

    fn collect(body: &PageBody, paths: &mut Vec<&'static str>) {
        let PageBody::Generated(items) = body;
        for item in items {
            match item {
                SettingsPageItem::Item(path) => paths.push(path),
                SettingsPageItem::SectionHeader(_) | SettingsPageItem::OwnerSurface(_) => {}
            }
        }
    }

    #[test]
    fn every_field_resolves_to_one_page_without_duplicate_explicit_rows() {
        let pages = pages();
        let fields = all_fields();
        let placed = all_placed_paths(&pages);
        let mut unique = HashSet::new();
        for path in &placed {
            assert!(unique.insert(path), "explicit placement repeats `{path}`");
            assert!(
                fields.iter().any(|field| field.json_path == *path),
                "explicit placement references missing field `{path}`"
            );
        }

        for field in &fields {
            assert!(
                field_location(&pages, field).is_some(),
                "field `{}` is not reachable from Settings",
                field.json_path
            );
        }
    }

    #[test]
    fn navigation_taxonomy_is_independent_of_persisted_areas() {
        let pages = pages();
        let fields = all_fields();
        assert_eq!(pages.len(), 10);
        assert_eq!(pages[4].title, "Languages & Tools");
        assert_eq!(pages[5].title, "Search & Files");
        assert_eq!(pages[6].title, "Window & Layout");
        assert_eq!(pages[8].title, "Version Control");
        assert_eq!(pages[9].title, "Network");
        for (page_index, page) in pages.iter().enumerate() {
            let has_fields = fields.iter().any(|field| {
                field_location(&pages, field)
                    .is_some_and(|location| location.page_index == page_index)
            });
            let PageBody::Generated(items) = &page.body;
            let has_owner_surface = items
                .iter()
                .any(|item| matches!(item, SettingsPageItem::OwnerSurface(_)));
            assert!(
                has_fields || has_owner_surface,
                "Settings category `{}` has no content",
                page.title
            );
        }
        assert_eq!(
            field_location(
                &pages,
                fields
                    .iter()
                    .find(|field| field.json_path == "file_manager.scmFileTree")
                    .unwrap()
            )
            .unwrap()
            .page_index,
            8
        );
    }

    #[test]
    fn keymap_page_links_to_the_canonical_keymap_owner() {
        let pages = pages();
        let (page_index, sub_page_index) = resolve_slug(&pages, "keymap").unwrap();
        assert_eq!(sub_page_index, None);
        assert_eq!(pages[page_index].title, "Keymap");
        let PageBody::Generated(items) = &pages[page_index].body;
        assert!(items.iter().any(|item| matches!(
            item,
            SettingsPageItem::OwnerSurface(id) if *id == SettingsSurfaceId::new("keymap")
        )));
    }

    #[test]
    fn appearance_links_to_the_owner_theme_surfaces() {
        let pages = pages();
        let appearance = pages
            .iter()
            .find(|page| page.key == "appearance")
            .expect("Appearance page exists");
        let PageBody::Generated(items) = &appearance.body;
        let surfaces: Vec<_> = items
            .iter()
            .filter_map(|item| match item {
                SettingsPageItem::OwnerSurface(surface) => Some(*surface),
                SettingsPageItem::SectionHeader(_) | SettingsPageItem::Item(_) => None,
            })
            .collect();
        assert_eq!(
            surfaces,
            [
                SettingsSurfaceId::new("app-themes"),
                SettingsSurfaceId::new("icon-themes"),
                SettingsSurfaceId::new("backgrounds"),
            ]
        );
        assert_eq!(
            section_label_for_field(&pages, "general.theme").map(|location| location.section),
            Some(Some("Theme"))
        );
    }

    #[test]
    fn field_section_lookup_routes_cross_group_placements() {
        let pages = pages();
        assert_eq!(
            section_label_for_field(&pages, "terminal.terminalCursorStyle"),
            Some(FieldLocation {
                page_index: 7,
                sub_page_slug: None,
                section: Some("Cursor"),
            })
        );
        assert_eq!(
            section_label_for_field(&pages, "terminal.terminalCopyOnSelect"),
            Some(FieldLocation {
                page_index: 7,
                sub_page_slug: Some("advanced"),
                section: Some("Input"),
            })
        );
        assert_eq!(
            section_label_for_field(&pages, "editor.editorGitWordDiff")
                .map(|location| (location.page_index, location.section)),
            Some((8, Some("Editor")))
        );
        assert_eq!(
            section_label_for_field(&pages, "general.restoreWindowState")
                .map(|location| (location.page_index, location.section)),
            Some((6, Some("Window")))
        );
        assert_eq!(section_label_for_field(&pages, "terminal.notPresent"), None);
    }

    #[test]
    fn explicit_cross_page_fields_are_not_in_their_storage_fallback() {
        let pages = pages();
        let fields = all_fields();
        let appearance = pages
            .iter()
            .position(|page| page.key == "appearance")
            .unwrap();
        let layout = pages
            .iter()
            .position(|page| page.key == "window-layout")
            .unwrap();
        assert!(leftover_fields(appearance, &pages, &fields)
            .iter()
            .all(|field| field.json_path != "appearance.tabsLocation"));
        assert!(leftover_fields(layout, &pages, &fields)
            .iter()
            .all(|field| field.json_path != "general.restoreWindowState"));
    }

    #[test]
    fn resolve_slug_rejects_unknown_page_or_sub_page() {
        let pages = pages();
        assert_eq!(resolve_slug(&pages, "does-not-exist"), None);
        assert_eq!(
            resolve_slug(&pages, "terminal/does-not-exist"),
            Some((7, None))
        );
        assert_eq!(
            resolve_slug(&pages, "terminal/advanced"),
            Some((7, Some(0)))
        );
    }
}
