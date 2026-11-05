//! A workspace-owned floating surface (popover or context menu) together with
//! its exit state (R10-002).
//!
//! GPUI unmounts an element the frame its owner stops building it, so the
//! surface's data has to outlive the dismissal for the exit fade to play. A
//! [`MenuSlot`] keeps the data next to a ui-kit [`Presence`]: `open` shows it,
//! `close` starts the fade (idempotent, so Esc spam is safe), `tick` runs once
//! per render and drops the data after the fade, and `mounted` yields the data
//! while the surface should be built, with the `closing` flag for the
//! `*_with_exit` variants.

use std::time::Instant;

use gpui::{App, Context, Window};
use labonair_ui_kit::{MotionPrefs, Palette, Presence, PresenceStep};

use crate::Workspace;

/// Data of one floating surface plus its open/closing/closed state.
pub(crate) struct MenuSlot<T> {
    data: Option<T>,
    presence: Presence,
    /// A close that finished at once (motion off): reported by the next
    /// [`MenuSlot::tick`] as `JustClosed`, so focus return stays on one path.
    closed_at_once: bool,
}

impl<T> MenuSlot<T> {
    pub(crate) const fn new() -> Self {
        Self {
            data: None,
            presence: Presence::new(),
            closed_at_once: false,
        }
    }

    /// Show the surface with `data`; cancels a running exit fade.
    pub(crate) fn open(&mut self, data: T) {
        self.data = Some(data);
        self.presence.open();
    }

    /// Start closing. Idempotent: closing a closing or closed surface changes
    /// nothing.
    pub(crate) fn close(&mut self, motion: &MotionPrefs) {
        if self.presence.request_close(Instant::now(), motion) {
            self.data = None;
            self.closed_at_once = true;
        }
    }

    /// Open and not fading out: the state in which the surface takes input.
    pub(crate) fn is_open(&self) -> bool {
        self.presence.is_mounted() && !self.presence.is_closing()
    }

    /// Advance the exit fade. Call once per render before building; drops the
    /// data once the surface is gone. [`PresenceStep::JustClosed`] is the
    /// moment to return focus.
    pub(crate) fn tick(&mut self, motion: &MotionPrefs) -> PresenceStep {
        let step = self.presence.tick(Instant::now(), motion);
        if !self.presence.is_mounted() {
            self.data = None;
        }
        if std::mem::take(&mut self.closed_at_once) {
            return PresenceStep::JustClosed;
        }
        step
    }

    /// The data and the `closing` flag while the surface must be built (open or
    /// fading out); `None` otherwise.
    pub(crate) fn mounted(&self) -> Option<(&T, bool)> {
        if self.presence.is_mounted() {
            self.data
                .as_ref()
                .map(|data| (data, self.presence.is_closing()))
        } else {
            None
        }
    }
}

impl<T> Default for MenuSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use labonair_ui_kit::MotionLevel;

    fn motion(level: MotionLevel) -> MotionPrefs {
        MotionPrefs::new(level, &Default::default())
    }

    #[test]
    fn closing_keeps_the_data_until_the_fade_ends() {
        let m = motion(MotionLevel::Full);
        let mut slot = MenuSlot::new();
        assert!(slot.mounted().is_none());

        slot.open(7);
        assert_eq!(slot.mounted(), Some((&7, false)));
        assert!(slot.is_open());

        slot.close(&m);
        assert_eq!(slot.mounted(), Some((&7, true)));
        assert!(!slot.is_open());
        // Esc spam while closing neither restarts nor unmounts.
        slot.close(&m);
        assert_eq!(slot.mounted(), Some((&7, true)));
        assert_eq!(slot.tick(&m), PresenceStep::Closing);
    }

    #[test]
    fn reopening_during_the_fade_cancels_it() {
        let m = motion(MotionLevel::Full);
        let mut slot = MenuSlot::new();
        slot.open(1);
        slot.close(&m);
        slot.open(2);
        assert_eq!(slot.mounted(), Some((&2, false)));
    }

    #[test]
    fn motion_off_closes_at_once_and_reports_it_once() {
        let m = motion(MotionLevel::Off);
        let mut slot = MenuSlot::new();
        slot.open(1);
        slot.close(&m);
        assert!(slot.mounted().is_none());
        assert_eq!(slot.tick(&m), PresenceStep::JustClosed);
        // Reported once; closing a surface that is not open is not a focus event.
        slot.close(&m);
        assert_eq!(slot.tick(&m), PresenceStep::Hidden);
    }
}
