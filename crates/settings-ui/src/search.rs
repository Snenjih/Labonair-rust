//! Global settings search (T19-007), replacing the T19-004 in-page substring
//! filter (`panes/generic.rs`'s old `render_global_search`, which rendered
//! matched fields inline). This module is pure data: an index built once
//! (task Warnung: never rebuilt per keystroke) over every generated field's
//! title + description + `json_path`, and a scorer on top of
//! the shared fuzzy matcher (`labonair_command_palette::fuzzy`, already used
//! by the command palette / `@`-file picker). `crate::view`/`crate::panes`
//! own the rendering + keyboard navigation on top of this.

use labonair_command_palette::{match_score, SearchMode};

use crate::pages::{field_location, PageBody, SettingsPage, SettingsPageItem};
use crate::schema::AnyField;
use crate::services::{SettingsSurface, SettingsSurfaceId};

/// What a search hit navigates to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchTarget {
    /// Indexes [`crate::view::SettingsView::all_fields`].
    Field(usize),
    /// Opens a canonical surface owned by another capability.
    OwnerSurface(SettingsSurfaceId),
}

/// One indexed, searchable entry.
struct SearchEntry {
    target: SearchTarget,
    page_index: usize,
    page_title: &'static str,
    haystack: String,
}

/// One scored, render-ready hit — `Copy` so it can be cached in
/// `SettingsView` and iterated without holding a borrow of the index.
#[derive(Clone, Copy)]
pub(crate) struct SearchRow {
    pub(crate) target: SearchTarget,
    pub(crate) page_index: usize,
    pub(crate) page_title: &'static str,
}

/// Build the full search index over every SettingsContent field. Rebuild only
/// when the settings window opens / its schema changes (task Warnung) — never
/// on every keystroke.
fn build_index(
    all_fields: &[AnyField],
    pages: &[SettingsPage],
    surfaces: &[SettingsSurface],
) -> Vec<SearchEntry> {
    let mut out = Vec::new();
    for (i, field) in all_fields.iter().enumerate() {
        let Some(location) = field_location(pages, field) else {
            continue;
        };
        out.push(SearchEntry {
            target: SearchTarget::Field(i),
            page_index: location.page_index,
            page_title: pages[location.page_index].title,
            haystack: format!(
                "{} {} {}",
                field.meta.title, field.meta.description, field.json_path
            ),
        });
    }
    for (page_index, page) in pages.iter().enumerate() {
        let PageBody::Generated(items) = &page.body;
        for item in items {
            let SettingsPageItem::OwnerSurface(id) = item else {
                continue;
            };
            let Some(surface) = surfaces.iter().find(|surface| surface.id == *id) else {
                continue;
            };
            out.push(SearchEntry {
                target: SearchTarget::OwnerSurface(*id),
                page_index,
                page_title: page.title,
                haystack: format!(
                    "{} {} {} {}",
                    page.title, surface.title, surface.description, surface.action_label
                ),
            });
        }
    }
    out
}

/// A built index, opaque to callers beyond [`search`].
pub(crate) struct SearchIndex(Vec<SearchEntry>);

impl SearchIndex {
    pub(crate) fn build(
        all_fields: &[AnyField],
        pages: &[SettingsPage],
        surfaces: &[SettingsSurface],
    ) -> Self {
        Self(build_index(all_fields, pages, surfaces))
    }
}

/// Score+sort the index against `query` (fuzzy mode, task step 2), grouping
/// hits by page — pages ordered by their best-scoring hit, fields within a
/// page ordered by score — and capped at `limit` (task Notizen:
/// "kein Performance-Thema" — the index is ~200 entries, a full linear scan
/// per query is fine).
pub(crate) fn search(index: &SearchIndex, query: &str, limit: usize) -> Vec<SearchRow> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(i64, &SearchEntry)> = index
        .0
        .iter()
        .filter_map(|e| match_score(SearchMode::Fuzzy, &e.haystack, query).map(|s| (s, e)))
        .collect();
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));

    // Group by page, preserving the order pages first appear in the
    // score-sorted list (i.e. the page of the single best match sorts first),
    // while fields within a group keep their relative score order.
    let mut order: Vec<usize> = Vec::new();
    let mut groups: std::collections::HashMap<usize, Vec<&SearchEntry>> =
        std::collections::HashMap::new();
    for (_, e) in &scored {
        groups.entry(e.page_index).or_insert_with(|| {
            order.push(e.page_index);
            Vec::new()
        });
        groups.get_mut(&e.page_index).unwrap().push(e);
    }

    let mut rows = Vec::new();
    'outer: for page_index in order {
        for e in &groups[&page_index] {
            rows.push(SearchRow {
                target: e.target,
                page_index: e.page_index,
                page_title: e.page_title,
            });
            if rows.len() >= limit {
                break 'outer;
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::all_fields;

    fn index() -> SearchIndex {
        let surfaces = [
            SettingsSurface::new(
                SettingsSurfaceId::new("app-themes"),
                "App theme",
                "Choose a color theme and preview its variants.",
                "Choose theme…",
                crate::services::SettingsSurfacePage::section("appearance", "Appearance", "Theme"),
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
        SearchIndex::build(&all_fields(), &crate::pages::pages(&surfaces), &surfaces)
    }

    /// Query `font` finds fields from multiple categories (Appearance,
    /// Terminal, and Editor), each grouped under its own category title.
    #[test]
    fn font_query_finds_multiple_categories() {
        let idx = index();
        let rows = search(&idx, "font", 50);
        assert!(rows.iter().any(|r| r.page_title == "Appearance"));
        assert!(rows.iter().any(|r| r.page_title == "Terminal"));
        assert!(rows.iter().any(|r| r.page_title == "Editor"));
        let fields = all_fields();
        let terminal = fields
            .iter()
            .position(|field| field.json_path == "terminal.terminalFontSize")
            .unwrap();
        let editor = fields
            .iter()
            .position(|field| field.json_path == "editor.editorFontSize")
            .unwrap();
        assert!(rows
            .iter()
            .any(|r| r.target == SearchTarget::Field(terminal)));
        assert!(rows.iter().any(|r| r.target == SearchTarget::Field(editor)));
    }

    /// An exact `json_path` query finds exactly that field.
    #[test]
    fn exact_json_path_finds_the_field() {
        let idx = index();
        let rows = search(&idx, "terminal.terminalFontSize", 50);
        let field_index = all_fields()
            .iter()
            .position(|field| field.json_path == "terminal.terminalFontSize")
            .unwrap();
        assert!(rows
            .iter()
            .any(|r| r.target == SearchTarget::Field(field_index)));
    }

    #[test]
    fn owner_surface_search_finds_keymap_and_theme_actions() {
        let idx = index();
        let keymap = search(&idx, "keyboard shortcuts", 50);
        assert!(keymap.iter().any(|row| {
            row.target == SearchTarget::OwnerSurface(SettingsSurfaceId::new("keymap"))
                && row.page_title == "Keymap"
        }));

        let themes = search(&idx, "choose theme", 50);
        assert!(themes.iter().any(|row| {
            row.target == SearchTarget::OwnerSurface(SettingsSurfaceId::new("app-themes"))
        }));
    }

    /// Empty query yields no results (category-view fallback is the caller's
    /// job — this module just reports "nothing to show").
    #[test]
    fn empty_query_yields_no_rows() {
        let idx = index();
        assert!(search(&idx, "", 50).is_empty());
        assert!(search(&idx, "   ", 50).is_empty());
    }

    /// Results are capped at `limit`.
    #[test]
    fn results_are_capped_at_limit() {
        let idx = index();
        // A single-letter fuzzy query matches almost everything.
        let rows = search(&idx, "e", 5);
        assert!(rows.len() <= 5);
    }
}
