//! Editable excerpt presentation for the Editor-owned Multi-Buffer model.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, ClickEvent, Context, Entity, EventEmitter, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window,
};
use labonair_editor::{
    ExcerptId, MultiBuffer, MultiBufferError, MultiBufferSaveOutcome, MultiBufferSaveRequest,
    ProjectSearchQuery,
};
use labonair_theme::ThemeStore;
use labonair_ui_kit::{
    button, button_disabled, text_area_surface, text_field, text_input, ButtonSize, ButtonVariant,
    InputEvent, InputState, Palette, ScrollableElement, TextFieldState,
};

struct ExcerptInput {
    id: ExcerptId,
    state: Entity<InputState>,
    _subscription: Subscription,
}

#[derive(Clone, Debug)]
pub enum MultiBufferEvent {
    SaveRequested(Vec<MultiBufferSaveRequest>),
}

/// UI for editing search-result excerpts while the Editor core retains the
/// source-file buffers, transactions, dirty state, and save preconditions.
pub struct MultiBufferView {
    buffer: MultiBuffer,
    theme: Entity<ThemeStore>,
    inputs: Vec<ExcerptInput>,
    inputs_ready: bool,
    saving: bool,
    error: Option<String>,
}

impl EventEmitter<MultiBufferEvent> for MultiBufferView {}

impl MultiBufferView {
    pub fn new(
        buffer: MultiBuffer,
        theme: Entity<ThemeStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        let mut view = Self {
            buffer,
            theme,
            inputs: Vec::new(),
            inputs_ready: false,
            saving: false,
            error: None,
        };
        view.ensure_inputs(window, cx);
        view
    }

    pub fn can_replace_all(&self) -> bool {
        self.buffer
            .file_snapshots()
            .iter()
            .all(|file| file.can_edit)
    }

    pub fn dirty_file_count(&self) -> usize {
        self.buffer
            .file_snapshots()
            .iter()
            .filter(|file| file.is_dirty)
            .count()
    }

    pub fn has_focused_input(&self, window: &Window, cx: &App) -> bool {
        self.inputs
            .iter()
            .any(|input| input.state.read(cx).focus_handle(cx).is_focused(window))
    }

    pub fn replace_project_search_all(
        &mut self,
        query: &ProjectSearchQuery,
        replacement: &str,
        cx: &mut Context<Self>,
    ) -> Result<usize, MultiBufferError> {
        match self.buffer.replace_project_search_all(query, replacement) {
            Ok(replaced) => {
                self.reset_inputs();
                self.error = None;
                cx.notify();
                Ok(replaced)
            }
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
                Err(error)
            }
        }
    }

    pub fn accept_save_result(
        &mut self,
        result: Result<MultiBufferSaveOutcome, String>,
        cx: &mut Context<Self>,
    ) {
        self.saving = false;
        match result {
            Ok(outcome) => {
                self.error = None;
                for saved in outcome.saved {
                    if let Err(error) =
                        self.buffer
                            .mark_saved_snapshot(&saved.path, saved.revision, saved.snapshot)
                    {
                        self.error = Some(error.to_string());
                        break;
                    }
                }
                if self.error.is_none() {
                    self.error = outcome.error;
                }
            }
            Err(error) => self.error = Some(error),
        }
        cx.notify();
    }

    fn reset_inputs(&mut self) {
        self.inputs.clear();
        self.inputs_ready = false;
    }

    fn ensure_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.inputs_ready {
            return;
        }
        let files = self.buffer.file_snapshots();
        let values = self
            .buffer
            .snapshot()
            .excerpts()
            .iter()
            .filter_map(|excerpt| {
                let file = files.iter().find(|file| file.path == excerpt.path)?;
                file.can_edit
                    .then(|| (excerpt.id, file.snapshot.text_range(excerpt.source_range)))
            })
            .collect::<Vec<_>>();
        self.inputs = values
            .into_iter()
            .map(|(id, text)| {
                let state =
                    cx.new(|cx| text_field(window, cx).multi_line(true).default_value(text));
                let subscription =
                    cx.subscribe(&state, move |this, input, event: &InputEvent, cx| {
                        if matches!(event, InputEvent::Change) {
                            let text = input.read(cx).value().to_string();
                            this.edit_excerpt(id, &text, cx);
                        }
                    });
                ExcerptInput {
                    id,
                    state,
                    _subscription: subscription,
                }
            })
            .collect();
        self.inputs_ready = true;
    }

    fn edit_excerpt(&mut self, excerpt: ExcerptId, text: &str, cx: &mut Context<Self>) {
        match self.buffer.replace_excerpt_text(excerpt, text) {
            Ok(_) => self.error = None,
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn request_save(&mut self, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let requests = match self.buffer.save_requests() {
            Ok(requests) if requests.is_empty() => return,
            Ok(requests) => requests,
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        self.saving = true;
        self.error = None;
        cx.emit(MultiBufferEvent::SaveRequested(requests));
        cx.notify();
    }

    fn undo(&mut self, cx: &mut Context<Self>) {
        match self.buffer.undo() {
            Ok(true) => {
                self.reset_inputs();
                self.error = None;
            }
            Ok(false) => {}
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn redo(&mut self, cx: &mut Context<Self>) {
        match self.buffer.redo() {
            Ok(true) => {
                self.reset_inputs();
                self.error = None;
            }
            Ok(false) => {}
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }
}

impl Render for MultiBufferView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_inputs(window, cx);
        let palette = Palette::from_theme(self.theme.read(cx));
        let excerpts = self.buffer.snapshot().excerpts().to_vec();
        let files = self.buffer.file_snapshots();
        let dirty_file_count = files.iter().filter(|file| file.is_dirty).count();
        let rows = excerpts
            .iter()
            .filter_map(|excerpt| {
                let file = files.iter().find(|file| file.path == excerpt.path)?;
                let position = file.snapshot.byte_to_position(excerpt.source_range.start);
                let content = file.snapshot.text_range(excerpt.source_range);
                let body =
                    if let Some(input) = self.inputs.iter().find(|input| input.id == excerpt.id) {
                        let focused = input.state.read(cx).focus_handle(cx).is_focused(window);
                        text_area_surface(
                            ("multi-buffer-excerpt", excerpt.id.value()),
                            palette,
                            if focused {
                                TextFieldState::Focused
                            } else {
                                TextFieldState::Normal
                            },
                            text_input(&input.state, palette),
                        )
                        .w_full()
                        .min_h(px(96.0))
                        .into_any_element()
                    } else {
                        div()
                            .max_h(px(140.0))
                            .overflow_y_scrollbar()
                            .p_1()
                            .rounded(px(palette.radius.sm))
                            .bg(palette.bg)
                            .font_family("monospace")
                            .text_size(px(11.0))
                            .text_color(palette.fg)
                            .children(
                                content
                                    .lines()
                                    .map(|line| {
                                        div()
                                            .whitespace_nowrap()
                                            .child(SharedString::from(line.to_string()))
                                            .into_any_element()
                                    })
                                    .collect::<Vec<_>>(),
                            )
                            .into_any_element()
                    };
                Some(
                    div()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .text_xs()
                                .text_color(palette.muted)
                                .child(format!("{}:{}", excerpt.path.display(), position.line + 1))
                                .child(if file.is_dirty { "Modified" } else { "Saved" }),
                        )
                        .child(body)
                        .into_any_element(),
                )
            })
            .collect::<Vec<_>>();
        let save_button = if self.saving {
            button_disabled(
                "multi-buffer-save",
                palette,
                ButtonVariant::Outlined,
                ButtonSize::Xs,
            )
            .child("Saving…")
        } else if dirty_file_count > 0 {
            button(
                "multi-buffer-save",
                palette,
                ButtonVariant::Outlined,
                ButtonSize::Xs,
            )
            .child(format!("Save {} files", dirty_file_count))
            .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| this.request_save(cx)))
        } else {
            button_disabled(
                "multi-buffer-save",
                palette,
                ButtonVariant::Outlined,
                ButtonSize::Xs,
            )
            .child("Saved")
        };
        let undo_button = if self.buffer.can_undo() {
            button(
                "multi-buffer-undo",
                palette,
                ButtonVariant::Subtle,
                ButtonSize::Xs,
            )
            .child("Undo")
            .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| this.undo(cx)))
        } else {
            button_disabled(
                "multi-buffer-undo",
                palette,
                ButtonVariant::Subtle,
                ButtonSize::Xs,
            )
            .child("Undo")
        };
        let redo_button = if self.buffer.can_redo() {
            button(
                "multi-buffer-redo",
                palette,
                ButtonVariant::Subtle,
                ButtonSize::Xs,
            )
            .child("Redo")
            .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| this.redo(cx)))
        } else {
            button_disabled(
                "multi-buffer-redo",
                palette,
                ButtonVariant::Subtle,
                ButtonSize::Xs,
            )
            .child("Redo")
        };
        div()
            .id("multi-buffer-editor")
            .max_h(px(320.0))
            .overflow_y_scrollbar()
            .flex_col()
            .gap_1()
            .p_1()
            .rounded(px(palette.radius.sm))
            .bg(palette.bg)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_xs()
                    .text_color(palette.muted)
                    .child(format!(
                        "{} modified files · edits stay here until saved",
                        dirty_file_count
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(undo_button)
                            .child(redo_button)
                            .child(save_button),
                    ),
            )
            .children(rows)
            .when_some(self.error.clone(), |d, error| {
                d.child(div().text_xs().text_color(palette.error).child(error))
            })
    }
}
