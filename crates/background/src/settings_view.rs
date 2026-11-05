//! Background-owned editor for persisted background preferences.
//!
//! Embedded inline on the Settings "Themes" page (ADR 0006). The view is a
//! stack of `settings_row`s separated by hairlines; the Settings page supplies
//! the surrounding card, so nothing here draws its own panel.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, App, AppContext, Context, Div, ElementId, Entity, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, SharedString, Stateful, StatefulInteractiveElement,
    Styled, Subscription, Task, Timer, Window,
};
use labonair_theme::ThemeStore;
use labonair_ui_kit::{
    button, button_disabled, divider, field_input, segmented_control, settings_row, slider_row,
    text_field, text_field_surface, Axis, ButtonSize, ButtonVariant, InputEvent, InputState,
    ListItem, Palette, SegmentSize, SegmentVariant, SettingsRow, SliderEvent, SliderState,
    SliderValue, TextFieldState,
};

use crate::{
    background_delete, background_import, BackgroundFit, BackgroundStore, BackgroundTarget,
};

type ErrorSink = Rc<dyn Fn(String, &mut App)>;

/// How long a slider drag waits for a pause before the store persists it.
const COMMIT_DELAY: Duration = Duration::from_millis(180);

/// The three sliders of the editor, for keyboard stepping.
#[derive(Clone, Copy)]
enum BackgroundSlider {
    Opacity,
    Blur,
    TintOpacity,
}

/// Background capability UI. It edits values through `BackgroundStore` and
/// never mirrors persistence into Settings.
pub struct BackgroundSettingsView {
    store: Entity<BackgroundStore>,
    theme: Entity<ThemeStore>,
    tint_input: Entity<InputState>,
    opacity: Entity<SliderState>,
    blur: Entity<SliderState>,
    tint_opacity: Entity<SliderState>,
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
        // Store-level failures (load, decode, save) are reported once, app-wide,
        // by the shell's `BackgroundEvent` subscription; this view only reports
        // its own filesystem failures.
        let report_error: ErrorSink = Rc::new(report_error);
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
                let value = percent(*value);
                this.pending_opacity = Some(value);
                this.opacity_commit = cx.spawn(async move |this, cx| {
                    Timer::after(COMMIT_DELAY).await;
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
                let value = percent(*value);
                this.pending_blur = Some(value);
                this.blur_commit = cx.spawn(async move |this, cx| {
                    Timer::after(COMMIT_DELAY).await;
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
                    let value = percent(*value);
                    this.pending_tint_opacity = Some(value);
                    this.tint_opacity_commit = cx.spawn(async move |this, cx| {
                        Timer::after(COMMIT_DELAY).await;
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

    /// Step a slider by `delta` percent from the keyboard and persist at once.
    fn step_slider(
        &mut self,
        which: BackgroundSlider,
        delta: i32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (slider, pending, commit) = match which {
            BackgroundSlider::Opacity => (
                &self.opacity,
                &mut self.pending_opacity,
                &mut self.opacity_commit,
            ),
            BackgroundSlider::Blur => (&self.blur, &mut self.pending_blur, &mut self.blur_commit),
            BackgroundSlider::TintOpacity => (
                &self.tint_opacity,
                &mut self.pending_tint_opacity,
                &mut self.tint_opacity_commit,
            ),
        };
        let current = slider_value(slider, cx).unwrap_or(0);
        let next = (i32::from(current) + delta).clamp(0, 100) as u8;
        *pending = None;
        *commit = Task::ready(());
        slider.update(cx, |slider, cx| {
            slider.set_value(f32::from(next), window, cx)
        });
        self.store.update(cx, |store, cx| match which {
            BackgroundSlider::Opacity => store.set_opacity(next, cx),
            BackgroundSlider::Blur => store.set_blur(next, cx),
            BackgroundSlider::TintOpacity => store.set_tint_opacity(next, cx),
        });
    }

    /// A slider row that the arrow keys step while it has keyboard focus.
    fn keyboard_slider(
        &self,
        id: &'static str,
        which: BackgroundSlider,
        row: SettingsRow,
        c: Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .w_full()
            .tab_index(0)
            .focus(move |style| style.bg(c.accent))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                let delta = match event.keystroke.key.as_str() {
                    "left" | "down" => -1,
                    "right" | "up" => 1,
                    "pagedown" => -10,
                    "pageup" => 10,
                    _ => return,
                };
                this.step_slider(which, delta, window, cx);
                cx.stop_propagation();
            }))
            .child(row)
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

impl Render for BackgroundSettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let settings = self.store.read(cx).settings().clone();
        let image_loading = self.store.read(cx).is_loading();
        let images = self.store.read(cx).available();
        let empty_images = images.is_empty();
        let selected = settings.background_image.clone();
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

        let mut image_rows = vec![ListItem::new("background-image-none", c)
            .selected(selected.is_empty())
            .child("None")
            .on_click(cx.listener(|this, _, _, cx| {
                this.update_setting(cx, |store, cx| store.set_image("", cx));
                this.confirm_delete = None;
            }))
            .into_any_element()];
        image_rows.extend(images.into_iter().map(|image| {
            let filename = image.filename;
            let action = filename.clone();
            ListItem::new(
                ElementId::Name(format!("background-image-{filename}").into()),
                c,
            )
            .selected(selected == filename)
            .child(div().min_w_0().overflow_hidden().child(filename))
            .on_click(cx.listener(move |this, _, _window, cx| {
                this.update_setting(cx, |store, cx| store.set_image(action.clone(), cx));
                this.confirm_delete = None;
            }))
            .into_any_element()
        }));

        let selected_for_delete = selected.clone();
        let selected_for_removal = selected.clone();
        let delete_controls = if self.deleting.as_deref() == Some(selected.as_str()) {
            button_disabled(
                "background-delete-pending",
                c,
                ButtonVariant::OutlinedGhost,
                ButtonSize::Xs,
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
                        ButtonSize::Xs,
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
                        ButtonSize::Xs,
                    )
                    .child("Remove image")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.remove_image(selected_for_removal.clone(), cx)
                    })),
                )
                .into_any_element()
        } else if !selected.is_empty() {
            button(
                "background-delete-start",
                c,
                ButtonVariant::OutlinedGhost,
                ButtonSize::Xs,
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
                ButtonSize::Xs,
            )
            .child("Remove selected…")
            .into_any_element()
        };
        let import_button = if self.importing {
            button_disabled(
                "background-import",
                c,
                ButtonVariant::Outlined,
                ButtonSize::Xs,
            )
            .child("Adding image…")
            .into_any_element()
        } else {
            button(
                "background-import",
                c,
                ButtonVariant::Outlined,
                ButtonSize::Xs,
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

        let hairline = || divider(Axis::Horizontal, c.line_subtle);
        div()
            .id("background-settings")
            .flex()
            .flex_col()
            .w_full()
            .child(
                settings_row("background-image", c, "Image")
                    .description("Choose an image shown behind the app and terminal.")
                    .control(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(import_button)
                            .child(delete_controls),
                    ),
            )
            .child(
                div()
                    .id("background-image-list")
                    .flex()
                    .flex_col()
                    .gap(c.space(2.0))
                    .max_h(c.space(220.0))
                    .overflow_y_scroll()
                    .px(c.control_space(12.0))
                    .pb(c.control_space(12.0))
                    .children(image_rows)
                    .when(empty_images, |list| {
                        list.child(
                            div()
                                .px(c.space(8.0))
                                .py(c.space(4.0))
                                .text_size(c.control_space(12.0))
                                .text_color(c.muted)
                                .child("No background images have been added."),
                        )
                    })
                    .when(image_loading, |list| {
                        list.child(
                            div()
                                .px(c.space(8.0))
                                .py(c.space(4.0))
                                .text_size(c.control_space(12.0))
                                .text_color(c.muted)
                                .child("Preparing background preview…"),
                        )
                    }),
            )
            .child(hairline())
            .child(
                self.keyboard_slider(
                    "background-opacity",
                    BackgroundSlider::Opacity,
                    slider_row(
                        "background-opacity-row",
                        c,
                        "Opacity",
                        &self.opacity,
                        format!("{opacity_value} %"),
                    )
                    .description("How strongly the image shows through the interface."),
                    c,
                    cx,
                ),
            )
            .child(hairline())
            .child(self.keyboard_slider(
                "background-blur",
                BackgroundSlider::Blur,
                slider_row(
                    "background-blur-row",
                    c,
                    "Blur",
                    &self.blur,
                    format!("{blur_value} px"),
                ),
                c,
                cx,
            ))
            .child(hairline())
            .child(
                settings_row("background-tint-color", c, "Tint color")
                    .description("A color laid over the image, as #rrggbb.")
                    .control(text_field_surface(
                        "background-tint-color-field",
                        c,
                        tint_state,
                        field_input(&self.tint_input),
                    )),
            )
            .child(hairline())
            .child(self.keyboard_slider(
                "background-tint-opacity",
                BackgroundSlider::TintOpacity,
                slider_row(
                    "background-tint-opacity-row",
                    c,
                    "Tint opacity",
                    &self.tint_opacity,
                    format!("{tint_opacity_value} %"),
                ),
                c,
                cx,
            ))
            .child(hairline())
            .child(
                settings_row("background-fit", c, "Image scaling").control(
                    segmented_control("background-fit-control", c, selected_fit)
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
            .child(hairline())
            .child(
                settings_row("background-target", c, "Show behind").control(
                    segmented_control("background-target-control", c, selected_target)
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
                            this.update_setting(cx, |store, cx| store.set_target(target, cx));
                        })),
                ),
            )
    }
}

fn slider_state(value: u8) -> SliderState {
    SliderState::new()
        .min(0.0)
        .max(100.0)
        .step(1.0)
        .default_value(f32::from(value))
}

fn percent(value: f32) -> u8 {
    value.round().clamp(0.0, 100.0) as u8
}

fn slider_value(state: &Entity<SliderState>, cx: &App) -> Option<u8> {
    match state.read(cx).value() {
        SliderValue::Single(value) => Some(percent(value)),
        SliderValue::Range(_, _) => None,
    }
}

fn valid_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::{percent, valid_hex_color};

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

    #[test]
    fn slider_values_round_and_clamp_to_percent() {
        assert_eq!(percent(-5.0), 0);
        assert_eq!(percent(29.6), 30);
        assert_eq!(percent(140.0), 100);
    }
}
