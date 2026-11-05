//! Pane kinds and the pure `PaneId`-keyed bookkeeping (R10-002, ADR 0005).
//!
//! Every [`TabKind`](crate::tabs::TabKind) becomes a pane kind: a tab is a
//! pane group and each leaf of its tree is a pane of some [`PaneKind`]. This
//! module is deliberately free of GPUI entities. It only carries the static
//! per-kind facts (icon, label, singleton / split policy) and the
//! [`PaneMetaStore`], a plain map from [`PaneId`] to the display metadata the
//! pane bar and the tab label need. The live views are owned by the adapters
//! that wave 2 adds under `workspace/src/pane_items/`.

use std::collections::BTreeMap;

use labonair_ui_kit::IconName;

use crate::pane::PaneId;
use crate::tabs::TabKind;

/// What a pane hosts. Mirrors [`TabKind`] one to one (`Workspace` is called
/// `Terminal` here because that is what the pane shows).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PaneKind {
    Terminal,
    Editor,
    Preview,
    Sftp,
    GitGraph,
    Diff,
    CommitDiff,
    AiDiff,
    Keymap,
    Hosts,
}

impl PaneKind {
    /// Every kind, in a stable order (tests, menus).
    pub const ALL: [PaneKind; 10] = [
        PaneKind::Terminal,
        PaneKind::Editor,
        PaneKind::Preview,
        PaneKind::Sftp,
        PaneKind::GitGraph,
        PaneKind::Diff,
        PaneKind::CommitDiff,
        PaneKind::AiDiff,
        PaneKind::Keymap,
        PaneKind::Hosts,
    ];

    /// The tab kind this pane kind corresponds to.
    pub fn tab_kind(self) -> TabKind {
        match self {
            PaneKind::Terminal => TabKind::Workspace,
            PaneKind::Editor => TabKind::Editor,
            PaneKind::Preview => TabKind::Preview,
            PaneKind::Sftp => TabKind::Sftp,
            PaneKind::GitGraph => TabKind::GitGraph,
            PaneKind::Diff => TabKind::Diff,
            PaneKind::CommitDiff => TabKind::CommitDiff,
            PaneKind::AiDiff => TabKind::AiDiff,
            PaneKind::Keymap => TabKind::Keymap,
            PaneKind::Hosts => TabKind::Hosts,
        }
    }

    /// The kind's icon. Identical to the tab-bar indicator so a tab turned
    /// into a pane (and back) never changes its glyph.
    pub fn icon(self) -> IconName {
        self.tab_kind().indicator()
    }

    /// Default human label (`"Terminal"`, `"Git Graph"`, ...), used until the
    /// adapter reports a more specific title.
    pub fn label(self) -> &'static str {
        self.tab_kind().default_title()
    }

    /// Singleton kinds exist at most once per workspace: opening one again
    /// focuses the existing pane instead of creating a duplicate. They can
    /// still be moved between groups and tabs.
    pub fn is_singleton(self) -> bool {
        matches!(
            self,
            PaneKind::GitGraph
                | PaneKind::Diff
                | PaneKind::CommitDiff
                | PaneKind::Keymap
                | PaneKind::Hosts
        )
    }

    /// Whether `pane::SplitRight/Down` may clone this pane. Singletons cannot
    /// (the button is disabled with a tooltip); an AI diff is bound to one
    /// pending proposal, so cloning it would make no sense either.
    pub fn can_split(self) -> bool {
        !self.is_singleton() && self != PaneKind::AiDiff
    }
}

impl From<TabKind> for PaneKind {
    fn from(kind: TabKind) -> Self {
        match kind {
            TabKind::Workspace => PaneKind::Terminal,
            TabKind::Editor => PaneKind::Editor,
            TabKind::Preview => PaneKind::Preview,
            TabKind::Sftp => PaneKind::Sftp,
            TabKind::GitGraph => PaneKind::GitGraph,
            TabKind::Diff => PaneKind::Diff,
            TabKind::CommitDiff => PaneKind::CommitDiff,
            TabKind::AiDiff => PaneKind::AiDiff,
            TabKind::Keymap => PaneKind::Keymap,
            TabKind::Hosts => PaneKind::Hosts,
        }
    }
}

/// Display metadata of one pane, as reported by its adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneMeta {
    pub kind: PaneKind,
    /// Adapter-supplied title; empty means "use the kind's default label".
    pub title: String,
    pub icon: IconName,
    /// Unsaved changes (editor panes); drives the per-pane close confirmation.
    pub dirty: bool,
}

impl PaneMeta {
    /// Metadata with the kind's default icon, an empty title and `dirty == false`.
    pub fn new(kind: PaneKind) -> Self {
        Self {
            kind,
            title: String::new(),
            icon: kind.icon(),
            dirty: false,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }

    /// The title to render: the adapter title, else the kind label.
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            self.kind.label()
        } else {
            &self.title
        }
    }
}

/// `PaneId` -> [`PaneMeta`]. The pure half of the single store that replaces
/// the per-tab-id maps in `Workspace` (the entity half stays GPUI-side).
/// Ordered by id so iteration is deterministic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaneMetaStore {
    panes: BTreeMap<PaneId, PaneMeta>,
}

impl PaneMetaStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.panes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.panes.is_empty()
    }

    pub fn contains(&self, id: PaneId) -> bool {
        self.panes.contains_key(&id)
    }

    pub fn get(&self, id: PaneId) -> Option<&PaneMeta> {
        self.panes.get(&id)
    }

    pub fn kind(&self, id: PaneId) -> Option<PaneKind> {
        self.panes.get(&id).map(|m| m.kind)
    }

    /// Register (or replace) a pane's metadata.
    pub fn insert(&mut self, id: PaneId, meta: PaneMeta) -> Option<PaneMeta> {
        self.panes.insert(id, meta)
    }

    pub fn remove(&mut self, id: PaneId) -> Option<PaneMeta> {
        self.panes.remove(&id)
    }

    /// Returns `false` when `id` is unknown.
    pub fn set_title(&mut self, id: PaneId, title: impl Into<String>) -> bool {
        self.panes
            .get_mut(&id)
            .map(|m| m.title = title.into())
            .is_some()
    }

    /// Returns `false` when `id` is unknown.
    pub fn set_dirty(&mut self, id: PaneId, dirty: bool) -> bool {
        self.panes.get_mut(&id).map(|m| m.dirty = dirty).is_some()
    }

    /// Returns `false` when `id` is unknown.
    pub fn set_icon(&mut self, id: PaneId, icon: IconName) -> bool {
        self.panes.get_mut(&id).map(|m| m.icon = icon).is_some()
    }

    /// All panes of `kind`, ascending by id.
    pub fn ids_of_kind(&self, kind: PaneKind) -> Vec<PaneId> {
        self.panes
            .iter()
            .filter(|(_, m)| m.kind == kind)
            .map(|(id, _)| *id)
            .collect()
    }

    /// The existing pane of a singleton `kind`, if any. Always `None` for
    /// non-singleton kinds. "Open again" focuses this pane.
    pub fn find_singleton(&self, kind: PaneKind) -> Option<PaneId> {
        if !kind.is_singleton() {
            return None;
        }
        self.panes
            .iter()
            .find(|(_, m)| m.kind == kind)
            .map(|(id, _)| *id)
    }

    /// Panes with unsaved changes, ascending by id.
    pub fn dirty_panes(&self) -> Vec<PaneId> {
        self.panes
            .iter()
            .filter(|(_, m)| m.dirty)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Drop every entry whose id is not in `live` (after a tree changed).
    /// Returns the removed ids.
    pub fn retain_live(&mut self, live: &[PaneId]) -> Vec<PaneId> {
        let gone: Vec<PaneId> = self
            .panes
            .keys()
            .copied()
            .filter(|id| !live.contains(id))
            .collect();
        for id in &gone {
            self.panes.remove(id);
        }
        gone
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_kind_round_trips_for_every_kind() {
        for kind in PaneKind::ALL {
            assert_eq!(PaneKind::from(kind.tab_kind()), kind);
            assert_eq!(kind.icon(), kind.tab_kind().indicator());
            assert_eq!(kind.label(), kind.tab_kind().default_title());
        }
    }

    #[test]
    fn singleton_and_split_policy() {
        for k in [
            PaneKind::GitGraph,
            PaneKind::Diff,
            PaneKind::CommitDiff,
            PaneKind::Keymap,
            PaneKind::Hosts,
        ] {
            assert!(k.is_singleton(), "{k:?}");
            assert!(!k.can_split(), "{k:?}");
        }
        for k in [
            PaneKind::Terminal,
            PaneKind::Editor,
            PaneKind::Preview,
            PaneKind::Sftp,
        ] {
            assert!(!k.is_singleton(), "{k:?}");
            assert!(k.can_split(), "{k:?}");
        }
        assert!(!PaneKind::AiDiff.is_singleton());
        assert!(!PaneKind::AiDiff.can_split());
    }

    #[test]
    fn display_title_falls_back_to_kind_label() {
        let m = PaneMeta::new(PaneKind::Terminal);
        assert_eq!(m.display_title(), "Terminal");
        assert_eq!(m.clone().with_title("build").display_title(), "build");
        assert!(!m.dirty);
        assert!(m.with_dirty(true).dirty);
    }

    #[test]
    fn store_tracks_meta_by_pane_id() {
        let mut s = PaneMetaStore::new();
        assert!(s.is_empty());
        s.insert(3, PaneMeta::new(PaneKind::Editor).with_title("a.rs"));
        s.insert(1, PaneMeta::new(PaneKind::Terminal));
        s.insert(2, PaneMeta::new(PaneKind::Editor).with_title("b.rs"));
        s.insert(9, PaneMeta::new(PaneKind::Keymap));
        assert_eq!(s.len(), 4);
        assert_eq!(s.ids_of_kind(PaneKind::Editor), vec![2, 3]);
        assert_eq!(s.kind(9), Some(PaneKind::Keymap));

        assert!(s.set_dirty(3, true));
        assert!(!s.set_dirty(77, true));
        assert_eq!(s.dirty_panes(), vec![3]);
        assert!(s.set_title(1, "zsh"));
        assert_eq!(s.get(1).map(PaneMeta::display_title), Some("zsh"));

        assert_eq!(s.find_singleton(PaneKind::Keymap), Some(9));
        assert_eq!(s.find_singleton(PaneKind::Hosts), None);
        // Non-singleton kinds never report a "existing" pane.
        assert_eq!(s.find_singleton(PaneKind::Editor), None);

        assert_eq!(s.retain_live(&[1, 3]), vec![2, 9]);
        assert_eq!(s.len(), 2);
        assert!(s.remove(1).is_some());
        assert!(s.remove(1).is_none());
    }
}
