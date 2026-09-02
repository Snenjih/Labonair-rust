//! Path bookmarks — quick jump targets for saved local and remote directory
//! paths (T12-003).
//!
//! Port of `reference-src/src/modules/bookmarks/`:
//! * `store/pathBookmarksStore.ts` — the `PathBookmark` model, `bookmarkKey`,
//!   `computeAddBookmark` (dedupe per `(host_id, path)`, label-update instead of
//!   a second entry), `computeRemoveByPath`, `isBookmarkOrphaned` (a bookmark
//!   whose host was deleted is kept, just flagged).
//! * `lib/filterBookmarksForContext.ts` — which bookmarks to show for the
//!   active context (local vs. a concrete host).
//!
//! The web app persisted this in a Tauri `Store` JSON file; here it is a plain
//! `bookmarks.json` object in the config dir, loaded at startup and rewritten
//! on every mutation. A corrupt file is treated as "no bookmarks" rather than
//! failing to load.

use serde::{Deserialize, Serialize};

use crate::modules::fs::paths::config_dir;

const BOOKMARKS_FILE: &str = "bookmarks.json";

/// A saved directory path. `host_id == None` means a local path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathBookmark {
    pub id: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub label: Option<String>,
    /// `None` = local.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub host_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct BookmarksFile {
    #[serde(default)]
    bookmarks: Vec<PathBookmark>,
}

/// Stable identity of a bookmark: `"<host_id|local>::<path>"`.
pub fn bookmark_key(host_id: Option<&str>, path: &str) -> String {
    format!("{}::{}", host_id.unwrap_or("local"), path)
}

/// A bookmark is orphaned when it points at a host that no longer exists.
/// Local bookmarks are never orphaned. Orphans are kept (inert user data), the
/// UI just flags them so they can be removed manually.
pub fn is_bookmark_orphaned(bm: &PathBookmark, host_ids: &[String]) -> bool {
    match &bm.host_id {
        None => false,
        Some(h) => !host_ids.iter().any(|id| id == h),
    }
}

/// Whether a `(host_id, path)` pair is already bookmarked.
pub fn is_bookmarked(current: &[PathBookmark], host_id: Option<&str>, path: &str) -> bool {
    let key = bookmark_key(host_id, path);
    current
        .iter()
        .any(|b| bookmark_key(b.host_id.as_deref(), &b.path) == key)
}

/// The bookmark for a `(host_id, path)` pair, if any.
pub fn find_bookmark<'a>(
    current: &'a [PathBookmark],
    host_id: Option<&str>,
    path: &str,
) -> Option<&'a PathBookmark> {
    let key = bookmark_key(host_id, path);
    current
        .iter()
        .find(|b| bookmark_key(b.host_id.as_deref(), &b.path) == key)
}

/// Pure dedupe/insert. Returns `None` when the call is a no-op (the pair is
/// already bookmarked and no new, different label was passed); otherwise the
/// next list. An existing pair updates its label rather than creating a second
/// entry for the same key.
pub fn compute_add_bookmark(
    current: &[PathBookmark],
    host_id: Option<&str>,
    path: &str,
    label: Option<&str>,
) -> Option<Vec<PathBookmark>> {
    let key = bookmark_key(host_id, path);
    let existing = current
        .iter()
        .position(|b| bookmark_key(b.host_id.as_deref(), &b.path) == key);

    match existing {
        None => {
            let mut next = current.to_vec();
            next.push(PathBookmark {
                id: uuid::Uuid::new_v4().to_string(),
                path: path.to_string(),
                label: label.map(str::to_string).filter(|s| !s.is_empty()),
                host_id: host_id.map(str::to_string),
            });
            Some(next)
        }
        Some(idx) => {
            let new_label = label.filter(|s| !s.is_empty());
            if new_label.is_none() || new_label == current[idx].label.as_deref() {
                return None; // no-op
            }
            let mut next = current.to_vec();
            next[idx].label = new_label.map(str::to_string);
            Some(next)
        }
    }
}

/// Remove the entry matching `(host_id, path)`. Safe no-op when nothing matches.
pub fn compute_remove_by_path(
    current: &[PathBookmark],
    host_id: Option<&str>,
    path: &str,
) -> Vec<PathBookmark> {
    let key = bookmark_key(host_id, path);
    current
        .iter()
        .filter(|b| bookmark_key(b.host_id.as_deref(), &b.path) != key)
        .cloned()
        .collect()
}

/// Remove the entry with the given id.
pub fn compute_remove_by_id(current: &[PathBookmark], id: &str) -> Vec<PathBookmark> {
    current.iter().filter(|b| b.id != id).cloned().collect()
}

/// The path context the active tab exposes — mirrors the reference
/// `filterBookmarksForContext` shape in the simpler Rust tab model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookmarkContext {
    /// A local terminal / explorer surface.
    Local,
    /// An SSH terminal for this host.
    Host(String),
    /// An SFTP browser for this host — shows host + local bookmarks.
    Sftp(String),
    /// No path context (editor / home / …) — show everything, grouped.
    None,
}

/// One rendered group in the bookmarks popover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookmarkSection {
    pub title: String,
    pub host_id: Option<String>,
    pub bookmarks: Vec<PathBookmark>,
}

fn host_title(host_id: &str, hosts: &[(String, String)]) -> String {
    hosts
        .iter()
        .find(|(id, _)| id == host_id)
        .map(|(_, name)| name.clone())
        .unwrap_or_else(|| "Unknown host".to_string())
}

fn of_host(bookmarks: &[PathBookmark], host_id: Option<&str>) -> Vec<PathBookmark> {
    bookmarks
        .iter()
        .filter(|b| b.host_id.as_deref() == host_id)
        .cloned()
        .collect()
}

/// Which bookmarks the popover should show for `ctx`. `hosts` is `(id, name)`
/// for every known host (used for section titles and orphan grouping).
pub fn filter_for_context(
    ctx: &BookmarkContext,
    bookmarks: &[PathBookmark],
    hosts: &[(String, String)],
) -> Vec<BookmarkSection> {
    match ctx {
        BookmarkContext::Local => vec![BookmarkSection {
            title: "Local".to_string(),
            host_id: None,
            bookmarks: of_host(bookmarks, None),
        }],
        BookmarkContext::Host(host_id) => vec![BookmarkSection {
            title: host_title(host_id, hosts),
            host_id: Some(host_id.clone()),
            bookmarks: of_host(bookmarks, Some(host_id)),
        }],
        BookmarkContext::Sftp(host_id) => vec![
            BookmarkSection {
                title: host_title(host_id, hosts),
                host_id: Some(host_id.clone()),
                bookmarks: of_host(bookmarks, Some(host_id)),
            },
            BookmarkSection {
                title: "Local".to_string(),
                host_id: None,
                bookmarks: of_host(bookmarks, None),
            },
        ],
        BookmarkContext::None => {
            let mut sections = Vec::new();
            let local = of_host(bookmarks, None);
            if !local.is_empty() {
                sections.push(BookmarkSection {
                    title: "Local".to_string(),
                    host_id: None,
                    bookmarks: local,
                });
            }
            let mut seen: Vec<&str> = Vec::new();
            for bm in bookmarks {
                let Some(host_id) = bm.host_id.as_deref() else {
                    continue;
                };
                if seen.contains(&host_id) {
                    continue;
                }
                seen.push(host_id);
                sections.push(BookmarkSection {
                    title: host_title(host_id, hosts),
                    host_id: Some(host_id.to_string()),
                    bookmarks: of_host(bookmarks, Some(host_id)),
                });
            }
            sections
        }
    }
}

// ── Persistence ──────────────────────────────────────────────────────────────

fn bookmarks_path() -> std::path::PathBuf {
    config_dir().join(BOOKMARKS_FILE)
}

fn load_from(path: &std::path::Path) -> Vec<PathBookmark> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str::<BookmarksFile>(&raw)
        .map(|f| f.bookmarks)
        .unwrap_or_default()
}

fn save_to(path: &std::path::Path, bookmarks: &[PathBookmark]) -> Result<(), String> {
    let file = BookmarksFile {
        bookmarks: bookmarks.to_vec(),
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

/// Load all bookmarks from the config dir (empty on a fresh install or a
/// corrupt file).
pub fn load() -> Vec<PathBookmark> {
    load_from(&bookmarks_path())
}

/// Persist the given bookmark list to the config dir.
pub fn save(bookmarks: &[PathBookmark]) -> Result<(), String> {
    save_to(&bookmarks_path(), bookmarks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bm(id: &str, host: Option<&str>, path: &str) -> PathBookmark {
        PathBookmark {
            id: id.to_string(),
            path: path.to_string(),
            label: None,
            host_id: host.map(str::to_string),
        }
    }

    #[test]
    fn key_uses_local_for_none_and_scopes_by_host() {
        assert_eq!(bookmark_key(None, "/foo"), "local::/foo");
        assert_eq!(bookmark_key(Some("host-a"), "/foo"), "host-a::/foo");
        assert_ne!(
            bookmark_key(Some("host-a"), "/foo"),
            bookmark_key(Some("host-b"), "/foo")
        );
    }

    #[test]
    fn add_inserts_when_absent() {
        let next = compute_add_bookmark(&[], None, "/foo", None).expect("insert");
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].path, "/foo");
        assert_eq!(next[0].host_id, None);
    }

    #[test]
    fn add_is_noop_for_same_pair() {
        let current = vec![bm("1", Some("host-a"), "/foo")];
        assert!(compute_add_bookmark(&current, Some("host-a"), "/foo", None).is_none());
    }

    #[test]
    fn add_distinguishes_hosts_and_local() {
        let current = vec![bm("1", Some("host-a"), "/foo")];
        let next = compute_add_bookmark(&current, Some("host-b"), "/foo", None).unwrap();
        assert_eq!(next.len(), 2);
        let next2 = compute_add_bookmark(&current, None, "/foo", None).unwrap();
        assert_eq!(next2.len(), 2);
    }

    #[test]
    fn add_updates_label_instead_of_second_entry() {
        let mut current = vec![bm("1", Some("host-a"), "/foo")];
        current[0].label = Some("old".to_string());
        let next = compute_add_bookmark(&current, Some("host-a"), "/foo", Some("new")).unwrap();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].label.as_deref(), Some("new"));
        // Same label → no-op.
        assert!(compute_add_bookmark(&next, Some("host-a"), "/foo", Some("new")).is_none());
    }

    #[test]
    fn remove_by_path_only_matching_pair() {
        let current = vec![
            bm("1", Some("host-a"), "/foo"),
            bm("2", Some("host-b"), "/foo"),
            bm("3", None, "/foo"),
        ];
        let next = compute_remove_by_path(&current, Some("host-a"), "/foo");
        assert_eq!(
            next.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(),
            ["2", "3"]
        );
    }

    #[test]
    fn remove_by_path_noop_when_no_match() {
        let current = vec![bm("1", Some("host-a"), "/foo")];
        assert_eq!(
            compute_remove_by_path(&current, Some("host-b"), "/foo").len(),
            1
        );
    }

    #[test]
    fn remove_by_id() {
        let current = vec![bm("1", None, "/a"), bm("2", None, "/b")];
        let next = compute_remove_by_id(&current, "1");
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, "2");
    }

    #[test]
    fn orphaned_only_when_host_gone() {
        let hosts = vec!["host-a".to_string()];
        assert!(!is_bookmark_orphaned(&bm("1", None, "/x"), &hosts));
        assert!(!is_bookmark_orphaned(
            &bm("1", Some("host-a"), "/x"),
            &hosts
        ));
        assert!(is_bookmark_orphaned(
            &bm("1", Some("host-gone"), "/x"),
            &hosts
        ));
    }

    fn sample() -> Vec<PathBookmark> {
        vec![
            bm("local-1", None, "/home/me"),
            bm("a-1", Some("host-a"), "/var/www"),
            bm("a-2", Some("host-a"), "/etc/nginx"),
            bm("b-1", Some("host-b"), "/srv"),
            bm("orphan-1", Some("host-deleted"), "/opt"),
        ]
    }

    fn hosts() -> Vec<(String, String)> {
        vec![
            ("host-a".to_string(), "Prod A".to_string()),
            ("host-b".to_string(), "Prod B".to_string()),
        ]
    }

    #[test]
    fn filter_local_shows_only_local() {
        let s = filter_for_context(&BookmarkContext::Local, &sample(), &hosts());
        assert_eq!(s.len(), 1);
        assert_eq!(
            s[0].bookmarks
                .iter()
                .map(|b| b.id.as_str())
                .collect::<Vec<_>>(),
            ["local-1"]
        );
    }

    #[test]
    fn filter_host_shows_only_that_host() {
        let s = filter_for_context(&BookmarkContext::Host("host-a".into()), &sample(), &hosts());
        assert_eq!(
            s[0].bookmarks
                .iter()
                .map(|b| b.id.as_str())
                .collect::<Vec<_>>(),
            ["a-1", "a-2"]
        );
    }

    #[test]
    fn filter_sftp_splits_host_and_local() {
        let s = filter_for_context(&BookmarkContext::Sftp("host-a".into()), &sample(), &hosts());
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].title, "Prod A");
        assert_eq!(s[1].title, "Local");
    }

    #[test]
    fn filter_none_groups_all_and_labels_orphan() {
        let s = filter_for_context(&BookmarkContext::None, &sample(), &hosts());
        let titles: Vec<&str> = s.iter().map(|x| x.title.as_str()).collect();
        assert!(titles.contains(&"Local"));
        assert!(titles.contains(&"Prod A"));
        assert!(titles.contains(&"Unknown host"));
        let orphan_section = s
            .iter()
            .find(|sec| sec.bookmarks.iter().any(|b| b.id == "orphan-1"))
            .unwrap();
        assert_eq!(orphan_section.title, "Unknown host");
    }

    #[test]
    fn persistence_round_trips() {
        let dir = std::env::temp_dir().join(format!("labonair-bm-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bookmarks.json");
        assert!(load_from(&path).is_empty());

        let list = vec![bm("1", None, "/a"), bm("2", Some("h"), "/b")];
        save_to(&path, &list).unwrap();
        assert_eq!(load_from(&path), list);

        // Corrupt file → empty, never an error.
        std::fs::write(&path, "{ not json").unwrap();
        assert!(load_from(&path).is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn is_bookmarked_and_find() {
        let current = sample();
        assert!(is_bookmarked(&current, Some("host-a"), "/var/www"));
        assert!(!is_bookmarked(&current, None, "/var/www"));
        assert_eq!(
            find_bookmark(&current, Some("host-b"), "/srv").map(|b| b.id.as_str()),
            Some("b-1")
        );
    }
}
