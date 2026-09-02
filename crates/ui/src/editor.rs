//! GPUI code-editor view (T06-001).
//!
//! Renders and drives a [`labonair_editor::Document`]: viewport-based line
//! rendering with a line-number gutter, caret + selection, keyboard editing,
//! undo/redo, `Cmd-S` save (atomic write via
//! [`labonair_backend::modules::fs::file::save_editor_file_sync`]), a find /
//! replace bar (`Cmd-F`), and external-change detection with a reload banner.
//!
//! The view owns no file IO on the main thread — reads and writes run on
//! `cx.background_executor().spawn`. It emits [`EditorEvent`] so the hosting
//! [`crate::workspace::Workspace`] can mirror the dirty flag and peek state
//! onto the tab.

use std::path::PathBuf;

use gpui::prelude::FluentBuilder;
use gpui::{
    canvas, div, px, App, Bounds, ClickEvent, ClipboardItem, Context, Entity, EventEmitter,
    FocusHandle, Focusable, HighlightStyle, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Render, ScrollWheelEvent,
    SharedString, StatefulInteractiveElement, Styled, StyledText, Window,
};
use labonair_backend::modules::fs::file::{
    file_mtime_sync, load_editor_file_sync, save_editor_file_sync, EditorLoad,
};
use labonair_backend::modules::settings::preferences::Preferences;

use crate::settings::GlobalPreferences;
use labonair_editor::{
    document::Motion, find_all, next_match, Document, Language, Match, Position, SearchQuery,
    SyntaxHighlighter, Vim, VimKey, VimMode, VimOptions,
};

use crate::notifications::{notification_center, Notification};
use crate::syntax_theme::EditorPalette;
use crate::theme::ThemeStore;

/// Editor → workspace notifications.
#[derive(Clone, Copy, Debug)]
pub enum EditorEvent {
    /// Dirty flag or title-relevant state changed — re-sync the tab.
    Changed,
    /// The user made their first edit — a peek tab should become permanent.
    Edited,
    /// Vim `:q` / `:wq` — the hosting workspace should close this editor's tab.
    CloseRequested,
}

/// Which field the find bar is typing into.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FindFocus {
    Query,
    Replace,
}

struct FindBar {
    query: SearchQuery,
    replacement: String,
    focus: FindFocus,
    replace_visible: bool,
    matches: Vec<Match>,
    active: usize,
}

impl Default for FindBar {
    fn default() -> Self {
        Self {
            query: SearchQuery::default(),
            replacement: String::new(),
            focus: FindFocus::Query,
            replace_visible: false,
            matches: Vec::new(),
            active: 0,
        }
    }
}

pub struct EditorView {
    doc: Document,
    theme: Entity<ThemeStore>,
    focus_handle: FocusHandle,
    /// Content-area bounds from the last paint (window-relative).
    bounds: Option<Bounds<Pixels>>,
    /// First visible buffer line.
    scroll_top: usize,
    /// Cached glyph metrics `(char_width, line_height)` in px.
    metrics: (f32, f32),
    gutter_width: f32,
    find: Option<FindBar>,
    /// Tree-sitter syntax highlighter for the current document (T06-002).
    syntax: SyntaxHighlighter,
    /// Bumped on every buffer mutation / load — the highlighter's cache key.
    syntax_rev: u64,
    /// Vim keybinding state machine (T06-003) — `None` when Vim mode is off.
    vim: Option<Vim>,
    /// Live editor preferences mirror (font/indent/line-numbers), refreshed
    /// from [`GlobalPreferences`] (T13-003).
    prefs: Preferences,
}

/// The current preferences snapshot (defaults before the global is published).
fn current_prefs(cx: &App) -> Preferences {
    cx.try_global::<GlobalPreferences>()
        .map(|g| g.0.clone())
        .unwrap_or_default()
}

fn vim_options(p: &Preferences) -> VimOptions {
    let e = p.editor_prefs();
    VimOptions {
        number: e.number,
        relativenumber: e.relative_number,
        hlsearch: e.hlsearch,
        incsearch: e.incsearch,
        smartcase: e.smartcase,
        expandtab: e.expandtab,
        tabstop: e.tabstop,
        shiftwidth: e.shiftwidth,
    }
}

impl EditorView {
    pub fn new(theme: Entity<ThemeStore>, cx: &mut Context<Self>) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe_global::<GlobalPreferences>(|this, cx| this.apply_prefs(cx))
            .detach();
        let prefs = current_prefs(cx);
        let vim = prefs.editor_vim_mode.then(|| Vim::new(vim_options(&prefs)));
        Self {
            doc: Document::empty(),
            theme,
            focus_handle: cx.focus_handle(),
            bounds: None,
            scroll_top: 0,
            metrics: (8.0, 18.0),
            gutter_width: 48.0,
            find: None,
            syntax: SyntaxHighlighter::new(Language::PlainText),
            syntax_rev: 0,
            vim,
            prefs,
        }
    }

    /// Re-read the live preferences and reconcile the Vim layer without
    /// touching the document buffer (T13-003).
    fn apply_prefs(&mut self, cx: &mut Context<Self>) {
        let p = current_prefs(cx);
        if p == self.prefs {
            return;
        }
        self.prefs = p;
        match (&mut self.vim, self.prefs.editor_vim_mode) {
            (Some(vim), true) => vim.options = vim_options(&self.prefs),
            (Some(_), false) => self.vim = None,
            (None, true) => self.vim = Some(Vim::new(vim_options(&self.prefs))),
            (None, false) => {}
        }
        cx.notify();
    }

    /// One indentation step for a Tab press outside Vim mode.
    fn indent_unit(&self) -> String {
        if self.prefs.editor_indent_with_tabs {
            "\t".to_string()
        } else {
            " ".repeat((self.prefs.editor_tab_size.max(1)) as usize)
        }
    }

    /// Soft-wrap width in columns (0 = wrapping off), from the
    /// `editor_word_wrap` preference and the measured content width (T06-005).
    /// Mirrors CodeMirror's `EditorView.lineWrapping`.
    fn wrap_cols(&self) -> usize {
        if !self.prefs.editor_word_wrap {
            return 0;
        }
        let (cw, _) = self.metrics;
        let width = self.bounds.map(|b| f32::from(b.size.width)).unwrap_or(0.0) - self.gutter_width;
        if cw <= 0.0 || width <= cw * 2.0 {
            return 0;
        }
        (width / cw).floor() as usize
    }

    /// Vertical caret motion across visual (wrapped) rows. Returns `false` when
    /// soft-wrap is off so the caller falls back to logical [`Motion`].
    fn wrap_vertical(&mut self, down: bool, extend: bool, cx: &mut Context<Self>) -> bool {
        let cols = self.wrap_cols();
        if cols == 0 {
            return false;
        }
        let cur = self.doc.cursor;
        let seg = cur.column / cols;
        let cin = cur.column % cols;
        let last_seg = Wrap { cols }
            .rows(self.doc.buffer.line_len(cur.line))
            .saturating_sub(1);
        let last_line = self.doc.buffer.line_count().saturating_sub(1);
        let target = if down {
            if seg < last_seg {
                Position::new(cur.line, (seg + 1) * cols + cin)
            } else if cur.line >= last_line {
                return true;
            } else {
                Position::new(cur.line + 1, cin)
            }
        } else if seg > 0 {
            Position::new(cur.line, (seg - 1) * cols + cin)
        } else if cur.line == 0 {
            return true;
        } else {
            let p = cur.line - 1;
            let plast = Wrap { cols }
                .rows(self.doc.buffer.line_len(p))
                .saturating_sub(1);
            Position::new(p, plast * cols + cin)
        };
        self.doc.set_caret(target, extend);
        self.ensure_cursor_visible();
        cx.notify();
        true
    }

    /// Home / End across the current visual row. Returns `false` when soft-wrap
    /// is off.
    fn wrap_horizontal(&mut self, end: bool, extend: bool, cx: &mut Context<Self>) -> bool {
        let cols = self.wrap_cols();
        if cols == 0 {
            return false;
        }
        let cur = self.doc.cursor;
        let seg = cur.column / cols;
        let line_len = self.doc.buffer.line_len(cur.line);
        let target = if end {
            Position::new(cur.line, ((seg + 1) * cols).min(line_len))
        } else {
            Position::new(cur.line, seg * cols)
        };
        self.doc.set_caret(target, extend);
        self.ensure_cursor_visible();
        cx.notify();
        true
    }

    fn bump_syntax(&mut self) {
        self.syntax_rev = self.syntax_rev.wrapping_add(1);
    }

    /// Point the highlighter at the current document's language and force a
    /// re-parse (called after a load / reload).
    fn resync_syntax(&mut self) {
        self.syntax.set_language(self.doc.language);
        self.syntax.invalidate();
        self.bump_syntax();
    }

    pub fn focus(&self, window: &mut Window) {
        window.focus(&self.focus_handle);
    }

    pub fn path(&self) -> Option<PathBuf> {
        self.doc.path.clone()
    }

    pub fn is_dirty(&self) -> bool {
        self.doc.is_dirty()
    }

    pub fn title(&self) -> String {
        let base = self
            .doc
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| "Untitled".to_string());
        base
    }

    // ── Loading ─────────────────────────────────────────────────────────────

    /// Load `path` into this view, replacing any current document.
    pub fn open_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let path_str = path.to_string_lossy().to_string();
        let load = cx
            .background_executor()
            .spawn(async move { load_editor_file_sync(&path_str, None) });
        cx.spawn(async move |this, cx| {
            let result = load.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(EditorLoad::Text { content, mtime }) => {
                        this.doc = Document::from_file(path, &content, mtime);
                        this.scroll_top = 0;
                    }
                    Ok(EditorLoad::Binary) => {
                        this.doc = Document::from_file(path, "// Binary file — not shown.\n", 0);
                    }
                    Ok(EditorLoad::TooLarge { size, limit }) => {
                        this.doc = Document::from_file(
                            path,
                            &format!("// File is {size} bytes (limit {limit}) — not shown.\n"),
                            0,
                        );
                    }
                    Err(message) => {
                        notify(cx, "Open file failed", &message);
                        return;
                    }
                }
                this.resync_syntax();
                cx.emit(EditorEvent::Changed);
                cx.notify();
            });
        })
        .detach();
    }

    /// Re-stat the file; auto-reload if we have no unsaved edits, otherwise flag
    /// the conflict for the reload banner. Called when the tab is (re)activated.
    pub fn check_external(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.doc.path.clone() else {
            return;
        };
        let path_str = path.to_string_lossy().to_string();
        let known = self.doc.disk_mtime;
        let stat = cx
            .background_executor()
            .spawn(async move { file_mtime_sync(&path_str) });
        cx.spawn(async move |this, cx| {
            let Ok(mtime) = stat.await else {
                return;
            };
            if mtime == known {
                return;
            }
            let _ = this.update(cx, |this, cx| {
                if this.doc.is_dirty() {
                    this.doc.note_disk_mtime(mtime);
                    cx.notify();
                } else {
                    this.reload_from_disk(cx);
                }
            });
        })
        .detach();
    }

    fn reload_from_disk(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.doc.path.clone() else {
            return;
        };
        let path_str = path.to_string_lossy().to_string();
        let load = cx
            .background_executor()
            .spawn(async move { load_editor_file_sync(&path_str, None) });
        cx.spawn(async move |this, cx| {
            let result = load.await;
            let _ = this.update(cx, |this, cx| {
                if let Ok(EditorLoad::Text { content, mtime }) = result {
                    this.doc.reload(&content, mtime);
                    this.clamp_scroll();
                    this.resync_syntax();
                    cx.emit(EditorEvent::Changed);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    // ── Saving ──────────────────────────────────────────────────────────────

    pub fn save(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.doc.path.clone() else {
            notify(cx, "Save", "This document has no file path.");
            return;
        };
        let path_str = path.to_string_lossy().to_string();
        let content = self.doc.text();
        let write = cx
            .background_executor()
            .spawn(async move { save_editor_file_sync(&path_str, &content) });
        cx.spawn(async move |this, cx| {
            let result = write.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(mtime) => {
                    this.doc.mark_saved(mtime);
                    cx.emit(EditorEvent::Changed);
                    cx.notify();
                }
                Err(message) => notify(cx, "Save failed", &message),
            });
        })
        .detach();
    }

    // ── Editing helpers ─────────────────────────────────────────────────────

    fn after_edit(&mut self, cx: &mut Context<Self>) {
        let was_first = self.doc.is_dirty();
        self.bump_syntax();
        self.ensure_cursor_visible();
        self.refresh_matches();
        cx.emit(EditorEvent::Changed);
        if was_first {
            cx.emit(EditorEvent::Edited);
        }
        cx.notify();
    }

    fn edit(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Document)) {
        let dirty_before = self.doc.is_dirty();
        f(&mut self.doc);
        self.bump_syntax();
        self.ensure_cursor_visible();
        self.refresh_matches();
        cx.emit(EditorEvent::Changed);
        if !dirty_before && self.doc.is_dirty() {
            cx.emit(EditorEvent::Edited);
        }
        cx.notify();
    }

    fn navigate(&mut self, motion: Motion, extend: bool, cx: &mut Context<Self>) {
        self.doc.move_caret(motion, extend);
        self.ensure_cursor_visible();
        cx.notify();
    }

    fn copy(&self, cx: &mut Context<Self>) {
        if let Some(text) = self.doc.selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn cut(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.doc.selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.edit(cx, |d| d.backspace());
        }
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|i| i.text()) {
            if !text.is_empty() {
                self.edit(cx, |d| d.insert(&text));
            }
        }
    }

    // ── Scrolling ───────────────────────────────────────────────────────────

    fn visible_rows(&self) -> usize {
        let h = self
            .bounds
            .map(|b| f32::from(b.size.height))
            .unwrap_or(600.0);
        ((h / self.metrics.1).floor() as usize).max(1)
    }

    fn clamp_scroll(&mut self) {
        let max = self.doc.buffer.line_count().saturating_sub(1);
        self.scroll_top = self.scroll_top.min(max);
    }

    fn ensure_cursor_visible(&mut self) {
        let rows = self.visible_rows();
        let line = self.doc.cursor.line;
        if line < self.scroll_top {
            self.scroll_top = line;
        } else if line >= self.scroll_top + rows {
            self.scroll_top = line + 1 - rows;
        }
        self.clamp_scroll();
    }

    fn scroll_by(&mut self, lines: isize, cx: &mut Context<Self>) {
        let max = self.doc.buffer.line_count().saturating_sub(1) as isize;
        let next = (self.scroll_top as isize + lines).clamp(0, max);
        if next as usize != self.scroll_top {
            self.scroll_top = next as usize;
            cx.notify();
        }
    }

    // ── Find / replace ──────────────────────────────────────────────────────

    pub fn toggle_find(&mut self, cx: &mut Context<Self>) {
        if self.find.is_some() {
            self.find = None;
        } else {
            let mut bar = FindBar::default();
            if let Some(sel) = self.doc.selected_text() {
                if !sel.contains('\n') {
                    bar.query.text = sel;
                }
            }
            self.find = Some(bar);
            self.refresh_matches();
        }
        cx.notify();
    }

    fn refresh_matches(&mut self) {
        if let Some(bar) = &mut self.find {
            bar.matches = find_all(&self.doc.buffer, &bar.query);
            if bar.active >= bar.matches.len() {
                bar.active = 0;
            }
        }
    }

    fn select_match(&mut self, idx: usize) {
        let Some(bar) = &mut self.find else { return };
        let Some(m) = bar.matches.get(idx).copied() else {
            return;
        };
        bar.active = idx;
        self.doc.set_caret(m.start, false);
        self.doc.set_caret(m.end, true);
        self.ensure_cursor_visible();
    }

    fn find_step(&mut self, forward: bool, cx: &mut Context<Self>) {
        self.refresh_matches();
        let Some(bar) = &self.find else { return };
        if bar.matches.is_empty() {
            return;
        }
        let from = self.doc.cursor;
        let idx = if forward {
            next_match(&bar.matches, from, true).unwrap_or(0)
        } else {
            next_match(&bar.matches, from, false).unwrap_or(0)
        };
        self.select_match(idx);
        cx.notify();
    }

    fn replace_current(&mut self, cx: &mut Context<Self>) {
        let Some(bar) = &self.find else { return };
        let replacement = bar.replacement.clone();
        if self.doc.selected_text().is_some() {
            self.edit(cx, |d| d.insert(&replacement));
            self.find_step(true, cx);
        } else {
            self.find_step(true, cx);
        }
    }

    fn replace_all(&mut self, cx: &mut Context<Self>) {
        let Some(bar) = &self.find else { return };
        let query = bar.query.clone();
        let replacement = bar.replacement.clone();
        if query.text.is_empty() {
            return;
        }
        let n = self.doc.replace_all(&query, &replacement);
        self.refresh_matches();
        self.after_edit(cx);
        notify_info(cx, "Replace all", &format!("{n} replacement(s)"));
    }

    fn find_key(&mut self, ev: &KeyDownEvent, cx: &mut Context<Self>) -> bool {
        let Some(bar) = self.find.as_mut() else {
            return false;
        };
        let ks = &ev.keystroke;
        let m = &ks.modifiers;
        match ks.key.as_str() {
            "escape" => {
                self.find = None;
                cx.notify();
                return true;
            }
            "enter" => {
                if m.platform {
                    self.replace_all(cx);
                } else {
                    self.find_step(!m.shift, cx);
                }
                return true;
            }
            "tab" => {
                bar.replace_visible = true;
                bar.focus = match bar.focus {
                    FindFocus::Query => FindFocus::Replace,
                    FindFocus::Replace => FindFocus::Query,
                };
                cx.notify();
                return true;
            }
            "backspace" => {
                match bar.focus {
                    FindFocus::Query => {
                        bar.query.text.pop();
                    }
                    FindFocus::Replace => {
                        bar.replacement.pop();
                    }
                }
                self.refresh_matches();
                cx.notify();
                return true;
            }
            _ => {}
        }
        if m.platform && !m.control && !m.alt {
            match ks.key.as_str() {
                "c" => {
                    bar.query.case_sensitive = !bar.query.case_sensitive;
                    self.refresh_matches();
                    cx.notify();
                    return true;
                }
                "w" => {
                    bar.query.whole_word = !bar.query.whole_word;
                    self.refresh_matches();
                    cx.notify();
                    return true;
                }
                _ => return true, // swallow other cmd combos while the bar is open
            }
        }
        if let Some(text) = printable(ks) {
            match bar.focus {
                FindFocus::Query => bar.query.text.push_str(&text),
                FindFocus::Replace => bar.replacement.push_str(&text),
            }
            self.refresh_matches();
            cx.notify();
            return true;
        }
        false
    }

    // ── Key handling ────────────────────────────────────────────────────────

    fn on_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.find.is_some() && self.find_key(ev, cx) {
            cx.stop_propagation();
            return;
        }

        let ks = &ev.keystroke;
        let m = &ks.modifiers;

        if m.platform && !m.control && !m.alt {
            match ks.key.as_str() {
                "s" => self.save(cx),
                "z" if m.shift => self.edit(cx, Document::redo),
                "z" => self.edit(cx, Document::undo),
                "y" => self.edit(cx, Document::redo),
                "a" => {
                    self.doc.select_all();
                    cx.notify();
                }
                "c" => self.copy(cx),
                "x" => self.cut(cx),
                "v" => self.paste(cx),
                "f" => self.toggle_find(cx),
                "left" => self.navigate(Motion::LineStart, m.shift, cx),
                "right" => self.navigate(Motion::LineEnd, m.shift, cx),
                "up" => self.navigate(Motion::DocStart, m.shift, cx),
                "down" => self.navigate(Motion::DocEnd, m.shift, cx),
                _ => return,
            }
            cx.stop_propagation();
            return;
        }

        if m.alt && !m.platform && !m.control {
            match ks.key.as_str() {
                "left" => self.navigate(Motion::WordLeft, m.shift, cx),
                "right" => self.navigate(Motion::WordRight, m.shift, cx),
                _ => return,
            }
            cx.stop_propagation();
            return;
        }

        // Vim mode (T06-003): consume the keystroke in the modal state machine.
        // Insert mode lets non-text keys (arrows, page-up/down) fall through to
        // the regular editor navigation below.
        if self.vim.is_some() {
            if let Some(key) = self.vim_key(ks) {
                self.handle_vim(key, cx);
                cx.stop_propagation();
                return;
            }
        }

        if m.control || m.alt {
            return;
        }

        let rows = self.visible_rows().saturating_sub(1).max(1);
        match ks.key.as_str() {
            "left" => self.navigate(Motion::Left, m.shift, cx),
            "right" => self.navigate(Motion::Right, m.shift, cx),
            "up" => {
                if !self.wrap_vertical(false, m.shift, cx) {
                    self.navigate(Motion::Up, m.shift, cx)
                }
            }
            "down" => {
                if !self.wrap_vertical(true, m.shift, cx) {
                    self.navigate(Motion::Down, m.shift, cx)
                }
            }
            "home" => {
                if !self.wrap_horizontal(false, m.shift, cx) {
                    self.navigate(Motion::LineStart, m.shift, cx)
                }
            }
            "end" => {
                if !self.wrap_horizontal(true, m.shift, cx) {
                    self.navigate(Motion::LineEnd, m.shift, cx)
                }
            }
            "pageup" => self.navigate(Motion::PageUp(rows), m.shift, cx),
            "pagedown" => self.navigate(Motion::PageDown(rows), m.shift, cx),
            "enter" => self.edit(cx, |d| d.insert("\n")),
            "tab" => {
                let indent = self.indent_unit();
                self.edit(cx, move |d| d.insert(&indent));
            }
            "backspace" => self.edit(cx, Document::backspace),
            "delete" => self.edit(cx, Document::delete_forward),
            "escape" => {}
            _ => {
                if let Some(text) = printable(ks) {
                    self.edit(cx, |d| d.insert(&text));
                } else {
                    return;
                }
            }
        }
        cx.stop_propagation();
    }

    // ── Vim mode ────────────────────────────────────────────────────────────

    /// Translate a keystroke into a [`VimKey`], or `None` to let the regular
    /// editor navigation handle it (arrow / paging keys in insert mode).
    fn vim_key(&self, ks: &gpui::Keystroke) -> Option<VimKey> {
        let m = &ks.modifiers;
        let insert = self.vim.as_ref().map(Vim::mode) == Some(VimMode::Insert);
        match ks.key.as_str() {
            "escape" => Some(VimKey::Esc),
            "enter" | "return" => Some(VimKey::Enter),
            "backspace" => Some(VimKey::Backspace),
            "tab" => Some(VimKey::Tab),
            "r" if m.control => Some(VimKey::Redo),
            "left" if !insert => Some(VimKey::Char('h')),
            "right" if !insert => Some(VimKey::Char('l')),
            "up" if !insert => Some(VimKey::Char('k')),
            "down" if !insert => Some(VimKey::Char('j')),
            _ => {
                if m.control || m.platform || m.alt {
                    None
                } else {
                    printable(ks)
                        .and_then(|s| s.chars().next())
                        .map(VimKey::Char)
                }
            }
        }
    }

    fn handle_vim(&mut self, key: VimKey, cx: &mut Context<Self>) {
        let dirty_before = self.doc.is_dirty();
        let resp = {
            let vim = self.vim.as_mut().expect("vim mode active");
            vim.on_key(&mut self.doc, key)
        };
        if resp.handled {
            self.bump_syntax();
            self.ensure_cursor_visible();
            self.refresh_matches();
            cx.emit(EditorEvent::Changed);
            if !dirty_before && self.doc.is_dirty() {
                cx.emit(EditorEvent::Edited);
            }
            cx.notify();
        }
        if resp.save {
            self.save(cx);
        }
        if resp.reload {
            self.reload_from_disk(cx);
        }
        if resp.quit {
            cx.emit(EditorEvent::CloseRequested);
        }
    }

    fn position_at(&self, p: Point<Pixels>) -> Position {
        let origin = self.bounds.map(|b| b.origin).unwrap_or_default();
        let (cw, lh) = self.metrics;
        let x = (f32::from(p.x - origin.x) - self.gutter_width).max(0.0);
        let y = f32::from(p.y - origin.y).max(0.0);
        let col_hint = ((x / cw) + 0.5) as usize;
        let cols = self.wrap_cols();
        if cols == 0 {
            return Position::new(self.scroll_top + (y / lh) as usize, col_hint);
        }
        // Walk visual rows from the scroll top to resolve the logical line and
        // which wrapped segment the click landed on (T06-005).
        let wrap = Wrap { cols };
        let count = self.doc.buffer.line_count();
        let mut cum = 0.0f32;
        let mut line = self.scroll_top;
        while line < count {
            let segs = wrap.rows(self.doc.buffer.line_len(line));
            let h = segs as f32 * lh;
            if y < cum + h || line + 1 == count {
                let seg = (((y - cum) / lh).max(0.0) as usize).min(segs.saturating_sub(1));
                return Position::new(line, seg * cols + col_hint);
            }
            cum += h;
            line += 1;
        }
        Position::new(count.saturating_sub(1), col_hint)
    }

    // ── Rendering ───────────────────────────────────────────────────────────

    fn render_find_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (card, fg, muted, border, accent) = (
            theme.card(),
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
            theme.accent(),
        );
        let bar = self.find.as_ref().unwrap();
        let count = if bar.matches.is_empty() {
            "No results".to_string()
        } else {
            format!("{}/{}", bar.active + 1, bar.matches.len())
        };

        let field = |label: &str, value: &str, focused: bool| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .px_2()
                .py_0p5()
                .min_w(px(200.0))
                .rounded_sm()
                .border_1()
                .border_color(if focused { accent } else { border })
                .text_color(fg)
                .child(div().text_xs().text_color(muted).child(label.to_string()))
                .child(SharedString::from(value.to_string()))
        };

        let toggle = |on: bool, glyph: &str| {
            div()
                .px_1()
                .rounded_sm()
                .text_xs()
                .text_color(if on { accent } else { muted })
                .child(glyph.to_string())
        };

        div()
            .flex()
            .items_center()
            .gap_2()
            .w_full()
            .flex_shrink_0()
            .px_2()
            .py_1()
            .bg(card)
            .border_b_1()
            .border_color(border)
            .child(field(
                "Find",
                &bar.query.text,
                bar.focus == FindFocus::Query,
            ))
            .when(bar.replace_visible, |d| {
                d.child(field(
                    "Replace",
                    &bar.replacement,
                    bar.focus == FindFocus::Replace,
                ))
            })
            .child(toggle(bar.query.case_sensitive, "Aa"))
            .child(toggle(bar.query.whole_word, "\u{2039}W\u{203a}"))
            .child(div().text_xs().text_color(muted).child(count))
            .child(
                div()
                    .id("find-prev")
                    .px_1()
                    .rounded_sm()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child("\u{2191}")
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| this.find_step(false, cx)),
                    ),
            )
            .child(
                div()
                    .id("find-next")
                    .px_1()
                    .rounded_sm()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child("\u{2193}")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.find_step(true, cx))),
            )
            .child(
                div()
                    .id("replace-one")
                    .px_1()
                    .text_xs()
                    .rounded_sm()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child("Replace")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.replace_current(cx))),
            )
            .child(
                div()
                    .id("replace-all")
                    .px_1()
                    .text_xs()
                    .rounded_sm()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child("All")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.replace_all(cx))),
            )
            .child(
                div()
                    .id("find-close")
                    .px_1()
                    .rounded_sm()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child("\u{2715}")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.find = None;
                        cx.notify();
                    })),
            )
    }

    fn render_conflict_banner(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (warn, fg, border) = (theme.status_warning(), theme.foreground(), theme.border());
        div()
            .flex()
            .items_center()
            .gap_3()
            .w_full()
            .flex_shrink_0()
            .px_3()
            .py_1()
            .bg(warn.opacity(0.15))
            .border_b_1()
            .border_color(border)
            .text_xs()
            .text_color(fg)
            .child("This file changed on disk since you started editing.")
            .child(
                div()
                    .id("conflict-reload")
                    .px_2()
                    .rounded_sm()
                    .border_1()
                    .border_color(border)
                    .hover(|s| s.bg(border))
                    .child("Reload (discard my changes)")
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| this.reload_from_disk(cx)),
                    ),
            )
            .child(
                div()
                    .id("conflict-keep")
                    .px_2()
                    .rounded_sm()
                    .border_1()
                    .border_color(border)
                    .hover(|s| s.bg(border))
                    .child("Keep mine")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.doc.external_change = false;
                        cx.notify();
                    })),
            )
    }

    /// Bottom status line for Vim mode: the mode indicator plus the live
    /// `:` / `/` command line (T06-003).
    fn render_vim_status(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (card, fg, muted, border, accent) = (
            theme.card(),
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
            theme.accent(),
        );
        let vim = self.vim.as_ref().unwrap();
        let (label, is_command) = match vim.command_line() {
            Some((prefix, text)) => (format!("{prefix}{text}"), true),
            None => {
                let s = vim.status();
                (
                    if s.is_empty() {
                        "NORMAL".to_string()
                    } else {
                        s
                    },
                    false,
                )
            }
        };
        let cursor = self.doc.cursor;
        div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .flex_shrink_0()
            .px_3()
            .py_0p5()
            .bg(card)
            .border_t_1()
            .border_color(border)
            .text_xs()
            .font(theme.buffer_font())
            .text_color(if is_command { fg } else { accent })
            .child(SharedString::from(label))
            .child(div().text_color(muted).child(SharedString::from(format!(
                "{}:{}",
                cursor.line + 1,
                cursor.column + 1
            ))))
    }
}

impl EventEmitter<EditorEvent> for EditorView {}

impl Focusable for EditorView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EditorView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let bg = theme.background();
        let fg = theme.foreground();
        let muted = theme.muted_foreground();
        let accent = theme.accent();
        let gutter_bg = theme.card();
        let font = theme.buffer_font();
        let font_px = theme.buffer_font_size();
        let line_h = (font_px * 1.5).ceil().max(1.0);

        let font_id = cx.text_system().resolve_font(&font);
        let char_w = cx
            .text_system()
            .ch_advance(font_id, px(font_px))
            .map(f32::from)
            .unwrap_or(font_px * 0.6)
            .max(1.0);
        self.metrics = (char_w, line_h);

        let line_count = self.doc.buffer.line_count();
        let digits = line_count.to_string().len().max(3);
        self.gutter_width = digits as f32 * char_w + 16.0;
        self.clamp_scroll();

        let rows = self.visible_rows();
        let first = self.scroll_top;
        let wrap = Wrap {
            cols: self.wrap_cols(),
        };
        // Visible logical lines → (line, top offset px, visual-row count).
        // Without soft-wrap every line is exactly one row tall (T06-005).
        let viewport_h = rows as f32 * line_h;
        let content_w = (wrap.cols as f32 * char_w).max(char_w);
        let mut layout: Vec<(usize, f32, usize)> = Vec::new();
        {
            let mut cum = 0.0f32;
            let mut line = first;
            while line < line_count {
                let segs = wrap.rows(self.doc.buffer.line_len(line));
                layout.push((line, cum, segs));
                cum += segs as f32 * line_h;
                line += 1;
                if cum >= viewport_h + line_h {
                    break;
                }
            }
        }
        let last = layout.last().map(|(l, _, _)| *l + 1).unwrap_or(first);
        let top_of = |target: usize| -> Option<(f32, usize)> {
            layout
                .iter()
                .find(|(l, _, _)| *l == target)
                .map(|(_, t, s)| (*t, *s))
        };

        let sel = self.doc.selection();
        let cursor = self.doc.cursor;
        let gutter_width = self.gutter_width;

        // Syntax highlighting (T06-002): parse once per revision, keep only the
        // spans covering the visible line range, and repaint with the palette
        // resolved from the active app theme.
        let palette = EditorPalette::resolve(theme.editor_theme(), theme);
        let doc_text = self.doc.buffer.text();
        let mut line_starts: Vec<usize> = Vec::with_capacity(last.saturating_sub(first) + 1);
        let mut offset = 0usize;
        for i in 0..last {
            if i >= first {
                line_starts.push(offset);
            }
            offset += self.doc.buffer.line(i).len() + 1;
        }
        let visible_start = line_starts.first().copied().unwrap_or(0);
        let visible_end = offset.min(doc_text.len());
        self.syntax
            .update(&doc_text, self.syntax_rev, visible_start..visible_end);

        // Gutter rows. Line-number visibility follows the editor preferences
        // (T13-003); Vim's `:set number` overrides them while Vim mode is on.
        let (show_numbers, relative) = self
            .vim
            .as_ref()
            .map(|v| (v.options.number, v.options.relativenumber))
            .unwrap_or((
                self.prefs.editor_line_numbers,
                self.prefs.editor_relative_line_numbers,
            ));
        let gutter = (first..last).map(|line| {
            let on_cursor = line == cursor.line;
            let label = if !show_numbers {
                String::new()
            } else if relative && !on_cursor {
                (line as isize - cursor.line as isize)
                    .unsigned_abs()
                    .to_string()
            } else {
                (line + 1).to_string()
            };
            let row_top = top_of(line).map(|(t, _)| t).unwrap_or(0.0);
            div()
                .absolute()
                .top(px(row_top))
                .left_0()
                .w(px(gutter_width))
                .h(px(line_h))
                .flex()
                .items_center()
                .justify_end()
                .pr(px(8.0))
                .text_size(px(font_px))
                .font(font.clone())
                .text_color(if on_cursor { fg } else { muted })
                .child(SharedString::from(label))
        });

        // Text + selection rows.
        let text_rows = (first..last).map(|line| {
            let (row_top, segs) = top_of(line).unwrap_or((0.0, 1));
            let content = self.doc.buffer.line(line).to_string();
            let mut row = div()
                .absolute()
                .top(px(row_top))
                .left(px(0.0))
                .h(px(segs as f32 * line_h))
                .flex()
                .items_start()
                .text_size(px(font_px))
                .line_height(px(line_h))
                .font(font.clone())
                .text_color(fg);
            row = if wrap.enabled() {
                row.w(px(content_w))
            } else {
                row.items_center().whitespace_nowrap()
            };

            // Selection highlight — one rect per visual segment when wrapped.
            if let Some((s, e)) = sel {
                if line >= s.line && line <= e.line {
                    let start_col = if line == s.line { s.column } else { 0 };
                    let end_col = if line == e.line {
                        e.column
                    } else {
                        self.doc.buffer.line_len(line) + 1
                    };
                    if wrap.enabled() {
                        for seg in 0..segs {
                            let seg_lo = seg * wrap.cols;
                            let lo = start_col.max(seg_lo);
                            let hi = end_col.min(seg_lo + wrap.cols);
                            if hi > lo {
                                row = row.child(
                                    div()
                                        .absolute()
                                        .left(px((lo - seg_lo) as f32 * char_w))
                                        .top(px(seg as f32 * line_h))
                                        .w(px(((hi - lo) as f32 * char_w).max(2.0)))
                                        .h(px(line_h))
                                        .bg(accent.opacity(0.3)),
                                );
                            }
                        }
                    } else {
                        let x = start_col as f32 * char_w;
                        let w = ((end_col.saturating_sub(start_col)) as f32 * char_w).max(2.0);
                        row = row.child(
                            div()
                                .absolute()
                                .left(px(x))
                                .top(px(0.0))
                                .w(px(w))
                                .h(px(line_h))
                                .bg(accent.opacity(0.3)),
                        );
                    }
                }
            }

            // Syntax-highlighted line text.
            let line_start = line_starts[line - first];
            let runs = self.syntax.line_runs(&content, line_start);
            let mut highlights: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
            let mut byte = 0usize;
            for run in &runs {
                let len = run.text.len();
                if let Some(kind) = run.kind {
                    highlights.push((
                        byte..byte + len,
                        HighlightStyle {
                            color: Some(palette.color(kind)),
                            ..Default::default()
                        },
                    ));
                }
                byte += len;
            }
            let text = if highlights.is_empty() {
                div().child(SharedString::from(content)).into_any_element()
            } else {
                StyledText::new(SharedString::from(content))
                    .with_highlights(highlights)
                    .into_any_element()
            };
            let text = if wrap.enabled() {
                div()
                    .w(px(content_w))
                    .line_height(px(line_h))
                    .child(text)
                    .into_any_element()
            } else {
                text
            };
            row.child(text)
        });

        // Caret + current-line band (over visual rows when soft-wrap is on).
        let caret_layout = top_of(cursor.line);
        let (caret_seg, caret_col) = if wrap.enabled() {
            let seg = (cursor.column / wrap.cols)
                .min(caret_layout.map(|(_, s)| s.saturating_sub(1)).unwrap_or(0));
            (seg, cursor.column - seg * wrap.cols)
        } else {
            (0, cursor.column)
        };
        let caret = caret_layout.map(|(top, _)| {
            div()
                .absolute()
                .top(px(top + caret_seg as f32 * line_h))
                .left(px(caret_col as f32 * char_w))
                .w(px(2.0))
                .h(px(line_h))
                .bg(accent)
        });

        let current_line = caret_layout.map(|(top, segs)| {
            div()
                .absolute()
                .top(px(top))
                .left(px(0.0))
                .right(px(0.0))
                .h(px(segs as f32 * line_h))
                .bg(fg.opacity(0.04))
        });

        let weak = cx.weak_entity();
        let probe = canvas(
            move |bounds, _window, cx| {
                let _ = weak.update(cx, |this, cx| {
                    if this.bounds != Some(bounds) {
                        this.bounds = Some(bounds);
                        cx.notify();
                    }
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        let _ = window;

        let text_area = div()
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(probe)
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .bg(gutter_bg)
                    .w(px(gutter_width))
                    .children(gutter),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(gutter_width))
                    .right_0()
                    .overflow_hidden()
                    .children(current_line)
                    .children(text_rows)
                    .children(caret),
            );

        div()
            .id("editor")
            .track_focus(&self.focus_handle)
            .key_context("Editor")
            .flex()
            .flex_col()
            .size_full()
            .bg(bg)
            .when(self.doc.external_change, |d| {
                d.child(self.render_conflict_banner(cx))
            })
            .when(self.find.is_some(), |d| d.child(self.render_find_bar(cx)))
            .child(text_area)
            .when(self.vim.is_some(), |d| d.child(self.render_vim_status(cx)))
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus_handle);
                    let pos = this.position_at(ev.position);
                    this.doc.set_caret(pos, ev.modifiers.shift);
                    cx.notify();
                }),
            )
            .on_scroll_wheel(cx.listener(|this, ev: &ScrollWheelEvent, window, cx| {
                let dy = match ev.delta {
                    gpui::ScrollDelta::Lines(p) => p.y,
                    gpui::ScrollDelta::Pixels(p) => f32::from(p.y) / this.metrics.1,
                };
                let _ = window;
                if dy.abs() >= 0.01 {
                    this.scroll_by(-(dy.round() as isize).clamp(-8, 8), cx);
                }
            }))
    }
}

/// Soft-wrap geometry (T06-005). `cols == 0` disables wrapping; otherwise a
/// logical line is broken every `cols` characters (character grid, like a
/// monospace terminal — no word-boundary breaking, matching CodeMirror's
/// default `lineWrapping` for code).
#[derive(Clone, Copy)]
struct Wrap {
    cols: usize,
}

impl Wrap {
    fn enabled(&self) -> bool {
        self.cols > 0
    }

    /// Visual-row count for a logical line of `len` characters (at least 1).
    fn rows(&self, len: usize) -> usize {
        if self.cols == 0 {
            return 1;
        }
        len.max(1).div_ceil(self.cols)
    }
}

/// A GPUI keystroke that produces a single printable character, if any.
fn printable(ks: &gpui::Keystroke) -> Option<String> {
    let text = ks.key_char.clone()?;
    if text.chars().any(|c| c.is_control()) || text.is_empty() {
        return None;
    }
    Some(text)
}

fn notify(cx: &mut App, title: &str, body: &str) {
    let (title, body) = (title.to_string(), body.to_string());
    notification_center(cx).update(cx, |center, cx| {
        center.push(Notification::error(title, body), cx);
    });
}

fn notify_info(cx: &mut App, title: &str, body: &str) {
    let (title, body) = (title.to_string(), body.to_string());
    notification_center(cx).update(cx, |center, cx| {
        center.push(Notification::info(title, body), cx);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext};

    fn setup(cx: &mut TestAppContext) -> Entity<EditorView> {
        cx.update(|cx| {
            let theme = cx.new(|_| crate::theme::ThemeStore::new(gpui::WindowAppearance::Light));
            cx.new(|cx| EditorView::new(theme, cx))
        })
    }

    #[gpui::test]
    fn edits_set_dirty_and_emit(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.doc = Document::from_file("t.txt".into(), "abc", 1);
                v.doc.move_caret(Motion::DocEnd, false);
                v.edit(cx, |d| d.insert("d"));
                assert!(v.is_dirty());
                assert_eq!(v.doc.text(), "abcd");
            });
        });
    }

    #[gpui::test]
    fn find_navigates_matches(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.doc = Document::from_file("t.txt".into(), "x y x y x", 1);
                v.toggle_find(cx);
                v.find.as_mut().unwrap().query.text = "x".to_string();
                v.refresh_matches();
                assert_eq!(v.find.as_ref().unwrap().matches.len(), 3);
                v.find_step(true, cx);
                assert!(v.doc.selection().is_some());
            });
        });
    }

    #[gpui::test]
    fn live_prefs_toggle_vim_and_indent(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            let p = Preferences {
                editor_indent_with_tabs: false,
                editor_tab_size: 3,
                editor_vim_mode: true,
                ..Default::default()
            };
            cx.set_global(GlobalPreferences(p));
        });
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.apply_prefs(cx);
                assert!(v.vim.is_some(), "vim enabled via prefs");
                assert_eq!(v.indent_unit(), "   ");

                let p = Preferences {
                    editor_vim_mode: false,
                    editor_indent_with_tabs: true,
                    ..Default::default()
                };
                cx.set_global(GlobalPreferences(p));
                v.apply_prefs(cx);
                assert!(v.vim.is_none(), "vim disabled via prefs");
                assert_eq!(v.indent_unit(), "\t");
            });
        });
    }

    #[test]
    fn wrap_rows_math() {
        let w = Wrap { cols: 10 };
        assert_eq!(w.rows(0), 1);
        assert_eq!(w.rows(1), 1);
        assert_eq!(w.rows(10), 1);
        assert_eq!(w.rows(11), 2);
        assert_eq!(w.rows(25), 3);
        assert_eq!(Wrap { cols: 0 }.rows(999), 1, "wrap off = one row");
    }

    #[gpui::test]
    fn soft_wrap_navigation_crosses_visual_rows(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                // 45 visible columns: (408 - 48 gutter) / 8.
                v.metrics = (8.0, 18.0);
                v.gutter_width = 48.0;
                v.bounds = Some(gpui::Bounds {
                    origin: gpui::Point::default(),
                    size: gpui::Size {
                        width: px(408.0),
                        height: px(360.0),
                    },
                });
                v.prefs.editor_word_wrap = true;
                assert_eq!(v.wrap_cols(), 45);

                let long: String = "x".repeat(120);
                v.doc = Document::from_file("t.txt".into(), &long, 1);
                v.doc.set_caret(Position::new(0, 10), false);

                assert!(v.wrap_vertical(true, false, cx));
                assert_eq!(v.doc.cursor.column, 55, "down one visual row = +cols");
                assert!(v.wrap_vertical(true, false, cx));
                assert_eq!(v.doc.cursor.column, 100);
                assert!(v.wrap_vertical(false, false, cx));
                assert_eq!(v.doc.cursor.column, 55, "up one visual row = -cols");

                assert!(v.wrap_horizontal(false, false, cx));
                assert_eq!(v.doc.cursor.column, 45, "home = start of visual row");
                assert!(v.wrap_horizontal(true, false, cx));
                assert_eq!(v.doc.cursor.column, 90, "end = end of visual row");
            });
        });
    }

    #[gpui::test]
    fn soft_wrap_disabled_falls_back_to_logical(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.prefs.editor_word_wrap = false;
                assert_eq!(v.wrap_cols(), 0);
                assert!(!v.wrap_vertical(true, false, cx));
                assert!(!v.wrap_horizontal(true, false, cx));
            });
        });
    }

    #[gpui::test]
    fn vim_mode_routes_keys_and_edits(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.doc = Document::from_file("t.txt".into(), "hello world", 1);
                v.vim = Some(Vim::default());
                // `dw` deletes the first word.
                v.handle_vim(VimKey::Char('d'), cx);
                v.handle_vim(VimKey::Char('w'), cx);
                assert_eq!(v.doc.text(), "world");
                assert_eq!(v.vim.as_ref().unwrap().mode(), VimMode::Normal);
                // `:q` asks the workspace to close the tab.
                for k in [VimKey::Char(':'), VimKey::Char('q'), VimKey::Enter] {
                    v.handle_vim(k, cx);
                }
            });
        });
    }
}
