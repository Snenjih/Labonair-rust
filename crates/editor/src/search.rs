//! Typed in-buffer find and replace.
//!
//! Matching operates on one immutable [`BufferSnapshot`]. Regex compilation
//! and byte-to-position conversion stay in the editor owner, while mutation
//! is returned as transaction-ready edits for [`crate::Document`].

use regex::RegexBuilder;
use serde::{Deserialize, Serialize};

use crate::buffer::{BufferSnapshot, Edit, Position, TextRange};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub smart_case: bool,
    pub whole_word: bool,
    pub regex: bool,
    pub multiline: bool,
    pub preserve_case: bool,
    pub wrap: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            smart_case: true,
            whole_word: false,
            regex: false,
            multiline: false,
            preserve_case: false,
            wrap: true,
        }
    }
}

/// The three case modes exposed by the in-buffer search UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchCaseMode {
    Sensitive,
    Insensitive,
    Smart,
}

impl SearchOptions {
    pub fn case_mode(self) -> SearchCaseMode {
        if self.case_sensitive {
            SearchCaseMode::Sensitive
        } else if self.smart_case {
            SearchCaseMode::Smart
        } else {
            SearchCaseMode::Insensitive
        }
    }

    pub fn with_case_mode(mut self, mode: SearchCaseMode) -> Self {
        (self.case_sensitive, self.smart_case) = match mode {
            SearchCaseMode::Sensitive => (true, false),
            SearchCaseMode::Insensitive => (false, false),
            SearchCaseMode::Smart => (false, true),
        };
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    // These fields remain flat so version-2 Editor session payloads retain
    // their shape. SearchOptions is the typed construction boundary.
    pub case_sensitive: bool,
    #[serde(default)]
    pub smart_case: bool,
    pub whole_word: bool,
    pub regex: bool,
    pub multiline: bool,
    pub preserve_case: bool,
    pub wrap: bool,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self::new(String::new(), SearchOptions::default())
    }
}

impl SearchQuery {
    pub fn new(text: impl Into<String>, options: SearchOptions) -> Self {
        Self {
            text: text.into(),
            case_sensitive: options.case_sensitive,
            smart_case: options.smart_case,
            whole_word: options.whole_word,
            regex: options.regex,
            multiline: options.multiline,
            preserve_case: options.preserve_case,
            wrap: options.wrap,
        }
    }

    pub fn options(&self) -> SearchOptions {
        SearchOptions {
            case_sensitive: self.case_sensitive,
            smart_case: self.smart_case,
            whole_word: self.whole_word,
            regex: self.regex,
            multiline: self.multiline,
            preserve_case: self.preserve_case,
            wrap: self.wrap,
        }
    }

    pub fn case_mode(&self) -> SearchCaseMode {
        self.options().case_mode()
    }

    pub fn case_sensitive_for_matching(&self) -> bool {
        self.case_sensitive || (self.smart_case && self.text.chars().any(char::is_uppercase))
    }
}

/// Options that the filesystem-backed project search can apply safely.
///
/// Multiline matching is intentionally not part of this contract: the
/// filesystem provider is line-oriented and must not pretend to support a
/// cross-file multiline mode it cannot implement consistently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectSearchOptions {
    pub case_sensitive: bool,
    pub smart_case: bool,
    pub whole_word: bool,
    pub regex: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    CurrentFile,
    Project,
}

impl Default for ProjectSearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            smart_case: true,
            whole_word: false,
            regex: false,
        }
    }
}

impl ProjectSearchOptions {
    pub fn from_search_options(options: SearchOptions) -> Self {
        Self {
            case_sensitive: options.case_sensitive,
            smart_case: options.smart_case,
            whole_word: options.whole_word,
            regex: options.regex,
        }
    }

    pub fn case_sensitive_for_matching(self, text: &str) -> bool {
        self.case_sensitive || (self.smart_case && text.chars().any(char::is_uppercase))
    }
}

/// A project-wide search request. `generation` is allocated by the Editor
/// owner and is the only identity a provider needs to echo back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSearchQuery {
    pub text: String,
    pub options: ProjectSearchOptions,
    pub max_results: usize,
}

impl ProjectSearchQuery {
    pub const DEFAULT_MAX_RESULTS: usize = 200;
    pub const MAX_RESULTS: usize = 2_000;

    pub fn new(text: impl Into<String>, options: ProjectSearchOptions, max_results: usize) -> Self {
        Self {
            text: text.into(),
            options,
            max_results: max_results.clamp(1, Self::MAX_RESULTS),
        }
    }

    /// Translate the Editor query to the regex accepted by the existing
    /// filesystem grep provider. Literal and whole-word semantics are encoded
    /// here so the adapter does not grow a second matcher.
    pub fn backend_pattern(&self) -> Result<String, SearchError> {
        if self.text.is_empty() {
            return Err(SearchError {
                message: "empty pattern".to_string(),
            });
        }
        let pattern = if self.options.regex {
            self.text.clone()
        } else {
            regex::escape(&self.text)
        };
        let pattern = if self.options.whole_word {
            format!(r"\b(?:{pattern})\b")
        } else {
            pattern
        };
        RegexBuilder::new(&pattern)
            .case_insensitive(!self.options.case_sensitive_for_matching(&self.text))
            .multi_line(false)
            .build()
            .map_err(|error| SearchError {
                message: format!("invalid project search pattern: {error}"),
            })?;
        Ok(pattern)
    }

    /// Return the first match's character columns for a provider result line.
    /// Filesystem search remains the only operation that walks files; this
    /// method only projects already-returned line text into Editor coordinates.
    pub fn first_match_columns(&self, line: &str) -> Result<Option<(usize, usize)>, SearchError> {
        let pattern = self.backend_pattern()?;
        let regex = RegexBuilder::new(&pattern)
            .case_insensitive(!self.options.case_sensitive_for_matching(&self.text))
            .multi_line(false)
            .build()
            .map_err(|error| SearchError {
                message: format!("invalid project search pattern: {error}"),
            })?;
        Ok(regex.find(line).map(|matched| {
            (
                line[..matched.start()].chars().count(),
                line[..matched.end()].chars().count(),
            )
        }))
    }
}

/// One line returned by a project search provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSearchHit {
    pub path: String,
    pub relative_path: String,
    pub line: u64,
    pub text: String,
    pub start_column: usize,
    pub end_column: usize,
}

impl ProjectSearchHit {
    pub fn new(
        path: impl Into<String>,
        line: u64,
        text: impl Into<String>,
        start_column: usize,
        end_column: usize,
    ) -> Self {
        let path = path.into();
        Self {
            relative_path: path.clone(),
            path,
            line,
            text: text.into(),
            start_column,
            end_column,
        }
    }

    pub fn with_relative_path(
        path: impl Into<String>,
        relative_path: impl Into<String>,
        line: u64,
        text: impl Into<String>,
        start_column: usize,
        end_column: usize,
    ) -> Self {
        Self {
            path: path.into(),
            relative_path: relative_path.into(),
            line,
            text: text.into(),
            start_column,
            end_column,
        }
    }
}

/// Result envelope returned by an asynchronous project-search provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSearchResult {
    pub generation: u64,
    pub hits: Vec<ProjectSearchHit>,
    pub truncated: bool,
    pub files_scanned: usize,
}

impl ProjectSearchResult {
    pub fn new(
        generation: u64,
        hits: Vec<ProjectSearchHit>,
        truncated: bool,
        files_scanned: usize,
    ) -> Self {
        Self {
            generation,
            hits,
            truncated,
            files_scanned,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ProjectSearchStatus {
    #[default]
    Idle,
    Loading,
    Empty,
    Ready,
    Error(String),
}

/// Public Editor-owned snapshot used by the existing workspace overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSearchSnapshot {
    pub query: Option<ProjectSearchQuery>,
    pub generation: u64,
    pub status: ProjectSearchStatus,
    pub hits: Vec<ProjectSearchHit>,
    pub selected: Option<usize>,
    pub truncated: bool,
    pub files_scanned: usize,
}

/// Editor-owned lifecycle for asynchronous project search.
///
/// `begin` invalidates every previous request. `accept` and `fail` are
/// generation-checked, so dropping a provider task is an optimisation rather
/// than a correctness requirement.
#[derive(Debug, Default)]
pub struct ProjectSearchSession {
    query: Option<ProjectSearchQuery>,
    generation: u64,
    status: ProjectSearchStatus,
    hits: Vec<ProjectSearchHit>,
    selected: Option<usize>,
    truncated: bool,
    files_scanned: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSearchRequest {
    pub generation: u64,
    pub query: ProjectSearchQuery,
}

impl ProjectSearchSession {
    pub fn begin(&mut self, query: ProjectSearchQuery) -> ProjectSearchRequest {
        self.generation = self.generation.saturating_add(1);
        self.query = Some(query.clone());
        self.hits.clear();
        self.selected = None;
        self.truncated = false;
        self.files_scanned = 0;
        self.status = if query.text.is_empty() {
            ProjectSearchStatus::Empty
        } else {
            ProjectSearchStatus::Loading
        };
        ProjectSearchRequest {
            generation: self.generation,
            query,
        }
    }

    pub fn cancel(&mut self) {
        self.generation = self.generation.saturating_add(1);
        self.query = None;
        self.hits.clear();
        self.selected = None;
        self.truncated = false;
        self.files_scanned = 0;
        self.status = ProjectSearchStatus::Idle;
    }

    pub fn accept(&mut self, result: ProjectSearchResult) -> bool {
        if result.generation != self.generation {
            return false;
        }
        self.hits = result.hits;
        self.selected = (!self.hits.is_empty()).then_some(0);
        self.truncated = result.truncated;
        self.files_scanned = result.files_scanned;
        self.status = if self.hits.is_empty() {
            ProjectSearchStatus::Empty
        } else {
            ProjectSearchStatus::Ready
        };
        true
    }

    pub fn fail(&mut self, generation: u64, message: impl Into<String>) -> bool {
        if generation != self.generation {
            return false;
        }
        self.hits.clear();
        self.selected = None;
        self.truncated = false;
        self.files_scanned = 0;
        self.status = ProjectSearchStatus::Error(message.into());
        true
    }

    pub fn select(&mut self, index: usize) -> Option<ProjectSearchHit> {
        let index = index.min(self.hits.len().saturating_sub(1));
        let hit = self.hits.get(index)?.clone();
        self.selected = Some(index);
        Some(hit)
    }

    pub fn move_selection(&mut self, delta: isize) -> Option<ProjectSearchHit> {
        let len = self.hits.len();
        if len == 0 {
            return None;
        }
        let current = self.selected.unwrap_or(0) as isize;
        let index = (current + delta).rem_euclid(len as isize) as usize;
        self.select(index)
    }

    pub fn selected_hit(&self) -> Option<&ProjectSearchHit> {
        self.selected.and_then(|index| self.hits.get(index))
    }

    pub fn snapshot(&self) -> ProjectSearchSnapshot {
        ProjectSearchSnapshot {
            query: self.query.clone(),
            generation: self.generation,
            status: self.status.clone(),
            hits: self.hits.clone(),
            selected: self.selected,
            truncated: self.truncated,
            files_scanned: self.files_scanned,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub start: Position,
    pub end: Position,
}

impl Match {
    pub fn text_range(self, snapshot: &BufferSnapshot) -> TextRange {
        TextRange::new(
            snapshot.position_to_byte(self.start),
            snapshot.position_to_byte(self.end),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub matches: Vec<Match>,
    pub active: Option<usize>,
    pub revision: crate::Revision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchError {
    pub message: String,
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SearchError {}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Find all matches, returning inline-validation errors for invalid regexes.
pub fn find(snapshot: &BufferSnapshot, query: &SearchQuery) -> Result<SearchResult, SearchError> {
    if query.text.is_empty() {
        return Ok(SearchResult {
            matches: Vec::new(),
            active: None,
            revision: snapshot.revision(),
        });
    }
    let regex = compile(query)?;
    let text = snapshot.text();
    let matches = search_hits(&regex, &text, query, None)
        .into_iter()
        .map(|hit| Match {
            start: snapshot.byte_to_position(hit.range.start),
            end: snapshot.byte_to_position(hit.range.end),
        })
        .collect();
    Ok(SearchResult {
        matches,
        active: None,
        revision: snapshot.revision(),
    })
}

struct SearchHit {
    range: TextRange,
    matched_text: String,
    expanded_replacement: Option<String>,
}

fn compile(query: &SearchQuery) -> Result<regex::Regex, SearchError> {
    let pattern = if query.regex {
        query.text.clone()
    } else {
        regex::escape(&query.text)
    };
    RegexBuilder::new(&pattern)
        .case_insensitive(!query.case_sensitive_for_matching())
        .multi_line(query.multiline)
        .dot_matches_new_line(query.multiline)
        .build()
        .map_err(|error| SearchError {
            message: error.to_string(),
        })
}

fn search_hits(
    regex: &regex::Regex,
    text: &str,
    query: &SearchQuery,
    replacement: Option<&str>,
) -> Vec<SearchHit> {
    regex
        .captures_iter(text)
        .filter_map(|captures| {
            let matched = captures.get(0)?;
            if !query.multiline
                && (matched.as_str().contains('\n') || matched.as_str().contains('\r'))
            {
                return None;
            }
            if query.whole_word {
                let before = text[..matched.start()].chars().next_back();
                let after = text[matched.end()..].chars().next();
                if before.is_some_and(is_word_char) || after.is_some_and(is_word_char) {
                    return None;
                }
            }
            let expanded_replacement = replacement.map(|replacement| {
                let mut expanded = String::new();
                captures.expand(replacement, &mut expanded);
                expanded
            });
            Some(SearchHit {
                range: TextRange::new(matched.start(), matched.end()),
                matched_text: matched.as_str().to_string(),
                expanded_replacement,
            })
        })
        .collect()
}

/// Compatibility helper used by existing consumers. Invalid regex input is
/// represented as no matches; UI callers should use [`find`] for validation.
pub fn find_all(snapshot: &BufferSnapshot, query: &SearchQuery) -> Vec<Match> {
    find(snapshot, query)
        .map(|result| result.matches)
        .unwrap_or_default()
}

/// Index of the first match at or after `from`, optionally wrapping, if any.
pub fn next_match(matches: &[Match], from: Position, forward: bool) -> Option<usize> {
    next_match_with_wrap(matches, from, forward, true)
}

pub fn next_match_with_wrap(
    matches: &[Match],
    from: Position,
    forward: bool,
    wrap: bool,
) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    if forward {
        matches
            .iter()
            .position(|m| m.start >= from)
            .or_else(|| wrap.then_some(0))
    } else {
        matches
            .iter()
            .rposition(|m| m.start < from)
            .or_else(|| wrap.then_some(matches.len() - 1))
    }
}

/// Build one transaction-ready edit per match. The canonical document owner
/// applies these edits in one undoable transaction.
pub fn replace_all(snapshot: &BufferSnapshot, query: &SearchQuery, replacement: &str) -> Vec<Edit> {
    replace_all_checked(snapshot, query, replacement).unwrap_or_default()
}

/// Build the edit for one indexed match. The index is in document order and
/// is the same index returned in [`SearchResult::matches`].
pub fn replace_one(
    snapshot: &BufferSnapshot,
    query: &SearchQuery,
    replacement: &str,
    match_index: usize,
) -> Result<Option<Edit>, SearchError> {
    let regex = compile(query)?;
    let text = snapshot.text();
    let Some(hit) = search_hits(&regex, &text, query, Some(replacement))
        .into_iter()
        .nth(match_index)
    else {
        return Ok(None);
    };
    Ok(Some(Edit::new(
        hit.range,
        hit.replacement(replacement, query),
    )))
}

/// Build all replacement edits in ascending source order. The document owner
/// applies them as one transaction, so replacement ordering and undo history
/// stay deterministic even when replacement lengths differ.
pub fn replace_all_checked(
    snapshot: &BufferSnapshot,
    query: &SearchQuery,
    replacement: &str,
) -> Result<Vec<Edit>, SearchError> {
    let regex = compile(query)?;
    let text = snapshot.text();
    Ok(search_hits(&regex, &text, query, Some(replacement))
        .into_iter()
        .map(|hit| Edit::new(hit.range, hit.replacement(replacement, query)))
        .collect())
}

impl SearchHit {
    fn replacement(&self, replacement: &str, query: &SearchQuery) -> String {
        replacement_for_match(
            &self.matched_text,
            self.expanded_replacement.as_deref().unwrap_or(replacement),
            query.preserve_case,
        )
    }
}

fn replacement_for_match(matched: &str, replacement: &str, preserve_case: bool) -> String {
    if !preserve_case || replacement.is_empty() {
        return replacement.to_string();
    }
    if matched
        .chars()
        .all(|c| !c.is_alphabetic() || c.is_uppercase())
    {
        return replacement.chars().flat_map(char::to_uppercase).collect();
    }
    if matched.chars().next().is_some_and(|c| c.is_uppercase()) {
        let mut chars = replacement.chars();
        return chars
            .next()
            .map(|first| first.to_uppercase().chain(chars).collect())
            .unwrap_or_default();
    }
    replacement.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(text: &str) -> BufferSnapshot {
        crate::EditorBuffer::from_text(text).snapshot()
    }

    #[test]
    fn literal_options_and_unicode_crlf_are_position_safe() {
        let source = snapshot("Grüße\r\nGRÜße\r\nbar barrier");
        let query = SearchQuery::new(
            "grüße",
            SearchOptions {
                whole_word: true,
                ..Default::default()
            },
        );
        let result = find(&source, &query).unwrap();
        assert_eq!(result.matches.len(), 2);
        assert_eq!(result.matches[1].start, Position::new(1, 0));
        let bar = SearchQuery::new(
            "bar",
            SearchOptions {
                whole_word: true,
                ..Default::default()
            },
        );
        assert_eq!(find_all(&source, &bar).len(), 1);
    }

    #[test]
    fn regex_multiline_and_invalid_input_are_typed() {
        let source = snapshot("one\ntwo\nthree");
        let query = SearchQuery::new(
            "^t[^\\n]*$",
            SearchOptions {
                regex: true,
                multiline: true,
                ..Default::default()
            },
        );
        assert_eq!(find(&source, &query).unwrap().matches.len(), 2);
        let invalid = SearchQuery::new(
            "[",
            SearchOptions {
                regex: true,
                ..Default::default()
            },
        );
        assert!(find(&source, &invalid).is_err());
    }

    #[test]
    fn smart_case_and_multiline_dotall_are_explicit_options() {
        let source = snapshot("foo\nFOO\nstart middle\nend");
        let smart = SearchQuery::new("FOO", SearchOptions::default());
        assert_eq!(find(&source, &smart).unwrap().matches.len(), 1);

        let insensitive = SearchQuery::new(
            "FOO",
            SearchOptions::default().with_case_mode(SearchCaseMode::Insensitive),
        );
        assert_eq!(find(&source, &insensitive).unwrap().matches.len(), 2);

        let across_lines = SearchQuery::new(
            "start.*end",
            SearchOptions {
                regex: true,
                multiline: true,
                ..Default::default()
            },
        );
        let result = find(&source, &across_lines).unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].start, Position::new(2, 0));
        assert_eq!(result.matches[0].end, Position::new(3, 3));
    }

    #[test]
    fn replace_all_can_preserve_case() {
        let source = snapshot("Foo foo FOO");
        let query = SearchQuery::new(
            "foo",
            SearchOptions {
                preserve_case: true,
                ..Default::default()
            },
        );
        let edits = replace_all(&source, &query, "bar");
        assert_eq!(
            edits
                .iter()
                .map(|edit| edit.replacement.as_str())
                .collect::<Vec<_>>(),
            ["Bar", "bar", "BAR"]
        );
    }

    #[test]
    fn next_match_wraps_in_both_directions() {
        let matches = [
            Match {
                start: Position::new(0, 1),
                end: Position::new(0, 2),
            },
            Match {
                start: Position::new(1, 0),
                end: Position::new(1, 1),
            },
        ];
        assert_eq!(next_match(&matches, Position::new(2, 0), true), Some(0));
        assert_eq!(next_match(&matches, Position::new(0, 0), false), Some(1));
        assert_eq!(
            next_match_with_wrap(&matches, Position::new(2, 0), true, false),
            None
        );
    }

    #[test]
    fn replacement_is_ordered_replace_one_is_indexed_and_regex_captures_expand() {
        let source = snapshot("one two three");
        let query = SearchQuery::new(
            "\\w+",
            SearchOptions {
                regex: true,
                ..Default::default()
            },
        );
        let all = replace_all_checked(&source, &query, "x").unwrap();
        assert_eq!(
            all.iter().map(|edit| edit.range).collect::<Vec<_>>(),
            [
                TextRange::new(0, 3),
                TextRange::new(4, 7),
                TextRange::new(8, 13)
            ]
        );
        let one = replace_one(&source, &query, "$0!", 1).unwrap().unwrap();
        assert_eq!(one.range, TextRange::new(4, 7));
        assert_eq!(one.replacement, "two!");

        let captures = snapshot("left:right");
        let capture_query = SearchQuery::new(
            "(\\w+):(\\w+)",
            SearchOptions {
                regex: true,
                ..Default::default()
            },
        );
        let edits = replace_all_checked(&captures, &capture_query, "$2/$1").unwrap();
        assert_eq!(edits[0].replacement, "right/left");
    }

    #[test]
    fn invalid_regex_is_safe_for_replace() {
        let source = snapshot("text");
        let query = SearchQuery::new(
            "[",
            SearchOptions {
                regex: true,
                ..Default::default()
            },
        );
        assert!(replace_one(&source, &query, "x", 0).is_err());
        assert!(replace_all_checked(&source, &query, "x").is_err());
    }

    #[test]
    fn project_query_translates_literal_and_whole_word_options() {
        let query = ProjectSearchQuery::new(
            "a+b",
            ProjectSearchOptions {
                whole_word: true,
                ..Default::default()
            },
            10_000,
        );
        assert_eq!(query.max_results, ProjectSearchQuery::MAX_RESULTS);
        assert_eq!(query.backend_pattern().unwrap(), r"\b(?:a\+b)\b");
        assert_eq!(query.first_match_columns("x a+b y").unwrap(), Some((2, 5)));
    }

    #[test]
    fn project_session_rejects_stale_results_and_cancellation() {
        let mut session = ProjectSearchSession::default();
        let first = session.begin(ProjectSearchQuery::new(
            "first",
            Default::default(),
            ProjectSearchQuery::DEFAULT_MAX_RESULTS,
        ));
        let second = session.begin(ProjectSearchQuery::new(
            "second",
            Default::default(),
            ProjectSearchQuery::DEFAULT_MAX_RESULTS,
        ));
        assert!(!session.accept(ProjectSearchResult::new(
            first.generation,
            vec![ProjectSearchHit::new("first.rs", 1, "first", 0, 5)],
            false,
            1,
        )));
        assert!(session.accept(ProjectSearchResult::new(
            second.generation,
            vec![ProjectSearchHit::new("second.rs", 2, "second", 0, 6)],
            true,
            2,
        )));
        assert_eq!(session.snapshot().selected, Some(0));
        assert!(session.snapshot().truncated);
        session.cancel();
        assert!(!session.fail(second.generation, "late failure"));
        assert_eq!(session.snapshot().status, ProjectSearchStatus::Idle);
    }

    #[test]
    fn project_session_navigation_wraps_and_empty_results_are_local() {
        let mut session = ProjectSearchSession::default();
        let request = session.begin(ProjectSearchQuery::new(
            "x",
            Default::default(),
            ProjectSearchQuery::DEFAULT_MAX_RESULTS,
        ));
        assert!(session.accept(ProjectSearchResult::new(
            request.generation,
            vec![
                ProjectSearchHit::new("a", 1, "x", 0, 1),
                ProjectSearchHit::new("b", 2, "x", 0, 1),
            ],
            false,
            2,
        )));
        assert_eq!(session.move_selection(-1).unwrap().path, "b");
        let empty = session.begin(ProjectSearchQuery::new(
            "missing",
            Default::default(),
            ProjectSearchQuery::DEFAULT_MAX_RESULTS,
        ));
        assert!(session.accept(ProjectSearchResult::new(
            empty.generation,
            Vec::new(),
            false,
            4,
        )));
        assert_eq!(session.snapshot().status, ProjectSearchStatus::Empty);
    }
}
