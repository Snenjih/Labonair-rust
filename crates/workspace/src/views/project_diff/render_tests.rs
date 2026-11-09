//! Headless tests for the DiffView: a scripted in-memory `GitService` drives
//! load → render → stage → reload end to end.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{point, px, size, AppContext as _, TestAppContext, VisualTestContext};
use labonair_editor::diff_document::{DiffStyle, FlatRow, HunkId, HunkStage};
use labonair_git::*;
use labonair_panel::{DiffSource, ProjectDiffFile, ProjectDiffMode, ProjectDiffRequest};

use super::ProjectDiffView;
use crate::theme::ThemeStore;

fn unsupported<T: Send + 'static>() -> GitFuture<T> {
    Box::pin(async { Err("unsupported in test".to_string()) })
}

const UNSTAGED: &str = "diff --git a/src/lib.rs b/src/lib.rs
index 111..222 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
 fn a() {}
-fn b() {}
+fn b2() {}
 fn c() {}
@@ -20,2 +20,2 @@
-let x = 1;
+let x = 2;
 end
";

const STAGED: &str = "diff --git a/README.md b/README.md
index 111..222 100644
--- a/README.md
+++ b/README.md
@@ -1 +1 @@
-old
+new
";

/// Scripted git: `diff` answers from a map, mutations are recorded.
#[derive(Default)]
struct FakeGit {
    diffs: Mutex<HashMap<(String, bool), String>>,
    /// Whole-file patches answered by `diff_full`; absent paths fall back.
    fulls: Mutex<HashMap<(String, bool), String>>,
    /// Patch answered by `commit_diff`.
    commit_patch: Mutex<Option<String>>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl FakeGit {
    fn record(&self, call: String) -> GitFuture<()> {
        self.calls.lock().unwrap().push(call);
        Box::pin(async { Ok(()) })
    }
}

impl GitService for FakeGit {
    fn diff(
        &self,
        _path: String,
        file: String,
        staged: bool,
        _ignore_whitespace: Option<bool>,
        _is_untracked: Option<bool>,
        _session_id: Option<String>,
    ) -> GitFuture<String> {
        let text = self.diffs.lock().unwrap().get(&(file, staged)).cloned();
        Box::pin(async move { text.ok_or_else(|| "no diff".to_string()) })
    }

    fn diff_full(
        &self,
        path: String,
        file: String,
        staged: bool,
        is_untracked: Option<bool>,
        session_id: Option<String>,
    ) -> GitFuture<String> {
        if let Some(text) = self.fulls.lock().unwrap().get(&(file.clone(), staged)) {
            let text = text.clone();
            return Box::pin(async move { Ok(text) });
        }
        self.diff(path, file, staged, Some(false), is_untracked, session_id)
    }

    fn stage_hunk(
        &self,
        _p: String,
        file: String,
        patch: String,
        _s: Option<String>,
    ) -> GitFuture<()> {
        self.record(format!("stage_hunk {file} {}", patch.contains("@@")))
    }

    fn unstage_hunk(
        &self,
        _p: String,
        file: String,
        patch: String,
        _s: Option<String>,
    ) -> GitFuture<()> {
        self.record(format!("unstage_hunk {file} {}", patch.contains("@@")))
    }

    fn stage_file(&self, _p: String, file: String, _s: Option<String>) -> GitFuture<()> {
        self.record(format!("stage_file {file}"))
    }

    fn unstage_file(&self, _p: String, file: String, _s: Option<String>) -> GitFuture<()> {
        self.record(format!("unstage_file {file}"))
    }

    fn stage_all(&self, _p: String, _s: Option<String>) -> GitFuture<()> {
        self.record("stage_all".to_string())
    }

    fn unstage_all(&self, _p: String, _s: Option<String>) -> GitFuture<()> {
        self.record("unstage_all".to_string())
    }

    fn discard_hunk(
        &self,
        _p: String,
        file: String,
        _patch: String,
        staged: bool,
        _s: Option<String>,
    ) -> GitFuture<()> {
        self.record(format!("discard_hunk {file} staged={staged}"))
    }

    fn is_repo(&self, _path: String, _session_id: Option<String>) -> GitFuture<bool> {
        unsupported()
    }

    fn repo_root(&self, _path: String, _session_id: Option<String>) -> GitFuture<String> {
        unsupported()
    }

    fn workspace_state(
        &self,
        _path: String,
        _session_id: Option<String>,
    ) -> GitFuture<WorkspaceGitState> {
        unsupported()
    }

    fn log(
        &self,
        _path: String,
        _limit: Option<u32>,
        _all_branches: bool,
        _session_id: Option<String>,
        _skip: Option<usize>,
    ) -> GitFuture<Vec<CommitInfo>> {
        unsupported()
    }

    fn blame_line(
        &self,
        _path: String,
        _file: String,
        _line: u32,
        _session_id: Option<String>,
    ) -> GitFuture<GitBlameLine> {
        unsupported()
    }

    fn commit_diff(
        &self,
        _path: String,
        _hash: String,
        _session_id: Option<String>,
    ) -> GitFuture<String> {
        match self.commit_patch.lock().unwrap().clone() {
            Some(text) => Box::pin(async move { Ok(text) }),
            None => unsupported(),
        }
    }

    fn commit_header(
        &self,
        _path: String,
        hash: String,
        _session_id: Option<String>,
    ) -> GitFuture<CommitHeader> {
        Box::pin(async move {
            Ok(CommitHeader {
                hash,
                author_name: "Ada".into(),
                author_email: "ada@example.dev".into(),
                timestamp: 1_700_000_000,
                message: "Fix the thing\n\nBecause it was broken.".into(),
            })
        })
    }

    fn stash_diff(
        &self,
        _path: String,
        _hash: String,
        _session_id: Option<String>,
    ) -> GitFuture<String> {
        unsupported()
    }

    fn branch_diff(
        &self,
        _path: String,
        _base: String,
        _head: String,
        _session_id: Option<String>,
    ) -> GitFuture<String> {
        unsupported()
    }

    fn discard_file(
        &self,
        _path: String,
        _file: String,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }

    fn discard_all(&self, _path: String, _session_id: Option<String>) -> GitFuture<()> {
        unsupported()
    }

    fn clean_untracked(&self, _path: String, _session_id: Option<String>) -> GitFuture<()> {
        unsupported()
    }

    fn commit(
        &self,
        _path: String,
        _message: String,
        _amend: bool,
        _session_id: Option<String>,
    ) -> GitFuture<CommitResult> {
        unsupported()
    }

    fn push(
        &self,
        _path: String,
        _remote: Option<String>,
        _branch: Option<String>,
        _session_id: Option<String>,
    ) -> GitFuture<String> {
        unsupported()
    }

    fn pull(&self, _path: String, _session_id: Option<String>) -> GitFuture<String> {
        unsupported()
    }

    fn fetch(&self, _path: String, _session_id: Option<String>) -> GitFuture<String> {
        unsupported()
    }

    fn abort(&self, _path: String, _session_id: Option<String>) -> GitFuture<()> {
        unsupported()
    }

    fn continue_operation(&self, _path: String, _session_id: Option<String>) -> GitFuture<()> {
        unsupported()
    }

    fn checkout_branch(
        &self,
        _path: String,
        _branch: String,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }

    fn create_branch(
        &self,
        _path: String,
        _name: String,
        _from_ref: Option<String>,
        _checkout: bool,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }

    fn delete_branch(
        &self,
        _path: String,
        _name: String,
        _force: bool,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }

    fn rename_branch(
        &self,
        _path: String,
        _old_name: String,
        _new_name: String,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }

    fn create_tag(
        &self,
        _path: String,
        _name: String,
        _message: Option<String>,
        _from_ref: Option<String>,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }

    fn delete_tag(
        &self,
        _path: String,
        _name: String,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }

    fn push_tag(
        &self,
        _path: String,
        _name: String,
        _remote: Option<String>,
        _session_id: Option<String>,
    ) -> GitFuture<String> {
        unsupported()
    }

    fn stash_push(
        &self,
        _path: String,
        _message: Option<String>,
        _include_untracked: Option<bool>,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }

    fn stash_pop(
        &self,
        _path: String,
        _hash: String,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }

    fn stash_apply(
        &self,
        _path: String,
        _hash: String,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }

    fn stash_drop(
        &self,
        _path: String,
        _hash: String,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }

    fn add_to_gitignore(
        &self,
        _path: String,
        _file: String,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }

    fn add_to_exclude(
        &self,
        _path: String,
        _file: String,
        _session_id: Option<String>,
    ) -> GitFuture<()> {
        unsupported()
    }
}

struct Harness {
    view: gpui::Entity<ProjectDiffView>,
    calls: Arc<Mutex<Vec<String>>>,
    _runtime: tokio::runtime::Runtime,
}

fn harness(cx: &mut TestAppContext) -> (Harness, &mut VisualTestContext) {
    harness_with(cx, |_| {})
}

fn harness_with(
    cx: &mut TestAppContext,
    configure: impl FnOnce(&FakeGit),
) -> (Harness, &mut VisualTestContext) {
    cx.update(|cx| {
        labonair_notifications::init(cx);
    });
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let git = FakeGit::default();
    configure(&git);
    git.diffs
        .lock()
        .unwrap()
        .insert(("src/lib.rs".into(), false), UNSTAGED.into());
    git.diffs
        .lock()
        .unwrap()
        .insert(("README.md".into(), true), STAGED.into());
    let calls = git.calls.clone();
    let handle = runtime.handle().clone();
    let (view, vcx) = cx.add_window_view(|_, cx| {
        let theme = cx.new(|_| ThemeStore::new(gpui::WindowAppearance::Light));
        ProjectDiffView::new(theme, Arc::new(git), handle, cx)
    });
    (
        Harness {
            view,
            calls,
            _runtime: runtime,
        },
        vcx,
    )
}

fn request() -> ProjectDiffRequest {
    let file = |path: &str, staged| ProjectDiffFile {
        path: path.into(),
        staged,
        untracked: false,
    };
    ProjectDiffRequest {
        repo_root: "/repo".into(),
        session_id: None,
        source: DiffSource::WorkingTree,
        files: vec![file("src/lib.rs", false), file("README.md", true)],
        selected: Some("src/lib.rs".into()),
        mode: ProjectDiffMode::Unified,
    }
}

/// Pump the foreground executor while background git work finishes.
fn wait_until(cx: &mut VisualTestContext, done: impl Fn(&mut VisualTestContext) -> bool) {
    for _ in 0..200 {
        cx.run_until_parked();
        if done(cx) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("condition not reached");
}

fn draw(cx: &mut VisualTestContext, view: &gpui::Entity<ProjectDiffView>, width: f32) {
    let view = view.clone();
    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(width), px(600.0)),
        move |_, _| view.clone(),
    );
    // The width probe defers its update; an app update flushes it.
    cx.update(|_, _| {});
}

#[gpui::test]
fn loads_merges_and_renders_both_styles(cx: &mut TestAppContext) {
    let (h, cx) = harness(cx);
    h.view.update(cx, |v, cx| v.apply_request(request(), cx));
    wait_until(cx, |cx| h.view.read_with(cx, |v, _| !v.doc.is_empty()));

    h.view.read_with(cx, |v, _| {
        // Tracked files sort by path: README.md (staged) before src/lib.rs.
        let paths: Vec<_> = v.doc.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["README.md", "src/lib.rs"]);
        assert_eq!(v.doc.totals(), (3, 3));
        assert_eq!(v.doc.files[0].hunks[0].stage, HunkStage::Staged);
        assert_eq!(v.doc.files[1].hunks.len(), 2);
        // Requested selection scrolls to its file; cursor lands on a hunk.
        assert!(v.cursor.is_some());
        assert!(matches!(v.rows[0], FlatRow::File(0)));
    });

    draw(cx, &h.view, 1200.0);
    h.view.update(cx, |v, cx| v.set_style(DiffStyle::Split, cx));
    draw(cx, &h.view, 1200.0);
    h.view.read_with(cx, |v, _| {
        assert!(v.rows.iter().any(|r| matches!(r, FlatRow::Split { .. })));
    });
}

#[gpui::test]
fn split_suspends_below_the_minimum_width(cx: &mut TestAppContext) {
    let (h, cx) = harness(cx);
    h.view.update(cx, |v, cx| v.apply_request(request(), cx));
    wait_until(cx, |cx| h.view.read_with(cx, |v, _| !v.doc.is_empty()));
    h.view.update(cx, |v, cx| v.set_style(DiffStyle::Split, cx));

    // The width probe measures the real container on paint.
    cx.simulate_resize(size(px(400.0), px(700.0)));
    cx.update(|_, _| {});
    h.view.read_with(cx, |v, _| {
        assert!(v.too_narrow_for_split);
        assert!(!v.rows.iter().any(|r| matches!(r, FlatRow::Split { .. })));
        assert_eq!(v.style, DiffStyle::Split);
    });

    cx.simulate_resize(size(px(2400.0), px(700.0)));
    cx.update(|_, _| {});
    h.view.read_with(cx, |v, _| {
        assert!(!v.too_narrow_for_split);
        assert!(v.rows.iter().any(|r| matches!(r, FlatRow::Split { .. })));
    });
}

#[gpui::test]
fn stage_and_next_stages_the_cursor_hunk_and_advances(cx: &mut TestAppContext) {
    let (h, cx) = harness(cx);
    h.view.update(cx, |v, cx| v.apply_request(request(), cx));
    wait_until(cx, |cx| h.view.read_with(cx, |v, _| !v.doc.is_empty()));

    let first_unstaged = HunkId { file: 1, hunk: 0 };
    h.view.update(cx, |v, cx| {
        v.set_cursor(first_unstaged, cx);
        v.apply_cursor_and_next(HunkStage::Unstaged, cx);
    });
    wait_until(cx, |_| !h.calls.lock().unwrap().is_empty());
    assert_eq!(
        h.calls.lock().unwrap().as_slice(),
        ["stage_hunk src/lib.rs true"]
    );
    h.view.read_with(cx, |v, _| {
        assert_eq!(v.cursor, Some(HunkId { file: 1, hunk: 1 }));
    });

    // Unstaged hunks stage; staged hunks need Unstage.
    h.view.update(cx, |v, cx| {
        v.set_cursor(HunkId { file: 0, hunk: 0 }, cx);
        v.apply_cursor_and_next(HunkStage::Unstaged, cx);
    });
    cx.run_until_parked();
    assert_eq!(h.calls.lock().unwrap().len(), 1);
}

#[gpui::test]
fn restore_needs_a_second_click(cx: &mut TestAppContext) {
    let (h, cx) = harness(cx);
    h.view.update(cx, |v, cx| v.apply_request(request(), cx));
    wait_until(cx, |cx| h.view.read_with(cx, |v, _| !v.doc.is_empty()));

    let id = HunkId { file: 1, hunk: 0 };
    h.view.update(cx, |v, cx| v.restore_hunk(id, cx));
    cx.run_until_parked();
    assert!(h.calls.lock().unwrap().is_empty());
    h.view.update(cx, |v, cx| v.restore_hunk(id, cx));
    wait_until(cx, |_| !h.calls.lock().unwrap().is_empty());
    assert_eq!(
        h.calls.lock().unwrap().as_slice(),
        ["discard_hunk src/lib.rs staged=false"]
    );
}

#[gpui::test]
fn fold_hides_a_file_body_and_alt_folds_all(cx: &mut TestAppContext) {
    let (h, cx) = harness(cx);
    h.view.update(cx, |v, cx| v.apply_request(request(), cx));
    wait_until(cx, |cx| h.view.read_with(cx, |v, _| !v.doc.is_empty()));
    let full = h.view.read_with(cx, |v, _| v.rows.len());

    h.view
        .update(cx, |v, cx| v.toggle_fold("src/lib.rs", false, cx));
    let after_one = h.view.read_with(cx, |v, _| v.rows.len());
    assert!(after_one < full);
    h.view.update(cx, |v, cx| v.toggle_fold("", true, cx));
    h.view.read_with(cx, |v, _| assert_eq!(v.rows.len(), 2));
    draw(cx, &h.view, 900.0);
    h.view.update(cx, |v, cx| v.toggle_fold("", true, cx));
    assert_eq!(h.view.read_with(cx, |v, _| v.rows.len()), full);
}

#[gpui::test]
fn read_only_sources_offer_no_staging(cx: &mut TestAppContext) {
    let (h, cx) = harness(cx);
    let mut req = request();
    req.source = DiffSource::Commit {
        hash: "abc1234".into(),
        subject: "subject".into(),
    };
    req.files.clear();
    // The fake has no `commit_diff`, so loading fails and the view stays empty
    // without panicking and without staging controls.
    h.view.update(cx, |v, cx| v.apply_request(req, cx));
    cx.run_until_parked();
    draw(cx, &h.view, 900.0);
    h.view.read_with(cx, |v, _| {
        assert!(!v.supports_staging());
        assert!(v.doc.is_empty());
    });
}

#[gpui::test]
fn settings_drive_style_and_minimum_width(cx: &mut TestAppContext) {
    use labonair_settings::{EditorSettings, SettingsContent, SettingsLayer, SettingsStore};

    cx.update(|cx| {
        let mut content = SettingsContent::default();
        content.editor.editor_diff_view_style = Some("unified".into());
        content.editor.editor_minimum_split_diff_width = Some(300);
        let mut store = SettingsStore::new(std::env::temp_dir().join(format!(
            "labonair-diff-settings-{}.json",
            std::process::id()
        )));
        store.register_setting::<EditorSettings>();
        store.set_layer(SettingsLayer::User, content);
        cx.set_global(store);
    });
    let (h, cx) = harness(cx);
    h.view.read_with(cx, |v, _| {
        assert_eq!(v.style, DiffStyle::Unified);
        assert_eq!(v.min_split_columns, 300.0);
    });

    cx.update(|_, cx| {
        let mut content = SettingsContent::default();
        content.editor.editor_diff_view_style = Some("split".into());
        content.editor.editor_minimum_split_diff_width = Some(0);
        cx.global_mut::<SettingsStore>()
            .set_layer(SettingsLayer::User, content);
    });
    cx.run_until_parked();
    h.view.read_with(cx, |v, _| {
        assert_eq!(v.style, DiffStyle::Split);
        // 0 disables the narrow fallback entirely.
        assert_eq!(v.min_split_columns, 0.0);
        assert!(!v.too_narrow_for_split);
    });
}

#[gpui::test]
fn staged_and_unstaged_sources_show_only_their_hunks(cx: &mut TestAppContext) {
    for (source, expected_stage, title, empty) in [
        (
            DiffSource::Staged,
            HunkStage::Staged,
            "Staged Changes",
            "No staged changes",
        ),
        (
            DiffSource::Unstaged,
            HunkStage::Unstaged,
            "Unstaged Changes",
            "No unstaged changes",
        ),
    ] {
        let (h, cx) = harness(cx);
        let mut req = request();
        req.files
            .retain(|f| f.staged == (source == DiffSource::Staged));
        req.source = source;
        h.view.update(cx, |v, cx| v.apply_request(req, cx));
        wait_until(cx, |cx| h.view.read_with(cx, |v, _| !v.doc.is_empty()));
        draw(cx, &h.view, 1200.0);
        h.view.read_with(cx, |v, _| {
            assert!(v.supports_staging());
            assert_eq!(v.header_title(), title);
            assert_eq!(v.empty_label(), empty);
            assert!(v
                .doc
                .files
                .iter()
                .flat_map(|f| &f.hunks)
                .all(|hunk| hunk.stage == expected_stage));
        });
    }
}

#[gpui::test]
fn hunk_style_settings_default_to_staged_hollow_with_buttons(cx: &mut TestAppContext) {
    use labonair_settings::{
        EditorSettings, Settings as _, SettingsContent, SettingsLayer, SettingsStore,
    };

    cx.update(|cx| {
        let mut content = SettingsContent::default();
        content.editor.editor_diff_hunk_style = Some("unstagedHollow".into());
        content.editor.editor_diff_show_stage_restore_buttons = Some(false);
        let mut store = SettingsStore::new(std::env::temp_dir().join(format!(
            "labonair-diff-hunk-style-{}.json",
            std::process::id()
        )));
        store.register_setting::<EditorSettings>();
        store.set_layer(SettingsLayer::User, content);
        cx.set_global(store);
        let s = EditorSettings::try_get(cx).expect("settings");
        assert!(!s.diff_staged_hollow());
        assert!(!s.diff_show_stage_restore_buttons());
    });
    let (h, cx) = harness(cx);
    h.view.update(cx, |v, cx| v.apply_request(request(), cx));
    wait_until(cx, |cx| h.view.read_with(cx, |v, _| !v.doc.is_empty()));
    // Renders with the inverted hollow style and without hunk controls.
    draw(cx, &h.view, 1200.0);
}

#[gpui::test]
fn command_actions_navigate_stage_and_toggle_split(cx: &mut TestAppContext) {
    use super::DiffAction;

    let (h, cx) = harness(cx);
    h.view.update(cx, |v, cx| v.apply_request(request(), cx));
    wait_until(cx, |cx| h.view.read_with(cx, |v, _| !v.doc.is_empty()));

    let first = h.view.read_with(cx, |v, _| v.cursor);
    h.view
        .update(cx, |v, cx| v.perform(DiffAction::GoToHunk, cx));
    assert_ne!(h.view.read_with(cx, |v, _| v.cursor), first);
    h.view
        .update(cx, |v, cx| v.perform(DiffAction::GoToPreviousHunk, cx));
    assert_eq!(h.view.read_with(cx, |v, _| v.cursor), first);

    let before = h.view.read_with(cx, |v, _| v.style);
    h.view
        .update(cx, |v, cx| v.perform(DiffAction::ToggleSplit, cx));
    assert_ne!(h.view.read_with(cx, |v, _| v.style), before);

    h.view
        .update(cx, |v, cx| v.perform(DiffAction::StageAll, cx));
    wait_until(cx, |_| {
        h.calls.lock().unwrap().iter().any(|c| c == "stage_all")
    });
}

#[gpui::test]
fn expand_controls_reveal_context_from_the_whole_file(cx: &mut TestAppContext) {
    // 30-line file, lines 10 and 24 replaced.
    let mut full = String::from(
        "diff --git a/src/lib.rs b/src/lib.rs\nindex 111..222 100644\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,30 +1,30 @@\n",
    );
    for n in 1..=30 {
        if n == 10 || n == 24 {
            full.push_str(&format!("-old{n}\n+new{n}\n"));
        } else {
            full.push_str(&format!(" l{n}\n"));
        }
    }
    let (h, cx) = harness_with(cx, |git| {
        git.fulls
            .lock()
            .unwrap()
            .insert(("src/lib.rs".into(), false), full);
    });
    let mut req = request();
    req.source = DiffSource::Unstaged;
    req.files.retain(|f| !f.staged);
    h.view.update(cx, |v, cx| v.apply_request(req, cx));
    wait_until(cx, |cx| h.view.read_with(cx, |v, _| !v.doc.is_empty()));

    let expands = |cx: &mut VisualTestContext| {
        h.view.read_with(cx, |v, _| {
            v.rows
                .iter()
                .filter(|r| matches!(r, FlatRow::Expand { .. }))
                .count()
        })
    };
    // Two excerpts; each has a hidden region above and below (the middle gap
    // counts for both neighbours).
    assert_eq!(expands(cx), 4);
    let lines_before = h
        .view
        .read_with(cx, |v, _| v.doc.files[0].hunks[0].lines.len());
    let id = HunkId { file: 0, hunk: 0 };
    h.view.update(cx, |v, cx| v.expand(id, true, cx));
    let lines_after = h
        .view
        .read_with(cx, |v, _| v.doc.files[0].hunks[0].lines.len());
    assert_eq!(lines_after, lines_before + 5);
    draw(cx, &h.view, 1200.0);
    // Staging still works from the regrouped, Git-style hunk.
    h.view.update(cx, |v, cx| v.apply_hunk(id, cx));
    wait_until(cx, |_| {
        h.calls
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.starts_with("stage_hunk src/lib.rs true"))
    });
}

#[gpui::test]
fn solo_file_diff_toggles_between_full_file_and_changes(cx: &mut TestAppContext) {
    let mut full = String::from(
        "diff --git a/src/lib.rs b/src/lib.rs\nindex 111..222 100644\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,30 +1,30 @@\n",
    );
    for n in 1..=30 {
        if n == 10 || n == 24 {
            full.push_str(&format!("-old{n}\n+new{n}\n"));
        } else {
            full.push_str(&format!(" l{n}\n"));
        }
    }
    let (h, cx) = harness_with(cx, |git| {
        git.fulls
            .lock()
            .unwrap()
            .insert(("src/lib.rs".into(), false), full);
    });
    let mut req = request();
    req.source = DiffSource::File {
        path: "src/lib.rs".into(),
    };
    req.files.retain(|f| !f.staged);
    h.view.update(cx, |v, cx| v.apply_request(req, cx));
    wait_until(cx, |cx| h.view.read_with(cx, |v, _| !v.doc.is_empty()));

    let (shown, expand_rows) = h.view.read_with(cx, |v, _| {
        (
            v.doc.files[0]
                .hunks
                .iter()
                .map(|h| h.lines.len())
                .sum::<usize>(),
            v.rows
                .iter()
                .filter(|r| matches!(r, FlatRow::Expand { .. }))
                .count(),
        )
    });
    // The whole 30-line file (+2 replaced lines) with nothing left to expand,
    // split across the two Git-style hunks that meet in the middle.
    assert_eq!(shown, 32);
    assert_eq!(expand_rows, 0);
    assert_eq!(
        h.view.read_with(cx, |v, _| v.header_title()),
        "Changes: src/lib.rs"
    );

    h.view.update(cx, |v, cx| v.toggle_full_file(cx));
    let (shown, expand_rows) = h.view.read_with(cx, |v, _| {
        (
            v.doc.files[0]
                .hunks
                .iter()
                .map(|h| h.lines.len())
                .sum::<usize>(),
            v.rows
                .iter()
                .filter(|r| matches!(r, FlatRow::Expand { .. }))
                .count(),
        )
    });
    assert!(shown < 32);
    assert_eq!(expand_rows, 4);
    draw(cx, &h.view, 1200.0);
}

#[gpui::test]
fn cursor_moves_tell_the_host_which_file_is_active(cx: &mut TestAppContext) {
    use super::{DiffAction, ProjectDiffEvent};

    let (h, cx) = harness(cx);
    h.view.update(cx, |v, cx| v.apply_request(request(), cx));
    wait_until(cx, |cx| h.view.read_with(cx, |v, _| !v.doc.is_empty()));

    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let _sub = cx.update(|_, app| {
        app.subscribe(&h.view, move |_, event: &ProjectDiffEvent, _| {
            let ProjectDiffEvent::CursorFile(path) = event;
            sink.lock().unwrap().push(path.clone());
        })
    });
    // Loading never announces a file; user-driven moves do.
    assert!(seen.lock().unwrap().is_empty());
    h.view
        .update(cx, |v, cx| v.perform(DiffAction::GoToHunk, cx));
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[gpui::test]
fn commit_source_shows_header_and_read_only_files(cx: &mut TestAppContext) {
    let (h, cx) = harness_with(cx, |git| {
        *git.commit_patch.lock().unwrap() = Some(STAGED.to_string());
    });
    let mut req = request();
    req.source = DiffSource::Commit {
        hash: "abc1234def".into(),
        subject: "Fix the thing".into(),
    };
    req.files.clear();
    h.view.update(cx, |v, cx| v.apply_request(req, cx));
    wait_until(cx, |cx| {
        h.view
            .read_with(cx, |v, _| !v.doc.is_empty() && v.commit_header.is_some())
    });
    draw(cx, &h.view, 1200.0);
    h.view.read_with(cx, |v, _| {
        let header = v.commit_header.as_ref().unwrap();
        assert_eq!(header.hash, "abc1234def");
        assert_eq!(header.author_name, "Ada");
        assert!(!v.supports_staging());
        assert_eq!(v.doc.files[0].hunks[0].stage, HunkStage::ReadOnly);
    });
}

#[gpui::test]
fn text_source_diffs_two_strings_read_only(cx: &mut TestAppContext) {
    let (h, cx) = harness(cx);
    let mut req = request();
    req.source = DiffSource::Text {
        title: "Clipboard \u{2194} Selection".into(),
        old: "a\nb\nc\n".into(),
        new: "a\nB\nc\n".into(),
    };
    req.files.clear();
    h.view.update(cx, |v, cx| v.apply_request(req, cx));
    wait_until(cx, |cx| h.view.read_with(cx, |v, _| !v.doc.is_empty()));
    draw(cx, &h.view, 1200.0);
    h.view.read_with(cx, |v, _| {
        assert!(!v.supports_staging() || v.doc.files[0].hunks[0].stage == HunkStage::ReadOnly);
        assert_eq!(v.doc.totals(), (1, 1));
        assert_eq!(v.header_title(), "Clipboard \u{2194} Selection");
    });
}
