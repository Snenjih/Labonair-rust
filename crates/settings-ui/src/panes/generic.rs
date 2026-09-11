//! Generic field renderer (T19-004): dropdown layer, `render_field`
//! (dispatches on `FieldControl` — the renderer registry), the generated-page
//! renderer (static section headers + trailing "Other" fallback; section
//! navigation lives in the sidebar per `docs/architecture.md` §8.3), the
//! top-level `render_body` dispatch.
//!
//! Part of `SettingsView` — see `crate::view`.

use crate::view::*;
use labonair_settings_content::file_manager::{default_sftp_columns, SftpColumn};
use labonair_ui_kit::{caret, DISABLED_OPACITY};

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
        if !self.system_fonts.is_empty() {
            return;
        }
        let service = self.font_service.clone();
        let task = self.tokio.spawn(async move { service.list().await });
        cx.spawn(async move |this, cx| match task.await {
            Ok(Ok(mut names)) => {
                names.sort_by_key(|name| name.to_lowercase());
                let _ = this.update(cx, |this, cx| {
                    this.system_fonts = names.into_iter().map(SharedString::from).collect();
                    cx.notify();
                });
            }
            Ok(Err(error)) => {
                let _ = this.update(cx, |this, cx| {
                    this.notify_error(cx, "System fonts", error);
                });
            }
            Err(error) => {
                let _ = this.update(cx, |this, cx| {
                    this.notify_error(cx, "System fonts", error.to_string());
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
        let view = cx.entity();
        Some(select_popover(
            "settings-dropdown",
            menu.at,
            *c,
            &options,
            cur.as_ref(),
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

    /// Resolve `(origin badge, effective value)` for every field a page is
    /// about to render, in one pass. Keeps the per-row `render_field` calls
    /// free of store lookups so a scroll repaint doesn't re-query
    /// `source_of` / `field_value` for each visible row every frame.
    pub(crate) fn field_render_inputs<'a>(
        &self,
        fields: impl IntoIterator<Item = &'a AnyField>,
        cx: &App,
    ) -> std::collections::HashMap<&'static str, (OriginBadge, Option<Value>)> {
        fields
            .into_iter()
            .map(|f| {
                (
                    f.json_path,
                    (self.field_origin(f, cx), self.field_value(f, cx)),
                )
            })
            .collect()
    }

    /// Render one generated field row: label/description + origin badge +
    /// reset (rule 5) + a control chosen by `FieldControl` (rule 3's
    /// renderer registry — `bool → Switch`, numeric → stepper, `enum`/closed
    /// `String` → dropdown, `String` → text input, anything else → the raw
    /// JSON fallback).
    ///
    /// `origin` + `value` are passed in already computed: the batch renderers
    /// (`render_generated_body`, `render_field_groups`) resolve them once per
    /// visible field via [`Self::field_render_inputs`] instead of every row
    /// re-querying the store — this is part of what keeps scrolling smooth.
    /// An invisible overlay that records a select trigger's window-space
    /// bounds (keyed by `json_path`) on every paint, so the dropdown can drop
    /// from the trigger's bottom-left instead of the click position (P2.1).
    fn select_bounds_probe(
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
    fn select_anchor(&self, json_path: &'static str, click: Point<Pixels>) -> Point<Pixels> {
        self.select_bounds
            .get(json_path)
            .map(|b| b.bottom_left())
            .unwrap_or(click)
    }

    pub(crate) fn render_field(
        &self,
        field: &AnyField,
        origin: OriginBadge,
        value: Option<Value>,
        row_first: bool,
        row_last: bool,
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
                Switch::new(SharedString::from(format!("sw-{json_path}")))
                    .checked(on)
                    .on_click(cx.listener(move |this, _: &bool, _w, cx| {
                        if let Some(f) = this.field_by_path(json_path).copied() {
                            this.toggle_bool(&f, cx);
                        }
                    }))
                    .into_any_element()
            }
            // T20-001: both numeric controls are the shared `NumberField`
            // primitive now — it owns the stepper chrome, the filled track and
            // the min/max/step clamping.
            FieldControl::Int { min, max, step } => {
                let cur = value.as_ref().and_then(|v| v.as_i64()).unwrap_or(min);
                number_field(
                    SharedString::from(format!("int-{json_path}")),
                    *c,
                    cur as f64,
                    min as f64,
                    max as f64,
                    step as f64,
                )
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
                number_field(
                    SharedString::from(format!("float-{json_path}")),
                    *c,
                    cur,
                    min_centi as f64 / 100.0,
                    max_centi as f64 / 100.0,
                    step_centi as f64 / 100.0,
                )
                .decimals(2)
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
                        });
                    }
                    cx.notify();
                }))
                .into_any_element()
            }
            FieldControl::FontFamily => {
                let cur = value
                    .as_ref()
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default();
                let is_open = self.dropdown.as_ref().is_some_and(|d| d.key == json_path);
                let label = if cur.is_empty() {
                    "(default)".to_string()
                } else {
                    cur
                };
                let fonts = self.system_fonts.clone();
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
                        });
                    }
                    cx.notify();
                }))
                .into_any_element()
            }
            FieldControl::Text => self.render_text_control(json_path, value, c, cx),
            FieldControl::SftpColumns => self.render_sftp_columns_control(json_path, value, c, cx),
        };

        let non_default = origin != OriginBadge::Default;
        // T19-007: a search jump briefly pulses the target row so the user
        // can find it among a page's other fields.
        let highlighted = self.highlight == Some(json_path);

        // Consecutive fields under one section header share a single
        // grouped card instead of each being its own box: every row draws
        // side borders + a bottom hairline (the separator between rows, and
        // on the last row the card's bottom edge); only the first row draws
        // the top edge, and only the first/last round the outer corners.
        let mut row = div()
            .id(SharedString::from(format!("field-row-{json_path}")))
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .px_4()
            .py_3()
            .border_l_1()
            .border_r_1()
            .border_b_1()
            .border_color(c.border)
            .bg(if highlighted {
                c.accent.opacity(0.25)
            } else {
                c.muted_bg
            });
        if row_first {
            row = row.border_t_1().rounded_t_md().mt_2();
        }
        if row_last {
            row = row.rounded_b_md();
        }

        let mut title_row = h_stack().gap_1p5().child(
            div()
                .text_color(c.fg)
                .child(SharedString::from(field.meta.title)),
        );
        if non_default {
            title_row = title_row
                .child(
                    div()
                        .px_1()
                        .rounded_sm()
                        .text_size(px(9.0))
                        .text_color(c.muted)
                        .border_1()
                        .border_color(c.border)
                        .child(origin.label()),
                )
                .child(
                    // Icon-only, muted (no accent/"yellow", no "reset"
                    // text) — a quiet affordance beside the origin badge.
                    button(
                        SharedString::from(format!("reset-{json_path}")),
                        *c,
                        ButtonVariant::Ghost,
                        ButtonSize::IconXs,
                    )
                    .child(IconName::Refresh.svg(c.muted).size(px(12.0)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.reset_field(json_path, cx);
                    })),
                );
        }

        row.child(
            v_stack()
                .gap_0p5()
                .flex_1()
                .min_w_0()
                .child(title_row)
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(c.muted)
                        .child(SharedString::from(field.meta.description)),
                ),
        )
        .child(control)
        .into_any_element()
    }

    /// Text fields use the shared click-to-edit text-box widget.
    fn render_text_control(
        &self,
        json_path: &'static str,
        value: Option<Value>,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let editing = self
            .editing
            .as_ref()
            .filter(|e| e.key == json_path)
            .map(|e| e.buffer.clone());
        let display_value = editing.clone().unwrap_or_else(|| match value {
            Some(Value::String(s)) => s,
            _ => String::new(),
        });
        let active = editing.is_some();
        let empty = display_value.is_empty();
        let show_caret = active && self.blink.read(cx).visible();
        // T20-003: a click-to-edit text field driven by `self.editing`'s
        // keydown-buffer state machine — no `button()`/`ListItem` fits a
        // text-input trigger, documented exception (same shape as
        // `panes/ai.rs`'s provider-key box).
        div()
            .id(SharedString::from(format!("txt-{json_path}")))
            .w(px(200.0))
            .px_2()
            .py(px(3.0))
            .flex()
            .items_center()
            .rounded_sm()
            .border_1()
            .border_color(if active { c.accent } else { c.border })
            .bg(c.bg)
            .text_color(if empty { c.muted } else { c.fg })
            .text_size(px(11.0))
            .child(SharedString::from(if empty {
                "(default)".to_string()
            } else {
                display_value
            }))
            .when(show_caret, |d| d.child(caret(c.fg, 12.0)))
            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                this.begin_edit(json_path, false, cx);
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
                let mut b = button(
                    SharedString::from(format!("{id}-{}", col.token())),
                    *c,
                    ButtonVariant::Ghost,
                    ButtonSize::IconXs,
                )
                .child(icon.svg(if enabled { c.fg } else { c.muted }).size(px(12.0)));
                if enabled {
                    b = b.on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.sftp_columns_move(json_path, col, delta, cx);
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
                        .on_click(cx.listener(move |this, checked: &bool, _w, cx| {
                            this.sftp_columns_toggle(json_path, col, *checked, cx);
                        })),
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
                div()
                    .text_size(px(10.5))
                    .text_color(c.muted)
                    .child("Order also adjusts by dragging the column headers in the SFTP browser."),
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
        // T19-007: the global search now lives in the sidebar (a flat,
        // category-grouped result list, `SettingsView::render_search_results`)
        // — the main content area always shows the active category/sub-page,
        // exactly as when browsing, so a search jump lands the field in its
        // normal place (with a highlight pulse) rather than a duplicate
        // inline render.
        self.render_generated_body(c, cx)
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
        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        if area.key == "general" && self.active_subpage.is_none() {
            rows.push(self.render_about_hero(c, cx));
        }

        // Direct-child index of each section header within the scroll
        // container below — this is what `ScrollHandle::scroll_to_item`
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
        let inputs = self.field_render_inputs(placed.iter().flatten().chain(leftover.iter()), cx);
        let row_input = |field: &AnyField| {
            inputs
                .get(field.json_path)
                .cloned()
                .unwrap_or((OriginBadge::Default, None))
        };

        // Resolve headers + fields that actually render into one flat
        // sequence first, so consecutive fields under one header can be
        // grouped into a single card: a field's position in that sequence
        // (first/last since the previous/next header) decides which edges
        // `render_field` draws (see its doc comment).
        enum Resolved {
            Header(&'static str),
            Field(AnyField),
        }
        let mut resolved: Vec<Resolved> = Vec::new();
        for (item, field) in items.iter().zip(placed.iter()) {
            match item {
                SettingsPageItemOwned::SectionHeader(label) => {
                    resolved.push(Resolved::Header(label));
                }
                SettingsPageItemOwned::Item(_) => {
                    if let Some(field) = field {
                        resolved.push(Resolved::Field(*field));
                    }
                }
            }
        }
        if !leftover.is_empty() {
            resolved.push(Resolved::Header("Other"));
            for field in &leftover {
                resolved.push(Resolved::Field(*field));
            }
        }

        for (i, entry) in resolved.iter().enumerate() {
            match entry {
                Resolved::Header(label) => {
                    section_rows.push((rows.len(), label));
                    rows.push(self.render_section_header(label, c, cx));
                }
                Resolved::Field(field) => {
                    if pending_scroll == Some(field.json_path) {
                        scroll_to_row = Some(rows.len());
                    }
                    let row_first = i == 0 || matches!(resolved.get(i - 1), Some(Resolved::Header(_)));
                    let row_last = resolved
                        .get(i + 1)
                        .map_or(true, |next| matches!(next, Resolved::Header(_)));
                    let (origin, value) = row_input(field);
                    rows.push(self.render_field(field, origin, value, row_first, row_last, c, cx));
                }
            }
        }

        if let Some(row) = scroll_to_row {
            self.content_scroll.scroll_to_item(row);
            self.pending_scroll = None;
        }
        if let Some(target) = self.scroll_to_section.take() {
            if let Some((row, _)) = section_rows.iter().find(|(_, l)| *l == target) {
                // Pin the section header to the top of the content area.
                self.content_scroll.scroll_to_top_of_item(*row);
            }
        }

        // The content-area scroll container: section headers and field rows
        // are its direct children so `ScrollHandle::scroll_to_item` can
        // address them by index. `track_scroll` lives here, not on an outer
        // wrapper, for the same reason.
        div()
            .id("settings-scroll")
            .flex_1()
            .min_h_0()
            .w_full()
            .max_w(px(580.0))
            .flex()
            .flex_col()
            .p_4()
            .overflow_y_scroll()
            .track_scroll(&self.content_scroll)
            .children(rows)
            .into_any_element()
    }

    /// A static section heading (`docs/architecture.md` §8.3 deviation from
    /// `settings-guidelines.md` rule 1: no longer a user-collapsible
    /// disclosure — the section list moved to the sidebar as scroll
    /// anchors). A small `primary`-tinted marker + a heavier label give each
    /// section its own beat, so scrolling reads as distinct blocks instead
    /// of one undifferentiated column of rows.
    fn render_section_header(
        &self,
        label: &'static str,
        c: &Palette,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div()
            .pt_5()
            .pb_2()
            .flex()
            .items_center()
            .gap_2()
            .child(div().w(px(3.0)).h(px(12.0)).rounded_full().bg(c.primary))
            .child(
                div()
                    .text_size(px(12.5))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c.fg)
                    .child(SharedString::from(label)),
            )
            .into_any_element()
    }

    // The scroll-spy jump bar that used to sit at the top of every generated
    // page (image #4's chip row) was removed here per `docs/architecture.md`
    // §8.3 — section navigation is now the sidebar's expandable sub-entries.
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
