//! Developer preview: opens the workspace DiffView in its own window against a
//! real repository, for visual checks (`docs/parity/diff-view-spec.md` §12).
//!
//! ```text
//! cargo run -p labonair-shell --example diff_preview -- <repo> [staged|unstaged|file:<path>|since:<ref>|commit:<hash>] [light] [narrow]
//! ```

use std::sync::Arc;

use gpui::{px, size, App, AppContext as _, Application, Bounds, WindowBounds, WindowOptions};
use gpui_component::Root;
use labonair_events::EventBus;
use labonair_git::GitService;
use labonair_git_transport::GitTransportService;
use labonair_panel::{DiffSource, ProjectDiffFile, ProjectDiffMode, ProjectDiffRequest};
use labonair_ssh_transport::SshState;
use labonair_workspace::views::project_diff::ProjectDiffView;

fn main() {
    let mut args = std::env::args().skip(1);
    let repo = args.next().expect("usage: diff_preview <repo> [mode]");
    let mode = args.next().unwrap_or_default();
    let flags: Vec<String> = args.collect();
    let has = move |flag: &str| flags.iter().any(|f| f == flag);
    let (light, narrow) = (has("light"), has("narrow"));

    let runtime = tokio::runtime::Runtime::new().expect("tokio");
    let handle = runtime.handle().clone();
    let _guard = runtime.enter();
    let git: Arc<dyn GitService> = Arc::new(GitTransportService::new(
        SshState::default(),
        EventBus::new(),
    ));

    Application::new()
        .with_assets(labonair_shell::Assets)
        .run(move |cx: &mut App| {
            labonair_shell::init_fonts(cx);
            labonair_shell::init_settings(cx);
            gpui_component::init(cx);
            let bounds = Bounds::centered(
                None,
                size(px(if narrow { 640.0 } else { 1280.0 }), px(820.0)),
                cx,
            );
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                move |window, cx| {
                    let theme = labonair_shell::init_theme(window.appearance(), cx);
                    if light {
                        theme.update(cx, |store, cx| {
                            store.set_system_appearance(gpui::WindowAppearance::Light, cx)
                        });
                    }
                    labonair_shell::init_notifications(cx);
                    let view =
                        cx.new(|cx| ProjectDiffView::new(theme, git.clone(), handle.clone(), cx));
                    let (git, repo, mode, view_handle) =
                        (git.clone(), repo.clone(), mode.clone(), view.clone());
                    let task = handle.spawn({
                        let (git, repo) = (git.clone(), repo.clone());
                        async move { git.workspace_state(repo, None).await }
                    });
                    cx.spawn(async move |cx| {
                        let state = task.await.expect("join").expect("git status");
                        let mut files = Vec::new();
                        let conflicted: Vec<String> = state
                            .status
                            .unstaged
                            .iter()
                            .chain(state.status.staged.iter())
                            .filter(|f| f.conflicted)
                            .map(|f| f.path.clone())
                            .collect();
                        let mut push = |paths: Vec<String>, staged, untracked| {
                            for path in paths {
                                files.push(ProjectDiffFile {
                                    conflicted: conflicted.contains(&path),
                                    path,
                                    staged,
                                    untracked,
                                });
                            }
                        };
                        push(
                            state.status.staged.iter().map(|f| f.path.clone()).collect(),
                            true,
                            false,
                        );
                        push(
                            state
                                .status
                                .unstaged
                                .iter()
                                .map(|f| f.path.clone())
                                .collect(),
                            false,
                            false,
                        );
                        push(
                            state
                                .status
                                .untracked
                                .iter()
                                .map(|f| f.path.clone())
                                .collect(),
                            false,
                            true,
                        );
                        let source = match mode.split_once(':') {
                            Some(("file", path)) => DiffSource::File { path: path.into() },
                            Some(("since", base)) => DiffSource::Since { base: base.into() },
                            Some(("commit", hash)) => DiffSource::Commit {
                                hash: hash.into(),
                                subject: String::new(),
                            },
                            _ if mode == "staged" => {
                                files.retain(|f| f.staged);
                                DiffSource::Staged
                            }
                            _ if mode == "unstaged" => {
                                files.retain(|f| !f.staged);
                                DiffSource::Unstaged
                            }
                            _ => DiffSource::WorkingTree,
                        };
                        if let DiffSource::File { path } = &source {
                            files.retain(|f| &f.path == path);
                        }
                        let request = ProjectDiffRequest {
                            repo_root: repo,
                            session_id: None,
                            source,
                            files,
                            selected: None,
                            mode: ProjectDiffMode::Unified,
                        };
                        let _ = view_handle.update(cx, |view, cx| view.apply_request(request, cx));
                    })
                    .detach();
                    let any: gpui::AnyView = view.into();
                    cx.new(|cx| Root::new(any, window, cx))
                },
            )
            .expect("open window");
            cx.activate(true);
        });
}
