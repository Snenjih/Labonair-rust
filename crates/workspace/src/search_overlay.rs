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
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Window,
};
use labonair_editor::{
    ProjectSearchOptions, ProjectSearchQuery, ProjectSearchStatus, SearchCaseMode, SearchError,
    SearchOptions, SearchQuery, SearchScope,
};
use labonair_ui_kit::{
    button, field_input, text_field, ButtonSize, ButtonVariant, InputEvent, InputState, ListItem,
    Palette, DISABLED_OPACITY,
};
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
    target: SearchTarget,
    scope: SearchScope,
    options: SearchOptions,
    count: (usize, usize),
    error: Option<SearchError>,
    _input_sub: Subscription,
    _replace_sub: Option<Subscription>,
}

impl SearchOverlay {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let target = workspace.read(cx).active_search_target(cx);
        let seed = workspace
            .read(cx)
            .search_seed(cx)
            .unwrap_or_else(last_query);

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
            target,
            scope: SearchScope::CurrentFile,
            options: SearchOptions::default(),
            count: (0, 0),
            error: None,
            _input_sub: input_sub,
            _replace_sub: replace_sub,
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
            self.run_project_search(text, cx);
            return;
        }
        let query = SearchQuery::new(text, self.options);
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
        let options = ProjectSearchOptions::from_search_options(self.options);
        let query = ProjectSearchQuery::new(text, options, ProjectSearchQuery::DEFAULT_MAX_RESULTS);
        let _ = self
            .workspace
            .update(cx, |workspace, cx| workspace.project_search_set(query, cx));
        self.error = None;
        cx.notify();
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
            let _ = self.workspace.update(cx, |workspace, cx| {
                workspace.project_search_move(if forward { 1 } else { -1 }, cx)
            });
            cx.notify();
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

    fn toggle_case(&mut self, cx: &mut Context<Self>) {
        let mode = next_case_mode(self.options.case_mode());
        self.options = self.options.with_case_mode(mode);
        self.run_search(cx);
    }

    fn toggle_option(&mut self, option: impl FnOnce(&mut SearchOptions), cx: &mut Context<Self>) {
        option(&mut self.options);
        self.run_search(cx);
    }

    fn replace_one(&mut self, cx: &mut Context<Self>) {
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let palette = Palette::from_theme(theme);
        let unavailable = self.target == SearchTarget::Unavailable;
        let editor = self.target == SearchTarget::Editor;
        let project_snapshot = (editor && self.scope == SearchScope::Project)
            .then(|| self.workspace.read(cx).project_search_snapshot(cx))
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
        let count = project_count.unwrap_or(self.count);
        let has_error = self.error.is_some() || project_error.is_some();
        let has_matches = count.1 > 0 && !has_error;
        let can_replace = replacement_actions_enabled(
            self.target,
            self.scope,
            has_matches,
            has_error,
            self.workspace.read(cx).search_can_replace(cx),
        );
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
        let project_truncated = project_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.truncated);
        let project_rows = project_snapshot.as_ref().map(|snapshot| {
            snapshot
                .hits
                .iter()
                .enumerate()
                .map(|(index, hit)| {
                    let label = format!("{}:{}", hit.relative_path, hit.line);
                    let detail = hit.text.clone();
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
                    .into_any_element()
                })
                .collect::<Vec<_>>()
        });
        let replace_input = self.replace_input.clone();
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
                        .child(field_input(&self.input).w(px(220.0)))
                        .child(
                            button("search-case", palette, ButtonVariant::Ghost, ButtonSize::Xs)
                                .child(case_label)
                                .on_click(
                                    cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.toggle_case(cx)
                                    }),
                                ),
                        )
                        .when(editor, |d| {
                            d.child(
                                button(
                                    "search-scope-file",
                                    palette,
                                    ButtonVariant::Ghost,
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
                                    ButtonVariant::Ghost,
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
                            .when(self.scope == SearchScope::CurrentFile, |d| {
                                d.child(
                                    button(
                                        "search-regex",
                                        palette,
                                        ButtonVariant::Ghost,
                                        ButtonSize::Xs,
                                    )
                                    .child(".*")
                                    .when(self.options.regex, |d| {
                                        d.bg(palette.selected_fill)
                                            .text_color(palette.selected_accent)
                                    })
                                    .on_click(cx.listener(
                                        |this, _: &ClickEvent, _w, cx| {
                                            this.toggle_option(
                                                |options| options.regex = !options.regex,
                                                cx,
                                            )
                                        },
                                    )),
                                )
                            })
                            .when(self.scope == SearchScope::CurrentFile, |d| {
                                d.child(
                                    button(
                                        "search-word",
                                        palette,
                                        ButtonVariant::Ghost,
                                        ButtonSize::Xs,
                                    )
                                    .child("Word")
                                    .when(self.options.whole_word, |d| {
                                        d.bg(palette.selected_fill)
                                            .text_color(palette.selected_accent)
                                    })
                                    .on_click(cx.listener(
                                        |this, _: &ClickEvent, _w, cx| {
                                            this.toggle_option(
                                                |options| options.whole_word = !options.whole_word,
                                                cx,
                                            )
                                        },
                                    )),
                                )
                            })
                            .when(self.scope == SearchScope::CurrentFile, |d| {
                                d.child(
                                    button(
                                        "search-multiline",
                                        palette,
                                        ButtonVariant::Ghost,
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
                                            ButtonVariant::Ghost,
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
                        .child(
                            button("search-prev", palette, ButtonVariant::Ghost, ButtonSize::Xs)
                                .child("↑")
                                .when(!has_matches, |d| {
                                    d.opacity(DISABLED_OPACITY).cursor_default()
                                })
                                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                    if this.count.1 > 0 && this.error.is_none() {
                                        this.step(false, cx)
                                    }
                                })),
                        )
                        .child(
                            button("search-next", palette, ButtonVariant::Ghost, ButtonSize::Xs)
                                .child("↓")
                                .when(!has_matches, |d| {
                                    d.opacity(DISABLED_OPACITY).cursor_default()
                                })
                                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                    if this.count.1 > 0 && this.error.is_none() {
                                        this.step(true, cx)
                                    }
                                })),
                        ),
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
                                .map(|input| field_input(input).w(px(220.0)).into_any_element())
                                .unwrap_or_else(|| div().into_any_element()),
                        )
                        .child(
                            button(
                                "search-replace-one",
                                palette,
                                ButtonVariant::Outline,
                                ButtonSize::Xs,
                            )
                            .child("Replace")
                            .when(!can_replace, |d| {
                                d.opacity(DISABLED_OPACITY).cursor_default()
                            })
                            .on_click(cx.listener(
                                |this, _: &ClickEvent, _w, cx| {
                                    if this.count.1 > 0 && this.error.is_none() {
                                        this.replace_one(cx)
                                    }
                                },
                            )),
                        )
                        .child(
                            button(
                                "search-replace-all",
                                palette,
                                ButtonVariant::Secondary,
                                ButtonSize::Xs,
                            )
                            .child("All")
                            .when(!can_replace, |d| {
                                d.opacity(DISABLED_OPACITY).cursor_default()
                            })
                            .on_click(cx.listener(
                                |this, _: &ClickEvent, _w, cx| {
                                    if this.count.1 > 0 && this.error.is_none() {
                                        this.replace_all(cx)
                                    }
                                },
                            )),
                        ),
                )
            })
            .when(self.scope == SearchScope::Project, |d| {
                d.child(
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
                        .when(!project_loading && total == 0 && error.is_none(), |d| {
                            d.child(
                                div()
                                    .text_xs()
                                    .text_color(palette.muted)
                                    .child("No project results"),
                            )
                        })
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
                    ButtonVariant::Ghost,
                    ButtonSize::IconXs,
                )
                .child("×")
                .on_click(cx.listener(|_this, _: &ClickEvent, _w, cx| cx.emit(DismissEvent))),
            )
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                if ev.keystroke.key == "escape" {
                    cx.emit(DismissEvent);
                    cx.stop_propagation();
                } else if this.scope == SearchScope::Project
                    && this.target == SearchTarget::Editor
                    && ev.keystroke.key == "up"
                {
                    let _ = this
                        .workspace
                        .update(cx, |workspace, cx| workspace.project_search_move(-1, cx));
                    cx.stop_propagation();
                } else if this.scope == SearchScope::Project
                    && this.target == SearchTarget::Editor
                    && ev.keystroke.key == "down"
                {
                    let _ = this
                        .workspace
                        .update(cx, |workspace, cx| workspace.project_search_move(1, cx));
                    cx.stop_propagation();
                } else if this.scope == SearchScope::Project
                    && this.target == SearchTarget::Editor
                    && ev.keystroke.key == "enter"
                {
                    this.open_selected_project_result(window, cx);
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
