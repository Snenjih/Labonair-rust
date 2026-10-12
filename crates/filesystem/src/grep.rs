use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use globset::{Glob, GlobSet, GlobSetBuilder};
use grep_regex::RegexMatcherBuilder;
use grep_searcher::sinks::UTF8;
use grep_searcher::{BinaryDetection, SearcherBuilder};
use ignore::{WalkBuilder, WalkState};
use serde::Serialize;

const FILE_SIZE_CAP: u64 = 5 * 1024 * 1024;
const DEFAULT_MAX_RESULTS: usize = 200;
const HARD_MAX_RESULTS: usize = 2000;

#[derive(Serialize)]
pub struct GrepHit {
    pub path: String,
    pub rel: String,
    pub line: u64,
    pub text: String,
}

#[derive(Serialize)]
pub struct GrepResponse {
    pub hits: Vec<GrepHit>,
    pub truncated: bool,
    pub files_scanned: usize,
}

fn build_globset(patterns: &[String]) -> Result<Option<GlobSet>, String> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        let g = Glob::new(p).map_err(|e| format!("bad glob {p:?}: {e}"))?;
        b.add(g);
    }
    let set = b.build().map_err(|e| format!("globset build: {e}"))?;
    Ok(Some(set))
}

pub fn fs_grep(
    pattern: String,
    root: String,
    glob: Option<Vec<String>>,
    case_insensitive: Option<bool>,
    max_results: Option<usize>,
) -> Result<GrepResponse, String> {
    fs_grep_with_ignored(pattern, root, glob, case_insensitive, max_results, false)
}

/// Search files while optionally traversing paths excluded by ignore rules.
/// Hidden-file handling remains unchanged; this switch only controls ignore files.
pub fn fs_grep_with_ignored(
    pattern: String,
    root: String,
    glob: Option<Vec<String>>,
    case_insensitive: Option<bool>,
    max_results: Option<usize>,
    include_ignored: bool,
) -> Result<GrepResponse, String> {
    if pattern.is_empty() {
        return Err("empty pattern".into());
    }
    let root_path = super::paths::expand_home(&root)?;
    if !root_path.is_dir() {
        return Err(format!("not a directory: {root}"));
    }
    let cap = max_results
        .unwrap_or(DEFAULT_MAX_RESULTS)
        .clamp(1, HARD_MAX_RESULTS);

    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(case_insensitive.unwrap_or(false))
        .line_terminator(Some(b'\n'))
        .build(&pattern)
        .map_err(|e| format!("bad regex: {e}"))?;

    let globs = build_globset(glob.as_deref().unwrap_or(&[]))?;

    let walker = WalkBuilder::new(&root_path)
        .hidden(true)
        .git_ignore(!include_ignored)
        .git_global(!include_ignored)
        .git_exclude(!include_ignored)
        .ignore(!include_ignored)
        .parents(true)
        .follow_links(false)
        .build_parallel();

    let hits: Arc<std::sync::Mutex<Vec<GrepHit>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let scanned = Arc::new(AtomicUsize::new(0));
    let truncated = Arc::new(std::sync::atomic::AtomicBool::new(false));

    walker.run(|| {
        let hits = hits.clone();
        let scanned = scanned.clone();
        let truncated = truncated.clone();
        let matcher = matcher.clone();
        let globs = globs.clone();
        let root_path = root_path.clone();
        Box::new(move |entry| {
            if truncated.load(Ordering::Relaxed) {
                return WalkState::Quit;
            }
            let Ok(dent) = entry else {
                return WalkState::Continue;
            };
            if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) {
                return WalkState::Continue;
            }
            let path = dent.path();
            let rel = match path.strip_prefix(&root_path) {
                Ok(r) => r.to_string_lossy().into_owned(),
                Err(_) => return WalkState::Continue,
            };
            if let Some(set) = globs.as_ref() {
                if !set.is_match(&rel) {
                    return WalkState::Continue;
                }
            }
            if let Ok(meta) = std::fs::metadata(path) {
                if meta.len() > FILE_SIZE_CAP {
                    return WalkState::Continue;
                }
            }

            scanned.fetch_add(1, Ordering::Relaxed);

            let abs = path.to_string_lossy().into_owned();
            let rel_clone = rel.clone();
            let hits_ref = hits.clone();
            let truncated_ref = truncated.clone();
            let mut searcher = SearcherBuilder::new()
                .binary_detection(BinaryDetection::quit(b'\x00'))
                .line_number(true)
                .build();

            let _ = searcher.search_path(
                &matcher,
                path,
                UTF8(|line_num, text| {
                    let line_text = text.trim_end_matches('\n').to_string();
                    let mut guard = hits_ref.lock().unwrap();
                    if guard.len() >= cap {
                        truncated_ref.store(true, Ordering::Relaxed);
                        return Ok(false);
                    }
                    guard.push(GrepHit {
                        path: abs.clone(),
                        rel: rel_clone.clone(),
                        line: line_num,
                        text: line_text,
                    });
                    Ok(true)
                }),
            );
            WalkState::Continue
        })
    });

    let final_hits = Arc::try_unwrap(hits)
        .map(|m| m.into_inner().unwrap())
        .unwrap_or_default();

    Ok(GrepResponse {
        hits: final_hits,
        truncated: truncated.load(Ordering::Relaxed),
        files_scanned: scanned.load(Ordering::Relaxed),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignored_files_are_included_only_when_requested() {
        let root =
            std::env::temp_dir().join(format!("labonair-grep-ignored-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create test root");
        std::fs::create_dir(root.join(".git")).expect("create test git directory");
        std::fs::write(root.join(".gitignore"), "ignored.txt\n").expect("write gitignore");
        std::fs::write(root.join("ignored.txt"), "needle\n").expect("write ignored file");

        let run = |include_ignored| {
            fs_grep_with_ignored(
                "needle".into(),
                root.to_string_lossy().into_owned(),
                None,
                None,
                None,
                include_ignored,
            )
            .expect("grep succeeds")
        };
        assert!(run(false).hits.is_empty());
        assert!(run(true).hits.iter().any(|hit| hit.rel == "ignored.txt"));

        std::fs::remove_dir_all(root).expect("remove test root");
    }
}

#[derive(Serialize)]
pub struct GlobHit {
    pub path: String,
    pub rel: String,
}

#[derive(Serialize)]
pub struct GlobResponse {
    pub hits: Vec<GlobHit>,
    pub truncated: bool,
}

pub fn fs_glob(
    pattern: String,
    root: String,
    max_results: Option<usize>,
) -> Result<GlobResponse, String> {
    if pattern.is_empty() {
        return Err("empty pattern".into());
    }
    let root_path = super::paths::expand_home(&root)?;
    if !root_path.is_dir() {
        return Err(format!("not a directory: {root}"));
    }
    let cap = max_results.unwrap_or(500).clamp(1, HARD_MAX_RESULTS);

    let glob = Glob::new(&pattern).map_err(|e| format!("bad glob: {e}"))?;
    let mut gb = GlobSetBuilder::new();
    gb.add(glob);
    let set = gb.build().map_err(|e| format!("globset build: {e}"))?;

    let walker = WalkBuilder::new(&root_path)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        .parents(true)
        .follow_links(false)
        .build_parallel();

    let hits: Arc<std::sync::Mutex<Vec<GlobHit>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let truncated = Arc::new(std::sync::atomic::AtomicBool::new(false));

    walker.run(|| {
        let hits = hits.clone();
        let truncated = truncated.clone();
        let set = set.clone();
        let root_path = root_path.clone();
        Box::new(move |entry| {
            if truncated.load(Ordering::Relaxed) {
                return WalkState::Quit;
            }
            let Ok(dent) = entry else {
                return WalkState::Continue;
            };
            if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) {
                return WalkState::Continue;
            }
            let path = dent.path();
            let rel = match path.strip_prefix(&root_path) {
                Ok(r) => r.to_string_lossy().into_owned(),
                Err(_) => return WalkState::Continue,
            };
            if !set.is_match(&rel) {
                return WalkState::Continue;
            }
            let mut guard = hits.lock().unwrap();
            if guard.len() >= cap {
                truncated.store(true, Ordering::Relaxed);
                return WalkState::Quit;
            }
            guard.push(GlobHit {
                path: path.to_string_lossy().into_owned(),
                rel,
            });
            WalkState::Continue
        })
    });

    let final_hits = Arc::try_unwrap(hits)
        .map(|m| m.into_inner().unwrap())
        .unwrap_or_default();

    Ok(GlobResponse {
        hits: final_hits,
        truncated: truncated.load(Ordering::Relaxed),
    })
}
