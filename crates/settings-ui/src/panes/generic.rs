//! Generic field renderer (T19-004): dropdown layer, `render_field`
//! (dispatches on `FieldControl` — the renderer registry), the generated-page
//! renderer (static section headers + trailing "Other" fallback; section
//! navigation lives in the sidebar per `docs/architecture.md` §8.3), the
//! top-level `render_body` dispatch.
//!
//! Part of `SettingsView` — see `crate::view`.

use crate::view::*;
use labonair_settings::SettingsSurfaceMode;
use labonair_settings_content::file_manager::{default_sftp_columns, SftpColumn};
use labonair_ui_kit::icon_button_builder;

/// Parse a stored `sftpColumns` JSON value into an ordered, de-duplicated
/// column list, falling back to the shipped default when absent/unparseable.
fn sftp_visible_columns(value: Option<&Value>) -> Vec<SftpColumn> {
    let Some(arr) = value.and_then(|v| v.as_array()) else {
        return default_sftp_columns();
    };
    let mut out = Vec::new();
    for tok in arr.iter().filter_map(|t| t.as_str()) {
        if let Some(col) = SftpColumn::from_token(tok) {
            if !out.contains(&col) {
                out.push(col);
            }
        }
    }
    out
}

fn field_disabled_in_scope(scope: SettingsScope, json_path: &str) -> bool {
    scope == SettingsScope::Project
        && !labonair_settings::project::is_project_setting_allowed(json_path)
}

impl SettingsView {
    /// Load system fonts off the UI thread for the shared `FontFamily` field
    /// renderer. Font selection is a Settings value; the picker itself is
    /// owned by the generic settings UI rather than a theme-management pane.
    pub(crate) fn load_system_fonts(&mut self, cx: &mut Context<Self>) {
        if self.font_loading || self.font_loaded || self.font_error.is_some() {
            return;
        }
        self.font_loading = true;
        self.font_error = None;
        cx.notify();
        let service = self.services.fonts.clone();
        let task = self.tokio.spawn(async move { service.list().await });
        cx.spawn(async move |this, cx| match task.await {
            Ok(Ok(mut names)) => {
                names.sort_by_key(|name| name.to_lowercase());
                let _ = this.update(cx, |this, cx| {
                    this.font_loading = false;
                    this.system_fonts = names.into_iter().map(SharedString::from).collect();
                    this.font_loaded = true;
                    cx.notify();
                });
            }
            Ok(Err(error)) => {
                let _ = this.update(cx, |this, cx| {
                    this.font_loading = false;
                    this.font_error = Some(error.clone());
                    this.notify_error(cx, "System fonts", error);
                    cx.notify();
                });
            }
            Err(error) => {
                let _ = this.update(cx, |this, cx| {
                    this.font_loading = false;
                    this.font_error = Some(error.to_string());
                    this.notify_error(cx, "System fonts", error.to_string());
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// The floating options list for an open `Select`/`FontFamily` dropdown
    /// (T16-010). Rendered as a `deferred` + `anchored` layer so it is not
    /// clipped by the scroll area, with a transparent full-window backdrop
    /// that dismisses it. `menu.key` is a field's `json_path`.
    pub(crate) fn render_dropdown(
        &mut self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let menu = self.dropdown.as_ref()?;
        let json_path = menu.key;
        let anchor = menu.at;
        let options: Vec<SelectOption> = menu.options.clone();
        let highlighted = menu
            .options
            .get(menu.highlighted)
            .map(|(token, _)| token.clone());
        let sentinel = menu.default_sentinel.clone();
        let stored = self
            .field_by_path(json_path)
            .and_then(|f| self.field_value(f, cx))
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        let cur: SharedString = if stored.is_empty() {
            sentinel.clone().unwrap_or_default()
        } else {
            SharedString::from(stored)
        };
        let highlighted = highlighted.unwrap_or(cur);
        let view = cx.entity();
        Some(select_popover(
            SelectPopoverAnchor::new("settings-dropdown", anchor, self.select_list.clone()),
            *c,
            &options,
            highlighted.as_ref(),
            {
                let v = view.clone();
                move |_w, cx| {
                    v.update(cx, |this, cx| {
                        this.dropdown = None;
                        cx.notify();
                    })
                }
            },
            move |token, _w, cx| {
                let is_sentinel = sentinel.as_ref() == Some(token);
                let token = token.clone();
                view.update(cx, |this, cx| {
                    this.dropdown = None;
                    let v = if is_sentinel {
                        String::new()
                    } else {
                        token.to_string()
                    };
                    this.set_field_value(json_path, Value::String(v), cx);
                });
            },
        ))
    }

    /// Render one generated field row: label/description + modified source +
    /// reset (rule 5) + a control chosen by `FieldControl` (rule 3's
    /// renderer registry — `bool → Switch`, numeric → stepper, `enum`/closed
    /// `String` → dropdown, `String` → text input, anything else → the raw
    /// JSON fallback).
    ///
    /// `source` + `value` are passed in already computed: the batch renderers
    /// (`render_generated_body` and the search list) resolve the field inside
    /// the virtualized row callback, so only visible rows query the store.
    /// An invisible overlay that records a select trigger's window-space
    /// bounds (keyed by `json_path`) on every paint, so the dropdown can drop
    /// from the trigger's bottom-left instead of the click position (P2.1).
    pub(crate) fn select_bounds_probe(
        &self,
        json_path: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let weak = cx.weak_entity();
        canvas(
            move |bounds, _w, cx| {
                let _ = weak.update(cx, |this, cx| {
                    if this.select_bounds.get(json_path) != Some(&bounds) {
                        this.select_bounds.insert(json_path, bounds);
                        cx.notify();
                    }
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full()
    }

    /// Anchor for the dropdown opened from the `json_path` trigger: its
    /// recorded bottom-left, or `click` before the first paint records bounds.
    pub(crate) fn select_anchor(
        &self,
        json_path: &'static str,
        click: Point<Pixels>,
    ) -> Point<Pixels> {
        self.select_bounds
            .get(json_path)
            .map(|b| b.bottom_left())
            .unwrap_or(click)
    }

    fn open_select_dropdown(
        &mut self,
        json_path: &'static str,
        options: Vec<SelectOption>,
        default_sentinel: Option<SharedString>,
        highlighted: usize,
        fallback: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let option_count = options.len();
        self.dropdown = Some(SelectMenu {
            key: json_path,
            options,
            at: self.select_anchor(json_path, fallback),
            default_sentinel,
            highlighted,
        });
        self.select_list.reset(option_count);
        self.select_list.scroll_to(ListOffset {
            item_ix: highlighted,
            offset_in_item: px(0.0),
        });
        cx.notify();
    }

    fn activate_select_trigger(
        &mut self,
        json_path: &'static str,
        event: &ClickEvent,
        options: Vec<SelectOption>,
        default_sentinel: Option<SharedString>,
        highlighted: usize,
        cx: &mut Context<Self>,
    ) {
        if self
            .dropdown
            .as_ref()
            .is_some_and(|menu| menu.key == json_path)
        {
            if matches!(event, ClickEvent::Keyboard(_)) {
                let selected = self.dropdown.take().and_then(|menu| {
                    menu.options
                        .get(menu.highlighted)
                        .cloned()
                        .map(|(token, _)| (token, menu.default_sentinel))
                });
                if let Some((token, sentinel)) = selected {
                    let value = if sentinel.as_ref() == Some(&token) {
                        String::new()
                    } else {
                        token.to_string()
                    };
                    self.set_field_value(json_path, Value::String(value), cx);
                }
            } else {
                self.dropdown = None;
            }
            cx.notify();
            return;
        }

        self.open_select_dropdown(
            json_path,
            options,
            default_sentinel,
            highlighted,
            event.position(),
            cx,
        );
    }

    fn render_field(
        &mut self,
        field: &AnyField,
        source: SettingSource,
        value: Option<Value>,
        window: &mut Window,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let json_path = field.json_path;
        let field_disabled = field_disabled_in_scope(self.scope, json_path);
        // Tile groups are wide, so their row puts the control under the title.
        let mut control_below = false;
        // Slider read-outs carry their own unit.
        let mut unit_in_control = false;
        let control = match field.control {
            FieldControl::Switch => {
                let on = value.as_ref().and_then(|v| v.as_bool()).unwrap_or(false);
                let view = cx.entity();
                let switch =
                    labonair_ui_kit::switch(SharedString::from(format!("sw-{json_path}")), *c, on)
                        .disabled(field_disabled)
                        .on_click(move |_: &bool, _window, app| {
                            view.update(app, |this, cx| {
                                if let Some(f) = this.field_by_path(json_path).copied() {
                                    this.toggle_bool(&f, cx);
                                }
                            });
                        });
                let surface = div()
                    .id(SharedString::from(format!("settings-switch-{json_path}")))
                    .rounded(px(c.radius.sm))
                    .border_1()
                    .border_color(gpui::transparent_black())
                    .child(switch);
                if field_disabled {
                    surface.tab_index(-1).cursor_default().into_any_element()
                } else {
                    surface
                        .tab_index(0)
                        .focus(|style| style.border_1().border_color(c.ring))
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _w, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                if let Some(f) = this.field_by_path(json_path).copied() {
                                    this.toggle_bool(&f, cx);
                                }
                                cx.stop_propagation();
                            }
                        }))
                        .into_any_element()
                }
            }
            // T20-001: both numeric controls are the shared `NumberField`
            // primitive now — it owns the stepper chrome, the filled track and
            // the min/max/step clamping.
            FieldControl::Int { min, max, step } => {
                let cur = value.as_ref().and_then(|v| v.as_i64()).unwrap_or(min);
                let editor = (self.number_input_key == Some(json_path))
                    .then(|| self.number_input.clone())
                    .flatten();
                number_field(
                    SharedString::from(format!("int-{json_path}")),
                    *c,
                    cur as f64,
                    min as f64,
                    max as f64,
                    step as f64,
                )
                .disabled(field_disabled)
                .track(false)
                .editor(editor)
                .on_edit({
                    let view = cx.entity();
                    move |window, app| {
                        view.update(app, |this, cx| {
                            this.begin_number_edit(json_path, window, cx);
                        });
                    }
                })
                .on_change(cx.listener(move |this, next: &f64, _w, cx| {
                    this.set_field_value(json_path, Value::from(*next as i64), cx);
                }))
                .into_any_element()
            }
            FieldControl::Float {
                min_centi,
                max_centi,
                step_centi,
            } => {
                let cur = value
                    .as_ref()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(min_centi as f64 / 100.0);
                let editor = (self.number_input_key == Some(json_path))
                    .then(|| self.number_input.clone())
                    .flatten();
                number_field(
                    SharedString::from(format!("float-{json_path}")),
                    *c,
                    cur,
                    min_centi as f64 / 100.0,
                    max_centi as f64 / 100.0,
                    step_centi as f64 / 100.0,
                )
                .disabled(field_disabled)
                .decimals(2)
                .track(false)
                .editor(editor)
                .on_edit({
                    let view = cx.entity();
                    move |window, app| {
                        view.update(app, |this, cx| {
                            this.begin_number_edit(json_path, window, cx);
                        });
                    }
                })
                .on_change(cx.listener(move |this, next: &f64, _w, cx| {
                    this.set_float_field(json_path, *next, cx);
                }))
                .into_any_element()
            }
            FieldControl::Select(opts) => {
                let cur = value
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let label = opts
                    .iter()
                    .find(|(tok, _)| *tok == cur)
                    .map(|(_, l)| *l)
                    .unwrap_or(&cur);
                let selected_index = opts
                    .iter()
                    .position(|(token, _)| *token == cur)
                    .unwrap_or(0);
                let options: Vec<SelectOption> = opts
                    .iter()
                    .map(|(token, label)| (SharedString::from(*token), SharedString::from(*label)))
                    .collect();
                let keyboard_options = options.clone();
                let is_open = self.dropdown.as_ref().is_some_and(|d| d.key == json_path);
                let trigger = if field_disabled {
                    select_trigger_disabled(
                        SharedString::from(format!("sel-{json_path}")),
                        *c,
                        SharedString::from(label.to_string()),
                    )
                    .into_any_element()
                } else {
                    select_trigger(
                        SharedString::from(format!("sel-{json_path}")),
                        *c,
                        SharedString::from(label.to_string()),
                        is_open,
                    )
                    .relative()
                    .child(self.select_bounds_probe(json_path, cx))
                    .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                        this.activate_select_trigger(
                            json_path,
                            ev,
                            options.clone(),
                            None,
                            selected_index,
                            cx,
                        );
                    }))
                    .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _w, cx| {
                        let key = ev.keystroke.key.as_str();
                        if matches!(key, "enter" | "space") {
                            cx.stop_propagation();
                        } else if matches!(key, "up" | "down")
                            && this
                                .dropdown
                                .as_ref()
                                .is_none_or(|menu| menu.key != json_path)
                        {
                            let highlighted = if key == "up" && !keyboard_options.is_empty() {
                                (selected_index + keyboard_options.len() - 1)
                                    % keyboard_options.len()
                            } else {
                                selected_index
                            };
                            this.open_select_dropdown(
                                json_path,
                                keyboard_options.clone(),
                                None,
                                highlighted,
                                Point::default(),
                                cx,
                            );
                            cx.stop_propagation();
                        }
                    }))
                    .into_any_element()
                };
                trigger
            }
            FieldControl::FontFamily => {
                let cur = value
                    .as_ref()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default();
                let is_open = self.dropdown.as_ref().is_some_and(|d| d.key == json_path);
                let label = if self.font_loading {
                    "Loading fonts…".to_string()
                } else if self.font_error.is_some() {
                    "Fonts unavailable".to_string()
                } else if self.font_loaded && self.system_fonts.is_empty() {
                    "No system fonts found".to_string()
                } else if cur.is_empty() {
                    "(default)".to_string()
                } else {
                    cur.clone()
                };
                let fonts = self.system_fonts.clone();
                let selected_font_index = if cur.is_empty() {
                    0
                } else {
                    fonts
                        .iter()
                        .position(|font| font.as_ref() == cur)
                        .map(|index| index + 1)
                        .unwrap_or(0)
                };
                let sentinel = SharedString::from("(default)");
                let click_sentinel = sentinel.clone();
                let key_sentinel = sentinel.clone();
                let mut options = vec![(sentinel.clone(), sentinel.clone())];
                options.extend(fonts.iter().map(|font| (font.clone(), font.clone())));
                let keyboard_options = options.clone();
                let mut trigger = if field_disabled {
                    select_trigger_disabled(
                        SharedString::from(format!("font-{json_path}")),
                        *c,
                        SharedString::from(label),
                    )
                } else {
                    select_trigger(
                        SharedString::from(format!("font-{json_path}")),
                        *c,
                        SharedString::from(label),
                        is_open,
                    )
                }
                .min_w(px(200.0));
                if !field_disabled {
                    trigger = trigger
                        .relative()
                        .child(self.select_bounds_probe(json_path, cx))
                        .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                            this.activate_select_trigger(
                                json_path,
                                ev,
                                options.clone(),
                                Some(click_sentinel.clone()),
                                selected_font_index,
                                cx,
                            );
                        }))
                        .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _w, cx| {
                            let key = ev.keystroke.key.as_str();
                            if matches!(key, "enter" | "space") {
                                cx.stop_propagation();
                            } else if matches!(key, "up" | "down")
                                && this
                                    .dropdown
                                    .as_ref()
                                    .is_none_or(|menu| menu.key != json_path)
                            {
                                let highlighted = if key == "up" && !keyboard_options.is_empty() {
                                    (selected_font_index + keyboard_options.len() - 1)
                                        % keyboard_options.len()
                                } else {
                                    selected_font_index
                                };
                                this.open_select_dropdown(
                                    json_path,
                                    keyboard_options.clone(),
                                    Some(key_sentinel.clone()),
                                    highlighted,
                                    Point::default(),
                                    cx,
                                );
                                cx.stop_propagation();
                            }
                        }));
                }
                if self.font_error.is_some() {
                    h_stack()
                        .items_center()
                        .gap(c.space(4.0))
                        .child(trigger)
                        .child(
                            button(
                                SharedString::from(format!("font-retry-{json_path}")),
                                *c,
                                ButtonVariant::Subtle,
                                ButtonSize::Xs,
                            )
                            .child("Retry")
                            .on_click(cx.listener(
                                |this, _: &ClickEvent, _w, cx| {
                                    this.font_error = None;
                                    this.load_system_fonts(cx);
                                },
                            )),
                        )
                        .into_any_element()
                } else {
                    trigger.into_any_element()
                }
            }
            FieldControl::Text => self.render_text_control(json_path, value, field_disabled, c, cx),
            FieldControl::SftpColumns => {
                self.render_sftp_columns_control(json_path, value, field_disabled, c, cx)
            }
            FieldControl::Segmented(opts) => {
                let current = value
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned();
                let view = cx.entity();
                let mut segments =
                    segmented_control(SharedString::from(format!("seg-{json_path}")), *c, current)
                        .variant(SegmentVariant::Outline)
                        .size(SegmentSize::Sm)
                        .disabled(field_disabled);
                for (token, label) in opts {
                    segments = segments.segment(*token, *label);
                }
                segments
                    .on_select(move |token, _window, app| {
                        let token = token.to_string();
                        view.update(app, |this, cx| {
                            this.set_field_value(json_path, Value::String(token), cx);
                        });
                    })
                    .into_any_element()
            }
            FieldControl::Slider { min, int, .. } => {
                unit_in_control = true;
                let current = value.as_ref().and_then(|v| v.as_f64()).unwrap_or(min);
                let readout = slider_readout(current, int, field.meta.unit);
                match self.slider_state(field, current, window, cx) {
                    Some(state) => {
                        slider_control(&state, *c, readout, field_disabled).into_any_element()
                    }
                    None => div().into_any_element(),
                }
            }
            FieldControl::Color => {
                let stored = value.as_ref().and_then(|v| v.as_str()).unwrap_or("");
                let picker = self.color_state(json_path, stored, window, cx);
                ColorPicker::new(&picker).into_any_element()
            }
            FieldControl::Tiles(options) => {
                control_below = true;
                let current = value
                    .as_ref()
                    .and_then(tile_token)
                    .unwrap_or_default()
                    .to_owned();
                let stored = value.clone();
                let view = cx.entity();
                let mut group = preview_tile_group(
                    SharedString::from(format!("tiles-{json_path}")),
                    *c,
                    current,
                )
                .disabled(field_disabled);
                for option in options {
                    group = group.tile(preview_tile(option.token, option.label, option.preview));
                }
                group
                    .on_select(move |token, _window, app| {
                        let next = tile_value(stored.as_ref(), token);
                        view.update(app, |this, cx| this.set_field_value(json_path, next, cx));
                    })
                    .into_any_element()
            }
        };
        let control = if let (Some(unit), false) = (field.meta.unit, unit_in_control) {
            div()
                .flex()
                .items_center()
                .gap(c.space(8.0))
                .child(control)
                .child(div().text_size(px(12.0)).text_color(c.muted).child(unit))
                .into_any_element()
        } else {
            control
        };

        let modified_in = source.modified_in();
        // T19-007: a search jump briefly pulses the target row so the user
        // can find it among a page's other fields.
        let highlighted = self.highlight == Some(json_path);

        let copied_link = self.last_copied_link_path == Some(json_path);
        let copy_link = icon_button_builder(
            SharedString::from(format!("copy-link-{json_path}")),
            *c,
            if copied_link {
                IconName::Check
            } else {
                IconName::Link
            },
        )
        .variant(ButtonVariant::Subtle)
        .size(ButtonSize::IconXs)
        .shape(IconButtonShape::Square)
        .icon_color(if copied_link { c.success } else { c.muted })
        .tooltip("Copy Link")
        .render()
        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
            this.copy_setting_link(json_path, cx);
        }))
        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _window, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                this.copy_setting_link(json_path, cx);
                cx.stop_propagation();
            }
        }))
        .invisible()
        .group_hover("settings-field-row", |style| style.visible());

        // Reset sits beside the control and appears only for overridden values.
        let mut actions = h_stack().items_center().gap(c.space(2.0)).child(copy_link);
        if modified_in.is_some() {
            actions = actions.child(
                icon_button_builder(
                    SharedString::from(format!("reset-{json_path}")),
                    *c,
                    IconName::Undo,
                )
                .variant(ButtonVariant::Subtle)
                .size(ButtonSize::IconXs)
                .shape(IconButtonShape::Square)
                .tooltip("Reset to default")
                .render()
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    this.reset_field(json_path, cx);
                }))
                .on_key_down(cx.listener(
                    move |this, event: &KeyDownEvent, _window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.reset_field(json_path, cx);
                            cx.stop_propagation();
                        }
                    },
                )),
            );
        }

        // Secondary notes under the description: hint, override source and the
        // project-scope explanation.
        let mut notes = v_stack().gap(c.space(2.0));
        if let Some(hint) = field.meta.hint {
            notes = notes.child(
                div()
                    .text_size(c.control_space(11.0))
                    .text_color(c.muted.opacity(0.85))
                    .child(hint),
            );
        }
        if let Some(modified_in) = modified_in {
            notes = notes.child(
                div()
                    .text_size(c.control_space(11.0))
                    .text_color(c.muted)
                    .child(SharedString::from(format!("Modified in {modified_in}"))),
            );
        }
        if field_disabled {
            notes = notes.child(
                div()
                    .text_size(c.control_space(11.0))
                    .text_color(c.muted)
                    .child("Unavailable in Project settings."),
            );
        }

        let row = settings_row(
            SharedString::from(format!("row-{json_path}")),
            *c,
            SharedString::from(field.meta.title),
        )
        .description(SharedString::from(field.meta.description))
        .extra(notes)
        .reset(actions);
        let content = if control_below {
            v_stack()
                .child(row)
                .child(
                    div()
                        .px(c.control_space(16.0))
                        .pb(c.control_space(16.0))
                        .child(control),
                )
                .into_any_element()
        } else {
            row.control(control).into_any_element()
        };

        let is_slider = matches!(field.control, FieldControl::Slider { .. });
        div()
            .id(SharedString::from(format!("field-row-{json_path}")))
            .group("settings-field-row")
            .w_full()
            .min_w_0()
            .when(highlighted, |d| d.bg(c.selected_fill.opacity(0.7)))
            .when(is_slider && !field_disabled, |d| {
                d.tab_index(0)
                    .focus(|style| style.bg(c.accent))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _w, cx| {
                        let direction = match event.keystroke.key.as_str() {
                            "left" | "down" => -1.0,
                            "right" | "up" => 1.0,
                            _ => return,
                        };
                        this.step_slider(json_path, direction, cx);
                        cx.stop_propagation();
                    }))
            })
            .child(content)
            .into_any_element()
    }

    /// Text fields are always real UI-kit `InputState` editors. Their entities
    /// are created once by `SettingsView::ensure_text_inputs`, so virtualized
    /// row rebuilding never turns an active editor into a text preview.
    fn render_text_control(
        &self,
        json_path: &'static str,
        _value: Option<Value>,
        disabled: bool,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(input) = self.text_inputs.get(json_path) else {
            return div()
                .min_w(c.control_space(256.0))
                .text_color(c.muted)
                .child("(input unavailable)")
                .into_any_element();
        };
        let focused = self.text_input_key == Some(json_path);
        let field = text_field_surface(
            SharedString::from(format!("txt-{json_path}")),
            *c,
            if disabled {
                TextFieldState::Disabled
            } else if focused {
                TextFieldState::Focused
            } else {
                TextFieldState::Normal
            },
            text_input(input, *c)
                .disabled(disabled)
                .tab_index(if disabled { -1 } else { 0 })
                .text_color(if disabled { c.muted } else { c.fg }),
        )
        .min_w(c.control_space(256.0));
        if disabled {
            field.into_any_element()
        } else {
            field
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.begin_text_edit(json_path, window, cx);
                }))
                .into_any_element()
        }
    }

    /// Ordered visible-column editor for the SFTP browser
    /// (`FieldControl::SftpColumns`). Each column gets a checkbox
    /// (visible/hidden) and up/down reorder buttons; the stored value is a
    /// JSON array of column tokens in display order.
    fn render_sftp_columns_control(
        &self,
        json_path: &'static str,
        value: Option<Value>,
        disabled: bool,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let visible = sftp_visible_columns(value.as_ref());

        // Rows: the visible columns in order, then the hidden ones.
        let mut ordered = visible.clone();
        for col in SftpColumn::ALL {
            if !ordered.contains(&col) {
                ordered.push(col);
            }
        }

        let mut stack = v_stack().gap(px(2.0));
        for col in ordered {
            let vis_pos = visible.iter().position(|x| *x == col);
            let is_visible = vis_pos.is_some();
            let can_up = vis_pos.is_some_and(|i| i > 0);
            let can_down = vis_pos.is_some_and(|i| i + 1 < visible.len());

            let arrow = |icon: IconName, id: &str, enabled: bool, delta: i32| {
                let tooltip = SharedString::from(format!(
                    "Move {} column {}",
                    col.label(),
                    if delta < 0 { "up" } else { "down" }
                ));
                let enabled = enabled && !disabled;
                let button_id = SharedString::from(format!("{id}-{}", col.token()));
                let mut button = icon_button_builder(button_id, *c, icon)
                    .variant(ButtonVariant::Subtle)
                    .size(ButtonSize::IconXs)
                    .shape(IconButtonShape::Square)
                    .disabled(!enabled);
                if enabled {
                    button = button.tooltip(tooltip);
                    button
                        .render()
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.sftp_columns_move(json_path, col, delta, cx);
                        }))
                } else {
                    button.render()
                }
            };

            stack = stack.child(
                h_stack()
                    .gap(px(4.0))
                    .items_center()
                    .child(
                        checkbox(
                            SharedString::from(format!("sftpcol-{}", col.token())),
                            *c,
                            is_visible,
                        )
                        .label(col.label())
                        .disabled(disabled)
                        .on_click(cx.listener(
                            move |this, checked: &bool, _w, cx| {
                                this.sftp_columns_toggle(json_path, col, *checked, cx);
                            },
                        )),
                    )
                    .child(div().flex_1())
                    .child(arrow(IconName::ArrowUp, "sftpcol-up", can_up, -1))
                    .child(arrow(IconName::ArrowDown, "sftpcol-down", can_down, 1)),
            );
        }
        v_stack()
            .w(px(240.0))
            .gap(px(4.0))
            .child(stack)
            .child(
                div().text_size(px(10.5)).text_color(c.muted).child(
                    "Order also adjusts by dragging the column headers in the SFTP browser.",
                ),
            )
            .into_any_element()
    }

    fn sftp_columns_write(
        &mut self,
        json_path: &'static str,
        cols: &[SftpColumn],
        cx: &mut Context<Self>,
    ) {
        let arr = Value::Array(
            cols.iter()
                .map(|col| Value::String(col.token().to_string()))
                .collect(),
        );
        self.set_field_value(json_path, arr, cx);
    }

    fn sftp_columns_toggle(
        &mut self,
        json_path: &'static str,
        col: SftpColumn,
        checked: bool,
        cx: &mut Context<Self>,
    ) {
        let field = self.field_by_path(json_path).copied();
        let cur = field.and_then(|f| self.field_value(&f, cx));
        let mut cols = sftp_visible_columns(cur.as_ref());
        cols.retain(|x| *x != col);
        if checked {
            cols.push(col);
        }
        self.sftp_columns_write(json_path, &cols, cx);
    }

    fn sftp_columns_move(
        &mut self,
        json_path: &'static str,
        col: SftpColumn,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let field = self.field_by_path(json_path).copied();
        let cur = field.and_then(|f| self.field_value(&f, cx));
        let mut cols = sftp_visible_columns(cur.as_ref());
        let Some(i) = cols.iter().position(|x| *x == col) else {
            return;
        };
        let j = i as i32 + delta;
        if j < 0 || j as usize >= cols.len() {
            return;
        }
        cols.swap(i, j as usize);
        self.sftp_columns_write(json_path, &cols, cx);
    }

    // ── T19-004: top-level render dispatch ──────────────────────────────

    pub(crate) fn render_body(
        &mut self,
        window: &mut Window,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        // T19-007: search results occupy the main surface and use the same
        // canonical field renderer as ordinary category pages. The sidebar
        // remains stable, so search never replaces the user's orientation.
        if !self.search.trim().is_empty() {
            return self.render_search_body(c, cx);
        }
        self.render_generated_body(window, c, cx)
    }

    fn render_search_body(&mut self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let query = self.search.trim().to_owned();
        let mut blocks: Vec<SectionBlock> = Vec::new();
        let mut last_page = None;
        for row in self.search_results.clone() {
            if last_page != Some(row.page_index) {
                last_page = Some(row.page_index);
                blocks.push(SectionBlock {
                    title: Some(row.page_title),
                    items: Vec::new(),
                });
            }
            let Some(block) = blocks.last_mut() else {
                continue;
            };
            match row.target {
                SearchTarget::Field(index) => {
                    if let Some(field) = self.all_fields.get(index).copied() {
                        block.items.push(SectionItem::Field(field));
                    }
                }
                SearchTarget::OwnerSurface(surface) => {
                    block.items.push(SectionItem::OwnerSurface(surface));
                }
            }
        }

        let header = v_stack()
            .gap(c.space(4.0))
            .pb(c.space(16.0))
            .child(
                div()
                    .text_size(c.control_space(15.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c.fg)
                    .child("Search settings"),
            )
            .child(
                div()
                    .text_size(c.control_space(12.0))
                    .text_color(c.muted)
                    .child(SharedString::from(format!(
                        "Results for \u{201C}{query}\u{201D}"
                    ))),
            );

        if blocks.is_empty() {
            self.prepare_content_list("search-empty".to_string(), 0);
            return div()
                .id("settings-search-scroll")
                .flex_1()
                .min_h_0()
                .w_full()
                .flex()
                .flex_col()
                .p(c.space(24.0))
                .child(header)
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(c.control_space(13.0))
                        .text_color(c.muted)
                        .child("No setting found for this search."),
                )
                .into_any_element();
        }

        self.prepare_content_list(format!("search:{query}"), blocks.len());
        let list = self.block_list(blocks, false, c, cx);
        div()
            .id("settings-search-scroll")
            .flex_1()
            .min_h_0()
            .w_full()
            .flex()
            .flex_col()
            .p(c.space(24.0))
            .child(header)
            .child(list)
            .into_any_element()
    }

    /// Render the active page as grouped cards: one `settings_group` card per
    /// section (title above it), with a trailing "Other" card for any field no
    /// curated group placed. Each section is one virtualized list item.
    fn render_generated_body(
        &mut self,
        window: &mut Window,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let PageBody::Generated(items) = self.active_body();
        let surface_id = items.iter().find_map(|item| match item {
            SettingsPageItem::OwnerSurface(id) => Some(*id),
            _ => None,
        });
        if let Some(view) = surface_id.and_then(|id| self.full_page_surface(id, window, cx)) {
            return view;
        }
        let PageBody::Generated(items) = self.active_body();
        // The trailing "Other" fallback belongs on the category's main page only —
        // a sub-page shows just its own curated groups.
        let leftover: Vec<AnyField> = if self.active_subpage.is_none() {
            leftover_fields(self.active_area, &self.pages, &self.all_fields)
                .into_iter()
                .copied()
                .collect()
        } else {
            Vec::new()
        };
        let blocks = section_blocks(items, &self.all_fields, &leftover);

        // A sidebar anchor click or a search jump asks to land on a section or
        // a specific field; both resolve to a block index.
        let pending_scroll = self.pending_scroll;
        let scroll_to_block = pending_scroll.and_then(|path| {
            blocks.iter().position(|block| {
                block
                    .items
                    .iter()
                    .any(|item| matches!(item, SectionItem::Field(f) if f.json_path == path))
            })
        });
        let section_block = self
            .scroll_to_section
            .take()
            .and_then(|target| blocks.iter().position(|block| block.title == Some(target)));

        self.prepare_content_list(
            format!(
                "area:{}:{}",
                self.active_area,
                self.active_subpage.unwrap_or(usize::MAX)
            ),
            blocks.len(),
        );
        if let Some(index) = scroll_to_block {
            self.content_list.scroll_to_reveal_item(index);
            self.pending_scroll = None;
        }
        if let Some(index) = section_block {
            self.content_list.scroll_to_reveal_item(index);
        }

        let list = self.block_list(blocks, true, c, cx);
        div()
            .id("settings-scroll")
            .flex_1()
            .min_h_0()
            .w_full()
            .flex()
            .flex_col()
            .px(c.space(24.0))
            .pt(c.space(8.0))
            .child(list)
            .into_any_element()
    }

    /// The virtualized list of section cards. `embed` lets inline owner
    /// surfaces render their view (normal pages); search results show a link.
    fn block_list(
        &mut self,
        blocks: Vec<SectionBlock>,
        embed: bool,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let list_state = self.content_list.clone();
        let view = cx.entity();
        let palette = *c;
        list(list_state, move |index, window, app| {
            let Some(block) = blocks.get(index) else {
                return div().into_any_element();
            };
            view.update(app, |this, cx| {
                this.render_block(index, block, embed, window, &palette, cx)
            })
        })
        .flex_1()
        .min_h_0()
        .into_any_element()
    }

    fn prepare_content_list(&mut self, key: String, count: usize) {
        if self.content_list_key.as_deref() != Some(key.as_str())
            || self.content_list.item_count() != count
        {
            self.content_list.reset(count);
            self.content_list_key = Some(key);
        }
    }

    /// Whether the active page is a `full_page` inline surface: it owns the
    /// keyboard (its own inputs and shortcuts), so typing must not reach the
    /// Settings search.
    pub(crate) fn full_page_active(&self) -> bool {
        let PageBody::Generated(items) = self.active_body();
        items.iter().any(|item| match item {
            SettingsPageItem::OwnerSurface(id) => {
                self.services.surface(*id).is_some_and(|surface| {
                    surface.full_page && surface.mode == SettingsSurfaceMode::Inline
                })
            }
            _ => false,
        })
    }

    /// The embedded view of a page that is one `full_page` inline surface, laid
    /// out in the whole content area (the surface scrolls itself).
    fn full_page_surface(
        &mut self,
        id: SettingsSurfaceId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let surface = self.services.surface(id)?;
        if !(surface.full_page && surface.mode == SettingsSurfaceMode::Inline) {
            return None;
        }
        if !self.inline_views.contains_key(&id) {
            let view = self.services.inline_surface(id, window, cx)?;
            self.inline_views.insert(id, view);
        }
        let view = self.inline_views.get(&id)?.clone();
        Some(
            div()
                .id(SharedString::from(format!(
                    "settings-inline-{}",
                    id.as_str()
                )))
                .flex_1()
                .min_h_0()
                .w_full()
                .flex()
                .flex_col()
                .child(view)
                .into_any_element(),
        )
    }

    /// One section: a 15px semibold title above a `settings_group` card of rows.
    fn render_block(
        &mut self,
        index: usize,
        block: &SectionBlock,
        embed: bool,
        window: &mut Window,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let mut group = settings_group(
            SharedString::from(format!("settings-group-{}-{index}", self.active_area)),
            *c,
        );
        for item in &block.items {
            let row = match item {
                SectionItem::Field(field) => {
                    let source = self.field_source(field, cx);
                    let value = self.field_value(field, cx);
                    self.render_field(field, source, value, window, c, cx)
                }
                SectionItem::OwnerSurface(id) => {
                    self.render_owner_surface_row(*id, embed, window, c, cx)
                }
            };
            group = group.row(row);
        }
        v_stack()
            .w_full()
            .gap(c.space(8.0))
            .pb(c.space(24.0))
            .children(block.title.map(|title| {
                div()
                    .px(c.control_space(4.0))
                    .text_size(c.control_space(15.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c.fg)
                    .child(SharedString::from(title))
            }))
            .child(group)
            .into_any_element()
    }

    /// An owner surface row. Inline surfaces embed the owner's view (the host
    /// card supplies the surface; the view contributes rows); link surfaces and
    /// search results show a title, description and one button.
    fn render_owner_surface_row(
        &mut self,
        id: SettingsSurfaceId,
        embed: bool,
        window: &mut Window,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(surface) = self.services.surface(id) else {
            tracing::error!(
                "Settings page references an unregistered surface `{}`",
                id.as_str()
            );
            return div().into_any_element();
        };
        let inline = surface.mode == SettingsSurfaceMode::Inline;
        if embed && inline {
            if !self.inline_views.contains_key(&id) {
                if let Some(view) = self.services.inline_surface(id, window, cx) {
                    self.inline_views.insert(id, view);
                }
            }
            if let Some(view) = self.inline_views.get(&id) {
                return div()
                    .id(SharedString::from(format!(
                        "settings-inline-{}",
                        id.as_str()
                    )))
                    .w_full()
                    .min_w_0()
                    .child(view.clone())
                    .into_any_element();
            }
        }
        let button_id = SharedString::from(format!("settings-surface-{}", id.as_str()));
        let label = if inline { "Show" } else { surface.action_label };
        let activate = move |this: &mut Self, cx: &mut Context<Self>| {
            if inline {
                this.show_surface(id, cx);
            } else if let Err(error) = this.services.open_surface(id, cx) {
                tracing::error!("could not open Settings surface: {error}");
                this.notify_error(cx, "Settings", error);
            }
        };
        let button = button(button_id, *c, ButtonVariant::OutlinedGhost, ButtonSize::Xs)
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| activate(this, cx)))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    activate(this, cx);
                    cx.stop_propagation();
                }
            }));
        settings_row(
            SharedString::from(format!("settings-row-{}", id.as_str())),
            *c,
            surface.title,
        )
        .description(surface.description)
        .control(button)
        .into_any_element()
    }

    /// Navigate to the page section that hosts `id` (inline surfaces opened
    /// from a search hit).
    pub(crate) fn show_surface(&mut self, id: SettingsSurfaceId, cx: &mut Context<Self>) {
        if let Some((page, section)) = surface_location(&self.pages, id) {
            self.go_to_section(page, section, cx);
        }
    }
}

/// One virtualized list item: an optional title and the rows of its card.
#[derive(Clone)]
struct SectionBlock {
    title: Option<&'static str>,
    items: Vec<SectionItem>,
}

#[derive(Clone)]
enum SectionItem {
    Field(AnyField),
    OwnerSurface(SettingsSurfaceId),
}

/// Group a page's items into section cards. Rows before the first header form
/// an untitled card; `leftover` fields become a trailing "Other" card; empty
/// sections are dropped.
fn section_blocks(
    items: &[SettingsPageItem],
    fields: &[AnyField],
    leftover: &[AnyField],
) -> Vec<SectionBlock> {
    let mut blocks: Vec<SectionBlock> = Vec::new();
    fn current(blocks: &mut Vec<SectionBlock>) -> &mut SectionBlock {
        if blocks.is_empty() {
            blocks.push(SectionBlock {
                title: None,
                items: Vec::new(),
            });
        }
        let last = blocks.len() - 1;
        &mut blocks[last]
    }
    for item in items {
        match item {
            SettingsPageItem::SectionHeader(label) => blocks.push(SectionBlock {
                title: Some(label),
                items: Vec::new(),
            }),
            SettingsPageItem::Item(path) => {
                if let Some(field) = fields.iter().find(|field| field.json_path == *path) {
                    current(&mut blocks).items.push(SectionItem::Field(*field));
                }
            }
            SettingsPageItem::OwnerSurface(id) => {
                current(&mut blocks)
                    .items
                    .push(SectionItem::OwnerSurface(*id));
            }
        }
    }
    if !leftover.is_empty() {
        blocks.push(SectionBlock {
            title: Some("Other"),
            items: leftover.iter().copied().map(SectionItem::Field).collect(),
        });
    }
    blocks.retain(|block| !block.items.is_empty());
    blocks
}

/// The read-out next to a slider: whole numbers for integer fields, two
/// decimals otherwise, followed by the field's unit.
fn slider_readout(value: f64, int: bool, unit: Option<&str>) -> String {
    let number = if int {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.2}")
    };
    match unit {
        Some(unit) => format!("{number} {unit}"),
        None => number,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_become_cards_and_empty_sections_are_dropped() {
        let fields = crate::schema::all_fields();
        let items = [
            SettingsPageItem::SectionHeader("Empty"),
            SettingsPageItem::SectionHeader("Startup"),
            SettingsPageItem::Item("general.restoreWindowState"),
            SettingsPageItem::OwnerSurface(SettingsSurfaceId::new("surface")),
            SettingsPageItem::Item("general.missing"),
        ];
        let leftover = [fields[0]];
        let blocks = section_blocks(&items, &fields, &leftover);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].title, Some("Startup"));
        assert_eq!(blocks[0].items.len(), 2);
        assert_eq!(blocks[1].title, Some("Other"));
    }

    #[test]
    fn slider_readouts_format_by_kind() {
        assert_eq!(slider_readout(13.0, true, Some("pt")), "13 pt");
        assert_eq!(slider_readout(1.5, false, None), "1.50");
    }

    #[test]
    fn sftp_column_values_are_filtered_and_deduplicated_in_order() {
        let value = Value::Array(
            ["modified", "unknown", "size", "modified", "permissions"]
                .into_iter()
                .map(|token| Value::String(token.to_owned()))
                .collect(),
        );
        assert_eq!(
            sftp_visible_columns(Some(&value)),
            [
                SftpColumn::Modified,
                SftpColumn::Size,
                SftpColumn::Permissions
            ]
        );
    }

    #[test]
    fn invalid_or_missing_sftp_column_values_use_the_default() {
        let expected = default_sftp_columns();
        assert_eq!(sftp_visible_columns(None), expected);
        assert_eq!(
            sftp_visible_columns(Some(&Value::String("invalid".to_owned()))),
            expected
        );
    }

    #[test]
    fn project_scope_disables_only_fields_outside_the_project_whitelist() {
        assert!(!field_disabled_in_scope(
            SettingsScope::User,
            "appearance.appFontSize"
        ));
        assert!(!field_disabled_in_scope(
            SettingsScope::Project,
            "general.restoreWindowState"
        ));
        assert!(field_disabled_in_scope(
            SettingsScope::Project,
            "appearance.appFontSize"
        ));
    }
}
