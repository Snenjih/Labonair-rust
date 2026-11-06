//! CI guard for the one-button-family rule (`docs/design-system.md`).
//!
//! Every button, icon button and toggle comes from `labonair-ui-kit`
//! (`button` / `icon_button_builder`), which owns hover, pressed, focus,
//! selected and disabled. A hand-built control styles its own hover and drifts
//! from the rest of the app. This test counts style `.hover(|..|)` calls
//! outside the kit and fails when a file gains one.
//!
//! The allowlist is the set of reviewed, non-button surfaces: list rows,
//! drag/resize grips and scrollbar thumbs. A new entry means a new surface
//! that is not a button — if it is one, use the kit instead.

use std::fs;
use std::path::Path;

/// `(path suffix, allowed count)` — what each entry is:
/// * `palette.rs` — result row, `hosts/page.rs` — host / credential rows,
///   `panel_git_graph.rs` / `panel_scm.rs` / `git_change_row.rs` /
///   `project_diff.rs` / `search_overlay.rs` / `spaces_ui.rs` — list rows,
/// * `pane_view.rs` / `sftp.rs` — resize grips (`sftp.rs` also a file row),
/// * `editor.rs` — the two scrollbar thumbs.
const ALLOWED: &[(&str, usize)] = &[
    ("command-palette/src/palette.rs", 1),
    ("editor-ui/src/editor.rs", 2),
    ("hosts-ui/src/hosts/page.rs", 3),
    ("panel-git-graph/src/panel_git_graph.rs", 1),
    ("panel-scm/src/git_change_row.rs", 1),
    ("panel-scm/src/panel_scm.rs", 1),
    ("workspace/src/pane_view.rs", 1),
    ("workspace/src/search_overlay.rs", 1),
    ("workspace/src/spaces_ui.rs", 1),
    ("workspace/src/views/project_diff.rs", 1),
    ("workspace/src/views/sftp.rs", 2),
];

#[test]
fn hover_styling_lives_in_the_ui_kit() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    visit(&crates, &mut found);

    let mut offenders = Vec::new();
    for (path, count) in &found {
        let allowed = ALLOWED
            .iter()
            .find(|(suffix, _)| path.ends_with(suffix))
            .map_or(0, |(_, n)| *n);
        if *count > allowed {
            offenders.push(format!("{path}: {count} hover style(s), {allowed} allowed"));
        }
    }
    assert!(
        offenders.is_empty(),
        "hand-built hover styling outside `labonair-ui-kit` — use `button` / \
         `icon_button_builder` (or `ListItem` / `TreeRow` for rows):\n{}",
        offenders.join("\n")
    );
}

fn visit(dir: &Path, out: &mut Vec<(String, usize)>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if matches!(name, "ui-kit" | "target" | "tests") {
                continue;
            }
            visit(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).unwrap();
            let count = text
                .lines()
                .filter(|line| {
                    let line = line.trim_start();
                    !line.starts_with("//")
                        && (line.contains(".hover(|") || line.contains(".hover(move |"))
                })
                .count();
            if count > 0 {
                out.push((path.to_string_lossy().replace('\\', "/"), count));
            }
        }
    }
}
