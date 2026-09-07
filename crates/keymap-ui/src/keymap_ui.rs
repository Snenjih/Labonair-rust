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
use labonair_theme::{theme_store, ThemeStore};
use labonair_ui_kit::{
    button, field_input, kbd_row, ButtonSize, ButtonVariant, InputEvent, InputState, ListItem,
    Palette,
};

type OpenRawCallback = Box<dyn FnMut(&mut Window, &mut App) + 'static>;

/// The management window's view state.
pub struct KeymapManagementView {
    theme: Entity<ThemeStore>,
    input: Entity<InputState>,
    snapshot: Option<KeymapManagementSnapshot>,
    load_error: Option<String>,
    open_raw: Option<OpenRawCallback>,
    focus: FocusHandle,
    _input_subscription: Subscription,
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

        let view = Self {
            theme,
            input,
            snapshot: None,
            load_error: None,
            open_raw: Some(open_raw),
            focus: cx.focus_handle(),
            _input_subscription: input_subscription,
        };

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
                    view.snapshot = Some(snapshot);
                    view.load_error = None;
                    cx.notify();
                });
            }
            Err(error) => {
                let _ = entity.update(cx, |view, cx| {
                    view.load_error = Some(error);
                    cx.notify();
                });
            }
        })
        .detach();

        view
    }

    fn open_raw_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(callback) = self.open_raw.as_mut() {
            callback(window, cx);
        }
    }

    fn render_diagnostics(
        &self,
        snapshot: &KeymapManagementSnapshot,
        palette: Palette,
    ) -> Option<gpui::AnyElement> {
        if snapshot.issues.is_empty() {
            return None;
        }
        let error_count = snapshot
            .issues
            .iter()
            .filter(|issue| matches!(issue.severity, file::Severity::Error))
            .count();
        let warning_count = snapshot.issues.len() - error_count;
        Some(
            div()
                .mx(px(16.0))
                .mb(px(10.0))
                .px(px(12.0))
                .py(px(9.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(if error_count > 0 { palette.error } else { palette.border })
                .bg(palette.muted_bg)
                .text_size(px(11.0))
                .text_color(if error_count > 0 { palette.error } else { palette.muted })
                .child(SharedString::from(format!(
                    "{} error(s), {} warning(s) in keymap.json. Open the raw document to inspect the original text.",
                    error_count, warning_count
                )))
                .into_any_element(),
        )
    }

    fn render_row(&self, row: &management::KeymapCommandRow, palette: Palette) -> impl IntoElement {
        let keys = row
            .effective_bindings
            .first()
            .map(|binding| labonair_keymap::keystroke_tokens(&binding.keystrokes))
            .unwrap_or_default();
        let binding_label = if keys.is_empty() {
            div()
                .text_size(px(10.0))
                .text_color(palette.muted)
                .child("Unbound")
                .into_any_element()
        } else {
            kbd_row(keys, palette).into_any_element()
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
}

impl Render for KeymapManagementView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::from_theme(self.theme.read(cx));
        let query = self.input.read(cx).value().to_string();
        let mut body = div().flex().flex_col().flex_1().min_h_0();

        if let Some(error) = &self.load_error {
            body = body.child(
                div()
                    .p(px(24.0))
                    .text_color(palette.error)
                    .child(SharedString::from(error.clone())),
            );
        } else if let Some(snapshot) = &self.snapshot {
            body = body.children(self.render_diagnostics(snapshot, palette));
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
                    list = list.child(self.render_row(row, palette));
                }
                body = body.child(list);
            }
        } else {
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
