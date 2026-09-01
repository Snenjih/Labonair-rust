//! Self-contained Vim emulation layer over [`Document`] (T06-003).
//!
//! Labonair's web build used the CodeMirror `@replit/codemirror-vim` addon.
//! GPUI has no equivalent drop-in and the crates that exist (`vim`,
//! `helix`-derived) are tightly bound to their own buffer types, so the port
//! implements a compact modal state machine directly on top of the framework
//! -free [`Document`] editing model. It covers the commands Labonair users
//! actually reach for: modal switching, counted motions, operator + motion
//! composition (`d3w`, `ciw`-style doubled operators), character search,
//! visual / visual-line, the common single-key edits, register yank / paste,
//! `/` `?` `n` `N` search wired to [`crate::search`], and an ex command line
//! (`:w` `:q` `:wq` `:e` `:noh` `:s` `:set`).
//!
//! Every command drives `Document`'s public API, so undo/redo, the dirty
//! baseline and syntax invalidation all keep working unchanged. `Document`'s
//! caret helpers break history coalescing, so each operator application forms
//! its own undo unit while a stretch of insert-mode typing stays one unit.

use crate::buffer::Position;
use crate::document::Document;
use crate::search::{find_all, SearchQuery};

/// The editor mode. `Operator*` is implicit (tracked via `pending_op`); the
/// public mode never reports operator-pending separately — it stays `Normal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
    VisualLine,
    /// Ex command line (`:`), forward search (`/`) or backward search (`?`).
    Command,
}

impl VimMode {
    fn is_visual(self) -> bool {
        matches!(self, VimMode::Visual | VimMode::VisualLine)
    }
}

/// Vim options fed from the app preferences (Phase 12) — see [`VimOptions`]
/// defaults for the out-of-the-box values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VimOptions {
    pub number: bool,
    pub relativenumber: bool,
    pub hlsearch: bool,
    pub incsearch: bool,
    pub smartcase: bool,
    pub expandtab: bool,
    pub tabstop: usize,
    pub shiftwidth: usize,
}

impl Default for VimOptions {
    fn default() -> Self {
        Self {
            number: true,
            relativenumber: false,
            hlsearch: true,
            incsearch: true,
            smartcase: true,
            expandtab: true,
            tabstop: 4,
            shiftwidth: 4,
        }
    }
}

/// A key handed to [`Vim::on_key`]. The GPUI view translates keystrokes into
/// this alphabet (arrow keys become `h`/`j`/`k`/`l`, etc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimKey {
    Char(char),
    Esc,
    Enter,
    Backspace,
    Tab,
    /// Ctrl-R — redo.
    Redo,
}

/// Side effects the hosting view must carry out after a key was handled.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct VimResponse {
    /// The command was recognised and consumed (stop key propagation).
    pub handled: bool,
    /// `:w` / `:wq` — save the document.
    pub save: bool,
    /// `:q` / `:wq` — close the editor.
    pub quit: bool,
    /// `:e` — reload the document from disk.
    pub reload: bool,
}

impl VimResponse {
    fn ok() -> Self {
        Self {
            handled: true,
            ..Default::default()
        }
    }
}

#[derive(Clone, Default)]
struct Register {
    text: String,
    linewise: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchDir {
    Fwd,
    Back,
}

enum Line {
    Fwd,
    Back,
}

/// The modal state machine. One instance per editor view; dropped when Vim
/// mode is switched off.
pub struct Vim {
    mode: VimMode,
    pub options: VimOptions,
    count: usize,
    pending_op: Option<char>,
    pending_g: bool,
    pending_replace: bool,
    pending_find: Option<char>,
    last_find: Option<(char, char)>,
    register: Register,
    /// Anchor line for visual-line mode (visual char mode uses `doc.anchor`).
    vline_anchor: usize,
    cmdline: String,
    cmd_kind: Option<CmdKind>,
    last_search: Option<(String, SearchDir)>,
    /// `hlsearch` matches for the active pattern (view reads this to paint).
    pub search_matches: Vec<crate::search::Match>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmdKind {
    Ex,
    Search(SearchDir),
}

enum MotionKind {
    /// Target caret, end not included in an operator range.
    Exclusive(Position),
    /// Target caret, the character at the target *is* included.
    Inclusive(Position),
    /// Whole lines `first..=last`.
    Linewise(usize, usize),
}

impl Default for Vim {
    fn default() -> Self {
        Self::new(VimOptions::default())
    }
}

impl Vim {
    pub fn new(options: VimOptions) -> Self {
        Self {
            mode: VimMode::Normal,
            options,
            count: 0,
            pending_op: None,
            pending_g: false,
            pending_replace: false,
            pending_find: None,
            last_find: None,
            register: Register::default(),
            vline_anchor: 0,
            cmdline: String::new(),
            cmd_kind: None,
            last_search: None,
            search_matches: Vec::new(),
        }
    }

    pub fn mode(&self) -> VimMode {
        self.mode
    }

    pub fn command_line(&self) -> Option<(&str, &str)> {
        match self.cmd_kind {
            Some(CmdKind::Ex) => Some((":", &self.cmdline)),
            Some(CmdKind::Search(SearchDir::Fwd)) => Some(("/", &self.cmdline)),
            Some(CmdKind::Search(SearchDir::Back)) => Some(("?", &self.cmdline)),
            None => None,
        }
    }

    /// A short mode label for the status line (`-- INSERT --`, …).
    pub fn status(&self) -> String {
        match self.mode {
            VimMode::Normal => {
                let mut s = String::new();
                if let Some(op) = self.pending_op {
                    s.push(op);
                }
                if self.count > 0 {
                    s.push_str(&self.count.to_string());
                }
                s
            }
            VimMode::Insert => "-- INSERT --".into(),
            VimMode::Visual => "-- VISUAL --".into(),
            VimMode::VisualLine => "-- VISUAL LINE --".into(),
            VimMode::Command => self
                .command_line()
                .map(|(p, t)| format!("{p}{t}"))
                .unwrap_or_default(),
        }
    }

    // ── Entry point ─────────────────────────────────────────────────────────

    pub fn on_key(&mut self, doc: &mut Document, key: VimKey) -> VimResponse {
        match self.mode {
            VimMode::Insert => self.insert_key(doc, key),
            VimMode::Command => self.command_key(doc, key),
            VimMode::Normal | VimMode::Visual | VimMode::VisualLine => self.normal_key(doc, key),
        }
    }

    // ── Insert mode ─────────────────────────────────────────────────────────

    fn insert_key(&mut self, doc: &mut Document, key: VimKey) -> VimResponse {
        match key {
            VimKey::Esc => {
                self.leave_insert(doc);
                VimResponse::ok()
            }
            VimKey::Enter => {
                doc.insert("\n");
                VimResponse::ok()
            }
            VimKey::Backspace => {
                doc.backspace();
                VimResponse::ok()
            }
            VimKey::Tab => {
                if self.options.expandtab {
                    let n = self.options.tabstop.max(1);
                    doc.insert(&" ".repeat(n));
                } else {
                    doc.insert("\t");
                }
                VimResponse::ok()
            }
            VimKey::Char(c) => {
                doc.insert(&c.to_string());
                VimResponse::ok()
            }
            VimKey::Redo => VimResponse::ok(),
        }
    }

    fn leave_insert(&mut self, doc: &mut Document) {
        self.mode = VimMode::Normal;
        // Vim nudges the caret one column left when leaving insert.
        if doc.cursor.column > 0 {
            doc.set_caret(Position::new(doc.cursor.line, doc.cursor.column - 1), false);
        }
        self.clamp_normal(doc);
    }

    // ── Command line ────────────────────────────────────────────────────────

    fn command_key(&mut self, doc: &mut Document, key: VimKey) -> VimResponse {
        match key {
            VimKey::Esc => {
                self.cmdline.clear();
                self.cmd_kind = None;
                self.mode = VimMode::Normal;
                VimResponse::ok()
            }
            VimKey::Backspace => {
                if self.cmdline.pop().is_none() {
                    self.cmd_kind = None;
                    self.mode = VimMode::Normal;
                }
                VimResponse::ok()
            }
            VimKey::Char(c) => {
                self.cmdline.push(c);
                VimResponse::ok()
            }
            VimKey::Enter => {
                let text = std::mem::take(&mut self.cmdline);
                let kind = self.cmd_kind.take();
                self.mode = VimMode::Normal;
                match kind {
                    Some(CmdKind::Ex) => self.run_ex(doc, &text),
                    Some(CmdKind::Search(dir)) => {
                        if !text.is_empty() {
                            self.last_search = Some((text.clone(), dir));
                        }
                        self.run_search(doc, dir == SearchDir::Fwd, true);
                        VimResponse::ok()
                    }
                    None => VimResponse::ok(),
                }
            }
            VimKey::Tab | VimKey::Redo => VimResponse::ok(),
        }
    }

    fn run_ex(&mut self, doc: &mut Document, cmd: &str) -> VimResponse {
        let cmd = cmd.trim();
        let mut r = VimResponse::ok();

        // `:set opt opt2` — apply Vim options.
        if let Some(rest) = cmd.strip_prefix("set ").or_else(|| cmd.strip_prefix("se ")) {
            for tok in rest.split_whitespace() {
                self.apply_set(tok);
            }
            return r;
        }

        // Substitution: `:s/a/b/[g]` (current line) or `:%s/a/b/[g]` (whole file).
        let (whole_file, body) = match cmd.strip_prefix('%') {
            Some(b) => (true, b),
            None => (false, cmd),
        };
        if let Some(spec) = body.strip_prefix("s/") {
            let parts: Vec<&str> = spec.splitn(3, '/').collect();
            if parts.len() >= 2 {
                let query = SearchQuery {
                    text: parts[0].to_string(),
                    case_sensitive: self.case_sensitive(parts[0]),
                    whole_word: false,
                };
                let global = parts.get(2).map(|f| f.contains('g')).unwrap_or(false);
                if whole_file {
                    doc.replace_all(&query, parts[1]);
                } else {
                    let line = doc.cursor.line;
                    let src = doc.buffer.line(line).to_string();
                    let out = src.replacen(parts[0], parts[1], if global { usize::MAX } else { 1 });
                    if out != src {
                        let len = src.chars().count();
                        self.range_delete(doc, Position::new(line, 0), Position::new(line, len));
                        doc.set_caret(Position::new(line, 0), false);
                        doc.insert(&out);
                        doc.set_caret(Position::new(line, 0), false);
                    }
                }
            }
            return r;
        }

        for token in cmd.split_whitespace() {
            match token {
                "w" | "write" => r.save = true,
                "q" | "q!" | "quit" => r.quit = true,
                "wq" | "wq!" | "x" | "xit" => {
                    r.save = true;
                    r.quit = true;
                }
                "e" | "e!" | "edit" => r.reload = true,
                "noh" | "nohl" | "nohlsearch" => self.search_matches.clear(),
                _ => {}
            }
        }
        r
    }

    fn apply_set(&mut self, token: &str) {
        let o = &mut self.options;
        if let Some((k, v)) = token.split_once('=') {
            if let Ok(n) = v.parse::<usize>() {
                match k {
                    "tabstop" | "ts" => o.tabstop = n.max(1),
                    "shiftwidth" | "sw" => o.shiftwidth = n.max(1),
                    _ => {}
                }
            }
            return;
        }
        let (name, on) = match token.strip_prefix("no") {
            Some(rest) => (rest, false),
            None => (token.trim_end_matches('!'), true),
        };
        let toggle = token.ends_with('!');
        let set = |field: &mut bool| {
            *field = if toggle { !*field } else { on };
        };
        match name {
            "number" | "nu" => set(&mut o.number),
            "relativenumber" | "rnu" => set(&mut o.relativenumber),
            "hlsearch" | "hls" => set(&mut o.hlsearch),
            "incsearch" | "is" => set(&mut o.incsearch),
            "smartcase" | "scs" => set(&mut o.smartcase),
            "expandtab" | "et" => set(&mut o.expandtab),
            _ => {}
        }
    }

    // ── Normal / visual ────────────────────────────────────────────────────

    fn normal_key(&mut self, doc: &mut Document, key: VimKey) -> VimResponse {
        let c = match key {
            VimKey::Esc => {
                self.reset_pending();
                if self.mode.is_visual() {
                    doc.set_caret(doc.cursor, false);
                    self.mode = VimMode::Normal;
                }
                return VimResponse::ok();
            }
            VimKey::Redo => {
                doc.redo();
                return VimResponse::ok();
            }
            VimKey::Enter => '\r',
            VimKey::Backspace => 'h',
            VimKey::Tab => return VimResponse::default(),
            VimKey::Char(c) => c,
        };

        if self.pending_replace {
            self.pending_replace = false;
            if c != '\r' {
                self.replace_char(doc, c);
            }
            return VimResponse::ok();
        }
        if let Some(op_first) = self.pending_find.take() {
            self.last_find = Some((op_first, c));
            self.apply_char_search(doc, op_first, c);
            return VimResponse::ok();
        }
        if self.pending_g {
            self.pending_g = false;
            return self.handle_g(doc, c);
        }

        // Count accumulation (leading 0 is the "line start" motion).
        if c.is_ascii_digit() && !(c == '0' && self.count == 0) {
            self.count = self.count.saturating_mul(10) + (c as usize - '0' as usize);
            return VimResponse::ok();
        }

        match c {
            'i' | 'a' | 'I' | 'A' | 'o' | 'O' | 's' | 'S' | 'C' if self.pending_op.is_none() => {
                self.enter_insert(doc, c);
                VimResponse::ok()
            }
            'v' => {
                self.toggle_visual(doc, VimMode::Visual);
                VimResponse::ok()
            }
            'V' => {
                self.toggle_visual(doc, VimMode::VisualLine);
                VimResponse::ok()
            }
            ':' if !self.mode.is_visual() => {
                self.cmd_kind = Some(CmdKind::Ex);
                self.mode = VimMode::Command;
                VimResponse::ok()
            }
            '/' => {
                self.cmd_kind = Some(CmdKind::Search(SearchDir::Fwd));
                self.mode = VimMode::Command;
                VimResponse::ok()
            }
            '?' => {
                self.cmd_kind = Some(CmdKind::Search(SearchDir::Back));
                self.mode = VimMode::Command;
                VimResponse::ok()
            }
            'n' | 'N' => {
                if let Some((_, dir)) = self.last_search {
                    let fwd = (dir == SearchDir::Fwd) == (c == 'n');
                    self.run_search(doc, fwd, false);
                }
                VimResponse::ok()
            }
            'd' | 'c' | 'y' if self.mode.is_visual() => {
                self.visual_operate(doc, c);
                VimResponse::ok()
            }
            'x' if self.mode.is_visual() => {
                self.visual_operate(doc, 'd');
                VimResponse::ok()
            }
            'p' if self.mode.is_visual() => {
                self.visual_operate(doc, 'd');
                self.paste(doc, true);
                VimResponse::ok()
            }
            'd' | 'c' | 'y' => {
                let count = self.take_count();
                if self.pending_op == Some(c) {
                    // Doubled operator → linewise on `count` lines.
                    self.pending_op = None;
                    let last = (doc.cursor.line + count.saturating_sub(1))
                        .min(doc.buffer.line_count() - 1);
                    self.apply_operator(doc, c, MotionKind::Linewise(doc.cursor.line, last));
                } else {
                    self.pending_op = Some(c);
                    if count > 1 {
                        self.count = count;
                    }
                }
                VimResponse::ok()
            }
            'D' => {
                self.op_to_line_end(doc, 'd');
                VimResponse::ok()
            }
            'Y' => {
                let l = doc.cursor.line;
                self.apply_operator(doc, 'y', MotionKind::Linewise(l, l));
                VimResponse::ok()
            }
            'g' => {
                self.pending_g = true;
                VimResponse::ok()
            }
            'r' => {
                self.pending_replace = true;
                VimResponse::ok()
            }
            'x' => {
                let n = self.take_count() as isize;
                self.delete_chars(doc, n);
                VimResponse::ok()
            }
            'X' => {
                let n = self.take_count() as isize;
                self.delete_chars(doc, -n);
                VimResponse::ok()
            }
            '~' => {
                self.toggle_case(doc);
                VimResponse::ok()
            }
            'J' => {
                let n = self.take_count().max(2);
                self.join_lines(doc, n);
                VimResponse::ok()
            }
            'p' => {
                self.paste(doc, true);
                VimResponse::ok()
            }
            'P' => {
                self.paste(doc, false);
                VimResponse::ok()
            }
            'u' => {
                doc.undo();
                VimResponse::ok()
            }
            'f' | 'F' | 't' | 'T' => {
                self.pending_find = Some(c);
                VimResponse::ok()
            }
            ';' | ',' => {
                if let Some((fc, target)) = self.last_find {
                    let fc = if c == ',' { flip_find(fc) } else { fc };
                    self.apply_char_search(doc, fc, target);
                }
                VimResponse::ok()
            }
            _ => {
                // Treat as a motion, possibly under an operator.
                let count = self.take_count();
                // `cw` / `cW` behave like `ce` / `cE` (Vim quirk).
                let c = match (self.pending_op, c) {
                    (Some('c'), 'w') => 'e',
                    (Some('c'), 'W') => 'E',
                    _ => c,
                };
                match self.resolve_motion(doc, c, count) {
                    Some(m) => {
                        if let Some(op) = self.pending_op.take() {
                            self.apply_operator(doc, op, m);
                        } else {
                            self.move_to(doc, m);
                        }
                        VimResponse::ok()
                    }
                    None => {
                        self.reset_pending();
                        VimResponse::default()
                    }
                }
            }
        }
    }

    fn handle_g(&mut self, doc: &mut Document, c: char) -> VimResponse {
        let count = self.take_count();
        match c {
            'g' => {
                let line = if count > 0 {
                    (count - 1).min(doc.buffer.line_count() - 1)
                } else {
                    0
                };
                let m = MotionKind::Linewise(doc.cursor.line.min(line), doc.cursor.line.max(line));
                if let Some(op) = self.pending_op.take() {
                    self.apply_operator(doc, op, m);
                } else {
                    let col = first_non_blank(doc, line);
                    doc.set_caret(Position::new(line, col), self.mode.is_visual());
                }
                VimResponse::ok()
            }
            'e' => {
                let target = word_end_back(doc, doc.cursor);
                self.finish_motion(doc, MotionKind::Inclusive(target));
                VimResponse::ok()
            }
            _ => {
                self.reset_pending();
                VimResponse::default()
            }
        }
    }

    // ── Insert entry variants ──────────────────────────────────────────────

    fn enter_insert(&mut self, doc: &mut Document, c: char) {
        let cur = doc.cursor;
        let line_len = doc.buffer.line_len(cur.line);
        match c {
            'i' => {}
            'a' if line_len > 0 => {
                doc.set_caret(
                    Position::new(cur.line, (cur.column + 1).min(line_len)),
                    false,
                );
            }
            'a' => {}
            'I' => {
                let col = first_non_blank(doc, cur.line);
                doc.set_caret(Position::new(cur.line, col), false);
            }
            'A' => doc.set_caret(Position::new(cur.line, line_len), false),
            'o' => {
                doc.set_caret(Position::new(cur.line, line_len), false);
                doc.insert("\n");
            }
            'O' => {
                doc.set_caret(Position::new(cur.line, 0), false);
                doc.insert("\n");
                doc.set_caret(Position::new(cur.line, 0), false);
            }
            's' => {
                self.delete_chars(doc, self.count.max(1) as isize);
            }
            'S' | 'C' => {
                let l = cur.line;
                if c == 'S' {
                    self.range_delete(doc, Position::new(l, 0), Position::new(l, line_len));
                } else {
                    self.range_delete(doc, cur, Position::new(l, line_len));
                }
            }
            _ => {}
        }
        self.count = 0;
        self.mode = VimMode::Insert;
    }

    fn op_to_line_end(&mut self, doc: &mut Document, op: char) {
        let cur = doc.cursor;
        let end = Position::new(cur.line, doc.buffer.line_len(cur.line));
        self.apply_operator(doc, op, MotionKind::Exclusive(end));
    }

    // ── Motions ────────────────────────────────────────────────────────────

    fn resolve_motion(&mut self, doc: &Document, c: char, count: usize) -> Option<MotionKind> {
        let n = count.max(1);
        let cur = doc.cursor;
        let last_line = doc.buffer.line_count() - 1;
        Some(match c {
            'h' => {
                let col = cur.column.saturating_sub(n);
                MotionKind::Exclusive(Position::new(cur.line, col))
            }
            'l' | ' ' => {
                let max = doc.buffer.line_len(cur.line);
                MotionKind::Exclusive(Position::new(cur.line, (cur.column + n).min(max)))
            }
            'j' | '\r' => {
                let line = (cur.line + n).min(last_line);
                MotionKind::Linewise(cur.line, line)
            }
            'k' => {
                let line = cur.line.saturating_sub(n);
                MotionKind::Linewise(line, cur.line)
            }
            '0' => MotionKind::Exclusive(Position::new(cur.line, 0)),
            '^' => MotionKind::Exclusive(Position::new(cur.line, first_non_blank(doc, cur.line))),
            '$' => {
                let line = (cur.line + n - 1).min(last_line);
                MotionKind::Inclusive(Position::new(
                    line,
                    doc.buffer.line_len(line).saturating_sub(1),
                ))
            }
            'G' => {
                let line = if count > 0 {
                    (count - 1).min(last_line)
                } else {
                    last_line
                };
                MotionKind::Linewise(cur.line.min(line), cur.line.max(line))
            }
            'w' | 'W' => {
                let mut p = cur;
                for _ in 0..n {
                    p = word_forward(doc, p, c == 'W');
                }
                MotionKind::Exclusive(p)
            }
            'b' | 'B' => {
                let mut p = cur;
                for _ in 0..n {
                    p = word_back(doc, p, c == 'B');
                }
                MotionKind::Exclusive(p)
            }
            'e' | 'E' => {
                let mut p = cur;
                for _ in 0..n {
                    p = word_end(doc, p, c == 'E');
                }
                MotionKind::Inclusive(p)
            }
            '{' => MotionKind::Exclusive(paragraph(doc, cur, Line::Back)),
            '}' => MotionKind::Exclusive(paragraph(doc, cur, Line::Fwd)),
            '%' => MotionKind::Inclusive(match_bracket(doc, cur)?),
            _ => return None,
        })
    }

    fn finish_motion(&mut self, doc: &mut Document, m: MotionKind) {
        if let Some(op) = self.pending_op.take() {
            self.apply_operator(doc, op, m);
        } else {
            self.move_to(doc, m);
        }
    }

    fn move_to(&mut self, doc: &mut Document, m: MotionKind) {
        let extend = self.mode == VimMode::Visual;
        match m {
            MotionKind::Exclusive(p) | MotionKind::Inclusive(p) => {
                doc.set_caret(p, extend);
            }
            MotionKind::Linewise(a, b) => {
                // Whichever end differs from the current line is the target.
                let target = if b != doc.cursor.line { b } else { a };
                let col = doc.cursor.column.min(doc.buffer.line_len(target));
                doc.set_caret(Position::new(target, col), extend);
            }
        }
        self.clamp_normal(doc);
    }

    fn apply_char_search(&mut self, doc: &mut Document, kind: char, target: char) {
        let cur = doc.cursor;
        let chars: Vec<char> = doc.buffer.line(cur.line).chars().collect();
        let found = match kind {
            'f' | 't' => {
                let start = cur.column + 1;
                (start..chars.len()).find(|&i| chars[i] == target)
            }
            'F' | 'T' => (0..cur.column).rev().find(|&i| chars[i] == target),
            _ => None,
        };
        if let Some(mut idx) = found {
            if kind == 't' {
                idx = idx.saturating_sub(1);
            }
            if kind == 'T' {
                idx += 1;
            }
            let inclusive = matches!(kind, 'f' | 't');
            let m = if inclusive {
                MotionKind::Inclusive(Position::new(cur.line, idx))
            } else {
                MotionKind::Exclusive(Position::new(cur.line, idx))
            };
            self.finish_motion(doc, m);
        } else {
            self.pending_op = None;
        }
    }

    // ── Operators ──────────────────────────────────────────────────────────

    fn apply_operator(&mut self, doc: &mut Document, op: char, m: MotionKind) {
        let cur = doc.cursor;
        let (start, end, linewise) = match m {
            MotionKind::Exclusive(p) => {
                let (a, b) = order(cur, p);
                (a, b, false)
            }
            MotionKind::Inclusive(p) => {
                let (a, b) = order(cur, p);
                (a, next_char_pos(doc, b), false)
            }
            MotionKind::Linewise(a, b) => {
                let start = Position::new(a, 0);
                let end = if b + 1 < doc.buffer.line_count() {
                    Position::new(b + 1, 0)
                } else {
                    Position::new(b, doc.buffer.line_len(b))
                };
                (start, end, true)
            }
        };

        let text = self.range_text(doc, start, end);
        self.register = Register {
            text: if linewise && !text.ends_with('\n') {
                format!("{text}\n")
            } else {
                text
            },
            linewise,
        };

        match op {
            'y' => {
                doc.set_caret(start, false);
            }
            'd' => {
                self.range_delete(doc, start, end);
                if linewise {
                    let line = start.line.min(doc.buffer.line_count() - 1);
                    doc.set_caret(Position::new(line, first_non_blank(doc, line)), false);
                }
            }
            'c' => {
                if linewise {
                    // Change lines: delete the block, then leave one empty line
                    // and drop into insert at its start.
                    let l0 = start.line;
                    let multiline = end.line > start.line;
                    self.range_delete(doc, start, end);
                    let at = l0.min(doc.buffer.line_count().saturating_sub(1));
                    if multiline {
                        doc.set_caret(Position::new(at, 0), false);
                        doc.insert("\n");
                    }
                    doc.set_caret(Position::new(at, 0), false);
                } else {
                    self.range_delete(doc, start, end);
                }
                self.mode = VimMode::Insert;
            }
            _ => {}
        }
        self.count = 0;
        self.clamp_normal(doc);
    }

    fn visual_operate(&mut self, doc: &mut Document, op: char) {
        let (start, end, linewise) = self.visual_range(doc);
        let text = self.range_text(doc, start, end);
        self.register = Register {
            text: if linewise && !text.ends_with('\n') {
                format!("{text}\n")
            } else {
                text
            },
            linewise,
        };
        self.mode = VimMode::Normal;
        match op {
            'y' => doc.set_caret(start, false),
            'd' => {
                self.range_delete(doc, start, end);
            }
            'c' => {
                self.range_delete(doc, start, end);
                self.mode = VimMode::Insert;
            }
            _ => {}
        }
        self.clamp_normal(doc);
    }

    fn visual_range(&self, doc: &Document) -> (Position, Position, bool) {
        if self.mode == VimMode::VisualLine {
            let a = self.vline_anchor.min(doc.cursor.line);
            let b = self.vline_anchor.max(doc.cursor.line);
            let start = Position::new(a, 0);
            let end = if b + 1 < doc.buffer.line_count() {
                Position::new(b + 1, 0)
            } else {
                Position::new(b, doc.buffer.line_len(b))
            };
            (start, end, true)
        } else {
            let anchor = doc.anchor.unwrap_or(doc.cursor);
            let (a, b) = order(anchor, doc.cursor);
            (a, next_char_pos(doc, b), false)
        }
    }

    // ── Small edits ────────────────────────────────────────────────────────

    fn delete_chars(&mut self, doc: &mut Document, count: isize) {
        let cur = doc.cursor;
        if count >= 0 {
            let end = (cur.column + count as usize).min(doc.buffer.line_len(cur.line));
            if end > cur.column {
                let text = self.range_text(doc, cur, Position::new(cur.line, end));
                self.register = Register {
                    text,
                    linewise: false,
                };
                self.range_delete(doc, cur, Position::new(cur.line, end));
            }
        } else {
            let start = cur.column.saturating_sub((-count) as usize);
            if start < cur.column {
                self.range_delete(doc, Position::new(cur.line, start), cur);
            }
        }
        self.clamp_normal(doc);
    }

    fn replace_char(&mut self, doc: &mut Document, c: char) {
        let cur = doc.cursor;
        if doc.buffer.line_len(cur.line) == 0 {
            return;
        }
        let n = self.take_count().max(1);
        let end = (cur.column + n).min(doc.buffer.line_len(cur.line));
        if end == cur.column {
            return;
        }
        self.range_delete(doc, cur, Position::new(cur.line, end));
        doc.set_caret(cur, false);
        doc.insert(&c.to_string().repeat(end - cur.column));
        doc.set_caret(Position::new(cur.line, end.saturating_sub(1)), false);
    }

    fn toggle_case(&mut self, doc: &mut Document) {
        let cur = doc.cursor;
        let chars: Vec<char> = doc.buffer.line(cur.line).chars().collect();
        let n = self.take_count();
        let end = (cur.column + n).min(chars.len());
        if end <= cur.column {
            return;
        }
        let flipped: String = chars[cur.column..end]
            .iter()
            .map(|c| {
                if c.is_uppercase() {
                    c.to_lowercase().next().unwrap_or(*c)
                } else {
                    c.to_uppercase().next().unwrap_or(*c)
                }
            })
            .collect();
        self.range_delete(doc, cur, Position::new(cur.line, end));
        doc.set_caret(cur, false);
        doc.insert(&flipped);
        doc.set_caret(
            Position::new(cur.line, end.min(doc.buffer.line_len(cur.line))),
            false,
        );
    }

    fn join_lines(&mut self, doc: &mut Document, count: usize) {
        let line = doc.cursor.line;
        for _ in 0..count.saturating_sub(1) {
            if line + 1 >= doc.buffer.line_count() {
                break;
            }
            let cur_len = doc.buffer.line_len(line);
            let next_full = doc.buffer.line_len(line + 1);
            let next_trimmed = doc.buffer.line(line + 1).trim_start().chars().count();
            let ws = next_full - next_trimmed;
            self.range_delete(
                doc,
                Position::new(line, cur_len),
                Position::new(line + 1, ws),
            );
            if cur_len > 0 && next_trimmed > 0 {
                doc.set_caret(Position::new(line, cur_len), false);
                doc.insert(" ");
            }
            doc.set_caret(Position::new(line, cur_len), false);
        }
    }

    fn paste(&mut self, doc: &mut Document, after: bool) {
        if self.register.text.is_empty() {
            return;
        }
        let reg = self.register.clone();
        let cur = doc.cursor;
        if reg.linewise {
            let body = reg.text.trim_end_matches('\n');
            if after {
                let end = doc.buffer.line_len(cur.line);
                doc.set_caret(Position::new(cur.line, end), false);
                doc.insert(&format!("\n{body}"));
                doc.set_caret(
                    Position::new(cur.line + 1, first_non_blank(doc, cur.line + 1)),
                    false,
                );
            } else {
                doc.set_caret(Position::new(cur.line, 0), false);
                doc.insert(&format!("{body}\n"));
                doc.set_caret(
                    Position::new(cur.line, first_non_blank(doc, cur.line)),
                    false,
                );
            }
        } else {
            let at = if after && doc.buffer.line_len(cur.line) > 0 {
                Position::new(cur.line, cur.column + 1)
            } else {
                cur
            };
            doc.set_caret(at, false);
            doc.insert(&reg.text);
            if doc.cursor.column > 0 {
                doc.set_caret(Position::new(doc.cursor.line, doc.cursor.column - 1), false);
            }
        }
        self.clamp_normal(doc);
    }

    // ── Visual toggles ─────────────────────────────────────────────────────

    fn toggle_visual(&mut self, doc: &mut Document, target: VimMode) {
        if self.mode == target {
            doc.set_caret(doc.cursor, false);
            self.mode = VimMode::Normal;
        } else {
            self.mode = target;
            if target == VimMode::VisualLine {
                self.vline_anchor = doc.cursor.line;
            } else {
                doc.set_caret(doc.cursor, false);
                doc.anchor = Some(doc.cursor);
            }
        }
    }

    // ── Search ─────────────────────────────────────────────────────────────

    fn case_sensitive(&self, pattern: &str) -> bool {
        self.options.smartcase && pattern.chars().any(|c| c.is_uppercase())
    }

    fn run_search(&mut self, doc: &mut Document, forward: bool, _initial: bool) {
        let Some((pattern, _)) = self.last_search.clone() else {
            return;
        };
        let query = SearchQuery {
            text: pattern.clone(),
            case_sensitive: self.case_sensitive(&pattern),
            whole_word: false,
        };
        let matches = find_all(&doc.buffer, &query);
        if self.options.hlsearch {
            self.search_matches = matches.clone();
        }
        if matches.is_empty() {
            return;
        }
        let from = doc.cursor;
        let idx = if forward {
            matches.iter().position(|m| m.start > from).unwrap_or(0)
        } else {
            matches
                .iter()
                .rposition(|m| m.start < from)
                .unwrap_or(matches.len() - 1)
        };
        doc.set_caret(matches[idx].start, false);
    }

    // ── Range helpers over Document's public API ───────────────────────────

    fn range_text(&self, doc: &Document, a: Position, b: Position) -> String {
        let mut clone = doc.buffer.clone();
        clone.delete(a, b)
    }

    fn range_delete(&self, doc: &mut Document, a: Position, b: Position) {
        // Establish a real selection a..b (order-insensitive), then delete it in
        // a single history step. `set_caret` breaks undo coalescing, so each
        // operator application forms its own undo unit.
        doc.set_caret(a, false);
        doc.set_caret(b, true);
        doc.backspace();
    }

    // ── Bookkeeping ────────────────────────────────────────────────────────

    fn take_count(&mut self) -> usize {
        std::mem::take(&mut self.count)
    }

    fn reset_pending(&mut self) {
        self.count = 0;
        self.pending_op = None;
        self.pending_g = false;
        self.pending_replace = false;
        self.pending_find = None;
    }

    /// Keep the caret on a real character in normal mode (never one past EOL).
    fn clamp_normal(&self, doc: &mut Document) {
        if self.mode == VimMode::Normal || self.mode == VimMode::VisualLine {
            let len = doc.buffer.line_len(doc.cursor.line);
            let max = len.saturating_sub(if self.mode == VimMode::Normal { 1 } else { 0 });
            if len > 0 && doc.cursor.column > max {
                doc.set_caret(Position::new(doc.cursor.line, max), self.mode.is_visual());
            }
        }
    }
}

// ── Free helpers ───────────────────────────────────────────────────────────

fn flip_find(c: char) -> char {
    match c {
        'f' => 'F',
        'F' => 'f',
        't' => 'T',
        'T' => 't',
        other => other,
    }
}

fn order(a: Position, b: Position) -> (Position, Position) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn next_char_pos(doc: &Document, p: Position) -> Position {
    if p.column < doc.buffer.line_len(p.line) {
        Position::new(p.line, p.column + 1)
    } else if p.line + 1 < doc.buffer.line_count() {
        Position::new(p.line + 1, 0)
    } else {
        p
    }
}

fn first_non_blank(doc: &Document, line: usize) -> usize {
    doc.buffer
        .line(line)
        .chars()
        .position(|c| !c.is_whitespace())
        .unwrap_or(0)
}

#[derive(PartialEq, Clone, Copy)]
enum Class {
    Blank,
    Word,
    Punct,
}

fn class(c: char, big: bool) -> Class {
    if c.is_whitespace() {
        Class::Blank
    } else if big || c.is_alphanumeric() || c == '_' {
        Class::Word
    } else {
        Class::Punct
    }
}

/// Char offset stream that treats `\n` as a single blank character.
fn to_offset(doc: &Document, p: Position) -> usize {
    let mut off = 0;
    for l in 0..p.line {
        off += doc.buffer.line_len(l) + 1;
    }
    off + p.column
}

fn from_offset(doc: &Document, mut off: usize) -> Position {
    let mut line = 0;
    let last = doc.buffer.line_count() - 1;
    loop {
        let len = doc.buffer.line_len(line);
        if off <= len || line == last {
            return Position::new(line, off.min(len));
        }
        off -= len + 1;
        line += 1;
    }
}

fn char_at(doc: &Document, off: usize) -> Option<char> {
    let p = from_offset(doc, off);
    let len = doc.buffer.line_len(p.line);
    if p.column < len {
        doc.buffer.line(p.line).chars().nth(p.column)
    } else if p.line + 1 < doc.buffer.line_count() {
        Some('\n')
    } else {
        None
    }
}

fn total_offset(doc: &Document) -> usize {
    let last = doc.buffer.line_count() - 1;
    to_offset(doc, Position::new(last, doc.buffer.line_len(last)))
}

fn word_forward(doc: &Document, p: Position, big: bool) -> Position {
    let end = total_offset(doc);
    let mut o = to_offset(doc, p);
    let Some(start_c) = char_at(doc, o) else {
        return p;
    };
    let start_class = class(start_c, big);
    if start_class != Class::Blank {
        while o < end {
            match char_at(doc, o) {
                Some(c) if class(c, big) == start_class && c != '\n' => o += 1,
                _ => break,
            }
        }
    }
    while o < end {
        match char_at(doc, o) {
            Some(c) if c.is_whitespace() => o += 1,
            _ => break,
        }
    }
    from_offset(doc, o.min(end))
}

fn word_back(doc: &Document, p: Position, big: bool) -> Position {
    let mut o = to_offset(doc, p);
    if o == 0 {
        return p;
    }
    o -= 1;
    while o > 0 {
        match char_at(doc, o) {
            Some(c) if c.is_whitespace() => o -= 1,
            _ => break,
        }
    }
    let Some(c0) = char_at(doc, o) else {
        return from_offset(doc, o);
    };
    let cl = class(c0, big);
    while o > 0 {
        match char_at(doc, o - 1) {
            Some(c) if class(c, big) == cl && !c.is_whitespace() => o -= 1,
            _ => break,
        }
    }
    from_offset(doc, o)
}

fn word_end(doc: &Document, p: Position, big: bool) -> Position {
    let end = total_offset(doc);
    let mut o = to_offset(doc, p);
    o += 1;
    while o < end {
        match char_at(doc, o) {
            Some(c) if c.is_whitespace() => o += 1,
            _ => break,
        }
    }
    if o >= end {
        return from_offset(doc, end);
    }
    let cl = class(char_at(doc, o).unwrap(), big);
    while o + 1 < end {
        match char_at(doc, o + 1) {
            Some(c) if class(c, big) == cl && !c.is_whitespace() => o += 1,
            _ => break,
        }
    }
    from_offset(doc, o)
}

fn word_end_back(doc: &Document, p: Position) -> Position {
    let mut o = to_offset(doc, p).saturating_sub(1);
    while o > 0 && char_at(doc, o).map(|c| c.is_whitespace()).unwrap_or(false) {
        o -= 1;
    }
    from_offset(doc, o)
}

fn paragraph(doc: &Document, p: Position, dir: Line) -> Position {
    let last = doc.buffer.line_count() - 1;
    let mut line = p.line;
    let blank = |l: usize| doc.buffer.line(l).trim().is_empty();
    match dir {
        Line::Fwd => {
            if line < last {
                line += 1;
            }
            while line < last && !blank(line) {
                line += 1;
            }
        }
        Line::Back => {
            line = line.saturating_sub(1);
            while line > 0 && !blank(line) {
                line -= 1;
            }
        }
    }
    Position::new(line, 0)
}

fn match_bracket(doc: &Document, p: Position) -> Option<Position> {
    const PAIRS: &[(char, char)] = &[('(', ')'), ('[', ']'), ('{', '}')];
    let line: Vec<char> = doc.buffer.line(p.line).chars().collect();
    let (col, open, close, forward) = (p.column..line.len()).find_map(|i| {
        PAIRS.iter().find_map(|&(o, c)| {
            if line[i] == o {
                Some((i, o, c, true))
            } else if line[i] == c {
                Some((i, c, o, false))
            } else {
                None
            }
        })
    })?;
    let start = to_offset(doc, Position::new(p.line, col));
    let end_off = total_offset(doc);
    let mut depth = 0i32;
    if forward {
        let mut o = start;
        while o < end_off {
            match char_at(doc, o) {
                Some(c) if c == open => depth += 1,
                Some(c) if c == close => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(from_offset(doc, o));
                    }
                }
                _ => {}
            }
            o += 1;
        }
    } else {
        let mut o = start + 1;
        while o > 0 {
            o -= 1;
            match char_at(doc, o) {
                Some(c) if c == open => depth += 1,
                Some(c) if c == close => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(from_offset(doc, o));
                    }
                }
                _ => {}
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::from_file("t.rs".into(), text, 0)
    }

    fn keys(vim: &mut Vim, d: &mut Document, s: &str) {
        for ch in s.chars() {
            let k = match ch {
                '\u{1b}' => VimKey::Esc,
                '\n' => VimKey::Enter,
                _ => VimKey::Char(ch),
            };
            vim.on_key(d, k);
        }
    }

    #[test]
    fn mode_switching() {
        let mut v = Vim::default();
        let mut d = doc("hello");
        assert_eq!(v.mode(), VimMode::Normal);
        keys(&mut v, &mut d, "i");
        assert_eq!(v.mode(), VimMode::Insert);
        keys(&mut v, &mut d, "X\u{1b}");
        assert_eq!(v.mode(), VimMode::Normal);
        assert_eq!(d.text(), "Xhello");
    }

    #[test]
    fn motions_with_counts() {
        let mut v = Vim::default();
        let mut d = doc("one two three four");
        keys(&mut v, &mut d, "3w");
        assert_eq!(d.cursor, Position::new(0, 14));
        keys(&mut v, &mut d, "0");
        assert_eq!(d.cursor.column, 0);
        keys(&mut v, &mut d, "$");
        assert_eq!(d.cursor.column, 17);
    }

    #[test]
    fn vertical_motion_counts() {
        let mut v = Vim::default();
        let mut d = doc("a\nb\nc\nd\ne");
        keys(&mut v, &mut d, "3j");
        assert_eq!(d.cursor.line, 3);
        keys(&mut v, &mut d, "k");
        assert_eq!(d.cursor.line, 2);
        keys(&mut v, &mut d, "gg");
        assert_eq!(d.cursor.line, 0);
        keys(&mut v, &mut d, "G");
        assert_eq!(d.cursor.line, 4);
    }

    #[test]
    fn delete_word_and_line() {
        let mut v = Vim::default();
        let mut d = doc("foo bar baz");
        keys(&mut v, &mut d, "dw");
        assert_eq!(d.text(), "bar baz");
        keys(&mut v, &mut d, "dd");
        assert_eq!(d.text(), "");
    }

    #[test]
    fn dd_multiline_and_paste() {
        let mut v = Vim::default();
        let mut d = doc("l1\nl2\nl3");
        keys(&mut v, &mut d, "dd");
        assert_eq!(d.text(), "l2\nl3");
        keys(&mut v, &mut d, "p");
        assert_eq!(d.text(), "l2\nl1\nl3");
    }

    #[test]
    fn change_word_enters_insert() {
        let mut v = Vim::default();
        let mut d = doc("foo bar");
        keys(&mut v, &mut d, "cw");
        assert_eq!(v.mode(), VimMode::Insert);
        keys(&mut v, &mut d, "baz\u{1b}");
        assert_eq!(d.text(), "baz bar");
    }

    #[test]
    fn x_deletes_char_with_count() {
        let mut v = Vim::default();
        let mut d = doc("abcdef");
        keys(&mut v, &mut d, "3x");
        assert_eq!(d.text(), "def");
    }

    #[test]
    fn replace_char() {
        let mut v = Vim::default();
        let mut d = doc("cat");
        keys(&mut v, &mut d, "rb");
        assert_eq!(d.text(), "bat");
    }

    #[test]
    fn yank_and_paste_charwise() {
        let mut v = Vim::default();
        let mut d = doc("hello");
        keys(&mut v, &mut d, "yw");
        keys(&mut v, &mut d, "$p");
        assert_eq!(d.text(), "hellohello");
    }

    #[test]
    fn visual_delete() {
        let mut v = Vim::default();
        let mut d = doc("abcdef");
        keys(&mut v, &mut d, "vlld");
        assert_eq!(d.text(), "def");
    }

    #[test]
    fn visual_line_delete() {
        let mut v = Vim::default();
        let mut d = doc("l1\nl2\nl3");
        keys(&mut v, &mut d, "Vjd");
        assert_eq!(d.text(), "l3");
    }

    #[test]
    fn find_char_motion() {
        let mut v = Vim::default();
        let mut d = doc("a.b.c.d");
        keys(&mut v, &mut d, "f.");
        assert_eq!(d.cursor.column, 1);
        keys(&mut v, &mut d, ";");
        assert_eq!(d.cursor.column, 3);
        keys(&mut v, &mut d, "dt.");
        assert_eq!(d.text(), "a.b.d");
    }

    #[test]
    fn open_line_below() {
        let mut v = Vim::default();
        let mut d = doc("first");
        keys(&mut v, &mut d, "onext\u{1b}");
        assert_eq!(d.text(), "first\nnext");
    }

    #[test]
    fn undo_is_one_unit_per_command() {
        let mut v = Vim::default();
        let mut d = doc("foo bar baz");
        keys(&mut v, &mut d, "dw");
        keys(&mut v, &mut d, "dw");
        assert_eq!(d.text(), "baz");
        keys(&mut v, &mut d, "u");
        assert_eq!(d.text(), "bar baz");
        keys(&mut v, &mut d, "u");
        assert_eq!(d.text(), "foo bar baz");
    }

    #[test]
    fn cmdline_write_and_quit() {
        let mut v = Vim::default();
        let mut d = doc("x");
        for k in [
            VimKey::Char(':'),
            VimKey::Char('w'),
            VimKey::Char('q'),
            VimKey::Enter,
        ] {
            let r = v.on_key(&mut d, k);
            if k == VimKey::Enter {
                assert!(r.save && r.quit);
            }
        }
        assert_eq!(v.mode(), VimMode::Normal);
    }

    #[test]
    fn set_option_toggles() {
        let mut v = Vim::default();
        let mut d = doc("x");
        assert!(v.options.number);
        keys(&mut v, &mut d, ":set nonumber\n");
        assert!(!v.options.number);
        keys(&mut v, &mut d, ":set tabstop=2\n");
        assert_eq!(v.options.tabstop, 2);
    }

    #[test]
    fn search_moves_to_match() {
        let mut v = Vim::default();
        let mut d = doc("alpha beta gamma beta");
        keys(&mut v, &mut d, "/beta\n");
        assert_eq!(d.cursor, Position::new(0, 6));
        keys(&mut v, &mut d, "n");
        assert_eq!(d.cursor, Position::new(0, 17));
        keys(&mut v, &mut d, "N");
        assert_eq!(d.cursor, Position::new(0, 6));
    }

    #[test]
    fn match_bracket_percent() {
        let mut v = Vim::default();
        let mut d = doc("foo(bar, baz)");
        keys(&mut v, &mut d, "%");
        assert_eq!(d.cursor.column, 12);
        keys(&mut v, &mut d, "%");
        assert_eq!(d.cursor.column, 3);
    }

    #[test]
    fn join_lines() {
        let mut v = Vim::default();
        let mut d = doc("hello\n  world");
        keys(&mut v, &mut d, "J");
        assert_eq!(d.text(), "hello world");
    }
}
