//! Fuzzy / prefix / substring matcher (port of the reference `filter` in
//! `CommandPalette.tsx`, which switches on `commandPaletteSearchMode`).
//!
//! Shared by the palette itself, the `@`-file-picker in the AI composer and the
//! settings search — so it lives in its own module with no view dependencies.

/// The three search modes the reference footer cycles through.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SearchMode {
    Contains,
    StartsWith,
    Fuzzy,
}

impl SearchMode {
    /// Next mode in the cycle (matches the reference `cycleSearchMode` order).
    pub fn next(self) -> Self {
        match self {
            SearchMode::Contains => SearchMode::StartsWith,
            SearchMode::StartsWith => SearchMode::Fuzzy,
            SearchMode::Fuzzy => SearchMode::Contains,
        }
    }

    /// The persisted / displayed slug (matches `commandPaletteSearchMode`).
    pub fn label(self) -> &'static str {
        match self {
            SearchMode::Contains => "contains",
            SearchMode::StartsWith => "startsWith",
            SearchMode::Fuzzy => "fuzzy",
        }
    }

    /// Inverse of [`SearchMode::label`].
    pub fn from_label(s: &str) -> Option<Self> {
        Some(match s {
            "contains" => SearchMode::Contains,
            "startsWith" => SearchMode::StartsWith,
            "fuzzy" => SearchMode::Fuzzy,
            _ => return None,
        })
    }
}

/// Score `needle` against `haystack` under `mode`. `None` = no match.
/// Higher score = better; an empty needle matches everything with score `0`.
/// The port adds ranking (the reference `filter` is boolean) so results order
/// by relevance, mirroring `cmdk`'s built-in scoring.
pub fn match_score(mode: SearchMode, haystack: &str, needle: &str) -> Option<i64> {
    let h = haystack.to_lowercase();
    let n = needle.trim().to_lowercase();
    if n.is_empty() {
        return Some(0);
    }
    match mode {
        SearchMode::StartsWith => h.starts_with(&n).then_some(1_000),
        SearchMode::Contains => h.find(&n).map(|idx| 1_000 - idx as i64),
        SearchMode::Fuzzy => {
            let hb: Vec<char> = h.chars().collect();
            let nb: Vec<char> = n.chars().collect();
            let mut hi = 0;
            let mut score = 0i64;
            let mut last_hit: Option<usize> = None;
            for &nc in &nb {
                let mut found = false;
                while hi < hb.len() {
                    if hb[hi] == nc {
                        if last_hit == Some(hi.wrapping_sub(1)) {
                            score += 8; // consecutive-match bonus
                        }
                        if hi == 0 {
                            score += 6; // start-of-string bonus
                        }
                        last_hit = Some(hi);
                        hi += 1;
                        found = true;
                        break;
                    }
                    score -= 1; // gap penalty
                    hi += 1;
                }
                if !found {
                    return None;
                }
            }
            Some(score)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_score_modes() {
        // Empty needle always matches.
        assert!(match_score(SearchMode::Fuzzy, "anything", "").is_some());
        // StartsWith is anchored.
        assert!(match_score(SearchMode::StartsWith, "split pane right", "split").is_some());
        assert!(match_score(SearchMode::StartsWith, "split pane right", "pane").is_none());
        // Contains is a substring anywhere, earlier = higher score.
        let early = match_score(SearchMode::Contains, "split pane", "split").unwrap();
        let late = match_score(SearchMode::Contains, "split pane", "pane").unwrap();
        assert!(early > late);
        // Fuzzy matches a subsequence with gaps.
        assert!(match_score(SearchMode::Fuzzy, "split pane right", "spr").is_some());
        assert!(match_score(SearchMode::Fuzzy, "split pane right", "xyz").is_none());
        // Consecutive letters outrank scattered ones.
        let consec = match_score(SearchMode::Fuzzy, "format document", "format").unwrap();
        let scattered = match_score(SearchMode::Fuzzy, "focus source control mode", "format");
        assert!(scattered.is_none() || consec > scattered.unwrap());
    }

    #[test]
    fn search_mode_cycles_and_maps_labels() {
        assert_eq!(SearchMode::Contains.next(), SearchMode::StartsWith);
        assert_eq!(SearchMode::StartsWith.next(), SearchMode::Fuzzy);
        assert_eq!(SearchMode::Fuzzy.next(), SearchMode::Contains);
        for m in [
            SearchMode::Contains,
            SearchMode::StartsWith,
            SearchMode::Fuzzy,
        ] {
            assert_eq!(SearchMode::from_label(m.label()), Some(m));
        }
    }
}
