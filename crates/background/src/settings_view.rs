//! Background-owned editor for persisted background preferences.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, Context, DismissEvent, ElementId, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    Stateful, StatefulInteractiveElement, Styled, Subscription, Task, Timer, Window,
};
use labonair_theme::ThemeStore;
use labonair_ui_kit::{
    button, button_disabled, field_input, number_field, segmented_control, slider, text_field,
    text_field_surface, ButtonSize, ButtonVariant, IconName, InputEvent, InputState, Palette,
    ScrollableElement, SegmentSize, SegmentVariant, SliderEvent, SliderState, SliderValue,
    TextFieldState,
};

use crate::{
    background_delete, background_import, BackgroundFit, BackgroundStore, BackgroundTarget,
};

type ErrorSink = Rc<dyn Fn(String, &mut App)>;

/// Background capability UI. It edits values through `BackgroundStore` and
/// never mirrors persistence into Settings.
pub struct BackgroundSettingsView {
    store: Entity<BackgroundStore>,
    theme: Entity<ThemeStore>,
    tint_input: Entity<InputState>,
    opacity: Entity<SliderState>,
    blur: Entity<SliderState>,
    tint_opacity: Entity<SliderState>,
    focus: FocusHandle,
    tint_invalid: bool,
    confirm_delete: Option<String>,
    importing: bool,
    deleting: Option<String>,
    pending_opacity: Option<u8>,
    pending_blur: Option<u8>,
    pending_tint_opacity: Option<u8>,
    opacity_commit: Task<()>,
    blur_commit: Task<()>,
    tint_opacity_commit: Task<()>,
    report_error: ErrorSink,
    _subscriptions: Vec<Subscription>,
}

impl BackgroundSettingsView {
    /// Build the editor with owner entities and a Shell-provided notification
    /// sink for filesystem failures.
    pub fn new(
        store: Entity<BackgroundStore>,
        theme: Entity<ThemeStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
        report_error: impl Fn(String, &mut App) + 'static,
    ) -> Self {
        let settings = store.read(cx).settings().clone();
        let tint_input = cx.new(|cx| {
            let mut state = text_field(window, cx).placeholder("#000000");
            state.set_value(settings.background_tint_color.clone(), window, cx);
            state
        });
        let opacity = cx.new(|_| slider_state(settings.background_opacity));
        let blur = cx.new(|_| slider_state(settings.background_blur));
        let tint_opacity = cx.new(|_| slider_state(settings.background_tint_opacity));

        let mut subscriptions = Vec::new();
        let report_error: ErrorSink = Rc::new(report_error);
        let event_reporter = report_error.clone();
        subscriptions.push(cx.subscribe(
            &store,
            move |_, _, event: &crate::BackgroundEvent, cx| match event {
                crate::BackgroundEvent::Error(message) => event_reporter(message.clone(), cx),
            },
        ));
        let tint_store = store.clone();
        subscriptions.push(
            cx.subscribe(&tint_input, move |this, input, event, cx| match event {
                InputEvent::Change => cx.notify(),
                InputEvent::PressEnter { .. } | InputEvent::Blur => {
                    let value = input.read(cx).value().to_string();
                    this.tint_invalid = !valid_hex_color(&value);
                    if !this.tint_invalid {
                        tint_store.update(cx, |store, cx| store.set_tint_color(value, cx));
                    }
                    cx.notify();
                }
                _ => {}
            }),
        );

        subscriptions.push(cx.subscribe(&opacity, |this, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(SliderValue::Single(value)) = event {
                let value = value.round().clamp(0.0, 100.0) as u8;
                this.pending_opacity = Some(value);
                this.opacity_commit = cx.spawn(async move |this, cx| {
                    Timer::after(Duration::from_millis(180)).await;
                    let _ = this.update(cx, |this, cx| {
                        if let Some(value) = this.pending_opacity.take() {
                            this.store
                                .update(cx, |store, cx| store.set_opacity(value, cx));
                        }
                    });
                });
                cx.notify();
            }
        }));
        subscriptions.push(cx.subscribe(&blur, |this, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(SliderValue::Single(value)) = event {
                let value = value.round().clamp(0.0, 100.0) as u8;
                this.pending_blur = Some(value);
                this.blur_commit = cx.spawn(async move |this, cx| {
                    Timer::after(Duration::from_millis(180)).await;
                    let _ = this.update(cx, |this, cx| {
                        if let Some(value) = this.pending_blur.take() {
                            this.store.update(cx, |store, cx| store.set_blur(value, cx));
                        }
                    });
                });
                cx.notify();
            }
        }));
        subscriptions.push(
            cx.subscribe(&tint_opacity, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(SliderValue::Single(value)) = event {
                    let value = value.round().clamp(0.0, 100.0) as u8;
                    this.pending_tint_opacity = Some(value);
                    this.tint_opacity_commit = cx.spawn(async move |this, cx| {
                        Timer::after(Duration::from_millis(180)).await;
                        let _ = this.update(cx, |this, cx| {
                            if let Some(value) = this.pending_tint_opacity.take() {
                                this.store
                                    .update(cx, |store, cx| store.set_tint_opacity(value, cx));
                            }
                        });
                    });
                    cx.notify();
                }
            }),
        );

        cx.observe(&store, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe(&opacity, |_, _, cx| cx.notify()).detach();
        cx.observe(&blur, |_, _, cx| cx.notify()).detach();
        cx.observe(&tint_opacity, |_, _, cx| cx.notify()).detach();

        Self {
            store,
            theme,
            tint_input,
            opacity,
            blur,
            tint_opacity,
            focus: cx.focus_handle(),
            tint_invalid: false,
            confirm_delete: None,
            importing: false,
            deleting: None,
            pending_opacity: None,
            pending_blur: None,
            pending_tint_opacity: None,
            opacity_commit: Task::ready(()),
            blur_commit: Task::ready(()),
            tint_opacity_commit: Task::ready(()),
            report_error,
            _subscriptions: subscriptions,
        }
    }

    fn update_setting(
        &self,
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut BackgroundStore, &mut Context<BackgroundStore>),
    ) {
        self.store.update(cx, update);
    }

    pub fn flush_pending(&mut self, cx: &mut Context<Self>) {
        self.opacity_commit = Task::ready(());
        self.blur_commit = Task::ready(());
        self.tint_opacity_commit = Task::ready(());
        let opacity = self.pending_opacity.take();
        let blur = self.pending_blur.take();
        let tint_opacity = self.pending_tint_opacity.take();
        self.store.update(cx, |store, cx| {
            if let Some(value) = opacity {
                store.set_opacity(value, cx);
            }
            if let Some(value) = blur {
                store.set_blur(value, cx);
            }
            if let Some(value) = tint_opacity {
                store.set_tint_opacity(value, cx);
            }
        });
    }

    fn request_import(&mut self, cx: &mut Context<Self>) {
        if self.importing {
            return;
        }
        self.importing = true;
        cx.notify();
        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose background image".into()),
        });
        let store = self.store.clone();
        let report_error = self.report_error.clone();
        cx.spawn(async move |this, cx| {
            let path = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let imported = if let Some(path) = path {
                Some(
                    cx.background_executor()
                        .spawn(async move { background_import(path.to_string_lossy().to_string()) })
                        .await,
                )
            } else {
                None
            };
            let _ = cx.update(|cx| {
                if let Some(result) = imported {
                    match result {
                        Ok(info) => {
                            store.update(cx, |store, cx| {
                                store.select_imported(info, cx);
                            });
                        }
                        Err(message) => report_error(message, cx),
                    }
                }
            });
            let _ = this.update(cx, |this, cx| {
                this.importing = false;
                cx.notify();
            });
        })
        .detach();
    }

    fn remove_image(&mut self, filename: String, cx: &mut Context<Self>) {
        self.confirm_delete = None;
        self.deleting = Some(filename.clone());
        cx.notify();
        let store = self.store.clone();
        let report_error = self.report_error.clone();
        cx.spawn(async move |this, cx| {
            let delete_name = filename.clone();
            let result = cx
                .background_executor()
                .spawn(async move { background_delete(delete_name) })
                .await;
            let _ = cx.update(|cx| match result {
                Ok(()) => store.update(cx, |store, cx| store.forget_deleted(&filename, cx)),
                Err(message) => report_error(message, cx),
            });
            let _ = this.update(cx, |this, cx| {
                this.deleting = None;
                cx.notify();
            });
        })
        .detach();
    }
}

impl EventEmitter<DismissEvent> for BackgroundSettingsView {}

impl Focusable for BackgroundSettingsView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for BackgroundSettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let viewport_height: f32 = window.viewport_size().height.into();
        let modal_inset: f32 = c.space(112.0).into();
        let settings = self.store.read(cx).settings().clone();
        let image_loading = self.store.read(cx).is_loading();
        let images = self.store.read(cx).available();
        let empty_images = images.is_empty();
        let selected = settings.background_image.clone();
        let selected_for_delete = selected.clone();
        let selected_for_removal = selected.clone();
        let selected_fit = match settings.background_fit {
            BackgroundFit::Cover => "cover",
            BackgroundFit::Contain => "contain",
            BackgroundFit::Tile => "tile",
        };
        let selected_target = match settings.background_target {
            BackgroundTarget::Both => "both",
            BackgroundTarget::App => "app",
            BackgroundTarget::Terminal => "terminal",
        };

        let mut image_rows = vec![button(
            "background-image-none",
            c,
            ButtonVariant::OutlinedGhost,
            ButtonSize::Default,
        )
        .w_full()
        .justify_start()
        .when(selected.is_empty(), |row| {
            row.bg(c.selected_fill).border_color(c.selected_accent)
        })
        .child("None")
        .on_click(cx.listener(|this, _, _, cx| {
            this.update_setting(cx, |store, cx| store.set_image("", cx));
            this.confirm_delete = None;
        }))
        .into_any_element()];
        image_rows.extend(images.into_iter().map(|image| {
            let filename = image.filename;
            let active = selected == filename;
            let action = filename.clone();
            button(
                ElementId::Name(format!("background-image-{filename}").into()),
                c,
                ButtonVariant::OutlinedGhost,
                ButtonSize::Default,
            )
            .w_full()
            .justify_start()
            .when(active, |row| {
                row.bg(c.selected_fill).border_color(c.selected_accent)
            })
            .child(filename)
            .on_click(cx.listener(move |this, _, _window, cx| {
                this.update_setting(cx, |store, cx| store.set_image(action.clone(), cx));
                this.confirm_delete = None;
            }))
            .into_any_element()
        }));

        let selected_image_exists = !selected.is_empty();
        let delete_controls = if self.deleting.as_deref() == Some(selected.as_str()) {
            button_disabled(
                "background-delete-pending",
                c,
                ButtonVariant::OutlinedGhost,
                ButtonSize::Default,
            )
            .child("Removing…")
            .into_any_element()
        } else if self.confirm_delete.as_deref() == Some(selected.as_str()) {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    button(
                        "background-delete-cancel",
                        c,
                        ButtonVariant::OutlinedGhost,
                        ButtonSize::Default,
                    )
                    .child("Cancel")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.confirm_delete = None;
                        cx.notify();
                    })),
                )
                .child(
                    button(
                        "background-delete-confirm",
                        c,
                        ButtonVariant::TintedError,
                        ButtonSize::Default,
                    )
                    .child("Remove image")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.remove_image(selected_for_removal.clone(), cx)
                    })),
                )
                .into_any_element()
        } else if selected_image_exists {
            button(
                "background-delete-start",
                c,
                ButtonVariant::OutlinedGhost,
                ButtonSize::Default,
            )
            .child("Remove selected…")
            .on_click(cx.listener(move |this, _, _, cx| {
                this.confirm_delete = Some(selected_for_delete.clone());
                cx.notify();
            }))
            .into_any_element()
        } else {
            button_disabled(
                "background-delete-start",
                c,
                ButtonVariant::OutlinedGhost,
                ButtonSize::Default,
            )
            .child("Remove selected…")
            .into_any_element()
        };
        let import_button = if self.importing {
            button_disabled(
                "background-import",
                c,
                ButtonVariant::Outlined,
                ButtonSize::Default,
            )
            .child("Adding image…")
            .into_any_element()
        } else {
            button(
                "background-import",
                c,
                ButtonVariant::Outlined,
                ButtonSize::Default,
            )
            .child("Add image…")
            .on_click(cx.listener(|this, _, _, cx| this.request_import(cx)))
            .into_any_element()
        };

        let tint_state = if self.tint_invalid {
            TextFieldState::Invalid
        } else {
            TextFieldState::Normal
        };
        let opacity_value = slider_value(&self.opacity, cx).unwrap_or(settings.background_opacity);
        let blur_value = slider_value(&self.blur, cx).unwrap_or(settings.background_blur);
        let tint_opacity_value =
            slider_value(&self.tint_opacity, cx).unwrap_or(settings.background_tint_opacity);

        div()
            .id("background-settings-dialog")
            .track_focus(&self.focus)
            .w_full()
            .mx(c.space(12.0))
            .max_w(c.space(620.0))
            .max_h(px(
                (viewport_height - modal_inset).max(c.space(240.0).into())
            ))
            .flex()
            .flex_col()
            .rounded(px(c.radius.lg))
            .border_1()
            .border_color(c.border)
            .bg(c.card)
            .text_color(c.card_fg)
            .shadow_lg()
            .on_key_down(cx.listener(|_, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    cx.emit(DismissEvent);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_4()
                    .p(c.space(18.0))
                    .border_b_1()
                    .border_color(c.border)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(15.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Background"),
                            )
                            .child(div().text_size(px(11.5)).text_color(c.muted).child(
                                "Choose an image and control how it appears behind the app.",
                            )),
                    )
                    .child(
                        button(
                            "background-settings-close",
                            c,
                            ButtonVariant::Subtle,
                            ButtonSize::Icon,
                        )
                        .child(IconName::Close.svg(c.muted).size(px(14.0)))
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(DismissEvent))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .gap_5()
                    .p(c.space(18.0))
                    .overflow_y_scrollbar()
                    .child(section_heading("Image", c))
                    .child(if empty_images {
                        div()
                            .rounded(px(c.radius.sm))
                            .border_1()
                            .border_color(c.border)
                            .bg(c.bg)
                            .p(c.space(12.0))
                            .text_size(px(11.5))
                            .text_color(c.muted)
                            .child("No background images have been added.")
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    })
                    .child(div().flex().flex_col().gap_1().children(image_rows))
                    .when(image_loading, |body| {
                        body.child(
                            div()
                                .text_size(px(11.5))
                                .text_color(c.muted)
                                .child("Preparing background preview…"),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(import_button)
                            .child(delete_controls),
                    )
                    .child(section_heading("Image appearance", c))
                    .child(slider_row(
                        "Opacity",
                        opacity_value,
                        "%",
                        &self.opacity,
                        c,
                        cx.listener(|this, value: &f64, window, cx| {
                            this.pending_opacity = None;
                            this.opacity_commit = Task::ready(());
                            this.opacity.update(cx, |slider, cx| {
                                slider.set_value(*value as f32, window, cx)
                            });
                            this.store
                                .update(cx, |store, cx| store.set_opacity(*value as u8, cx));
                        }),
                    ))
                    .child(slider_row(
                        "Blur",
                        blur_value,
                        "px",
                        &self.blur,
                        c,
                        cx.listener(|this, value: &f64, window, cx| {
                            this.pending_blur = None;
                            this.blur_commit = Task::ready(());
                            this.blur.update(cx, |slider, cx| {
                                slider.set_value(*value as f32, window, cx)
                            });
                            this.store
                                .update(cx, |store, cx| store.set_blur(*value as u8, cx));
                        }),
                    ))
                    .child(setting_row("Tint color", c).child(text_field_surface(
                        "background-tint-color",
                        c,
                        tint_state,
                        field_input(&self.tint_input),
                    )))
                    .child(slider_row(
                        "Tint opacity",
                        tint_opacity_value,
                        "%",
                        &self.tint_opacity,
                        c,
                        cx.listener(|this, value: &f64, window, cx| {
                            this.pending_tint_opacity = None;
                            this.tint_opacity_commit = Task::ready(());
                            this.tint_opacity.update(cx, |slider, cx| {
                                slider.set_value(*value as f32, window, cx)
                            });
                            this.store
                                .update(cx, |store, cx| store.set_tint_opacity(*value as u8, cx));
                        }),
                    ))
                    .child(section_heading("Placement", c))
                    .child(
                        setting_row("Image scaling", c).child(
                            segmented_control("background-fit", c, selected_fit)
                                .size(SegmentSize::Sm)
                                .variant(SegmentVariant::Outline)
                                .segment("cover", "Cover")
                                .segment("contain", "Contain")
                                .segment("tile", "Tile")
                                .on_select(cx.listener(|this, key: &SharedString, _, cx| {
                                    let fit = match key.as_ref() {
                                        "contain" => BackgroundFit::Contain,
                                        "tile" => BackgroundFit::Tile,
                                        _ => BackgroundFit::Cover,
                                    };
                                    this.update_setting(cx, |store, cx| store.set_fit(fit, cx));
                                })),
                        ),
                    )
                    .child(
                        setting_row("Show behind", c).child(
                            segmented_control("background-target", c, selected_target)
                                .size(SegmentSize::Sm)
                                .variant(SegmentVariant::Outline)
                                .segment("both", "App + terminal")
                                .segment("app", "App")
                                .segment("terminal", "Terminal")
                                .on_select(cx.listener(|this, key: &SharedString, _, cx| {
                                    let target = match key.as_ref() {
                                        "app" => BackgroundTarget::App,
                                        "terminal" => BackgroundTarget::Terminal,
                                        _ => BackgroundTarget::Both,
                                    };
                                    this.update_setting(cx, |store, cx| {
                                        store.set_target(target, cx)
                                    });
                                })),
                        ),
                    ),
            )
            .into_any_element()
    }
}

fn slider_state(value: u8) -> SliderState {
    SliderState::new()
        .min(0.0)
        .max(100.0)
        .step(1.0)
        .default_value(f32::from(value))
}

fn slider_value(state: &Entity<SliderState>, cx: &App) -> Option<u8> {
    match state.read(cx).value() {
        SliderValue::Single(value) => Some(value.round().clamp(0.0, 100.0) as u8),
        SliderValue::Range(_, _) => None,
    }
}

fn slider_row(
    label: &'static str,
    value: u8,
    suffix: &'static str,
    state: &Entity<SliderState>,
    c: Palette,
    on_change: impl Fn(&f64, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    setting_row(label, c)
        .child(slider(state, c).flex_1())
        .child(
            number_field(
                ElementId::Name(format!("background-{label}-value").into()),
                c,
                f64::from(value),
                0.0,
                100.0,
                1.0,
            )
            .track(false)
            .on_change(on_change),
        )
        .child(div().text_size(px(11.5)).text_color(c.muted).child(suffix))
}

fn setting_row(label: &'static str, c: Palette) -> Stateful<gpui::Div> {
    div()
        .id(ElementId::Name(
            format!("background-setting-{label}").into(),
        ))
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .min_h(c.space(34.0))
        .child(
            div()
                .w(c.space(140.0))
                .flex_shrink_0()
                .text_size(px(11.5))
                .child(label),
        )
}

fn section_heading(label: &'static str, c: Palette) -> impl IntoElement {
    div()
        .mt(c.space(4.0))
        .text_size(px(10.5))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(c.muted)
        .child(label)
}

fn valid_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::valid_hex_color;

    #[test]
    fn accepts_six_digit_hex_colors() {
        assert!(valid_hex_color("#000000"));
        assert!(valid_hex_color("#A1b2C3"));
    }

    #[test]
    fn rejects_incomplete_or_non_hex_colors() {
        for value in ["", "000000", "#00000", "#0000000", "#GG0000", "red"] {
            assert!(!valid_hex_color(value), "accepted {value:?}");
        }
    }
}
