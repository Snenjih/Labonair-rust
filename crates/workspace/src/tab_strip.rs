//! Tab strip rendering: the titlebar strip, the vertical sidebar list and the
//! per-tab block (R10-002).

use std::path::Path;
use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, point, px, Animation, AnimationExt, App, AppContext, ClipboardItem, Context,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use labonair_settings::{Settings as _, WorkspaceSettings};
use labonair_ui_kit::{
    caret, context_menu, divider, icon_button_builder, tab_item, Axis, ButtonSize, IconButtonShape,
    IconName, MenuItem, Palette, SubmenuHoverSource, TabLayout,
};

use labonair_command_palette_core::CommandId;

use crate::tabs::{Tab, TabDiagnosticStatus, TabGitStatus, TabKind};
use crate::{
    relative_path_under_root, DraggedTab, NewTabSubmenu, TabDragPreview, Workspace, TITLEBAR_OFFSET,
};

impl Workspace {
    /// Render one tab row. `sidebar == true` lays the row out for the vertical
    /// Tabs sidebar panel (full width, click-anchored menus, top-edge drop
    /// indicator); `false` is the horizontal titlebar strip.
    pub(crate) fn render_tab(
        &self,
        tab: &Tab,
        sidebar: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let fg = theme.foreground();
        let c = Palette::from_theme(theme);
        let id = tab.id;
        let active = self.tabs.read(cx).active_id() == id;
        let label = SharedString::from(tab.label());
        let status_color = (tab.kind == TabKind::Editor
            && WorkspaceSettings::try_get(cx)
                .is_some_and(|settings| settings.show_editor_tab_git_status()))
        .then_some(tab.git_status)
        .flatten()
        .map(|status| match status {
            TabGitStatus::Added => c.success,
            TabGitStatus::Modified => c.warning,
            TabGitStatus::Deleted => c.error,
        });
        let diagnostic_color = (tab.kind == TabKind::Editor
            && WorkspaceSettings::try_get(cx)
                .is_some_and(|settings| settings.show_editor_tab_diagnostics()))
        .then_some(tab.diagnostic_status)
        .flatten()
        .map(|status| match status {
            TabDiagnosticStatus::Error => c.error,
            TabDiagnosticStatus::Warning => c.warning,
        });

        // D4 — tab-entrance animation (`@keyframes labonair-tab-in` in the
        // reference `globals.css`: fade + `scale(0.86) → 1` over `--dur-base`
        // with `--ease-premium`). GPUI 0.2.2 `Div` can't take a scale
        // transform, so we animate opacity only. Reduce-motion mirrors the
        // reference's `animation-duration: 0.01ms` clamp rather than dropping
        // the keyframe entirely.
        // T20-007: `reduce_motion` + timings now come from the metric-aware
        // `ActiveTheme` — `ThemeStore::animation()` already reports zero
        // durations when reduce-motion is on. GPUI's `with_animation` can
        // divide by a zero duration, so we still clamp to a 10µs floor here
        // (mirrors the reference `animation-duration: 0.01ms`).
        let (ease, dur_base, reduce_motion) = {
            let store = self.theme.read(cx);
            let a = store.animation();
            (
                a.ease_premium,
                a.dur_base,
                store.active_theme().reduce_motion(),
            )
        };
        let tab_in_dur = if reduce_motion || dur_base.is_zero() {
            Duration::from_micros(10)
        } else {
            dur_base
        };

        let label_content = match self.rename_tab.as_ref().filter(|(rid, _)| *rid == id) {
            Some((_, buf)) => div()
                .track_focus(&self.rename_focus)
                .key_context("TabRename")
                .on_key_down(cx.listener(Self::on_rename_key))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseDownEvent, window, cx| {
                        window.focus(&this.rename_focus);
                        cx.stop_propagation();
                    }),
                )
                .min_w(px(80.0))
                .max_w(px(180.0))
                .px_1()
                .rounded_sm()
                .border_1()
                .border_color(theme.primary())
                .flex()
                .items_center()
                .child(SharedString::from(buf.clone()))
                .when(self.rename_blink.read(cx).visible(), |d| {
                    d.child(caret(fg, 12.0))
                })
                .into_any_element(),
            None => div().child(label.clone()).into_any_element(),
        };

        let view = cx.entity();
        let mut tab_item = tab_item(format!("workspace-tab-{id}"), c, label.clone());
        if sidebar || tab.kind != TabKind::Editor {
            tab_item = tab_item.leading(tab.kind.indicator());
        }
        tab_item = tab_item
            .layout(if sidebar {
                TabLayout::Vertical
            } else {
                TabLayout::Horizontal
            })
            .selected(active)
            .dirty(tab.kind == TabKind::Editor && tab.dirty)
            .peek(tab.kind == TabKind::Editor && tab.peek)
            .pinned(tab.pinned)
            .status_color(status_color)
            .status_marker(diagnostic_color)
            .label_content(label_content)
            .on_activate({
                let view = view.clone();
                move |window, app| {
                    view.update(app, |this, cx| this.select_tab(id, window, cx));
                }
            })
            .on_navigate({
                let view = view.clone();
                move |forward, window, app| {
                    view.update(app, |this, cx| this.cycle_from_tab(id, forward, window, cx));
                }
            })
            .on_double_click({
                let view = view.clone();
                move |window, app| {
                    view.update(app, |this, cx| {
                        if this.tabs.read(cx).get(id).is_some_and(|tab| tab.peek) {
                            this.tabs
                                .update(cx, |store, cx| store.set_peek(id, false, cx));
                        } else {
                            this.begin_tab_rename(id, window, cx);
                        }
                    });
                }
            });
        if !tab.pinned {
            tab_item = tab_item.on_close(move |window, app| {
                view.update(app, |this, cx| this.request_close(id, window, cx));
            });
        }
        let row = tab_item
            .render()
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                    let y = if sidebar {
                        ev.position.y
                    } else {
                        px(TITLEBAR_OFFSET)
                    };
                    this.context_menu = Some((id, point(ev.position.x, y)));
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    if !this.tabs.read(cx).get(id).is_some_and(|tab| tab.pinned) {
                        this.request_close(id, window, cx);
                    }
                }),
            )
            .on_drag(
                DraggedTab {
                    id,
                    label: label.clone(),
                },
                |dragged, _, _, cx| {
                    cx.new(|_| TabDragPreview {
                        label: dragged.label.clone(),
                    })
                },
            )
            .drag_over::<DraggedTab>(move |style, _, _, _| {
                let style = if sidebar {
                    style.border_t_2()
                } else {
                    style.border_l_2()
                };
                style.border_color(fg)
            })
            .on_drop(cx.listener(move |this, dragged: &DraggedTab, _window, cx| {
                this.tabs.update(cx, |s, cx| s.reorder(dragged.id, id, cx));
            }))
            .with_animation(
                ("tab-in", id),
                Animation::new(tab_in_dur).with_easing(move |t| ease.eval(t)),
                |el, delta| el.opacity(delta),
            );
        row
    }

    /// The tab strip. Rendered inside `AppShell`'s single overlay titlebar
    /// (the header) between the left/right bar-item buckets — mirrors the
    /// reference `Header.tsx` where the `TabBar` lives in the titlebar.
    pub fn render_tab_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (muted, fg, border) = (theme.muted_foreground(), theme.foreground(), theme.border());
        let c = Palette::from_theme(theme);
        let active_space_id = self.spaces.read(cx).active_id();
        // Only the active Space's tabs show in the titlebar strip — every
        // other Space's tabs stay alive (T20-008) but out of sight until
        // switched to. The sidebar's `render_tab_list_vertical` intentionally
        // shows every Space's tabs at once, grouped into sections instead.
        let tabs: Vec<Tab> = self
            .tabs
            .read(cx)
            .tabs_in_space(active_space_id)
            .into_iter()
            .cloned()
            .collect();
        let pinned_tabs: Vec<Tab> = tabs.iter().filter(|tab| tab.pinned).cloned().collect();
        let unpinned_tabs: Vec<Tab> = tabs.iter().filter(|tab| !tab.pinned).cloned().collect();

        let active_space = self.spaces.read(cx).active().cloned();
        let (show_nav_history_buttons, show_tab_bar_buttons) = WorkspaceSettings::try_get(cx)
            .map(|settings| {
                (
                    settings.show_nav_history_buttons(),
                    settings.show_tab_bar_buttons(),
                )
            })
            .unwrap_or((true, true));
        let editor_history = self.active_editor_history_state(cx);
        let can_cycle_tabs = tabs.len() > 1;
        let (can_go_back, can_go_forward) =
            editor_history.unwrap_or((can_cycle_tabs, can_cycle_tabs));
        let history_buttons_active = editor_history.is_some();
        let workspace = cx.entity();
        let previous = icon_button_builder("tab-previous", c, IconName::ArrowLeft)
            .size(ButtonSize::IconLg)
            .shape(IconButtonShape::Square)
            .disabled(!can_go_back)
            .tooltip(if history_buttons_active {
                "Back"
            } else {
                "Previous Tab"
            })
            .render()
            .when(can_go_back, |button| {
                button.on_click({
                    let workspace = workspace.clone();
                    move |_, window, app| {
                        workspace.update(app, |this, cx| this.navigate_tab_bar(false, window, cx));
                    }
                })
            });
        let next = icon_button_builder("tab-next", c, IconName::ArrowRight)
            .size(ButtonSize::IconLg)
            .shape(IconButtonShape::Square)
            .disabled(!can_go_forward)
            .tooltip(if history_buttons_active {
                "Forward"
            } else {
                "Next Tab"
            })
            .render()
            .when(can_go_forward, |button| {
                button.on_click({
                    let workspace = workspace.clone();
                    move |_, window, app| {
                        workspace.update(app, |this, cx| this.navigate_tab_bar(true, window, cx));
                    }
                })
            });
        let new_tab = icon_button_builder("tab-new", c, IconName::PlusBold)
            .size(ButtonSize::IconSm)
            .shape(IconButtonShape::Square)
            .tooltip("New Tab")
            .render()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, _window, cx| {
                    this.new_tab_menu = Some(point(ev.position.x, px(TITLEBAR_OFFSET)));
                    this.new_tab_submenu = None;
                    this.context_menu = None;
                    cx.notify();
                }),
            );

        let spaces_trigger = div()
            .id("spaces-trigger")
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_1()
            .h(px(24.0))
            .px_1p5()
            .rounded_md()
            .cursor_pointer()
            .hover(|s| s.bg(border))
            .child(IconName::Box.svg(muted).size(px(13.0)))
            .when_some(active_space, |d, space| {
                d.child(
                    div()
                        .max_w(px(96.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_xs()
                        .text_color(fg)
                        .child(space.name),
                )
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, _window, cx| {
                    this.spaces_menu = Some(point(ev.position.x, px(TITLEBAR_OFFSET)));
                    this.new_tab_menu = None;
                    this.context_menu = None;
                    cx.notify();
                }),
            );

        div()
            .flex()
            .items_center()
            .gap_1()
            .h(px(32.0))
            .w_full()
            .flex_shrink_0()
            .bg(c.toolbar_bg)
            .when(show_nav_history_buttons, |bar| {
                bar.child(
                    div()
                        .id("tab-navigation")
                        .flex()
                        .items_center()
                        .h_full()
                        .border_r_1()
                        .border_color(border)
                        .child(previous)
                        .child(next),
                )
            })
            .when(show_tab_bar_buttons, |bar| bar.child(spaces_trigger))
            .child(
                div()
                    .id("tab-strip")
                    .flex()
                    .items_center()
                    .gap_0()
                    .min_w_0()
                    .h_full()
                    .overflow_x_scroll()
                    .children(pinned_tabs.iter().map(|t| self.render_tab(t, false, cx)))
                    .when(
                        !pinned_tabs.is_empty() && !unpinned_tabs.is_empty(),
                        |strip| strip.child(div().mx_1().w(px(1.0)).h(px(16.0)).bg(border)),
                    )
                    .children(unpinned_tabs.iter().map(|t| self.render_tab(t, false, cx))),
            )
            .when(show_tab_bar_buttons, |bar| bar.child(new_tab))
    }

    /// The vertical tab list shown in the Tabs sidebar panel when
    /// `tabsLocation == "sidebar"`. Same behaviour as [`Workspace::render_tab_bar`]
    /// (select / close / reorder / rename / context menu), stacked in a column
    /// with a "New Tab" footer. The new-tab / context menus themselves are
    /// rendered by [`Workspace::render`], anchored at the click position.
    pub fn render_tab_list_vertical(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let (muted, fg, border) = {
            let theme = self.theme.read(cx);
            (theme.muted_foreground(), theme.foreground(), theme.border())
        };
        let spaces = self.spaces.read(cx).spaces().to_vec();
        let tabs = self.tabs.read(cx).tabs().to_vec();

        // Grouped into a collapsible section per Space (T20-008) — the same
        // data and row component the Spaces popover uses, just expanded
        // inline instead of in a floating card.
        let mut groups: Vec<gpui::AnyElement> = Vec::with_capacity(spaces.len() * 2);
        let last_space_idx = spaces.len().saturating_sub(1);
        for (idx, space) in spaces.iter().enumerate() {
            let space_tabs: Vec<&Tab> = tabs.iter().filter(|t| t.space_id == space.id).collect();
            groups.push(
                self.render_space_header(space, space_tabs.len(), false, cx)
                    .into_any_element(),
            );
            if !self.collapsed_spaces.contains(&space.id) {
                groups.extend(
                    space_tabs
                        .into_iter()
                        .map(|t| self.render_tab(t, true, cx).into_any_element()),
                );
            }
            if idx != last_space_idx {
                groups.push(divider(Axis::Horizontal, border).my_1().into_any_element());
            }
        }
        // Recent Projects lives in the "New Space" dropdown (`sidebar-new-space`)
        // here, not inline — the titlebar Spaces popover still shows it inline
        // since it has no equivalent creation dropdown of its own.

        div()
            .flex()
            .flex_col()
            .min_h_0()
            .h_full()
            .w_full()
            .child(
                div()
                    .id("sidebar-tab-list")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .overflow_y_scroll()
                    .p_1p5()
                    .children(groups),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(border)
                    .p_1p5()
                    .flex()
                    .justify_end()
                    .child(
                        div()
                            .id("sidebar-new-space")
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .text_color(muted)
                            .cursor_pointer()
                            .hover(|s| s.bg(border).text_color(fg))
                            .child(IconName::FolderAdd.svg(muted).size(px(15.0)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                                    this.new_space_menu = Some(ev.position);
                                    this.new_space_search.clear();
                                    window.focus(&this.new_space_search_focus);
                                    cx.notify();
                                }),
                            ),
                    ),
            )
            .into_any_element()
    }

    /// The "+" new-tab dropdown (port of `NewTabDropdownItems`): Terminal /
    /// Editor / Preview / Git Graph, then `SSH ▸` / `SFTP ▸` submenus listing
    /// the recent hosts (name + address) with an "All hosts…" entry each.
    pub(crate) fn render_new_tab_menu(
        &mut self,
        pos: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let recent = self.recent_host_picker_rows(cx, 5);
        let view = cx.entity();
        let open_sub = self.new_tab_submenu;

        // Hover wiring for a controlled `SSH ▸` / `SFTP ▸` submenu: entering
        // the trigger or its flyout opens that one (which closes the other,
        // since only one id is held); leaving the flyout closes it. Leaving
        // the trigger is deliberately ignored — the pointer is usually on its
        // way into the flyout.
        let submenu_cb = |which: NewTabSubmenu| {
            let v = view.clone();
            move |src: SubmenuHoverSource, hovered: bool, _w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    let changed = if hovered {
                        let was = this.new_tab_submenu;
                        this.new_tab_submenu = Some(which);
                        was != Some(which)
                    } else if src == SubmenuHoverSource::Flyout
                        && this.new_tab_submenu == Some(which)
                    {
                        this.new_tab_submenu = None;
                        true
                    } else {
                        false
                    };
                    if changed {
                        cx.notify();
                    }
                });
            }
        };

        // Recent-host rows for one protocol submenu (`ssh == false` → SFTP),
        // followed by a separator and the protocol's "All hosts…" entry.
        let host_items = |ssh: bool| -> Vec<MenuItem> {
            let proto = if ssh { "ssh" } else { "sftp" };
            let mut sub: Vec<MenuItem> = Vec::new();
            if recent.is_empty() {
                sub.push(MenuItem::new(format!("nt-{proto}-empty"), "No hosts yet").disabled(true));
            } else {
                for row in &recent {
                    let (id, name, subtitle) =
                        (row.id.clone(), row.name.clone(), row.subtitle.clone());
                    sub.push(
                        MenuItem::new(SharedString::from(format!("nt-{proto}-{id}")), name)
                            .detail(subtitle)
                            .on_click({
                                let v = view.clone();
                                move |_, w, cx| {
                                    let id = id.clone();
                                    v.update(cx, |this, cx| {
                                        this.new_tab_menu = None;
                                        if ssh {
                                            this.open_ssh_tab(id, w, cx)
                                        } else {
                                            this.open_sftp_tab(id, w, cx)
                                        }
                                    })
                                }
                            }),
                    );
                }
            }
            sub.push(MenuItem::separator());
            sub.push(
                MenuItem::new(format!("nt-{proto}-all"), "All hosts\u{2026}")
                    .icon(IconName::Server)
                    .on_click({
                        let v = view.clone();
                        move |_, _w, cx| {
                            v.update(cx, |this, cx| {
                                this.new_tab_menu = None;
                                this.open_hosts_tab(cx)
                            })
                        }
                    }),
            );
            sub
        };

        let items: Vec<MenuItem> = vec![
            MenuItem::new("nt-term", "Terminal")
                .icon(IconName::ToolTerminal)
                .keybind(["\u{2318}", "T"])
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.new_tab_menu = None;
                            this.new_terminal_tab(w, cx)
                        })
                    }
                }),
            MenuItem::new("nt-editor", "Editor")
                .icon(IconName::Pencil)
                .keybind(["\u{2318}", "E"])
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.new_tab_menu = None;
                            this.new_editor_tab(w, cx)
                        })
                    }
                }),
            MenuItem::new("nt-preview", "Preview")
                .icon(IconName::Globe)
                .keybind(["\u{2318}", "\u{21E7}", "P"])
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.new_tab_menu = None;
                            this.new_preview_tab(w, cx)
                        })
                    }
                }),
            MenuItem::new("nt-gitgraph", "Git Graph")
                .icon(IconName::GitBranch)
                .on_click({
                    let v = view.clone();
                    move |_, _w, cx| {
                        v.update(cx, |this, cx| {
                            this.new_tab_menu = None;
                            this.open_git_graph_tab(cx)
                        })
                    }
                }),
            MenuItem::separator(),
            MenuItem::submenu("nt-ssh", "SSH", host_items(true))
                .icon(IconName::Server)
                .submenu_control(
                    open_sub == Some(NewTabSubmenu::Ssh),
                    submenu_cb(NewTabSubmenu::Ssh),
                ),
            MenuItem::submenu("nt-sftp", "SFTP", host_items(false))
                .icon(IconName::Cloud)
                .submenu_control(
                    open_sub == Some(NewTabSubmenu::Sftp),
                    submenu_cb(NewTabSubmenu::Sftp),
                ),
        ];

        let dismiss = {
            let v = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.new_tab_menu = None;
                    cx.notify()
                })
            }
        };
        context_menu(
            pos,
            Palette::from_theme(self.theme.read(cx)),
            dismiss,
            items,
        )
    }

    pub(crate) fn render_context_menu(
        &mut self,
        id: u64,
        pos: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tab = self.tabs.read(cx).get(id).cloned();
        let kind = tab.as_ref().map(|t| t.kind);
        let is_peek = tab.as_ref().map(|t| t.peek).unwrap_or(false);
        let is_pinned = tab.as_ref().is_some_and(|tab| tab.pinned);
        let plural = kind.map(|k| k.plural_label().to_string());
        let grant_target = self.mcp_grant_target(id, cx);
        let is_granted = self.agent_access.read(cx).is_granted(id);
        let space_id = tab
            .as_ref()
            .map(|tab| tab.space_id)
            .unwrap_or_else(|| self.spaces.read(cx).active_id());
        let file_path = tab
            .as_ref()
            .filter(|tab| tab.kind == TabKind::Editor && tab.data.host_id.is_none())
            .and_then(|tab| tab.data.path.as_deref())
            .filter(|path| Path::new(path).is_absolute())
            .map(str::to_string);
        let editor_editability = self
            .editors
            .get(&id)
            .and_then(|editor| editor.read(cx).editability());
        let project_root = self
            .spaces
            .read(cx)
            .get(space_id)
            .and_then(|space| space.identity.project_root())
            .map(Path::to_path_buf);
        let relative_file_path = file_path
            .as_deref()
            .and_then(|path| relative_path_under_root(Path::new(path), project_root.as_deref()?));
        let space_tabs: Vec<Tab> = self
            .tabs
            .read(cx)
            .tabs_in_space(space_id)
            .into_iter()
            .cloned()
            .collect();
        let tab_index = space_tabs.iter().position(|tab| tab.id == id).unwrap_or(0);
        let multi = space_tabs.len() > 1;
        let can_close_left = tab_index > 0;
        let can_close_right = tab_index + 1 < space_tabs.len();
        let has_clean_tabs = space_tabs.iter().any(|tab| !tab.dirty);
        let view = cx.entity();

        let mut items: Vec<MenuItem> = Vec::new();
        if let (Some(TabKind::Editor), Some(editability)) = (kind, editor_editability) {
            let (label, checked, disabled) = match editability {
                labonair_editor::EditorEditability::Editable => {
                    ("Make Tab Read-Only", false, false)
                }
                labonair_editor::EditorEditability::TemporarilyReadOnly => {
                    ("Make Tab Editable", true, false)
                }
                labonair_editor::EditorEditability::ReadOnly => ("Read-Only", true, true),
            };
            items.push(
                MenuItem::new("toggle-tab-read-only", label)
                    .icon(IconName::Lock)
                    .checked(checked)
                    .disabled(disabled)
                    .on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            view.update(cx, |this, cx| {
                                this.context_menu = None;
                                if let Some(editor) =
                                    this.editors.get(&this.editor_slot_for_tab(id)).cloned()
                                {
                                    editor.update(cx, |editor, cx| {
                                        let _ = editor.dispatch_registered_command(
                                            CommandId::EditorToggleReadOnly,
                                            cx,
                                        );
                                    });
                                }
                                cx.notify();
                            });
                        }
                    }),
            );
        }
        if let Some(path) = file_path.as_deref() {
            items.push(
                MenuItem::new("copy-file-path", "Copy Path")
                    .icon(IconName::Copy)
                    .on_click({
                        let view = view.clone();
                        let path = path.to_string();
                        move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(path.clone()));
                            view.update(cx, |this, cx| {
                                this.context_menu = None;
                                cx.notify();
                            });
                        }
                    }),
            );
            if let Some(relative_path) = relative_file_path {
                items.push(
                    MenuItem::new("copy-relative-file-path", "Copy Relative Path").on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(relative_path.clone()));
                            view.update(cx, |this, cx| {
                                this.context_menu = None;
                                cx.notify();
                            });
                        }
                    }),
                );
            }
            items.push(
                MenuItem::new("reveal-file", "Reveal in File Manager")
                    .icon(IconName::FolderOpen)
                    .on_click({
                        let view = view.clone();
                        let path = path.to_string();
                        move |_, _, cx| {
                            cx.reveal_path(Path::new(&path));
                            view.update(cx, |this, cx| {
                                this.context_menu = None;
                                cx.notify();
                            });
                        }
                    }),
            );
            if let Some(directory) = Path::new(path).parent().map(|path| path.to_path_buf()) {
                items.push(
                    MenuItem::new("open-file-terminal", "Open Terminal in File Directory")
                        .icon(IconName::Terminal)
                        .on_click({
                            let view = view.clone();
                            let directory = directory.to_string_lossy().into_owned();
                            move |_, window, cx| {
                                view.update(cx, |this, cx| {
                                    this.context_menu = None;
                                    this.new_terminal_tab_in(Some(directory.clone()), window, cx);
                                });
                            }
                        }),
                );
            }
            items.push(
                MenuItem::new("open-file-history", "File History")
                    .icon(IconName::GitCompare)
                    .on_click({
                        let view = view.clone();
                        let path = path.to_string();
                        move |_, _, cx| {
                            view.update(cx, |this, cx| {
                                this.context_menu = None;
                                this.open_file_history_tab(path.clone(), cx);
                                cx.notify();
                            });
                        }
                    }),
            );
            items.push(MenuItem::separator());
        }
        if kind == Some(TabKind::Editor) && is_peek {
            items.push(
                MenuItem::new("keep", "Keep Tab Open")
                    .icon(IconName::Eye)
                    .on_click({
                        let v = view.clone();
                        move |_, _w, cx| {
                            v.update(cx, |this, cx| {
                                this.context_menu = None;
                                this.tabs.update(cx, |s, cx| s.set_peek(id, false, cx));
                            })
                        }
                    }),
            );
        }
        items.push(
            MenuItem::new(
                "toggle-pin-tab",
                if is_pinned { "Unpin Tab" } else { "Pin Tab" },
            )
            .icon(IconName::Pin)
            .on_click({
                let view = view.clone();
                move |_, _, cx| {
                    view.update(cx, |this, cx| {
                        this.context_menu = None;
                        this.tabs
                            .update(cx, |store, cx| store.set_pinned(id, !is_pinned, cx));
                    });
                }
            }),
        );
        items.push(
            MenuItem::new("rename", "Rename")
                .icon(IconName::Pencil)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| v.update(cx, |this, cx| this.begin_tab_rename(id, w, cx))
                }),
        );
        items.push(
            MenuItem::new("dup", "Duplicate")
                .icon(IconName::Copy)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.context_menu = None;
                            this.tabs.update(cx, |s, cx| s.set_active(id, cx));
                            this.duplicate_active_tab(w, cx);
                        })
                    }
                }),
        );
        items.push(MenuItem::separator());
        // T20-001: `MenuItem::keybind` renders the item's live binding as
        // `Kbd` chips, resolved through the user's `keymap.json` overrides.
        let close_keys = cx
            .try_global::<labonair_command_palette::KeybindDisplay>()
            .map(|g| g.keys_for(labonair_command_palette::CommandId::CloseTab, Some("cmd-w")))
            .unwrap_or_default();
        items.push(
            MenuItem::new("close", "Close")
                .keybind(close_keys)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.context_menu = None;
                            this.request_close(id, w, cx)
                        })
                    }
                }),
        );
        items.push(
            MenuItem::new("others", "Close Others")
                .disabled(!multi)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.context_menu = None;
                            this.close_others(id, w, cx)
                        })
                    }
                }),
        );
        items.push(
            MenuItem::new("left", "Close Left")
                .disabled(!can_close_left)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.context_menu = None;
                            this.close_tabs_to_side(id, false, w, cx)
                        })
                    }
                }),
        );
        items.push(
            MenuItem::new("right", "Close Right")
                .disabled(!can_close_right)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.context_menu = None;
                            this.close_tabs_to_side(id, true, w, cx)
                        })
                    }
                }),
        );
        items.push(
            MenuItem::new("clean", "Close Clean")
                .disabled(!has_clean_tabs)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.context_menu = None;
                            this.close_clean_tabs(space_id, w, cx)
                        })
                    }
                }),
        );
        items.push(MenuItem::new("all", "Close All").on_click({
            let v = view.clone();
            move |_, w, cx| {
                v.update(cx, |this, cx| {
                    this.context_menu = None;
                    this.close_all_tabs(space_id, w, cx)
                })
            }
        }));
        if multi {
            if let (Some(k), Some(pl)) = (kind, plural) {
                items.push(MenuItem::new("kind", format!("Close All {pl}")).on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.context_menu = None;
                            this.close_by_kind(space_id, k, w, cx)
                        })
                    }
                }));
            }
        }
        if let Some((session_id, label, gkind, host_id)) = grant_target {
            items.push(MenuItem::separator());
            items.push(
                MenuItem::new("mcp-grant", "Grant AI Agent Access")
                    .icon(IconName::Shield)
                    .checked(is_granted)
                    .on_click({
                        let v = view.clone();
                        move |_, _w, cx| {
                            let (session_id, label, host_id) =
                                (session_id.clone(), label.clone(), host_id.clone());
                            v.update(cx, |this, cx| {
                                this.context_menu = None;
                                this.agent_access.update(cx, |s, cx| {
                                    s.set_grant(
                                        id,
                                        session_id,
                                        !is_granted,
                                        label,
                                        gkind,
                                        host_id,
                                        cx,
                                    );
                                });
                                cx.notify();
                            })
                        }
                    }),
            );
        }

        let dismiss = {
            let v = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.context_menu = None;
                    cx.notify()
                })
            }
        };
        context_menu(
            pos,
            Palette::from_theme(self.theme.read(cx)),
            dismiss,
            items,
        )
    }
}
