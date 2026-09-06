//! Shortcuts pane: keybind capture / conflict resolution and `render_shortcuts`.
//!
//! Part of `SettingsView` — see `crate::view`. T19-008: the backing store
//! moved from the legacy `Preferences.keybinds` blob to the merged
//! `keymap.json` (`crate::keymap_edit`'s surgical writer + the
//! `KeybindDisplay` GPUI global `labonair-shell` republishes on every
//! load/reload); the capture/conflict UI logic itself is unchanged.

use crate::view::*;

impl SettingsView {
    // ── keyboard shortcuts ────────────────────────────────────────────────

    /// The current effective-binding display map (T19-008): absent = runs on
    /// the shipped default, `Some(keystrokes)` = user override, `Some("")` =
    /// explicitly unbound. Published by `labonair-shell::keymap_loader`.
    pub(crate) fn keybinds(&self, cx: &App) -> KeybindMap {
        cx.try_global::<labonair_command_palette::KeybindDisplay>()
            .map(|d| d.0.clone())
            .unwrap_or_default()
    }

    /// After a `keymap.json` write, ask the shell to reload + re-apply the
    /// live bindings (updates `KeybindDisplay` too) — same
    /// `KeybindApplyHook` indirection `crate::window` already uses.
    fn refresh_keymap(&mut self, cx: &mut Context<Self>) {
        crate::window::apply_keybinds(cx);
        cx.notify();
    }

    /// Translate a captured keystroke into a persisted override (or a
    /// conflict prompt / rejection).
    pub(crate) fn capture_shortcut(
        &mut self,
        id: ShortcutId,
        binding: String,
        cx: &mut Context<Self>,
    ) {
        let map = self.keybinds(cx);
        match capture_keybind(&map, id, &binding) {
            KbCapture::Set => {
                self.kb_conflict = None;
                if let Err(e) = crate::keymap_edit::rebind(id, &binding) {
                    self.notify_error(cx, "Keymap", e);
                    return;
                }
                self.refresh_keymap(cx);
            }
            KbCapture::Conflict(other) => {
                self.kb_conflict = Some(KbConflict { id, binding, other });
                cx.notify();
            }
            KbCapture::Reserved(label) => {
                self.notify_error(
                    cx,
                    "Reserved shortcut",
                    format!("{binding} is reserved for \u{201c}{label}\u{201d}."),
                );
                cx.notify();
            }
        }
    }

    pub(crate) fn resolve_kb_conflict(&mut self, cx: &mut Context<Self>) {
        let Some(kc) = self.kb_conflict.take() else {
            return;
        };
        if let Err(e) = crate::keymap_edit::rebind_with_overwrite(kc.id, kc.other, &kc.binding) {
            self.notify_error(cx, "Keymap", e);
            return;
        }
        self.refresh_keymap(cx);
    }

    pub(crate) fn reset_keybind(&mut self, id: ShortcutId, cx: &mut Context<Self>) {
        let map = self.keybinds(cx);
        if !map.contains_key(shortcut_slug(id)) {
            return; // already on the default — nothing to reset.
        }
        if let Err(e) = crate::keymap_edit::unbind(id) {
            self.notify_error(cx, "Keymap", e);
            return;
        }
        self.refresh_keymap(cx);
    }

    pub(crate) fn reset_all_keybinds(&mut self, cx: &mut Context<Self>) {
        self.kb_conflict = None;
        self.recording = None;
        if let Err(e) = crate::keymap_edit::reset_all() {
            self.notify_error(cx, "Keymap", e);
            return;
        }
        self.refresh_keymap(cx);
    }

    /// Handle a key press while a shortcut row is recording.
    pub(crate) fn record_key(
        &mut self,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(id) = self.recording else { return };
        let ks = &ev.keystroke;
        let key = ks.key.as_str();
        window.focus(&self.focus);
        if key == "escape" {
            self.recording = None;
            self.kb_conflict = None;
            cx.notify();
            return;
        }
        // A bare modifier press just updates the live hint — keep waiting.
        if matches!(
            key,
            "cmd" | "ctrl" | "control" | "alt" | "option" | "shift" | "fn" | "function"
        ) {
            return;
        }
        // The reference `eventToBinding` requires a non-shift modifier.
        if !(ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt) {
            self.notify_error(
                cx,
                "Shortcut needs a modifier",
                "Combine the key with \u{2318}, \u{2303} or \u{2325}.".to_string(),
            );
            return;
        }
        let binding = ks.unparse();
        self.recording = None;
        self.capture_shortcut(id, binding, cx);
    }

    pub(crate) fn render_shortcuts(
        &self,
        query: &str,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let overrides = self.keybinds(cx);
        let recording = self.recording;
        let conflict_id = self.kb_conflict.as_ref().map(|k| k.id);

        // T19-004: the one `keymap` `SettingsContent` field — which preset a
        // reset seeds from — is rendered directly here rather than through
        // the generic grid (`pages::DEDICATED_PANE_EXEMPTIONS`).
        let mut root = div().flex().flex_col();
        if let Some(base_keymap) = self
            .all_fields
            .iter()
            .find(|f| f.json_path == "keymap.baseKeymap")
            .copied()
        {
            let origin = self.field_origin(&base_keymap, cx);
            let value = self.field_value(&base_keymap, cx);
            root = root.child(self.render_field(&base_keymap, origin, value, c, cx));
        }
        let mut root = root.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .py_2()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(11.0))
                        .text_color(c.muted)
                        .child(
                            "Click a shortcut, then press the new key combination. Esc cancels.",
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap_1()
                        // T20-003: shared `button()` primitive (`Outline`/`Xs`).
                        .child(
                            button(
                                "kb-open-keymap-json",
                                *c,
                                ButtonVariant::Outline,
                                ButtonSize::Xs,
                            )
                            .child("Open keymap.json")
                            .on_click(cx.listener(
                                |this, _: &ClickEvent, window, cx| {
                                    this.workspace.update(cx, |w, cx| {
                                        w.open_or_create_user_keymap_json(window, cx)
                                    });
                                },
                            )),
                        )
                        .child(
                            button("kb-reset-all", *c, ButtonVariant::Outline, ButtonSize::Xs)
                                .child("Reset all")
                                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                    this.reset_all_keybinds(cx)
                                })),
                        ),
                ),
        );

        for s in shortcuts() {
            if !query.is_empty()
                && !s.label.to_lowercase().contains(query)
                && !shortcut_slug(s.id).to_lowercase().contains(query)
            {
                continue;
            }
            let id = s.id;
            let slug = shortcut_slug(id);
            let overridden = overrides.contains_key(slug);
            let is_rec = recording == Some(id);
            let display = if is_rec {
                "Press keys\u{2026}".to_string()
            } else {
                effective_binding(id, &overrides).unwrap_or_else(|| "Disabled".to_string())
            };

            // T20-003: the binding row is a shared `ListItem` — the
            // record-pill and the "Reset" action keep their own `on_click`
            // handlers (via `.trailing()`) rather than the row's own, so
            // clicking the label doesn't start recording.
            let trailing = div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    // The recording pill needs a dynamic accent-border
                    // "recording" state `button()`'s variants don't express;
                    // documented exception.
                    div()
                        .id(SharedString::from(format!("kb-rec-{slug}")))
                        .px_2()
                        .py(px(3.0))
                        .min_w(px(120.0))
                        .text_center()
                        .rounded_sm()
                        .border_1()
                        .border_color(if is_rec { c.accent } else { c.border })
                        .bg(c.bg)
                        .text_color(c.fg)
                        .hover(|st| st.bg(c.border))
                        .child(SharedString::from(display))
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.recording = Some(id);
                            this.kb_conflict = None;
                            window.focus(&this.focus);
                            cx.notify();
                        })),
                )
                .when(overridden, |d| {
                    d.child(
                        button(
                            SharedString::from(format!("kb-reset-{slug}")),
                            *c,
                            ButtonVariant::Ghost,
                            ButtonSize::Xs,
                        )
                        .child("Reset")
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _w, cx| {
                                this.reset_keybind(id, cx);
                            },
                        )),
                    )
                });
            let border = c.border;
            let row = ListItem::new(
                SharedString::from(format!("kb-row-{slug}")),
                c.fg,
                c.muted,
                c.accent,
            )
            .child(SharedString::from(s.label))
            .trailing(trailing)
            .extra(move |row| row.border_b_1().border_color(border));
            root = root.child(row);

            if conflict_id == Some(id) {
                let kc = self.kb_conflict.as_ref().unwrap();
                let msg = format!(
                    "{} is already used by \u{201c}{}\u{201d}.",
                    kc.binding,
                    shortcut(kc.other).label
                );
                // T20-003: the shared `Banner` (`Warning`) — message + the
                // two resolution actions as `button()`s.
                root = root.child(
                    banner(Severity::Warning, *c)
                        .child(div().flex_1().min_w_0().child(SharedString::from(msg)))
                        .child(
                            div()
                                .flex()
                                .gap_1()
                                .child(
                                    button(
                                        "kb-conflict-overwrite",
                                        *c,
                                        ButtonVariant::Outline,
                                        ButtonSize::Xs,
                                    )
                                    .child("Overwrite")
                                    .on_click(cx.listener(
                                        |this, _: &ClickEvent, _w, cx| this.resolve_kb_conflict(cx),
                                    )),
                                )
                                .child(
                                    button(
                                        "kb-conflict-cancel",
                                        *c,
                                        ButtonVariant::Ghost,
                                        ButtonSize::Xs,
                                    )
                                    .child("Cancel")
                                    .on_click(cx.listener(
                                        |this, _: &ClickEvent, _w, cx| {
                                            this.kb_conflict = None;
                                            cx.notify();
                                        },
                                    )),
                                ),
                        ),
                );
            }
        }

        root.into_any_element()
    }
}
