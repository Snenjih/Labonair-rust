//! Spaces: named, switchable groups of tabs.
//!
//! A workspace previously had exactly one identity (Standalone or a single
//! Project root, see [`crate::context::WorkspaceIdentity`]) and one flat list
//! of tabs. A *Space* is a named group of tabs plus the identity they share;
//! exactly one Space is visible in the tab strip at a time, but the others'
//! tabs are not torn down — they simply aren't the ones [`crate::TabStore`]
//! is currently handing out to the tab-strip renderer. Terminal/SSH/editor
//! liveness already does not depend on tab visibility (see
//! `labonair_terminal::TerminalRegistry`'s own doc comment), so hiding a
//! Space's tabs costs nothing new.
//!
//! There is no such thing as an "ownerless" tab: every tab is created into
//! whichever Space is active at creation time (mirrors `TabStore`'s
//! `current_space`), and the seeded `DEFAULT_SPACE_ID` space — Standalone
//! identity, never closable — is today's implicit single workspace, just
//! given a name and a place in the model.

use gpui::{hsla, EventEmitter, Hsla};

use crate::context::WorkspaceIdentity;

pub type SpaceId = u64;

/// The always-present catch-all space. Never closable; every workspace has at
/// least this one.
pub const DEFAULT_SPACE_ID: SpaceId = 1;

/// A small fixed set of badge colors (mirrors the reference Spaces switcher's
/// colored initial badges) rather than an arbitrary theme-independent `Hsla`,
/// so the color survives session-snapshot round-trips as a plain enum instead
/// of float components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpaceColor {
    #[default]
    Orange,
    Blue,
    Green,
    Purple,
    Pink,
    Yellow,
    Gray,
}

impl SpaceColor {
    /// Fixed accent color for the badge — deliberately independent of the
    /// active theme (mirrors the reference switcher's own fixed palette).
    pub fn hsla(self) -> Hsla {
        match self {
            SpaceColor::Orange => hsla(24. / 360., 0.85, 0.53, 1.0),
            SpaceColor::Blue => hsla(217. / 360., 0.85, 0.60, 1.0),
            SpaceColor::Green => hsla(142. / 360., 0.60, 0.45, 1.0),
            SpaceColor::Purple => hsla(266. / 360., 0.65, 0.62, 1.0),
            SpaceColor::Pink => hsla(330. / 360., 0.70, 0.62, 1.0),
            SpaceColor::Yellow => hsla(45. / 360., 0.90, 0.55, 1.0),
            SpaceColor::Gray => hsla(220. / 360., 0.06, 0.55, 1.0),
        }
    }

    /// Cycles through the preset set, skipping `Orange` (reserved for
    /// [`DEFAULT_SPACE_ID`]) so newly created spaces read as visually
    /// distinct from Default and from each other for a while.
    fn nth(n: usize) -> Self {
        const CYCLE: [SpaceColor; 6] = [
            SpaceColor::Blue,
            SpaceColor::Purple,
            SpaceColor::Green,
            SpaceColor::Pink,
            SpaceColor::Yellow,
            SpaceColor::Gray,
        ];
        CYCLE[n % CYCLE.len()]
    }
}

/// One named group of tabs.
#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    pub id: SpaceId,
    pub name: String,
    pub color: SpaceColor,
    pub identity: WorkspaceIdentity,
    /// Which tab to reactivate when switching back into this space. `None`
    /// once the space has no tabs left.
    pub last_active_tab: Option<u64>,
}

/// Emitted whenever the active space changes.
pub struct ActiveSpaceChanged(pub SpaceId);

/// The single source of truth for the set of spaces and which one is active.
pub struct SpaceStore {
    spaces: Vec<Space>,
    active_id: SpaceId,
    next_id: SpaceId,
}

impl EventEmitter<ActiveSpaceChanged> for SpaceStore {}

impl Default for SpaceStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SpaceStore {
    /// Seeds the permanent `Default` space (Standalone identity) and makes it
    /// active — this is today's implicit single workspace, unchanged.
    pub fn new() -> Self {
        Self {
            spaces: vec![Space {
                id: DEFAULT_SPACE_ID,
                name: "Default".to_string(),
                color: SpaceColor::Orange,
                identity: WorkspaceIdentity::Standalone,
                last_active_tab: None,
            }],
            active_id: DEFAULT_SPACE_ID,
            next_id: DEFAULT_SPACE_ID + 1,
        }
    }

    pub fn spaces(&self) -> &[Space] {
        &self.spaces
    }

    pub fn active_id(&self) -> SpaceId {
        self.active_id
    }

    pub fn active(&self) -> Option<&Space> {
        self.get(self.active_id)
    }

    pub fn get(&self, id: SpaceId) -> Option<&Space> {
        self.spaces.iter().find(|s| s.id == id)
    }

    pub fn len(&self) -> usize {
        self.spaces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spaces.is_empty()
    }

    /// Create a new space and return its id. Does not switch to it — the
    /// caller (`Workspace::create_space_from_project` /
    /// `create_space_standalone`) does that as a separate, explicit step.
    pub fn create(
        &mut self,
        name: impl Into<String>,
        identity: WorkspaceIdentity,
        cx: &mut gpui::Context<Self>,
    ) -> SpaceId {
        let id = self.next_id;
        self.next_id += 1;
        let color = SpaceColor::nth(self.spaces.len().saturating_sub(1));
        self.spaces.push(Space {
            id,
            name: name.into(),
            color,
            identity,
            last_active_tab: None,
        });
        cx.notify();
        id
    }

    /// Re-create the seeded snapshot-restore shape used at startup: replaces
    /// the seeded Default space's name/color/identity in place (keeping
    /// `DEFAULT_SPACE_ID`) rather than creating a second space, or creates a
    /// fresh space for every subsequent snapshot entry. Returns the id the
    /// entry ended up at.
    pub fn restore_entry(
        &mut self,
        is_first: bool,
        name: String,
        color: SpaceColor,
        identity: WorkspaceIdentity,
        cx: &mut gpui::Context<Self>,
    ) -> SpaceId {
        if is_first {
            if let Some(default) = self.spaces.iter_mut().find(|s| s.id == DEFAULT_SPACE_ID) {
                default.name = name;
                default.color = color;
                default.identity = identity;
            }
            cx.notify();
            DEFAULT_SPACE_ID
        } else {
            let id = self.next_id;
            self.next_id += 1;
            self.spaces.push(Space {
                id,
                name,
                color,
                identity,
                last_active_tab: None,
            });
            cx.notify();
            id
        }
    }

    pub fn rename(&mut self, id: SpaceId, name: impl Into<String>, cx: &mut gpui::Context<Self>) {
        if let Some(space) = self.spaces.iter_mut().find(|s| s.id == id) {
            space.name = name.into();
            cx.notify();
        }
    }

    pub fn set_identity(
        &mut self,
        id: SpaceId,
        identity: WorkspaceIdentity,
        cx: &mut gpui::Context<Self>,
    ) {
        if let Some(space) = self.spaces.iter_mut().find(|s| s.id == id) {
            space.identity = identity;
            cx.notify();
        }
    }

    /// Make `id` the active space (no-op if unknown or already active).
    pub fn set_active(&mut self, id: SpaceId, cx: &mut gpui::Context<Self>) {
        if self.active_id == id || !self.spaces.iter().any(|s| s.id == id) {
            return;
        }
        self.active_id = id;
        cx.emit(ActiveSpaceChanged(id));
        cx.notify();
    }

    pub fn record_last_active_tab(&mut self, id: SpaceId, tab_id: Option<u64>) {
        if let Some(space) = self.spaces.iter_mut().find(|s| s.id == id) {
            space.last_active_tab = tab_id;
        }
    }

    /// Remove a space. Rejected for [`DEFAULT_SPACE_ID`] — it always exists.
    /// Returns the removed space so the caller can tear down its tabs.
    pub fn close(&mut self, id: SpaceId, cx: &mut gpui::Context<Self>) -> Option<Space> {
        if id == DEFAULT_SPACE_ID {
            return None;
        }
        let idx = self.spaces.iter().position(|s| s.id == id)?;
        let removed = self.spaces.remove(idx);
        cx.notify();
        Some(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn seeds_one_permanent_default_space(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| SpaceStore::new());
            store.update(cx, |s, _| {
                assert_eq!(s.len(), 1);
                assert_eq!(s.active_id(), DEFAULT_SPACE_ID);
                assert_eq!(s.active().unwrap().name, "Default");
                assert_eq!(s.active().unwrap().identity, WorkspaceIdentity::Standalone);
            });
        });
    }

    #[gpui::test]
    fn create_and_switch(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| SpaceStore::new());
            store.update(cx, |s, cx| {
                let id = s.create("Work", WorkspaceIdentity::project("/repo"), cx);
                assert_eq!(s.len(), 2);
                assert_eq!(s.active_id(), DEFAULT_SPACE_ID, "create does not switch");

                s.set_active(id, cx);
                assert_eq!(s.active_id(), id);
                assert_eq!(s.active().unwrap().name, "Work");
            });
        });
    }

    #[gpui::test]
    fn default_space_cannot_be_closed(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| SpaceStore::new());
            store.update(cx, |s, cx| {
                assert!(s.close(DEFAULT_SPACE_ID, cx).is_none());
                assert_eq!(s.len(), 1);
            });
        });
    }

    #[gpui::test]
    fn closing_a_space_removes_it(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| SpaceStore::new());
            store.update(cx, |s, cx| {
                let id = s.create("Scratch", WorkspaceIdentity::Standalone, cx);
                let removed = s.close(id, cx).unwrap();
                assert_eq!(removed.name, "Scratch");
                assert_eq!(s.len(), 1);
            });
        });
    }

    #[gpui::test]
    fn last_active_tab_round_trips(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| SpaceStore::new());
            store.update(cx, |s, _| {
                assert_eq!(s.get(DEFAULT_SPACE_ID).unwrap().last_active_tab, None);
                s.record_last_active_tab(DEFAULT_SPACE_ID, Some(42));
                assert_eq!(s.get(DEFAULT_SPACE_ID).unwrap().last_active_tab, Some(42));
            });
        });
    }

    #[gpui::test]
    fn rename_and_recolor(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| SpaceStore::new());
            store.update(cx, |s, cx| {
                s.rename(DEFAULT_SPACE_ID, "Renamed", cx);
                assert_eq!(s.get(DEFAULT_SPACE_ID).unwrap().name, "Renamed");

                s.set_identity(DEFAULT_SPACE_ID, WorkspaceIdentity::project("/x"), cx);
                assert_eq!(
                    s.get(DEFAULT_SPACE_ID).unwrap().identity,
                    WorkspaceIdentity::project("/x")
                );
            });
        });
    }

    #[gpui::test]
    fn restore_entry_reuses_default_id_for_first(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| SpaceStore::new());
            store.update(cx, |s, cx| {
                let first = s.restore_entry(
                    true,
                    "Restored".into(),
                    SpaceColor::Blue,
                    WorkspaceIdentity::project("/a"),
                    cx,
                );
                assert_eq!(first, DEFAULT_SPACE_ID);
                assert_eq!(s.len(), 1);

                let second = s.restore_entry(
                    false,
                    "Second".into(),
                    SpaceColor::Green,
                    WorkspaceIdentity::Standalone,
                    cx,
                );
                assert_ne!(second, DEFAULT_SPACE_ID);
                assert_eq!(s.len(), 2);
            });
        });
    }
}
