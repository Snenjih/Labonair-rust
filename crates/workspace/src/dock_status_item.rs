//! Workspace-owned statusbar controls for panel docks.

use gpui::{
    div, AnyElement, App, AppContext, ClickEvent, Context, Entity, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use labonair_panel::{DockPosition, PanelIcon, StatusItem, StatusSide};
use labonair_theme::store::ThemeStore;
use labonair_ui_kit::{icon_toggle_button, IconName, Palette};

use crate::Workspace;

pub(crate) fn panel_toggle_icon(icon: PanelIcon) -> IconName {
    match icon {
        PanelIcon::Explorer => IconName::FolderTree,
        PanelIcon::SourceControl => IconName::GitBranch,
        PanelIcon::Snippets => IconName::Zap,
        PanelIcon::Ai => IconName::MessageSquare,
        PanelIcon::Tabs => IconName::Tab,
    }
}

fn panel_toggle_command(persistent_name: &str) -> Option<labonair_command_palette::CommandId> {
    match persistent_name {
        "explorer" => Some(labonair_command_palette::CommandId::ToggleSidebar),
        _ => None,
    }
}

fn panel_toggle_title(persistent_name: &str) -> &'static str {
    match persistent_name {
        "explorer" => "Explorer",
        "source-control" => "Source Control",
        "snippets" => "Snippets",
        "tabs" => "Tabs",
        _ => "Panel",
    }
}

fn dock_buttons_id(position: DockPosition) -> &'static str {
    match position {
        DockPosition::Left => "dock-buttons-left",
        DockPosition::Right => "dock-buttons-right",
        DockPosition::Bottom => "dock-buttons-bottom",
    }
}

/// Panel buttons grouped by the dock edge they control.
pub struct DockPanelButtons {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
    position: DockPosition,
    dock_menu: Option<(SharedString, Point<Pixels>)>,
    hidden: std::collections::HashSet<SharedString>,
}

impl DockPanelButtons {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        position: DockPosition,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe_global::<crate::status_placements::StatusBarLayoutTick>(|this, cx| {
            this.reload_hidden();
            cx.notify();
        })
        .detach();
        let mut this = Self {
            workspace,
            theme,
            position,
            dock_menu: None,
            hidden: Default::default(),
        };
        this.reload_hidden();
        this
    }

    fn reload_hidden(&mut self) {
        self.hidden = crate::status_placements::panel_toggle_visibility_load()
            .into_iter()
            .filter(|(_, value)| !value.as_bool().unwrap_or(true))
            .map(|(key, _)| SharedString::from(key))
            .collect();
    }

    fn open_dock_menu(
        &mut self,
        name: SharedString,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.dock_menu = Some((name, position));
        cx.notify();
    }

    fn render_dock_menu(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        use labonair_ui_kit::{context_menu, MenuItem};

        let (name, position) = self.dock_menu.clone()?;
        let current = self.position;
        let positions = {
            let workspace = self.workspace.read(cx);
            let mut positions = workspace.dock(current).move_destinations(name.as_ref(), cx);
            positions.push(current);
            positions.sort_by_key(|position| match position {
                DockPosition::Left => 0,
                DockPosition::Bottom => 1,
                DockPosition::Right => 2,
            });
            positions
        };
        let view = cx.entity();
        let close = {
            let view = view.clone();
            move |cx: &mut App| {
                view.update(cx, |this, cx| {
                    this.dock_menu = None;
                    cx.notify();
                })
            }
        };
        let label = |position: DockPosition| match position {
            DockPosition::Left => "Dock Left",
            DockPosition::Right => "Dock Right",
            DockPosition::Bottom => "Dock Bottom",
        };

        let mut items = Vec::new();
        for destination in positions {
            let is_current = destination == current;
            let move_name = name.clone();
            let workspace = self.workspace.clone();
            let close = close.clone();
            items.push(
                MenuItem::new(
                    SharedString::from(format!("dock-move-{}", label(destination))),
                    label(destination),
                )
                .checked(is_current)
                .on_click(move |_, _, cx| {
                    if !is_current {
                        let move_name = move_name.clone();
                        workspace.update(cx, |workspace, cx| {
                            if workspace.move_panel(move_name.as_ref(), destination, cx) {
                                workspace.persist_docks(cx);
                                cx.notify();
                            }
                        });
                    }
                    close(cx);
                }),
            );
        }
        items.push(MenuItem::separator());
        let hide_name = name;
        let workspace = self.workspace.clone();
        let close_hide = close.clone();
        items.push(
            MenuItem::new("dock-hide", "Hide Button").on_click(move |_, _, cx| {
                workspace.update(cx, |workspace, cx| {
                    workspace.set_panel_toggle_visible(hide_name.to_string(), false, cx);
                });
                close_hide(cx);
            }),
        );

        Some(context_menu(
            position,
            Palette::from_theme(self.theme.read(cx)),
            move |_window, cx| close(cx),
            items,
        ))
    }
}

impl Render for DockPanelButtons {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for DockPanelButtons {
    fn id(&self) -> &'static str {
        dock_buttons_id(self.position)
    }

    fn default_side(&self) -> StatusSide {
        match self.position {
            DockPosition::Left => StatusSide::Left,
            DockPosition::Right | DockPosition::Bottom => StatusSide::Right,
        }
    }

    fn order(&self) -> i32 {
        match self.position {
            DockPosition::Left | DockPosition::Right => -10,
            DockPosition::Bottom => -20,
        }
    }

    fn group(&self) -> u32 {
        match self.position {
            DockPosition::Left => 0,
            DockPosition::Bottom => 8,
            DockPosition::Right => 9,
        }
    }

    fn render_status(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let palette = Palette::from_theme(self.theme.read(cx));
        let keybind_display = cx
            .try_global::<labonair_command_palette::KeybindDisplay>()
            .cloned()
            .unwrap_or_default();
        let mut panels = {
            let workspace = self.workspace.read(cx);
            let dock = workspace.dock(self.position);
            let (open, active_name) = (dock.is_open(), dock.active_name());
            let icon_of = |name: &str| {
                workspace
                    .panel_registry()
                    .iter()
                    .find(|registration| registration.persistent_name == name)
                    .map(|registration| registration.icon)
                    .unwrap_or(PanelIcon::Explorer)
            };
            dock.panels()
                .iter()
                .map(|panel| panel.persistent_name())
                .filter(|name| !self.hidden.contains(*name))
                .map(|name| {
                    (
                        SharedString::from(name),
                        panel_toggle_icon(icon_of(name)),
                        open && active_name == Some(name),
                    )
                })
                .collect::<Vec<_>>()
        };
        if self.position == DockPosition::Right {
            panels.reverse();
        }
        let dock_menu = self.render_dock_menu(cx);

        div()
            .relative()
            .flex()
            .items_center()
            .gap_0p5()
            .children(panels.into_iter().map(|(name, icon, active)| {
                let click_name = name.clone();
                let menu_name = name.clone();
                let title = panel_toggle_title(name.as_ref());
                let keys = panel_toggle_command(name.as_ref())
                    .map(|id| keybind_display.keys_for(id, None))
                    .unwrap_or_default();
                let tooltip = if keys.is_empty() {
                    SharedString::from(title)
                } else {
                    SharedString::from(format!("{title} ({})", keys.join("")))
                };
                icon_toggle_button(
                    SharedString::from(format!("dock-btn-{name}")),
                    palette,
                    icon,
                    active,
                )
                .tab_index(0)
                .tooltip(move |window, cx| {
                    labonair_ui_kit::Tooltip::new(tooltip.clone()).build(window, cx)
                })
                .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                    this.workspace.update(cx, |workspace, cx| {
                        workspace.select_panel(click_name.as_ref(), cx);
                    });
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                        this.open_dock_menu(menu_name.clone(), event.position, cx);
                    }),
                )
            }))
            .children(dock_menu)
            .into_any_element()
    }
}

/// Build a dock-panel status-bar contribution.
pub fn registration(
    workspace: &Entity<Workspace>,
    theme: &Entity<ThemeStore>,
    position: DockPosition,
    cx: &mut App,
) -> labonair_panel::StatusItemRegistration {
    let item = cx.new(|cx| DockPanelButtons::new(workspace.clone(), theme.clone(), position, cx));
    let handle = item.clone();
    labonair_panel::StatusItemRegistration {
        id: item.read(cx).id(),
        default_side: item.read(cx).default_side(),
        order: item.read(cx).order(),
        group: item.read(cx).group(),
        build: std::sync::Arc::new(move |_window, _cx| {
            std::sync::Arc::new(handle.clone()) as labonair_panel::AnyStatusItemHandle
        }),
    }
}
