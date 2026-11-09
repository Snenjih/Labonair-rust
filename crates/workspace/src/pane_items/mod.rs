//! Pane adapters (R10-002, ADR 0005).
//!
//! Every tab is a pane group and every leaf of its tree is a pane of some
//! [`PaneKind`]. Capability crates keep owning their views; this module is the
//! workspace-owned adapter layer that gives each kind the same small surface:
//! a title / icon / dirty marker / bar slot ([`PaneSpec`]), a body element,
//! focus, `can_split` and `split_clone`, and teardown. One file per kind:
//! [`terminal`], [`editor`], [`preview`], [`sftp`] and [`singleton`] (git
//! graph, diffs, keymap, hosts).
//!
//! Pane ids and tab ids come from one process-wide counter
//! ([`crate::pane::next_id`]), so the legacy per-id maps on `Workspace`
//! (`panes`, `editors`, `previews`, `sftp_views`, ...) are now keyed by pane
//! id: a tab's first pane simply reuses the tab id as its pane id, further
//! panes get fresh ids. An editor document is shared between panes: the
//! first pane owns the document's [`EditorView`] under its id (the *slot*),
//! every further pane is an [`EditorBinding`] to that slot with its own view
//! group.

mod editor;
mod preview;
mod sftp;
mod singleton;
mod terminal;

use gpui::{AnyElement, App, Context, Entity, IntoElement, SharedString, Styled, Window};
use labonair_editor::EditorGroupId;
use labonair_editor_ui::EditorView;
use labonair_ui_kit::IconName;

use crate::pane::{PaneId, WorkspaceLayout};
use crate::pane_kind::{PaneKind, PaneMeta};
use crate::tabs::TabKind;
use crate::Workspace;

/// Which document a (non-first) editor pane shows and which view group of it
/// is the pane's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EditorBinding {
    /// The pane id that owns the document's [`EditorView`] in `editors`.
    pub slot: u64,
    pub group: EditorGroupId,
}

/// What the pane bar and the tab label need from a pane.
#[derive(Clone, Debug)]
pub(crate) struct PaneSpec {
    pub kind: PaneKind,
    pub title: SharedString,
    pub icon: IconName,
    pub dirty: bool,
    /// Path / cwd / host shown after the title.
    pub trailing: Option<SharedString>,
}

/// Why a pane could not be cloned by a split.
#[derive(Clone, Debug)]
pub(crate) enum SplitCloneError {
    /// The kind cannot be split (singletons, AI diffs).
    Unsupported(&'static str),
    /// Creating the new view or session failed; the message is user facing.
    Failed(String),
}

impl Workspace {
    // ── Lookups ────────────────────────────────────────────────────────

    /// The tab whose tree contains `pane`.
    pub(crate) fn tab_of_pane(&self, pane: PaneId) -> Option<u64> {
        self.layouts
            .iter()
            .find(|(_, layout)| layout.group.find_pane(pane))
            .map(|(tab, _)| *tab)
    }

    /// The panes of `tab_id` in visual order (the tab's own id when no tree
    /// exists yet).
    pub(crate) fn tab_pane_ids(&self, tab_id: u64) -> Vec<PaneId> {
        self.layouts
            .get(&tab_id)
            .map(WorkspaceLayout::leaves)
            .unwrap_or_else(|| vec![tab_id])
    }

    /// The focused pane of `tab_id`.
    pub(crate) fn focused_pane(&self, tab_id: u64) -> PaneId {
        self.layouts
            .get(&tab_id)
            .and_then(|l| l.active.or_else(|| l.leaves().first().copied()))
            .unwrap_or(tab_id)
    }

    /// `(tab, focused pane)` of the active tab of the active space.
    pub(crate) fn active_pane(&self, cx: &App) -> Option<(u64, PaneId)> {
        let tab = self.tabs.read(cx).active()?;
        (tab.space_id == self.spaces.read(cx).active_id())
            .then(|| (tab.id, self.focused_pane(tab.id)))
    }

    /// The kind of `pane`. Registered metadata wins; otherwise the kind is
    /// inferred from the view map that holds the pane (and registered).
    pub(crate) fn pane_kind(&self, pane: PaneId, cx: &App) -> PaneKind {
        if let Some(kind) = self.pane_meta.kind(pane) {
            return kind;
        }
        self.infer_pane_kind(pane, cx)
    }

    fn infer_pane_kind(&self, pane: PaneId, cx: &App) -> PaneKind {
        if self.panes.contains_key(&pane) {
            PaneKind::Terminal
        } else if self.editors.contains_key(&pane) || self.editor_aliases.contains_key(&pane) {
            PaneKind::Editor
        } else if self.previews.contains_key(&pane) {
            PaneKind::Preview
        } else if self.sftp_views.contains_key(&pane) {
            PaneKind::Sftp
        } else {
            self.tabs
                .read(cx)
                .get(pane)
                .map(|tab| PaneKind::from(tab.kind))
                .unwrap_or(PaneKind::Terminal)
        }
    }

    /// Make sure `tab_id` has a pane tree and that its panes are registered.
    /// A non-terminal tab starts as a single pane whose id is the tab id.
    pub(crate) fn ensure_layout(&mut self, tab_id: u64, cx: &App) {
        if !self.layouts.contains_key(&tab_id) {
            let Some(tab) = self.tabs.read(cx).get(tab_id) else {
                return;
            };
            if tab.kind == TabKind::Workspace {
                return;
            }
            let kind = PaneKind::from(tab.kind);
            self.pane_meta.insert(tab_id, PaneMeta::new(kind));
            self.layouts.insert(tab_id, WorkspaceLayout::new(tab_id));
            return;
        }
        let panes = self.tab_pane_ids(tab_id);
        for pane in panes {
            if !self.pane_meta.contains(pane) {
                let kind = self.infer_pane_kind(pane, cx);
                self.pane_meta.insert(pane, PaneMeta::new(kind));
            }
        }
    }

    /// Register a freshly created pane's kind.
    pub(crate) fn register_pane(&mut self, pane: PaneId, kind: PaneKind) {
        self.pane_meta.insert(pane, PaneMeta::new(kind));
    }

    /// The slot (owning pane id) of the editor document shown by `pane`.
    pub(crate) fn editor_slot_of_pane(&self, pane: PaneId) -> Option<u64> {
        if let Some(binding) = self.editor_aliases.get(&pane) {
            return Some(binding.slot);
        }
        self.editors.contains_key(&pane).then_some(pane)
    }

    /// The editor view and view group backing `pane`.
    pub(crate) fn editor_binding(
        &self,
        pane: PaneId,
        cx: &App,
    ) -> Option<(Entity<EditorView>, EditorGroupId)> {
        if let Some(binding) = self.editor_aliases.get(&pane) {
            return self
                .editors
                .get(&binding.slot)
                .map(|view| (view.clone(), binding.group));
        }
        let view = self.editors.get(&pane)?;
        Some((view.clone(), view.read(cx).first_group()))
    }

    /// The editor slot a tab-level operation should target for `tab_or_pane`:
    /// a pane id with an editor resolves to its slot; a tab id resolves to the
    /// focused editor pane of the tab, else its first editor pane, else the id
    /// itself (so callers keep their "no such editor" behaviour).
    pub(crate) fn editor_slot_for_tab(&self, tab_or_pane: u64) -> u64 {
        if let Some(slot) = self.editor_slot_of_pane(tab_or_pane) {
            return slot;
        }
        let Some(layout) = self.layouts.get(&tab_or_pane) else {
            return tab_or_pane;
        };
        let focused = layout.active;
        focused
            .into_iter()
            .chain(layout.leaves())
            .find_map(|pane| self.editor_slot_of_pane(pane))
            .unwrap_or(tab_or_pane)
    }

    /// The editor slot of the focused pane of the active tab, only when that
    /// pane is an editor.
    pub(crate) fn active_editor_slot(&self, cx: &App) -> Option<u64> {
        let (_, pane) = self.active_pane(cx)?;
        self.editor_slot_of_pane(pane)
    }

    /// The pane bound to `slot`/`group`.
    pub(crate) fn pane_of_editor_group(
        &self,
        slot: u64,
        group: EditorGroupId,
        cx: &App,
    ) -> Option<PaneId> {
        if let Some((pane, _)) = self
            .editor_aliases
            .iter()
            .find(|(_, b)| b.slot == slot && b.group == group)
        {
            return Some(*pane);
        }
        let view = self.editors.get(&slot)?;
        (view.read(cx).first_group() == group).then_some(slot)
    }

    // ── Adapter surface ────────────────────────────────────────────────

    /// Title, icon, dirty marker and bar slot of `pane`.
    pub(crate) fn pane_spec(&self, pane: PaneId, cx: &App) -> PaneSpec {
        let kind = self.pane_kind(pane, cx);
        let mut spec = PaneSpec {
            kind,
            title: SharedString::from(kind.label()),
            icon: kind.icon(),
            dirty: false,
            trailing: None,
        };
        match kind {
            PaneKind::Terminal => self.terminal_pane_spec(pane, cx, &mut spec),
            PaneKind::Editor => self.editor_pane_spec(pane, cx, &mut spec),
            PaneKind::Preview => self.preview_pane_spec(pane, cx, &mut spec),
            PaneKind::Sftp => self.sftp_pane_spec(pane, cx, &mut spec),
            _ => self.singleton_pane_spec(pane, cx, &mut spec),
        }
        spec
    }

    /// Whether the user may split `pane` (`pane::SplitRight/Down`).
    pub(crate) fn pane_can_split(&self, pane: PaneId, cx: &App) -> bool {
        self.pane_kind(pane, cx).can_split()
    }

    /// The element showing `pane`'s content. A pane whose SSH connection is
    /// still being established shows its loading screen instead (per pane,
    /// not per tab).
    pub(crate) fn pane_body(
        &mut self,
        pane: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match self.pane_kind(pane, cx) {
            PaneKind::Terminal => self.terminal_pane_body(pane, cx),
            PaneKind::Editor => self.editor_pane_body(pane, window, cx),
            PaneKind::Preview => self.preview_pane_body(pane, cx),
            PaneKind::Sftp => self.sftp_pane_body(pane, cx),
            other => self.singleton_pane_body(pane, other, cx),
        }
    }

    /// Move keyboard focus into `pane`.
    pub(crate) fn focus_pane(&mut self, pane: PaneId, window: &mut Window, cx: &mut Context<Self>) {
        match self.pane_kind(pane, cx) {
            PaneKind::Terminal => {
                if let Some(entry) = self.panes.get(&pane) {
                    entry.view.read(cx).focus(window);
                    return;
                }
            }
            PaneKind::Editor => {
                if let Some((editor, group)) = self.editor_binding(pane, cx) {
                    editor.update(cx, |e, cx| {
                        e.focus_group(group, cx);
                    });
                    editor.read(cx).focus(window);
                    return;
                }
            }
            PaneKind::Preview => {
                if let Some(view) = self.previews.get(&pane) {
                    window.focus(&gpui::Focusable::focus_handle(view.read(cx), cx));
                    return;
                }
            }
            PaneKind::Diff => {
                if let Some(view) = &self.project_diff {
                    window.focus(&gpui::Focusable::focus_handle(view.read(cx), cx));
                    return;
                }
            }
            PaneKind::Keymap => {
                if let Some(view) = &self.keymap {
                    window.focus(&gpui::Focusable::focus_handle(view.read(cx), cx));
                    return;
                }
            }
            _ => {}
        }
        window.focus(&self.focus_handle);
    }

    /// Create the resources of a new pane `new_id` that clones `source`
    /// (same kind, same location). Layout insertion is the caller's job.
    pub(crate) fn spawn_pane_clone(
        &mut self,
        source: PaneId,
        new_id: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), SplitCloneError> {
        let kind = self.pane_kind(source, cx);
        if !kind.can_split() {
            return Err(SplitCloneError::Unsupported(
                "This kind of pane cannot be split.",
            ));
        }
        match kind {
            PaneKind::Terminal => self.clone_terminal_pane(source, new_id, window, cx)?,
            PaneKind::Editor => self.clone_editor_pane(source, new_id, cx)?,
            PaneKind::Preview => self.clone_preview_pane(source, new_id, cx)?,
            PaneKind::Sftp => self.clone_sftp_pane(source, new_id, cx)?,
            _ => {
                return Err(SplitCloneError::Unsupported(
                    "This kind of pane cannot be split.",
                ))
            }
        }
        self.register_pane(new_id, kind);
        Ok(())
    }

    /// Tear down everything one pane owns (session, view, document binding,
    /// SFTP / SSH connection). Does not touch the layout or the tab.
    pub(crate) fn retire_any_pane(&mut self, pane: PaneId, cx: &mut Context<Self>) {
        self.zoom.pane_closed(pane);
        self.retire_terminal_pane(pane, cx);
        self.retire_editor_pane(pane, cx);
        self.retire_preview_pane(pane);
        self.retire_sftp_pane(pane, cx);
        self.pane_meta.remove(pane);
        self.explorer_modes.remove(&pane);
    }

    /// Refresh the metadata the tab label and close confirmation read.
    pub(crate) fn sync_pane_meta(&mut self, cx: &App) {
        let panes: Vec<PaneId> = self.layouts.values().flat_map(|l| l.leaves()).collect();
        for pane in panes {
            let spec = self.pane_spec(pane, cx);
            self.pane_meta.set_title(pane, spec.title.to_string());
            self.pane_meta.set_dirty(pane, spec.dirty);
        }
    }

    /// Wrap pane content in the `Pane` key context so `pane::*` bindings work
    /// whatever the pane kind is.
    pub(crate) fn pane_context(content: impl IntoElement) -> gpui::Div {
        use gpui::{InteractiveElement, ParentElement};
        gpui::div()
            .key_context("Pane")
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(content)
    }
}
