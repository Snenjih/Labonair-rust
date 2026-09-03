//! Tab data model and tab store (T04-001).
//!
//! Labonair's tab system is the central navigation element. A *tab* is a light
//! descriptor with a [`TabKind`] discriminant (`workspace`, `editor`,
//! `preview`, `home`, `sftp`, `git-graph`, `git-diff`, `commit-diff`,
//! `ai-diff`) plus kind-specific data in [`TabData`]. The reference keeps this
//! in the `useTabs` Zustand store; here it is the GPUI [`TabStore`] entity.
//!
//! "Tab" and "session" are deliberately **not** the same thing (mirrors the
//! React version): the visible workspace tab is backed by a local PTY session
//! that lives independently in the [`TerminalRegistry`](labonair_terminal::TerminalRegistry)
//! and keeps running regardless of which tab is on screen. The tab only holds
//! that session's [`SessionId`].
//!
//! Splitting a workspace tab into multiple panes is T04-002; for now a
//! workspace tab owns exactly one session.

use gpui::{Context, EventEmitter};

use labonair_terminal::SessionId;
use labonair_ui_kit::IconName;

/// The category of a tab. Content views for most kinds arrive in later phases;
/// the model already covers them so the tab bar and store are stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TabKind {
    /// The landing / dashboard tab. Never closable, always present.
    Home,
    /// A local (or, later, SSH) terminal workspace.
    Workspace,
    /// A code editor tab (Phase 05).
    Editor,
    /// A markdown / web preview tab (Phase — native markdown replacement).
    Preview,
    /// An AI-proposed diff awaiting approval (Phase 10).
    AiDiff,
    /// An SFTP file browser (Phase 07).
    Sftp,
    /// A commit-graph view (Phase 09).
    GitGraph,
    /// A single-file working-tree diff (Phase 08).
    GitDiff,
    /// A committed-change diff (Phase 09).
    CommitDiff,
}

impl TabKind {
    /// The icon shown in the tab bar before the title (mirrors the reference
    /// `tabUtils.tsx` per-kind Hugeicons).
    pub fn indicator(&self) -> IconName {
        match self {
            TabKind::Home => IconName::Home,
            TabKind::Workspace => IconName::Terminal,
            TabKind::Editor => IconName::SquarePen,
            TabKind::Preview => IconName::Globe,
            TabKind::AiDiff => IconName::Sparkles,
            TabKind::Sftp => IconName::FolderOpen,
            TabKind::GitGraph => IconName::GitCompare,
            TabKind::GitDiff => IconName::GitBranch,
            TabKind::CommitDiff => IconName::GitBranch,
        }
    }

    /// Plural label for "Close All {kind}" menu entries (port of
    /// `tabUtils.tsx` `pluralLabelFor`).
    pub fn plural_label(&self) -> &'static str {
        match self {
            TabKind::Home => "Home Tabs",
            TabKind::Workspace => "Terminals",
            TabKind::Editor => "Editors",
            TabKind::Preview => "Previews",
            TabKind::AiDiff => "AI Diffs",
            TabKind::Sftp => "SFTP Tabs",
            TabKind::GitGraph => "Git Graphs",
            TabKind::GitDiff => "Git Diffs",
            TabKind::CommitDiff => "Commit Diffs",
        }
    }

    /// The default title used before anything more specific is known.
    pub fn default_title(&self) -> &'static str {
        match self {
            TabKind::Home => "Home",
            TabKind::Workspace => "Terminal",
            TabKind::Editor => "Untitled",
            TabKind::Preview => "Preview",
            TabKind::AiDiff => "AI Diff",
            TabKind::Sftp => "SFTP",
            TabKind::GitGraph => "Git Graph",
            TabKind::GitDiff => "Diff",
            TabKind::CommitDiff => "Commit",
        }
    }
}

/// Kind-specific tab payload. A flat optional bag rather than a per-kind enum so
/// later phases can add fields without a churny match everywhere; each field
/// documents which kinds populate it.
#[derive(Debug, Clone, Default)]
pub struct TabData {
    /// Registry session id backing a `Workspace` tab.
    pub session_id: Option<SessionId>,
    /// Live cwd from OSC 7 shell integration (`Workspace`).
    pub cwd: Option<String>,
    /// Live process title from OSC 0/1/2 (`Workspace`).
    pub process_title: Option<String>,
    /// File path (`Editor`, `GitDiff`).
    pub path: Option<String>,
    /// Remote host id (`Sftp`, remote `Editor`).
    pub host_id: Option<String>,
    /// Repository path (`GitGraph`, `GitDiff`, `CommitDiff`).
    pub repo_path: Option<String>,
    /// Target URL (`Preview`).
    pub url: Option<String>,
}

/// A single tab.
#[derive(Debug, Clone)]
pub struct Tab {
    /// Process-unique id, never reused.
    pub id: u64,
    pub kind: TabKind,
    /// Fallback title. Real label comes from [`Tab::label`].
    pub title: String,
    /// User-set title (via rename) — frozen, wins over everything.
    pub custom_title: Option<String>,
    /// Unsaved-changes indicator (`Editor`).
    pub dirty: bool,
    /// "Peek" preview tab, rendered italic (`Editor`).
    pub peek: bool,
    pub data: TabData,
}

impl Tab {
    /// The label to render, mirroring the reference `labelFor`: custom title →
    /// process title → cwd basename (workspace) → fallback title.
    pub fn label(&self) -> String {
        if let Some(custom) = &self.custom_title {
            return custom.clone();
        }
        if self.kind == TabKind::Workspace {
            if let Some(pt) = self.data.process_title.as_deref().filter(|s| !s.is_empty()) {
                return pt.to_string();
            }
            if let Some(cwd) = &self.data.cwd {
                return cwd
                    .rsplit('/')
                    .find(|s| !s.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| "/".to_string());
            }
        }
        self.title.clone()
    }

    /// Whether this tab has an "are you sure?" close (unsaved editor).
    pub fn needs_close_confirm(&self) -> bool {
        self.kind == TabKind::Editor && self.dirty
    }
}

/// Emitted whenever the active tab changes, so the workspace can move focus.
pub struct ActiveTabChanged(pub u64);

/// The single source of truth for the open tabs and which one is active.
pub struct TabStore {
    tabs: Vec<Tab>,
    active_id: u64,
    next_id: u64,
}

impl EventEmitter<ActiveTabChanged> for TabStore {}

impl Default for TabStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TabStore {
    /// An empty store. The workspace immediately opens the first tab.
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_id: 0,
            next_id: 1,
        }
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active_id(&self) -> u64 {
        self.active_id
    }

    pub fn active(&self) -> Option<&Tab> {
        self.get(self.active_id)
    }

    pub fn get(&self, id: u64) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// All tabs of a given kind, in tab order.
    pub fn tabs_by_kind(&self, kind: TabKind) -> Vec<&Tab> {
        self.tabs.iter().filter(|t| t.kind == kind).collect()
    }

    /// Open a new tab and activate it. Returns its id.
    pub fn open(&mut self, kind: TabKind, data: TabData, cx: &mut Context<Self>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab {
            id,
            kind,
            title: kind.default_title().to_string(),
            custom_title: None,
            dirty: false,
            peek: false,
            data,
        });
        self.set_active(id, cx);
        cx.notify();
        id
    }

    /// Open a workspace tab backed by `session_id`, inheriting `cwd`.
    pub fn open_workspace(
        &mut self,
        session_id: SessionId,
        cwd: Option<String>,
        cx: &mut Context<Self>,
    ) -> u64 {
        self.open(
            TabKind::Workspace,
            TabData {
                session_id: Some(session_id),
                cwd,
                ..TabData::default()
            },
            cx,
        )
    }

    /// Make `id` the active tab (no-op if unknown or already active).
    pub fn set_active(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.active_id == id || !self.tabs.iter().any(|t| t.id == id) {
            return;
        }
        self.active_id = id;
        cx.emit(ActiveTabChanged(id));
        cx.notify();
    }

    /// Activate the next / previous tab, wrapping around.
    pub fn cycle(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(pos) = self.tabs.iter().position(|t| t.id == self.active_id) else {
            return;
        };
        let n = self.tabs.len();
        if n <= 1 {
            return;
        }
        let next = if forward {
            (pos + 1) % n
        } else {
            (pos + n - 1) % n
        };
        let id = self.tabs[next].id;
        self.set_active(id, cx);
    }

    /// Close a tab. `Home` and the last remaining tab can't be closed. If the
    /// active tab is closed, its left neighbour (or index 0) becomes active.
    /// Returns the removed tab so the caller can tear down its session.
    pub fn close(&mut self, id: u64, cx: &mut Context<Self>) -> Option<Tab> {
        if self.tabs.len() <= 1 {
            return None;
        }
        let idx = self.tabs.iter().position(|t| t.id == id)?;
        if self.tabs[idx].kind == TabKind::Home {
            return None;
        }
        let removed = self.tabs.remove(idx);
        if self.active_id == id {
            let neighbour = self.tabs[idx.saturating_sub(1).min(self.tabs.len() - 1)].id;
            self.active_id = neighbour;
            cx.emit(ActiveTabChanged(neighbour));
        }
        cx.notify();
        Some(removed)
    }

    /// Close every tab except `keep` (and any `Home` tab). Returns removed tabs.
    pub fn close_others(&mut self, keep: u64, cx: &mut Context<Self>) -> Vec<Tab> {
        let (kept, removed): (Vec<Tab>, Vec<Tab>) = std::mem::take(&mut self.tabs)
            .into_iter()
            .partition(|t| t.id == keep || t.kind == TabKind::Home);
        self.tabs = kept;
        if !self.tabs.iter().any(|t| t.id == self.active_id) {
            self.activate_fallback(cx);
        }
        cx.notify();
        removed
    }

    /// Close every tab of a given kind (except the last tab overall). Returns
    /// removed tabs.
    pub fn close_by_kind(&mut self, kind: TabKind, cx: &mut Context<Self>) -> Vec<Tab> {
        if kind == TabKind::Home {
            return Vec::new();
        }
        let (removed, kept): (Vec<Tab>, Vec<Tab>) = std::mem::take(&mut self.tabs)
            .into_iter()
            .partition(|t| t.kind == kind);
        self.tabs = kept;
        let mut removed = removed;
        if self.tabs.is_empty() {
            // Never leave zero tabs — put the first one back.
            if let Some(first) = removed.first().cloned() {
                self.active_id = first.id;
                self.tabs.push(first);
                removed.remove(0);
            }
        } else if !self.tabs.iter().any(|t| t.id == self.active_id) {
            self.activate_fallback(cx);
        }
        cx.notify();
        removed
    }

    fn activate_fallback(&mut self, cx: &mut Context<Self>) {
        if let Some(first) = self.tabs.first() {
            self.active_id = first.id;
            cx.emit(ActiveTabChanged(first.id));
        }
    }

    /// Move the tab `dragged` to the position currently held by `target`.
    pub fn reorder(&mut self, dragged: u64, target: u64, cx: &mut Context<Self>) {
        let (Some(from), Some(to)) = (
            self.tabs.iter().position(|t| t.id == dragged),
            self.tabs.iter().position(|t| t.id == target),
        ) else {
            return;
        };
        if from == to {
            return;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        cx.notify();
    }

    // ── Field mutators (all notify on change) ────────────────────────────────

    fn with_tab(&mut self, id: u64, cx: &mut Context<Self>, f: impl FnOnce(&mut Tab) -> bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            if f(tab) {
                cx.notify();
            }
        }
    }

    pub fn set_title(&mut self, id: u64, title: impl Into<String>, cx: &mut Context<Self>) {
        let title = title.into();
        self.with_tab(id, cx, |t| {
            let changed = t.title != title;
            t.title = title;
            changed
        });
    }

    /// Set (or clear) the user rename. A set value freezes the label.
    pub fn set_custom_title(&mut self, id: u64, title: Option<String>, cx: &mut Context<Self>) {
        self.with_tab(id, cx, |t| {
            let changed = t.custom_title != title;
            t.custom_title = title;
            changed
        });
    }

    pub fn set_dirty(&mut self, id: u64, dirty: bool, cx: &mut Context<Self>) {
        self.with_tab(id, cx, |t| {
            let changed = t.dirty != dirty;
            t.dirty = dirty;
            changed
        });
    }

    pub fn set_peek(&mut self, id: u64, peek: bool, cx: &mut Context<Self>) {
        self.with_tab(id, cx, |t| {
            let changed = t.peek != peek;
            t.peek = peek;
            changed
        });
    }

    pub fn set_path(&mut self, id: u64, path: Option<String>, cx: &mut Context<Self>) {
        self.with_tab(id, cx, |t| {
            let changed = t.data.path != path;
            t.data.path = path;
            changed
        });
    }

    /// Update the live cwd/process-title of a workspace tab from its terminal.
    pub fn sync_workspace_meta(
        &mut self,
        id: u64,
        cwd: Option<String>,
        process_title: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.with_tab(id, cx, |t| {
            if t.kind != TabKind::Workspace {
                return false;
            }
            let changed = t.data.cwd != cwd || t.data.process_title != process_title;
            t.data.cwd = cwd;
            t.data.process_title = process_title;
            changed
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext};

    fn ws(sid: SessionId) -> TabData {
        TabData {
            session_id: Some(sid),
            ..TabData::default()
        }
    }

    #[test]
    fn plural_labels_match_reference() {
        assert_eq!(TabKind::Workspace.plural_label(), "Terminals");
        assert_eq!(TabKind::Editor.plural_label(), "Editors");
        assert_eq!(TabKind::Preview.plural_label(), "Previews");
        assert_eq!(TabKind::Sftp.plural_label(), "SFTP Tabs");
        assert_eq!(TabKind::GitGraph.plural_label(), "Git Graphs");
    }

    #[gpui::test]
    fn add_switch_and_close(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| TabStore::new());
            store.update(cx, |s, cx| {
                let a = s.open(TabKind::Workspace, ws(1), cx);
                let b = s.open(TabKind::Workspace, ws(2), cx);
                let c = s.open(TabKind::Editor, TabData::default(), cx);
                assert_eq!(s.len(), 3);
                assert_eq!(s.active_id(), c);

                s.set_active(a, cx);
                assert_eq!(s.active_id(), a);

                // Closing a non-active tab keeps the active one.
                let removed = s.close(b, cx).unwrap();
                assert_eq!(removed.data.session_id, Some(2));
                assert_eq!(s.active_id(), a);
                assert_eq!(s.len(), 2);
            });
        });
    }

    #[gpui::test]
    fn closing_active_activates_left_neighbour(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| TabStore::new());
            store.update(cx, |s, cx| {
                let a = s.open(TabKind::Workspace, ws(1), cx);
                let b = s.open(TabKind::Workspace, ws(2), cx);
                let c = s.open(TabKind::Workspace, ws(3), cx);
                s.set_active(b, cx);
                s.close(b, cx);
                assert_eq!(s.active_id(), a, "left neighbour becomes active");

                s.set_active(c, cx);
                s.close(c, cx);
                assert_eq!(s.active_id(), a);
            });
        });
    }

    #[gpui::test]
    fn last_tab_and_home_cannot_close(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| TabStore::new());
            store.update(cx, |s, cx| {
                let home = s.open(TabKind::Home, TabData::default(), cx);
                assert!(s.close(home, cx).is_none(), "last tab stays");
                let _t = s.open(TabKind::Workspace, ws(1), cx);
                assert!(s.close(home, cx).is_none(), "home never closes");
                assert_eq!(s.len(), 2);
            });
        });
    }

    #[gpui::test]
    fn title_and_label_resolution(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| TabStore::new());
            store.update(cx, |s, cx| {
                let a = s.open(TabKind::Workspace, ws(1), cx);
                assert_eq!(s.get(a).unwrap().label(), "Terminal");

                s.sync_workspace_meta(a, Some("/home/me/project".into()), None, cx);
                assert_eq!(s.get(a).unwrap().label(), "project");

                s.sync_workspace_meta(
                    a,
                    Some("/home/me/project".into()),
                    Some("claude".into()),
                    cx,
                );
                assert_eq!(s.get(a).unwrap().label(), "claude", "process title wins");

                s.set_custom_title(a, Some("My Tab".into()), cx);
                assert_eq!(s.get(a).unwrap().label(), "My Tab", "custom title freezes");
            });
        });
    }

    #[gpui::test]
    fn reorder_moves_tab(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| TabStore::new());
            store.update(cx, |s, cx| {
                let a = s.open(TabKind::Workspace, ws(1), cx);
                let b = s.open(TabKind::Workspace, ws(2), cx);
                let c = s.open(TabKind::Workspace, ws(3), cx);
                s.reorder(c, a, cx);
                let order: Vec<u64> = s.tabs().iter().map(|t| t.id).collect();
                assert_eq!(order, vec![c, a, b]);
            });
        });
    }

    #[gpui::test]
    fn dirty_editor_needs_confirm(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| TabStore::new());
            store.update(cx, |s, cx| {
                let _a = s.open(TabKind::Workspace, ws(1), cx);
                let e = s.open(TabKind::Editor, TabData::default(), cx);
                assert!(!s.get(e).unwrap().needs_close_confirm());
                s.set_dirty(e, true, cx);
                assert!(s.get(e).unwrap().needs_close_confirm());
                s.set_dirty(e, false, cx);
                assert!(!s.get(e).unwrap().needs_close_confirm());
            });
        });
    }

    #[gpui::test]
    fn tabs_by_kind_and_close_by_kind(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = cx.new(|_| TabStore::new());
            store.update(cx, |s, cx| {
                s.open(TabKind::Workspace, ws(1), cx);
                s.open(TabKind::Workspace, ws(2), cx);
                let keep = s.open(TabKind::Editor, TabData::default(), cx);
                assert_eq!(s.tabs_by_kind(TabKind::Workspace).len(), 2);

                let removed = s.close_by_kind(TabKind::Workspace, cx);
                assert_eq!(removed.len(), 2);
                assert_eq!(s.len(), 1);
                assert_eq!(s.active_id(), keep);
            });
        });
    }
}
