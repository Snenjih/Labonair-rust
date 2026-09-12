//! Workspace adapter and modal presentation for the Editor file finder.
//!
//! Filesystem traversal is deliberately kept at this boundary. The view owns
//! only transient controls and forwards validated candidates to
//! [`Workspace::open_file`].

use std::path::PathBuf;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, ClickEvent, Context, DismissEvent, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Task, Window,
};
use labonair_editor::{
    FileFinderCandidate, FileFinderQuery, FileFinderResult, FileFinderSession, FileFinderStatus,
    DEFAULT_FILE_FINDER_MAX_RESULTS,
};
use labonair_filesystem::grep::fs_glob;
use labonair_ui_kit::{
    banner, button, field_input, keybinding_hint, text_field, ButtonSize, ButtonVariant, IconName,
    InputEvent, InputState, ListItem, Palette, Severity,
};
use tokio::task::{AbortHandle, JoinHandle};

use crate::modal_layer::ModalView;
use crate::theme::ThemeStore;
use crate::Workspace;

/// The filesystem adapter's hard bound. Ranking happens in Editor after this
/// bounded listing, so a slow or very large root cannot produce an unbounded
/// in-memory result set.
pub(crate) const FILE_FINDER_LIST_CAP: usize = 5_000;

/// List and validate local files for one Editor-owned finder request.
pub(crate) fn list_blocking(
    filesystem_root: PathBuf,
    query: FileFinderQuery,
    generation: u64,
) -> Result<FileFinderResult, String> {
    let canonical_root = std::fs::canonicalize(&filesystem_root).map_err(|error| {
        format!(
            "cannot resolve workspace root {}: {error}",
            filesystem_root.display()
        )
    })?;
    if !canonical_root.is_dir() {
        return Err(format!(
            "workspace root is not a directory: {}",
            canonical_root.display()
        ));
    }

    let response = fs_glob(
        "**/*".to_string(),
        canonical_root.to_string_lossy().into_owned(),
        Some(FILE_FINDER_LIST_CAP),
    )?;
    let mut candidates = Vec::with_capacity(response.hits.len());
    for hit in response.hits {
        let Ok(path) = std::fs::canonicalize(&hit.path) else {
            continue;
        };
        if !path.starts_with(&canonical_root) || !path.is_file() {
            continue;
        }
        let Ok(relative_path) = path.strip_prefix(&canonical_root) else {
            continue;
        };
        let relative_path = relative_path.to_string_lossy().into_owned();
        candidates.push(FileFinderCandidate::new(path, relative_path));
    }

    Ok(FileFinderResult::rank(
        generation,
        &query,
        candidates,
        response.truncated,
    ))
}

/// A keyboard-first Open File modal hosted by the workspace's existing
/// [`crate::modal_layer::ModalLayer`].
pub struct FileFinderView {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
    input: Entity<InputState>,
    session: FileFinderSession,
    listing_abort: Option<AbortHandle>,
    listing_task: Option<Task<()>>,
    _input_subscription: Subscription,
}

impl FileFinderView {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input =
            cx.new(|cx| text_field(window, cx).placeholder("Search files by name or path…"));
        let input_subscription = cx.subscribe(&input, |this, _input, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.query_changed(cx);
            }
        });
        let mut this = Self {
            workspace,
            theme,
            input,
            session: FileFinderSession::new(DEFAULT_FILE_FINDER_MAX_RESULTS),
            listing_abort: None,
            listing_task: None,
            _input_subscription: input_subscription,
        };
        this.query_changed(cx);
        this
    }

    fn cancel_listing(&mut self) {
        if let Some(abort) = self.listing_abort.take() {
            abort.abort();
        }
        self.listing_task.take();
    }

    fn query_changed(&mut self, cx: &mut Context<Self>) {
        self.cancel_listing();
        let query_text = self.input.read(cx).value().to_string();
        let generation = self.session.begin_query(query_text);
        let Some(root) = self
            .workspace
            .read(cx)
            .filesystem_root(cx)
            .map(PathBuf::from)
        else {
            self.session.fail(
                generation,
                "No filesystem root is available for this workspace.",
            );
            cx.notify();
            return;
        };

        let query = self.session.query().clone();
        let job = self
            .workspace
            .read(cx)
            .file_finder_listing(root, query, generation);
        self.listing_abort = Some(job.abort_handle());
        let view = cx.entity().downgrade();
        self.listing_task = Some(cx.spawn(async move |_, cx| {
            let result = job
                .await
                .map_err(|error| format!("file finder task failed: {error}"))
                .and_then(|result| result);
            let _ = view.update(cx, |view, cx| {
                if view.session.generation() != generation {
                    return;
                }
                match result {
                    Ok(result) => {
                        view.session.accept(result);
                    }
                    Err(error) => {
                        view.session.fail(generation, error);
                    }
                }
                view.listing_abort = None;
                view.listing_task = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn clear_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input
            .update(cx, |input, cx| input.set_value("", window, cx));
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.session.move_selection(delta).is_some() {
            cx.notify();
        }
    }

    fn open_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(candidate) = self.session.selected_candidate().cloned() else {
            return;
        };
        self.open_candidate(candidate, window, cx);
    }

    fn open_candidate(
        &mut self,
        candidate: FileFinderCandidate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let generation = self.session.generation();
        let result = self.workspace.update(cx, |workspace, cx| {
            workspace.open_file_finder_candidate(candidate, window, cx)
        });
        match result {
            Ok(()) => cx.emit(DismissEvent),
            Err(error) => {
                self.session.fail(generation, error);
                cx.notify();
            }
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = &event.keystroke.modifiers;
        if key == "escape" {
            cx.emit(DismissEvent);
            cx.stop_propagation();
            return;
        }
        let delta = match key {
            "up" if !modifiers.platform && !modifiers.control && !modifiers.alt => Some(-1),
            "down" if !modifiers.platform && !modifiers.control && !modifiers.alt => Some(1),
            "p" if modifiers.control && !modifiers.platform && !modifiers.alt => Some(-1),
            "n" if modifiers.control && !modifiers.platform && !modifiers.alt => Some(1),
            _ => None,
        };
        if let Some(delta) = delta {
            self.move_selection(delta, cx);
            cx.stop_propagation();
        } else if key == "enter" && !modifiers.platform && !modifiers.control && !modifiers.alt {
            self.open_selected(window, cx);
            cx.stop_propagation();
        }
    }
}

impl EventEmitter<DismissEvent> for FileFinderView {}

impl Focusable for FileFinderView {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input.read(cx).focus_handle(cx)
    }
}

impl ModalView for FileFinderView {
    fn on_dismiss(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.cancel_listing();
        self.session.cancel();
    }
}

impl Render for FileFinderView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::from_theme(self.theme.read(cx));
        let snapshot = self.session.snapshot().clone();
        let error = match &snapshot.status {
            FileFinderStatus::Error(message) => Some(message.clone()),
            _ => None,
        };
        let rows = snapshot
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let candidate = candidate.clone();
                ListItem::new(
                    ("file-finder-row", index),
                    palette.fg,
                    palette.muted,
                    palette.selected_fill,
                )
                .icon(IconName::File)
                .selected(snapshot.selected == Some(index))
                .on_click(cx.listener({
                    let row_candidate = candidate.clone();
                    move |this, _: &ClickEvent, window, cx| {
                        this.open_candidate(row_candidate.clone(), window, cx);
                    }
                }))
                .child(
                    div()
                        .min_w_0()
                        .text_sm()
                        .overflow_hidden()
                        .child(candidate.relative_path.clone()),
                )
                .into_any_element()
            })
            .collect::<Vec<_>>();
        let has_query = !snapshot.query.text.is_empty();
        let status = match snapshot.status {
            FileFinderStatus::Loading => div()
                .py_3()
                .flex()
                .justify_center()
                .text_sm()
                .text_color(palette.muted)
                .child("Loading files…")
                .into_any_element(),
            FileFinderStatus::Empty => div()
                .py_3()
                .flex()
                .justify_center()
                .text_sm()
                .text_color(palette.muted)
                .child(if has_query {
                    "No files match this query."
                } else {
                    "No files found in this workspace."
                })
                .into_any_element(),
            FileFinderStatus::Error(_) => div().into_any_element(),
            FileFinderStatus::Ready | FileFinderStatus::Idle => div()
                .id("file-finder-results")
                .max_h(px(384.0))
                .overflow_y_scroll()
                .children(rows)
                .into_any_element(),
        };
        let close_button = button(
            "file-finder-close",
            palette,
            ButtonVariant::Ghost,
            ButtonSize::IconXs,
        )
        .child("×")
        .on_click(cx.listener(|_, _: &ClickEvent, _window, cx| cx.emit(DismissEvent)));

        div()
            .id("file-finder")
            .w(px(640.0))
            .max_w(px(640.0))
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded_lg()
            .bg(palette.card)
            .border_1()
            .border_color(if error.is_some() {
                palette.error
            } else {
                palette.border
            })
            .text_color(palette.fg)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(field_input(&self.input).flex_1())
                    .when(has_query, |d| {
                        d.child(
                            button(
                                "file-finder-clear",
                                palette,
                                ButtonVariant::Ghost,
                                ButtonSize::Xs,
                            )
                            .child("Clear")
                            .on_click(cx.listener(
                                |this, _: &ClickEvent, window, cx| {
                                    this.clear_query(window, cx);
                                },
                            )),
                        )
                    })
                    .child(close_button),
            )
            .when_some(error, |d, message| {
                d.child(
                    banner(Severity::Error, palette)
                        .child(message)
                        .into_any_element(),
                )
            })
            .child(status)
            .when(snapshot.truncated, |d| {
                d.child(
                    div()
                        .text_xs()
                        .text_color(palette.muted)
                        .child("File list truncated; refine the query for more precise results."),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pt_1()
                    .border_t_1()
                    .border_color(palette.border)
                    .child(keybinding_hint("Navigate", ["↑", "↓"], palette))
                    .child(keybinding_hint("Open", ["Enter"], palette))
                    .child(keybinding_hint("Close", ["Esc"], palette)),
            )
            .on_key_down(cx.listener(Self::on_key_down))
    }
}

/// Expose the background adapter through Workspace while keeping the Tokio
/// runtime and workspace-root decision out of the modal's filesystem logic.
pub(crate) fn spawn_listing(
    tokio: &tokio::runtime::Handle,
    filesystem_root: PathBuf,
    query: FileFinderQuery,
    generation: u64,
) -> JoinHandle<Result<FileFinderResult, String>> {
    tokio.spawn_blocking(move || list_blocking(filesystem_root, query, generation))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "labonair-file-finder-{label}-{}",
            std::process::id()
        ))
    }

    #[test]
    fn adapter_lists_only_root_safe_files_and_preserves_generation() {
        let root = test_root("listing");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("test root");
        std::fs::write(root.join("README.md"), "readme").expect("test file");
        std::fs::write(root.join("src/lib.rs"), "lib").expect("test file");

        let result =
            list_blocking(root.clone(), FileFinderQuery::new("lib", 10), 42).expect("file listing");
        assert_eq!(result.generation, 42);
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].relative_path, "src/lib.rs");
        let canonical_root = std::fs::canonicalize(&root).expect("canonical test root");
        assert!(result.candidates[0].path.starts_with(canonical_root));

        std::fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn adapter_reports_an_unavailable_root_as_an_error() {
        let root = test_root("missing");
        let _ = std::fs::remove_dir_all(&root);
        let error = list_blocking(root, FileFinderQuery::default(), 1).expect_err("missing root");
        assert!(error.contains("cannot resolve workspace root"));
    }
}
