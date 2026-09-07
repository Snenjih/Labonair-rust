//! Native keymap management surface.
//!
//! This crate owns presentation only. The keymap domain supplies the
//! lossless document, diagnostics, and effective command rows; the shell
//! supplies the one composition callback that opens the raw JSONC document.
//! No command execution, file parsing, or feature state belongs here.

use gpui::{
    div, point, px, size, App, AppContext, Bounds, ClickEvent, Context, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, TitlebarOptions, Window, WindowBounds,
    WindowHandle, WindowKind, WindowOptions,
};
use gpui_component::Root;

use labonair_command_palette_core::CommandDescriptor;
use labonair_keymap::{adapter, file, management, management::KeymapManagementSnapshot};
use labonair_notifications::{notification_center, Notification};
use labonair_theme::{theme_store, ThemeStore};
use labonair_ui_kit::{
    button, field_input, kbd_row, ButtonSize, ButtonVariant, InputEvent, InputState, ListItem,
    Palette,
};

type OpenRawCallback = Box<dyn FnMut(&mut Window, &mut App) + 'static>;

/// The management window's view state.
pub struct KeymapManagementView {
    theme: Entity<ThemeStore>,
    descriptors: Vec<CommandDescriptor>,
    input: Entity<InputState>,
    snapshot: Option<KeymapManagementSnapshot>,
    load_error: Option<String>,
    open_raw: Option<OpenRawCallback>,
    editing: Option<EditingBinding>,
    edit_input: Option<Entity<InputState>>,
    edit_error: Option<String>,
    saving: bool,
    diagnostic_key: Option<String>,
    focus: FocusHandle,
    _input_subscription: Subscription,
    _edit_subscription: Option<Subscription>,
}

struct EditingBinding {
    command: labonair_command_palette_core::CommandId,
    title: String,
    context: Option<String>,
    old_bindings: Vec<String>,
}

struct BindingEditRequest {
    command: labonair_command_palette_core::CommandId,
    title: String,
    context: Option<String>,
    old_bindings: Vec<String>,
    initial: String,
}

impl Focusable for KeymapManagementView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl KeymapManagementView {
    fn new(
        theme: Entity<ThemeStore>,
        descriptors: Vec<CommandDescriptor>,
        open_raw: OpenRawCallback,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Filter commands…"));
        let input_subscription = cx.subscribe(&input, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });

        let mut view = Self {
            theme,
            descriptors,
            input,
            snapshot: None,
            load_error: None,
            open_raw: Some(open_raw),
            editing: None,
            edit_input: None,
            edit_error: None,
            saving: false,
            diagnostic_key: None,
            focus: cx.focus_handle(),
            _input_subscription: input_subscription,
            _edit_subscription: None,
        };
        view.load_snapshot(cx);
        view
    }

    fn load_snapshot(&mut self, cx: &mut Context<Self>) {
        let descriptors = self.descriptors.clone();
        let load = cx.background_executor().spawn(async move {
            let known_actions = management::known_actions(descriptors.iter());
            let document = file::read_user_keymap_document(&known_actions)?;
            let runtime = adapter::load_descriptors(descriptors.iter());
            Ok::<_, String>(management::snapshot(descriptors.iter(), &runtime, document))
        });
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| match load.await {
            Ok(snapshot) => {
                let _ = entity.update(cx, |view, cx| {
                    view.publish_diagnostics(&snapshot, cx);
                    view.snapshot = Some(snapshot);
                    view.load_error = None;
                    cx.notify();
                });
            }
            Err(error) => {
                let _ = entity.update(cx, |view, cx| {
                    view.publish_error(
                        "Keymap could not be loaded",
                        "The active keymap was not changed.",
                        error.clone(),
                        cx,
                    );
                    view.load_error = Some(error);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn publish_error(
        &self,
        title: &'static str,
        summary: &'static str,
        details: String,
        cx: &mut Context<Self>,
    ) {
        if !cx.has_global::<labonair_notifications::GlobalNotificationCenter>() {
            return;
        }
        notification_center(cx).update(cx, |center, cx| {
            center.push(
                Notification::error(title, summary)
                    .details(details.clone())
                    .dedupe_key(format!("keymap:{title}:{details}")),
                cx,
            );
        });
    }

    fn publish_diagnostics(&mut self, snapshot: &KeymapManagementSnapshot, cx: &mut Context<Self>) {
        if snapshot.issues.is_empty() {
            self.diagnostic_key = None;
            return;
        }

        let details = snapshot
            .issues
            .iter()
            .map(|issue| format!("line {}: {}", issue.line, issue.message))
            .collect::<Vec<_>>()
            .join("\n");
        let key = format!("keymap-diagnostics:{details}");
        if self.diagnostic_key.as_deref() == Some(key.as_str()) {
            return;
        }
        let error_count = snapshot
            .issues
            .iter()
            .filter(|issue| matches!(issue.severity, file::Severity::Error))
            .count();
        let summary = format!(
            "{} error(s), {} warning(s) found in keymap.json.",
            error_count,
            snapshot.issues.len() - error_count
        );
        if !cx.has_global::<labonair_notifications::GlobalNotificationCenter>() {
            return;
        }
        self.diagnostic_key = Some(key.clone());
        notification_center(cx).update(cx, |center, cx| {
            let notification = if error_count > 0 {
                Notification::error("Keymap diagnostics", summary)
            } else {
                Notification::warning("Keymap diagnostics", summary)
            };
            center.push(notification.details(details).dedupe_key(key), cx);
        });
    }

    fn begin_edit(
        &mut self,
        request: BindingEditRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Keystroke, e.g. cmd-shift-p")
                .default_value(request.initial)
        });
        let subscription = cx.subscribe(&input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::PressEnter { secondary: false }) {
                this.commit_edit(cx);
            }
        });
        input.update(cx, |state, cx| state.focus(window, cx));
        self.editing = Some(EditingBinding {
            command: request.command,
            title: request.title,
            context: request.context,
            old_bindings: request.old_bindings,
        });
        self.edit_input = Some(input);
        self.edit_error = None;
        self.saving = false;
        self._edit_subscription = Some(subscription);
        cx.notify();
    }

    fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        self.editing = None;
        self.edit_input = None;
        self.edit_error = None;
        self._edit_subscription = None;
        self.saving = false;
        cx.notify();
    }

    fn commit_edit(&mut self, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let (Some(editing), Some(input), Some(snapshot)) =
            (&self.editing, &self.edit_input, &self.snapshot)
        else {
            return;
        };
        let candidate = input.read(cx).value().trim().to_string();
        if !candidate.is_empty() {
            for token in candidate.split_whitespace() {
                if let Err(error) = gpui::Keystroke::parse(token) {
                    self.edit_error = Some(format!("Invalid keystroke `{token}`: {error:?}"));
                    cx.notify();
                    return;
                }
            }
            if let Some(conflict) =
                snapshot.conflict(editing.command, editing.context.as_deref(), &candidate)
            {
                self.edit_error = Some(format!(
                    "Conflicts with `{}` in {}.",
                    conflict.title,
                    conflict.context.as_deref().unwrap_or("Global")
                ));
                cx.notify();
                return;
            }
        }

        let source = snapshot.document.source.clone();
        let context = editing.context.clone();
        let old_bindings = editing.old_bindings.clone();
        let action = editing.command.action_name().to_string();
        let replacement = (!candidate.is_empty()).then_some(candidate);
        let updated = match file::append_user_binding_override(
            &source,
            context.as_deref(),
            &old_bindings,
            &action,
            replacement.as_deref(),
        ) {
            Ok(updated) => updated,
            Err(error) => {
                self.edit_error = Some(error);
                cx.notify();
                return;
            }
        };

        self.saving = true;
        self.edit_error = None;
        let save = cx
            .background_executor()
            .spawn(async move { file::save_user_keymap_document(&updated) });
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| match save.await {
            Ok(_) => {
                let _ = entity.update(cx, |view, cx| {
                    view.editing = None;
                    view.edit_input = None;
                    view._edit_subscription = None;
                    view.saving = false;
                    view.load_snapshot(cx);
                    cx.notify();
                });
            }
            Err(error) => {
                let _ = entity.update(cx, |view, cx| {
                    view.saving = false;
                    view.edit_error = Some(format!("Could not save keymap.json: {error}"));
                    view.publish_error(
                        "Could not save keymap.json",
                        "The keyboard binding was not saved.",
                        error,
                        cx,
                    );
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn render_editor(&self, palette: Palette, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let (Some(editing), Some(input)) = (&self.editing, &self.edit_input) else {
            return None;
        };
        let context = editing.context.as_deref().unwrap_or("Global");
        let mut card = div()
            .mx(px(16.0))
            .mb(px(10.0))
            .p(px(12.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(if self.edit_error.is_some() {
                palette.error
            } else {
                palette.border
            })
            .bg(palette.card)
            .child(
                div()
                    .mb(px(8.0))
                    .text_size(px(11.0))
                    .text_color(palette.muted)
                    .child(SharedString::from(format!(
                        "Editing {} · {}",
                        editing.title, context
                    ))),
            )
            .child(
                div()
                    .h(px(32.0))
                    .border_1()
                    .border_color(palette.border)
                    .rounded(px(5.0))
                    .child(field_input(input)),
            );
        if let Some(error) = &self.edit_error {
            card = card.child(
                div()
                    .mt(px(7.0))
                    .text_size(px(10.0))
                    .text_color(palette.error)
                    .child(SharedString::from(error.clone())),
            );
        }
        card = card.child(
            div()
                .mt(px(8.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    button(
                        "keymap-save-binding",
                        palette,
                        ButtonVariant::Default,
                        ButtonSize::Xs,
                    )
                    .child(if self.saving { "Saving…" } else { "Save" })
                    .on_click(cx.listener(
                        |this, _: &ClickEvent, _window, cx| {
                            this.commit_edit(cx);
                        },
                    )),
                )
                .child(
                    button(
                        "keymap-unbind-binding",
                        palette,
                        ButtonVariant::Outline,
                        ButtonSize::Xs,
                    )
                    .child("Unbind")
                    .on_click(cx.listener(
                        |this, _: &ClickEvent, window, cx| {
                            if let Some(input) = this.edit_input.clone() {
                                input.update(cx, |state, input_cx| {
                                    state.set_value("", window, input_cx)
                                });
                            }
                            this.commit_edit(cx);
                        },
                    )),
                )
                .child(
                    button(
                        "keymap-cancel-binding",
                        palette,
                        ButtonVariant::Ghost,
                        ButtonSize::Xs,
                    )
                    .child("Cancel")
                    .on_click(cx.listener(
                        |this, _: &ClickEvent, _window, cx| {
                            this.cancel_edit(cx);
                        },
                    )),
                ),
        );
        Some(card.into_any_element())
    }

    fn binding_edit_button(
        &self,
        row: &management::KeymapCommandRow,
        index: usize,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let binding = &row.effective_bindings[index];
        let command = row.command;
        let title = row.title.clone();
        let context = binding.context.clone();
        let initial = binding.keystrokes.clone();
        let old_bindings = row
            .effective_bindings
            .iter()
            .filter(|candidate| candidate.context == binding.context)
            .map(|candidate| candidate.keystrokes.clone())
            .collect::<Vec<_>>();
        button(
            (row.command.action_name(), index),
            palette,
            ButtonVariant::Ghost,
            ButtonSize::Xs,
        )
        .child(kbd_row(
            labonair_keymap::keystroke_tokens(&binding.keystrokes),
            palette,
        ))
        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
            this.begin_edit(
                BindingEditRequest {
                    command,
                    title: title.clone(),
                    context: context.clone(),
                    old_bindings: old_bindings.clone(),
                    initial: initial.clone(),
                },
                window,
                cx,
            );
        }))
        .into_any_element()
    }

    fn unbound_edit_button(
        &self,
        row: &management::KeymapCommandRow,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let command = row.command;
        let title = row.title.clone();
        let (context, old_bindings) = row
            .default_bindings
            .first()
            .map(|binding| {
                (
                    binding.context.map(|context| format!("{context:?}")),
                    Vec::new(),
                )
            })
            .unwrap_or((None, Vec::new()));
        button(
            row.command.action_name(),
            palette,
            ButtonVariant::Outline,
            ButtonSize::Xs,
        )
        .child("Unbound · Edit")
        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
            this.begin_edit(
                BindingEditRequest {
                    command,
                    title: title.clone(),
                    context: context.clone(),
                    old_bindings: old_bindings.clone(),
                    initial: String::new(),
                },
                window,
                cx,
            );
        }))
        .into_any_element()
    }

    fn render_row(
        &self,
        row: &management::KeymapCommandRow,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let binding_label = if row.effective_bindings.is_empty() {
            self.unbound_edit_button(row, palette, cx)
        } else {
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .children(
                    row.effective_bindings
                        .iter()
                        .enumerate()
                        .map(|(index, _)| self.binding_edit_button(row, index, palette, cx)),
                )
                .into_any_element()
        };
        let context = if row.contexts.is_empty() {
            "Global".to_string()
        } else {
            row.contexts
                .iter()
                .map(|context| format!("{context:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        };

        ListItem::new(
            row.command.action_name(),
            palette.fg,
            palette.muted,
            palette.muted_bg,
        )
        .icon(labonair_ui_kit::IconName::Command)
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_size(px(12.0))
                        .child(SharedString::from(row.title.clone())),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(palette.muted)
                        .child(SharedString::from(format!("{} · {}", row.section, context))),
                ),
        )
        .trailing(binding_label)
    }

    fn open_raw_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(callback) = self.open_raw.as_mut() {
            callback(window, cx);
        }
    }
}

impl Render for KeymapManagementView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::from_theme(self.theme.read(cx));
        let query = self.input.read(cx).value().to_string();
        let mut body = div().flex().flex_col().flex_1().min_h_0();

        if let Some(snapshot) = &self.snapshot {
            body = body.children(self.render_editor(palette, cx));
            let rows = snapshot.search(&query);
            if rows.is_empty() {
                body = body.child(
                    div()
                        .flex()
                        .justify_center()
                        .py(px(42.0))
                        .text_size(px(12.0))
                        .text_color(palette.muted)
                        .child("No commands match the filter."),
                );
            } else {
                let mut list = div()
                    .id("keymap-command-list")
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .px(px(12.0))
                    .pb(px(16.0))
                    .overflow_y_scroll();
                let mut section = None::<String>;
                for row in rows {
                    if section.as_deref() != Some(row.section.as_str()) {
                        list = list.child(
                            div()
                                .pt(px(10.0))
                                .pb(px(4.0))
                                .px(px(8.0))
                                .text_size(px(10.0))
                                .text_color(palette.muted)
                                .child(SharedString::from(row.section.to_uppercase())),
                        );
                        section = Some(row.section.clone());
                    }
                    list = list.child(self.render_row(row, palette, cx));
                }
                body = body.child(list);
            }
        } else if self.load_error.is_none() {
            body = body.child(
                div()
                    .flex()
                    .justify_center()
                    .py(px(42.0))
                    .text_size(px(12.0))
                    .text_color(palette.muted)
                    .child("Loading keymap…"),
            );
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.bg)
            .text_color(palette.fg)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(18.0))
                    .py(px(14.0))
                    .border_b_1()
                    .border_color(palette.border)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(div().text_size(px(16.0)).child("Keymap"))
                            .child(
                                div()
                                    .mt(px(3.0))
                                    .text_size(px(11.0))
                                    .text_color(palette.muted)
                                    .child("Commands and their active keyboard bindings"),
                            ),
                    )
                    .child(
                        button(
                            "keymap-edit-json",
                            palette,
                            ButtonVariant::Outline,
                            ButtonSize::Sm,
                        )
                        .child("Edit keymap.json")
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| {
                                this.open_raw_document(window, cx);
                            },
                        )),
                    ),
            )
            .child(
                div()
                    .mx(px(16.0))
                    .my(px(12.0))
                    .h(px(32.0))
                    .border_1()
                    .border_color(palette.border)
                    .rounded(px(6.0))
                    .child(field_input(&self.input)),
            )
            .child(body)
    }
}

#[derive(Default)]
struct KeymapWindowRef {
    handle: Option<WindowHandle<Root>>,
}

impl gpui::Global for KeymapWindowRef {}

/// Open the keymap management window or activate the existing one.
pub fn open_keymap_window<F>(descriptors: Vec<CommandDescriptor>, on_open_raw: F, cx: &mut App)
where
    F: FnMut(&mut Window, &mut App) + 'static,
{
    if let Some(handle) = cx
        .try_global::<KeymapWindowRef>()
        .and_then(|ref_| ref_.handle)
    {
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            cx.activate(true);
            return;
        }
        cx.set_global(KeymapWindowRef { handle: None });
    }

    let bounds = Bounds::centered(None, size(px(760.0), px(680.0)), cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Keymap".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(19.0), px(13.0))),
            }),
            window_min_size: Some(size(px(560.0), px(420.0))),
            kind: WindowKind::Normal,
            is_movable: true,
            ..Default::default()
        },
        move |window, cx| {
            let theme = theme_store(cx);
            let view = cx.new(|cx| {
                KeymapManagementView::new(theme, descriptors, Box::new(on_open_raw), window, cx)
            });
            let view: gpui::AnyView = view.into();
            cx.new(|cx| Root::new(view, window, cx))
        },
    );

    match opened {
        Ok(handle) => {
            cx.set_global(KeymapWindowRef {
                handle: Some(handle),
            });
            cx.activate(true);
        }
        Err(error) => tracing::error!("failed to open keymap window: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keymap_window_type_is_ui_owned() {
        fn assert_focusable<T: Focusable>() {}
        assert_focusable::<KeymapManagementView>();
        let _ = labonair_command_palette_core::CommandId::OpenKeymapJson;
    }
}
