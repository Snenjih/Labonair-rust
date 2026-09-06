//! Generic field renderer (T19-004): dropdown layer, `render_field`
//! (dispatches on `FieldControl` — the renderer registry), the generated-page
//! renderer (static section headers + trailing "Other" fallback; section
//! navigation lives in the sidebar per `docs/architecture.md` §8.3), the
//! top-level `render_body` dispatch (search / Generated / Custom), and the
//! MCP "AI Agent Bridge" pane.
//!
//! Part of `SettingsView` — see `crate::view`.

use crate::view::*;

impl SettingsView {
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
    pub(crate) fn render_field(
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
                // T20-003: the shared `gpui-component` `Switch` (re-exported
                // by `labonair-ui-kit` — its own colours are the sanctioned
                // exception, see `ui_kit.rs`'s module doc).
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
                            at: ev.position(),
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
                            at: ev.position(),
                            default_sentinel: Some(sentinel),
                        });
                    }
                    cx.notify();
                }))
                .into_any_element()
            }
            FieldControl::Text => self.render_text_control(json_path, false, value, c, cx),
            FieldControl::Json => self.render_text_control(json_path, true, value, c, cx),
        };

        let non_default = origin != OriginBadge::Default;
        // T19-007: a search jump briefly pulses the target row so the user
        // can find it among a page's other fields.
        let highlighted = self.highlight == Some(json_path);

        // Each setting is its own card (rounded, hairline border, raised
        // `--card` fill) with the row list spacing them apart — see the
        // Settings visual spec.
        div()
            .id(SharedString::from(format!("field-row-{json_path}")))
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .px_4()
            .py_3()
            .rounded_md()
            .border_1()
            .border_color(c.border)
            .bg(if highlighted {
                c.accent.opacity(0.25)
            } else {
                c.card
            })
            .child(
                v_stack()
                    .gap_0p5()
                    .flex_1()
                    .min_w_0()
                    .child(
                        h_stack()
                            .gap_1p5()
                            .child(
                                div()
                                    .text_color(c.fg)
                                    .child(SharedString::from(field.meta.title)),
                            )
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
                            .when(non_default, |d| {
                                d.child(
                                    // Icon-only, muted (no accent/"yellow", no
                                    // "reset" text) — a quiet affordance beside
                                    // the origin badge.
                                    button(
                                        SharedString::from(format!("reset-{json_path}")),
                                        *c,
                                        ButtonVariant::Ghost,
                                        ButtonSize::IconXs,
                                    )
                                    .child(IconName::Refresh.svg(c.muted).size(px(12.0)))
                                    .on_click(cx.listener(
                                        move |this, _: &ClickEvent, _w, cx| {
                                            this.reset_field(json_path, cx);
                                        },
                                    )),
                                )
                            }),
                    )
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

    /// `Text`/`Json` share the same click-to-edit text-box widget; `Json`
    /// round-trips through `serde_json::from_str` instead of storing the raw
    /// string (the settings-guidelines rule 3 fallback: "a raw JSON snippet
    /// editor" for any type without a dedicated widget).
    fn render_text_control(
        &self,
        json_path: &'static str,
        json_mode: bool,
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
            Some(v) if json_mode => v.to_string(),
            _ => String::new(),
        });
        let active = editing.is_some();
        let empty = display_value.is_empty();
        // T20-003: a click-to-edit text field driven by `self.editing`'s
        // keydown-buffer state machine — no `button()`/`ListItem` fits a
        // text-input trigger, documented exception (same shape as
        // `panes/ai.rs`'s provider-key box).
        div()
            .id(SharedString::from(format!("txt-{json_path}")))
            .w(px(if json_mode { 260.0 } else { 200.0 }))
            .px_2()
            .py(px(3.0))
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
            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                this.begin_edit(json_path, false, cx);
            }))
            .into_any_element()
    }

    pub(crate) fn render_agent_bridge(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let m = self.mcp;
        let setup = if m.bridge_enabled {
            self.mcp_token.as_ref().map(|tok| {
                format!(
                    "claude mcp add --transport http labonair http://127.0.0.1:{}/mcp --header \"Authorization: Bearer {}\" --scope user",
                    m.bridge_port, tok
                )
            })
        } else {
            None
        };

        let mut col = div().flex().flex_col();

        col = col.child(bridge_switch_row(
            "Enable AI Agent Bridge",
            "Let an external agent CLI drive granted SSH / local tabs over MCP.",
            m.bridge_enabled,
            c,
            cx,
            |this, cx| {
                let next = !this.mcp.bridge_enabled;
                this.mcp.bridge_enabled = next;
                let _ = mcp_prefs_save(&this.mcp);
                let app = this.backend.clone();
                this.tokio.spawn(async move {
                    let _ = mcp_set_enabled(next, app.clone(), &app.mcp, &app.secrets).await;
                });
                this.refresh_mcp_status(cx);
                cx.notify();
            },
        ));

        col = col.child(bridge_int_row(
            "Port",
            m.bridge_port as i64,
            1024,
            65535,
            1,
            c,
            cx,
            |this, v, cx| {
                this.mcp.bridge_port = v as u16;
                let _ = mcp_prefs_save(&this.mcp);
                let app = this.backend.clone();
                let port = this.mcp.bridge_port;
                this.tokio.spawn(async move {
                    let _ = mcp_set_port(port, app.clone(), &app.mcp, &app.secrets).await;
                });
                this.refresh_mcp_status(cx);
                cx.notify();
            },
        ));

        col = col.child(bridge_int_row(
            "Max command timeout (s)",
            m.max_command_timeout_secs as i64,
            5,
            3600,
            5,
            c,
            cx,
            |this, v, cx| {
                this.mcp.max_command_timeout_secs = v as u64;
                let _ = mcp_prefs_save(&this.mcp);
                let app = this.backend.clone();
                let secs = this.mcp.max_command_timeout_secs;
                this.tokio.spawn(async move {
                    let _ = mcp_set_max_command_timeout_secs(secs, &app.mcp).await;
                });
                cx.notify();
            },
        ));

        col = col.child(bridge_int_row(
            "Auto-revoke after (min, 0 = off)",
            m.auto_revoke_minutes as i64,
            0,
            1440,
            5,
            c,
            cx,
            |this, v, cx| {
                this.mcp.auto_revoke_minutes = v as u32;
                let _ = mcp_prefs_save(&this.mcp);
                let app = this.backend.clone();
                let mins = this.mcp.auto_revoke_minutes;
                this.tokio.spawn(async move {
                    let _ = mcp_set_auto_revoke_minutes(mins, &app.mcp).await;
                });
                cx.notify();
            },
        ));

        col = col.child(bridge_switch_row(
            "Notify on agent activity",
            "Show a toast for every command / keystroke an agent sends.",
            m.notify_on_activity,
            c,
            cx,
            |this, cx| {
                this.mcp.notify_on_activity = !this.mcp.notify_on_activity;
                let _ = mcp_prefs_save(&this.mcp);
                cx.notify();
            },
        ));

        col = col.child(
            div().flex().items_center().gap_2().py_2().child(
                button("mcp-regen", *c, ButtonVariant::Outline, ButtonSize::Xs)
                    .child("Regenerate token")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        let app = this.backend.clone();
                        let task = this.tokio.spawn(async move {
                            mcp_regenerate_token(app.clone(), &app.mcp, &app.secrets).await
                        });
                        cx.spawn(async move |this, cx| {
                            if let Ok(Ok(status)) = task.await {
                                let _ = this.update(cx, |this, cx| {
                                    this.mcp_token = status.token;
                                    cx.notify();
                                });
                            }
                        })
                        .detach();
                    })),
            ),
        );

        if let Some(cmd) = setup {
            let copy = cmd.clone();
            col = col.child(
                div()
                    .mt_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(c.muted)
                            .child("claude mcp add \u{2026}"),
                    )
                    .child(
                        div()
                            .p_2()
                            .rounded_sm()
                            .bg(c.bg)
                            .border_1()
                            .border_color(c.border)
                            .text_size(px(11.0))
                            .text_color(c.fg)
                            .child(SharedString::from(cmd)),
                    )
                    .child(
                        button("mcp-copy", *c, ButtonVariant::Outline, ButtonSize::Xs)
                            .child(IconName::Copy.svg(c.fg))
                            .child("Copy")
                            .on_click(cx.listener(move |_this, _: &ClickEvent, _w, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()));
                                notification_center(cx).update(cx, |n, cx| {
                                    n.push(
                                        Notification::info("Copied", "Setup command copied."),
                                        cx,
                                    );
                                });
                            })),
                    ),
            );
        }

        col.into_any_element()
    }

    // ── T19-004: top-level render dispatch ──────────────────────────────

    pub(crate) fn render_body(&mut self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        // T19-007: the global search now lives in the sidebar (a flat,
        // category-grouped result list, `SettingsView::render_search_results`)
        // — the main content area always shows the active category/sub-page,
        // exactly as when browsing, so a search jump lands the field in its
        // normal place (with a highlight pulse) rather than a duplicate
        // inline render.
        let area = &AREAS[self.active_area];
        match self.active_body_kind() {
            PageBodyKind::Generated => self.render_generated_body(c, cx),
            PageBodyKind::Custom => self.render_custom_body(area.key, c, cx),
        }
    }

    /// Cheap tag mirroring `active_body()`'s variant, without holding a
    /// borrow of `self.pages` across the `render_generated_body`/
    /// `render_custom_body` call (both need `&mut self`).
    fn active_body_kind(&self) -> PageBodyKind {
        match self.active_body() {
            PageBody::Generated(_) => PageBodyKind::Generated,
            PageBody::Custom => PageBodyKind::Custom,
        }
    }

    /// Render the active `PageBody::Generated` page/sub-page: collapsible
    /// disclosure sections + a scroll-spy jump bar + a trailing "Other"
    /// fallback for any field not placed by a curated group.
    fn render_generated_body(&mut self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        // `self.pages[..].area` is the same `&'static AreaMeta` `AREAS[..]`
        // would give — reading it through `pages` (rather than `AREAS`
        // directly) keeps `SettingsPage::area` a real, exercised field.
        let area = *self.pages[self.active_area].area;
        let items: Vec<SettingsPageItemOwned> = match self.active_body() {
            PageBody::Generated(items) => items.iter().map(SettingsPageItemOwned::from).collect(),
            PageBody::Custom => return div().into_any_element(),
        };
        let leading = if area.key == "general" && self.active_subpage.is_none() {
            Some(self.render_about_hero(c, cx))
        } else {
            None
        };

        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        // Row index of each section header, so a sidebar sub-entry click
        // (`scroll_to_section`, §8.3) or a search jump can scroll to it.
        let mut section_rows: Vec<(usize, &'static str)> = Vec::new();
        // T19-007: a search jump asks to land on a specific field's row
        // (`pending_scroll`) — recorded here so it can be scrolled to once
        // all rows are built.
        let pending_scroll = self.pending_scroll;
        let mut scroll_to_row: Option<usize> = None;

        let leftover: Vec<AnyField> = leftover_fields(area.target_module, &self.all_fields)
            .into_iter()
            .copied()
            .collect();

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

        for (item, resolved) in items.iter().zip(placed.iter()) {
            match item {
                SettingsPageItemOwned::SectionHeader(label) => {
                    section_rows.push((rows.len(), label));
                    rows.push(self.render_section_header(label, c, cx));
                }
                SettingsPageItemOwned::Item(_) => {
                    if let Some(field) = resolved {
                        if pending_scroll == Some(field.json_path) {
                            scroll_to_row = Some(rows.len());
                        }
                        let (origin, value) = row_input(field);
                        rows.push(self.render_field(field, origin, value, c, cx));
                    }
                }
            }
        }

        if !leftover.is_empty() {
            let label: &'static str = "Other";
            section_rows.push((rows.len(), label));
            rows.push(self.render_section_header(label, c, cx));
            for field in &leftover {
                if pending_scroll == Some(field.json_path) {
                    scroll_to_row = Some(rows.len());
                }
                let (origin, value) = row_input(field);
                rows.push(self.render_field(field, origin, value, c, cx));
            }
        }

        if let Some(target) = self.scroll_to_section.take() {
            if let Some((row, _)) = section_rows.iter().find(|(_, l)| *l == target) {
                scroll_to_row = Some(*row);
            }
        }
        if let Some(row) = scroll_to_row {
            self.content_scroll.scroll_to_item(row);
            self.pending_scroll = None;
        }

        // T20-001: shared `v_stack` layout helper.
        v_stack()
            .children(leading)
            .child(v_stack().gap_2().children(rows))
            .into_any_element()
    }

    /// A static section heading (`docs/architecture.md` §8.3 deviation from
    /// `settings-guidelines.md` rule 1: no longer a user-collapsible
    /// disclosure — the section list moved to the sidebar as scroll
    /// anchors). Muted, semibold, hairline underneath — like image #3's
    /// "Typography".
    fn render_section_header(
        &self,
        label: &'static str,
        c: &Palette,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div()
            .pt_4()
            .pb_1()
            .text_size(px(11.0))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(c.muted)
            .child(SharedString::from(label))
            .into_any_element()
    }

    // The scroll-spy jump bar that used to sit at the top of every generated
    // page (image #4's chip row) was removed here per `docs/architecture.md`
    // §8.3 — section navigation is now the sidebar's expandable sub-entries.

    /// Dispatch a Custom top-level category's body (rule 4) — the one
    /// registration point a new custom category needs: an `AREAS` entry
    /// (data) + one match arm here (render_fn). `Personalization` also folds
    /// in its own generic field grid (`PERSONALIZATION_GROUPS`) before its
    /// bespoke sections, exactly as rule 4 allows ("may still read/write
    /// fields under `target_module`").
    fn render_custom_body(
        &mut self,
        area_key: &'static str,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        match (area_key, self.active_subpage) {
            ("themes", _) => self.render_themes(c, cx),
            ("shortcuts", _) => {
                let query = String::new();
                self.render_shortcuts(&query, c, cx)
            }
            ("mcp", _) => self.render_agent_bridge(c, cx),
            ("hosts", None) => self.render_hosts_pane(c, cx),
            ("hosts", Some(0)) => self.render_hosts_ssh_config(c, cx),
            ("hosts", Some(_)) => self.render_hosts_availability(c, cx),
            ("personalization", _) => self.render_personalization(c, cx),
            _ => div().into_any_element(),
        }
    }

    /// Render one group list (`SectionHeader`+`Item` rows only, honoring
    /// disclosure collapse state) — the core loop `render_generated_body`
    /// uses for `AreaKind::Generated` pages, reused directly by the Custom
    /// panes that fold their own field grid into a bespoke body before their
    /// non-field sections (`PERSONALIZATION_GROUPS` — rule 4: "may still
    /// read/write fields under `target_module`").
    pub(crate) fn render_field_groups(
        &mut self,
        groups: &[(&'static str, &'static [&'static str])],
        area_target_module: &'static str,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let pending_scroll = self.pending_scroll;
        let mut scroll_to_row: Option<usize> = None;
        let mut rows = Vec::new();

        let visible_fields: Vec<AnyField> = groups
            .iter()
            .filter(|(label, _)| !self.section_collapsed(label))
            .flat_map(|(_, keys)| keys.iter())
            .filter_map(|key| {
                self.all_fields
                    .iter()
                    .find(|f| f.area() == area_target_module && f.local_key() == *key)
                    .copied()
            })
            .collect();
        let inputs = self.field_render_inputs(visible_fields.iter(), cx);

        for (label, keys) in groups {
            rows.push(self.render_section_header(label, c, cx));
            if self.section_collapsed(label) {
                continue;
            }
            for key in *keys {
                if let Some(field) = self
                    .all_fields
                    .iter()
                    .find(|f| f.area() == area_target_module && f.local_key() == *key)
                    .copied()
                {
                    if pending_scroll == Some(field.json_path) {
                        scroll_to_row = Some(rows.len());
                    }
                    let (origin, value) = inputs
                        .get(field.json_path)
                        .cloned()
                        .unwrap_or((OriginBadge::Default, None));
                    rows.push(self.render_field(&field, origin, value, c, cx));
                }
            }
        }
        if let Some(row) = scroll_to_row {
            self.content_scroll.scroll_to_item(row);
            self.pending_scroll = None;
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .children(rows)
            .into_any_element()
    }
}

/// Which variant `SettingsPage::body`/`SubPage::body` is, without holding a
/// borrow across a `&mut self` call.
enum PageBodyKind {
    Generated,
    Custom,
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
