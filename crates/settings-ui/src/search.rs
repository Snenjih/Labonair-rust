//! Global settings search (T19-007), replacing the T19-004 in-page substring
//! filter (`panes/generic.rs`'s old `render_global_search`, which rendered
//! matched fields inline). This module is pure data: an index built once
//! (task Warnung: never rebuilt per keystroke) over every generated field's
//! title + description + `json_path`, plus one hand-curated entry per
//! `AreaKind::Custom` pane (main page and sub-pages), and a scorer on top of
//! the shared fuzzy matcher (`labonair_command_palette::fuzzy`, already used
//! by the command palette / `@`-file picker). `crate::view`/`crate::panes`
//! own the rendering + keyboard navigation on top of this.

use labonair_command_palette::{match_score, SearchMode};
use labonair_settings_content::areas::{AreaKind, AREAS};

use crate::pages::SettingsPage;
use crate::schema::AnyField;

/// What a search hit navigates to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchTarget {
    /// Indexes [`crate::view::SettingsView::all_fields`].
    Field(usize),
    /// A custom pane: the area index into `AREAS`/`SettingsView::pages`, and
    /// the sub-page index (`None` = the area's main page).
    Pane {
        area_index: usize,
        subpage_index: Option<usize>,
    },
}

/// One indexed, searchable entry.
struct SearchEntry {
    target: SearchTarget,
    area_index: usize,
    area_title: &'static str,
    title: &'static str,
    /// Shown small under the title in a result row (a field's `json_path`;
    /// empty for a pane).
    subtitle: &'static str,
    haystack: String,
}

/// One scored, render-ready hit — `Copy` so it can be cached in
/// `SettingsView` and iterated without holding a borrow of the index.
#[derive(Clone, Copy)]
pub(crate) struct SearchRow {
    pub(crate) target: SearchTarget,
    pub(crate) area_title: &'static str,
    pub(crate) title: &'static str,
    pub(crate) subtitle: &'static str,
}

/// Hand-curated keywords for a `AreaKind::Custom` pane (task step 1: "je
/// Pane ein Grob-Eintrag + optional handgepflegte Stichworte", with the
/// task's own example — "Keymap, Shortcut, Tastenkürzel" for Shortcuts).
fn pane_keywords(area_key: &str, subpage_slug: Option<&str>) -> &'static str {
    match (area_key, subpage_slug) {
        ("themes", _) => "theme color scheme appearance palette variant",
        ("hosts", _) => "host ssh server connection saved hosts",
        ("shortcuts", _) => "keymap shortcut keybinding tastenkürzel hotkey",
        ("mcp", _) => "mcp agent bridge model context protocol",
        ("personalization", _) => "personalization status bar layout panel toggle",
        _ => "",
    }
}

/// Build the full search index: every [`AnyField`] plus one entry per
/// `AreaKind::Custom` page (main page + sub-pages). Rebuild only when the
/// settings window opens / its schema changes (task Warnung) — never on
/// every keystroke.
fn build_index(all_fields: &[AnyField], pages: &[SettingsPage]) -> Vec<SearchEntry> {
    let mut out = Vec::new();
    for (i, field) in all_fields.iter().enumerate() {
        let Some(area_index) = AREAS.iter().position(|a| a.target_module == field.area()) else {
            continue;
        };
        out.push(SearchEntry {
            target: SearchTarget::Field(i),
            area_index,
            area_title: AREAS[area_index].title,
            title: field.meta.title,
            subtitle: field.json_path,
            haystack: format!(
                "{} {} {}",
                field.meta.title, field.meta.description, field.json_path
            ),
        });
    }
    for (area_index, page) in pages.iter().enumerate() {
        let area = page.area;
        if area.kind != AreaKind::Custom {
            continue;
        }
        let mut push_pane =
            |title: &'static str, subpage_index: Option<usize>, slug: Option<&str>| {
                let keywords = pane_keywords(area.key, slug);
                out.push(SearchEntry {
                    target: SearchTarget::Pane {
                        area_index,
                        subpage_index,
                    },
                    area_index,
                    area_title: area.title,
                    title,
                    subtitle: "",
                    haystack: format!("{title} {keywords}"),
                });
            };
        push_pane(area.title, None, None);
        for (sp_i, sp) in page.sub_pages.iter().enumerate() {
            push_pane(sp.title, Some(sp_i), Some(sp.slug));
        }
    }
    out
}

/// A built index, opaque to callers beyond [`search`].
pub(crate) struct SearchIndex(Vec<SearchEntry>);

impl SearchIndex {
    pub(crate) fn build(all_fields: &[AnyField], pages: &[SettingsPage]) -> Self {
        Self(build_index(all_fields, pages))
    }
}

/// Score+sort the index against `query` (fuzzy mode, task step 2), grouping
/// hits by category — categories ordered by their best-scoring hit, fields
/// within a category ordered by score — and capped at `limit` (task Notizen:
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

    // Group by area, preserving the order areas first appear in the
    // score-sorted list (i.e. the category of the single best match sorts
    // first), fields within a group keep their relative score order.
    let mut order: Vec<usize> = Vec::new();
    let mut groups: std::collections::HashMap<usize, Vec<&SearchEntry>> =
        std::collections::HashMap::new();
    for (_, e) in &scored {
        groups.entry(e.area_index).or_insert_with(|| {
            order.push(e.area_index);
            Vec::new()
        });
        groups.get_mut(&e.area_index).unwrap().push(e);
    }

    let mut rows = Vec::new();
    'outer: for area_index in order {
        for e in &groups[&area_index] {
            rows.push(SearchRow {
                target: e.target,
                area_title: e.area_title,
                title: e.title,
                subtitle: e.subtitle,
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
    use crate::pages::pages;
    use crate::schema::all_fields;

    fn index() -> SearchIndex {
        SearchIndex::build(&all_fields(), &pages())
    }

    /// Query `cursor` finds fields from multiple categories (Terminal's
    /// cursor style/blink, Editor's cursor-position toggle), each grouped
    /// under its own category title (task Anweisung step 7 / Akzeptanz).
    #[test]
    fn cursor_query_finds_multiple_categories() {
        let idx = index();
        let rows = search(&idx, "cursor", 50);
        assert!(rows.iter().any(|r| r.area_title == "Terminal"));
        assert!(rows.iter().any(|r| r.area_title == "Editor"));
        assert!(rows
            .iter()
            .any(|r| r.subtitle == "terminal.terminalCursorStyle"));
        assert!(rows
            .iter()
            .any(|r| r.subtitle == "editor.editorShowCursorPosition"));
    }

    /// An exact `json_path` query finds exactly that field.
    #[test]
    fn exact_json_path_finds_the_field() {
        let idx = index();
        let rows = search(&idx, "terminal.terminalFontSize", 50);
        assert!(rows
            .iter()
            .any(|r| r.subtitle == "terminal.terminalFontSize"));
    }

    /// A query that only hits a custom pane's curated keyword list surfaces
    /// that pane as a result (task Anweisung step 7's "shortcut" example).
    #[test]
    fn keyword_query_finds_a_custom_pane() {
        let idx = index();
        let rows = search(&idx, "tastenkürzel", 50);
        assert!(rows.iter().any(|r| r.area_title == "Shortcuts"
            && matches!(
                r.target,
                SearchTarget::Pane {
                    subpage_index: None,
                    ..
                }
            )));
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
