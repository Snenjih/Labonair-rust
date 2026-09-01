//! Git-Graph — commit-graph view (T10-001).
//!
//! [`GitGraphView`] is the GPUI-native port of the reference web app's
//! `src/modules/git-graph/` module (`GitGraphPane` + `GitGraphCanvas` +
//! `GraphRail` + `CommitDetailPanel`, layout from `lib/graphLayout.ts`).
//!
//! The lane-assignment algorithm ([`build_graph_layout`]) is a direct,
//! behaviour-preserving port of `buildGraphLayout` — a stateful left-to-right
//! sweep that assigns every commit a lane (column) and emits the top/bottom
//! half graph edges (straight pass-throughs, merge collapses, branch fan-outs)
//! needed to paint the rail.
//!
//! The rail itself is painted with plain absolutely-positioned GPUI `div`s
//! (vertical lane segments + horizontal connectors + a node dot) rather than a
//! `<canvas>`; the row list is virtualised with [`uniform_list`] so tens of
//! thousands of commits stay cheap (only visible rows build elements).
//!
//! All git access goes through `labonair_backend::modules::git` (a `git` CLI
//! wrapper). Every call is dispatched onto the tokio runtime and folded back
//! on the GPUI thread; a generation guard (`gen`, bumped on every
//! root/session/reload change) drops stale responses.

use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, uniform_list, App, ClickEvent, ClipboardItem, Context, Div, Entity, FocusHandle,
    Focusable, Font, Hsla, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    Stateful, StatefulInteractiveElement, Styled, Window,
};
use labonair_backend::modules::git::{self, CommitInfo};
use labonair_backend::App as Backend;
use tokio::runtime::Handle as TokioHandle;

use crate::theme::ThemeStore;

// ── geometry ───────────────────────────────────────────────────────────────

const ROW_H: f32 = 32.0;
const LANE_W: f32 = 14.0;
const DOT: f32 = 9.0;
const RAIL_PAD: f32 = 9.0;
/// Beyond this the rail stops widening — deep merge storms don't push the
/// commit text off-screen.
const MAX_VISIBLE_LANES: usize = 12;
const PAGE_INCREMENT: u32 = 200;
const AGE_TICK: Duration = Duration::from_secs(30);

// ── colours ────────────────────────────────────────────────────────────────

/// Lane colours — the reference `LANE_COLORS` (Tailwind `*-400`), as packed RGB.
const LANE_HEX: [u32; 8] = [
    0x60a5fa, 0xc084fc, 0x34d399, 0xfbbf24, 0xf472b6, 0x22d3ee, 0xfb923c, 0xa3e635,
];

/// Colour for lane `color_index` (wraps at 8, matching the reference).
pub fn lane_color(color_index: usize) -> Hsla {
    gpui::rgb(LANE_HEX[color_index % LANE_HEX.len()]).into()
}

/// Avatar fallback colours — the reference `AVATAR_COLORS`.
const AVATAR_HEX: [u32; 8] = [
    0x60a5fa, 0xa78bfa, 0x34d399, 0xfb923c, 0xf472b6, 0x22d3ee, 0xfbbf24, 0x818cf8,
];

/// Deterministic avatar colour for an author name — port of `getAvatarColor`.
pub fn avatar_color(name: &str) -> Hsla {
    let mut hash: u32 = 0;
    for b in name.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(b as u32) & 0xffff;
    }
    gpui::rgb(AVATAR_HEX[(hash % AVATAR_HEX.len() as u32) as usize]).into()
}

/// One/two-letter initials for an author name (port of the reference logic).
pub fn initials(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    match parts.as_slice() {
        [] => "?".to_string(),
        [one] => one
            .chars()
            .next()
            .unwrap_or('?')
            .to_ascii_uppercase()
            .to_string(),
        _ => {
            let f = parts[0].chars().next().unwrap_or('?').to_ascii_uppercase();
            let l = parts[parts.len() - 1]
                .chars()
                .next()
                .unwrap_or('?')
                .to_ascii_uppercase();
            format!("{f}{l}")
        }
    }
}

// ── graph layout ───────────────────────────────────────────────────────────

/// A graph edge drawn on the top or bottom half of a commit row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphEdge {
    /// A lane passing straight through this row.
    Straight { lane: usize, color_index: usize },
    /// An incoming merge: a lane above that expected this commit collapses
    /// into the commit's own lane.
    Merge {
        from_lane: usize,
        to_lane: usize,
        color_index: usize,
    },
    /// An outgoing branch: an extra parent fans out into a new lane below.
    Branch {
        from_lane: usize,
        to_lane: usize,
        color_index: usize,
    },
}

/// A commit decorated with its graph position — port of `LayoutCommit`.
#[derive(Debug, Clone)]
pub struct LayoutCommit {
    pub info: CommitInfo,
    pub row: usize,
    pub lane: usize,
    pub color_index: usize,
    pub lane_count: usize,
    pub top_edges: Vec<GraphEdge>,
    pub bottom_edges: Vec<GraphEdge>,
}

fn first_free_slot(lanes: &[Option<String>]) -> usize {
    lanes
        .iter()
        .position(Option::is_none)
        .unwrap_or(lanes.len())
}

fn trim_trailing(lanes: &mut Vec<Option<String>>) {
    while matches!(lanes.last(), Some(None)) {
        lanes.pop();
    }
}

/// Lane-assignment sweep — a direct port of `buildGraphLayout`.
///
/// `commits` must be topologically ordered newest-first (as `git log` emits).
pub fn build_graph_layout(commits: &[CommitInfo]) -> Vec<LayoutCommit> {
    let mut lanes: Vec<Option<String>> = Vec::new();
    let mut lane_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut result = Vec::with_capacity(commits.len());

    for (row, commit) in commits.iter().enumerate() {
        // Lanes currently expecting this commit (merge targets).
        let claiming: Vec<usize> = lanes
            .iter()
            .enumerate()
            .filter(|(_, v)| v.as_deref() == Some(commit.hash.as_str()))
            .map(|(i, _)| i)
            .collect();

        let lane = if let Some(&first) = claiming.first() {
            first
        } else {
            let l = first_free_slot(&lanes);
            if l == lanes.len() {
                lanes.push(None);
            }
            l
        };

        let lanes_before = lanes.clone();
        let mut top_edges: Vec<GraphEdge> = Vec::new();
        for (i, v) in lanes_before.iter().enumerate() {
            let Some(v) = v else { continue };
            if v == &commit.hash && i != lane {
                top_edges.push(GraphEdge::Merge {
                    from_lane: i,
                    to_lane: lane,
                    color_index: i % 8,
                });
            } else {
                top_edges.push(GraphEdge::Straight {
                    lane: i,
                    color_index: i % 8,
                });
            }
        }

        // Consume all claiming lanes; reset a fresh allocation too.
        for &idx in &claiming {
            if let Some(prev) = lanes[idx].take() {
                lane_map.remove(&prev);
            }
        }
        if claiming.is_empty() {
            lanes[lane] = None;
        }

        let parents = &commit.parent_hashes;
        let mut bottom_edges: Vec<GraphEdge> = Vec::new();

        if !parents.is_empty() {
            lanes[lane] = Some(parents[0].clone());
            lane_map.insert(parents[0].clone(), lane);

            for parent_hash in parents.iter().skip(1) {
                let parent_lane = if let Some(&pl) = lane_map.get(parent_hash) {
                    pl
                } else {
                    let pl = first_free_slot(&lanes);
                    if pl == lanes.len() {
                        lanes.push(None);
                    }
                    lanes[pl] = Some(parent_hash.clone());
                    lane_map.insert(parent_hash.clone(), pl);
                    pl
                };
                if parent_lane != lane {
                    bottom_edges.push(GraphEdge::Branch {
                        from_lane: lane,
                        to_lane: parent_lane,
                        color_index: parent_lane % 8,
                    });
                }
            }
        }

        let branch_targets: std::collections::HashSet<usize> = bottom_edges
            .iter()
            .filter_map(|e| match e {
                GraphEdge::Branch { to_lane, .. } => Some(*to_lane),
                _ => None,
            })
            .collect();
        for (i, v) in lanes.iter().enumerate() {
            if v.is_none() || branch_targets.contains(&i) {
                continue;
            }
            bottom_edges.push(GraphEdge::Straight {
                lane: i,
                color_index: i % 8,
            });
        }

        trim_trailing(&mut lanes);

        let lane_count = lanes_before.len().max(lanes.len()).max(lane + 1);

        result.push(LayoutCommit {
            info: commit.clone(),
            row,
            lane,
            color_index: lane % 8,
            lane_count,
            top_edges,
            bottom_edges,
        });
    }

    result
}

// ── pure helpers ───────────────────────────────────────────────────────────

/// First-page size — port of `initialGraphPageSize` (smaller for remote).
pub fn initial_graph_page_size(session_id: Option<&str>) -> u32 {
    if session_id.is_some() {
        200
    } else {
        500
    }
}

/// One `git show --numstat` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStat {
    pub path: String,
    pub added: u32,
    pub removed: u32,
}

/// Parse `additions\tdeletions\tpath` lines — port of `parseNumstat`.
/// Binary files (`-`) count as zero.
pub fn parse_numstat(raw: &str) -> Vec<FileStat> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let added = if parts[0] == "-" {
            0
        } else {
            parts[0].trim().parse().unwrap_or(0)
        };
        let removed = if parts[1] == "-" {
            0
        } else {
            parts[1].trim().parse().unwrap_or(0)
        };
        let path = parts[2..].join("\t");
        if !path.is_empty() {
            out.push(FileStat {
                path,
                added,
                removed,
            });
        }
    }
    out
}

/// Whether a backend error means "there is no repository here" (vs. a real
/// failure) — port of `isNoRepoError`.
pub fn is_no_repo_error(err: &str) -> bool {
    let l = err.to_lowercase();
    l.contains("not a git repo")
        || l.contains("not a git repository")
        || l.contains("does not have any commits")
        || l.contains("no such file or directory")
        || l.contains("repository not found")
        || (l.contains("fatal") && l.contains("git"))
}

/// `"12s ago"` / `"3m ago"` — port of the reference `RefreshAge` label.
pub fn relative_age(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s ago")
    } else {
        format!("{}m ago", (secs + 30) / 60)
    }
}

/// `"Mar 07 2024"` (UTC) — self-contained days→civil, no `chrono`.
pub fn format_commit_date(secs: i64) -> String {
    if secs <= 0 {
        return "\u{2014}".to_string();
    }
    let days = secs.div_euclid(86_400);
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
    const M: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!("{} {:02} {}", M[(month - 1) as usize], d, year)
}

/// Ref-badge classification — port of the inline logic in `GitGraphCanvas`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Tag,
    Remote,
    Local,
}

/// Local conventional-branch prefixes that must NOT be mistaken for a remote.
const LOCAL_BRANCH_PREFIXES: [&str; 8] = [
    "feat/",
    "fix/",
    "chore/",
    "refactor/",
    "docs/",
    "test/",
    "perf/",
    "ci/",
];

pub fn classify_ref(r: &str) -> RefKind {
    let bytes = r.as_bytes();
    if bytes.first() == Some(&b'v') && bytes.get(1).is_some_and(u8::is_ascii_digit) {
        return RefKind::Tag;
    }
    if r.starts_with("origin/") || r.starts_with("upstream/") {
        return RefKind::Remote;
    }
    if let Some(slash) = r.find('/') {
        let prefix = &r[..=slash];
        let head_is_slug = r[..slash]
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
        if head_is_slug && !LOCAL_BRANCH_PREFIXES.contains(&prefix) {
            return RefKind::Remote;
        }
    }
    RefKind::Local
}

// ── view state ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct Colors {
    bg: Hsla,
    fg: Hsla,
    card: Hsla,
    muted: Hsla,
    border: Hsla,
    accent: Hsla,
    success: Hsla,
    error: Hsla,
    info: Hsla,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GraphState {
    /// No working directory selected yet.
    Idle,
    Loading,
    NoRepo,
    Error(String),
    Loaded,
}

pub struct GitGraphView {
    backend: Backend,
    tokio: TokioHandle,
    theme: Entity<ThemeStore>,
    focus: FocusHandle,

    /// The active terminal's cwd (fed from the app shell).
    root: Option<String>,
    /// Remote SSH session backing `root`, if any.
    session_id: Option<String>,
    /// Resolved repository root (`git rev-parse --show-toplevel`).
    repo_path: Option<String>,

    state: GraphState,
    /// Bumped on every genuine target change / reload; stale responses drop.
    gen: u64,
    loading: bool,

    /// Undecorated commits as fetched — `load_more` appends here.
    raw: Vec<CommitInfo>,
    commits: Arc<Vec<LayoutCommit>>,
    max_lane_count: usize,
    total_loaded: u32,
    has_more: bool,
    /// Current branch name (`None` when detached / unknown) — drives the
    /// `HEAD` badge marker.
    head_branch: Option<String>,
    last_refreshed: Option<Instant>,

    /// Selected row index into `commits`.
    selected: Option<usize>,
    detail_numstat: Option<Vec<FileStat>>,
    detail_diff: Option<Result<String, String>>,
    show_diff: bool,
}

impl GitGraphView {
    pub fn new(
        backend: Backend,
        tokio: TokioHandle,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();

        // Repaint the "refreshed Ns ago" label periodically. Stops when dropped.
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(AGE_TICK).await;
            if this
                .update(cx, |this, cx| {
                    if this.last_refreshed.is_some() {
                        cx.notify();
                    }
                })
                .is_err()
            {
                break;
            }
        })
        .detach();

        Self {
            backend,
            tokio,
            theme,
            focus: cx.focus_handle(),
            root: None,
            session_id: None,
            repo_path: None,
            state: GraphState::Idle,
            gen: 0,
            loading: false,
            raw: Vec::new(),
            commits: Arc::new(Vec::new()),
            max_lane_count: 1,
            total_loaded: 0,
            has_more: false,
            head_branch: None,
            last_refreshed: None,
            selected: None,
            detail_numstat: None,
            detail_diff: None,
            show_diff: false,
        }
    }

    /// Point the graph at a new working directory (app-shell driven).
    pub fn set_root(&mut self, root: Option<String>, cx: &mut Context<Self>) {
        if self.root == root {
            return;
        }
        self.root = root;
        self.reset_and_load(cx);
    }

    /// Point the graph at a remote SSH session (or back to local with `None`).
    pub fn set_session(&mut self, session_id: Option<String>, cx: &mut Context<Self>) {
        if self.session_id == session_id {
            return;
        }
        self.session_id = session_id;
        self.reset_and_load(cx);
    }

    fn reset_and_load(&mut self, cx: &mut Context<Self>) {
        self.gen += 1;
        self.raw.clear();
        self.commits = Arc::new(Vec::new());
        self.max_lane_count = 1;
        self.selected = None;
        self.detail_numstat = None;
        self.detail_diff = None;
        self.show_diff = false;
        self.total_loaded = 0;
        self.has_more = false;
        self.repo_path = None;
        if self.root.is_some() {
            let size = initial_graph_page_size(self.session_id.as_deref());
            self.load(size, false, cx);
        } else {
            self.state = GraphState::Idle;
        }
        cx.notify();
    }

    /// Re-walk from HEAD keeping the currently-loaded depth (catches new commits).
    fn reload(&mut self, cx: &mut Context<Self>) {
        if self.root.is_none() {
            return;
        }
        self.gen += 1;
        let n = if self.total_loaded == 0 {
            initial_graph_page_size(self.session_id.as_deref())
        } else {
            self.total_loaded
        };
        self.load(n, false, cx);
        cx.notify();
    }

    fn load_more(&mut self, cx: &mut Context<Self>) {
        if self.loading || !self.has_more {
            return;
        }
        self.load(PAGE_INCREMENT, true, cx);
    }

    fn load(&mut self, limit: u32, append: bool, cx: &mut Context<Self>) {
        let Some(root) = self.root.clone() else {
            return;
        };
        self.loading = true;
        if !append {
            self.state = GraphState::Loading;
        }
        let generation = self.gen;
        let session = self.session_id.clone();
        let backend = self.backend.clone();
        let skip = Some(if append {
            self.total_loaded as usize
        } else {
            0
        });
        // Over-fetch by one so `has_more` is exact even when the repo has
        // exactly `limit` commits left (port of `fetchPage`).
        let fetch = limit + 1;

        let jh = self.tokio.spawn(async move {
            let is_repo =
                git::git_is_repo(root.clone(), session.clone(), &backend.ssh, backend.clone())
                    .await?;
            if !is_repo {
                return Ok::<_, String>(None);
            }
            let repo_root = git::git_get_repo_root(
                root.clone(),
                session.clone(),
                &backend.ssh,
                backend.clone(),
            )
            .await?;
            let head = git::git_get_current_branch(
                repo_root.clone(),
                session.clone(),
                &backend.ssh,
                backend.clone(),
            )
            .await
            .ok();
            let page = git::git_get_log(
                repo_root.clone(),
                Some(fetch),
                true,
                session.clone(),
                skip,
                &backend.ssh,
                backend.clone(),
            )
            .await?;
            Ok(Some((repo_root, head, page)))
        });

        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if this.gen != generation {
                    return;
                }
                this.loading = false;
                this.last_refreshed = Some(Instant::now());
                match res {
                    Ok(Some((repo_root, head, mut page))) => {
                        this.repo_path = Some(repo_root);
                        this.head_branch =
                            head.filter(|h| !h.is_empty() && !h.starts_with("HEAD detached"));
                        let has_more = page.len() as u32 > limit;
                        if has_more {
                            page.truncate(limit as usize);
                        }
                        if append {
                            this.total_loaded += page.len() as u32;
                            this.raw.extend(page);
                        } else {
                            this.total_loaded = limit;
                            this.raw = page;
                            this.selected = None;
                            this.detail_numstat = None;
                            this.detail_diff = None;
                        }
                        this.has_more = has_more;
                        this.rebuild_layout();
                        this.state = GraphState::Loaded;
                    }
                    Ok(None) => {
                        this.state = GraphState::NoRepo;
                        this.raw.clear();
                        this.commits = Arc::new(Vec::new());
                    }
                    Err(e) => {
                        this.state = if is_no_repo_error(&e) {
                            GraphState::NoRepo
                        } else {
                            GraphState::Error(e)
                        };
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn rebuild_layout(&mut self) {
        let commits = build_graph_layout(&self.raw);
        self.max_lane_count = commits
            .iter()
            .map(|c| c.lane_count)
            .max()
            .unwrap_or(1)
            .max(1);
        self.commits = Arc::new(commits);
    }

    fn select(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx >= self.commits.len() {
            return;
        }
        self.selected = Some(idx);
        self.detail_numstat = None;
        self.detail_diff = None;
        self.show_diff = false;
        cx.notify();

        let (Some(repo), Some(commit)) = (self.repo_path.clone(), self.commits.get(idx).cloned())
        else {
            return;
        };
        let hash = commit.info.hash.clone();
        let session = self.session_id.clone();
        let backend = self.backend.clone();
        let generation = self.gen;

        let jh = self.tokio.spawn(async move {
            git::git_get_commit_numstat(repo, hash, session, &backend.ssh, backend.clone()).await
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if this.gen != generation || this.selected != Some(idx) {
                    return;
                }
                this.detail_numstat = Some(res.map(|s| parse_numstat(&s)).unwrap_or_default());
                cx.notify();
            });
        })
        .detach();
    }

    fn toggle_diff(&mut self, cx: &mut Context<Self>) {
        self.show_diff = !self.show_diff;
        cx.notify();
        if !self.show_diff || self.detail_diff.is_some() {
            return;
        }
        let (Some(repo), Some(idx)) = (self.repo_path.clone(), self.selected) else {
            return;
        };
        let Some(commit) = self.commits.get(idx).cloned() else {
            return;
        };
        let hash = commit.info.hash.clone();
        let session = self.session_id.clone();
        let backend = self.backend.clone();
        let generation = self.gen;

        let jh = self.tokio.spawn(async move {
            git::git_get_commit_diff(repo, hash, session, &backend.ssh, backend.clone()).await
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if this.gen != generation || this.selected != Some(idx) {
                    return;
                }
                this.detail_diff = Some(res);
                cx.notify();
            });
        })
        .detach();
    }

    fn colors(&self, cx: &App) -> Colors {
        let t = self.theme.read(cx);
        Colors {
            bg: t.background(),
            fg: t.foreground(),
            card: t.card(),
            muted: t.muted_foreground(),
            border: t.border(),
            accent: t.accent(),
            success: t.status_success(),
            error: t.status_error(),
            info: t.status_info(),
        }
    }

    fn mono(&self, cx: &App) -> Font {
        self.theme.read(cx).buffer_font()
    }

    // ── rendering ──────────────────────────────────────────────────────────

    fn render_toolbar(&self, c: Colors, cx: &mut Context<Self>) -> Div {
        let repo = self.repo_path.clone().unwrap_or_default();
        let name = repo.rsplit('/').next().unwrap_or(&repo).to_string();
        let parent = repo.strip_suffix(&name).unwrap_or("").to_string();
        let remote = self.session_id.is_some();
        let age = self
            .last_refreshed
            .map(|t| relative_age(t.elapsed().as_secs()));

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.0))
            .h(px(28.0))
            .px(px(10.0))
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            .text_size(px(11.0))
                            .child(div().text_color(c.muted).child(SharedString::from(parent)))
                            .child(div().text_color(c.fg).child(SharedString::from(name))),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(c.border)
                            .px(px(4.0))
                            .text_size(px(9.0))
                            .text_color(c.muted)
                            .child(SharedString::from(if remote { "Remote" } else { "Local" })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .flex_shrink_0()
                    .when_some(age, |d, a| {
                        d.child(
                            div()
                                .text_size(px(10.0))
                                .text_color(c.muted)
                                .child(SharedString::from(a)),
                        )
                    })
                    .child(
                        div()
                            .id("git-graph-refresh")
                            .px(px(4.0))
                            .text_size(px(11.0))
                            .text_color(c.muted)
                            .hover(|s| s.text_color(c.fg))
                            .child(SharedString::from(if self.loading {
                                "\u{2026}"
                            } else {
                                "\u{21bb}"
                            }))
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.reload(cx))),
                    ),
            )
    }

    fn render_rows(&self, c: Colors, cx: &mut Context<Self>) -> Div {
        let commits = self.commits.clone();
        let selected = self.selected;
        let max_lanes = self.max_lane_count;
        let head = self.head_branch.clone();
        let mono = self.mono(cx);
        let view = cx.entity();

        let list = uniform_list("git-graph-rows", commits.len(), move |range, _win, _cx| {
            let head = head.as_deref();
            range
                .map(|i| {
                    let commit = &commits[i];
                    let v = view.clone();
                    commit_row(i, c, commit, selected == Some(i), max_lanes, head, &mono).on_click(
                        move |_e: &ClickEvent, _w, cx| {
                            v.update(cx, |this, cx| this.select(i, cx));
                        },
                    )
                })
                .collect::<Vec<_>>()
        })
        .flex_1();

        let mut col = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .child(column_headers(c, self.max_lane_count))
            .child(list);

        if self.has_more {
            col = col.child(
                div()
                    .id("git-graph-load-more")
                    .flex()
                    .items_center()
                    .justify_center()
                    .h(px(30.0))
                    .border_t_1()
                    .border_color(c.border)
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child(SharedString::from(if self.loading {
                        "Loading\u{2026}"
                    } else {
                        "Load more commits"
                    }))
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.load_more(cx))),
            );
        }

        col
    }

    fn render_detail(&self, c: Colors, cx: &mut Context<Self>) -> Div {
        let Some(idx) = self.selected else {
            return div();
        };
        let Some(commit) = self.commits.get(idx).cloned() else {
            return div();
        };
        let info = &commit.info;
        let mono = self.mono(cx);

        let (added, removed): (u32, u32) = self
            .detail_numstat
            .as_ref()
            .map(|files| {
                files
                    .iter()
                    .fold((0, 0), |(a, r), f| (a + f.added, r + f.removed))
            })
            .unwrap_or((0, 0));

        let header = div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(4.0))
            .p(px(16.0))
            .child(
                div()
                    .w(px(48.0))
                    .h(px(48.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .bg(avatar_color(&info.author_name))
                    .text_size(px(18.0))
                    .text_color(c.bg)
                    .child(SharedString::from(initials(&info.author_name))),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(c.fg)
                    .child(SharedString::from(info.author_name.clone())),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .child(SharedString::from(format_commit_date(info.timestamp))),
            );

        let meta = div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .px(px(16.0))
            .pb(px(12.0))
            .text_size(px(11.0))
            .text_color(c.muted)
            .child(
                div()
                    .truncate()
                    .child(SharedString::from(info.author_email.clone())),
            )
            .child(
                div()
                    .id("git-graph-copy-hash")
                    .font(mono.clone())
                    .truncate()
                    .hover(|s| s.text_color(c.fg))
                    .child(SharedString::from(info.hash.clone()))
                    .on_click({
                        let h = info.hash.clone();
                        cx.listener(move |_this, _: &ClickEvent, _w, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(h.clone()));
                        })
                    }),
            )
            .child(div().truncate().child(SharedString::from(format!(
                "parents: {}",
                if info.parent_hashes.is_empty() {
                    "(root)".to_string()
                } else {
                    info.parent_hashes
                        .iter()
                        .map(|p| p.chars().take(7).collect::<String>())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            ))));

        let subject = div()
            .px(px(16.0))
            .py(px(10.0))
            .border_t_1()
            .border_color(c.border)
            .text_size(px(12.0))
            .text_color(c.fg)
            .child(SharedString::from(info.subject.clone()));

        let files_head = div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(16.0))
            .py(px(8.0))
            .text_size(px(11.0))
            .child(div().text_color(c.muted).child(SharedString::from(format!(
                "{} changed files",
                info.files_changed
            ))))
            .child(
                div()
                    .flex()
                    .gap(px(6.0))
                    .font(mono.clone())
                    .when(added > 0, |d| {
                        d.child(
                            div()
                                .text_color(c.success)
                                .child(SharedString::from(format!("+{added}"))),
                        )
                    })
                    .when(removed > 0, |d| {
                        d.child(
                            div()
                                .text_color(c.error)
                                .child(SharedString::from(format!("\u{2212}{removed}"))),
                        )
                    }),
            );

        let mut file_list = div().flex().flex_col().px(px(8.0)).pb(px(8.0));
        match &self.detail_numstat {
            None => {
                file_list = file_list.child(
                    div()
                        .px(px(8.0))
                        .py(px(4.0))
                        .text_size(px(11.0))
                        .text_color(c.muted)
                        .child(SharedString::from("Loading\u{2026}")),
                );
            }
            Some(files) => {
                for f in files {
                    file_list =
                        file_list.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(6.0))
                                .px(px(8.0))
                                .py(px(3.0))
                                .text_size(px(11.0))
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_color(c.fg.opacity(0.8))
                                        .child(SharedString::from(f.path.clone())),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_shrink_0()
                                        .gap(px(4.0))
                                        .font(mono.clone())
                                        .text_size(px(9.5))
                                        .when(f.added > 0, |d| {
                                            d.child(
                                                div().text_color(c.success).child(
                                                    SharedString::from(format!("+{}", f.added)),
                                                ),
                                            )
                                        })
                                        .when(f.removed > 0, |d| {
                                            d.child(div().text_color(c.error).child(
                                                SharedString::from(format!("-{}", f.removed)),
                                            ))
                                        }),
                                ),
                        );
                }
            }
        }

        let can_prev = idx + 1 < self.commits.len();
        let can_next = idx > 0;
        let nav = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.0))
            .p(px(8.0))
            .border_t_1()
            .border_color(c.border)
            .child(nav_btn(
                "git-graph-prev",
                "\u{2191} Older",
                can_prev,
                c,
                cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    if idx + 1 < this.commits.len() {
                        this.select(idx + 1, cx);
                    }
                }),
            ))
            .child(
                div()
                    .id("git-graph-toggle-diff")
                    .px(px(6.0))
                    .py(px(3.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(c.border)
                    .text_size(px(11.0))
                    .text_color(c.fg)
                    .hover(|s| s.bg(c.fg.opacity(0.05)))
                    .child(SharedString::from(if self.show_diff {
                        "Hide diff"
                    } else {
                        "View diff"
                    }))
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.toggle_diff(cx))),
            )
            .child(nav_btn(
                "git-graph-next",
                "Newer \u{2193}",
                can_next,
                c,
                cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    if this.selected.is_some_and(|s| s > 0) {
                        this.select(idx - 1, cx);
                    }
                }),
            ));

        let mut body = div()
            .id("git-graph-detail-body")
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .child(subject)
            .child(div().border_t_1().border_color(c.border).child(files_head))
            .child(file_list);

        if self.show_diff {
            let mut diff_box = div()
                .flex()
                .flex_col()
                .border_t_1()
                .border_color(c.border)
                .font(mono.clone())
                .text_size(px(10.5));
            match &self.detail_diff {
                None => {
                    diff_box = diff_box.child(
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .text_color(c.muted)
                            .child(SharedString::from("Loading diff\u{2026}")),
                    );
                }
                Some(Err(e)) => {
                    diff_box = diff_box.child(
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .text_color(c.error)
                            .child(SharedString::from(e.clone())),
                    );
                }
                Some(Ok(text)) => {
                    for line in text.lines().take(800) {
                        diff_box = diff_box.child(diff_line(line, c));
                    }
                }
            }
            body = body.child(diff_box);
        }

        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .w(px(320.0))
            .border_l_1()
            .border_color(c.border)
            .bg(c.bg)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .h(px(24.0))
                    .px(px(8.0))
                    .child(
                        div()
                            .id("git-graph-detail-close")
                            .text_size(px(12.0))
                            .text_color(c.muted)
                            .hover(|s| s.text_color(c.fg))
                            .child(SharedString::from("\u{2715}"))
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                this.selected = None;
                                this.detail_numstat = None;
                                this.detail_diff = None;
                                cx.notify();
                            })),
                    ),
            )
            .child(header)
            .child(meta)
            .child(body)
            .child(nav)
    }

    fn render_center(&self, c: Colors, cx: &mut Context<Self>) -> Div {
        match &self.state {
            GraphState::Idle => center_message(
                "No repository",
                "Open a folder containing a Git repository to view the graph.",
                c,
            ),
            GraphState::Loading if self.commits.is_empty() => {
                center_message("Loading commits\u{2026}", "", c)
            }
            GraphState::NoRepo => center_message(
                "No repository",
                "The selected folder is not a Git repository.",
                c,
            ),
            GraphState::Error(e) => center_message("Failed to load git log", e, c),
            GraphState::Loading | GraphState::Loaded => {
                if self.commits.is_empty() {
                    center_message("No commits found", "", c)
                } else {
                    self.render_rows(c, cx)
                }
            }
        }
    }
}

impl Focusable for GitGraphView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for GitGraphView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = self.colors(cx);
        div()
            .track_focus(&self.focus)
            .flex()
            .flex_col()
            .size_full()
            .bg(c.bg)
            .text_color(c.fg)
            .font(self.theme.read(cx).ui_font())
            .child(self.render_toolbar(c, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.0))
                    .child(self.render_center(c, cx))
                    .child(self.render_detail(c, cx)),
            )
    }
}

// ── free rendering helpers ─────────────────────────────────────────────────

fn nav_btn(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    c: Colors,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let mut d = div()
        .id(id)
        .px(px(6.0))
        .py(px(3.0))
        .text_size(px(11.0))
        .text_color(if enabled {
            c.muted
        } else {
            c.muted.opacity(0.4)
        })
        .child(SharedString::from(label));
    if enabled {
        d = d.hover(|s| s.text_color(c.fg)).on_click(on_click);
    }
    d
}

fn center_message(title: &str, body: &str, c: Colors) -> Div {
    div()
        .flex()
        .flex_1()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .p(px(16.0))
        .child(
            div()
                .text_size(px(13.0))
                .text_color(c.fg)
                .child(SharedString::from(title.to_string())),
        )
        .when(!body.is_empty(), |d| {
            d.child(
                div()
                    .max_w(px(320.0))
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .child(SharedString::from(body.to_string())),
            )
        })
}

fn column_headers(c: Colors, max_lanes: usize) -> Div {
    let rail_w = rail_width(max_lanes);
    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .h(px(24.0))
        .px(px(6.0))
        .bg(c.card.opacity(0.5))
        .border_b_1()
        .border_color(c.border)
        .text_size(px(9.5))
        .text_color(c.muted)
        .child(div().flex_shrink_0().w(px(rail_w)))
        .child(
            div()
                .flex_shrink_0()
                .w(px(60.0))
                .child(SharedString::from("SHA")),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(SharedString::from("SUBJECT")),
        )
        .child(
            div()
                .flex_shrink_0()
                .w(px(140.0))
                .child(SharedString::from("AUTHOR")),
        )
        .child(
            div()
                .flex_shrink_0()
                .w(px(96.0))
                .child(SharedString::from("DATE")),
        )
        .child(
            div()
                .flex_shrink_0()
                .w(px(72.0))
                .child(SharedString::from("CHANGES")),
        )
}

/// Reserved rail width for `lane_count` lanes (clamped to [`MAX_VISIBLE_LANES`]).
fn rail_width(lane_count: usize) -> f32 {
    let shown = lane_count.clamp(1, MAX_VISIBLE_LANES);
    RAIL_PAD * 2.0 + (shown.saturating_sub(1)) as f32 * LANE_W + DOT
}

fn v_seg(x: f32, top: f32, h: f32, col: Hsla) -> Div {
    div()
        .absolute()
        .left(px(x - 1.0))
        .top(px(top))
        .w(px(2.0))
        .h(px(h))
        .bg(col)
}

fn h_seg(x0: f32, x1: f32, y: f32, col: Hsla) -> Div {
    let (a, b) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    div()
        .absolute()
        .left(px(a))
        .top(px(y))
        .w(px((b - a).max(2.0)))
        .h(px(2.0))
        .bg(col)
}

fn rail(commit: &LayoutCommit, max_lanes: usize, active: bool, c: Colors) -> Div {
    let shown = max_lanes.clamp(1, MAX_VISIBLE_LANES);
    let clamp = |i: usize| i.min(shown - 1);
    let lane_x = |i: usize| RAIL_PAD + clamp(i) as f32 * LANE_W;

    let mut root = div()
        .relative()
        .flex_shrink_0()
        .w(px(rail_width(max_lanes)))
        .h(px(ROW_H));

    for e in &commit.top_edges {
        match *e {
            GraphEdge::Straight { lane, color_index } => {
                let col = lane_color(color_index).opacity(0.85);
                root = root.child(v_seg(lane_x(lane), 0.0, ROW_H / 2.0, col));
            }
            GraphEdge::Merge {
                from_lane,
                to_lane,
                color_index,
            } => {
                let col = lane_color(color_index).opacity(0.85);
                root = root
                    .child(v_seg(lane_x(from_lane), 0.0, 8.0, col))
                    .child(h_seg(lane_x(from_lane), lane_x(to_lane), 8.0, col))
                    .child(v_seg(lane_x(to_lane), 8.0, 8.0, col));
            }
            GraphEdge::Branch { .. } => {}
        }
    }
    for e in &commit.bottom_edges {
        match *e {
            GraphEdge::Straight { lane, color_index } => {
                let col = lane_color(color_index).opacity(0.85);
                root = root.child(v_seg(lane_x(lane), ROW_H / 2.0, ROW_H / 2.0, col));
            }
            GraphEdge::Branch {
                from_lane,
                to_lane,
                color_index,
            } => {
                let col = lane_color(color_index).opacity(0.85);
                root = root
                    .child(v_seg(lane_x(from_lane), 16.0, 8.0, col))
                    .child(h_seg(lane_x(from_lane), lane_x(to_lane), 24.0, col))
                    .child(v_seg(lane_x(to_lane), 24.0, 8.0, col));
            }
            GraphEdge::Merge { .. } => {}
        }
    }

    let nx = lane_x(commit.lane);
    let mut node = div()
        .absolute()
        .left(px(nx - DOT / 2.0))
        .top(px(ROW_H / 2.0 - DOT / 2.0))
        .w(px(DOT))
        .h(px(DOT))
        .rounded(px(999.0))
        .bg(lane_color(commit.color_index));
    if active {
        node = node.border_1().border_color(c.fg);
    }
    root.child(node)
}

#[allow(clippy::too_many_arguments)]
fn commit_row(
    idx: usize,
    c: Colors,
    commit: &LayoutCommit,
    selected: bool,
    max_lanes: usize,
    head: Option<&str>,
    mono: &Font,
) -> Stateful<Div> {
    let info = &commit.info;

    let badges = info.refs.iter().map(|r| {
        let kind = classify_ref(r);
        let is_head = head == Some(r.as_str());
        let (bg, fg, bord) = match kind {
            RefKind::Tag => (c.info.opacity(0.18), c.info, c.info.opacity(0.4)),
            _ => {
                let lc = lane_color(commit.color_index);
                (lc.opacity(0.15), lc, lc.opacity(0.4))
            }
        };
        let dim = matches!(kind, RefKind::Remote) && !is_head;
        div()
            .flex()
            .items_center()
            .gap(px(3.0))
            .flex_shrink_0()
            .rounded(px(3.0))
            .px(px(4.0))
            .text_size(px(9.5))
            .bg(bg)
            .text_color(fg)
            .border_1()
            .border_color(bord)
            .when(dim, |d| d.opacity(0.65))
            .when(is_head, |d| {
                d.child(div().text_color(c.fg).child(SharedString::from("HEAD")))
            })
            .child(SharedString::from(r.clone()))
    });

    let changes = div()
        .flex()
        .flex_shrink_0()
        .w(px(72.0))
        .items_center()
        .justify_end()
        .gap(px(4.0))
        .font(mono.clone())
        .text_size(px(10.0))
        .when(info.insertions == 0 && info.deletions == 0, |d| {
            d.child(
                div()
                    .text_color(c.muted.opacity(0.4))
                    .child(SharedString::from("\u{2014}")),
            )
        })
        .when(info.insertions > 0, |d| {
            d.child(
                div()
                    .text_color(c.success)
                    .child(SharedString::from(format!("+{}", info.insertions))),
            )
        })
        .when(info.deletions > 0, |d| {
            d.child(
                div()
                    .text_color(c.error)
                    .child(SharedString::from(format!("\u{2212}{}", info.deletions))),
            )
        });

    div()
        .id(("git-graph-row", idx))
        .flex()
        .items_center()
        .gap(px(12.0))
        .h(px(ROW_H))
        .px(px(6.0))
        .border_l_1()
        .border_color(c.accent.opacity(if selected { 1.0 } else { 0.0 }))
        .when(selected, |d| d.bg(c.accent.opacity(0.12)))
        .hover(|s| s.bg(c.fg.opacity(0.04)))
        .cursor_pointer()
        .child(rail(commit, max_lanes, selected, c))
        .child(
            div()
                .flex_shrink_0()
                .w(px(60.0))
                .font(mono.clone())
                .text_size(px(10.5))
                .text_color(c.muted)
                .child(SharedString::from(info.short_hash.clone())),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(6.0))
                .overflow_hidden()
                .children(badges)
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(12.0))
                        .text_color(c.fg.opacity(0.9))
                        .child(SharedString::from(info.subject.clone())),
                ),
        )
        .child(
            div()
                .flex_shrink_0()
                .w(px(140.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .flex_shrink_0()
                        .w(px(16.0))
                        .h(px(16.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(3.0))
                        .bg(avatar_color(&info.author_name))
                        .text_size(px(8.0))
                        .text_color(c.bg)
                        .child(SharedString::from(initials(&info.author_name))),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(10.5))
                        .text_color(c.muted)
                        .child(SharedString::from(info.author_name.clone())),
                ),
        )
        .child(
            div()
                .flex_shrink_0()
                .w(px(96.0))
                .font(mono.clone())
                .text_size(px(10.5))
                .text_color(c.muted)
                .child(SharedString::from(format_commit_date(info.timestamp))),
        )
        .child(changes)
}

fn diff_line(line: &str, c: Colors) -> Div {
    let color = match line.as_bytes().first() {
        Some(b'+') if !line.starts_with("+++") => c.success,
        Some(b'-') if !line.starts_with("---") => c.error,
        Some(b'@') => c.info,
        _ => c.fg,
    };
    div()
        .px(px(8.0))
        .whitespace_nowrap()
        .text_color(color)
        .child(SharedString::from(if line.is_empty() {
            " ".to_string()
        } else {
            line.to_string()
        }))
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(hash: &str, parents: &[&str]) -> CommitInfo {
        CommitInfo {
            hash: hash.to_string(),
            short_hash: hash.chars().take(7).collect(),
            parent_hashes: parents.iter().map(|s| s.to_string()).collect(),
            author_name: "Ada Lovelace".to_string(),
            author_email: "ada@example.com".to_string(),
            timestamp: 1_700_000_000,
            subject: format!("commit {hash}"),
            refs: vec![],
            files_changed: 1,
            insertions: 2,
            deletions: 1,
        }
    }

    #[test]
    fn initial_page_size_matches_reference() {
        assert_eq!(initial_graph_page_size(None), 500);
        assert_eq!(initial_graph_page_size(Some("explorer:host-1")), 200);
    }

    #[test]
    fn linear_history_stays_in_one_lane() {
        let commits = vec![commit("c", &["b"]), commit("b", &["a"]), commit("a", &[])];
        let layout = build_graph_layout(&commits);
        assert_eq!(layout.len(), 3);
        assert!(layout.iter().all(|lc| lc.lane == 0));
        assert!(layout.iter().all(|lc| lc.lane_count == 1));
        // No merge/branch edges in linear history.
        for lc in &layout {
            assert!(lc
                .top_edges
                .iter()
                .chain(&lc.bottom_edges)
                .all(|e| matches!(e, GraphEdge::Straight { .. })));
        }
    }

    #[test]
    fn feature_branch_gets_its_own_lane_then_merges_back() {
        // main:   M ── C ── A
        //          \       /
        // feature:   F ────
        // log order (newest first): M, C, F, A
        let commits = vec![
            commit("M", &["C", "F"]), // merge commit
            commit("C", &["A"]),
            commit("F", &["A"]),
            commit("A", &[]),
        ];
        let layout = build_graph_layout(&commits);

        let m = &layout[0];
        assert_eq!(m.lane, 0);
        // Merge commit fans a second parent (F) into a new lane below.
        assert!(m.bottom_edges.iter().any(
            |e| matches!(e, GraphEdge::Branch { from_lane: 0, to_lane, .. } if *to_lane != 0)
        ));

        let f = &layout[2];
        assert_ne!(f.lane, 0, "feature commit must not sit in main's lane");

        // A is claimed by both lane 0 (from C) and F's lane → it collapses
        // back to a single lane with an incoming merge edge.
        let a = &layout[3];
        assert_eq!(a.lane, 0);
        assert!(a
            .top_edges
            .iter()
            .any(|e| matches!(e, GraphEdge::Merge { to_lane: 0, .. })));

        // Lanes never grow unbounded: at most 2 here.
        assert!(layout.iter().all(|lc| lc.lane_count <= 2));
    }

    #[test]
    fn lanes_are_reused_after_a_branch_ends() {
        // Two independent roots — the second must reuse lane 0 after the first
        // terminates, not pile onto lane 1 forever.
        let commits = vec![
            commit("y2", &["y1"]),
            commit("y1", &[]),
            commit("x2", &["x1"]),
            commit("x1", &[]),
        ];
        let layout = build_graph_layout(&commits);
        assert!(layout.iter().all(|lc| lc.lane == 0));
        assert!(layout.iter().all(|lc| lc.lane_count == 1));
    }

    #[test]
    fn parse_numstat_handles_binary_and_tabs_in_path() {
        let raw = "3\t1\tsrc/main.rs\n-\t-\tlogo.png\n0\t5\tpath\twith\ttab.txt\n\n";
        let stats = parse_numstat(raw);
        assert_eq!(
            stats,
            vec![
                FileStat {
                    path: "src/main.rs".into(),
                    added: 3,
                    removed: 1
                },
                FileStat {
                    path: "logo.png".into(),
                    added: 0,
                    removed: 0
                },
                FileStat {
                    path: "path\twith\ttab.txt".into(),
                    added: 0,
                    removed: 5
                },
            ]
        );
    }

    #[test]
    fn classify_ref_distinguishes_tags_remotes_and_local_branches() {
        assert_eq!(classify_ref("v1.2.0"), RefKind::Tag);
        assert_eq!(classify_ref("origin/main"), RefKind::Remote);
        assert_eq!(classify_ref("someuser/topic"), RefKind::Remote);
        assert_eq!(classify_ref("feat/git-graph"), RefKind::Local);
        assert_eq!(classify_ref("main"), RefKind::Local);
    }

    #[test]
    fn is_no_repo_error_matches_reference_phrases() {
        assert!(is_no_repo_error("fatal: not a git repository"));
        assert!(is_no_repo_error(
            "fatal: your current branch 'main' does not have any commits yet"
        ));
        assert!(!is_no_repo_error(
            "could not read Username for 'https://github.com'"
        ));
    }

    #[test]
    fn relative_age_switches_from_seconds_to_minutes() {
        assert_eq!(relative_age(12), "12s ago");
        assert_eq!(relative_age(59), "59s ago");
        assert_eq!(relative_age(90), "2m ago");
    }

    #[test]
    fn format_commit_date_is_utc_civil() {
        // 1700000000 = 2023-11-14T22:13:20Z
        assert_eq!(format_commit_date(1_700_000_000), "Nov 14 2023");
        assert_eq!(format_commit_date(0), "\u{2014}");
    }

    #[test]
    fn octopus_merge_fans_every_extra_parent() {
        let commits = vec![
            commit("O", &["a", "b", "c"]),
            commit("a", &[]),
            commit("b", &[]),
            commit("c", &[]),
        ];
        let layout = build_graph_layout(&commits);
        let o = &layout[0];
        let branch_targets: Vec<usize> = o
            .bottom_edges
            .iter()
            .filter_map(|e| match e {
                GraphEdge::Branch { to_lane, .. } => Some(*to_lane),
                _ => None,
            })
            .collect();
        // Two extra parents → two distinct new lanes.
        assert_eq!(branch_targets.len(), 2);
        assert_ne!(branch_targets[0], branch_targets[1]);
    }
}
