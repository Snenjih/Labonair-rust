//! Move/open/close transition of a tab's panes.
//!
//! The same reveal the docks use (`dur_slow`, `ease_premium`, grow from an edge
//! plus fade), applied to panes. The flex tree cannot animate a pane that is
//! being removed or re-parented, so while a *shape* change plays the panes are
//! placed absolutely at rectangles interpolated between the previous and the
//! new [`tiling::solve`] result; afterwards the plain flex tree takes over.
//!
//! * a pane that is new (split, drop) grows out of the edge it shares with its
//!   neighbour while fading in;
//! * a pane that is gone (close) leaves a hollow [`Placed::Ghost`] that shrinks
//!   toward its far edge while fading out;
//! * every other pane slides/resizes from where it was shown.
//!
//! Divider drags and window/dock resizes keep the shape and therefore never
//! animate. [`MotionLevel::Reduced`] and `Off` skip the transition entirely.

use std::time::{Duration, Instant};

use labonair_ui_kit::{MotionLevel, MotionPrefs};

use crate::pane::{PaneId, SplitAxis};
use crate::pane_group::{Member, PaneGroup};
use crate::tiling::PxRect;

/// One element of a transition frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Placed {
    Pane {
        id: PaneId,
        rect: PxRect,
        opacity: f32,
    },
    /// The shell of a pane that no longer exists.
    Ghost { rect: PxRect, opacity: f32 },
}

struct Ghost {
    from: PxRect,
    to: PxRect,
}

struct Anim {
    start: Instant,
    /// Start rectangle of every pane in the new layout (entering panes start
    /// collapsed). Ids missing from `to` are not in the layout.
    from: Vec<(PaneId, PxRect, bool)>,
    ghosts: Vec<Ghost>,
}

/// Per-tab transition state.
#[derive(Default)]
pub(crate) struct PaneMotion {
    shape: Vec<u64>,
    /// What the previous frame displayed (the start of an interrupted run).
    shown: Vec<(PaneId, PxRect)>,
    anim: Option<Anim>,
}

/// Structure of the tree, ignoring flexes: axis ids and directions in preorder
/// plus the pane ids. Equal shapes never animate.
pub(crate) fn shape(group: &PaneGroup) -> Vec<u64> {
    fn walk(m: &Member, out: &mut Vec<u64>) {
        match m {
            Member::Pane(id) => out.push(*id),
            Member::Axis(ax) => {
                out.push(ax.id);
                out.push(match ax.axis {
                    SplitAxis::Horizontal => 0,
                    SplitAxis::Vertical => 1,
                });
                out.push(ax.members.len() as u64);
                ax.members.iter().for_each(|m| walk(m, out));
            }
        }
    }
    let mut out = Vec::new();
    if let Some(root) = &group.root {
        walk(root, &mut out);
    }
    out
}

fn lerp(a: PxRect, b: PxRect, t: f32) -> PxRect {
    let l = |a: f32, b: f32| a + (b - a) * t;
    PxRect::new(l(a.x, b.x), l(a.y, b.y), l(a.w, b.w), l(a.h, b.h))
}

/// Length of the edge `a` and `b` share when they are `gap` apart (0 if not
/// neighbours), and which side of `a` the neighbour is on.
fn shared_edge(a: PxRect, b: PxRect, gap: f32) -> Option<(f32, Side)> {
    let tol = gap + 1.0;
    let overlap = |a0: f32, a1: f32, b0: f32, b1: f32| (a1.min(b1) - a0.max(b0)).max(0.0);
    let (dy, dx) = (
        overlap(a.y, a.bottom(), b.y, b.bottom()),
        overlap(a.x, a.right(), b.x, b.right()),
    );
    if dy > 0.0 && (b.x - a.right()).abs() <= tol {
        Some((dy, Side::Right))
    } else if dy > 0.0 && (a.x - b.right()).abs() <= tol {
        Some((dy, Side::Left))
    } else if dx > 0.0 && (b.y - a.bottom()).abs() <= tol {
        Some((dx, Side::Bottom))
    } else if dx > 0.0 && (a.y - b.bottom()).abs() <= tol {
        Some((dx, Side::Top))
    } else {
        None
    }
}

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

/// `r` collapsed to a zero-thickness strip on the edge opposite the neighbour
/// (or to its centre without one).
fn collapsed(r: PxRect, neighbour: Option<(PxRect, f32)>) -> PxRect {
    let near = neighbour.and_then(|(n, gap)| shared_edge(r, n, gap));
    match near.map(|(_, side)| side) {
        Some(Side::Left) => PxRect::new(r.right(), r.y, 0.0, r.h),
        Some(Side::Right) => PxRect::new(r.x, r.y, 0.0, r.h),
        Some(Side::Top) => PxRect::new(r.x, r.bottom(), r.w, 0.0),
        Some(Side::Bottom) => PxRect::new(r.x, r.y, r.w, 0.0),
        None => {
            let (cx, cy) = r.center();
            PxRect::new(cx, cy, 0.0, 0.0)
        }
    }
}

/// The surviving pane (among `others`) that shares the longest edge with `r`.
fn best_neighbour(r: PxRect, others: &[(PaneId, PxRect)], gap: f32) -> Option<PxRect> {
    others
        .iter()
        .filter_map(|(_, o)| shared_edge(r, *o, gap).map(|(len, _)| (len, *o)))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, o)| o)
}

impl Anim {
    fn new(was: &[(PaneId, PxRect)], target: &[(PaneId, PxRect)], now: Instant, gap: f32) -> Self {
        let survivors_now: Vec<_> = target
            .iter()
            .filter(|(id, _)| was.iter().any(|(w, _)| w == id))
            .copied()
            .collect();
        let from = target
            .iter()
            .map(|(id, to)| match was.iter().find(|(w, _)| w == id) {
                Some((_, r)) => (*id, *r, false),
                None => {
                    let n = best_neighbour(*to, &survivors_now, gap).map(|n| (n, gap));
                    (*id, collapsed(*to, n), true)
                }
            })
            .collect();
        let survivors_was: Vec<_> = was
            .iter()
            .filter(|(id, _)| target.iter().any(|(t, _)| t == id))
            .copied()
            .collect();
        let ghosts = was
            .iter()
            .filter(|(id, _)| !target.iter().any(|(t, _)| t == id))
            .map(|(_, r)| {
                let n = best_neighbour(*r, &survivors_was, gap).map(|n| (n, gap));
                Ghost {
                    from: *r,
                    to: collapsed(*r, n),
                }
            })
            .collect();
        Self {
            start: now,
            from,
            ghosts,
        }
    }
}

impl PaneMotion {
    /// Observe one frame. `Some` means "render these absolutely placed
    /// elements instead of the flex tree" and the caller must request another
    /// frame; `None` means the plain layout.
    pub(crate) fn step(
        &mut self,
        shape: Vec<u64>,
        target: &[(PaneId, PxRect)],
        gap: f32,
        now: Instant,
        motion: &MotionPrefs,
    ) -> Option<Vec<Placed>> {
        if shape != self.shape {
            let was = std::mem::take(&mut self.shown);
            self.shape = shape;
            self.anim =
                (motion.level == MotionLevel::Full && !was.is_empty() && !target.is_empty())
                    .then(|| Anim::new(&was, target, now, gap));
        }
        let dur: Duration = motion.dur_slow;
        let progress = self
            .anim
            .as_ref()
            .map(|a| now.saturating_duration_since(a.start).as_secs_f32() / dur.as_secs_f32())
            .filter(|p| *p < 1.0);
        let (Some(anim), Some(p)) = (&self.anim, progress) else {
            self.anim = None;
            self.shown = target.to_vec();
            return None;
        };
        let t = motion.ease_premium.eval(p);
        let mut placed: Vec<Placed> = anim
            .ghosts
            .iter()
            .map(|g| Placed::Ghost {
                rect: lerp(g.from, g.to, t),
                opacity: 1.0 - t,
            })
            .collect();
        self.shown.clear();
        for ((id, to), (_, from, entering)) in target.iter().zip(&anim.from) {
            let rect = lerp(*from, *to, t);
            self.shown.push((*id, rect));
            placed.push(Placed::Pane {
                id: *id,
                rect,
                opacity: if *entering { t } else { 1.0 },
            });
        }
        Some(placed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefs(level: MotionLevel) -> MotionPrefs {
        MotionPrefs::new(level, &Default::default())
    }

    fn rect(x: f32, w: f32) -> PxRect {
        PxRect::new(x, 0.0, w, 100.0)
    }

    fn pane_rect(placed: &[Placed], pane: PaneId) -> PxRect {
        placed
            .iter()
            .find_map(|p| match p {
                Placed::Pane { id, rect, .. } if *id == pane => Some(*rect),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn split_grows_the_new_pane_out_of_its_neighbours_far_edge() {
        let m = prefs(MotionLevel::Full);
        let (mut pm, t0) = (PaneMotion::default(), Instant::now());
        assert!(pm
            .step(vec![1], &[(1, rect(0.0, 100.0))], 6.0, t0, &m)
            .is_none());
        let target = [(1, rect(0.0, 47.0)), (2, rect(53.0, 47.0))];
        let first = pm.step(vec![1, 2], &target, 6.0, t0, &m).unwrap();
        assert_eq!(pane_rect(&first, 1), rect(0.0, 100.0));
        assert_eq!(pane_rect(&first, 2), rect(100.0, 0.0));
        let done = t0 + m.dur_slow;
        assert!(pm.step(vec![1, 2], &target, 6.0, done, &m).is_none());
    }

    #[test]
    fn close_leaves_a_shrinking_ghost() {
        let m = prefs(MotionLevel::Full);
        let (mut pm, t0) = (PaneMotion::default(), Instant::now());
        let both = [(1, rect(0.0, 47.0)), (2, rect(53.0, 47.0))];
        pm.step(vec![1, 2], &both, 6.0, t0, &m);
        let frame = pm
            .step(vec![1], &[(1, rect(0.0, 100.0))], 6.0, t0, &m)
            .unwrap();
        assert!(matches!(frame[0], Placed::Ghost { rect, .. } if rect == self::rect(53.0, 47.0)));
        assert_eq!(pane_rect(&frame, 1), rect(0.0, 47.0));
    }

    #[test]
    fn same_shape_or_reduced_motion_never_animates() {
        let m = prefs(MotionLevel::Full);
        let (mut pm, t0) = (PaneMotion::default(), Instant::now());
        pm.step(
            vec![1, 2],
            &[(1, rect(0.0, 47.0)), (2, rect(53.0, 47.0))],
            6.0,
            t0,
            &m,
        );
        // divider drag: same shape, other rects
        let dragged = [(1, rect(0.0, 30.0)), (2, rect(36.0, 64.0))];
        assert!(pm.step(vec![1, 2], &dragged, 6.0, t0, &m).is_none());

        let r = prefs(MotionLevel::Reduced);
        let mut pm = PaneMotion::default();
        pm.step(vec![1], &[(1, rect(0.0, 100.0))], 6.0, t0, &r);
        let split = [(1, rect(0.0, 47.0)), (2, rect(53.0, 47.0))];
        assert!(pm.step(vec![1, 2], &split, 6.0, t0, &r).is_none());
    }
}
