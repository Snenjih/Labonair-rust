//! Generic field renderer: dropdown layer, `render_field`, the `render_body` dispatch, category grouping, and the MCP "AI Agent Bridge" pane.
//!
//! Part of `SettingsView` — see `crate::view`. Mechanical T16-007 split, no
//! logic change.

use crate::view::*;

impl SettingsView {
    /// The floating options list for an open `Select` (T16-010). Rendered as a
    /// `deferred` + `anchored` layer so it is not clipped by the scroll area,
    /// with a transparent full-window backdrop that dismisses it.
    pub(crate) fn render_dropdown(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let menu = self.dropdown.as_ref()?;
        let key = menu.key;
        let sentinel = menu.default_sentinel.clone();
        let stored = self
            .prefs
            .read(cx)
            .value(key)
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        // The row highlighted as "current": the stored value, or the sentinel
        // when the stored value is empty.
        let cur: SharedString = if stored.is_empty() {
            sentinel.clone().unwrap_or_default()
        } else {
            SharedString::from(stored)
        };
        let list = anchored().position(menu.at).snap_to_window().child(
            div()
                .id("dropdown-list")
                .occlude()
                .min_w(px(180.0))
                .max_h(px(320.0))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .p_1()
                .rounded_md()
                .bg(c.card)
                .border_1()
                .border_color(c.border)
                .children(menu.options.iter().enumerate().map(|(i, opt)| {
                    let opt = opt.clone();
                    let selected = opt == cur;
                    let is_sentinel = sentinel.as_ref() == Some(&opt);
                    div()
                        .id(SharedString::from(format!("opt-{key}-{i}")))
                        .px_2()
                        .py(px(4.0))
                        .rounded_sm()
                        .text_size(px(11.5))
                        .text_color(if selected { c.fg } else { c.muted })
                        .when(selected, |d| d.bg(c.accent))
                        .when(!selected, |d| d.hover(|s| s.bg(c.border)))
                        .child(opt.clone())
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.dropdown = None;
                            let v = if is_sentinel {
                                String::new()
                            } else {
                                opt.to_string()
                            };
                            this.set_pref(key, Value::String(v), cx);
                        }))
                })),
        );
        Some(
            deferred(
                div()
                    .absolute()
                    .inset_0()
                    .child(div().id("dropdown-backdrop").absolute().inset_0().on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| {
                            this.dropdown = None;
                            cx.notify();
                        }),
                    ))
                    .child(list),
            )
            .with_priority(200)
            .into_any_element(),
        )
    }

    pub(crate) fn render_field(
        &self,
        def: &FieldDef,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let key = def.key;
        let control = match def.kind {
            FieldKind::Switch => {
                let on = self
                    .prefs
                    .read(cx)
                    .value(key)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                div()
                    .id(SharedString::from(format!("sw-{key}")))
                    .w(px(38.0))
                    .h(px(20.0))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .px(px(2.0))
                    .bg(if on { c.accent } else { c.border })
                    .child(
                        div()
                            .size(px(16.0))
                            .rounded_full()
                            .bg(c.bg)
                            .when(on, |d| d.ml(px(16.0))),
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.toggle_bool(key, cx);
                    }))
                    .into_any_element()
            }
            FieldKind::Int { min, max, step } => {
                let cur = self
                    .prefs
                    .read(cx)
                    .value(key)
                    .and_then(|v| v.as_i64())
                    .unwrap_or(min);
                let frac = if max > min {
                    (cur - min) as f32 / (max - min) as f32
                } else {
                    0.0
                };
                div()
                    .flex()
                    .flex_col()
                    .items_end()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(step_btn("dec", key, "\u{2212}", c, cx, move |this, cx| {
                                this.bump_int(key, min, max, -step, cx)
                            }))
                            .child(
                                div()
                                    .min_w(px(52.0))
                                    .text_center()
                                    .text_color(c.fg)
                                    .child(SharedString::from(cur.to_string())),
                            )
                            .child(step_btn("inc", key, "+", c, cx, move |this, cx| {
                                this.bump_int(key, min, max, step, cx)
                            })),
                    )
                    .child(slider_track(frac, c))
                    .into_any_element()
            }
            FieldKind::Select(options) => {
                let cur = self
                    .prefs
                    .read(cx)
                    .value(key)
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default();
                let is_open = self.dropdown.as_ref().is_some_and(|d| d.key == key);
                div()
                    .id(SharedString::from(format!("sel-{key}")))
                    .min_w(px(160.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_2()
                    .py(px(4.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(if is_open { c.accent } else { c.border })
                    .bg(c.bg)
                    .text_color(c.fg)
                    .text_size(px(11.5))
                    .child(SharedString::from(cur))
                    .child(div().text_color(c.muted).child("\u{25BE}"))
                    .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                        if this.dropdown.as_ref().is_some_and(|d| d.key == key) {
                            this.dropdown = None;
                        } else {
                            this.dropdown = Some(SelectMenu {
                                key,
                                options: options.iter().map(|s| SharedString::from(*s)).collect(),
                                at: ev.position(),
                                default_sentinel: None,
                            });
                        }
                        cx.notify();
                    }))
                    .into_any_element()
            }
            FieldKind::Float {
                min_centi,
                max_centi,
                step_centi,
            } => {
                let cur = self
                    .prefs
                    .read(cx)
                    .value(key)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(min_centi as f64 / 100.0);
                let frac = if max_centi > min_centi {
                    ((cur * 100.0) as f32 - min_centi as f32) / (max_centi - min_centi) as f32
                } else {
                    0.0
                };
                div()
                    .flex()
                    .flex_col()
                    .items_end()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(step_btn("dec", key, "\u{2212}", c, cx, move |this, cx| {
                                this.bump_float(key, min_centi, max_centi, -step_centi, cx)
                            }))
                            .child(
                                div()
                                    .min_w(px(52.0))
                                    .text_center()
                                    .text_color(c.fg)
                                    .child(SharedString::from(format!("{cur:.2}"))),
                            )
                            .child(step_btn("inc", key, "+", c, cx, move |this, cx| {
                                this.bump_float(key, min_centi, max_centi, step_centi, cx)
                            })),
                    )
                    .child(slider_track(frac, c))
                    .into_any_element()
            }
            FieldKind::FontFamily => {
                let cur = self
                    .prefs
                    .read(cx)
                    .value(key)
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default();
                let is_open = self.dropdown.as_ref().is_some_and(|d| d.key == key);
                let label = if cur.is_empty() {
                    "(default)".to_string()
                } else {
                    cur
                };
                let fonts = self.system_fonts.clone();
                div()
                    .id(SharedString::from(format!("font-{key}")))
                    .min_w(px(200.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_2()
                    .py(px(4.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(if is_open { c.accent } else { c.border })
                    .bg(c.bg)
                    .text_color(c.fg)
                    .text_size(px(11.5))
                    .child(SharedString::from(label))
                    .child(div().text_color(c.muted).child("\u{25BE}"))
                    .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                        if this.dropdown.as_ref().is_some_and(|d| d.key == key) {
                            this.dropdown = None;
                        } else {
                            let sentinel = SharedString::from("(default)");
                            let mut options = vec![sentinel.clone()];
                            options.extend(fonts.iter().cloned());
                            this.dropdown = Some(SelectMenu {
                                key,
                                options,
                                at: ev.position(),
                                default_sentinel: Some(sentinel),
                            });
                        }
                        cx.notify();
                    }))
                    .into_any_element()
            }
            FieldKind::Text => {
                let editing = self
                    .editing
                    .as_ref()
                    .filter(|e| e.key == key)
                    .map(|e| e.buffer.clone());
                let value = editing.clone().unwrap_or_else(|| {
                    self.prefs
                        .read(cx)
                        .value(key)
                        .and_then(|v| v.as_str().map(str::to_string))
                        .unwrap_or_default()
                });
                let active = editing.is_some();
                let empty = value.is_empty();
                div()
                    .id(SharedString::from(format!("txt-{key}")))
                    .w(px(200.0))
                    .px_2()
                    .py(px(3.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(if active { c.accent } else { c.border })
                    .bg(c.bg)
                    .text_color(if empty { c.muted } else { c.fg })
                    .child(SharedString::from(if empty {
                        "(default)".to_string()
                    } else {
                        value
                    }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.begin_edit(key, false, cx);
                    }))
                    .into_any_element()
            }
        };

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .py_2()
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .flex_1()
                    .min_w_0()
                    .child(div().text_color(c.fg).child(SharedString::from(def.title)))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(c.muted)
                            .child(SharedString::from(def.desc)),
                    ),
            )
            .child(control)
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
                div()
                    .id("mcp-regen")
                    .px_2()
                    .py(px(3.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(c.border)
                    .text_color(c.fg)
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
                        div()
                            .id("mcp-copy")
                            .px_2()
                            .py(px(3.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(c.border)
                            .text_color(c.fg)
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

    pub(crate) fn render_body(&mut self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let query = self.search.trim().to_lowercase();
        if CATEGORIES[self.active_cat] == KEYBOARD && query.is_empty() {
            return self.render_shortcuts(&query, c, cx);
        }
        if !query.is_empty() {
            // Global search: results grouped by their top-level section, a port
            // of the reference `SearchResults` layout.
            let mut root = div().flex().flex_col();
            let mut any = false;
            for &cat in CATEGORIES {
                let matches: Vec<&FieldDef> = FIELDS
                    .iter()
                    .filter(|f| {
                        f.category == cat
                            && (f.title.to_lowercase().contains(&query)
                                || f.desc.to_lowercase().contains(&query)
                                || f.key.to_lowercase().contains(&query))
                    })
                    .collect();
                if matches.is_empty() {
                    continue;
                }
                any = true;
                root = root.child(section_label(cat, c)).children(
                    matches
                        .into_iter()
                        .map(|f| self.render_field(f, c, cx))
                        .collect::<Vec<_>>(),
                );
            }
            if !any {
                return div()
                    .p_4()
                    .text_color(c.muted)
                    .child("No matching settings.")
                    .into_any_element();
            }
            return root.into_any_element();
        }

        let cat = CATEGORIES[self.active_cat];
        match cat {
            "General" => return self.render_general(c, cx),
            "Themes" => return self.render_themes(c, cx),
            _ if cat == CAT_APPEARANCE => return self.render_appearance(c, cx),
            "Connections" => {
                return div()
                    .flex()
                    .flex_col()
                    .child(self.render_grouped(cat, c, cx))
                    .child(section_label(AGENT_BRIDGE, c))
                    .child(self.render_agent_bridge(c, cx))
                    .into_any_element();
            }
            "AI" => {
                return div()
                    .flex()
                    .flex_col()
                    .child(self.render_grouped(cat, c, cx))
                    .child(section_label("Providers", c))
                    .child(self.render_providers(c, cx))
                    .child(section_label("Agents", c))
                    .child(self.render_agents_section(c, cx))
                    .child(section_label("Directives", c))
                    .child(self.render_directives_section(c, cx))
                    .children(self.render_ai_editor(c, cx))
                    .into_any_element();
            }
            _ => {}
        }
        self.render_grouped(cat, c, cx)
    }

    /// `sessionRestore`). An unknown key is always visible.
    pub(crate) fn field_visible(&self, key: &str, cx: &App) -> bool {
        let p = self.prefs.read(cx).get();
        match key {
            "sessionScrollbackLines" | "scrollbackMaxSizeMb" | "scrollbackRetentionDays" => {
                p.session_restore
            }
            "terminalCursorBlinkInterval" => p.terminal_cursor_blink,
            "terminalComposerHistoryPopup" | "terminalComposerArgumentCompletion" => {
                p.terminal_composer_enabled
            }
            "terminalBlocksAutoCollapseOnAltScreen" => p.terminal_blocks_enabled,
            "editorAutoSaveDelay" => p.editor_auto_save != "off",
            "sshAutoReconnectDelay" | "sshAutoReconnectMaxAttempts" => p.ssh_auto_reconnect,
            "explorerIdleSessionTimeoutMin"
            | "explorerMaxIdleSessions"
            | "explorerMaxCachedRemoteScopes" => p.explorer_auto_reconnect,
            "autocompleteProvider" | "autocompleteModelId" => p.autocomplete_enabled,
            "bookmarksActionNewTerminal"
            | "bookmarksActionCurrentTerminal"
            | "bookmarksActionCurrentSftp"
            | "bookmarksActionNewSftp"
            | "bookmarksPrimaryClickBehavior"
            | "bookmarksShowBadge" => p.bookmarks_enabled,
            "commandPaletteHistorySize" => p.command_palette_show_recent,
            _ => true,
        }
    }

    /// Render a category's fields, split into the reference sub-sections
    /// (`SECTION_GROUPS`); any field not listed in a group falls through to a
    /// trailing "Other" block so nothing is ever silently dropped.
    pub(crate) fn render_grouped(
        &self,
        cat: &str,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let groups = SECTION_GROUPS
            .iter()
            .find(|(g, _)| *g == cat)
            .map(|(_, g)| *g)
            .unwrap_or(&[]);
        let mut placed: Vec<&str> = Vec::new();
        let mut root = div().flex().flex_col();
        for (label, keys) in groups {
            placed.extend(keys.iter().copied());
            let defs: Vec<&FieldDef> = keys
                .iter()
                .filter_map(|k| FIELDS.iter().find(|f| f.key == *k && f.category == cat))
                .filter(|f| self.field_visible(f.key, cx))
                .collect();
            if defs.is_empty() {
                continue;
            }
            root = root.child(section_label(label, c)).children(
                defs.into_iter()
                    .map(|f| self.render_field(f, c, cx))
                    .collect::<Vec<_>>(),
            );
        }
        let leftover: Vec<&FieldDef> = FIELDS
            .iter()
            .filter(|f| {
                f.category == cat && !placed.contains(&f.key) && self.field_visible(f.key, cx)
            })
            .collect();
        if !leftover.is_empty() {
            if !groups.is_empty() {
                root = root.child(section_label("Other", c));
            }
            root = root.children(
                leftover
                    .into_iter()
                    .map(|f| self.render_field(f, c, cx))
                    .collect::<Vec<_>>(),
            );
        }
        root.into_any_element()
    }
}
