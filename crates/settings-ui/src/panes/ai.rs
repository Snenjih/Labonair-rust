//! AI pane: agents + directives editors, and the provider list (`render_providers`).
//!
//! Part of `SettingsView` — see `crate::view`. Mechanical T16-007 split, no
//! logic change.

use crate::view::*;

impl SettingsView {
    pub(crate) fn refresh_agents_directives(&mut self) {
        use labonair_backend::modules::{agents, directives};
        let loaded = agents::load();
        let mut all = agents::builtin_agents();
        all.extend(loaded.custom);
        self.active_agent_id = if all.iter().any(|a| a.id == loaded.active_id) {
            loaded.active_id
        } else {
            agents::default_active_id()
        };
        self.agents = all;
        self.directives = directives::load();
    }

    pub(crate) fn save_custom_agents(&self) {
        use labonair_backend::modules::agents;
        let custom: Vec<agents::Agent> = self
            .agents
            .iter()
            .filter(|a| !a.built_in)
            .cloned()
            .collect();
        let _ = agents::save(&custom, &self.active_agent_id);
    }

    pub(crate) fn set_active_agent(&mut self, id: String, cx: &mut Context<Self>) {
        self.active_agent_id = id;
        self.save_custom_agents();
        cx.notify();
    }

    pub(crate) fn new_agent(&mut self, cx: &mut Context<Self>) {
        use labonair_backend::modules::agents;
        self.agents.push(agents::Agent {
            id: agents::new_agent_id(),
            name: "New Agent".to_string(),
            description: "Custom agent — edit in labonair-agents.json".to_string(),
            instructions: String::new(),
            icon: "spark".to_string(),
            built_in: false,
        });
        self.save_custom_agents();
        cx.notify();
    }

    pub(crate) fn delete_agent(&mut self, id: &str, cx: &mut Context<Self>) {
        self.agents.retain(|a| a.id != id);
        if self.active_agent_id == id {
            self.active_agent_id = labonair_backend::modules::agents::default_active_id();
        }
        self.save_custom_agents();
        cx.notify();
    }

    pub(crate) fn new_directive(&mut self, cx: &mut Context<Self>) {
        use labonair_backend::modules::directives;
        self.directives.push(directives::Directive {
            id: directives::new_directive_id(),
            handle: "new-directive".to_string(),
            name: "New Directive".to_string(),
            description: "Edit in labonair-directives.json".to_string(),
            content: String::new(),
        });
        let _ = directives::save(&self.directives);
        cx.notify();
    }

    pub(crate) fn delete_directive(&mut self, id: &str, cx: &mut Context<Self>) {
        self.directives.retain(|d| d.id != id);
        let _ = labonair_backend::modules::directives::save(&self.directives);
        cx.notify();
    }

    pub(crate) fn edit_agent(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(a) = self.agents.iter().find(|a| a.id == id) else {
            return;
        };
        self.ai_editor = Some(AiEditor {
            kind: AiEditKind::Agent,
            id: id.to_string(),
            labels: ["Name", "Description", "Instructions"],
            fields: [
                a.name.clone(),
                a.description.clone(),
                a.instructions.clone(),
            ],
            focus_idx: 0,
            multiline_last: true,
        });
        window.focus(&self.ai_editor_focus);
        cx.notify();
    }

    pub(crate) fn edit_directive(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = self.directives.iter().find(|d| d.id == id) else {
            return;
        };
        self.ai_editor = Some(AiEditor {
            kind: AiEditKind::Directive,
            id: id.to_string(),
            labels: ["Handle (#…)", "Name", "Content"],
            fields: [d.handle.clone(), d.name.clone(), d.content.clone()],
            focus_idx: 0,
            multiline_last: true,
        });
        window.focus(&self.ai_editor_focus);
        cx.notify();
    }

    pub(crate) fn save_ai_editor(&mut self, cx: &mut Context<Self>) {
        let Some(ed) = self.ai_editor.take() else {
            return;
        };
        match ed.kind {
            AiEditKind::Agent => {
                if let Some(a) = self.agents.iter_mut().find(|a| a.id == ed.id) {
                    a.name = ed.fields[0].trim().to_string();
                    a.description = ed.fields[1].trim().to_string();
                    a.instructions = ed.fields[2].clone();
                }
                self.save_custom_agents();
            }
            AiEditKind::Directive => {
                if let Some(d) = self.directives.iter_mut().find(|d| d.id == ed.id) {
                    d.handle =
                        labonair_backend::modules::directives::normalize_handle(&ed.fields[0]);
                    d.name = ed.fields[1].trim().to_string();
                    d.content = ed.fields[2].clone();
                }
                let _ = labonair_backend::modules::directives::save(&self.directives);
            }
        }
        cx.notify();
    }

    pub(crate) fn on_ai_editor_key(
        &mut self,
        ev: &gpui::KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ed) = self.ai_editor.as_mut() else {
            return;
        };
        let key = ev.keystroke.key.as_str();
        let shift = ev.keystroke.modifiers.shift;
        let multiline_field = ed.focus_idx == 2 && ed.multiline_last;
        match key {
            "escape" => {
                self.ai_editor = None;
                cx.notify();
            }
            "tab" => {
                ed.focus_idx = if shift {
                    (ed.focus_idx + 2) % 3
                } else {
                    (ed.focus_idx + 1) % 3
                };
                cx.notify();
            }
            "enter" => {
                if multiline_field && shift {
                    ed.fields[2].push('\n');
                    cx.notify();
                } else {
                    self.save_ai_editor(cx);
                }
            }
            "backspace" => {
                ed.fields[ed.focus_idx].pop();
                cx.notify();
            }
            _ => {
                if let Some(ch) = ev
                    .keystroke
                    .key_char
                    .as_ref()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                {
                    let i = ed.focus_idx;
                    ed.fields[i].push_str(ch);
                    cx.notify();
                }
            }
        }
    }

    pub(crate) fn render_ai_editor(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let ed = self.ai_editor.as_ref()?;
        let rows: Vec<_> = (0..3)
            .map(|i| {
                let focused = ed.focus_idx == i;
                let multiline = i == 2;
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .child(SharedString::from(ed.labels[i])),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .border_1()
                            .border_color(if focused { c.accent } else { c.border })
                            .bg(c.bg)
                            .text_size(px(11.0))
                            .text_color(c.fg)
                            .when(multiline, |d| d.min_h(px(96.0)).whitespace_normal())
                            .child(SharedString::from(if focused {
                                format!("{}\u{2502}", ed.fields[i])
                            } else {
                                ed.fields[i].clone()
                            })),
                    )
            })
            .collect();
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
                        this.ai_editor = None;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .track_focus(&self.ai_editor_focus)
                        .key_context("AiEditor")
                        .on_key_down(cx.listener(Self::on_ai_editor_key))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|_, _: &gpui::MouseDownEvent, _w, cx| cx.stop_propagation()),
                        )
                        .flex()
                        .flex_col()
                        .gap_2()
                        .w(px(440.0))
                        .p_3()
                        .rounded_md()
                        .bg(c.card)
                        .border_1()
                        .border_color(c.border)
                        .child(div().text_size(px(12.0)).text_color(c.fg).child(
                            if ed.kind == AiEditKind::Agent {
                                "Edit agent"
                            } else {
                                "Edit directive"
                            },
                        ))
                        .children(rows)
                        .child(
                            div()
                                .text_size(px(9.0))
                                .text_color(c.muted)
                                .child("Tab to switch field \u{00b7} Enter to save \u{00b7} Shift+Enter newline \u{00b7} Esc cancel"),
                        ),
                )
                .into_any_element(),
        )
    }

    /// The AI Providers section — a functional port of `AiSection`'s provider
    /// list + `ProviderInstanceCard` + `AddProviderDropdown`. Instances persist
    /// via `labonair_ai::InstanceStore`; API keys go to the OS keychain
    /// (`secret_store`), never the preferences JSON.
    pub(crate) fn render_providers(&self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let active_ref = self.instances.active_model_ref();
        let cards = self.instances.instances().iter().map(|inst| {
            let id = inst.id.clone();
            let id_key = id.clone();
            let has_key =
                labonair_ai::secret_store::get_instance_key(&*self.secrets, &inst.id).is_some();
            let needs_key = inst.provider_id.needs_key();
            let editing_key = self
                .editing
                .as_ref()
                .filter(|e| e.key == format!("provkey:{id}"))
                .map(|e| e.buffer.clone());
            div()
                .flex()
                .flex_col()
                .gap_1()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(c.border)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().text_size(px(11.5)).text_color(c.fg).child(
                            SharedString::from(format!(
                                "{}  ({})",
                                inst.name,
                                inst.provider_id.label()
                            )),
                        ))
                        .child(
                            div()
                                .id(SharedString::from(format!("prov-del-{id}")))
                                .px_2()
                                .py(px(1.0))
                                .rounded_sm()
                                .border_1()
                                .border_color(c.border)
                                .text_size(px(10.5))
                                .text_color(c.muted)
                                .hover(|s| s.text_color(c.fg))
                                .child("Remove")
                                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                    this.remove_provider(id_key.clone(), cx)
                                })),
                        ),
                )
                .when(needs_key, |d| {
                    let label = match &editing_key {
                        Some(buf) if buf.is_empty() => "\u{2022}\u{2022}\u{2022}".to_string(),
                        Some(buf) => "\u{2022}".repeat(buf.len().min(24)),
                        None if has_key => "API key set \u{2014} click to replace".to_string(),
                        None => "Set API key\u{2026}".to_string(),
                    };
                    let active = editing_key.is_some();
                    d.child(
                        div()
                            .id(SharedString::from(format!("prov-key-{id}")))
                            .px_2()
                            .py(px(3.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(if active { c.accent } else { c.border })
                            .bg(c.bg)
                            .text_size(px(11.0))
                            .text_color(if has_key || active { c.fg } else { c.muted })
                            .child(SharedString::from(label))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.begin_edit(&format!("provkey:{id}"), false, cx);
                            })),
                    )
                })
        });

        let mut root = div().flex().flex_col().gap_2();
        root = root.child(
            div()
                .text_size(px(11.0))
                .text_color(c.muted)
                .child(SharedString::from(format!("Active model: {active_ref}"))),
        );
        root = root.children(cards.collect::<Vec<_>>());
        root = root.child(section_label("Add provider", c)).child(
            div().flex().flex_wrap().gap_1().children(
                labonair_ai::ProviderId::ALL
                    .into_iter()
                    .map(|p| {
                        div()
                            .id(SharedString::from(format!("add-prov-{}", p.as_str())))
                            .px_2()
                            .py(px(2.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(c.border)
                            .text_size(px(10.5))
                            .text_color(c.fg)
                            .hover(|s| s.bg(c.border))
                            .child(SharedString::from(p.label()))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.add_provider(p, cx)
                            }))
                    })
                    .collect::<Vec<_>>(),
            ),
        );
        root.into_any_element()
    }

    pub(crate) fn render_agents_section(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let active = self.active_agent_id.clone();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .children(self.agents.iter().map(|a| {
                let id = a.id.clone();
                let id_del = a.id.clone();
                let on = a.id == active;
                let builtin = a.built_in;
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(if on { c.accent } else { c.border })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(c.fg)
                                    .child(SharedString::from(a.name.clone())),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(c.muted)
                                    .child(SharedString::from(a.description.clone())),
                            ),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("agent-active-{}", a.id)))
                            .px_2()
                            .py(px(2.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(c.border)
                            .text_size(px(10.5))
                            .text_color(if on { c.fg } else { c.muted })
                            .hover(|s| s.bg(c.border))
                            .child(if on { "Active" } else { "Set active" })
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.set_active_agent(id.clone(), cx)
                            })),
                    )
                    .when(!builtin, |d| {
                        let id_edit = id_del.clone();
                        d.child(
                            div()
                                .id(SharedString::from(format!("agent-edit-{id_del}")))
                                .px_2()
                                .py(px(2.0))
                                .rounded_sm()
                                .border_1()
                                .border_color(c.border)
                                .text_size(px(10.5))
                                .text_color(c.muted)
                                .hover(|s| s.text_color(c.fg))
                                .child("Edit")
                                .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                                    this.edit_agent(&id_edit, w, cx)
                                })),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("agent-del-{id_del}")))
                                .px_2()
                                .py(px(2.0))
                                .rounded_sm()
                                .border_1()
                                .border_color(c.border)
                                .text_size(px(10.5))
                                .text_color(c.muted)
                                .hover(|s| s.text_color(c.fg))
                                .child("Delete")
                                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                    this.delete_agent(&id_del, cx)
                                })),
                        )
                    })
            }))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .py_1()
                    .child(
                        div()
                            .id("agent-new")
                            .px_2()
                            .py(px(3.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(c.accent)
                            .text_color(c.fg)
                            .hover(|s| s.bg(c.border))
                            .child("New Agent")
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.new_agent(cx)),
                            ),
                    )
                    .child(
                        div()
                            .id("agent-folder")
                            .px_2()
                            .py(px(3.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(c.border)
                            .text_color(c.muted)
                            .hover(|s| s.text_color(c.fg))
                            .child("Open config folder")
                            .on_click(cx.listener(|_, _: &ClickEvent, _w, cx| {
                                cx.reveal_path(&config_dir());
                            })),
                    ),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(c.muted)
                    .child("Instructions are edited in labonair-agents.json."),
            )
            .into_any_element()
    }

    pub(crate) fn render_directives_section(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .children(self.directives.iter().map(|d| {
                let id_del = d.id.clone();
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(c.border)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div().text_size(px(11.5)).text_color(c.fg).child(
                                    SharedString::from(format!("#{} — {}", d.handle, d.name)),
                                ),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(c.muted)
                                    .child(SharedString::from(d.description.clone())),
                            ),
                    )
                    .child({
                        let id_edit = id_del.clone();
                        div()
                            .id(SharedString::from(format!("dir-edit-{id_edit}")))
                            .px_2()
                            .py(px(2.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(c.border)
                            .text_size(px(10.5))
                            .text_color(c.muted)
                            .hover(|s| s.text_color(c.fg))
                            .child("Edit")
                            .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                                this.edit_directive(&id_edit, w, cx)
                            }))
                    })
                    .child(
                        div()
                            .id(SharedString::from(format!("dir-del-{id_del}")))
                            .px_2()
                            .py(px(2.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(c.border)
                            .text_size(px(10.5))
                            .text_color(c.muted)
                            .hover(|s| s.text_color(c.fg))
                            .child("Delete")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.delete_directive(&id_del, cx)
                            })),
                    )
            }))
            .child(
                div()
                    .id("dir-new")
                    .mt_1()
                    .px_2()
                    .py(px(3.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(c.accent)
                    .text_color(c.fg)
                    .hover(|s| s.bg(c.border))
                    .child("New Directive")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.new_directive(cx))),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(c.muted)
                    .child("Content is edited in labonair-directives.json."),
            )
            .into_any_element()
    }

    // ── T19-004: AI is a Custom top-level category (`AREAS`) — its main
    // page folds the generic `AI_GROUPS` field grid in before a
    // `SubPageLink` to the bespoke provider/agent/directive sections (the
    // task's Notizen require AI to have at least one sub-page).

    pub(crate) fn render_ai_overview(
        &mut self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let grid = self.render_field_groups(AI_GROUPS, "ai", c, cx);
        div()
            .flex()
            .flex_col()
            .child(grid)
            .child(
                div()
                    .id("ai-goto-providers")
                    .mt_2()
                    .px_2()
                    .py(px(6.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded_sm()
                    .border_1()
                    .border_color(c.border)
                    .text_color(c.fg)
                    .hover(|s| s.bg(c.border))
                    .child("Providers, Agents & Directives")
                    .child(div().text_color(c.muted).child("\u{203A}"))
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.go_to_subpage(0, cx);
                    })),
            )
            .into_any_element()
    }

    pub(crate) fn render_ai_providers_subpage(
        &mut self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .child(section_label("Providers", c))
            .child(self.render_providers(c, cx))
            .child(section_label("Agents", c))
            .child(self.render_agents_section(c, cx))
            .child(section_label("Directives", c))
            .child(self.render_directives_section(c, cx))
            .children(self.render_ai_editor(c, cx))
            .into_any_element()
    }
}
