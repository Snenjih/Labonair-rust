//! Pure path helpers for the statusbar CWD breadcrumb — port of
//! `reference-src/src/modules/statusbar/lib/pathUtils.ts` and the
//! `resolveProvider` seam from `CwdBreadcrumb.tsx`.
//!
//! The interactive rendering lives in `AppShell::render_cwd_breadcrumb`
//! (needs `Context<AppShell>`); everything testable is here.

/// One breadcrumb segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub label: String,
    pub full_path: String,
    pub is_home: bool,
}

/// `relativePath(from, to)` — POSIX-style, `..` walk-up + walk-down, `"."` when
/// equal.
pub fn relative_path(from: &str, to: &str) -> String {
    let from_parts: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
    let to_parts: Vec<&str> = to.split('/').filter(|s| !s.is_empty()).collect();
    let mut i = 0;
    while i < from_parts.len() && i < to_parts.len() && from_parts[i] == to_parts[i] {
        i += 1;
    }
    let ups = from_parts.len() - i;
    let mut out: Vec<&str> = vec![".."; ups];
    out.extend_from_slice(&to_parts[i..]);
    if out.is_empty() {
        ".".to_string()
    } else {
        out.join("/")
    }
}

/// `segmentsFromCwd(cwd, home)` — the first segment is `~`/home or `/`, then one
/// clickable segment per path component with its accumulated absolute path.
pub fn segments_from_cwd(cwd: &str, home: Option<&str>) -> Vec<Segment> {
    let using_home = match home {
        Some(h) => !h.is_empty() && (cwd == h || cwd.starts_with(&format!("{h}/"))),
        None => false,
    };
    let tail = if using_home {
        let h = home.unwrap();
        cwd[h.len()..].trim_start_matches('/').to_string()
    } else {
        cwd.trim_start_matches('/').to_string()
    };
    let parts: Vec<&str> = if tail.is_empty() {
        Vec::new()
    } else {
        tail.split('/').filter(|s| !s.is_empty()).collect()
    };

    let mut segments = Vec::new();
    if using_home {
        segments.push(Segment {
            label: "~".to_string(),
            full_path: home.unwrap().to_string(),
            is_home: true,
        });
    } else {
        segments.push(Segment {
            label: "/".to_string(),
            full_path: "/".to_string(),
            is_home: false,
        });
    }

    let mut acc = segments[0].full_path.clone();
    for part in parts {
        acc = if acc == "/" {
            format!("/{part}")
        } else {
            format!("{acc}/{part}")
        };
        segments.push(Segment {
            label: part.to_string(),
            full_path: acc.clone(),
            is_home: false,
        });
    }
    segments
}

pub fn dirname(path: &str) -> String {
    match path.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(i) => path[..i].to_string(),
    }
}

pub fn basename(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

/// The fs-provider identity the breadcrumb browses through — `"local"` or
/// `"ssh:<hostId>"`. Port of `resolveProvider` (`CwdBreadcrumb.test.ts`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderId(pub String);

/// `{hostId, sessionId}` identifying the SSH session backing the active pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTarget {
    pub host_id: String,
    pub session_id: String,
}

pub fn resolve_provider(remote: Option<&RemoteTarget>) -> ProviderId {
    match remote {
        Some(t) => ProviderId(format!("ssh:{}", t.host_id)),
        None => ProviderId("local".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_matches_reference() {
        assert_eq!(relative_path("/a/b/c", "/a/b/c"), ".");
        assert_eq!(relative_path("/a/b/c", "/a/b"), "..");
        assert_eq!(relative_path("/a/b/c", "/a/b/d"), "../d");
        assert_eq!(relative_path("/a/b", "/a/b/c/d"), "c/d");
        assert_eq!(relative_path("/a/b/c", "/x/y"), "../../../x/y");
    }

    #[test]
    fn segments_with_home_collapse() {
        let s = segments_from_cwd("/home/nik/dev/proj", Some("/home/nik"));
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].label, "~");
        assert!(s[0].is_home);
        assert_eq!(s[0].full_path, "/home/nik");
        assert_eq!(s[1].label, "dev");
        assert_eq!(s[1].full_path, "/home/nik/dev");
        assert_eq!(s[2].full_path, "/home/nik/dev/proj");
    }

    #[test]
    fn segments_at_home_root() {
        let s = segments_from_cwd("/home/nik", Some("/home/nik"));
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].label, "~");
    }

    #[test]
    fn segments_absolute_no_home() {
        let s = segments_from_cwd("/usr/local/bin", None);
        assert_eq!(s.len(), 4);
        assert_eq!(s[0].label, "/");
        assert_eq!(s[0].full_path, "/");
        assert_eq!(s[1].full_path, "/usr");
        assert_eq!(s[3].full_path, "/usr/local/bin");
    }

    #[test]
    fn segments_root() {
        let s = segments_from_cwd("/", None);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].label, "/");
    }

    #[test]
    fn dirname_basename() {
        assert_eq!(dirname("/a/b/c.txt"), "/a/b");
        assert_eq!(dirname("/a"), "/");
        assert_eq!(dirname("a"), "/");
        assert_eq!(basename("/a/b/c.txt"), "c.txt");
        assert_eq!(basename("foo"), "foo");
    }

    #[test]
    fn resolve_provider_local_vs_remote() {
        assert_eq!(resolve_provider(None).0, "local");
        let t = RemoteTarget {
            host_id: "host-1".into(),
            session_id: "explorer:host-1".into(),
        };
        assert_eq!(resolve_provider(Some(&t)).0, "ssh:host-1");
        // Same local id for None twice.
        assert_eq!(resolve_provider(None), resolve_provider(None));
    }
}
