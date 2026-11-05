//! Workspace's narrow adapter for the Editor-owned project-search contract.
//!
//! This module owns neither query state nor result state. It translates one
//! typed Editor request to the existing `labonair-filesystem::grep` provider
//! and maps the response back once, on a blocking background task.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use labonair_editor::{
    ProjectSearchHit, ProjectSearchQuery, ProjectSearchRequest, ProjectSearchResult,
};
use labonair_filesystem::grep::{fs_grep_with_options, GrepOptions, GrepResponse};

pub(crate) fn search_blocking(
    root: PathBuf,
    request: ProjectSearchRequest,
    open_file_paths: Vec<String>,
) -> Result<ProjectSearchResult, String> {
    let pattern = request.query.backend_pattern().map_err(|e| e.to_string())?;
    let options = GrepOptions {
        include_globs: request.query.include_globs.clone(),
        exclude_globs: request.query.exclude_globs.clone(),
        only_paths: request
            .query
            .options
            .only_open_files
            .then_some(open_file_paths),
        include_ignored: request.query.options.include_ignored,
    };
    let response = fs_grep_with_options(
        pattern,
        root.to_string_lossy().into_owned(),
        Some(
            !request
                .query
                .options
                .case_sensitive_for_matching(&request.query.text),
        ),
        Some(request.query.max_results),
        options,
    )?;
    map_response(&root, &request.query, request.generation, response)
}

/// Map open editor paths into stable root-relative filters. Missing paths and
/// paths that resolve outside the project root are excluded.
pub(crate) fn relative_open_paths(
    root: &Path,
    candidates: impl IntoIterator<Item = PathBuf>,
) -> Vec<String> {
    let Ok(canonical_root) = std::fs::canonicalize(root) else {
        return Vec::new();
    };
    let mut paths = BTreeSet::new();
    for candidate in candidates {
        let Ok(canonical_candidate) = std::fs::canonicalize(candidate) else {
            continue;
        };
        let Ok(relative) = canonical_candidate.strip_prefix(&canonical_root) else {
            continue;
        };
        paths.insert(relative.to_string_lossy().into_owned());
    }
    paths.into_iter().collect()
}

fn map_response(
    root: &Path,
    query: &ProjectSearchQuery,
    generation: u64,
    response: GrepResponse,
) -> Result<ProjectSearchResult, String> {
    let mut hits = Vec::with_capacity(response.hits.len());
    for hit in response.hits {
        let path = root_safe_path(root, Path::new(&hit.path))?;
        let Some((start_column, end_column)) = query
            .first_match_columns(&hit.text)
            .map_err(|e| e.to_string())?
        else {
            continue;
        };
        hits.push(ProjectSearchHit::with_relative_path(
            path.to_string_lossy(),
            hit.rel,
            hit.line,
            hit.text,
            start_column,
            end_column,
        ));
        if hits.len() == query.max_results {
            break;
        }
    }
    Ok(ProjectSearchResult::new(
        generation,
        hits,
        response.truncated,
        response.files_scanned,
    ))
}

/// Canonicalize both sides before accepting a provider path. This prevents a
/// malformed or symlinked provider result from being opened outside the
/// explicit workspace root.
pub(crate) fn root_safe_path(root: &Path, candidate: &Path) -> Result<PathBuf, String> {
    let canonical_root = std::fs::canonicalize(root)
        .map_err(|error| format!("cannot resolve search root {}: {error}", root.display()))?;
    let canonical_candidate = std::fs::canonicalize(candidate).map_err(|error| {
        format!(
            "cannot resolve search result {}: {error}",
            candidate.display()
        )
    })?;
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err(format!(
            "search result escapes workspace root: {}",
            candidate.display()
        ));
    }
    Ok(canonical_candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use labonair_editor::ProjectSearchOptions;
    use labonair_filesystem::grep::{GrepHit, GrepResponse};

    fn query() -> ProjectSearchQuery {
        ProjectSearchQuery::new("needle", ProjectSearchOptions::default(), 2)
    }

    #[test]
    fn adapter_maps_line_text_columns_and_truncation() {
        let root =
            std::env::temp_dir().join(format!("labonair-project-search-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("test root");
        let file = root.join("src.rs");
        std::fs::write(&file, "needle\nneedle\n").expect("test file");
        let response = GrepResponse {
            hits: vec![
                GrepHit {
                    path: file.to_string_lossy().into_owned(),
                    rel: "src.rs".into(),
                    line: 1,
                    text: "needle".into(),
                },
                GrepHit {
                    path: file.to_string_lossy().into_owned(),
                    rel: "src.rs".into(),
                    line: 2,
                    text: "needle".into(),
                },
            ],
            truncated: true,
            files_scanned: 1,
        };
        let result = map_response(&root, &query(), 7, response).expect("mapped result");
        assert_eq!(result.generation, 7);
        assert_eq!(result.hits.len(), 2);
        assert_eq!(result.hits[1].relative_path, "src.rs");
        assert_eq!(result.hits[1].start_column, 0);
        assert_eq!(result.hits[1].end_column, 6);
        assert!(result.truncated);
        assert_eq!(result.files_scanned, 1);
        std::fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn root_safe_path_rejects_results_outside_root() {
        let root = std::env::temp_dir().join(format!(
            "labonair-project-search-root-{}",
            std::process::id()
        ));
        let outside = std::env::temp_dir().join(format!(
            "labonair-project-search-outside-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&outside);
        std::fs::create_dir_all(&root).expect("test root");
        std::fs::write(&outside, "outside").expect("outside file");
        let error = root_safe_path(&root, &outside).expect_err("escape must be rejected");
        assert!(error.contains("escapes workspace root"));
        std::fs::remove_dir_all(root).expect("remove test root");
        std::fs::remove_file(outside).expect("remove outside file");
    }

    #[test]
    fn relative_open_paths_are_deduplicated_and_root_scoped() {
        let root = std::env::temp_dir().join(format!(
            "labonair-project-open-paths-{}",
            std::process::id()
        ));
        let outside = std::env::temp_dir().join(format!(
            "labonair-project-open-outside-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&outside);
        std::fs::create_dir_all(root.join("src")).expect("create root source directory");
        std::fs::write(root.join("src/main.rs"), "open").expect("write open file");
        std::fs::write(&outside, "outside").expect("write outside file");

        let file = root.join("src/main.rs");
        assert_eq!(
            relative_open_paths(&root, [file.clone(), file, outside.clone()]),
            ["src/main.rs"]
        );
        assert!(relative_open_paths(&root, [root.join("missing.rs")]).is_empty());

        std::fs::remove_dir_all(root).expect("remove test root");
        std::fs::remove_file(outside).expect("remove outside file");
    }
}
