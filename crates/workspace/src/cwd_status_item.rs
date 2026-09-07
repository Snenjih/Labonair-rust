//! Workspace-owned current-working-directory breadcrumb status item.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, AnyElement, App, AppContext, ClickEvent, Context, Entity, InteractiveElement,
    IntoElement, ParentElement, Pixels, Point, Render, SharedString, StatefulInteractiveElement,
    Styled, Window,
};
use labonair_panel::{StatusItem, StatusItemRegistration, StatusMenuEntry, StatusSide};
use labonair_theme::store::ThemeStore;
use labonair_ui_kit::{context_menu, icon_toggle_button, IconName, MenuItem, Palette};

use crate::cwd_breadcrumb as bc;
use crate::Workspace;

/// Statusbar breadcrumb for the active workspace directory or file.
pub struct CwdStatusItem {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
    expanded: bool,
    subdir_menu: Option<(String, Point<Pixels>, Option<Vec<String>>)>,
}

impl CwdStatusItem {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            workspace,
            theme,
            expanded: false,
            subdir_menu: None,
        }
    }

    fn home_dir() -> Option<String> {
        dirs::home_dir().map(|path| path.to_string_lossy().into_owned())
    }

    fn open_subdir_menu(&mut self, dir: String, position: Point<Pixels>, cx: &mut Context<Self>) {
        self.subdir_menu = Some((dir.clone(), position, None));
        cx.notify();
        if self.workspace.read(cx).active_remote_target(cx).is_some() {
            self.subdir_menu = Some((dir, position, Some(Vec::new())));
            return;
        }
        let read_dir = dir.clone();
        cx.spawn(async move |view, cx| {
            let result =
                cx.background_executor()
                    .spawn(async move {
                        labonair_filesystem::tree::read_dir_page(&read_dir, 0, 200, false)
                    })
                    .await;
            let _ = view.update(cx, |this, cx| {
                let Some((current, _, entries)) = this.subdir_menu.as_mut() else {
                    return;
                };
                if *current != dir {
                    return;
                }
                let names = result
                    .map(|page| {
                        page.entries
                            .into_iter()
                            .filter(|entry| {
                                matches!(entry.kind, labonair_filesystem::tree::EntryKind::Dir)
                            })
                            .map(|entry| entry.name)
                            .collect()
                    })
                    .unwrap_or_default();
                *entries = Some(names);
                cx.notify();
            });
        })
        .detach();
    }

    fn render_breadcrumb(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let cwd = self.workspace.read(cx).active_cwd(cx);
        let file_path = self.workspace.read(cx).active_file_path(cx);
        let home = Self::home_dir();
        let (foreground, muted) = {
            let theme = self.theme.read(cx);
            (theme.foreground(), theme.muted_foreground())
        };
        let text_size = 11.0_f32;
        let (dir, leaf) = match &file_path {
            Some(path) => (bc::dirname(path), Some(bc::basename(path).to_string())),
            None => match &cwd {
                Some(path) => (path.clone(), None),
                None => {
                    return div()
                        .id("crumb-empty")
                        .text_size(px(text_size))
                        .text_color(muted.opacity(0.7))
                        .child("no directory")
                        .into_any_element();
                }
            },
        };
        let segments = bc::segments_from_cwd(&dir, home.as_deref());
        let last_index = segments.len().saturating_sub(1);
        let current_is_dropdown = leaf.is_none();
        let parent_count = if current_is_dropdown {
            last_index
        } else {
            segments.len()
        };
        let collapse = parent_count > 4 && !self.expanded;
        let mut row = div()
            .flex()
            .items_center()
            .gap_1()
            .min_w_0()
            .overflow_hidden();
        for (index, segment) in segments.iter().enumerate() {
            let is_current = current_is_dropdown && index == last_index;
            if collapse && index > 0 && index < parent_count - 1 {
                if index == 1 {
                    row = row
                        .child(
                            icon_toggle_button(
                                "crumb-collapse",
                                Palette::from_theme(self.theme.read(cx)),
                                IconName::Ellipsis,
                                false,
                            )
                            .on_click(cx.listener(
                                |this, _: &ClickEvent, _, cx| {
                                    this.expanded = true;
                                    cx.notify();
                                },
                            )),
                        )
                        .child(div().text_color(muted).text_size(px(text_size)).child("/"));
                }
                continue;
            }
            row = row.child(self.render_crumb_segment(
                segment.clone(),
                is_current,
                current_is_dropdown,
                text_size,
                cx,
            ));
            if index != last_index || leaf.is_some() {
                row = row.child(div().text_color(muted).text_size(px(text_size)).child("/"));
            }
        }
        if let Some(name) = leaf {
            row = row.child(
                div()
                    .text_size(px(text_size))
                    .text_color(foreground)
                    .child(SharedString::from(name)),
            );
        }
        row.into_any_element()
    }

    fn render_crumb_segment(
        &self,
        segment: bc::Segment,
        is_current: bool,
        current_is_dropdown: bool,
        text_size: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (foreground, muted, border) = {
            let theme = self.theme.read(cx);
            (theme.foreground(), theme.muted_foreground(), theme.border())
        };
        let label = if segment.is_home {
            "~".to_string()
        } else {
            segment.label.clone()
        };
        let show_chevron = is_current && current_is_dropdown;
        let clicked = segment.clone();
        div()
            .id(SharedString::from(format!("crumb-{}", segment.full_path)))
            .flex()
            .items_center()
            .gap_1()
            .px(px(6.0))
            .py(px(1.0))
            .rounded_full()
            .border_1()
            .border_color(border)
            .text_size(px(text_size))
            .text_color(if is_current { foreground } else { muted })
            .hover(|style| style.text_color(foreground))
            .when(segment.is_home, |element| {
                element.child(IconName::Home.svg(muted))
            })
            .child(SharedString::from(label))
            .when(show_chevron, |element| {
                element.child(IconName::ChevronDown.svg(muted))
            })
            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                if show_chevron {
                    this.open_subdir_menu(clicked.full_path.clone(), event.position(), cx);
                } else {
                    let path = clicked.full_path.clone();
                    this.workspace
                        .update(cx, |workspace, cx| workspace.send_cd(&path, cx));
                }
            }))
    }

    fn render_subdir_menu(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (dir, position, entries) = self.subdir_menu.clone()?;
        let view = cx.entity();
        let close = {
            let view = view.clone();
            move |cx: &mut App| {
                view.update(cx, |this, cx| {
                    this.subdir_menu = None;
                    cx.notify();
                })
            }
        };
        let items: Vec<MenuItem> = match &entries {
            None => vec![MenuItem::label("Loading…")],
            Some(list) if list.is_empty() => vec![MenuItem::label("No subfolders")],
            Some(list) => list
                .iter()
                .take(50)
                .map(|name| {
                    let full = if dir == "/" {
                        format!("/{name}")
                    } else {
                        format!("{dir}/{name}")
                    };
                    let view = view.clone();
                    MenuItem::new(SharedString::from(format!("subdir-{name}")), name.clone())
                        .on_click(move |_, _, cx| {
                            let full = full.clone();
                            view.update(cx, |this, cx| {
                                this.workspace
                                    .update(cx, |workspace, cx| workspace.send_cd(&full, cx));
                                this.subdir_menu = None;
                                cx.notify();
                            });
                        })
                })
                .collect(),
        };
        Some(context_menu(
            position,
            Palette::from_theme(self.theme.read(cx)),
            move |_window, cx| close(cx),
            items,
        ))
    }
}

impl Render for CwdStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for CwdStatusItem {
    fn id(&self) -> &'static str {
        "cwd"
    }

    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }

    fn order(&self) -> i32 {
        10
    }

    fn group(&self) -> u32 {
        0
    }

    fn on_active_tab_changed(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    fn status_menu_entries(&mut self, cx: &mut Context<Self>) -> Vec<StatusMenuEntry> {
        let Some(cwd) = self.workspace.read(cx).active_cwd(cx) else {
            return Vec::new();
        };
        let workspace = self.workspace.clone();
        let copy = cwd.clone();
        let current = cwd.clone();
        let new_terminal = cwd.clone();
        vec![
            StatusMenuEntry::action("cwd-copy-path", "Copy path", move |_, cx| {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(copy.clone()));
            }),
            StatusMenuEntry::action("cwd-open-terminal", "Open in current terminal", {
                let workspace = workspace.clone();
                move |_, cx| {
                    workspace.update(cx, |workspace, cx| workspace.send_cd(&current, cx));
                }
            }),
            StatusMenuEntry::action(
                "cwd-open-new-terminal",
                "Open in new terminal",
                move |window, cx| {
                    workspace.update(cx, |workspace, cx| {
                        workspace.cd_in_new_tab(new_terminal.clone(), window, cx);
                    });
                },
            ),
        ]
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .items_center()
            .min_w_0()
            .child(self.render_breadcrumb(cx))
            .children(self.render_subdir_menu(cx))
            .into_any_element()
    }
}

/// Build the Workspace CWD status-bar contribution.
pub fn registration(
    workspace: &Entity<Workspace>,
    theme: &Entity<ThemeStore>,
    cx: &mut App,
) -> StatusItemRegistration {
    let item = cx.new(|cx| CwdStatusItem::new(workspace.clone(), theme.clone(), cx));
    let handle = item.clone();
    StatusItemRegistration {
        id: item.read(cx).id(),
        default_side: item.read(cx).default_side(),
        order: item.read(cx).order(),
        group: item.read(cx).group(),
        build: std::sync::Arc::new(move |_window, _cx| {
            std::sync::Arc::new(handle.clone()) as labonair_panel::AnyStatusItemHandle
        }),
    }
}
