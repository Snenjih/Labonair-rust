//! [`SearchOverlay`] — the `Cmd+F` search overlay (T18-002).
//!
//! Replaces the titlebar's provisional inline search (T18-001) and the
//! editor's own in-buffer find bar with a single transient overlay hosted by
//! the [`crate::modal_layer::ModalLayer`]. It is a **bare** [`ModalView`]: the
//! layer moves keyboard focus into it but paints no scrim and does not
//! `occlude()` the rest of the window, so the active tab keeps scrolling while
//! the overlay is open.
//!
//! Routing is driven entirely by the Workspace composition adapter. The
//! Editor owns the typed query, matches, validation, and replacement
//! transactions; this overlay only owns transient form controls and forwards
//! actions through that boundary.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, ClickEvent, Context, DismissEvent, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window,
};
use labonair_editor::{
    ProjectReplaceStatus, ProjectSearchHit, ProjectSearchOptions, ProjectSearchQuery,
    ProjectSearchStatus, SearchCaseMode, SearchError, SearchOptions, SearchQuery, SearchScope,
    TextRange,
};
use labonair_settings::{EditorSettings, Settings as _};
use labonair_ui_kit::{
    button, button_disabled, search_field, search_input, text_field, text_field_surface,
    text_input, ButtonSize, ButtonVariant, InputEvent, InputState, ListItem, Palette,
    TextFieldState,
};
use std::collections::{BTreeMap, HashSet};
use std::sync::Mutex;

use crate::modal_layer::ModalView;
use crate::theme::ThemeStore;
use crate::{SearchTarget, Workspace};

/// The last query typed into any search overlay, pre-filled (and replacing an
/// editor-selection seed when none exists) the next time one opens.
static LAST_QUERY: Mutex<String> = Mutex::new(String::new());

fn last_query() -> String {
    LAST_QUERY.lock().map(|q| q.clone()).unwrap_or_default()
}

fn save_last_query(query: &str) {
    if let Ok(mut slot) = LAST_QUERY.lock() {
        *slot = query.to_string();
    }
}

fn grouped_project_hit_indices(hits: &[ProjectSearchHit]) -> Vec<(String, Vec<usize>)> {
    let mut groups = BTreeMap::<&str, Vec<usize>>::new();
    for (index, hit) in hits.iter().enumerate() {
        groups
            .entry(hit.relative_path.as_str())
            .or_default()
            .push(index);
    }
    groups
        .into_iter()
        .map(|(path, indices)| (path.to_string(), indices))
        .collect()
}

fn parse_path_globs(value: &str) -> Vec<String> {
    value
        .lines()
        .flat_map(|line| line.split(','))
        .map(str::trim)
        .filter(|pattern| !pattern.is_empty())
        .map(str::to_string)
        .collect()
}

fn next_case_mode(mode: SearchCaseMode) -> SearchCaseMode {
    match mode {
        SearchCaseMode::Smart => SearchCaseMode::Sensitive,
        SearchCaseMode::Sensitive => SearchCaseMode::Insensitive,
        SearchCaseMode::Insensitive => SearchCaseMode::Smart,
    }
}

fn replacement_actions_enabled(
    target: SearchTarget,
    scope: SearchScope,
    has_matches: bool,
    has_error: bool,
    can_edit: bool,
) -> bool {
    target == SearchTarget::Editor
        && scope == SearchScope::CurrentFile
        && has_matches
        && !has_error
        && can_edit
}

pub struct SearchOverlay {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
    input: Entity<InputState>,
    replace_input: Option<Entity<InputState>>,
    include_glob_input: Option<Entity<InputState>>,
    exclude_glob_input: Option<Entity<InputState>>,
    target: SearchTarget,
    scope: SearchScope,
    options: SearchOptions,
    selection_range: Option<TextRange>,
    collapsed_project_files: HashSet<String>,
    search_on_type: bool,
    include_ignored: bool,
    only_open_files: bool,
    show_path_filters: bool,
    project_query_confirmed: bool,
    count: (usize, usize),
    error: Option<SearchError>,
    _input_sub: Subscription,
    _replace_sub: Option<Subscription>,
    _include_glob_sub: Option<Subscription>,
    _exclude_glob_sub: Option<Subscription>,
}

impl SearchOverlay {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_settings = EditorSettings::try_get(cx).cloned();
        let target = workspace.read(cx).active_search_target(cx);
        let selection_range = (target == SearchTarget::Editor)
            .then(|| workspace.read(cx).search_selection_range(cx))
            .flatten();
        let seed = workspace
            .read(cx)
            .search_seed(cx)
            .unwrap_or_else(last_query);
        let options = search_settings
            .as_ref()
            .map(|settings| SearchOptions {
                case_sensitive: settings.search_case_sensitive(),
                smart_case: settings.search_smart_case(),
                whole_word: settings.search_whole_word(),
                regex: settings.search_regex(),
                wrap: settings.search_wrap(),
                ..SearchOptions::default()
            })
            .unwrap_or_else(|| SearchOptions {
                smart_case: false,
                ..SearchOptions::default()
            });
        let search_on_type = search_settings
            .as_ref()
            .map(EditorSettings::search_on_type)
            .unwrap_or(true);
        let include_ignored = search_settings
            .as_ref()
            .map(EditorSettings::search_include_ignored)
            .unwrap_or(false);

        let input = cx.new(|cx| {
            text_field(window, cx)
                .placeholder("Search\u{2026}")
                .default_value(seed.clone())
        });
        let input_sub = cx.subscribe(
            &input,
            |this: &mut Self, _input, ev: &InputEvent, cx| match ev {
                InputEvent::Change => this.run_search(cx),
                InputEvent::PressEnter { secondary } if this.scope == SearchScope::CurrentFile => {
                    this.step(!secondary, cx)
                }
                _ => {}
            },
        );

        let (include_glob_input, include_glob_sub) = if target == SearchTarget::Editor {
            let input =
                cx.new(|cx| text_field(window, cx).placeholder("Include globs (comma-separated)…"));
            let sub = cx.subscribe(&input, |this: &mut Self, _input, ev: &InputEvent, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.run_search(cx);
                }
            });
            (Some(input), Some(sub))
        } else {
            (None, None)
        };
        let (exclude_glob_input, exclude_glob_sub) = if target == SearchTarget::Editor {
            let input =
                cx.new(|cx| text_field(window, cx).placeholder("Exclude globs (comma-separated)…"));
            let sub = cx.subscribe(&input, |this: &mut Self, _input, ev: &InputEvent, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.run_search(cx);
                }
            });
            (Some(input), Some(sub))
        } else {
            (None, None)
        };

        let (replace_input, replace_sub) = if target == SearchTarget::Editor {
            let replace_input = cx.new(|cx| text_field(window, cx).placeholder("Replace…"));
            let replace_sub = cx.subscribe(
                &replace_input,
                |this: &mut Self, _input, ev: &InputEvent, cx| {
                    if let InputEvent::PressEnter { secondary } = ev {
                        if *secondary {
                            this.replace_all(cx);
                        } else {
                            this.replace_one(cx);
                        }
                    }
                },
            );
            (Some(replace_input), Some(replace_sub))
        } else {
            (None, None)
        };

        let mut this = Self {
            workspace,
            theme,
            input,
            replace_input,
            include_glob_input,
            exclude_glob_input,
            target,
            scope: SearchScope::CurrentFile,
            options,
            selection_range,
            collapsed_project_files: HashSet::new(),
            search_on_type,
            include_ignored,
            only_open_files: false,
            show_path_filters: false,
            project_query_confirmed: false,
            count: (0, 0),
            error: None,
            _input_sub: input_sub,
            _replace_sub: replace_sub,
            _include_glob_sub: include_glob_sub,
            _exclude_glob_sub: exclude_glob_sub,
        };
        if !seed.is_empty() && target != SearchTarget::Unavailable {
            this.run_search(cx);
        }
        this
    }

    fn run_search(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        save_last_query(&text);
        if self.target == SearchTarget::Editor && self.scope == SearchScope::Project {
            if self.search_on_type {
                self.run_project_search(text, cx);
            } else {
                self.clear_project_search(cx);
            }
            return;
        }
        let selection_range = self.workspace.read(cx).search_selection_scope(cx);
        let query = SearchQuery::new(text, self.options).with_selection_range(selection_range);
        let result = self.workspace.update(cx, |w, cx| w.search_set(query, cx));
        match result {
            Some(Ok(count)) => {
                self.count = count;
                self.error = None;
            }
            Some(Err(error)) => {
                self.count = (0, 0);
                self.error = Some(error);
            }
            None => {
                self.count = (0, 0);
                self.error = None;
            }
        }
        cx.notify();
    }

    fn run_project_search(&mut self, text: String, cx: &mut Context<Self>) {
        let mut options = ProjectSearchOptions::from_search_options(self.options);
        options.include_ignored = self.include_ignored;
        options.only_open_files = self.only_open_files;
        let include_globs = self
            .include_glob_input
            .as_ref()
            .map(|input| parse_path_globs(&input.read(cx).value()))
            .unwrap_or_default();
        let exclude_globs = self
            .exclude_glob_input
            .as_ref()
            .map(|input| parse_path_globs(&input.read(cx).value()))
            .unwrap_or_default();
        let query = ProjectSearchQuery::new(text, options, ProjectSearchQuery::DEFAULT_MAX_RESULTS)
            .with_path_filters(include_globs, exclude_globs);
        let _ = self
            .workspace
            .update(cx, |workspace, cx| workspace.project_search_set(query, cx));
        self.project_query_confirmed = !self.input.read(cx).value().is_empty();
        self.count = (0, 0);
        self.error = None;
        cx.notify();
    }

    fn clear_project_search(&mut self, cx: &mut Context<Self>) {
        let mut options = ProjectSearchOptions::from_search_options(self.options);
        options.include_ignored = self.include_ignored;
        options.only_open_files = self.only_open_files;
        let query = ProjectSearchQuery::new("", options, ProjectSearchQuery::DEFAULT_MAX_RESULTS);
        let _ = self
            .workspace
            .update(cx, |workspace, cx| workspace.project_search_set(query, cx));
        self.project_query_confirmed = false;
        self.count = (0, 0);
        self.error = None;
        cx.notify();
    }

    fn confirm_project_search(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        self.run_project_search(text, cx);
    }

    fn set_scope(&mut self, scope: SearchScope, cx: &mut Context<Self>) {
        if self.scope == scope {
            return;
        }
        self.scope = scope;
        self.error = None;
        self.run_search(cx);
    }

    fn step(&mut self, forward: bool, cx: &mut Context<Self>) {
        if self.scope == SearchScope::Project && self.target == SearchTarget::Editor {
            self.move_project_search(if forward { 1 } else { -1 }, cx);
            return;
        }
        match self
            .workspace
            .update(cx, |w, cx| w.search_step(forward, cx))
        {
            Some(Ok(count)) => {
                self.count = count;
                self.error = None;
            }
            Some(Err(error)) => self.error = Some(error),
            None => self.count = (0, 0),
        }
        cx.notify();
    }

    fn move_project_search(&mut self, delta: isize, cx: &mut Context<Self>) {
        let hit = self
            .workspace
            .update(cx, |workspace, cx| workspace.project_search_move(delta, cx));
        if let Some(hit) = hit {
            self.collapsed_project_files.remove(&hit.relative_path);
        }
        cx.notify();
    }

    fn toggle_case(&mut self, cx: &mut Context<Self>) {
        let mode = next_case_mode(self.options.case_mode());
        self.options = self.options.with_case_mode(mode);
        self.run_search(cx);
    }

    fn toggle_option(&mut self, option: impl FnOnce(&mut SearchOptions), cx: &mut Context<Self>) {
        option(&mut self.options);
        self.run_search(cx);
    }

    fn toggle_selection_scope(&mut self, cx: &mut Context<Self>) {
        let toggled = self.workspace.update(cx, |workspace, cx| {
            workspace.toggle_search_selection_scope(cx)
        });
        if toggled {
            self.run_search(cx);
        }
    }

    fn toggle_include_ignored(&mut self, cx: &mut Context<Self>) {
        self.include_ignored = !self.include_ignored;
        self.run_search(cx);
    }

    fn toggle_only_open_files(&mut self, cx: &mut Context<Self>) {
        self.only_open_files = !self.only_open_files;
        self.run_search(cx);
    }

    fn toggle_path_filters(&mut self, cx: &mut Context<Self>) {
        self.show_path_filters = !self.show_path_filters;
        cx.notify();
    }

    fn toggle_project_file(&mut self, path: String, cx: &mut Context<Self>) {
        if !self.collapsed_project_files.remove(&path) {
            self.collapsed_project_files.insert(path);
        }
        cx.notify();
    }

    fn collapse_all_project_files(&mut self, cx: &mut Context<Self>) {
        self.collapsed_project_files = self
            .workspace
            .read(cx)
            .project_search_snapshot(cx)
            .map(|snapshot| {
                grouped_project_hit_indices(&snapshot.hits)
                    .into_iter()
                    .map(|(path, _)| path)
                    .collect()
            })
            .unwrap_or_default();
        cx.notify();
    }

    fn expand_all_project_files(&mut self, cx: &mut Context<Self>) {
        self.collapsed_project_files.clear();
        cx.notify();
    }

    fn replace_one(&mut self, cx: &mut Context<Self>) {
        if self.scope == SearchScope::Project {
            return;
        }
        let Some(replace_input) = &self.replace_input else {
            return;
        };
        let replacement = replace_input.read(cx).value().to_string();
        match self
            .workspace
            .update(cx, |w, cx| w.search_replace_one(&replacement, cx))
        {
            Some(Ok(_)) => self.run_search(cx),
            Some(Err(error)) => {
                self.error = Some(error);
                cx.notify();
            }
            None => cx.notify(),
        }
    }

    fn replace_all(&mut self, cx: &mut Context<Self>) {
        let Some(replace_input) = &self.replace_input else {
            return;
        };
        let replacement = replace_input.read(cx).value().to_string();
        if self.target == SearchTarget::Editor && self.scope == SearchScope::Project {
            match self.workspace.update(cx, |workspace, cx| {
                workspace.project_replace_all(&replacement, cx)
            }) {
                Some(Ok(_)) => {
                    self.error = None;
                    cx.notify();
                }
                Some(Err(_error)) => {
                    self.error = None;
                    cx.notify();
                }
                _ => cx.notify(),
            }
            return;
        }
        match self
            .workspace
            .update(cx, |w, cx| w.search_replace_all(&replacement, cx))
        {
            Some(Ok(_)) => self.run_search(cx),
            Some(Err(error)) => {
                self.error = Some(error);
                cx.notify();
            }
            None => cx.notify(),
        }
    }

    fn open_selected_project_result(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.workspace.read(cx).project_search_snapshot(cx) else {
            return;
        };
        let Some(index) = snapshot.selected else {
            return;
        };
        let Some(hit) = snapshot.hits.get(index).cloned() else {
            return;
        };
        self.workspace.update(cx, |workspace, cx| {
            workspace.open_project_search_hit(hit, window, cx)
        });
    }
}

impl EventEmitter<DismissEvent> for SearchOverlay {}

impl Focusable for SearchOverlay {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input.read(cx).focus_handle(cx)
    }
}

impl ModalView for SearchOverlay {
    fn on_dismiss(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.search_end(cx));
    }

    /// No backdrop, no centering — a small box over the top-right of the
    /// active tab content that does not steal mouse input from it.
    fn render_bare(&self) -> bool {
        true
    }
}

impl Render for SearchOverlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let palette = Palette::from_theme(theme);
        let unavailable = self.target == SearchTarget::Unavailable;
        let editor = self.target == SearchTarget::Editor;
        let project_snapshot = (editor && self.scope == SearchScope::Project)
            .then(|| self.workspace.read(cx).project_search_snapshot(cx))
            .flatten();
        let selection_scoped =
            editor && self.workspace.read(cx).search_selection_scope(cx).is_some();
        let buffer_count = (editor && self.scope == SearchScope::CurrentFile)
            .then(|| self.workspace.read(cx).buffer_search_count(cx))
            .flatten();
        let project_error = project_snapshot.as_ref().and_then(|snapshot| {
            if let ProjectSearchStatus::Error(message) = &snapshot.status {
                Some(message.clone())
            } else {
                None
            }
        });
        let project_count = project_snapshot.as_ref().map(|snapshot| {
            (
                snapshot.selected.map_or(0, |index| index + 1),
                snapshot.hits.len(),
            )
        });
        let count = project_count.or(buffer_count).unwrap_or(self.count);
        let has_error = self.error.is_some() || project_error.is_some();
        let has_matches = count.1 > 0 && !has_error;
        let can_replace_file = replacement_actions_enabled(
            self.target,
            self.scope,
            has_matches,
            has_error,
            self.workspace.read(cx).search_can_replace(cx),
        );
        let can_replace_project = project_snapshot.as_ref().is_some_and(|snapshot| {
            snapshot.status == ProjectSearchStatus::Ready
                && !snapshot.truncated
                && !snapshot.hits.is_empty()
                && !matches!(&snapshot.replacement_status, ProjectReplaceStatus::Applying)
                && !has_error
        });
        let can_replace_one = can_replace_file;
        let can_replace_all = match self.scope {
            SearchScope::CurrentFile => can_replace_file,
            SearchScope::Project => can_replace_project,
        };
        let (cur, total) = count;
        let count_label = if has_error {
            "Invalid pattern".to_string()
        } else if total == 0 {
            "No results".to_string()
        } else {
            format!("{cur}/{total}")
        };
        let case_label = match self.options.case_mode() {
            SearchCaseMode::Sensitive => "Case",
            SearchCaseMode::Insensitive => "case",
            SearchCaseMode::Smart => "Smart",
        };
        let error = self
            .error
            .as_ref()
            .map(|error| error.message.clone())
            .or(project_error);
        let project_loading = project_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.status == ProjectSearchStatus::Loading);
        let project_query_pending = editor
            && self.scope == SearchScope::Project
            && !self.search_on_type
            && !self.project_query_confirmed
            && !self.input.read(cx).value().is_empty();
        let project_truncated = project_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.truncated);
        let project_replace_message = project_snapshot.as_ref().and_then(|snapshot| {
            match &snapshot.replacement_status {
                ProjectReplaceStatus::Idle => None,
                ProjectReplaceStatus::Applying => Some("Replacing open files…".to_string()),
                ProjectReplaceStatus::Complete {
                    files_changed: 0, ..
                } => Some("No matching open files to replace; closed files were unchanged.".to_string()),
                ProjectReplaceStatus::Complete {
                    files_changed,
                    replacements,
                } => Some(format!(
                    "Updated {replacements} matches in {files_changed} open files. Changes are unsaved and undoable; closed files were unchanged."
                )),
                ProjectReplaceStatus::Error(message) => Some(message.clone()),
            }
        });
        let project_rows = project_snapshot.as_ref().map(|snapshot| {
            let mut rows = Vec::new();
            for (path, indices) in grouped_project_hit_indices(&snapshot.hits) {
                let collapsed = self.collapsed_project_files.contains(&path);
                let toggle_path = path.clone();
                rows.push(
                    div()
                        .id(SharedString::from(format!("project-search-file-{path}")))
                        .flex()
                        .items_center()
                        .gap_1()
                        .px_1()
                        .py_1()
                        .cursor_pointer()
                        .hover(|style| style.bg(palette.accent))
                        .child(if collapsed { "▸" } else { "▾" })
                        .child(div().flex_1().text_xs().text_color(palette.fg).child(path))
                        .child(
                            div()
                                .text_xs()
                                .text_color(palette.muted)
                                .child(format!("{}", indices.len())),
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                            this.toggle_project_file(toggle_path.clone(), cx)
                        }))
                        .into_any_element(),
                );
                if collapsed {
                    continue;
                }
                for index in indices {
                    let hit = &snapshot.hits[index];
                    let label = format!("{}:{}", hit.relative_path, hit.line);
                    let detail = hit.text.clone();
                    rows.push(
                        ListItem::new(
                            ("project-search-result", index),
                            palette.fg,
                            palette.muted,
                            palette.selected_fill,
                        )
                        .selected(snapshot.selected == Some(index))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                            let _ = this.workspace.update(cx, |workspace, cx| {
                                workspace.project_search_move(
                                    index as isize
                                        - workspace
                                            .project_search_snapshot(cx)
                                            .and_then(|s| s.selected)
                                            .unwrap_or(0)
                                            as isize,
                                    cx,
                                )
                            });
                        }))
                        .child(
                            div().flex().flex_col().child(label).child(
                                div()
                                    .text_xs()
                                    .text_color(palette.muted)
                                    .overflow_hidden()
                                    .child(detail),
                            ),
                        )
                        .into_any_element(),
                    );
                }
            }
            rows
        });
        let replace_input = self.replace_input.clone();
        let include_glob_input = self.include_glob_input.clone();
        let exclude_glob_input = self.exclude_glob_input.clone();
        let search_focused = self.input.read(cx).focus_handle(cx).is_focused(window);
        let replace_focused = replace_input
            .as_ref()
            .is_some_and(|input| input.read(cx).focus_handle(cx).is_focused(window));
        let include_glob_focused = include_glob_input
            .as_ref()
            .is_some_and(|input| input.read(cx).focus_handle(cx).is_focused(window));
        let exclude_glob_focused = exclude_glob_input
            .as_ref()
            .is_some_and(|input| input.read(cx).focus_handle(cx).is_focused(window));
        let include_glob_field = include_glob_input
            .as_ref()
            .map(|input| {
                search_field(
                    "search-include-globs",
                    palette,
                    include_glob_focused,
                    search_input(input, palette),
                    None,
                )
                .w(px(260.0))
                .into_any_element()
            })
            .unwrap_or_else(|| div().into_any_element());
        let exclude_glob_field = exclude_glob_input
            .as_ref()
            .map(|input| {
                search_field(
                    "search-exclude-globs",
                    palette,
                    exclude_glob_focused,
                    search_input(input, palette),
                    None,
                )
                .w(px(260.0))
                .into_any_element()
            })
            .unwrap_or_else(|| div().into_any_element());
        let previous_button = if has_matches {
            button(
                "search-prev",
                palette,
                ButtonVariant::Subtle,
                ButtonSize::Xs,
            )
            .child("↑")
            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                if this.count.1 > 0 && this.error.is_none() {
                    this.step(false, cx)
                }
            }))
        } else {
            button_disabled(
                "search-prev",
                palette,
                ButtonVariant::Subtle,
                ButtonSize::Xs,
            )
            .child("↑")
        };
        let next_button = if has_matches {
            button(
                "search-next",
                palette,
                ButtonVariant::Subtle,
                ButtonSize::Xs,
            )
            .child("↓")
            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                if this.count.1 > 0 && this.error.is_none() {
                    this.step(true, cx)
                }
            }))
        } else {
            button_disabled(
                "search-next",
                palette,
                ButtonVariant::Subtle,
                ButtonSize::Xs,
            )
            .child("↓")
        };
        let replace_one_button = if can_replace_one {
            button(
                "search-replace-one",
                palette,
                ButtonVariant::Outlined,
                ButtonSize::Xs,
            )
            .child("Replace")
            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                if this.count.1 > 0 && this.error.is_none() {
                    this.replace_one(cx)
                }
            }))
        } else {
            button_disabled(
                "search-replace-one",
                palette,
                ButtonVariant::Outlined,
                ButtonSize::Xs,
            )
            .child("Replace")
        };
        let replace_all_button = if can_replace_all {
            button(
                "search-replace-all",
                palette,
                ButtonVariant::OutlinedGhost,
                ButtonSize::Xs,
            )
            .child(if self.scope == SearchScope::Project {
                "Open"
            } else {
                "All"
            })
            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                let has_actionable_matches = this.scope == SearchScope::Project || this.count.1 > 0;
                if has_actionable_matches && this.error.is_none() {
                    this.replace_all(cx)
                }
            }))
        } else {
            button_disabled(
                "search-replace-all",
                palette,
                ButtonVariant::OutlinedGhost,
                ButtonSize::Xs,
            )
            .child(if self.scope == SearchScope::Project {
                "Open"
            } else {
                "All"
            })
        };
        div()
            .id("search-overlay")
            .absolute()
            .top(px(44.0))
            .right(px(16.0))
            .occlude()
            .flex_col()
            .gap_1()
            .px_2()
            .py_1p5()
            .rounded(px(palette.radius.md))
            .bg(palette.card)
            .border_1()
            .border_color(if error.is_some() {
                palette.error
            } else {
                palette.border
            })
            .when(unavailable, |d| {
                d.child(
                    div()
                        .text_xs()
                        .text_color(palette.muted)
                        .child("Search is not available for this tab"),
                )
            })
            .when(!unavailable, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            search_field(
                                "search-overlay-query",
                                palette,
                                search_focused,
                                search_input(&self.input, palette),
                                None,
                            )
                            .w(px(248.0)),
                        )
                        .child(
                            button(
                                "search-case",
                                palette,
                                ButtonVariant::Subtle,
                                ButtonSize::Xs,
                            )
                            .child(case_label)
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.toggle_case(cx)),
                            ),
                        )
                        .when(editor, |d| {
                            d.child(
                                button(
                                    "search-scope-file",
                                    palette,
                                    ButtonVariant::Subtle,
                                    ButtonSize::Xs,
                                )
                                .child("File")
                                .when(self.scope == SearchScope::CurrentFile, |d| {
                                    d.bg(palette.selected_fill)
                                        .text_color(palette.selected_accent)
                                })
                                .on_click(cx.listener(
                                    |this, _: &ClickEvent, _w, cx| {
                                        this.set_scope(SearchScope::CurrentFile, cx)
                                    },
                                )),
                            )
                            .child(
                                button(
                                    "search-scope-project",
                                    palette,
                                    ButtonVariant::Subtle,
                                    ButtonSize::Xs,
                                )
                                .child("Project")
                                .when(self.scope == SearchScope::Project, |d| {
                                    d.bg(palette.selected_fill)
                                        .text_color(palette.selected_accent)
                                })
                                .on_click(cx.listener(
                                    |this, _: &ClickEvent, _w, cx| {
                                        this.set_scope(SearchScope::Project, cx)
                                    },
                                )),
                            )
                            .when(self.scope == SearchScope::Project, |d| {
                                d.child(
                                    button(
                                        "search-include-ignored",
                                        palette,
                                        ButtonVariant::Subtle,
                                        ButtonSize::Xs,
                                    )
                                    .child("Ignored")
                                    .when(self.include_ignored, |d| {
                                        d.bg(palette.selected_fill)
                                            .text_color(palette.selected_accent)
                                    })
                                    .on_click(cx.listener(
                                        |this, _: &ClickEvent, _w, cx| {
                                            this.toggle_include_ignored(cx)
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "search-only-open-files",
                                        palette,
                                        ButtonVariant::Subtle,
                                        ButtonSize::Xs,
                                    )
                                    .child("Open files")
                                    .when(self.only_open_files, |d| {
                                        d.bg(palette.selected_fill)
                                            .text_color(palette.selected_accent)
                                    })
                                    .on_click(cx.listener(
                                        |this, _: &ClickEvent, _w, cx| {
                                            this.toggle_only_open_files(cx)
                                        },
                                    )),
                                )
                                .child(
                                    button(
                                        "search-path-filters-toggle",
                                        palette,
                                        ButtonVariant::Subtle,
                                        ButtonSize::Xs,
                                    )
                                    .child("Filters")
                                    .when(self.show_path_filters, |d| {
                                        d.bg(palette.selected_fill)
                                            .text_color(palette.selected_accent)
                                    })
                                    .on_click(cx.listener(
                                        |this, _: &ClickEvent, _w, cx| this.toggle_path_filters(cx),
                                    )),
                                )
                            })
                            .when(
                                self.scope == SearchScope::CurrentFile
                                    && self.selection_range.is_some(),
                                |d| {
                                    d.child(
                                        button(
                                            "search-selection-scope",
                                            palette,
                                            ButtonVariant::Subtle,
                                            ButtonSize::Xs,
                                        )
                                        .child("Selection")
                                        .when(selection_scoped, |d| {
                                            d.bg(palette.selected_fill)
                                                .text_color(palette.selected_accent)
                                        })
                                        .on_click(
                                            cx.listener(|this, _: &ClickEvent, _w, cx| {
                                                this.toggle_selection_scope(cx)
                                            }),
                                        ),
                                    )
                                },
                            )
                            .when(
                                matches!(
                                    self.scope,
                                    SearchScope::CurrentFile | SearchScope::Project
                                ),
                                |d| {
                                    d.child(
                                        button(
                                            "search-regex",
                                            palette,
                                            ButtonVariant::Subtle,
                                            ButtonSize::Xs,
                                        )
                                        .child(".*")
                                        .when(self.options.regex, |d| {
                                            d.bg(palette.selected_fill)
                                                .text_color(palette.selected_accent)
                                        })
                                        .on_click(
                                            cx.listener(|this, _: &ClickEvent, _w, cx| {
                                                this.toggle_option(
                                                    |options| options.regex = !options.regex,
                                                    cx,
                                                )
                                            }),
                                        ),
                                    )
                                },
                            )
                            .when(
                                matches!(
                                    self.scope,
                                    SearchScope::CurrentFile | SearchScope::Project
                                ),
                                |d| {
                                    d.child(
                                        button(
                                            "search-word",
                                            palette,
                                            ButtonVariant::Subtle,
                                            ButtonSize::Xs,
                                        )
                                        .child("Word")
                                        .when(self.options.whole_word, |d| {
                                            d.bg(palette.selected_fill)
                                                .text_color(palette.selected_accent)
                                        })
                                        .on_click(
                                            cx.listener(|this, _: &ClickEvent, _w, cx| {
                                                this.toggle_option(
                                                    |options| {
                                                        options.whole_word = !options.whole_word
                                                    },
                                                    cx,
                                                )
                                            }),
                                        ),
                                    )
                                },
                            )
                            .when(self.scope == SearchScope::CurrentFile, |d| {
                                d.child(
                                    button(
                                        "search-multiline",
                                        palette,
                                        ButtonVariant::Subtle,
                                        ButtonSize::Xs,
                                    )
                                    .child("Multi")
                                    .when(self.options.multiline, |d| {
                                        d.bg(palette.selected_fill)
                                            .text_color(palette.selected_accent)
                                    })
                                    .on_click(cx.listener(
                                        |this, _: &ClickEvent, _w, cx| {
                                            this.toggle_option(
                                                |options| options.multiline = !options.multiline,
                                                cx,
                                            )
                                        },
                                    )),
                                )
                            })
                            .when(
                                self.scope == SearchScope::CurrentFile,
                                |d| {
                                    d.child(
                                        button(
                                            "search-wrap",
                                            palette,
                                            ButtonVariant::Subtle,
                                            ButtonSize::Xs,
                                        )
                                        .child("Wrap")
                                        .when(self.options.wrap, |d| {
                                            d.bg(palette.selected_fill)
                                                .text_color(palette.selected_accent)
                                        })
                                        .on_click(
                                            cx.listener(|this, _: &ClickEvent, _w, cx| {
                                                this.toggle_option(
                                                    |options| options.wrap = !options.wrap,
                                                    cx,
                                                )
                                            }),
                                        ),
                                    )
                                },
                            )
                        })
                        .child(
                            div()
                                .min_w(px(76.0))
                                .text_xs()
                                .text_color(if error.is_some() {
                                    palette.error
                                } else {
                                    palette.muted
                                })
                                .child(count_label.clone()),
                        )
                        .child(previous_button)
                        .child(next_button),
                )
            })
            .when(editor, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            replace_input
                                .as_ref()
                                .map(|input| {
                                    text_field_surface(
                                        "search-overlay-replace-input",
                                        palette,
                                        if replace_focused {
                                            TextFieldState::Focused
                                        } else {
                                            TextFieldState::Normal
                                        },
                                        text_input(input, palette),
                                    )
                                    .w(px(220.0))
                                    .into_any_element()
                                })
                                .unwrap_or_else(|| div().into_any_element()),
                        )
                        .child(replace_one_button)
                        .child(replace_all_button),
                )
            })
            .when(self.scope == SearchScope::Project, |d| {
                d.when(self.show_path_filters, |d| {
                    d.child(
                        div()
                            .flex()
                            .gap_1()
                            .child(include_glob_field)
                            .child(exclude_glob_field),
                    )
                })
                .child(
                    div()
                        .flex()
                        .gap_1()
                        .child(
                            button(
                                "project-search-collapse-all",
                                palette,
                                ButtonVariant::Subtle,
                                ButtonSize::Xs,
                            )
                            .child("Collapse all")
                            .on_click(cx.listener(
                                |this, _: &ClickEvent, _window, cx| {
                                    this.collapse_all_project_files(cx)
                                },
                            )),
                        )
                        .child(
                            button(
                                "project-search-expand-all",
                                palette,
                                ButtonVariant::Subtle,
                                ButtonSize::Xs,
                            )
                            .child("Expand all")
                            .on_click(cx.listener(
                                |this, _: &ClickEvent, _window, cx| {
                                    this.expand_all_project_files(cx)
                                },
                            )),
                        ),
                )
                .child(
                    div()
                        .id("project-search-results")
                        .max_h(px(260.0))
                        .overflow_y_scroll()
                        .when(project_loading, |d| {
                            d.child(
                                div()
                                    .text_xs()
                                    .text_color(palette.muted)
                                    .child("Searching project…"),
                            )
                        })
                        .when(project_query_pending, |d| {
                            d.child(
                                div()
                                    .text_xs()
                                    .text_color(palette.muted)
                                    .child("Press Enter to search"),
                            )
                        })
                        .when(
                            !project_query_pending
                                && !project_loading
                                && total == 0
                                && error.is_none(),
                            |d| {
                                d.child(
                                    div()
                                        .text_xs()
                                        .text_color(palette.muted)
                                        .child("No project results"),
                                )
                            },
                        )
                        .when_some(project_rows, |d, rows| d.children(rows)),
                )
                .when(project_truncated, |d| {
                    d.child(
                        div()
                            .text_xs()
                            .text_color(palette.muted)
                            .child("Results truncated; refine the query."),
                    )
                })
                .when_some(project_replace_message, |d, message| {
                    d.child(
                        div()
                            .text_xs()
                            .text_color(
                                if matches!(
                                    project_snapshot
                                        .as_ref()
                                        .map(|snapshot| &snapshot.replacement_status),
                                    Some(ProjectReplaceStatus::Error(_))
                                ) {
                                    palette.error
                                } else {
                                    palette.muted
                                },
                            )
                            .child(message),
                    )
                })
            })
            .when(error.is_some(), |d| {
                d.child(
                    div()
                        .text_xs()
                        .text_color(palette.error)
                        .child(error.unwrap_or_default()),
                )
            })
            .child(
                button(
                    "search-close",
                    palette,
                    ButtonVariant::Subtle,
                    ButtonSize::IconXs,
                )
                .child("×")
                .on_click(cx.listener(|_this, _: &ClickEvent, _w, cx| cx.emit(DismissEvent))),
            )
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                let query_focused = this.input.read(cx).focus_handle(cx).is_focused(window);
                let include_focused = this
                    .include_glob_input
                    .as_ref()
                    .is_some_and(|input| input.read(cx).focus_handle(cx).is_focused(window));
                let exclude_focused = this
                    .exclude_glob_input
                    .as_ref()
                    .is_some_and(|input| input.read(cx).focus_handle(cx).is_focused(window));
                let search_controls_focused = query_focused || include_focused || exclude_focused;
                if ev.keystroke.key == "escape" {
                    cx.emit(DismissEvent);
                    cx.stop_propagation();
                } else if this.scope == SearchScope::Project
                    && this.target == SearchTarget::Editor
                    && ev.keystroke.key == "up"
                    && !search_controls_focused
                {
                    this.move_project_search(-1, cx);
                    cx.stop_propagation();
                } else if this.scope == SearchScope::Project
                    && this.target == SearchTarget::Editor
                    && ev.keystroke.key == "down"
                    && !search_controls_focused
                {
                    this.move_project_search(1, cx);
                    cx.stop_propagation();
                } else if this.scope == SearchScope::Project
                    && this.target == SearchTarget::Editor
                    && ev.keystroke.key == "enter"
                {
                    if search_controls_focused && !this.search_on_type {
                        this.confirm_project_search(cx);
                    } else if !include_focused && !exclude_focused {
                        this.open_selected_project_result(window, cx);
                    }
                    cx.stop_propagation();
                } else if ev.keystroke.key == "enter" && ev.keystroke.modifiers.platform {
                    if ev.keystroke.modifiers.shift {
                        this.replace_all(cx);
                    } else {
                        this.replace_one(cx);
                    }
                    cx.stop_propagation();
                }
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_toggle_cycles_smart_sensitive_and_insensitive() {
        assert_eq!(
            next_case_mode(SearchCaseMode::Smart),
            SearchCaseMode::Sensitive
        );
        assert_eq!(
            next_case_mode(SearchCaseMode::Sensitive),
            SearchCaseMode::Insensitive
        );
        assert_eq!(
            next_case_mode(SearchCaseMode::Insensitive),
            SearchCaseMode::Smart
        );
    }

    #[test]
    fn path_glob_fields_accept_comma_or_line_separated_patterns() {
        assert_eq!(
            parse_path_globs(" src/**/*.rs, *.toml\n  Cargo.lock  \n\n"),
            ["src/**/*.rs", "*.toml", "Cargo.lock"]
        );
        assert!(parse_path_globs(" , \n  ").is_empty());
    }

    #[test]
    fn project_hits_group_by_relative_path_without_losing_flat_selection_indices() {
        let hits = [
            ProjectSearchHit::with_relative_path("/root/z.rs", "src/z.rs", 2, "hit", 0, 3),
            ProjectSearchHit::with_relative_path("/root/a.rs", "src/a.rs", 4, "hit", 0, 3),
            ProjectSearchHit::with_relative_path("/root/z.rs", "src/z.rs", 8, "hit", 0, 3),
        ];

        assert_eq!(
            grouped_project_hit_indices(&hits),
            vec![
                ("src/a.rs".to_string(), vec![1]),
                ("src/z.rs".to_string(), vec![0, 2]),
            ]
        );
    }

    #[test]
    fn replacement_actions_are_only_enabled_for_valid_editable_matches() {
        assert!(replacement_actions_enabled(
            SearchTarget::Editor,
            SearchScope::CurrentFile,
            true,
            false,
            true
        ));
        assert!(!replacement_actions_enabled(
            SearchTarget::Editor,
            SearchScope::CurrentFile,
            false,
            false,
            true
        ));
        assert!(!replacement_actions_enabled(
            SearchTarget::Editor,
            SearchScope::CurrentFile,
            true,
            true,
            true
        ));
        assert!(!replacement_actions_enabled(
            SearchTarget::Editor,
            SearchScope::CurrentFile,
            true,
            false,
            false
        ));
        assert!(!replacement_actions_enabled(
            SearchTarget::Terminal,
            SearchScope::CurrentFile,
            true,
            false,
            true
        ));
    }
}
