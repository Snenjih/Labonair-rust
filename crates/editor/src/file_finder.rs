//! GPUI-free contract for the editor's workspace file finder.
//!
//! The editor owns matching, ranking, result limits, and the generation-bound
//! session. Workspace supplies local filesystem candidates through its narrow
//! adapter and the UI only presents the resulting immutable snapshot.

use std::path::PathBuf;

/// The default number of ranked rows exposed by the file finder.
pub const DEFAULT_FILE_FINDER_MAX_RESULTS: usize = 100;
/// The upper bound accepted by [`FileFinderQuery`].
pub const MAX_FILE_FINDER_RESULTS: usize = 500;

/// User input and the result cap for one finder request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFinderQuery {
    pub text: String,
    pub max_results: usize,
}

impl FileFinderQuery {
    pub fn new(text: impl Into<String>, max_results: usize) -> Self {
        Self {
            text: text.into(),
            max_results: max_results.clamp(1, MAX_FILE_FINDER_RESULTS),
        }
    }

    /// Smart-case matching: lowercase queries match without regard to case;
    /// an uppercase character makes the query case-sensitive.
    pub fn is_case_sensitive(&self) -> bool {
        self.text.chars().any(char::is_uppercase)
    }
}

impl Default for FileFinderQuery {
    fn default() -> Self {
        Self::new(String::new(), DEFAULT_FILE_FINDER_MAX_RESULTS)
    }
}

/// Metadata for one validated local file candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFinderCandidate {
    pub path: PathBuf,
    pub relative_path: String,
    pub basename: String,
}

impl FileFinderCandidate {
    pub fn new(path: PathBuf, relative_path: impl Into<String>) -> Self {
        let relative_path = relative_path.into();
        let basename = path
            .file_name()
            .and_then(|name| name.to_str())
            .or_else(|| relative_path.rsplit(['/', '\\']).next())
            .unwrap_or_default()
            .to_string();
        Self {
            path,
            relative_path,
            basename,
        }
    }
}

/// Ranked, immutable output for one finder generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFinderResult {
    pub generation: u64,
    pub candidates: Vec<FileFinderCandidate>,
    pub truncated: bool,
}

impl FileFinderResult {
    pub fn rank(
        generation: u64,
        query: &FileFinderQuery,
        candidates: impl IntoIterator<Item = FileFinderCandidate>,
        truncated: bool,
    ) -> Self {
        let mut ranked = candidates
            .into_iter()
            .filter_map(|candidate| {
                score_candidate(query, &candidate).map(|score| (score, candidate))
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|(left_score, left), (right_score, right)| {
            right_score
                .cmp(left_score)
                .then_with(|| left.relative_path.cmp(&right.relative_path))
        });
        ranked.truncate(query.max_results);
        Self {
            generation,
            candidates: ranked.into_iter().map(|(_, candidate)| candidate).collect(),
            truncated,
        }
    }
}

/// Presentation state for a finder session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileFinderStatus {
    Idle,
    Loading,
    Ready,
    Empty,
    Error(String),
}

/// Immutable state read by the finder view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFinderSnapshot {
    pub query: FileFinderQuery,
    pub generation: u64,
    pub status: FileFinderStatus,
    pub candidates: Vec<FileFinderCandidate>,
    pub selected: Option<usize>,
    pub truncated: bool,
}

/// Generation-safe state machine for one open file-finder modal.
#[derive(Clone, Debug)]
pub struct FileFinderSession {
    snapshot: FileFinderSnapshot,
}

impl FileFinderSession {
    pub fn new(max_results: usize) -> Self {
        Self {
            snapshot: FileFinderSnapshot {
                query: FileFinderQuery::new(String::new(), max_results),
                generation: 0,
                status: FileFinderStatus::Idle,
                candidates: Vec::new(),
                selected: None,
                truncated: false,
            },
        }
    }

    pub fn snapshot(&self) -> &FileFinderSnapshot {
        &self.snapshot
    }

    pub fn query(&self) -> &FileFinderQuery {
        &self.snapshot.query
    }

    pub fn generation(&self) -> u64 {
        self.snapshot.generation
    }

    /// Start a new request and invalidate every result from an older query.
    pub fn begin_query(&mut self, text: impl Into<String>) -> u64 {
        self.snapshot.generation = self.snapshot.generation.wrapping_add(1);
        self.snapshot.query = FileFinderQuery::new(text, self.snapshot.query.max_results);
        self.snapshot.status = FileFinderStatus::Loading;
        self.snapshot.candidates.clear();
        self.snapshot.selected = None;
        self.snapshot.truncated = false;
        self.snapshot.generation
    }

    /// Invalidate in-flight work when the modal closes.
    pub fn cancel(&mut self) -> u64 {
        self.snapshot.generation = self.snapshot.generation.wrapping_add(1);
        self.snapshot.status = FileFinderStatus::Idle;
        self.snapshot.candidates.clear();
        self.snapshot.selected = None;
        self.snapshot.truncated = false;
        self.snapshot.generation
    }

    /// Accept only the current generation. Returns `false` for stale results.
    pub fn accept(&mut self, result: FileFinderResult) -> bool {
        if result.generation != self.snapshot.generation {
            return false;
        }
        self.snapshot.status = if result.candidates.is_empty() {
            FileFinderStatus::Empty
        } else {
            FileFinderStatus::Ready
        };
        self.snapshot.selected = (!result.candidates.is_empty()).then_some(0);
        self.snapshot.candidates = result.candidates;
        self.snapshot.truncated = result.truncated;
        true
    }

    /// Accept an error only for the current generation.
    pub fn fail(&mut self, generation: u64, message: impl Into<String>) -> bool {
        if generation != self.snapshot.generation {
            return false;
        }
        self.snapshot.status = FileFinderStatus::Error(message.into());
        self.snapshot.candidates.clear();
        self.snapshot.selected = None;
        self.snapshot.truncated = false;
        true
    }

    /// Move the selected row with wrapping, returning its new index.
    pub fn move_selection(&mut self, delta: isize) -> Option<usize> {
        let len = self.snapshot.candidates.len();
        if len == 0 {
            return None;
        }
        let current = self.snapshot.selected.unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(len as isize) as usize;
        self.snapshot.selected = Some(next);
        Some(next)
    }

    pub fn selected_candidate(&self) -> Option<&FileFinderCandidate> {
        self.snapshot
            .selected
            .and_then(|index| self.snapshot.candidates.get(index))
    }
}

fn score_candidate(query: &FileFinderQuery, candidate: &FileFinderCandidate) -> Option<i64> {
    let basename = subsequence_score(&query.text, &candidate.basename, query.is_case_sensitive());
    let path = subsequence_score(
        &query.text,
        &candidate.relative_path,
        query.is_case_sensitive(),
    );
    match (basename, path) {
        (None, None) => None,
        (Some(base), _) => Some(base + 10_000),
        (None, Some(path)) => Some(path),
    }
}

fn subsequence_score(query: &str, haystack: &str, case_sensitive: bool) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let query_chars = query.chars().collect::<Vec<_>>();
    let haystack_chars = haystack.chars().collect::<Vec<_>>();
    let mut haystack_index = 0;
    let mut previous_match = None;
    let mut score = 0i64;

    for query_char in query_chars {
        let mut matched = None;
        while haystack_index < haystack_chars.len() {
            let candidate_char = haystack_chars[haystack_index];
            let equal = if case_sensitive {
                candidate_char == query_char
            } else {
                same_case_fold(candidate_char, query_char)
            };
            if equal {
                matched = Some((haystack_index, candidate_char == query_char));
                haystack_index += 1;
                break;
            }
            score -= 2;
            haystack_index += 1;
        }
        let (index, exact_case) = matched?;
        if previous_match == Some(index.saturating_sub(1)) {
            score += 35;
        }
        if index == 0 || is_path_boundary(haystack_chars.get(index.saturating_sub(1))) {
            score += 30;
        }
        if exact_case {
            score += 4;
        }
        score -= index as i64;
        previous_match = Some(index);
    }
    Some(score)
}

fn same_case_fold(left: char, right: char) -> bool {
    left.to_lowercase().eq(right.to_lowercase())
}

fn is_path_boundary(previous: Option<&char>) -> bool {
    previous.is_some_and(|character| matches!(character, '/' | '\\' | '_' | '-' | '.' | ' '))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(path: &str) -> FileFinderCandidate {
        FileFinderCandidate::new(PathBuf::from(path), path)
    }

    fn query(text: &str) -> FileFinderQuery {
        FileFinderQuery::new(text, 10)
    }

    #[test]
    fn basename_matches_beat_directory_only_matches() {
        let result = FileFinderResult::rank(
            1,
            &query("view"),
            [
                candidate("src/view.rs"),
                candidate("view/components/index.ts"),
            ],
            false,
        );
        assert_eq!(result.candidates[0].relative_path, "src/view.rs");
    }

    #[test]
    fn ranking_is_case_aware_and_uses_path_tie_breaks() {
        let result = FileFinderResult::rank(
            1,
            &query("App"),
            [candidate("src/apple.rs"), candidate("src/App.rs")],
            false,
        );
        assert_eq!(result.candidates[0].relative_path, "src/App.rs");

        let ties =
            FileFinderResult::rank(1, &query(""), [candidate("z.rs"), candidate("a.rs")], false);
        assert_eq!(ties.candidates[0].relative_path, "a.rs");
    }

    #[test]
    fn empty_query_matches_every_candidate_and_respects_cap() {
        let result = FileFinderResult::rank(
            1,
            &FileFinderQuery::new("", 1),
            [candidate("b.rs"), candidate("a.rs")],
            true,
        );
        assert_eq!(result.candidates.len(), 1);
        assert!(result.truncated);
    }

    #[test]
    fn no_match_produces_empty_session_state() {
        let mut session = FileFinderSession::new(10);
        let generation = session.begin_query("xyz");
        let result = FileFinderResult::rank(
            generation,
            session.query(),
            [candidate("src/lib.rs")],
            false,
        );
        assert!(session.accept(result));
        assert_eq!(session.snapshot().status, FileFinderStatus::Empty);
        assert_eq!(session.selected_candidate(), None);
    }

    #[test]
    fn stale_results_and_cancel_are_ignored() {
        let mut session = FileFinderSession::new(10);
        let stale_generation = session.begin_query("old");
        let current_generation = session.begin_query("new");
        let stale = FileFinderResult::rank(
            stale_generation,
            session.query(),
            [candidate("old.rs")],
            false,
        );
        assert!(!session.accept(stale));
        assert_eq!(session.snapshot().status, FileFinderStatus::Loading);

        session.cancel();
        let canceled = FileFinderResult::rank(
            current_generation,
            session.query(),
            [candidate("new.rs")],
            false,
        );
        assert!(!session.accept(canceled));
        assert_eq!(session.snapshot().status, FileFinderStatus::Idle);
    }

    #[test]
    fn selection_movement_wraps() {
        let mut session = FileFinderSession::new(10);
        let generation = session.begin_query("");
        session.accept(FileFinderResult::rank(
            generation,
            session.query(),
            [candidate("a.rs"), candidate("b.rs")],
            false,
        ));
        assert_eq!(session.move_selection(-1), Some(1));
        assert_eq!(session.move_selection(1), Some(0));
    }
}
