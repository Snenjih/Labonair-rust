//! Per-tab pane zoom (R10-002, ADR 0005).
//!
//! Zoom shows one pane of a tab as an overlay over the whole tab area while
//! the rest of the tree stays laid out underneath (so their terminal sizes do
//! not change). It is runtime state only: never persisted, ended by any split
//! or close in the tab, and dropped with the tab.

use std::collections::HashMap;

use crate::pane::PaneId;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ZoomState {
    by_tab: HashMap<u64, PaneId>,
}

impl ZoomState {
    /// The zoomed pane of `tab`, if any.
    pub fn zoomed(&self, tab: u64) -> Option<PaneId> {
        self.by_tab.get(&tab).copied()
    }

    /// Toggle zoom for `pane` in `tab`: zooming another pane moves the zoom,
    /// zooming the zoomed pane ends it. Returns whether `pane` is zoomed now.
    pub fn toggle(&mut self, tab: u64, pane: PaneId) -> bool {
        if self.by_tab.get(&tab) == Some(&pane) {
            self.by_tab.remove(&tab);
            false
        } else {
            self.by_tab.insert(tab, pane);
            true
        }
    }

    /// End the zoom of `tab` (split, close, drop). Returns whether one was active.
    pub fn clear(&mut self, tab: u64) -> bool {
        self.by_tab.remove(&tab).is_some()
    }

    /// Forget a closed pane: ends the zoom if `pane` was the zoomed one.
    pub fn pane_closed(&mut self, pane: PaneId) {
        self.by_tab.retain(|_, zoomed| *zoomed != pane);
    }

    /// Drop zoom entries whose pane is no longer in its tab's tree.
    pub fn retain_live(&mut self, live: impl Fn(u64, PaneId) -> bool) {
        self.by_tab.retain(|tab, pane| live(*tab, *pane));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_zooms_moves_and_ends() {
        let mut z = ZoomState::default();
        assert!(z.toggle(1, 10));
        assert_eq!(z.zoomed(1), Some(10));
        assert!(z.toggle(1, 11), "zooming another pane moves the zoom");
        assert_eq!(z.zoomed(1), Some(11));
        assert!(!z.toggle(1, 11));
        assert_eq!(z.zoomed(1), None);
    }

    #[test]
    fn closing_the_zoomed_pane_exits_zoom_but_not_other_tabs() {
        let mut z = ZoomState::default();
        z.toggle(1, 10);
        z.toggle(2, 20);
        z.pane_closed(10);
        assert_eq!(z.zoomed(1), None);
        assert_eq!(z.zoomed(2), Some(20));
        assert!(z.clear(2));
        assert!(!z.clear(2));
    }

    #[test]
    fn retain_live_drops_stale_entries() {
        let mut z = ZoomState::default();
        z.toggle(1, 10);
        z.retain_live(|_, pane| pane != 10);
        assert_eq!(z.zoomed(1), None);
    }
}
