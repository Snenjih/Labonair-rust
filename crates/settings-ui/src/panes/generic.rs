//! Generic field renderer (T19-004): dropdown layer, `render_field`
//! (dispatches on `FieldControl` — the renderer registry), the generated-page
//! renderer (static section headers + trailing "Other" fallback; section
//! navigation lives in the sidebar per `docs/architecture.md` §8.3), the
//! top-level `render_body` dispatch.
//!
//! Part of `SettingsView` — see `crate::view`.

use crate::view::*;
use labonair_settings_content::file_manager::{default_sftp_columns, SftpColumn};
use labonair_ui_kit::DISABLED_OPACITY;

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
        let service = self.font_service.clone();
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
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let menu = self.dropdown.as_ref()?;
        if menu.key == SETTINGS_SCOPE_KEY {
            return None;
        }
        let json_path = menu.key;
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
        // T20-001: the anchored option list is the shared `Select` primitive
        // (`select_popover`) — same `deferred` + `anchored().snap_to_window()`
        // layer, one implementation.
        let options: Vec<SelectOption> = menu.options.clone();
        let highlighted = menu
            .options
            .get(menu.highlighted)
            .map(|(token, _)| token.clone())
            .unwrap_or_else(|| cur.clone());
        let view = cx.entity();
        Some(select_popover(
            "settings-dropdown",
            menu.at,
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

    pub(crate) fn render_scope_dropdown(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let menu = self.dropdown.as_ref()?;
        if menu.key != SETTINGS_SCOPE_KEY {
            return None;
        }
        let view = cx.entity();
        let options = menu.options.clone();
        Some(select_popover(
            "settings-scope-dropdown",
            menu.at,
            *c,
            &options,
            menu.options
                .get(menu.highlighted)
                .map(|(token, _)| token.as_ref())
                .unwrap_or_else(|| self.scope.token()),
            {
                let view = view.clone();
                move |_window, cx| {
                    view.update(cx, |this, cx| {
                        this.dropdown = None;
                        cx.notify();
                    });
                }
            },
            move |token, _window, cx| {
                let token = token.clone();
                view.update(cx, |this, cx| {
                    this.dropdown = None;
                    if let Some(scope) = SettingsScope::from_token(token.as_ref()) {
                        this.set_scope(scope, cx);
                    }
                });
            },
        ))
    }

    /// Render one generated field row: label/description + origin badge +
    /// reset (rule 5) + a control chosen by `FieldControl` (rule 3's
    /// renderer registry — `bool → Switch`, numeric → stepper, `enum`/closed
    /// `String` → dropdown, `String` → text input, anything else → the raw
    /// JSON fallback).
    ///
    /// `origin` + `value` are passed in already computed: the batch renderers
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

    fn render_field(
        &self,
        field: &AnyField,
        origin: OriginBadge,
        value: Option<Value>,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let json_path = field.json_path;
        let control = match field.control {
            FieldControl::Switch => {
                let on = value.as_ref().and_then(|v| v.as_bool()).unwrap_or(false);
                // The shared `gpui-component` `Switch` (re-exported by
                // `labonair-ui-kit`). Its "on" fill reads gpui-component's own
                // global `Theme::primary`, which `apply_prefs_to_theme`
                // (`theme-ui/src/apply.rs`) keeps synced to this app's
                // `core.primary` token on every theme/settings apply.
                div()
                    .id(SharedString::from(format!("settings-switch-{json_path}")))
                    .rounded(px(c.radius.sm))
                    .border_1()
                    .border_color(gpui::transparent_black())
                    .tab_index(0)
                    .focus(|style| style.border_1().border_color(c.ring))
                    .child(
                        Switch::new(SharedString::from(format!("sw-{json_path}")))
                            .checked(on)
                            .on_click(cx.listener(move |this, _: &bool, _w, cx| {
                                if let Some(f) = this.field_by_path(json_path).copied() {
                                    this.toggle_bool(&f, cx);
                                }
                            })),
                    )
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
                let is_open = self.dropdown.as_ref().is_some_and(|d| d.key == json_path);
                // T20-001: shared `Select` trigger.
                select_trigger(
                    SharedString::from(format!("sel-{json_path}")),
                    *c,
                    SharedString::from(label.to_string()),
                    is_open,
                )
                .relative()
                .child(self.select_bounds_probe(json_path, cx))
                .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                    if this.dropdown.as_ref().is_some_and(|d| d.key == json_path) {
                        this.dropdown = None;
                    } else {
                        this.dropdown = Some(SelectMenu {
                            key: json_path,
                            options: opts
                                .iter()
                                .map(|(t, l)| (SharedString::from(*t), SharedString::from(*l)))
                                .collect(),
                            at: this.select_anchor(json_path, ev.position()),
                            default_sentinel: None,
                            highlighted: selected_index,
                        });
                    }
                    cx.notify();
                }))
                .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | "space" | "down") {
                        this.dropdown = Some(SelectMenu {
                            key: json_path,
                            options: opts
                                .iter()
                                .map(|(token, label)| {
                                    (SharedString::from(*token), SharedString::from(*label))
                                })
                                .collect(),
                            at: this.select_anchor(json_path, Point::default()),
                            default_sentinel: None,
                            highlighted: selected_index,
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }
                }))
                .into_any_element()
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
                let key_fonts = fonts.clone();
                let selected_font_index = if cur.is_empty() {
                    0
                } else {
                    fonts
                        .iter()
                        .position(|font| font.as_ref() == cur)
                        .map(|index| index + 1)
                        .unwrap_or(0)
                };
                select_trigger(
                    SharedString::from(format!("font-{json_path}")),
                    *c,
                    SharedString::from(label),
                    is_open,
                )
                .min_w(px(200.0))
                .relative()
                .child(self.select_bounds_probe(json_path, cx))
                .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                    if this.dropdown.as_ref().is_some_and(|d| d.key == json_path) {
                        this.dropdown = None;
                    } else {
                        let sentinel = SharedString::from("(default)");
                        let mut options = vec![(sentinel.clone(), sentinel.clone())];
                        options.extend(fonts.iter().map(|f| (f.clone(), f.clone())));
                        this.dropdown = Some(SelectMenu {
                            key: json_path,
                            options,
                            at: this.select_anchor(json_path, ev.position()),
                            default_sentinel: Some(sentinel),
                            highlighted: selected_font_index,
                        });
                    }
                    cx.notify();
                }))
                .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | "space" | "down") {
                        let sentinel = SharedString::from("(default)");
                        let mut options = vec![(sentinel.clone(), sentinel.clone())];
                        options.extend(key_fonts.iter().map(|font| (font.clone(), font.clone())));
                        this.dropdown = Some(SelectMenu {
                            key: json_path,
                            options,
                            at: this.select_anchor(json_path, Point::default()),
                            default_sentinel: Some(sentinel),
                            highlighted: selected_font_index,
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }
                }))
                .into_any_element()
            }
            FieldControl::Text => self.render_text_control(json_path, value, c, cx),
            FieldControl::SftpColumns => self.render_sftp_columns_control(json_path, value, c, cx),
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

        let non_default = origin != OriginBadge::Default;
        // T19-007: a search jump briefly pulses the target row so the user
        // can find it among a page's other fields.
        let highlighted = self.highlight == Some(json_path);

        // Ordinary values use one continuous surface. A single bottom
        // divider keeps the list scannable without turning every section into
        // a nested card, matching the native settings design direction.
        let row = div()
            .id(SharedString::from(format!("field-row-{json_path}")))
            .flex()
            .w_full()
            .min_w_0()
            .items_center()
            .justify_between()
            .gap(c.space(24.0))
            .py(c.space(8.0))
            .border_b_1()
            .border_color(c.border)
            .when(highlighted, |d| d.bg(c.selected_fill.opacity(0.7)));

        let mut title_row = h_stack().gap_1p5().child(
            div()
                .text_color(c.fg)
                .text_size(px(13.0))
                .child(SharedString::from(field.meta.title)),
        );
        if non_default {
            title_row = title_row.child(
                div()
                    .px_1()
                    .rounded_sm()
                    .text_size(px(9.0))
                    .text_color(c.muted)
                    .border_1()
                    .border_color(c.border)
                    .child(origin.label()),
            );
        }

        let mut info = v_stack()
            .gap(c.space(4.0))
            .flex_1()
            .min_w_0()
            .max_w(gpui::relative(0.66))
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

        let mut actions = h_stack().items_center().gap(c.space(8.0)).child(control);
        if non_default {
            actions = actions.child(
                // Keep reset beside the value control in the trailing action
                // cluster, so all field actions share the same right edge.
                button(
                    SharedString::from(format!("reset-{json_path}")),
                    *c,
                    ButtonVariant::Ghost,
                    ButtonSize::IconXs,
                )
                .tab_index(0)
                .focus(|style| style.border_1().border_color(c.ring))
                .child(IconName::Refresh.svg(c.muted).size(px(12.0)))
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    this.reset_field(json_path, cx);
                }))
                .on_key_down(cx.listener(
                    move |this, event: &KeyDownEvent, _w, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.reset_field(json_path, cx);
                            cx.stop_propagation();
                        }
                    },
                )),
            );
        }
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
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(input) = self.text_inputs.get(json_path) else {
            return div()
                .w(c.space(220.0))
                .text_color(c.muted)
                .child("(input unavailable)")
                .into_any_element();
        };
        let focused = self.text_input_key == Some(json_path);
        div()
            .id(SharedString::from(format!("txt-{json_path}")))
            .w(c.space(220.0))
            .px(c.space(8.0))
            .py(c.space(4.0))
            .min_h(px(32.0))
            .flex()
            .items_center()
            .rounded(px(c.radius.sm))
            .border_1()
            .border_color(if focused { c.ring } else { c.border })
            .bg(c.input)
            .child(
                field_input(input)
                    .appearance(false)
                    .bordered(false)
                    .focus_bordered(false)
                    .w_full()
                    .text_size(px(12.0)),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.begin_text_edit(json_path, window, cx);
            }))
            .into_any_element()
    }

    /// Ordered visible-column editor for the SFTP browser
    /// (`FieldControl::SftpColumns`). Each column gets a checkbox
    /// (visible/hidden) and up/down reorder buttons; the stored value is a
    /// JSON array of column tokens in display order.
    fn render_sftp_columns_control(
        &self,
        json_path: &'static str,
        value: Option<Value>,
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
                let mut b = button(
                    SharedString::from(format!("{id}-{}", col.token())),
                    *c,
                    ButtonVariant::Ghost,
                    ButtonSize::IconXs,
                )
                .child(
                    icon.svg(if enabled { c.fg } else { c.muted })
                        .size(px(12.0)),
                );
                if enabled {
                    b = b
                        .tab_index(0)
                        .focus(|style| style.border_1().border_color(c.ring))
                        .tooltip(move |window, cx| {
                            labonair_ui_kit::Tooltip::new(tooltip.clone()).build(window, cx)
                        })
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.sftp_columns_move(json_path, col, delta, cx);
                        }));
                    b = b.on_key_down(cx.listener(move |this, event: &KeyDownEvent, _w, cx| {
                        let key = event.keystroke.key.as_str();
                        let activate = matches!(key, "enter" | "space");
                        if activate {
                            this.sftp_columns_move(json_path, col, delta, cx);
                            cx.stop_propagation();
                            cx.notify();
                        }
                    }));
                } else {
                    b = b.opacity(DISABLED_OPACITY);
                }
                b
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
        let mut last_area = None;
        for row in self.search_results.clone() {
            if last_area != Some(row.area_title) {
                last_area = Some(row.area_title);
                content_rows.push(ContentRow::Header(row.area_title));
            }
            let SearchTarget::Field(index) = row.target;
            if let Some(field) = self.all_fields.get(index).copied() {
                content_rows.push(ContentRow::Field(field));
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
        let list = list(list_state, move |index, _window, app| match rows[index] {
            ContentRow::Header(label) => view.update(app, |this, cx| {
                this.render_section_header(label, &palette, cx)
            }),
            ContentRow::Field(field) => view.update(app, |this, cx| {
                let origin = this.field_origin(&field, cx);
                let value = this.field_value(&field, cx);
                this.render_field(&field, origin, value, &palette, cx)
            }),
            ContentRow::Title(_) => div().into_any_element(),
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
        // `self.pages[..].area` is the same `&'static AreaMeta` `AREAS[..]`
        // would give — reading it through `pages` (rather than `AREAS`
        // directly) keeps `SettingsPage::area` a real, exercised field.
        let area = *self.pages[self.active_area].area;
        let PageBody::Generated(items) = self.active_body();
        let items: Vec<SettingsPageItemOwned> =
            items.iter().map(SettingsPageItemOwned::from).collect();
        let page_title = self
            .active_subpage
            .map(|index| self.pages[self.active_area].sub_pages[index].title)
            .unwrap_or(area.title);
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

        // The trailing "Other" fallback belongs on the area's main page only —
        // a sub-page shows just its own curated groups.
        let leftover: Vec<AnyField> = if self.active_subpage.is_none() {
            leftover_fields(area.target_module, &self.all_fields)
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
                SettingsPageItemOwned::SectionHeader(_) => None,
                SettingsPageItemOwned::Item(key) => self
                    .all_fields
                    .iter()
                    .find(|f| f.area() == area.target_module && f.local_key() == *key)
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
                ContentRow::Title(_) => {}
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
        let list = list(list_state, move |index, _window, app| match rows[index] {
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
                let origin = this.field_origin(&field, cx);
                let value = this.field_value(&field, cx);
                this.render_field(&field, origin, value, &palette, cx)
            }),
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
            .pt(c.space(24.0))
            .pb(c.space(4.0))
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
}

/// An owned mirror of `SettingsPageItem` (`&'static str`s only — cheap to
/// clone out of `self.pages` so the borrow doesn't outlive the loop that
/// needs `&mut self` for each field's render call).
enum SettingsPageItemOwned {
    SectionHeader(&'static str),
    Item(&'static str),
}

impl From<&SettingsPageItem> for SettingsPageItemOwned {
    fn from(item: &SettingsPageItem) -> Self {
        match item {
            SettingsPageItem::SectionHeader(s) => SettingsPageItemOwned::SectionHeader(s),
            SettingsPageItem::Item(s) => SettingsPageItemOwned::Item(s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
