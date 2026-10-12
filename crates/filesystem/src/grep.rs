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

/// Path and ignore rules applied by [`fs_grep_with_options`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GrepOptions {
    pub include_globs: Vec<String>,
    pub exclude_globs: Vec<String>,
    /// Exact root-relative file paths. `Some(vec![])` intentionally matches
    /// nothing, which represents an empty open-file set.
    pub only_paths: Option<Vec<String>>,
    pub include_ignored: bool,
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
    fs_grep_with_options(
        pattern,
        root,
        case_insensitive,
        max_results,
        GrepOptions {
            include_globs: glob.unwrap_or_default(),
            ..GrepOptions::default()
        },
    )
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
    fs_grep_with_options(
        pattern,
        root,
        case_insensitive,
        max_results,
        GrepOptions {
            include_globs: glob.unwrap_or_default(),
            include_ignored,
            ..GrepOptions::default()
        },
    )
}

/// Search files with explicit path include/exclude globs and ignore handling.
/// Hidden-file handling remains unchanged; `include_ignored` only controls
/// ignore files.
pub fn fs_grep_with_filters(
    pattern: String,
    root: String,
    include_glob: Option<Vec<String>>,
    exclude_glob: Option<Vec<String>>,
    case_insensitive: Option<bool>,
    max_results: Option<usize>,
    include_ignored: bool,
) -> Result<GrepResponse, String> {
    fs_grep_with_options(
        pattern,
        root,
        case_insensitive,
        max_results,
        GrepOptions {
            include_globs: include_glob.unwrap_or_default(),
            exclude_globs: exclude_glob.unwrap_or_default(),
            include_ignored,
            ..GrepOptions::default()
        },
    )
}

/// Search files with typed path, open-file, and ignore rules.
pub fn fs_grep_with_options(
    pattern: String,
    root: String,
    case_insensitive: Option<bool>,
    max_results: Option<usize>,
    options: GrepOptions,
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
    if options.only_paths.as_ref().is_some_and(Vec::is_empty) {
        return Ok(GrepResponse {
            hits: Vec::new(),
            truncated: false,
            files_scanned: 0,
        });
    }

    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(case_insensitive.unwrap_or(false))
        .line_terminator(Some(b'\n'))
        .build(&pattern)
        .map_err(|e| format!("bad regex: {e}"))?;

    let include_globs = build_globset(&options.include_globs)?;
    let exclude_globs = build_globset(&options.exclude_globs)?;
    let only_paths = options
        .only_paths
        .map(|paths| paths.into_iter().collect::<std::collections::HashSet<_>>());

    let walker = WalkBuilder::new(&root_path)
        .hidden(true)
        .git_ignore(!options.include_ignored)
        .git_global(!options.include_ignored)
        .git_exclude(!options.include_ignored)
        .ignore(!options.include_ignored)
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
        let include_globs = include_globs.clone();
        let exclude_globs = exclude_globs.clone();
        let only_paths = only_paths.clone();
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
            if only_paths
                .as_ref()
                .is_some_and(|paths| !paths.contains(&rel))
            {
                return WalkState::Continue;
            }
            if let Some(set) = include_globs.as_ref() {
                if !set.is_match(&rel) {
                    return WalkState::Continue;
                }
            }
            if exclude_globs.as_ref().is_some_and(|set| set.is_match(&rel)) {
                return WalkState::Continue;
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

    #[test]
    fn path_globs_apply_include_and_exclude_filters() {
        let root = std::env::temp_dir().join(format!("labonair-grep-globs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("create source directory");
        std::fs::create_dir_all(root.join("docs")).expect("create docs directory");
        std::fs::write(root.join("src/keep.rs"), "needle\n").expect("write included file");
        std::fs::write(root.join("src/skip.rs"), "needle\n").expect("write excluded file");
        std::fs::write(root.join("docs/readme.md"), "needle\n").expect("write other file");

        let result = fs_grep_with_filters(
            "needle".into(),
            root.to_string_lossy().into_owned(),
            Some(vec!["src/*.rs".into()]),
            Some(vec!["**/skip.rs".into()]),
            None,
            None,
            false,
        )
        .expect("filtered grep succeeds");

        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].rel, "src/keep.rs");
        std::fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn only_paths_matches_exact_root_relative_open_files() {
        let root =
            std::env::temp_dir().join(format!("labonair-grep-open-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("create source directory");
        std::fs::write(root.join("src/open[1].rs"), "needle\n").expect("write open file");
        std::fs::write(root.join("src/closed.rs"), "needle\n").expect("write closed file");

        let result = fs_grep_with_options(
            "needle".into(),
            root.to_string_lossy().into_owned(),
            None,
            None,
            GrepOptions {
                only_paths: Some(vec!["src/open[1].rs".into()]),
                ..GrepOptions::default()
            },
        )
        .expect("open-file grep succeeds");
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].rel, "src/open[1].rs");

        let empty = fs_grep_with_options(
            "needle".into(),
            root.to_string_lossy().into_owned(),
            None,
            None,
            GrepOptions {
                only_paths: Some(Vec::new()),
                ..GrepOptions::default()
            },
        )
        .expect("empty open-file grep succeeds");
        assert!(empty.hits.is_empty());
        assert_eq!(empty.files_scanned, 0);

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
