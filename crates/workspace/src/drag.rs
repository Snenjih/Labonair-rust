//! In-tree drag payload + shell-quoting for drag-into-terminal.
//!
//! Extracted from `crates/ui/src/explorer.rs` in T16-006: `views::terminal`
//! needs [`DraggedPaths`] and [`quote_paths`] to accept explorer-row drops, and
//! `labonair-workspace` must not depend on `labonair-ui` (where the explorer
//! panel lives). The explorer re-imports these from here.

use std::path::{Path, PathBuf};

/// Payload of an in-tree drag (T05-002). Pure-data drag, mirroring the
/// reference `explorerDrag` module singleton — carries the selected paths from
/// an explorer row to a drop target (a folder in the same tree, or a terminal
/// pane which inserts the quoted path).
#[derive(Clone)]
pub struct DraggedPaths {
    pub paths: Vec<PathBuf>,
}

/// Shell-quote a single path for insertion into a terminal (single-quote wrap
/// unless it is entirely "safe" characters).
pub fn shell_quote(path: &Path) -> String {
    let s = path.to_string_lossy();
    let safe = !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || "-_./=:@%+,".contains(c));
    if safe {
        s.into_owned()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Space-joined shell-quoted paths (drag-into-terminal payload).
pub fn quote_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| shell_quote(p))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_paths_wraps_unsafe_only() {
        assert_eq!(
            quote_paths(&[PathBuf::from("/a/b"), PathBuf::from("/c d")]),
            "/a/b '/c d'"
        );
    }
}
