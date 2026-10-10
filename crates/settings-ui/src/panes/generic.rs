//! Generic field renderer (T19-004): dropdown layer, `render_field`
//! (dispatches on `FieldControl` — the renderer registry), the generated-page
//! renderer (static section headers + trailing "Other" fallback; section
//! navigation lives in the sidebar per `docs/architecture.md` §8.3), the
//! top-level `render_body` dispatch.
//!
//! Part of `SettingsView` — see `crate::view`.

use crate::view::*;
use gpui_component::Disableable;
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
        &self,
        field: &AnyField,
        source: SettingSource,
        value: Option<Value>,
        decoration: SettingsRowDecoration,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let json_path = field.json_path;
        let field_disabled = field_disabled_in_scope(self.scope, json_path);
        let control = match field.control {
            FieldControl::Switch => {
                let on = value.as_ref().and_then(|v| v.as_bool()).unwrap_or(false);
                // The shared `gpui-component` `Switch` (re-exported by
                // `labonair-ui-kit`). Its "on" fill reads gpui-component's own
                // global `Theme::primary`, which `apply_prefs_to_theme`
                // (`theme-ui/src/apply.rs`) keeps synced to this app's
                // `core.primary` token on every theme/settings apply.
                let switch = Switch::new(SharedString::from(format!("sw-{json_path}")))
                    .checked(on)
                    .disabled(field_disabled);
                let switch = if field_disabled {
                    switch
                } else {
                    switch.on_click(cx.listener(move |this, _: &bool, _w, cx| {
                        if let Some(f) = this.field_by_path(json_path).copied() {
                            this.toggle_bool(&f, cx);
                        }
                    }))
                };
                let surface = div()
                    .id(SharedString::from(format!("settings-switch-{json_path}")))
                    .rounded(px(c.radius.sm))
                    .border_1()
                    .border_color(gpui::transparent_black())
                    .child(switch);
                if field_disabled {
                    surface
                        .tab_index(-1)
                        .cursor_default()
                        .opacity(DISABLED_OPACITY)
                        .into_any_element()
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
        };
        let control = if let Some(unit) = field.meta.unit {
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

        // Ordinary values use one continuous surface. A single bottom
        // divider keeps the list scannable without turning every section into
        // a nested card, matching the native settings design direction.
        let row = div()
            .id(SharedString::from(format!("field-row-{json_path}")))
            .group("settings-field-row")
            .flex()
            .w_full()
            .min_w_0()
            .items_center()
            .justify_between()
            .gap(c.space(24.0))
            .pt(c.control_space(16.0))
            .pb(c.control_space(if decoration.section_end { 40.0 } else { 16.0 }))
            .when(decoration.bottom_divider, |row| {
                row.border_b_1().border_color(c.border)
            })
            .when(highlighted, |d| d.bg(c.selected_fill.opacity(0.7)));

        let mut title_row = h_stack().items_center().gap(c.space(6.0)).child(
            div()
                .text_color(c.fg)
                .text_size(px(13.0))
                .child(SharedString::from(field.meta.title)),
        );
        if let Some(modified_in) = modified_in {
            title_row = title_row
                .child(
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
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(c.muted)
                        .child(SharedString::from(format!("— Modified in {modified_in}"))),
                );
        }

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

        let mut info = v_stack()
            .relative()
            .gap(c.space(4.0))
            .flex_1()
            .min_w_0()
            .max_w(gpui::relative(0.66))
            .child(
                div()
                    .absolute()
                    .top(px(0.0))
                    .left(px(-24.0))
                    .child(copy_link),
            )
            .child(title_row)
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(c.muted)
                    .child(SharedString::from(field.meta.description)),
            );
        if let Some(hint) = field.meta.hint {
            info = info.child(
                div()
                    .text_size(px(11.0))
                    .text_color(c.muted.opacity(0.85))
                    .child(hint),
            );
        }
        if field_disabled {
            info = info.child(
                div()
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .child("Unavailable in Project settings."),
            );
        }

        let actions = h_stack().items_center().gap(c.space(8.0)).child(control);
        row.child(info)
            .child(div().ml_auto().flex_shrink_0().child(actions))
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

    pub(crate) fn render_body(&mut self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        // T19-007: search results occupy the main surface and use the same
        // canonical field renderer as ordinary category pages. The sidebar
        // remains stable, so search never replaces the user's orientation.
        if !self.search.trim().is_empty() {
            return self.render_search_body(c, cx);
        }
        self.render_generated_body(c, cx)
    }

    fn render_search_body(&mut self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let query = self.search.trim().to_owned();
        let mut content_rows = Vec::new();
        let mut last_page = None;
        for row in self.search_results.clone() {
            if last_page != Some(row.page_index) {
                last_page = Some(row.page_index);
                content_rows.push(ContentRow::Header(row.page_title));
            }
            match row.target {
                SearchTarget::Field(index) => {
                    if let Some(field) = self.all_fields.get(index).copied() {
                        content_rows.push(ContentRow::Field(field));
                    }
                }
                SearchTarget::OwnerSurface(surface) => {
                    content_rows.push(ContentRow::OwnerSurface(surface));
                }
            }
        }

        let header = v_stack()
            .gap(c.space(4.0))
            .pb(c.space(16.0))
            .child(
                div()
                    .text_size(px(22.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(c.fg)
                    .child("Search settings"),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(c.muted)
                    .child(SharedString::from(format!(
                        "Results for \u{201C}{query}\u{201D}"
                    ))),
            );

        if content_rows.is_empty() {
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
                        .text_size(px(13.0))
                        .text_color(c.muted)
                        .child("No setting found for this search."),
                )
                .into_any_element();
        }

        self.prepare_content_list(format!("search:{query}"), content_rows.len());
        let list_state = self.content_list.clone();
        let rows = content_rows;
        let view = cx.entity();
        let palette = *c;
        let list = list(list_state, move |index, _window, app| {
            let decoration = settings_row_decoration(&rows, index);
            match rows[index] {
                ContentRow::Header(label) => view.update(app, |this, cx| {
                    this.render_section_header(label, &palette, cx)
                }),
                ContentRow::Field(field) => view.update(app, |this, cx| {
                    let source = this.field_source(&field, cx);
                    let value = this.field_value(&field, cx);
                    this.render_field(&field, source, value, decoration, &palette, cx)
                }),
                ContentRow::OwnerSurface(surface) => view.update(app, |this, cx| {
                    this.render_owner_surface_row(surface, decoration, &palette, cx)
                }),
                ContentRow::Title(_) => div().into_any_element(),
            }
        })
        .flex_1()
        .min_h_0();

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

    /// Render the active `PageBody::Generated` page/sub-page: collapsible
    /// disclosure sections + a scroll-spy jump bar + a trailing "Other"
    /// fallback for any field not placed by a curated group.
    fn render_generated_body(&mut self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let page = &self.pages[self.active_area];
        let PageBody::Generated(items) = self.active_body();
        let items: Vec<SettingsPageItemOwned> =
            items.iter().map(SettingsPageItemOwned::from).collect();
        let page_title = self
            .active_subpage
            .map(|index| self.pages[self.active_area].sub_pages[index].title)
            .unwrap_or(page.title);
        // Direct-child index of each section header within the scroll
        // container below — this is what `ListState::scroll_to_reveal_item`
        // addresses, so a sidebar sub-entry click (`scroll_to_section`) or a
        // search jump can scroll the content to it.
        let mut section_rows: Vec<(usize, &'static str)> = Vec::new();
        // T19-007: a search jump asks to land on a specific field's row
        // (`pending_scroll`) — recorded here so it can be scrolled to once
        // all rows are built.
        let pending_scroll = self.pending_scroll;
        let mut scroll_to_row: Option<usize> = None;

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

        // Resolve every placed `Item` key to its `AnyField` once, then batch
        // the store lookups for all rows (placed + "Other") in a single pass.
        let placed: Vec<Option<AnyField>> = items
            .iter()
            .map(|item| match item {
                SettingsPageItemOwned::SectionHeader(_)
                | SettingsPageItemOwned::OwnerSurface(_) => None,
                SettingsPageItemOwned::Item(path) => self
                    .all_fields
                    .iter()
                    .find(|field| field.json_path == *path)
                    .copied(),
            })
            .collect();
        // Resolve headers + fields that actually render into one flat
        // sequence first so sidebar section anchors and field rows share one
        // stable scroll order.
        let mut resolved = vec![ContentRow::Title(page_title)];
        for (item, field) in items.iter().zip(placed.iter()) {
            match item {
                SettingsPageItemOwned::SectionHeader(label) => {
                    resolved.push(ContentRow::Header(label));
                }
                SettingsPageItemOwned::Item(_) => {
                    if let Some(field) = field {
                        resolved.push(ContentRow::Field(*field));
                    }
                }
                SettingsPageItemOwned::OwnerSurface(surface) => {
                    resolved.push(ContentRow::OwnerSurface(*surface));
                }
            }
        }
        if !leftover.is_empty() {
            resolved.push(ContentRow::Header("Other"));
            for field in &leftover {
                resolved.push(ContentRow::Field(*field));
            }
        }

        for (index, entry) in resolved.iter().enumerate() {
            match entry {
                ContentRow::Header(label) => {
                    section_rows.push((index, label));
                }
                ContentRow::Field(field) => {
                    if pending_scroll == Some(field.json_path) {
                        scroll_to_row = Some(index);
                    }
                }
                ContentRow::Title(_) | ContentRow::OwnerSurface(_) => {}
            }
        }

        self.prepare_content_list(
            format!(
                "area:{}:{}",
                self.active_area,
                self.active_subpage.unwrap_or(usize::MAX)
            ),
            resolved.len(),
        );
        if let Some(row) = scroll_to_row {
            self.content_list.scroll_to_reveal_item(row);
            self.pending_scroll = None;
        }
        if let Some(target) = self.scroll_to_section.take() {
            if let Some((row, _)) = section_rows.iter().find(|(_, l)| *l == target) {
                self.content_list.scroll_to_reveal_item(*row);
            }
        }

        let list_state = self.content_list.clone();
        let rows = resolved;
        let view = cx.entity();
        let palette = *c;
        let list = list(list_state, move |index, _window, app| {
            let decoration = settings_row_decoration(&rows, index);
            match rows[index] {
                ContentRow::Title(title) => div()
                    .pb(palette.space(20.0))
                    .text_size(px(22.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(palette.fg)
                    .child(SharedString::from(title))
                    .into_any_element(),
                ContentRow::Header(label) => view.update(app, |this, cx| {
                    this.render_section_header(label, &palette, cx)
                }),
                ContentRow::Field(field) => view.update(app, |this, cx| {
                    let source = this.field_source(&field, cx);
                    let value = this.field_value(&field, cx);
                    this.render_field(&field, source, value, decoration, &palette, cx)
                }),
                ContentRow::OwnerSurface(surface) => view.update(app, |this, cx| {
                    this.render_owner_surface_row(surface, decoration, &palette, cx)
                }),
            }
        })
        .flex_1()
        .min_h_0();

        div()
            .id("settings-scroll")
            .flex_1()
            .min_h_0()
            .w_full()
            .flex()
            .flex_col()
            .p(c.space(24.0))
            .child(list)
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

    fn render_owner_surface_row(
        &self,
        id: SettingsSurfaceId,
        decoration: SettingsRowDecoration,
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
        let button_id = SharedString::from(format!("settings-surface-{}", id.as_str()));
        let button = button(button_id, *c, ButtonVariant::OutlinedGhost, ButtonSize::Xs)
            .child(surface.action_label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                if let Err(error) = this.services.open_surface(id, cx) {
                    tracing::error!("could not open Settings surface: {error}");
                    this.notify_error(cx, "Settings", error);
                }
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    if let Err(error) = this.services.open_surface(id, cx) {
                        tracing::error!("could not open Settings surface: {error}");
                        this.notify_error(cx, "Settings", error);
                    }
                    cx.stop_propagation();
                }
            }));

        div()
            .id(SharedString::from(format!("settings-row-{}", id.as_str())))
            .flex()
            .w_full()
            .min_w_0()
            .items_center()
            .justify_between()
            .gap(c.space(24.0))
            .pt(c.control_space(16.0))
            .pb(c.control_space(if decoration.section_end { 40.0 } else { 16.0 }))
            .when(decoration.bottom_divider, |row| {
                row.border_b_1().border_color(c.border)
            })
            .child(
                v_stack()
                    .gap(c.space(4.0))
                    .flex_1()
                    .min_w_0()
                    .max_w(gpui::relative(0.66))
                    .child(
                        div()
                            .text_color(c.fg)
                            .text_size(px(13.0))
                            .child(surface.title),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(c.muted)
                            .child(surface.description),
                    ),
            )
            .child(div().ml_auto().flex_shrink_0().child(button))
            .into_any_element()
    }

    /// A static section heading (`docs/architecture.md` §8.3 deviation from
    /// `settings-guidelines.md` rule 1: no longer a user-collapsible
    /// disclosure — the section list moved to the sidebar as scroll anchors).
    /// Whitespace and typography provide the section beat; ordinary values
    /// remain a continuous, card-free list.
    fn render_section_header(
        &self,
        label: &'static str,
        c: &Palette,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        h_stack()
            .w_full()
            .items_center()
            .gap(c.space(8.0))
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c.muted)
                    .child(SharedString::from(label)),
            )
            .child(div().h(px(1.0)).flex_1().bg(c.border))
            .into_any_element()
    }

    // The scroll-spy jump bar that used to sit at the top of every generated
    // page (image #4's chip row) was removed here per `docs/architecture.md`
    // §8.3 — section navigation is now the sidebar's expandable sub-entries.
}

#[derive(Clone, Copy)]
enum ContentRow {
    Title(&'static str),
    Header(&'static str),
    Field(AnyField),
    OwnerSurface(SettingsSurfaceId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SettingsRowDecoration {
    bottom_divider: bool,
    section_end: bool,
}

fn settings_row_decoration(rows: &[ContentRow], index: usize) -> SettingsRowDecoration {
    let next_is_row = rows
        .get(index + 1)
        .is_some_and(|row| matches!(row, ContentRow::Field(_) | ContentRow::OwnerSurface(_)));
    SettingsRowDecoration {
        bottom_divider: next_is_row,
        section_end: !next_is_row,
    }
}

/// An owned mirror of `SettingsPageItem` (`&'static str`s only — cheap to
/// clone out of `self.pages` so the borrow doesn't outlive the loop that
/// needs `&mut self` for each field's render call).
enum SettingsPageItemOwned {
    SectionHeader(&'static str),
    Item(&'static str),
    OwnerSurface(SettingsSurfaceId),
}

impl From<&SettingsPageItem> for SettingsPageItemOwned {
    fn from(item: &SettingsPageItem) -> Self {
        match item {
            SettingsPageItem::SectionHeader(s) => SettingsPageItemOwned::SectionHeader(s),
            SettingsPageItem::Item(s) => SettingsPageItemOwned::Item(s),
            SettingsPageItem::OwnerSurface(surface) => {
                SettingsPageItemOwned::OwnerSurface(*surface)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_rows_divide_within_a_section_and_add_space_at_its_end() {
        let rows = [
            ContentRow::OwnerSurface(SettingsSurfaceId::new("first")),
            ContentRow::OwnerSurface(SettingsSurfaceId::new("last")),
            ContentRow::Header("Next section"),
        ];

        assert_eq!(
            settings_row_decoration(&rows, 0),
            SettingsRowDecoration {
                bottom_divider: true,
                section_end: false,
            }
        );
        assert_eq!(
            settings_row_decoration(&rows, 1),
            SettingsRowDecoration {
                bottom_divider: false,
                section_end: true,
            }
        );
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
