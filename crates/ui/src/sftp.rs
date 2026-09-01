//! Dual-pane SFTP file browser (T08-001).
//!
//! Ported from `reference-src/src/modules/sftp/` (`SftpPane` / `SftpToolbar` /
//! `SftpContextMenu` / `PropertiesDialog` / `VirtualizedFileList`). The React
//! version is a resizable two-pane browser — local filesystem on the left,
//! the connected host's filesystem (over SFTP) on the right — with an address
//! bar + up/reload/hidden-toggle per pane, inline rename / new file / new
//! folder, a right-click context menu, a permissions (chmod/chown) dialog and
//! a properties dialog. The transfer *queue* UI lives in
//! [`crate::transfers`] (T08-002); this module only *triggers* transfers
//! (drag between panes + context-menu upload/download) via `SftpEvent`.
//!
//! All backend work is in-process through
//! [`labonair_backend::modules::ssh::sftp`] (`sftp_read_dir`, `sftp_rename`,
//! `sftp_delete`, `sftp_mkdir`, `sftp_create_file`, `sftp_chmod`,
//! `sftp_chown`, `sftp_calculate_size`, `prepare_remote_edit`) and
//! [`labonair_backend::modules::fs`] for the local pane — no Tauri IPC.
//!
//! Deviations from the reference:
//! * Rows render into a plain `overflow_y_scroll` column, not
//!   `@tanstack/react-virtual` (same call the [`crate::explorer`] port makes).
//! * The two panes are a fixed 50/50 split rather than a draggable
//!   `ResizablePanelGroup`.
//! * Drag between panes has no drop-target pane highlight yet (the reference
//!   dims the hovered pane).
//! * Remote-edit conflict detection (remote file changed underneath the temp
//!   copy) is not implemented — the backend `save_remote_edit` is a plain
//!   overwrite. Documented as a follow-up.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Window,
};
use tokio::runtime::Handle as TokioHandle;

use labonair_backend::modules::fs::{mutate, tree};
use labonair_backend::modules::sftp::connection::sftp_connect;
use labonair_backend::modules::ssh::sftp as backend_sftp;
use labonair_backend::App as Backend;

use crate::theme::ThemeStore;

// ── pure helpers (unit-tested) ─────────────────────────────────────────────

/// Parent of a POSIX path. `/` and `""` map to `/`; a single-segment
/// absolute path maps to `/`. Mirrors the reference `parentPath`.
pub fn parent_path(p: &str) -> String {
    if p == "/" || p.is_empty() {
        return "/".to_string();
    }
    let trimmed = p.strip_suffix('/').unwrap_or(p);
    match trimmed.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(i) => trimmed[..i].to_string(),
    }
}

/// Joins `dir` and `name` with exactly one separator.
pub fn join_path(dir: &str, name: &str) -> String {
    if dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}

/// Trims and rejects empty / `.` / `..` / names containing a path separator.
/// Returns `None` when the name is invalid. Mirrors the reference
/// `sanitizeEntryName` (without the `allowNested` option — the SFTP panes
/// never create nested chains inline).
pub fn sanitize_entry_name(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() || t == "." || t == ".." {
        return None;
    }
    if t.contains('/') || t.contains('\\') {
        return None;
    }
    Some(t.to_string())
}

/// A 9-char `rwxr-xr-x`-style permission string → 3-digit octal string.
/// Mirrors the reference `permStringToOctal`.
pub fn perm_string_to_octal(perm: &str) -> String {
    let weights = [
        0o400, 0o200, 0o100, 0o040, 0o020, 0o010, 0o004, 0o002, 0o001,
    ];
    let chars: Vec<char> = perm.chars().take(9).collect();
    let mut n = 0u32;
    for (i, w) in weights.iter().enumerate() {
        if chars.get(i).is_some_and(|c| *c != '-') {
            n |= w;
        }
    }
    format!("{n:03o}")
}

/// Formats a UNIX epoch (seconds) as `YYYY-MM-DD HH:MM` UTC. `0` → `—`.
/// Self-contained (no `chrono`) via the well-known days→civil algorithm.
pub fn format_epoch(secs: i64) -> String {
    if secs <= 0 {
        return "\u{2014}".to_string();
    }
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m) = (rem / 3600, (rem % 3600) / 60);
    // days→civil (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    format!("{year:04}-{month:02}-{d:02} {h:02}:{m:02}")
}

/// Human-readable byte size.
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if b < KB * KB {
        format!("{:.1} KB", b / KB)
    } else if b < KB * KB * KB {
        format!("{:.2} MB", b / (KB * KB))
    } else {
        format!("{:.2} GB", b / (KB * KB * KB))
    }
}

// ── data model ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Local,
    Remote,
}

/// One directory entry, provider-agnostic (local or remote).
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified_at: i64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<String>,
    /// 9-char permission string (remote only; empty for local).
    pub permissions: String,
}

impl Entry {
    fn from_remote(n: backend_sftp::FileNode) -> Self {
        Self {
            name: n.name,
            path: n.path,
            size: n.size,
            modified_at: n.modified_at,
            is_dir: n.is_dir,
            is_symlink: n.is_symlink,
            symlink_target: n.symlink_target,
            permissions: n.permissions,
        }
    }
}

/// Sorts entries dirs-first, then case-insensitively by name.
pub fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditKind {
    Rename,
    NewFile,
    NewDir,
}

struct EditSlot {
    kind: EditKind,
    /// Original path for a rename.
    orig: Option<String>,
}

struct Pane {
    path: String,
    entries: Vec<Entry>,
    show_hidden: bool,
    loading: bool,
    error: Option<String>,
    selected: Option<String>,
    /// Bumped on every load; a stale async response compares and bails.
    generation: u64,
    edit: Option<EditSlot>,
    edit_buffer: String,
    /// Address-bar edit in progress.
    path_editing: bool,
    path_buffer: String,
}

impl Pane {
    fn new(path: String) -> Self {
        Self {
            path,
            entries: Vec::new(),
            show_hidden: false,
            loading: false,
            error: None,
            selected: None,
            generation: 0,
            edit: None,
            edit_buffer: String::new(),
            path_editing: false,
            path_buffer: String::new(),
        }
    }

    fn visible(&self) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|e| self.show_hidden || !e.name.starts_with('.'))
            .collect()
    }
}

/// Connection lifecycle of the remote pane.
enum Conn {
    Connecting,
    Ready,
    Error(String),
}

/// A pending chmod/chown edit.
struct PermDialog {
    path: String,
    name: String,
    octal: String,
    owner: String,
    group: String,
    field: PermField,
    error: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PermField {
    Octal,
    Owner,
    Group,
}

/// The properties dialog for a single entry.
struct PropsDialog {
    entry: Entry,
    calculated_size: Option<String>,
    calculating: bool,
}

/// Open right-click menu: `(side, entry path)`.
struct Menu {
    side: Side,
    path: String,
    is_dir: bool,
    /// Two-click delete arm state (mirrors the reference).
    confirming_delete: bool,
}

/// SFTP view → workspace notifications.
pub enum SftpEvent {
    /// Open a local file in the editor.
    OpenLocalFile(String),
    /// Download a remote file to a temp copy and open it for editing.
    OpenRemoteFile {
        session_id: String,
        remote_path: String,
        host_id: String,
    },
    /// Queue a transfer (T08-002). `direction` is `"upload"` or `"download"`;
    /// folders are handled recursively by the backend worker.
    Enqueue {
        session_id: String,
        src_path: String,
        dest_path: String,
        direction: &'static str,
    },
}

/// Payload of a pointer-drag of one or more rows from one pane to the other
/// (T08-002). Dropping on the opposite pane queues an upload/download.
#[derive(Clone)]
pub struct SftpDrag {
    pub from: Side,
    pub paths: Vec<String>,
}

pub struct SftpView {
    backend: Backend,
    tokio: TokioHandle,
    theme: Entity<ThemeStore>,
    /// The SSH/SFTP session id (shared registry key).
    session_id: String,
    host_id: String,
    host_label: SharedString,
    conn: Conn,
    local: Pane,
    remote: Pane,
    menu: Option<Menu>,
    perm: Option<PermDialog>,
    props: Option<PropsDialog>,
    focus: FocusHandle,
    edit_focus: FocusHandle,
    dialog_focus: FocusHandle,
}

impl EventEmitter<SftpEvent> for SftpView {}

impl Focusable for SftpView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl SftpView {
    pub fn new(
        backend: Backend,
        tokio: TokioHandle,
        theme: Entity<ThemeStore>,
        session_id: String,
        host_id: String,
        host_label: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());
        let mut this = Self {
            backend,
            tokio,
            theme,
            session_id,
            host_id,
            host_label: host_label.into(),
            conn: Conn::Connecting,
            local: Pane::new(home),
            remote: Pane::new("/".to_string()),
            menu: None,
            perm: None,
            props: None,
            focus: cx.focus_handle(),
            edit_focus: cx.focus_handle(),
            dialog_focus: cx.focus_handle(),
        };
        this.load_local(cx);
        this.connect(cx);
        this
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    fn pane(&mut self, side: Side) -> &mut Pane {
        match side {
            Side::Local => &mut self.local,
            Side::Remote => &mut self.remote,
        }
    }

    // ── connection ─────────────────────────────────────────────────────────

    fn connect(&mut self, cx: &mut Context<Self>) {
        self.conn = Conn::Connecting;
        let app = self.backend.clone();
        let (sid, hid) = (self.session_id.clone(), self.host_id.clone());
        let jh = self.tokio.spawn(async move {
            sftp_connect(
                sid,
                hid,
                None,
                None,
                &app.ssh,
                &app.trust,
                &app.db,
                &app.secrets,
                app.clone(),
            )
            .await
            .map_err(|e| e.to_string())
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                match res {
                    Ok(()) => {
                        this.conn = Conn::Ready;
                        this.load_remote(cx);
                    }
                    Err(e) => this.conn = Conn::Error(e),
                }
                cx.notify();
            });
        })
        .detach();
    }

    // ── loading ────────────────────────────────────────────────────────────

    fn load_local(&mut self, cx: &mut Context<Self>) {
        self.local.generation += 1;
        let generation = self.local.generation;
        self.local.loading = true;
        self.local.error = None;
        let path = self.local.path.clone();
        let show_hidden = self.local.show_hidden;
        cx.spawn(async move |this, cx| {
            let path2 = path.clone();
            let res = cx
                .background_executor()
                .spawn(async move { tree::list_dir_entries_sync(&path2, true) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.local.generation != generation {
                    return;
                }
                this.local.loading = false;
                match res {
                    Ok(list) => {
                        let mut entries: Vec<Entry> = list
                            .into_iter()
                            .map(|d| Entry {
                                name: d.name.clone(),
                                path: join_path(&path, &d.name),
                                size: d.size,
                                modified_at: (d.mtime / 1000) as i64,
                                is_dir: matches!(d.kind, tree::EntryKind::Dir),
                                is_symlink: matches!(d.kind, tree::EntryKind::Symlink),
                                symlink_target: None,
                                permissions: String::new(),
                            })
                            .collect();
                        sort_entries(&mut entries);
                        this.local.entries = entries;
                    }
                    Err(e) => {
                        this.local.entries.clear();
                        this.local.error = Some(e);
                    }
                }
                let _ = show_hidden;
                cx.notify();
            });
        })
        .detach();
    }

    fn load_remote(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.conn, Conn::Ready) {
            return;
        }
        self.remote.generation += 1;
        let generation = self.remote.generation;
        self.remote.loading = true;
        self.remote.error = None;
        let app = self.backend.clone();
        let (sid, path) = (self.session_id.clone(), self.remote.path.clone());
        let jh = self.tokio.spawn(async move {
            backend_sftp::sftp_read_dir(sid, path, &app.ssh, app.clone())
                .await
                .map_err(|e| e.to_string())
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if this.remote.generation != generation {
                    return;
                }
                this.remote.loading = false;
                match res {
                    Ok(list) => {
                        let mut entries: Vec<Entry> =
                            list.into_iter().map(Entry::from_remote).collect();
                        sort_entries(&mut entries);
                        this.remote.entries = entries;
                    }
                    Err(e) => {
                        this.remote.entries.clear();
                        this.remote.error = Some(e);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn reload(&mut self, side: Side, cx: &mut Context<Self>) {
        match side {
            Side::Local => self.load_local(cx),
            Side::Remote => self.load_remote(cx),
        }
    }

    // ── navigation ─────────────────────────────────────────────────────────

    fn navigate(&mut self, side: Side, path: String, cx: &mut Context<Self>) {
        let pane = self.pane(side);
        pane.path = path;
        pane.selected = None;
        pane.edit = None;
        pane.path_editing = false;
        self.reload(side, cx);
    }

    fn go_up(&mut self, side: Side, cx: &mut Context<Self>) {
        let up = match side {
            Side::Local => {
                let p = std::path::Path::new(&self.local.path);
                p.parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| self.local.path.clone())
            }
            Side::Remote => parent_path(&self.remote.path),
        };
        self.navigate(side, up, cx);
    }

    fn toggle_hidden(&mut self, side: Side, cx: &mut Context<Self>) {
        let pane = self.pane(side);
        pane.show_hidden = !pane.show_hidden;
        cx.notify();
    }

    fn activate(&mut self, side: Side, entry: &Entry, dbl: bool, cx: &mut Context<Self>) {
        self.pane(side).selected = Some(entry.path.clone());
        if entry.is_dir {
            if dbl {
                let target = entry
                    .symlink_target
                    .clone()
                    .filter(|_| entry.is_symlink)
                    .unwrap_or_else(|| entry.path.clone());
                self.navigate(side, target, cx);
            }
        } else if dbl {
            match side {
                Side::Local => cx.emit(SftpEvent::OpenLocalFile(entry.path.clone())),
                Side::Remote => cx.emit(SftpEvent::OpenRemoteFile {
                    session_id: self.session_id.clone(),
                    remote_path: entry.path.clone(),
                    host_id: self.host_id.clone(),
                }),
            }
        }
        cx.notify();
    }

    // ── mutations ──────────────────────────────────────────────────────────

    fn start_edit(&mut self, side: Side, kind: EditKind, cx: &mut Context<Self>) {
        self.menu = None;
        let (buffer, orig) = match kind {
            EditKind::Rename => {
                let sel = self.pane(side).selected.clone();
                let Some(sel) = sel else { return };
                let name = std::path::Path::new(&sel)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                (name, Some(sel))
            }
            _ => (String::new(), None),
        };
        let pane = self.pane(side);
        pane.edit_buffer = buffer;
        pane.edit = Some(EditSlot { kind, orig });
        cx.notify();
    }

    fn cancel_edit(&mut self, side: Side, cx: &mut Context<Self>) {
        self.pane(side).edit = None;
        cx.notify();
    }

    fn commit_edit(&mut self, side: Side, cx: &mut Context<Self>) {
        let (kind, orig, raw, dir) = {
            let pane = self.pane(side);
            let Some(slot) = pane.edit.as_ref() else {
                return;
            };
            (
                slot.kind,
                slot.orig.clone(),
                pane.edit_buffer.clone(),
                pane.path.clone(),
            )
        };
        let Some(name) = sanitize_entry_name(&raw) else {
            self.pane(side).error = Some("Invalid name".to_string());
            cx.notify();
            return;
        };
        self.pane(side).edit = None;

        let app = self.backend.clone();
        let sid = self.session_id.clone();
        match (side, kind) {
            (Side::Remote, EditKind::Rename) => {
                let Some(old) = orig else { return };
                let new = join_path(&parent_path(&old), &name);
                let jh = self.tokio.spawn(async move {
                    backend_sftp::sftp_rename(sid, old, new, &app.ssh, app.clone())
                        .await
                        .map_err(|e| e.to_string())
                });
                self.after_remote_op(jh, side, cx);
            }
            (Side::Remote, EditKind::NewFile) => {
                let path = join_path(&dir, &name);
                let jh = self.tokio.spawn(async move {
                    backend_sftp::sftp_create_file(sid, path, &app.ssh, app.clone())
                        .await
                        .map_err(|e| e.to_string())
                });
                self.after_remote_op(jh, side, cx);
            }
            (Side::Remote, EditKind::NewDir) => {
                let path = join_path(&dir, &name);
                let jh = self.tokio.spawn(async move {
                    backend_sftp::sftp_mkdir(sid, path, Some(false), &app.ssh, app.clone())
                        .await
                        .map_err(|e| e.to_string())
                });
                self.after_remote_op(jh, side, cx);
            }
            (Side::Local, kind) => {
                let dir2 = dir.clone();
                let name2 = name.clone();
                let orig2 = orig.clone();
                cx.spawn(async move |this, cx| {
                    let res = cx
                        .background_executor()
                        .spawn(async move {
                            match kind {
                                EditKind::Rename => {
                                    let old = orig2.unwrap_or_default();
                                    let new = join_path(&parent_path(&old), &name2);
                                    mutate::rename_sync(&old, &new)
                                }
                                EditKind::NewFile => {
                                    mutate::create_file_sync(&join_path(&dir2, &name2))
                                }
                                EditKind::NewDir => {
                                    mutate::create_dir_sync(&join_path(&dir2, &name2))
                                }
                            }
                        })
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        if let Err(e) = res {
                            this.local.error = Some(e);
                        }
                        this.load_local(cx);
                    });
                })
                .detach();
            }
        }
    }

    fn after_remote_op(
        &mut self,
        jh: tokio::task::JoinHandle<Result<(), String>>,
        side: Side,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = res {
                    this.pane(side).error = Some(e);
                }
                this.reload(side, cx);
            });
        })
        .detach();
    }

    fn delete(&mut self, side: Side, path: String, cx: &mut Context<Self>) {
        self.menu = None;
        let app = self.backend.clone();
        let sid = self.session_id.clone();
        match side {
            Side::Remote => {
                let jh = self.tokio.spawn(async move {
                    backend_sftp::sftp_delete(sid, vec![path], &app.ssh, app.clone())
                        .await
                        .map_err(|e| e.to_string())
                });
                self.after_remote_op(jh, side, cx);
            }
            Side::Local => {
                cx.spawn(async move |this, cx| {
                    let res = cx
                        .background_executor()
                        .spawn(async move { mutate::delete_sync(&path) })
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        if let Err(e) = res {
                            this.local.error = Some(e);
                        }
                        this.load_local(cx);
                    });
                })
                .detach();
            }
        }
    }

    // ── permissions dialog ─────────────────────────────────────────────────

    fn open_perm_dialog(&mut self, entry: &Entry, cx: &mut Context<Self>) {
        self.menu = None;
        let octal = if entry.permissions.is_empty() {
            "755".to_string()
        } else {
            perm_string_to_octal(&entry.permissions)
        };
        self.perm = Some(PermDialog {
            path: entry.path.clone(),
            name: entry.name.clone(),
            octal,
            owner: String::new(),
            group: String::new(),
            field: PermField::Octal,
            error: None,
        });
        cx.notify();
    }

    fn apply_perm(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.perm.as_ref() else { return };
        let path = d.path.clone();
        let (owner, group) = (d.owner.trim().to_string(), d.group.trim().to_string());
        let octal = u32::from_str_radix(d.octal.trim(), 8).ok();
        if octal.is_none() {
            if let Some(d) = self.perm.as_mut() {
                d.error = Some("Invalid octal value".to_string());
            }
            cx.notify();
            return;
        }
        self.perm = None;
        let mode = octal.unwrap();
        let app = self.backend.clone();
        let sid = self.session_id.clone();
        let do_chown = !owner.is_empty() || !group.is_empty();
        let jh = self.tokio.spawn(async move {
            backend_sftp::sftp_chmod(sid.clone(), path.clone(), mode, &app.ssh, app.clone())
                .await
                .map_err(|e| e.to_string())?;
            if do_chown {
                backend_sftp::sftp_chown(sid, path, owner, group, &app.ssh, app.clone())
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Ok::<(), String>(())
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = res {
                    this.remote.error = Some(e);
                }
                this.load_remote(cx);
            });
        })
        .detach();
    }

    // ── properties dialog ──────────────────────────────────────────────────

    fn open_props(&mut self, entry: &Entry, cx: &mut Context<Self>) {
        self.menu = None;
        self.props = Some(PropsDialog {
            entry: entry.clone(),
            calculated_size: None,
            calculating: false,
        });
        cx.notify();
    }

    fn calc_size(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.props.as_mut() else { return };
        d.calculating = true;
        let path = d.entry.path.clone();
        let app = self.backend.clone();
        let sid = self.session_id.clone();
        let jh = self.tokio.spawn(async move {
            backend_sftp::sftp_calculate_size(sid, path, &app.ssh, app.clone())
                .await
                .map_err(|e| e.to_string())
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if let Some(d) = this.props.as_mut() {
                    d.calculating = false;
                    match res {
                        Ok(s) => d.calculated_size = Some(s),
                        Err(e) => d.calculated_size = Some(format!("error: {e}")),
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    // ── transfers (T08-002) ────────────────────────────────────────────────

    /// Queue upload/download of `src_paths` (which live on the `from` side)
    /// into the opposite pane's current directory. Folders transfer
    /// recursively (handled by the backend worker).
    fn enqueue(&mut self, from: Side, src_paths: Vec<String>, cx: &mut Context<Self>) {
        self.menu = None;
        let (dest_dir, direction) = match from {
            Side::Local => (self.remote.path.clone(), "upload"),
            Side::Remote => (self.local.path.clone(), "download"),
        };
        let session_id = self.session_id.clone();
        for src in src_paths {
            let name = src
                .rsplit(['/', '\\'])
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or("file")
                .to_string();
            cx.emit(SftpEvent::Enqueue {
                session_id: session_id.clone(),
                src_path: src,
                dest_path: join_path(&dest_dir, &name),
                direction,
            });
        }
        cx.notify();
    }

    /// Reload one pane after a transfer landed a file in it.
    pub fn reload_side(&mut self, remote: bool, cx: &mut Context<Self>) {
        self.reload(if remote { Side::Remote } else { Side::Local }, cx);
    }

    fn menu_entry(&self) -> Option<(Side, Entry)> {
        let m = self.menu.as_ref()?;
        let pane = match m.side {
            Side::Local => &self.local,
            Side::Remote => &self.remote,
        };
        pane.entries
            .iter()
            .find(|e| e.path == m.path)
            .map(|e| (m.side, e.clone()))
    }
}

// ── keyboard for inline edit / dialogs ─────────────────────────────────────

impl SftpView {
    fn on_edit_key(&mut self, side: Side, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => self.cancel_edit(side, cx),
            "enter" => self.commit_edit(side, cx),
            "backspace" => {
                self.pane(side).edit_buffer.pop();
                cx.notify();
            }
            key => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                if let Some(ch) = printable(ks, key) {
                    self.pane(side).edit_buffer.push_str(&ch);
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }

    fn on_path_key(&mut self, side: Side, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => {
                self.pane(side).path_editing = false;
                cx.notify();
            }
            "enter" => {
                let p = self.pane(side).path_buffer.clone();
                if !p.trim().is_empty() {
                    self.navigate(side, p.trim().to_string(), cx);
                }
            }
            "backspace" => {
                self.pane(side).path_buffer.pop();
                cx.notify();
            }
            key => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                if let Some(ch) = printable(ks, key) {
                    self.pane(side).path_buffer.push_str(&ch);
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }

    fn on_perm_key(&mut self, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => {
                self.perm = None;
                cx.notify();
            }
            "enter" => self.apply_perm(cx),
            "tab" => {
                if let Some(d) = self.perm.as_mut() {
                    d.field = match d.field {
                        PermField::Octal => PermField::Owner,
                        PermField::Owner => PermField::Group,
                        PermField::Group => PermField::Octal,
                    };
                }
                cx.notify();
            }
            "backspace" => {
                if let Some(d) = self.perm.as_mut() {
                    match d.field {
                        PermField::Octal => d.octal.pop(),
                        PermField::Owner => d.owner.pop(),
                        PermField::Group => d.group.pop(),
                    };
                }
                cx.notify();
            }
            key => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                if let Some(ch) = printable(ks, key) {
                    if let Some(d) = self.perm.as_mut() {
                        match d.field {
                            PermField::Octal => {
                                if ch.chars().all(|c| c.is_ascii_digit()) && d.octal.len() < 4 {
                                    d.octal.push_str(&ch);
                                }
                            }
                            PermField::Owner => d.owner.push_str(&ch),
                            PermField::Group => d.group.push_str(&ch),
                        }
                    }
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }
}

fn printable(ks: &gpui::Keystroke, key: &str) -> Option<String> {
    ks.key_char
        .clone()
        .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
        .or_else(|| (key.chars().count() == 1).then(|| key.to_string()))
}

// ── rendering ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Colors {
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    accent: gpui::Hsla,
    border: gpui::Hsla,
    card: gpui::Hsla,
    bg: gpui::Hsla,
    err: gpui::Hsla,
}

impl Render for SftpView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = {
            let t = self.theme.read(cx);
            Colors {
                fg: t.foreground(),
                muted: t.muted_foreground(),
                accent: t.accent(),
                border: t.border(),
                card: t.card(),
                bg: t.background(),
                err: t.status_error(),
            }
        };

        let local = self.render_pane(Side::Local, c, cx);
        let remote = self.render_pane(Side::Remote, c, cx);

        let mut root = div()
            .id("sftp")
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .flex()
            .flex_row()
            .bg(c.bg)
            .text_color(c.fg)
            .child(div().flex_1().min_w_0().h_full().child(local))
            .child(div().w(px(1.0)).h_full().bg(c.border))
            .child(div().flex_1().min_w_0().h_full().child(remote));

        if self.menu.is_some() {
            root = root.child(self.render_menu(c, cx));
        }
        if self.perm.is_some() {
            root = root.child(self.render_perm_dialog(c, cx));
        }
        if self.props.is_some() {
            root = root.child(self.render_props_dialog(c, cx));
        }
        root
    }
}

impl SftpView {
    fn render_pane(&self, side: Side, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let pane = match side {
            Side::Local => &self.local,
            Side::Remote => &self.remote,
        };
        let title = match side {
            Side::Local => "Local".to_string(),
            Side::Remote => format!("Remote \u{00b7} {}", self.host_label),
        };

        // Toolbar
        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .h(px(30.0))
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .text_xs()
                    .text_color(c.muted)
                    .child(SharedString::from(title)),
            )
            .child(
                self.tool_btn(side, "up", "\u{2191}", c, cx, |this, side, cx| {
                    this.go_up(side, cx)
                }),
            )
            .child(
                self.tool_btn(side, "reload", "\u{21BB}", c, cx, |this, side, cx| {
                    this.reload(side, cx)
                }),
            )
            .child(self.tool_btn(
                side,
                "hidden",
                if pane.show_hidden {
                    "\u{25C9}"
                } else {
                    "\u{25CB}"
                },
                c,
                cx,
                |this, side, cx| this.toggle_hidden(side, cx),
            ))
            .child(self.render_path_bar(side, pane, c, cx));

        // Body
        let body: gpui::AnyElement = if side == Side::Remote {
            match &self.conn {
                Conn::Connecting => text_center("Connecting\u{2026}", c.muted),
                Conn::Error(e) => self.render_conn_error(e, c, cx),
                Conn::Ready => self.render_list(side, pane, c, cx),
            }
        } else {
            self.render_list(side, pane, c, cx)
        };

        let bhandler_side = side;
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(toolbar)
            .when_some(pane.error.clone(), |el, e| {
                el.child(
                    div()
                        .px_2()
                        .py(px(2.0))
                        .text_xs()
                        .bg(c.card)
                        .text_color(c.err)
                        .child(SharedString::from(e)),
                )
            })
            .child(
                div()
                    .id(match side {
                        Side::Local => "sftp-local-body",
                        Side::Remote => "sftp-remote-body",
                    })
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, _, _w, cx| {
                            // Right-click on empty space → menu anchored to the
                            // directory itself (new file / new folder only).
                            let path = match bhandler_side {
                                Side::Local => this.local.path.clone(),
                                Side::Remote => this.remote.path.clone(),
                            };
                            this.menu = Some(Menu {
                                side: bhandler_side,
                                path,
                                is_dir: true,
                                confirming_delete: false,
                            });
                            cx.notify();
                        }),
                    )
                    .on_drop(cx.listener(move |this, d: &SftpDrag, _w, cx| {
                        if d.from != bhandler_side {
                            this.enqueue(d.from, d.paths.clone(), cx);
                        }
                    }))
                    .child(body),
            )
            .into_any_element()
    }

    fn render_path_bar(
        &self,
        side: Side,
        pane: &Pane,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if pane.path_editing {
            div()
                .id(match side {
                    Side::Local => "sftp-local-path",
                    Side::Remote => "sftp-remote-path",
                })
                .track_focus(&self.edit_focus)
                .flex_1()
                .px_1()
                .text_xs()
                .font_family("monospace")
                .rounded_sm()
                .border_1()
                .border_color(c.accent)
                .bg(c.card)
                .child(SharedString::from(format!("{}\u{2502}", pane.path_buffer)))
                .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _w, cx| {
                    this.on_path_key(side, ev, cx)
                }))
                .into_any_element()
        } else {
            div()
                .id(match side {
                    Side::Local => "sftp-local-pathview",
                    Side::Remote => "sftp-remote-pathview",
                })
                .flex_1()
                .px_1()
                .text_xs()
                .font_family("monospace")
                .text_color(c.muted)
                .rounded_sm()
                .hover(|s| s.bg(c.card))
                .child(SharedString::from(pane.path.clone()))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    let p = match side {
                        Side::Local => &mut this.local,
                        Side::Remote => &mut this.remote,
                    };
                    p.path_buffer = p.path.clone();
                    p.path_editing = true;
                    window.focus(&this.edit_focus);
                    cx.notify();
                }))
                .into_any_element()
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn tool_btn(
        &self,
        side: Side,
        id: &'static str,
        glyph: &'static str,
        c: Colors,
        cx: &mut Context<Self>,
        handler: impl Fn(&mut Self, Side, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        div()
            .id(id)
            .h(px(20.0))
            .px_1()
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .text_xs()
            .text_color(c.muted)
            .hover(|s| s.bg(c.border))
            .child(glyph)
            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| handler(this, side, cx)))
    }

    fn render_list(
        &self,
        side: Side,
        pane: &Pane,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if pane.loading && pane.entries.is_empty() {
            return text_center("Loading\u{2026}", c.muted);
        }
        let mut list = div().flex().flex_col().py_1();

        if let Some(slot) = &pane.edit {
            if slot.kind != EditKind::Rename {
                list = list.child(self.render_inline_input(side, c, cx));
            }
        }

        for entry in pane.visible() {
            let is_rename = pane
                .edit
                .as_ref()
                .filter(|s| s.kind == EditKind::Rename)
                .and_then(|s| s.orig.as_deref())
                == Some(entry.path.as_str());
            if is_rename {
                list = list.child(self.render_inline_input(side, c, cx));
            } else {
                list = list.child(self.render_row(side, entry, pane, c, cx));
            }
        }
        list.into_any_element()
    }

    fn render_row(
        &self,
        side: Side,
        entry: &Entry,
        pane: &Pane,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let selected = pane.selected.as_deref() == Some(entry.path.as_str());
        let glyph = if entry.is_dir {
            "\u{1F4C1}"
        } else if entry.is_symlink {
            "\u{1F517}"
        } else {
            "\u{1F4C4}"
        };
        let id: SharedString = format!("row:{:?}:{}", side_key(side), entry.path).into();
        let e_click = entry.clone();
        let e_menu = entry.clone();
        let drag_path = entry.path.clone();
        let perm_col = entry.permissions.clone();
        let size_col = if entry.is_dir {
            String::new()
        } else {
            format_bytes(entry.size)
        };

        div()
            .id(id)
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_2()
            .py(px(2.0))
            .text_sm()
            .when(selected, |d| d.bg(c.border))
            .when(!selected, |d| d.hover(|s| s.bg(c.card)))
            .child(div().child(glyph))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(SharedString::from(entry.name.clone())),
            )
            .when(!perm_col.is_empty(), |d| {
                d.child(
                    div()
                        .text_xs()
                        .font_family("monospace")
                        .text_color(c.muted)
                        .child(SharedString::from(perm_col)),
                )
            })
            .when(!size_col.is_empty(), |d| {
                d.child(
                    div()
                        .w(px(64.0))
                        .text_xs()
                        .text_color(c.muted)
                        .child(SharedString::from(size_col)),
                )
            })
            .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                this.activate(side, &e_click, ev.click_count() >= 2, cx);
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, _, _w, cx| {
                    this.pane(side).selected = Some(e_menu.path.clone());
                    this.menu = Some(Menu {
                        side,
                        path: e_menu.path.clone(),
                        is_dir: e_menu.is_dir,
                        confirming_delete: false,
                    });
                    cx.notify();
                }),
            )
            .on_drag(
                SftpDrag {
                    from: side,
                    paths: vec![drag_path],
                },
                |_, _, _, cx| cx.new(|_| DragGhost),
            )
            .into_any_element()
    }

    fn render_inline_input(
        &self,
        side: Side,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let buffer = match side {
            Side::Local => &self.local.edit_buffer,
            Side::Remote => &self.remote.edit_buffer,
        };
        div()
            .flex()
            .flex_row()
            .items_center()
            .px_2()
            .py(px(1.0))
            .child(
                div()
                    .id("sftp-inline-input")
                    .track_focus(&self.edit_focus)
                    .flex_1()
                    .px_1()
                    .text_sm()
                    .rounded_sm()
                    .border_1()
                    .border_color(c.accent)
                    .bg(c.card)
                    .child(SharedString::from(format!("{buffer}\u{2502}")))
                    .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _w, cx| {
                        this.on_edit_key(side, ev, cx)
                    })),
            )
    }

    fn render_conn_error(&self, msg: &str, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .p_4()
            .child(
                div()
                    .text_sm()
                    .text_color(c.err)
                    .child(SharedString::from(format!("SFTP connection failed: {msg}"))),
            )
            .child(
                div()
                    .id("sftp-retry")
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .text_xs()
                    .text_color(c.accent)
                    .hover(|s| s.bg(c.border))
                    .child("Retry")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.connect(cx))),
            )
            .into_any_element()
    }

    fn render_menu(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(menu) = self.menu.as_ref() else {
            return div().into_any_element();
        };
        let side = menu.side;
        let has_entry = self.menu_entry().is_some();
        let (is_dir, is_remote) = (menu.is_dir, side == Side::Remote);
        let confirming = menu.confirming_delete;
        let single_entry = self.menu_entry().map(|(_, e)| e);

        let backdrop = div()
            .id("sftp-menu-backdrop")
            .absolute()
            .inset_0()
            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                this.menu = None;
                cx.notify();
            }));

        let item = move |id: &'static str, label: SharedString| {
            div()
                .id(id)
                .px_2()
                .py_1()
                .text_sm()
                .rounded_sm()
                .hover(|s| s.bg(c.border))
                .child(label)
        };

        let mut menu_el = div()
            .absolute()
            .top(px(36.0))
            .left(px(48.0))
            .w(px(220.0))
            .flex()
            .flex_col()
            .p_1()
            .rounded_md()
            .border_1()
            .border_color(c.border)
            .bg(c.card)
            .shadow_lg()
            .child(
                item("sftp-cm-newdir", "New Folder".into()).on_click(cx.listener(
                    move |this, _: &ClickEvent, _w, cx| this.start_edit(side, EditKind::NewDir, cx),
                )),
            )
            .child(
                item("sftp-cm-newfile", "New File".into()).on_click(cx.listener(
                    move |this, _: &ClickEvent, _w, cx| {
                        this.start_edit(side, EditKind::NewFile, cx)
                    },
                )),
            );

        if has_entry {
            menu_el = menu_el
                .child(
                    item("sftp-cm-rename", "Rename".into()).on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            this.start_edit(side, EditKind::Rename, cx)
                        },
                    )),
                )
                .child(
                    item("sftp-cm-copypath", "Copy Path".into()).on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            if let Some((_, e)) = this.menu_entry() {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(e.path));
                            }
                            this.menu = None;
                            cx.notify();
                        },
                    )),
                )
                .child(
                    item(
                        "sftp-cm-delete",
                        if confirming {
                            "Click again to delete".into()
                        } else {
                            SharedString::from("Delete\u{2026}")
                        },
                    )
                    .text_color(c.err)
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            let Some(menu) = this.menu.as_mut() else {
                                return;
                            };
                            if menu.confirming_delete {
                                let (side, path) = (menu.side, menu.path.clone());
                                this.delete(side, path, cx);
                            } else {
                                menu.confirming_delete = true;
                                cx.notify();
                            }
                        },
                    )),
                );
        }

        if has_entry {
            let transfer_label: SharedString = if is_remote {
                "Download to Local".into()
            } else {
                "Upload to Remote".into()
            };
            menu_el = menu_el.child(item("sftp-cm-transfer", transfer_label).on_click(
                cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    if let Some((s, e)) = this.menu_entry() {
                        this.enqueue(s, vec![e.path], cx);
                    }
                }),
            ));
        }

        if is_remote && has_entry {
            let e1 = single_entry.clone();
            let e2 = single_entry.clone();
            menu_el =
                menu_el
                    .child(item("sftp-cm-perm", "Permissions\u{2026}".into()).on_click(
                        cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            if let Some(e) = e1.clone() {
                                this.open_perm_dialog(&e, cx);
                            }
                        }),
                    ))
                    .child(item("sftp-cm-props", "Properties\u{2026}".into()).on_click(
                        cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            if let Some(e) = e2.clone() {
                                this.open_props(&e, cx);
                            }
                        }),
                    ));
            if !is_dir {
                let e3 = single_entry.clone();
                menu_el = menu_el.child(
                    item("sftp-cm-editremote", "Edit Remote File".into()).on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            if let Some(e) = e3.clone() {
                                cx.emit(SftpEvent::OpenRemoteFile {
                                    session_id: this.session_id.clone(),
                                    remote_path: e.path,
                                    host_id: this.host_id.clone(),
                                });
                            }
                            this.menu = None;
                            cx.notify();
                        },
                    )),
                );
            }
        }

        menu_el = menu_el.child(
            item("sftp-cm-refresh", "Refresh".into()).on_click(cx.listener(
                move |this, _: &ClickEvent, _w, cx| {
                    this.menu = None;
                    this.reload(side, cx);
                },
            )),
        );

        div()
            .absolute()
            .inset_0()
            .child(backdrop)
            .child(menu_el)
            .into_any_element()
    }

    fn render_perm_dialog(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(d) = self.perm.as_ref() else {
            return div().into_any_element();
        };
        let row = |label: &str, value: String, active: bool| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(px(56.0))
                        .text_xs()
                        .text_color(c.muted)
                        .child(SharedString::from(label.to_string())),
                )
                .child(
                    div()
                        .flex_1()
                        .px_1()
                        .text_sm()
                        .font_family("monospace")
                        .rounded_sm()
                        .border_1()
                        .border_color(if active { c.accent } else { c.border })
                        .bg(c.card)
                        .child(SharedString::from(if active {
                            format!("{value}\u{2502}")
                        } else {
                            value
                        })),
                )
        };
        overlay()
            .child(
                dialog_card("sftp-perm-dialog", c)
                    .track_focus(&self.dialog_focus)
                    .on_key_down(
                        cx.listener(|this, ev: &KeyDownEvent, _w, cx| this.on_perm_key(ev, cx)),
                    )
                    .child(div().text_sm().child(SharedString::from(format!(
                        "Permissions \u{2014} {}",
                        d.name
                    ))))
                    .child(row("Octal", d.octal.clone(), d.field == PermField::Octal))
                    .child(row("Owner", d.owner.clone(), d.field == PermField::Owner))
                    .child(row("Group", d.group.clone(), d.field == PermField::Group))
                    .child(
                        div().text_xs().text_color(c.muted).child(
                            "Tab switches field \u{00b7} Enter applies \u{00b7} Esc cancels",
                        ),
                    )
                    .when_some(d.error.clone(), |el, e| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(c.err)
                                .child(SharedString::from(e)),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(dialog_btn("perm-cancel", "Cancel", c, false).on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| {
                                    this.perm = None;
                                    cx.notify();
                                }),
                            ))
                            .child(dialog_btn("perm-apply", "Apply", c, true).on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.apply_perm(cx)),
                            )),
                    ),
            )
            .into_any_element()
    }

    fn render_props_dialog(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(d) = self.props.as_ref() else {
            return div().into_any_element();
        };
        let e = &d.entry;
        let kind = if e.is_dir {
            "Directory".to_string()
        } else if e.is_symlink {
            format!(
                "Symlink \u{2192} {}",
                e.symlink_target.as_deref().unwrap_or("?")
            )
        } else {
            "File".to_string()
        };
        let size = if e.is_dir {
            d.calculated_size
                .clone()
                .unwrap_or_else(|| "\u{2014}".to_string())
        } else {
            format_bytes(e.size)
        };
        let info =
            |label: &str, value: String| {
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(
                        div()
                            .w(px(72.0))
                            .text_xs()
                            .text_color(c.muted)
                            .child(SharedString::from(label.to_string())),
                    )
                    .child(div().flex_1().text_xs().child(SharedString::from(
                        if value.is_empty() {
                            "\u{2014}".to_string()
                        } else {
                            value
                        },
                    )))
            };
        overlay()
            .child(
                dialog_card("sftp-props-dialog", c)
                    .child(div().text_sm().child(SharedString::from(format!(
                        "Properties \u{2014} {}",
                        e.name
                    ))))
                    .child(info("Name", e.name.clone()))
                    .child(info("Path", e.path.clone()))
                    .child(info("Type", kind))
                    .child(info("Size", size))
                    .child(info("Permissions", e.permissions.clone()))
                    .child(info("Modified", format_epoch(e.modified_at)))
                    .when(e.is_dir, |el| {
                        el.child(
                            dialog_btn(
                                "props-calc",
                                if d.calculating {
                                    "Calculating\u{2026}"
                                } else {
                                    "Calculate size"
                                },
                                c,
                                false,
                            )
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.calc_size(cx)),
                            ),
                        )
                    })
                    .child(
                        dialog_btn("props-close", "Close", c, true).on_click(cx.listener(
                            |this, _: &ClickEvent, _w, cx| {
                                this.props = None;
                                cx.notify();
                            },
                        )),
                    ),
            )
            .into_any_element()
    }
}

/// Minimal drag preview — the cursor + drop-target highlighting do the work.
struct DragGhost;

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn side_key(side: Side) -> &'static str {
    match side {
        Side::Local => "L",
        Side::Remote => "R",
    }
}

fn text_center(msg: &str, color: gpui::Hsla) -> gpui::AnyElement {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(color)
        .child(SharedString::from(msg.to_string()))
        .into_any_element()
}

fn overlay() -> gpui::Div {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui::hsla(0.0, 0.0, 0.0, 0.4))
}

fn dialog_card(id: &'static str, c: Colors) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .w(px(360.0))
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(c.border)
        .bg(c.card)
        .text_color(c.fg)
        .shadow_lg()
}

fn dialog_btn(
    id: &'static str,
    label: &'static str,
    c: Colors,
    primary: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px_2()
        .py_1()
        .rounded_sm()
        .text_xs()
        .when(primary, |d| d.bg(c.accent).text_color(c.bg))
        .when(!primary, |d| d.bg(c.border).text_color(c.fg))
        .hover(|s| s.opacity(0.85))
        .child(label)
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_path_cases() {
        assert_eq!(parent_path("/"), "/");
        assert_eq!(parent_path(""), "/");
        assert_eq!(parent_path("/a"), "/");
        assert_eq!(parent_path("/a/b/c"), "/a/b");
        assert_eq!(parent_path("/a/b/"), "/a");
    }

    #[test]
    fn join_path_single_separator() {
        assert_eq!(join_path("/a", "b"), "/a/b");
        assert_eq!(join_path("/a/", "b"), "/a/b");
        assert_eq!(join_path("/", "b"), "/b");
    }

    #[test]
    fn sanitize_rejects_bad_names() {
        assert_eq!(sanitize_entry_name("  foo "), Some("foo".to_string()));
        assert_eq!(sanitize_entry_name(""), None);
        assert_eq!(sanitize_entry_name("."), None);
        assert_eq!(sanitize_entry_name(".."), None);
        assert_eq!(sanitize_entry_name("a/b"), None);
        assert_eq!(sanitize_entry_name("a\\b"), None);
    }

    #[test]
    fn perm_string_to_octal_matches_reference() {
        assert_eq!(perm_string_to_octal("rwxr-xr-x"), "755");
        assert_eq!(perm_string_to_octal("rw-r--r--"), "644");
        assert_eq!(perm_string_to_octal("---------"), "000");
        assert_eq!(perm_string_to_octal(""), "000");
        assert_eq!(perm_string_to_octal("rwxrwxrwx"), "777");
    }

    #[test]
    fn sort_entries_dirs_first_then_case_insensitive() {
        let mk = |name: &str, is_dir: bool| Entry {
            name: name.to_string(),
            path: format!("/{name}"),
            size: 0,
            modified_at: 0,
            is_dir,
            is_symlink: false,
            symlink_target: None,
            permissions: String::new(),
        };
        let mut v = vec![
            mk("Zebra", false),
            mk("apple", false),
            mk("Mango", true),
            mk("banana", true),
        ];
        sort_entries(&mut v);
        let names: Vec<&str> = v.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["banana", "Mango", "apple", "Zebra"]);
    }

    #[test]
    fn format_epoch_known_value() {
        // 2021-01-01 00:00:00 UTC
        assert_eq!(format_epoch(1_609_459_200), "2021-01-01 00:00");
        assert_eq!(format_epoch(0), "\u{2014}");
    }

    #[test]
    fn format_bytes_scales() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.00 MB");
    }

    #[test]
    fn pane_visible_respects_hidden_toggle() {
        let mut p = Pane::new("/".to_string());
        p.entries = vec![
            Entry {
                name: ".hidden".to_string(),
                path: "/.hidden".to_string(),
                size: 0,
                modified_at: 0,
                is_dir: false,
                is_symlink: false,
                symlink_target: None,
                permissions: String::new(),
            },
            Entry {
                name: "visible".to_string(),
                path: "/visible".to_string(),
                size: 0,
                modified_at: 0,
                is_dir: false,
                is_symlink: false,
                symlink_target: None,
                permissions: String::new(),
            },
        ];
        assert_eq!(p.visible().len(), 1);
        p.show_hidden = true;
        assert_eq!(p.visible().len(), 2);
    }
}
