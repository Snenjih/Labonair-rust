//! Explorer host contract and shared file-interaction value types (R07-004).
//!
//! The sidebar file explorer lives in `labonair-panel-explorer`. It needs to
//! open files, open terminals, and open previews, and it needs to know which
//! file the host currently has focused (for auto-reveal). Those are the only
//! things it needs from the surrounding application.
//!
//! Before this crate existed, `labonair-panel-explorer` held an
//! `Entity<Workspace>` and called Workspace methods directly, which coupled a
//! panel to the whole workspace feature. [`ExplorerHost`] replaces that entity
//! with a narrow set of injected callbacks. The composition root builds the
//! host from the active Workspace; Explorer never sees Workspace state.
//!
//! [`DraggedPaths`] / [`quote_paths`] / [`shell_quote`] and
//! [`is_previewable`] / [`PREVIEW_EXTENSIONS`] are plain value types shared by
//! the Explorer panel and the terminal / preview views. They moved here so
//! neither side has to depend on the other.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui::{App, Window};

type OpenFileFn = Rc<dyn Fn(String, bool, &mut Window, &mut App)>;
type OpenTerminalFn = Rc<dyn Fn(String, &mut Window, &mut App)>;
type OpenPreviewFn = Rc<dyn Fn(String, &mut Window, &mut App)>;
type ActiveFilePathFn = Rc<dyn Fn(&App) -> Option<String>>;

/// Narrow composition contract between the Explorer panel and its host.
///
/// The host (normally the active Workspace) owns tab and pane creation and
/// decides how each intent is fulfilled. Explorer only expresses intent.
#[derive(Clone)]
pub struct ExplorerHost {
    open_file: OpenFileFn,
    open_terminal_in: OpenTerminalFn,
    open_preview: OpenPreviewFn,
    active_file_path: ActiveFilePathFn,
}

impl ExplorerHost {
    /// Build a host from its four intent callbacks. The composition root wires
    /// these to the active Workspace entity.
    pub fn new(
        open_file: impl Fn(String, bool, &mut Window, &mut App) + 'static,
        open_terminal_in: impl Fn(String, &mut Window, &mut App) + 'static,
        open_preview: impl Fn(String, &mut Window, &mut App) + 'static,
        active_file_path: impl Fn(&App) -> Option<String> + 'static,
    ) -> Self {
        Self {
            open_file: Rc::new(open_file),
            open_terminal_in: Rc::new(open_terminal_in),
            open_preview: Rc::new(open_preview),
            active_file_path: Rc::new(active_file_path),
        }
    }

    /// A host that does nothing. Useful for headless views and tests that
    /// never exercise the open intents.
    pub fn disconnected() -> Self {
        Self::new(|_, _, _, _| {}, |_, _, _| {}, |_, _, _| {}, |_| None)
    }

    /// Open `path` in an editor tab. `peek` requests a transient preview tab
    /// rather than a permanent one.
    pub fn open_file(&self, path: String, peek: bool, window: &mut Window, cx: &mut App) {
        (self.open_file)(path, peek, window, cx);
    }

    /// Open a terminal tab rooted at `cwd`.
    pub fn open_terminal_in(&self, cwd: String, window: &mut Window, cx: &mut App) {
        (self.open_terminal_in)(cwd, window, cx);
    }

    /// Open `target` (a local path or URL) in a preview tab.
    pub fn open_preview(&self, target: String, window: &mut Window, cx: &mut App) {
        (self.open_preview)(target, window, cx);
    }

    /// The file path of the host's active editor, if any. Drives Explorer
    /// auto-reveal.
    pub fn active_file_path(&self, cx: &App) -> Option<String> {
        (self.active_file_path)(cx)
    }
}

/// Payload of an in-tree drag (T05-002). Pure-data drag, mirroring the
/// reference `explorerDrag` module singleton — carries the selected paths from
/// an explorer row to a drop target (a folder in the same tree, or a terminal
/// pane which inserts the quoted path).
#[derive(Clone)]
pub struct DraggedPaths {
    pub paths: Vec<PathBuf>,
}

/// Shell-quote a single path for insertion into a terminal (single-quote wrap
/// unless it is entirely "safe" characters).
pub fn shell_quote(path: &Path) -> String {
    let s = path.to_string_lossy();
    let safe = !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || "-_./=:@%+,".contains(c));
    if safe {
        s.into_owned()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Space-joined shell-quoted paths (drag-into-terminal payload).
pub fn quote_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| shell_quote(p))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extensions the explorer offers "Open in Preview" for, plus the
/// text/markdown kinds the native preview pane can additionally render.
pub const PREVIEW_EXTENSIONS: &[&str] = &[
    "html", "htm", "png", "jpg", "jpeg", "gif", "webp", "svg", "pdf", "bmp", "ico", "md",
    "markdown", "txt", "text",
];

/// Whether `path` is something the preview tab knows how to open.
pub fn is_previewable(path: &str) -> bool {
    let bare = match path.split_once("://") {
        Some((_, rest)) => rest,
        None => path,
    };
    Path::new(bare)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .is_some_and(|e| PREVIEW_EXTENSIONS.contains(&e.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_wraps_unsafe_only() {
        assert_eq!(
            shell_quote(Path::new("/home/me/file.txt")),
            "/home/me/file.txt"
        );
        assert_eq!(
            shell_quote(Path::new("/home/me/my file.txt")),
            "'/home/me/my file.txt'"
        );
        assert_eq!(shell_quote(Path::new("/a/it's")), "'/a/it'\\''s'");
    }

    #[test]
    fn quote_paths_joins_and_wraps() {
        assert_eq!(
            quote_paths(&[PathBuf::from("/a/b"), PathBuf::from("/c d")]),
            "/a/b '/c d'"
        );
    }

    #[test]
    fn is_previewable_matches_known_extensions() {
        assert!(is_previewable("/x/report.md"));
        assert!(is_previewable("/x/pic.PNG"));
        assert!(is_previewable("https://example.com/a.html"));
        assert!(!is_previewable("/x/main.rs"));
        assert!(!is_previewable("/x/Makefile"));
    }

    #[gpui::test]
    fn host_forwards_active_file_path(cx: &mut gpui::TestAppContext) {
        let host = ExplorerHost::new(
            |_, _, _, _| {},
            |_, _, _| {},
            |_, _, _| {},
            |_| Some("/tmp/active.rs".to_string()),
        );
        cx.update(|cx| {
            assert_eq!(host.active_file_path(cx).as_deref(), Some("/tmp/active.rs"));
        });

        let disconnected = ExplorerHost::disconnected();
        cx.update(|cx| assert_eq!(disconnected.active_file_path(cx), None));
    }
}
