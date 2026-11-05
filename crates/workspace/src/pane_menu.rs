//! Pane entries of the tab context menu (R10-002).
//!
//! The entries act on the focused pane of the right-clicked tab and run the
//! same code as the `pane::*` commands (`SplitRight`, `SplitDown`,
//! `MoveToNewTab`, `CloseOthers`), so there is one implementation and one set
//! of command ids; their key hints come from the live keymap.

use gpui::{App, Entity};
use labonair_command_palette::KeybindDisplay;
use labonair_command_palette_core::CommandId;
use labonair_ui_kit::{IconName, MenuItem};

use crate::pane::SplitDirection;
use crate::Workspace;

impl Workspace {
    /// Split Right / Split Down / Move Pane to New Tab / Close Other Panes
    /// for tab `tab`. Splits are disabled for pane kinds that cannot split
    /// (with the reason as detail); the other two need a group.
    pub(crate) fn pane_menu_items(
        &self,
        tab: u64,
        view: &Entity<Workspace>,
        cx: &App,
    ) -> Vec<MenuItem> {
        let pane = self.focused_pane(tab);
        let kind = self.pane_kind(pane, cx);
        let count = self.tab_pane_ids(tab).len();
        let keys = |command: CommandId, fallback: &str| {
            cx.try_global::<KeybindDisplay>()
                .map(|g| g.keys_for(command, Some(fallback)))
                .unwrap_or_default()
        };
        let split_reason = format!("{} cannot be split", kind.label());

        let split = |id: &'static str,
                     label: &'static str,
                     icon: IconName,
                     dir: SplitDirection,
                     command: CommandId,
                     fallback: &str| {
            let mut item = MenuItem::new(id, label)
                .icon(icon)
                .keybind(keys(command, fallback))
                .disabled(!kind.can_split())
                .on_click({
                    let view = view.clone();
                    move |_, window, cx| {
                        view.update(cx, |this, cx| {
                            this.close_tab_menu(cx);
                            this.select_tab(tab, window, cx);
                            this.split_pane_dir(pane, dir, true, window, cx);
                        })
                    }
                });
            if !kind.can_split() {
                item = item.detail(split_reason.clone());
            }
            item
        };

        let lone = "Only one pane";
        let mut move_to_tab = MenuItem::new("pane-to-tab", "Move Pane to New Tab")
            .icon(IconName::ArrowUpRight)
            .disabled(count < 2)
            .on_click({
                let view = view.clone();
                move |_, window, cx| {
                    view.update(cx, |this, cx| {
                        this.close_tab_menu(cx);
                        this.detach_pane_to_new_tab(pane, window, cx);
                    })
                }
            });
        let mut close_others = MenuItem::new("pane-close-others", "Close Other Panes")
            .disabled(count < 2)
            .on_click({
                let view = view.clone();
                move |_, window, cx| {
                    view.update(cx, |this, cx| {
                        this.close_tab_menu(cx);
                        this.close_other_panes(pane, window, cx);
                    })
                }
            });
        if count < 2 {
            move_to_tab = move_to_tab.detail(lone);
            close_others = close_others.detail(lone);
        }

        vec![
            split(
                "pane-split-right",
                "Split Right",
                IconName::ChevronRight,
                SplitDirection::Right,
                CommandId::SplitRight,
                "cmd-\\",
            ),
            split(
                "pane-split-down",
                "Split Down",
                IconName::ChevronDown,
                SplitDirection::Down,
                CommandId::SplitDown,
                "cmd-shift-\\",
            ),
            move_to_tab,
            close_others,
        ]
    }
}
