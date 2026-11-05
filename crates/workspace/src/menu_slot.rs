//! Exit-fade plumbing for the workspace-owned floating surfaces (popovers and
//! context menus, R10-002). Each is a `MenuSlot` (ui-kit `PresenceSlot`: the
//! surface data plus its open/closing/closed state); the helpers here dismiss
//! them with a fade and tick them once per render.

use gpui::{App, Context, Window};
use labonair_ui_kit::{MotionPrefs, Palette, PresenceStep};

pub(crate) use labonair_ui_kit::PresenceSlot as MenuSlot;

use crate::Workspace;

impl Workspace {
    /// The active theme's motion tokens (exit durations, level).
    pub(crate) fn motion(&self, cx: &App) -> MotionPrefs {
        Palette::from_theme(self.theme.read(cx)).motion
    }

    /// Dismiss the Spaces popover (fade out), committing an inline space
    /// rename that was still open inside it so typed text is not lost.
    pub(crate) fn close_spaces_menu(&mut self, cx: &mut Context<Self>) {
        let motion = self.motion(cx);
        self.spaces_menu.close(&motion);
        self.commit_space_rename(cx);
        cx.notify();
    }

    /// Dismiss the per-space "..." actions menu.
    pub(crate) fn close_space_actions_menu(&mut self, cx: &mut Context<Self>) {
        let motion = self.motion(cx);
        self.space_actions_menu.close(&motion);
        cx.notify();
    }

    /// Dismiss the tab context menu.
    pub(crate) fn close_tab_menu(&mut self, cx: &mut Context<Self>) {
        let motion = self.motion(cx);
        self.context_menu.close(&motion);
        cx.notify();
    }

    /// Dismiss the "+" new-tab dropdown.
    pub(crate) fn close_new_tab_menu(&mut self, cx: &mut Context<Self>) {
        let motion = self.motion(cx);
        self.new_tab_menu.close(&motion);
        self.new_tab_submenu = None;
        cx.notify();
    }

    /// Dismiss the sidebar "New Space" dropdown and reset its search.
    pub(crate) fn close_new_space_menu(&mut self, cx: &mut Context<Self>) {
        let motion = self.motion(cx);
        self.new_space_menu.close(&motion);
        self.new_space_search.clear();
        cx.notify();
    }

    /// Whether any workspace-owned menu is still on screen (open or fading).
    fn any_menu_mounted(&self) -> bool {
        self.spaces_menu.mounted().is_some()
            || self.space_actions_menu.mounted().is_some()
            || self.context_menu.mounted().is_some()
            || self.new_tab_menu.mounted().is_some()
            || self.new_space_menu.mounted().is_some()
    }

    /// Advance every menu's exit fade; call once at the top of `render`.
    /// Focus returns to the active pane only when a surface just unmounted,
    /// and never while an inline rename owns the keyboard or another menu is
    /// still up.
    pub(crate) fn tick_menus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let motion = self.motion(cx);
        let steps = [
            self.spaces_menu.tick(&motion),
            self.space_actions_menu.tick(&motion),
            self.context_menu.tick(&motion),
            self.new_tab_menu.tick(&motion),
            self.new_space_menu.tick(&motion),
        ];
        let just_closed = steps.contains(&PresenceStep::JustClosed);
        if just_closed
            && self.rename_tab.is_none()
            && self.space_rename.is_none()
            && !self.any_menu_mounted()
        {
            self.focus_active(window, cx);
        }
    }
}
