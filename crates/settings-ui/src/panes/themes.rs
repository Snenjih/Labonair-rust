//! Appearance + Themes panes: theme scan / import / export / community fetch, the appearance grid, and the theme card grid.
//!
//! The old "Titlebar & Status Bar Items" layout editor (bar/side/hidden
//! toggles over the transitional `BarItemId`/`BarLoc` model) was removed in
//! T18-005 — it had no live consumer (the titlebar and statusbar have each
//! rendered from their own registries since T17-003/T18-001) and its schema
//! had no equivalent for a titlebar that no longer has moveable items. The
//! statusbar's own items are now personalized directly via right-click
//! (`crates/workspace/src/status_bar.rs`); a dedicated settings page is
//! T18-007.
//!
//! Part of `SettingsView` — see `crate::view`. Mechanical T16-007 split, no
//! logic change.

use crate::view::*;

impl SettingsView {
    // ── appearance: themes ────────────────────────────────────────────────

    /// Rescans the user themes directory (`config_dir()/themes/*.json`) and
    /// rebuilds both [`Self::theme_files`] (the picker list) and the live
    /// [`labonair_theme::ThemeRegistry`] inside the shared `ThemeStore`. The
    /// built-in "Labonair" default is always first.
    pub(crate) fn refresh_themes(&mut self, cx: &mut Context<Self>) {
        self.theme_files = scan_themes(&themes_dir());
        self.theme
            .update(cx, |t, cx| t.reload_user_themes(&themes_dir(), cx));
        // Icon themes (T20-006) share the same refresh point.
        self.icon_theme_files = icon_theme_choices();
        self.theme.update(cx, |t, cx| {
            t.reload_user_icon_themes(&icon_themes_dir(), cx)
        });
    }

    // ── appearance: icon themes (T20-006) ────────────────────────────────

    /// Activate an icon theme by id (`"default"` = built-in glyph set) and
    /// persist the choice.
    pub(crate) fn set_icon_theme(&mut self, id: &str, cx: &mut Context<Self>) {
        match self
            .theme
            .update(cx, |t, cx| t.set_active_icon_theme(id, cx))
        {
            Ok(()) => {
                self.active_icon_theme_id = id.to_string();
                self.set_pref("iconTheme", Value::String(id.to_string()), cx);
            }
            Err(e) => self.notify_error(cx, "Failed to activate icon theme", e),
        }
        cx.notify();
    }

    /// File picker → copy the chosen `.json` into the icon-themes dir → activate.
    pub(crate) fn import_icon_theme(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import icon theme".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(src) = paths.into_iter().next() else {
                return;
            };
            let _ = this.update(cx, |this, cx| this.import_icon_theme_from(src, cx));
        })
        .detach();
    }

    pub(crate) fn import_icon_theme_from(&mut self, src: PathBuf, cx: &mut Context<Self>) {
        let raw = match fs::read_to_string(&src) {
            Ok(r) => r,
            Err(e) => return self.notify_error(cx, "Failed to read icon theme", e.to_string()),
        };
        if let Err(e) = labonair_theme::IconThemeContent::from_json(&raw) {
            return self.notify_error(cx, "Invalid icon theme file", e);
        }
        let stem = src
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("imported-icon-theme")
            .to_string();
        if let Err(e) = save_theme_file_in(&icon_themes_dir(), &stem, &raw) {
            return self.notify_error(cx, "Failed to save icon theme", e);
        }
        self.refresh_themes(cx);
        self.set_icon_theme(&stem, cx);
        self.notify(
            cx,
            Notification::success("Icon theme imported", "The icon theme is now active."),
        );
        cx.notify();
    }

    /// Load the scanned system font list once (async, off the main thread) for
    /// the `FontFamily` picker.
    pub(crate) fn load_system_fonts(&mut self, cx: &mut Context<Self>) {
        if !self.system_fonts.is_empty() {
            return;
        }
        let task = self
            .tokio
            .spawn(async { labonair_backend::modules::fonts::fonts_list_system().await });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(mut names)) = task.await {
                names.sort_by_key(|n| n.to_lowercase());
                let _ = this.update(cx, |this, cx| {
                    this.system_fonts = names.into_iter().map(SharedString::from).collect();
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Activates a listed theme by registry id (`"default"` reverts to the
    /// built-in light/dark themes; otherwise `"<file stem>/<variant>"` or a
    /// bare family id).
    pub(crate) fn activate_theme(&mut self, id: &str, cx: &mut Context<Self>) {
        match self.theme.update(cx, |t, cx| t.set_active_theme(id, cx)) {
            Ok(()) => {
                self.active_theme_id = (id != "default").then(|| id.to_string());
                self.set_pref("appTheme", Value::String(id.to_string()), cx);
            }
            Err(e) => {
                self.notify_error(cx, "Failed to activate theme", e);
                return;
            }
        }
        self.apply_stored_variant(id, cx);
        cx.notify();
    }

    /// The `"dark"`/`"light"` mode string currently resolved by the theme store.
    pub(crate) fn resolved_mode_str(&self, cx: &Context<Self>) -> &'static str {
        match self.theme.read(cx).mode() {
            labonair_theme::ThemeMode::Dark => "dark",
            labonair_theme::ThemeMode::Light => "light",
        }
    }

    /// The registry family id of the active (or given) theme id.
    fn family_of(&self, id: &str, cx: &Context<Self>) -> Option<String> {
        self.theme.read(cx).registry().family_of(id)
    }

    /// Re-apply the persisted `themeVariantOverrides[family][mode]` selection
    /// (if any) to the freshly-activated registry family.
    pub(crate) fn apply_stored_variant(&mut self, id: &str, cx: &mut Context<Self>) {
        let mode = self.resolved_mode_str(cx);
        let Some(family) = self.family_of(id, cx) else {
            return;
        };
        let key = self
            .prefs
            .read(cx)
            .get()
            .theme_variant_overrides
            .get(&family)
            .and_then(|v| v.get(mode))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        self.theme
            .update(cx, |t, cx| t.set_registry_variant(key, cx));
    }

    /// Persist and apply a named theme-variant selection for the active
    /// registry family (Catppuccin frappe / macchiato / mocha, …).
    pub(crate) fn set_theme_variant(&mut self, key: Option<String>, cx: &mut Context<Self>) {
        let Some(id) = self.active_theme_id.clone() else {
            return;
        };
        let Some(family) = self.family_of(&id, cx) else {
            return;
        };
        let mode = self.resolved_mode_str(cx);
        let mut overrides = self.prefs.read(cx).get().theme_variant_overrides.clone();
        {
            let entry = overrides
                .entry(family)
                .or_insert_with(|| Value::Object(Default::default()));
            if let Some(obj) = entry.as_object_mut() {
                match &key {
                    Some(k) => {
                        obj.insert(mode.to_string(), Value::String(k.clone()));
                    }
                    None => {
                        obj.remove(mode);
                    }
                }
            }
        }
        self.set_pref(
            "themeVariantOverrides",
            serde_json::to_value(&overrides).unwrap_or(Value::Null),
            cx,
        );
        self.theme
            .update(cx, |t, cx| t.set_registry_variant(key, cx));
        cx.notify();
    }

    /// Opens the file picker, copies the chosen `.json` into the themes dir and
    /// activates it (T02-003 wiring).
    pub(crate) fn import_theme(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import theme".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(src) = paths.into_iter().next() else {
                return;
            };
            let _ = this.update(cx, |this, cx| this.import_theme_from(src, cx));
        })
        .detach();
    }

    pub(crate) fn import_theme_from(&mut self, src: PathBuf, cx: &mut Context<Self>) {
        let raw = match fs::read_to_string(&src) {
            Ok(r) => r,
            Err(e) => return self.notify_error(cx, "Failed to read theme", e.to_string()),
        };
        // Accept both the new family format and the legacy `variants` map.
        if let Err(e) = labonair_theme::ThemeFamilyContent::from_json(&raw) {
            return self.notify_error(cx, "Invalid theme file", e);
        }
        let stem = src
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("imported-theme")
            .to_string();
        if let Err(e) = save_theme_file_in(&themes_dir(), &stem, &raw) {
            return self.notify_error(cx, "Failed to save theme", e);
        }
        self.refresh_themes(cx);
        // Activate the imported family's first variant (its full registry id).
        let id = self
            .theme_files
            .iter()
            .find(|t| t.id.split('/').next() == Some(stem.as_str()))
            .map(|t| t.id.clone())
            .unwrap_or(stem);
        self.activate_theme(&id, cx);
        self.notify(
            cx,
            Notification::success("Theme imported", "The theme is now active."),
        );
        cx.notify();
    }

    /// Exports the currently active theme to a user-chosen `.json` file.
    pub(crate) fn export_theme(&mut self, cx: &mut Context<Self>) {
        let name = self
            .active_theme_id
            .as_deref()
            .and_then(|id| self.theme_files.iter().find(|t| t.id == id))
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Labonair".to_string());
        let json = match self
            .theme
            .read(cx)
            .active_theme_file(name.clone())
            .to_json()
        {
            Ok(j) => j,
            Err(e) => return self.notify_error(cx, "Export failed", e),
        };
        let slug = slugify(&name);
        let receiver = cx.prompt_for_new_path(&config_dir(), Some(&format!("{slug}.json")));
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(dest))) = receiver.await else {
                return;
            };
            let _ = this.update(cx, |this, cx| match fs::write(&dest, &json) {
                Ok(()) => this.notify(
                    cx,
                    Notification::success("Theme exported", dest.to_string_lossy().to_string()),
                ),
                Err(e) => this.notify_error(cx, "Export failed", e.to_string()),
            });
        })
        .detach();
    }

    /// Deletes a user theme file. Built-in themes are protected. `id` may be a
    /// bare family id or a `"<stem>/<variant>"` id — the file stem is the part
    /// before the first `/`.
    pub(crate) fn delete_theme(&mut self, id: &str, cx: &mut Context<Self>) {
        let stem = id.split('/').next().unwrap_or(id);
        if stem == "default" {
            return;
        }
        if let Err(e) = delete_theme_in(&themes_dir(), stem) {
            self.notify_error(cx, "Failed to delete theme", e);
            return;
        }
        let dropped_active = self
            .active_theme_id
            .as_deref()
            .and_then(|a| a.split('/').next())
            == Some(stem);
        if dropped_active {
            self.theme.update(cx, |t, cx| {
                let _ = t.set_active_theme("default", cx);
            });
            self.active_theme_id = None;
            self.set_pref("appTheme", Value::String("default".into()), cx);
        }
        self.refresh_themes(cx);
        cx.notify();
    }

    // ── Community / marketplace (T16-018) ─────────────────────────────────

    /// Fetch the remote theme index; on failure fall back to the mock list.
    pub(crate) fn fetch_community_themes(&mut self, cx: &mut Context<Self>) {
        if self.community_loading {
            return;
        }
        self.community_loading = true;
        self.community_error = None;
        let jh = self.tokio.spawn(async move {
            labonair_backend::modules::themes::theme_fetch_index(COMMUNITY_INDEX_URL.to_string())
                .await
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                this.community_loading = false;
                match res.and_then(|raw| {
                    serde_json::from_str::<Vec<RemoteTheme>>(&raw).map_err(|e| e.to_string())
                }) {
                    Ok(list) => this.community_themes = list,
                    Err(_) => {
                        this.community_error = Some(
                            "Could not reach the theme registry — showing cached entries."
                                .to_string(),
                        );
                        this.community_themes = mock_community_themes();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn install_community_theme(&mut self, remote: RemoteTheme, cx: &mut Context<Self>) {
        if self.installing_themes.contains(&remote.id) {
            return;
        }
        self.installing_themes.insert(remote.id.clone());
        cx.notify();
        let app = self.backend.clone();
        let url = remote.raw_url.clone();
        let jh = self.tokio.spawn(async move {
            labonair_backend::modules::themes::theme_download(app, url).await
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                this.installing_themes.remove(&remote.id);
                match res {
                    Ok(_) => {
                        this.refresh_themes(cx);
                        this.notify(
                            cx,
                            Notification::success("Theme installed", remote.name.clone()),
                        );
                    }
                    Err(e) => this.notify_error(cx, "Install failed", e),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// "New Theme…" — seed a file from the default and activate it.
    pub(crate) fn create_theme(&mut self, name: String, cx: &mut Context<Self>) {
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let app = self.backend.clone();
        let jh = self
            .tokio
            .spawn(async move { labonair_backend::modules::themes::theme_create(app, name).await });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                match res {
                    Ok((meta, _path)) => {
                        this.refresh_themes(cx);
                        this.activate_theme(&meta.id, cx);
                        this.notify(
                            cx,
                            Notification::success(
                                "Theme created",
                                "Edit it in the themes folder, then re-activate.".to_string(),
                            ),
                        );
                    }
                    Err(e) => this.notify_error(cx, "Create failed", e),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn on_new_theme_key(
        &mut self,
        ev: &gpui::KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(buf) = self.new_theme_prompt.as_mut() else {
            return;
        };
        match ev.keystroke.key.as_str() {
            "enter" => {
                let name = buf.clone();
                self.new_theme_prompt = None;
                self.create_theme(name, cx);
            }
            "escape" => {
                self.new_theme_prompt = None;
                cx.notify();
            }
            "backspace" => {
                buf.pop();
                cx.notify();
            }
            _ => {
                if let Some(ch) = ev
                    .keystroke
                    .key_char
                    .as_ref()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                {
                    buf.push_str(ch);
                    cx.notify();
                }
            }
        }
    }

    /// Themes pane — a card grid over the installed themes (built-in + user
    /// `~/.config/labonair/themes/*.json`), a port of `ThemeMarketplace` /
    /// `ThemeCard`. Community fetch is not wired (documented in T16-012).
    pub(crate) fn render_themes(&self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let active_id = self.active_theme_id.clone();
        let cards: Vec<_> = self
            .theme_files
            .iter()
            .map(|t| {
                let id = t.id.clone();
                let id_del = t.id.clone();
                let is_active = active_id.as_deref() == Some(t.id.as_str())
                    || (active_id.is_none() && t.id == "default");
                let builtin = t.builtin;
                div()
                    .w(px(180.0))
                    .flex()
                    .flex_col()
                    .rounded_md()
                    .overflow_hidden()
                    .border_1()
                    .border_color(if is_active { c.accent } else { c.border })
                    .child(
                        div()
                            .h(px(84.0))
                            .bg(c.bg)
                            .flex()
                            .items_end()
                            .p_2()
                            .gap_1()
                            .child(div().size(px(14.0)).rounded_sm().bg(c.accent))
                            .child(div().size(px(14.0)).rounded_sm().bg(c.muted))
                            .child(div().size(px(14.0)).rounded_sm().bg(c.border)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_2()
                            .child(
                                div()
                                    .text_color(c.fg)
                                    .text_size(px(11.5))
                                    .child(SharedString::from(t.name.clone())),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1()
                                    .child(
                                        button(
                                            SharedString::from(format!("theme-use-{}", t.id)),
                                            *c,
                                            if is_active {
                                                ButtonVariant::Secondary
                                            } else {
                                                ButtonVariant::Outline
                                            },
                                            ButtonSize::Xs,
                                        )
                                        .child(if is_active { "Active" } else { "Activate" })
                                        .on_click(
                                            cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                                this.activate_theme(&id, cx);
                                            }),
                                        ),
                                    )
                                    .when(!builtin, |d| {
                                        d.child(
                                            button(
                                                SharedString::from(format!("theme-del-{id_del}")),
                                                *c,
                                                ButtonVariant::Ghost,
                                                ButtonSize::Xs,
                                            )
                                            .child("Delete")
                                            .on_click(
                                                cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                                    this.delete_theme(&id_del, cx);
                                                }),
                                            ),
                                        )
                                    }),
                            ),
                    )
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .py_2()
                    // T20-003: shared `button()` primitive (`Outline`/`Xs`).
                    .child(
                        button("theme-import", *c, ButtonVariant::Outline, ButtonSize::Xs)
                            .child("Import theme\u{2026}")
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.import_theme(cx)),
                            ),
                    )
                    .child(
                        button("theme-export", *c, ButtonVariant::Outline, ButtonSize::Xs)
                            .child("Export active theme\u{2026}")
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.export_theme(cx)),
                            ),
                    )
                    .child(
                        button("theme-folder", *c, ButtonVariant::Outline, ButtonSize::Xs)
                            .child("Open themes folder")
                            .on_click(cx.listener(|_, _: &ClickEvent, _w, cx| {
                                cx.reveal_path(&themes_dir());
                            })),
                    )
                    .child(
                        button("theme-new", *c, ButtonVariant::Outline, ButtonSize::Xs)
                            .child("New Theme\u{2026}")
                            .on_click(cx.listener(|this, _: &ClickEvent, w, cx| {
                                this.new_theme_prompt = Some(String::new());
                                w.focus(&this.new_theme_focus);
                                cx.notify();
                            })),
                    ),
            )
            .child(self.render_theme_tabs(c, cx))
            .child(if self.themes_community_tab {
                self.render_community_themes(c, cx)
            } else {
                div()
                    .flex()
                    .flex_col()
                    .child(self.render_icon_theme_picker(c, cx))
                    .children(self.render_variant_picker(c, cx))
                    .child(div().flex().flex_wrap().gap_3().py_2().children(cards))
                    .into_any_element()
            })
            .children(self.render_new_theme_prompt(c, cx))
            .into_any_element()
    }

    /// Icon-theme picker: a segmented control over the installed icon themes,
    /// a preview row of representative glyphs resolved through the *selected*
    /// theme, and Import / Open-folder actions (T20-006).
    pub(crate) fn render_icon_theme_picker(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let active = self.active_icon_theme_id.clone();
        let choices: Vec<(String, String)> = if self.icon_theme_files.is_empty() {
            vec![("default".to_string(), "Labonair".to_string())]
        } else {
            self.icon_theme_files.clone()
        };

        // Preview glyphs for the currently-selected icon theme.
        let store = self.theme.read(cx);
        let preview_theme = store
            .icon_registry()
            .get(&active)
            .unwrap_or_else(|_| store.icon_registry().builtin_theme());
        let samples = [
            ("main.rs", false, false),
            ("app.tsx", false, false),
            ("data.json", false, false),
            ("README.md", false, false),
            ("styles.css", false, false),
            ("", true, false),
        ];
        let preview: Vec<gpui::AnyElement> = samples
            .iter()
            .map(|(name, is_dir, exp)| {
                icon_for_path(preview_theme, name, *is_dir, *exp)
                    .svg(c.muted)
                    .into_any_element()
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .py_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(c.muted)
                            .child("Icon theme"),
                    )
                    .child(
                        segmented_control("icon-theme", *c, active.clone())
                            .segments(choices)
                            .on_select(cx.listener(|this, key: &SharedString, _w, cx| {
                                this.set_icon_theme(key.as_ref(), cx);
                            })),
                    ),
            )
            .child(div().flex().items_center().gap_2().children(preview))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button(
                            "icon-theme-import",
                            *c,
                            ButtonVariant::Outline,
                            ButtonSize::Xs,
                        )
                        .child("Import icon theme\u{2026}")
                        .on_click(
                            cx.listener(|this, _: &ClickEvent, _w, cx| this.import_icon_theme(cx)),
                        ),
                    )
                    .child(
                        button(
                            "icon-theme-folder",
                            *c,
                            ButtonVariant::Outline,
                            ButtonSize::Xs,
                        )
                        .child("Open icon themes folder")
                        .on_click(cx.listener(
                            |_, _: &ClickEvent, _w, cx| {
                                cx.reveal_path(&icon_themes_dir());
                            },
                        )),
                    ),
            )
            .into_any_element()
    }

    pub(crate) fn render_theme_tabs(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        // T20-001: shared `SegmentedControl`.
        segmented_control(
            "theme-tabs",
            *c,
            if self.themes_community_tab {
                "community"
            } else {
                "installed"
            },
        )
        .segments([("installed", "Installed"), ("community", "Community")])
        .on_select(cx.listener(|this, key: &SharedString, _w, cx| {
            let community = key.as_ref() == "community";
            this.themes_community_tab = community;
            if community && this.community_themes.is_empty() {
                this.fetch_community_themes(cx);
            }
            cx.notify();
        }))
        .into_any_element()
    }

    pub(crate) fn render_community_themes(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        // `theme_files` ids are `"<stem>/<variant>"`; a community entry's id is
        // the bare stem — compare on the stem.
        let installed: std::collections::HashSet<&str> = self
            .theme_files
            .iter()
            .map(|t| t.id.split('/').next().unwrap_or(t.id.as_str()))
            .collect();
        let cards: Vec<_> =
            self.community_themes
                .iter()
                .map(|r| {
                    let is_installed = installed.contains(r.id.as_str());
                    let is_installing = self.installing_themes.contains(&r.id);
                    let remote = r.clone();
                    let id_un = r.id.clone();
                    div()
                        .w(px(220.0))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(c.border)
                        .child(
                            div()
                                .text_color(c.fg)
                                .text_size(px(12.0))
                                .child(SharedString::from(r.name.clone())),
                        )
                        .child(div().text_size(px(10.0)).text_color(c.muted).child(
                            SharedString::from(if r.author.is_empty() {
                                r.description.clone()
                            } else {
                                format!("{} \u{2014} {}", r.author, r.description)
                            }),
                        ))
                        // T20-003: shared `button()` primitive (`Xs`).
                        .child(if is_installed {
                            button(
                                SharedString::from(format!("comm-un-{id_un}")),
                                *c,
                                ButtonVariant::Ghost,
                                ButtonSize::Xs,
                            )
                            .child("Uninstall")
                            .on_click(cx.listener(
                                move |this, _: &ClickEvent, _w, cx| {
                                    this.delete_theme(&id_un, cx);
                                },
                            ))
                        } else {
                            button(
                                SharedString::from(format!("comm-in-{}", r.id)),
                                *c,
                                ButtonVariant::Outline,
                                ButtonSize::Xs,
                            )
                            .child(if is_installing {
                                "Installing\u{2026}"
                            } else {
                                "Install"
                            })
                            .on_click(cx.listener(
                                move |this, _: &ClickEvent, _w, cx| {
                                    this.install_community_theme(remote.clone(), cx);
                                },
                            ))
                        })
                })
                .collect();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .py_2()
            .children(self.community_error.clone().map(|e| {
                div()
                    .text_size(px(10.5))
                    .text_color(c.muted)
                    .child(SharedString::from(e))
            }))
            .child(if self.community_loading {
                div()
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .child("Loading community themes\u{2026}")
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .children(cards)
                    .into_any_element()
            })
            .into_any_element()
    }

    pub(crate) fn render_new_theme_prompt(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let buf = self.new_theme_prompt.as_ref()?;
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(labonair_theme::modal_scrim())
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _: &gpui::MouseDownEvent, _w, cx| {
                        this.new_theme_prompt = None;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .track_focus(&self.new_theme_focus)
                        .key_context("NewThemePrompt")
                        .on_key_down(cx.listener(Self::on_new_theme_key))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|_, _: &gpui::MouseDownEvent, _w, cx| {
                                cx.stop_propagation()
                            }),
                        )
                        .flex()
                        .flex_col()
                        .gap_2()
                        .w(px(320.0))
                        .p_3()
                        .rounded_md()
                        .bg(c.card)
                        .border_1()
                        .border_color(c.border)
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(c.fg)
                                .child("New theme name"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded_sm()
                                .border_1()
                                .border_color(c.accent)
                                .text_size(px(12.0))
                                .text_color(c.fg)
                                .child(SharedString::from(format!("{buf}\u{2502}"))),
                        ),
                )
                .into_any_element(),
        )
    }

    /// A segmented control over the named variants of the active registry
    /// family for the current appearance (only rendered when the family exposes
    /// more than one — e.g. Catppuccin frappe / macchiato / mocha).
    pub(crate) fn render_variant_picker(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let id = self.active_theme_id.clone()?;
        let t = self.theme.read(cx);
        let want = match t.mode() {
            labonair_theme::ThemeMode::Dark => labonair_theme::Appearance::Dark,
            labonair_theme::ThemeMode::Light => labonair_theme::Appearance::Light,
        };
        let family = t.registry().family_of(&id)?;
        let choices: Vec<(String, String)> = t
            .registry()
            .family_variants(&family)
            .into_iter()
            .filter(|(_, ap)| *ap == want)
            .map(|(name, _)| (name.clone(), name))
            .collect();
        if choices.len() < 2 {
            return None;
        }
        let current = t.registry_variant().map(|s| s.to_string());
        let active = current
            .clone()
            .unwrap_or_else(|| choices.first().map(|(k, _)| k.clone()).unwrap_or_default());
        Some(
            div()
                .flex()
                .items_center()
                .gap_2()
                .py_2()
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(c.muted)
                        .child("Variant"),
                )
                // T20-001: shared `SegmentedControl`.
                .child(
                    segmented_control("theme-variant", *c, active)
                        .segments(choices)
                        .on_select(cx.listener(|this, key: &SharedString, _w, cx| {
                            this.set_theme_variant(Some(key.to_string()), cx);
                        })),
                )
                .into_any_element(),
        )
    }
}
